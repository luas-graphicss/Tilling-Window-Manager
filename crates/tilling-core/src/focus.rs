use serde::Serialize;

use crate::identity::WindowId;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct FocusOrder {
    least_recent_first: Vec<WindowId>,
}

impl FocusOrder {
    pub(crate) fn current(&self) -> Option<WindowId> {
        self.least_recent_first.last().copied()
    }

    pub(crate) fn windows(&self) -> &[WindowId] {
        &self.least_recent_first
    }

    pub(crate) fn promote(&mut self, window: WindowId) {
        self.forget(window);
        self.least_recent_first.push(window);
    }

    pub(crate) fn forget(&mut self, window: WindowId) {
        self.least_recent_first
            .retain(|candidate| *candidate != window);
    }
}
