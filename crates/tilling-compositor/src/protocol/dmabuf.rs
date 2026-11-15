use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::wayland::dmabuf::{
    DmabufFeedback, DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier,
};

use crate::runtime::Compositor;

pub(crate) struct DmabufImports {
    state: DmabufState,
    awaiting_renderer: Vec<(Dmabuf, ImportNotifier)>,
}

impl Default for DmabufImports {
    fn default() -> Self {
        Self {
            state: DmabufState::new(),
            awaiting_renderer: Vec::new(),
        }
    }
}

impl DmabufImports {
    pub(crate) fn advertise(&mut self, display: &DisplayHandle, feedback: &DmabufFeedback) {
        self.state
            .create_global_with_default_feedback::<Compositor>(display, feedback);
    }

    pub(crate) fn take_awaiting_renderer(&mut self) -> Vec<(Dmabuf, ImportNotifier)> {
        std::mem::take(&mut self.awaiting_renderer)
    }
}

impl DmabufHandler for Compositor {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf.state
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        self.dmabuf.awaiting_renderer.push((dmabuf, notifier));
    }
}
