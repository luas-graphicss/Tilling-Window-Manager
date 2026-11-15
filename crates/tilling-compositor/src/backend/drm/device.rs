use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

use smithay::backend::allocator::Fourcc;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
use smithay::backend::drm::exporter::gbm::GbmFramebufferExporter;
use smithay::backend::drm::output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements};
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmError, DrmEventMetadata, DrmNode};
use smithay::backend::session::Session;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::desktop::utils::OutputPresentationFeedback;
use smithay::output::{Mode as OutputMode, Output};
use smithay::reexports::calloop::{self, LoopHandle, RegistrationToken};
use smithay::reexports::drm::control::{self, ModeTypeFlags, connector, crtc};
use smithay::reexports::rustix::fs::OFlags;
use smithay::utils::{DeviceFd, Size};

use super::mode::{self, Candidate, Selection};
use super::monitor::Monitor;
use super::surface::{RenderTarget, Surface};
use super::{DrmRenderer, Gpus, connectors, render_node_of};
use crate::error::{CompositorError, cause_chain};
use crate::output::OutputDescription;
use crate::render::SceneElement;
use crate::runtime::{Compositor, Runtime};

const COLOR_FORMATS: [Fourcc; 2] = [Fourcc::Abgr8888, Fourcc::Argb8888];
const UNKNOWN: &str = "Unknown";
const UNSCALED: f64 = 1.0;

type Allocator = GbmAllocator<DrmDeviceFd>;
type Exporter = GbmFramebufferExporter<DrmDeviceFd>;
type Feedback = Option<OutputPresentationFeedback>;
type OutputManager = DrmOutputManager<Allocator, Exporter, Feedback, DrmDeviceFd>;
pub(super) type KmsOutput = DrmOutput<Allocator, Exporter, Feedback, DrmDeviceFd>;

