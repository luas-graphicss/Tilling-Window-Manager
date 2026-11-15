use std::time::Duration;

use smithay::backend::allocator::Fourcc;
use smithay::backend::drm::compositor::FrameFlags;
use smithay::backend::drm::{DrmEventMetadata, DrmNode};
use smithay::output::Output;
use smithay::reexports::calloop::RegistrationToken;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::drm::control::{connector, crtc};

use super::device::KmsOutput;
use super::frame_clock::{FrameClock, Wait};
use super::monitor::Monitor;
use super::recovery::Recovery;
use super::{DrmRenderer, Gpus, feedback};
use crate::render::{BACKGROUND, PointerSprite, SceneView};
use crate::runtime::Compositor;

const FALLBACK_REFRESH_INTERVAL: Duration = Duration::from_micros(16_667);
const NANOSECOND_MILLIHERTZ: u64 = 1_000_000_000_000;
const RENDER_ATTEMPTS: u32 = 4;
const FIRST_RENDER_RETRY: Duration = Duration::from_millis(100);

#[derive(Debug, thiserror::Error)]
enum RenderError {
    #[error("select the renderer: {0}")]
    Renderer(String),
    #[error("compose the frame: {0}")]
    Compose(String),
    #[error("queue the frame: {0}")]
    Queue(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frame {
    Queued,
    Unchanged,
}

pub(super) struct RenderTarget<'gpus> {
    pub(super) gpus: &'gpus mut Gpus,
    pub(super) primary: DrmNode,
    pub(super) render_node: DrmNode,
    pub(super) device: DrmNode,
    pub(super) crtc: crtc::Handle,
}

pub(super) struct Surface {
    output: Output,
    connector: connector::Handle,
    monitor: Option<Monitor>,
    kms: KmsOutput,
    view: SceneView,
    pointer: PointerSprite,
    clock: FrameClock,
    throttle: Option<RegistrationToken>,
    failures: Recovery,
}

impl RenderTarget<'_> {
    fn renderer(&mut self, copy_format: Fourcc) -> Result<DrmRenderer<'_>, RenderError> {
        let selected = if self.primary == self.render_node {
            self.gpus.single_renderer(&self.render_node)
        } else {
            self.gpus
                .renderer(&self.primary, &self.render_node, copy_format)
        };
        selected.map_err(|error| RenderError::Renderer(error.to_string()))
    }
}

impl Surface {
    pub(super) fn new(
        output: Output,
        connector: connector::Handle,
        monitor: Option<Monitor>,
        kms: KmsOutput,
    ) -> Self {
        Self {
            view: SceneView::new(&output),
            output,
            connector,
            monitor,
            kms,
            pointer: PointerSprite::new(),
            clock: FrameClock::default(),
            throttle: None,
            failures: Recovery::new(RENDER_ATTEMPTS, FIRST_RENDER_RETRY),
        }
    }

    pub(super) fn connector(&self) -> connector::Handle {
        self.connector
    }

    pub(super) fn monitor(&self) -> Option<&Monitor> {
        self.monitor.as_ref()
    }

    pub(super) fn redraw(&mut self, target: &mut RenderTarget<'_>, compositor: &mut Compositor) {
        if self.clock.request_redraw() {
            self.draw(target, compositor);
        }
    }

    pub(super) fn vblank(
        &mut self,
        metadata: Option<DrmEventMetadata>,
        target: &mut RenderTarget<'_>,
        compositor: &mut Compositor,
    ) {
        match self.kms.frame_submitted() {
            Ok(Some(Some(mut presented))) => {
                feedback::present(&mut presented, metadata, self.refresh_interval());
            }
            Ok(Some(None) | None) => {}
            Err(error) => tracing::warn!(%error, "confirm the presented frame"),
        }
        if self.clock.finish(Wait::Vblank) {
            self.draw(target, compositor);
        }
    }

