use serde::Serialize;

use crate::focus::FocusOrder;
use crate::identity::{OutputId, WindowId};
use crate::layout::{Layout, LayoutError, LayoutKind};
use crate::window::WindowMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FormerOutput {
    pub(crate) name: String,
    pub(crate) showing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Workspace {
    pub(crate) name: String,
    pub(crate) output: Option<OutputId>,
    pub(crate) former_output: Option<FormerOutput>,
    pub(crate) windows: Vec<WindowId>,
    pub(crate) focus: FocusOrder,
    pub(crate) layout: Layout,
}

impl Workspace {
    pub(crate) fn new(name: String, output: Option<OutputId>, layout: LayoutKind) -> Self {
        Self {
            name,
            output,
            former_output: None,
            windows: Vec::new(),
            focus: FocusOrder::default(),
            layout: Layout::new(layout),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn output(&self) -> Option<OutputId> {
        self.output
    }

    #[must_use]
    pub fn windows(&self) -> &[WindowId] {
        &self.windows
    }

    #[must_use]
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focus.current()
    }

    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub(crate) fn front_tile(&self) -> Option<WindowId> {
        self.focus
            .windows()
            .iter()
            .rev()
            .copied()
            .find(|window| self.layout.contains(*window))
    }

    pub(crate) fn attach(&mut self, window: WindowId, mode: WindowMode) -> Result<(), LayoutError> {
        if mode.occupies_tile() {
            self.layout.insert(self.focus.current(), window)?;
        }
        self.windows.push(window);
        self.focus.promote(window);
        Ok(())
    }

    pub(crate) fn detach(&mut self, window: WindowId) {
        self.layout.remove(window);
        self.windows.retain(|candidate| *candidate != window);
        self.focus.forget(window);
    }
}