#[derive(Debug, thiserror::Error)]
pub(super) enum DeviceError {
    #[error("open the device node: {0}")]
    Open(String),
    #[error("take over the display controller")]
    Controller(#[source] DrmError),
    #[error("create the buffer allocator")]
    Allocator(#[source] io::Error),
    #[error("start the renderer: {0}")]
    Renderer(String),
    #[error("watch display events")]
    Events(#[source] calloop::Error),
}

#[derive(Debug, thiserror::Error)]
enum ConnectError {
    #[error("register the output")]
    Output(#[from] CompositorError),
    #[error("select the renderer: {0}")]
    Renderer(String),
    #[error("list the planes of the display controller")]
    Planes(#[source] DrmError),
    #[error("set the display mode: {0}")]
    Modeset(String),
}

pub(super) struct Device {
    node: DrmNode,
    render_node: DrmNode,
    manager: OutputManager,
    events: RegistrationToken,
    surfaces: HashMap<crtc::Handle, Surface>,
}

impl Device {
    pub(super) fn open(
        node: DrmNode,
        path: &Path,
        session: &mut LibSeatSession,
        gpus: &mut Gpus,
        loop_handle: &LoopHandle<'static, Runtime>,
    ) -> Result<Self, DeviceError> {
        let flags = OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK;
        let descriptor = session
            .open(path, flags)
            .map_err(|error| DeviceError::Open(cause_chain(&error)))?;
        let descriptor = DrmDeviceFd::new(DeviceFd::from(descriptor));
        let (controller, notifier) =
            DrmDevice::new(descriptor.clone(), true).map_err(DeviceError::Controller)?;
        let gbm = GbmDevice::new(descriptor).map_err(DeviceError::Allocator)?;
        let render_node = render_node_of(node);
        gpus.as_mut()
            .add_node(render_node, gbm.clone())
            .map_err(|error| DeviceError::Renderer(error.to_string()))?;
        let prepared =
            Self::prepare(node, render_node, controller, gbm, gpus).and_then(|manager| {
                let events = loop_handle
                    .insert_source(notifier, move |event, metadata, runtime| {
                        let metadata = *metadata;
                        runtime.with_drm(|backend, compositor| {
                            backend.handle_drm(node, event, metadata, compositor);
                        });
                    })
                    .map_err(|error| DeviceError::Events(error.error))?;
                Ok(Self {
                    node,
                    render_node,
                    manager,
                    events,
                    surfaces: HashMap::new(),
                })
            });
        if prepared.is_err() {
            gpus.as_mut().remove_node(&render_node);
        }
        prepared
    }

    pub(super) fn close(mut self, gpus: &mut Gpus, compositor: &mut Compositor) {
        let lit: Vec<crtc::Handle> = self.surfaces.keys().copied().collect();
        for crtc in lit {
            self.disconnect(crtc, compositor);
        }
        compositor.loop_handle.remove(self.events);
        gpus.as_mut().remove_node(&self.render_node);
    }

    pub(super) fn render_node(&self) -> DrmNode {
        self.render_node
    }

    pub(super) fn monitors(&self) -> impl Iterator<Item = &Monitor> {
        self.surfaces.values().filter_map(Surface::monitor)
    }

    pub(super) fn rescan(
        &mut self,
        connected_monitors: &mut Vec<Monitor>,
        gpus: &mut Gpus,
        compositor: &mut Compositor,
    ) {
        let connected = match connectors::connected(self.manager.device()) {
            Ok(connected) => connected,
            Err(error) => {
                tracing::warn!(%error, "keep the known outputs, connectors cannot be read");
                return;
            }
        };
        let is_connected =
            |handle: connector::Handle| connected.iter().any(|info| info.handle() == handle);
        let gone: Vec<crtc::Handle> = self
            .surfaces
            .iter()
            .filter(|(_, surface)| !is_connected(surface.connector()))
            .map(|(crtc, _)| *crtc)
            .collect();
        for crtc in gone {
            self.disconnect(crtc, compositor);
        }
        for info in &connected {
            let lit = self
                .surfaces
                .values()
                .any(|surface| surface.connector() == info.handle());
            if lit {
                continue;
            }
            let taken: HashSet<crtc::Handle> = self.surfaces.keys().copied().collect();
            match connectors::free_crtc(self.manager.device(), info, &taken) {
                Some(crtc) => self.connect(info, crtc, connected_monitors, gpus, compositor),
                None => tracing::warn!("no display controller is free for a connected output"),
            }
        }
    }

    pub(super) fn pause(&mut self, compositor: &Compositor) {
        self.manager.pause();
        for surface in self.surfaces.values_mut() {
            surface.suspend(compositor);
        }
    }

    pub(super) fn activate(&mut self) -> Result<(), DrmError> {
        self.manager.activate(false)?;
        for surface in self.surfaces.values_mut() {
            surface.resume();
        }
        Ok(())
    }

    pub(super) fn redraw(
        &mut self,
        gpus: &mut Gpus,
        primary: DrmNode,
        compositor: &mut Compositor,
    ) {
        for (crtc, surface) in &mut self.surfaces {
            let mut target = RenderTarget {
                gpus: &mut *gpus,
                primary,
                render_node: self.render_node,
                device: self.node,
                crtc: *crtc,
            };
            surface.redraw(&mut target, compositor);
        }
    }

    pub(super) fn vblank(
        &mut self,
        crtc: crtc::Handle,
        metadata: Option<DrmEventMetadata>,
        gpus: &mut Gpus,
        primary: DrmNode,
        compositor: &mut Compositor,
    ) {
        let Some(surface) = self.surfaces.get_mut(&crtc) else {
            return;
        };
        let mut target = RenderTarget {
            gpus,
            primary,
            render_node: self.render_node,
            device: self.node,
            crtc,
        };
        surface.vblank(metadata, &mut target, compositor);
    }

    pub(super) fn throttle_elapsed(
        &mut self,
        crtc: crtc::Handle,
        gpus: &mut Gpus,
        primary: DrmNode,
        compositor: &mut Compositor,
    ) {
        let Some(surface) = self.surfaces.get_mut(&crtc) else {
            return;
        };
        let mut target = RenderTarget {
            gpus,
            primary,
            render_node: self.render_node,
            device: self.node,
            crtc,
        };
        surface.throttle_elapsed(&mut target, compositor);
    }

    fn prepare(
        node: DrmNode,
        render_node: DrmNode,
        controller: DrmDevice,
        gbm: GbmDevice<DrmDeviceFd>,
        gpus: &mut Gpus,
    ) -> Result<OutputManager, DeviceError> {
        let mut renderer = gpus
            .single_renderer(&render_node)
            .map_err(|error| DeviceError::Renderer(error.to_string()))?;
        let render_formats = renderer
            .as_mut()
            .egl_context()
            .dmabuf_render_formats()
            .clone();
        let flags = GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT;
        let allocator = GbmAllocator::new(gbm.clone(), flags);
        let exporter = GbmFramebufferExporter::new(gbm.clone(), Some(render_node));
        tracing::info!(device = %node, renderer = %render_node, "graphics device ready");
        Ok(DrmOutputManager::new(
            controller,
            allocator,
            exporter,
            Some(gbm),
            COLOR_FORMATS,
            render_formats,
        ))
    }

    fn connect(
        &mut self,
        info: &connector::Info,
        crtc: crtc::Handle,
        connected_monitors: &mut Vec<Monitor>,
        gpus: &mut Gpus,
        compositor: &mut Compositor,
    ) {
        let name = format!("{}-{}", info.interface().as_str(), info.interface_id());
        let monitor = Monitor::read(self.manager.device(), info.handle());
        let monitor_key = monitor
            .as_ref()
            .map(|monitor| monitor.key(&name, connected_monitors.contains(monitor)));
        let wanted = compositor
            .output_profile(&name, monitor_key.as_deref())
            .and_then(|profile| profile.mode);
        let candidates: Vec<Candidate> = info.modes().iter().map(candidate_of).collect();
        let Some(selection) = mode::select(&candidates, wanted) else {
            tracing::warn!(output = name, "connected output offers no display mode");
            return;
        };
        if wanted.is_some() && matches!(selection, Selection::Default(_)) {
            tracing::warn!(
                output = name,
                "configured mode is not offered, using the default"
            );
        }
        let fallback = mode::default_mode(&candidates).filter(|index| *index != selection.index());
        for index in [Some(selection.index()), fallback].into_iter().flatten() {
            let Some(drm_mode) = info.modes().get(index).copied() else {
                continue;
            };
            let description =
                describe(&name, monitor_key.clone(), monitor.as_ref(), info, drm_mode);
            match self.light_up(description, info.handle(), crtc, drm_mode, gpus, compositor) {
                Ok((output, kms)) => {
                    let surface = Surface::new(output, info.handle(), monitor.clone(), kms);
                    self.surfaces.insert(crtc, surface);
                    connected_monitors.extend(monitor);
                    compositor.demand_frame();
                    return;
                }
                Err(error) => {
                    let error = cause_chain(&error);
                    tracing::warn!(error, output = name, "display mode was refused");
                }
            }
        }
    }

    fn disconnect(&mut self, crtc: crtc::Handle, compositor: &mut Compositor) {
        if let Some(surface) = self.surfaces.remove(&crtc) {
            surface.retire(compositor);
        }
    }

    fn light_up(
        &mut self,
        description: OutputDescription,
        connector: connector::Handle,
        crtc: crtc::Handle,
        mode: control::Mode,
        gpus: &mut Gpus,
        compositor: &mut Compositor,
    ) -> Result<(Output, KmsOutput), ConnectError> {
        let output = compositor.attach_output(description)?;
        match self.modeset(&output, connector, crtc, mode, gpus) {
            Ok(kms) => Ok((output, kms)),
            Err(error) => {
                compositor.detach_output(&output);
                Err(error)
            }
        }
    }

    fn modeset(
        &mut self,
        output: &Output,
        connector: connector::Handle,
        crtc: crtc::Handle,
        mode: control::Mode,
        gpus: &mut Gpus,
    ) -> Result<KmsOutput, ConnectError> {
        let mut renderer = gpus
            .single_renderer(&self.render_node)
            .map_err(|error| ConnectError::Renderer(error.to_string()))?;
        let planes = self
            .manager
            .device()
            .planes(&crtc)
            .map_err(ConnectError::Planes)?;
        let other_outputs =
            DrmOutputRenderElements::<DrmRenderer<'_>, SceneElement<DrmRenderer<'_>>>::default();
        self.manager
            .initialize_output(
                crtc,
                mode,
                &[connector],
                output,
                Some(planes),
                &mut renderer,
                &other_outputs,
            )
            .map_err(|error| ConnectError::Modeset(error.to_string()))
    }
}

fn candidate_of(mode: &control::Mode) -> Candidate {
    let (width, height) = mode.size();
    let advertised = OutputMode::from(*mode);
    Candidate {
        width: u32::from(width),
        height: u32::from(height),
        refresh_millihertz: u32::try_from(advertised.refresh).unwrap_or(0),
        preferred: mode.mode_type().contains(ModeTypeFlags::PREFERRED),
    }
}

fn describe(
    name: &str,
    monitor_key: Option<String>,
    monitor: Option<&Monitor>,
    info: &connector::Info,
    mode: control::Mode,
) -> OutputDescription {
    let (width, height) = info.size().unwrap_or((0, 0));
    let millimetres = |length: u32| i32::try_from(length).unwrap_or(0);
    let labelled = |label: fn(&Monitor) -> &String| {
        monitor.map_or_else(|| UNKNOWN.to_owned(), |monitor| label(monitor).clone())
    };
    OutputDescription {
        name: name.to_owned(),
        monitor_key,
        make: labelled(|monitor| &monitor.make),
        model: labelled(|monitor| &monitor.model),
        physical_size: Size::from((millimetres(width), millimetres(height))),
        subpixel: info.subpixel().into(),
        mode: OutputMode::from(mode),
        suggested_scale: UNSCALED,
    }
}
