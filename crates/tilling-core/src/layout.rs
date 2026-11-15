mod bsp;

use crate::geometry::LogicalRect;
use crate::identity::WindowId;

pub use bsp::BspTree;

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
