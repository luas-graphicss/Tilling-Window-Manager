mod connectors;
mod device;
mod device_settings;
mod feedback;
mod frame_clock;
mod input;
mod mode;
mod monitor;
mod recovery;
mod seat;
mod surface;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use smithay::backend::drm::{DrmDeviceFd, DrmEvent, DrmEventMetadata, DrmNode, NodeType};
use smithay::backend::input::InputEvent;
use smithay::backend::libinput::LibinputInputBackend;
use smithay::backend::renderer::ImportDma;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::{GpuManager, MultiRenderer};
use smithay::backend::session::Event as SessionEvent;
use smithay::backend::udev::{self, UdevBackend, UdevEvent};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::drm::control::crtc;
use smithay::reexports::rustix::fs::Dev;
use smithay::utils::{Clock, Monotonic};
use smithay::wayland::dmabuf::DmabufFeedbackBuilder;
use smithay::wayland::presentation::PresentationState;

use crate::error::{CompositorError, cause_chain};
use crate::runtime::Compositor;
use device::Device;
use input::InputDevices;
use monitor::Monitor;
use recovery::Recovery;
use seat::Seat;

type Api = GbmGlesBackend<GlesRenderer, DrmDeviceFd>;
type Gpus = GpuManager<Api>;
type DrmRenderer<'gpus> = MultiRenderer<'gpus, 'gpus, Api, Api>;

const REACTIVATION_ATTEMPTS: u32 = 5;
const FIRST_REACTIVATION_DELAY: Duration = Duration::from_millis(250);

pub(crate) struct DrmBackend {
    devices: HashMap<DrmNode, Device>,
    gpus: Gpus,
    input: InputDevices,
    seat: Seat,
    primary: DrmNode,
    reactivation: Recovery,
}

impl DrmBackend {
    pub(crate) fn start(compositor: &mut Compositor) -> Result<Self, CompositorError> {
        let seat = Seat::open(&compositor.loop_handle)?;
        let primary = primary_gpu(seat.name())?;
        let gpus = GpuManager::new(Api::default())
            .map_err(|error| CompositorError::Renderer(error.to_string()))?;
        let input = InputDevices::open(&seat, &compositor.loop_handle)?;
        let hotplug = UdevBackend::new(seat.name()).map_err(CompositorError::Gpus)?;
        let present: Vec<(Dev, PathBuf)> = hotplug
            .device_list()
            .map(|(id, path)| (id, path.to_path_buf()))
            .collect();
        compositor
            .loop_handle
            .insert_source(hotplug, |event, &mut (), runtime| {
                runtime.with_drm(|backend, compositor| backend.handle_udev(event, compositor));
            })
            .map_err(|error| CompositorError::EventSource(error.error))?;
        let mut backend = Self {
            devices: HashMap::new(),
            gpus,
            input,
            seat,
            primary,
            reactivation: Recovery::new(REACTIVATION_ATTEMPTS, FIRST_REACTIVATION_DELAY),
        };
        for (id, path) in present {
            backend.add_device(id, &path, compositor);
        }
        backend.render_on_an_opened_device()?;
        backend.advertise_client_buffers(compositor);
        advertise_presentation_clock(compositor);
        draw_first_frame(compositor)?;
        compositor.draws_named_cursor = true;
        Ok(backend)
    }

    pub(crate) fn follow(&mut self, terminal: Option<i32>, compositor: &mut Compositor) {
        if let Some(terminal) = terminal {
            self.seat.switch_terminal(terminal);
        }
        self.input.follow_config(compositor);
        self.import_client_buffers(compositor);
        if self.seat.is_active() {
            for device in self.devices.values_mut() {
                device.follow_modes(&mut self.gpus, self.primary, compositor);
            }
        }
    }

    pub(crate) fn redraw(&mut self, compositor: &mut Compositor) {
        for device in self.devices.values_mut() {
            device.redraw(&mut self.gpus, self.primary, compositor);
        }
    }

    fn handle_session(&mut self, event: SessionEvent, compositor: &mut Compositor) {
        match event {
            SessionEvent::PauseSession => self.pause(compositor),
            SessionEvent::ActivateSession => {
                self.reactivation.succeeded();
                self.activate(compositor);
            }
        }
    }

