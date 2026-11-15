mod headless;
#[cfg(feature = "nested")]
mod nested;

#[cfg(feature = "nested")]
use smithay::backend::winit::WinitEvent;
use smithay::reexports::calloop::LoopHandle;

use crate::error::CompositorError;
use crate::options::{BackendKind, VirtualOutput};
use crate::runtime::{Compositor, Runtime};
use headless::HeadlessBackend;
#[cfg(feature = "nested")]
use nested::NestedBackend;

pub(crate) enum Backend {
    Headless(Box<HeadlessBackend>),
    #[cfg(feature = "nested")]
    Nested(Box<NestedBackend>),
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
        }
    }

    pub(crate) fn schedule_frame(&mut self, handle: &LoopHandle<'static, Runtime>) {
        match self {
            Self::Headless(backend) => backend.schedule_frame(handle),
            #[cfg(feature = "nested")]
            Self::Nested(backend) => backend.schedule_frame(),
        }
    }

    pub(crate) fn present(&mut self, compositor: &mut Compositor) {
        match self {
            Self::Headless(backend) => backend.present(compositor),
            #[cfg(feature = "nested")]
            Self::Nested(backend) => backend.present(compositor),
        }
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
