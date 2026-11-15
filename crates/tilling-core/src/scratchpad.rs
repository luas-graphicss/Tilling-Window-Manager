use serde::Serialize;

use crate::identity::WindowId;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Scratchpad {
    stashed_oldest_first: Vec<WindowId>,
    summoned: Vec<WindowId>,
}

impl Scratchpad {
    pub(crate) fn stashed(&self) -> &[WindowId] {
        &self.stashed_oldest_first
    }

    pub(crate) fn summoned(&self) -> &[WindowId] {
        &self.summoned
    }

    pub(crate) fn is_stashed(&self, window: WindowId) -> bool {
        self.stashed_oldest_first.contains(&window)
    }

    pub(crate) fn is_summoned(&self, window: WindowId) -> bool {
        self.summoned.contains(&window)
    }

    pub(crate) fn stash(&mut self, window: WindowId) {
        self.forget(window);
        self.stashed_oldest_first.push(window);
    }

    pub(crate) fn summon_oldest(&mut self) -> Option<WindowId> {
        if self.stashed_oldest_first.is_empty() {
            return None;
        }
        let window = self.stashed_oldest_first.remove(0);
        self.summoned.push(window);
        Some(window)
    }

    pub(crate) fn forget(&mut self, window: WindowId) {
        self.stashed_oldest_first
            .retain(|candidate| *candidate != window);
        self.summoned.retain(|candidate| *candidate != window);
    }
}