    fn handle_udev(&mut self, event: UdevEvent, compositor: &mut Compositor) {
        match event {
            UdevEvent::Added { device_id, path } => self.add_device(device_id, &path, compositor),
            UdevEvent::Changed { device_id } => self.rescan_device(device_id, compositor),
            UdevEvent::Removed { device_id } => self.remove_device(device_id, compositor),
        }
    }

    fn handle_drm(
        &mut self,
        node: DrmNode,
        event: DrmEvent,
        metadata: Option<DrmEventMetadata>,
        compositor: &mut Compositor,
    ) {
        let Some(device) = self.devices.get_mut(&node) else {
            return;
        };
        match event {
            DrmEvent::VBlank(crtc) => {
                device.vblank(crtc, metadata, &mut self.gpus, self.primary, compositor);
            }
            DrmEvent::Error(error) => {
                tracing::warn!(error = %cause_chain(&error), "display controller failed");
            }
        }
    }

    fn handle_input(
        &mut self,
        event: InputEvent<LibinputInputBackend>,
        compositor: &mut Compositor,
    ) {
        self.input.handle(event, compositor);
    }

    fn throttle_elapsed(&mut self, node: DrmNode, crtc: crtc::Handle, compositor: &mut Compositor) {
        if let Some(device) = self.devices.get_mut(&node) {
            device.throttle_elapsed(crtc, &mut self.gpus, self.primary, compositor);
        }
    }

    fn add_device(&mut self, id: Dev, path: &Path, compositor: &mut Compositor) {
        let node = match DrmNode::from_dev_id(id) {
            Ok(node) => node,
            Err(error) => {
                tracing::warn!(%error, device = %path.display(), "skip graphics device");
                return;
            }
        };
        if self.devices.contains_key(&node) {
            return;
        }
        let opened = Device::open(
            node,
            path,
            self.seat.session(),
            &mut self.gpus,
            &compositor.loop_handle,
        );
        let mut device = match opened {
            Ok(device) => device,
            Err(error) => {
                let error = cause_chain(&error);
                tracing::warn!(error, device = %path.display(), "skip graphics device");
                return;
            }
        };
        let mut connected = self.monitors();
        device.rescan(&mut connected, &mut self.gpus, compositor);
        self.devices.insert(node, device);
    }

    fn rescan_device(&mut self, id: Dev, compositor: &mut Compositor) {
        let mut connected = self.monitors();
        let known = DrmNode::from_dev_id(id)
            .ok()
            .and_then(|node| self.devices.get_mut(&node));
        if let Some(device) = known {
            device.rescan(&mut connected, &mut self.gpus, compositor);
        }
    }

    fn remove_device(&mut self, id: Dev, compositor: &mut Compositor) {
        let removed = DrmNode::from_dev_id(id)
            .ok()
            .and_then(|node| self.devices.remove(&node));
        if let Some(device) = removed {
            device.close(&mut self.gpus, compositor);
        }
    }

    fn render_on_an_opened_device(&mut self) -> Result<(), CompositorError> {
        let mut renderers = self.devices.values().map(Device::render_node);
        if renderers.any(|renderer| renderer == self.primary) {
            return Ok(());
        }
        let stand_in = self
            .devices
            .values()
            .map(Device::render_node)
            .next()
            .ok_or_else(|| CompositorError::NoGpu(self.seat.name().to_owned()))?;
        tracing::warn!(renderer = %stand_in, "primary graphics device is unusable");
        self.primary = stand_in;
        Ok(())
    }

    fn monitors(&self) -> Vec<Monitor> {
        self.devices
            .values()
            .flat_map(Device::monitors)
            .cloned()
            .collect()
    }

    fn pause(&mut self, compositor: &mut Compositor) {
        tracing::info!("seat session paused");
        self.input.suspend();
        compositor.release_held_keys();
        compositor.cancel_pointer_grab();
        for device in self.devices.values_mut() {
            device.pause(compositor);
        }
    }

