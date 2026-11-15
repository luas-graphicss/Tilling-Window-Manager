#[cfg(feature = "drm")]
mod drm;
mod headless;
#[cfg(feature = "nested")]
mod nested;

#[cfg(feature = "nested")]
use smithay::backend::winit::WinitEvent;

use crate::error::CompositorError;
use crate::options::{BackendKind, VirtualOutput};
use crate::runtime::Compositor;
#[cfg(any(feature = "nested", feature = "drm"))]
use crate::runtime::Runtime;
#[cfg(feature = "drm")]
use drm::DrmBackend;
use headless::HeadlessBackend;
#[cfg(feature = "nested")]
use nested::NestedBackend;

pub(crate) enum Backend {
    Headless(Box<HeadlessBackend>),
    #[cfg(feature = "nested")]
    Nested(Box<NestedBackend>),
    #[cfg(feature = "drm")]
    Drm(Box<DrmBackend>),
}

impl Backend {
    pub(crate) fn start(
        kind: BackendKind,
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<Self, CompositorError> {
        match kind {
            BackendKind::Headless => {
                let backend = HeadlessBackend::start(output, compositor)?;
                Ok(Self::Headless(Box::new(backend)))
            }
            #[cfg(feature = "nested")]
            BackendKind::Nested => {
                let backend = NestedBackend::start(output, compositor)?;
                Ok(Self::Nested(Box::new(backend)))
            }
            #[cfg(not(feature = "nested"))]
            BackendKind::Nested => Err(CompositorError::BackendUnavailable(kind.name())),
            #[cfg(feature = "drm")]
            BackendKind::Drm => {
                let backend = DrmBackend::start(compositor)?;
                Ok(Self::Drm(Box::new(backend)))
            }
            #[cfg(not(feature = "drm"))]
            BackendKind::Drm => Err(CompositorError::BackendUnavailable(kind.name())),
        }
    }

    pub(crate) fn plug_virtual_output(
        &mut self,
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<(), CompositorError> {
        match self {
            Self::Headless(backend) => backend.plug(output, compositor),
            #[cfg(feature = "nested")]
            Self::Nested(_) => Err(CompositorError::FixedOutputs(BackendKind::Nested.name())),
            #[cfg(feature = "drm")]
            Self::Drm(_) => Err(CompositorError::FixedOutputs(BackendKind::Drm.name())),
        }
    }

    pub(crate) fn unplug_virtual_output(
        &mut self,
        name: &str,
        compositor: &mut Compositor,
    ) -> Result<(), CompositorError> {
        match self {
            Self::Headless(backend) => backend.unplug(name, compositor),
            #[cfg(feature = "nested")]
            Self::Nested(_) => Err(CompositorError::FixedOutputs(BackendKind::Nested.name())),
            #[cfg(feature = "drm")]
            Self::Drm(_) => Err(CompositorError::FixedOutputs(BackendKind::Drm.name())),
        }
    }

    pub(crate) fn follow(&mut self, compositor: &mut Compositor) {
        let terminal = compositor.take_terminal_switch();
        match self {
            Self::Headless(_) => ignore_terminal_switch(terminal),
            #[cfg(feature = "nested")]
            Self::Nested(_) => ignore_terminal_switch(terminal),
            #[cfg(feature = "drm")]
            Self::Drm(backend) => backend.follow(terminal, compositor),
        }
    }

    pub(crate) fn schedule_frame(&mut self, compositor: &mut Compositor) {
        match self {
            Self::Headless(backend) => backend.schedule_frame(&compositor.loop_handle),
            #[cfg(feature = "nested")]
            Self::Nested(backend) => backend.schedule_frame(),
            #[cfg(feature = "drm")]
            Self::Drm(backend) => backend.redraw(compositor),
        }
    }

    pub(crate) fn present(&mut self, compositor: &mut Compositor) {
        match self {
            Self::Headless(backend) => backend.present(compositor),
            #[cfg(feature = "nested")]
            Self::Nested(backend) => backend.present(compositor),
            #[cfg(feature = "drm")]
            Self::Drm(backend) => backend.redraw(compositor),
        }
    }
}

fn ignore_terminal_switch(terminal: Option<i32>) {
    if let Some(terminal) = terminal {
        tracing::debug!(terminal, "no seat session to switch terminals on");
    }
}

#[cfg(feature = "nested")]
impl Runtime {
    pub(crate) fn host_event(&mut self, event: WinitEvent) {
        if let Backend::Nested(backend) = &mut self.backend {
            backend.handle_event(event, &mut self.compositor);
        }
    }
}

#[cfg(feature = "drm")]
impl Runtime {
    pub(crate) fn with_drm(&mut self, react: impl FnOnce(&mut DrmBackend, &mut Compositor)) {
        if let Backend::Drm(backend) = &mut self.backend {
            react(backend, &mut self.compositor);
        }
    }
}
