mod bsp;
mod master_stack;
mod monocle;

use serde::Serialize;

use self::bsp::BspTree;
use self::master_stack::MasterStack;
use crate::geometry::LogicalRect;
use crate::identity::WindowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gaps {
    pub inner: u32,
    pub outer: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub window: WindowId,
    pub area: LogicalRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    #[error("layout already tiles the maximum of {limit} windows")]
    Full { limit: usize },
    #[error("window {0} is already tiled")]
    AlreadyTiled(WindowId),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
pub enum LayoutKind {
    #[default]
    Bsp,
    MasterStack,
    Monocle,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Layout {
    kind: LayoutKind,
    tree: BspTree,
    master_stack: MasterStack,
}

impl Layout {
    pub(crate) fn new(kind: LayoutKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn kind(&self) -> LayoutKind {
        self.kind
    }

    #[must_use]
    pub fn windows(&self) -> Vec<WindowId> {
        self.tree.windows()
    }

    #[must_use]
    pub fn contains(&self, window: WindowId) -> bool {
        self.tree.contains(window)
    }

    #[must_use]
    pub fn arrange(&self, area: LogicalRect, gap: u32, front: Option<WindowId>) -> Vec<Placement> {
        match self.kind {
            LayoutKind::Bsp => self.tree.arrange(area, gap),
            LayoutKind::MasterStack => self.master_stack.arrange(&self.windows(), area, gap),
            LayoutKind::Monocle => monocle::arrange(&self.windows(), front, area),
        }
    }

    #[must_use]
    pub fn neighbor(&self, window: WindowId, direction: Direction) -> Option<WindowId> {
        match self.kind {
            LayoutKind::Bsp => self.tree.neighbor(window, direction),
            LayoutKind::MasterStack => {
                self.master_stack
                    .neighbor(&self.windows(), window, direction)
            }
            LayoutKind::Monocle => monocle::neighbor(&self.windows(), window, direction),
        }
    }

    pub(crate) fn set_kind(&mut self, kind: LayoutKind) {
        self.kind = kind;
    }

    pub(crate) fn insert(
        &mut self,
        anchor: Option<WindowId>,
        window: WindowId,
    ) -> Result<(), LayoutError> {
        self.tree.insert(anchor, window)
    }

    pub(crate) fn remove(&mut self, window: WindowId) -> bool {
        self.tree.remove(window)
    }

    pub(crate) fn swap(&mut self, first: WindowId, second: WindowId) -> bool {
        self.tree.swap(first, second)
    }

    pub(crate) fn toggle_orientation(&mut self, window: WindowId) -> bool {
        match self.kind {
            LayoutKind::Bsp => self.tree.toggle_orientation(window),
            LayoutKind::MasterStack => {
                let order = self.windows();
                self.master_stack.toggle_orientation(&order, window)
            }
            LayoutKind::Monocle => false,
        }
    }

    pub(crate) fn resize(&mut self, window: WindowId, delta: i16) -> bool {
        match self.kind {
            LayoutKind::Bsp => self.tree.resize(window, delta),
            LayoutKind::MasterStack => {
                let order = self.windows();
                self.master_stack.resize(&order, window, delta)
            }
            LayoutKind::Monocle => false,
        }
    }
}