    fn activate(&mut self, compositor: &mut Compositor) {
        if !self.seat.is_active() {
            return;
        }
        self.input.resume();
        let mut failure = None;
        for device in self.devices.values_mut() {
            if let Err(error) = device.activate() {
                failure = Some(cause_chain(&error));
            }
        }
        let Some(error) = failure else {
            tracing::info!("seat session active");
            self.reactivation.succeeded();
            self.rescan_every_device(compositor);
            compositor.demand_frame();
            return;
        };
        match self.reactivation.next_delay() {
            Some(delay) => {
                tracing::warn!(error, ?delay, "retry taking the displays back");
                self.retry_activation_after(delay, compositor);
            }
            None => {
                tracing::error!(error, "stop after failing to take the displays back");
                compositor.stop();
            }
        }
    }

    fn retry_activation_after(&mut self, delay: Duration, compositor: &mut Compositor) {
        let timer = Timer::from_duration(delay);
        let armed = compositor
            .loop_handle
            .insert_source(timer, |_, (), runtime| {
                runtime.with_drm(DrmBackend::activate);
                TimeoutAction::Drop
            });
        if let Err(error) = armed {
            tracing::error!(error = %error.error, "stop without a way to retry the displays");
            compositor.stop();
        }
    }

    fn rescan_every_device(&mut self, compositor: &mut Compositor) {
        let nodes: Vec<DrmNode> = self.devices.keys().copied().collect();
        for node in nodes {
            let mut connected = self.monitors();
            if let Some(device) = self.devices.get_mut(&node) {
                device.rescan(&mut connected, &mut self.gpus, compositor);
            }
        }
    }

    fn advertise_client_buffers(&mut self, compositor: &mut Compositor) {
        let formats = match self.gpus.single_renderer(&self.primary) {
            Ok(renderer) => renderer.dmabuf_formats(),
            Err(error) => {
                tracing::warn!(%error, "clients cannot share dmabufs");
                return;
            }
        };
        match DmabufFeedbackBuilder::new(self.primary.dev_id(), formats).build() {
            Ok(feedback) => compositor
                .dmabuf
                .advertise(&compositor.display_handle, &feedback),
            Err(error) => tracing::warn!(%error, "clients cannot share dmabufs"),
        }
    }

    fn import_client_buffers(&mut self, compositor: &mut Compositor) {
        for (dmabuf, notifier) in compositor.dmabuf.take_awaiting_renderer() {
            let imported = self
                .gpus
                .single_renderer(&self.primary)
                .map_err(|error| error.to_string())
                .and_then(|mut renderer| {
                    renderer
                        .import_dmabuf(&dmabuf, None)
                        .map(drop)
                        .map_err(|error| error.to_string())
                });
            match imported {
                Ok(()) => {
                    if notifier.successful::<Compositor>().is_err() {
                        tracing::debug!("dmabuf owner disconnected during the import");
                    }
                }
                Err(error) => {
                    tracing::debug!(error, "refuse a client dmabuf");
                    notifier.failed();
                }
            }
        }
    }
}

fn primary_gpu(seat: &str) -> Result<DrmNode, CompositorError> {
    let path = match udev::primary_gpu(seat).map_err(CompositorError::Gpus)? {
        Some(path) => path,
        None => udev::all_gpus(seat)
            .map_err(CompositorError::Gpus)?
            .into_iter()
            .next()
            .ok_or_else(|| CompositorError::NoGpu(seat.to_owned()))?,
    };
    let node =
        DrmNode::from_path(&path).map_err(|source| CompositorError::GpuNode { path, source })?;
    Ok(render_node_of(node))
}

fn render_node_of(node: DrmNode) -> DrmNode {
    node.node_with_type(NodeType::Render)
        .and_then(Result::ok)
        .unwrap_or(node)
}

fn draw_first_frame(compositor: &Compositor) -> Result<(), CompositorError> {
    compositor
        .loop_handle
        .insert_source(Timer::immediate(), |_, (), runtime| {
            runtime.with_drm(DrmBackend::redraw);
            TimeoutAction::Drop
        })
        .map(drop)
        .map_err(|error| CompositorError::EventSource(error.error))
}

fn advertise_presentation_clock(compositor: &Compositor) {
    let clock = Clock::<Monotonic>::new().id();
    PresentationState::new::<Compositor>(&compositor.display_handle, clock as u32);
}
