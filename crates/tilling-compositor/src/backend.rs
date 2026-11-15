mod headless;

use smithay::reexports::calloop::LoopHandle;

use crate::error::CompositorError;
use crate::options::{BackendKind, VirtualOutput};
use crate::runtime::{Compositor, Runtime};
use headless::HeadlessBackend;

pub(crate) enum Backend {
    Headless(HeadlessBackend),
}

impl Backend {
    pub(crate) fn start(
        kind: BackendKind,
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<Self, CompositorError> {
        match kind {
            BackendKind::Headless => HeadlessBackend::start(output, compositor).map(Self::Headless),
            BackendKind::Nested => Err(CompositorError::BackendUnavailable(kind.name())),
        }
    }

    pub(crate) fn schedule_frame(&mut self, handle: &LoopHandle<'static, Runtime>) {
        match self {
            Self::Headless(backend) => backend.schedule_frame(handle),
        }
    }

    pub(crate) fn present(&mut self, compositor: &mut Compositor) {
        match self {
            Self::Headless(backend) => backend.present(compositor),
        }
    }
}