    pub(super) fn throttle_elapsed(
        &mut self,
        target: &mut RenderTarget<'_>,
        compositor: &mut Compositor,
    ) {
        self.throttle = None;
        if self.clock.finish(Wait::Throttle) {
            self.draw(target, compositor);
        }
    }

    pub(super) fn suspend(&mut self, compositor: &Compositor) {
        self.cancel_throttle(compositor);
        self.clock.halt();
    }

    pub(super) fn resume(&mut self) {
        self.clock.reset();
        self.failures.succeeded();
        self.kms.reset_buffers();
    }

    pub(super) fn retire(mut self, compositor: &mut Compositor) {
        self.cancel_throttle(compositor);
        compositor.detach_output(&self.output);
    }

    fn draw(&mut self, target: &mut RenderTarget<'_>, compositor: &mut Compositor) {
        match self.render(target, compositor) {
            Ok(Frame::Queued) => {
                self.failures.succeeded();
                self.clock.wait_for(Wait::Vblank);
            }
            Ok(Frame::Unchanged) => {
                self.failures.succeeded();
                self.throttle_for(self.refresh_interval(), target, compositor);
            }
            Err(error) => self.back_off(&error, target, compositor),
        }
        compositor.send_frame_callbacks(&self.output);
    }

    fn render(
        &mut self,
        target: &mut RenderTarget<'_>,
        compositor: &mut Compositor,
    ) -> Result<Frame, RenderError> {
        let mut renderer = target.renderer(self.kms.format())?;
        let mut elements = self
            .pointer
            .elements(compositor, &mut renderer, &self.output);
        elements.extend(self.view.elements(compositor, &mut renderer));
        let frame = self
            .kms
            .render_frame(&mut renderer, &elements, BACKGROUND, FrameFlags::DEFAULT)
            .map_err(|error| RenderError::Compose(error.to_string()))?;
        if frame.is_empty {
            compositor.frames.record(None);
            return Ok(Frame::Unchanged);
        }
        let presented = feedback::collect(compositor, &self.output, &frame.states);
        self.kms
            .queue_frame(Some(presented))
            .map_err(|error| RenderError::Queue(error.to_string()))?;
        compositor.frames.record(Some(&[]));
        Ok(Frame::Queued)
    }

    fn back_off(
        &mut self,
        error: &RenderError,
        target: &RenderTarget<'_>,
        compositor: &Compositor,
    ) {
        let output = self.output.name();
        match self.failures.next_delay() {
            Some(delay) => {
                tracing::warn!(%error, output, ?delay, "retry the frame later");
                self.throttle_for(delay, target, compositor);
            }
            None => {
                tracing::error!(%error, output, "stop presenting after repeated failures");
                self.clock.halt();
            }
        }
    }

    fn throttle_for(
        &mut self,
        delay: Duration,
        target: &RenderTarget<'_>,
        compositor: &Compositor,
    ) {
        let (device, crtc) = (target.device, target.crtc);
        let timer = Timer::from_duration(delay);
        let armed = compositor
            .loop_handle
            .insert_source(timer, move |_, (), runtime| {
                runtime.with_drm(|backend, compositor| {
                    backend.throttle_elapsed(device, crtc, compositor);
                });
                TimeoutAction::Drop
            });
        match armed {
            Ok(token) => {
                self.throttle = Some(token);
                self.clock.wait_for(Wait::Throttle);
            }
            Err(error) => {
                tracing::warn!(error = %error.error, "draw the next frame without a pause");
                self.clock.reset();
            }
        }
    }

    fn cancel_throttle(&mut self, compositor: &Compositor) {
        if let Some(token) = self.throttle.take() {
            compositor.loop_handle.remove(token);
        }
    }

    fn refresh_interval(&self) -> Duration {
        self.output
            .current_mode()
            .and_then(|mode| u64::try_from(mode.refresh).ok())
            .filter(|millihertz| *millihertz > 0)
            .map_or(FALLBACK_REFRESH_INTERVAL, |millihertz| {
                Duration::from_nanos(NANOSECOND_MILLIHERTZ / millihertz)
            })
    }
}
