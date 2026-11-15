use serde::Serialize;

use crate::geometry::{LogicalRect, Orientation, Ratio};
use crate::identity::WindowId;
use crate::layout::Placement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    First,
    Second,
}

impl Side {
    pub(super) const fn opposite(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) enum Node {
    Leaf(WindowId),
    Split(Box<Split>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct Split {
    pub(super) orientation: Orientation,
    pub(super) ratio: Ratio,
    pub(super) first: Node,
    pub(super) second: Node,
}

pub(super) enum Removal {
    Absent,
    Detached,
    Collapsed,
}

impl Split {
    pub(super) fn child(&self, side: Side) -> &Node {
        match side {
            Side::First => &self.first,
            Side::Second => &self.second,
        }
    }

    fn child_mut(&mut self, side: Side) -> &mut Node {
        match side {
            Side::First => &mut self.first,
            Side::Second => &mut self.second,
        }
    }
}

impl Node {
    pub(super) fn collect_windows(&self, windows: &mut Vec<WindowId>) {
        match self {
            Self::Leaf(window) => windows.push(*window),
            Self::Split(split) => {
                split.first.collect_windows(windows);
                split.second.collect_windows(windows);
            }
        }
    }

    pub(super) fn path_to(&self, window: WindowId, path: &mut Vec<Side>) -> bool {
        let split = match self {
            Self::Leaf(leaf) => return *leaf == window,
            Self::Split(split) => split,
        };
        for side in [Side::First, Side::Second] {
            path.push(side);
            if split.child(side).path_to(window, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    pub(super) fn trace_last_leaf(&self, path: &mut Vec<Side>) {
        let mut node = self;
        while let Self::Split(split) = node {
            path.push(Side::Second);
            node = &split.second;
        }
    }

    pub(super) fn descend(&self, path: &[Side]) -> &Self {
        let mut node = self;
        for side in path {
            match node {
                Self::Leaf(_) => break,
                Self::Split(split) => node = split.child(*side),
            }
        }
        node
    }

    pub(super) fn descend_mut(&mut self, path: &[Side]) -> &mut Self {
        let mut node = self;
        for side in path {
            match node {
                Self::Leaf(_) => break,
                Self::Split(split) => node = split.child_mut(*side),
            }
        }
        node
    }

    pub(super) fn nearest_window(&self, orientation: Orientation, origin: Side) -> WindowId {
        match self {
            Self::Leaf(window) => *window,
            Self::Split(split) => {
                let side = if split.orientation == orientation {
                    origin
                } else {
                    Side::First
                };
                split.child(side).nearest_window(orientation, origin)
            }
        }
    }

    pub(super) fn arrange(&self, area: LogicalRect, gap: u32, placements: &mut Vec<Placement>) {
        match self {
            Self::Leaf(window) => placements.push(Placement {
                window: *window,
                area,
            }),
            Self::Split(split) => {
                let (first, second) = area.split(split.orientation, split.ratio, gap);
                split.first.arrange(first, gap, placements);
                split.second.arrange(second, gap, placements);
            }
        }
    }

    pub(super) fn swap(&mut self, first: WindowId, second: WindowId) {
        match self {
            Self::Leaf(leaf) if *leaf == first => *leaf = second,
            Self::Leaf(leaf) if *leaf == second => *leaf = first,
            Self::Leaf(_) => {}
            Self::Split(split) => {
                split.first.swap(first, second);
                split.second.swap(first, second);
            }
        }
    }

    pub(super) fn remove(&mut self, window: WindowId) -> Removal {
        let split = match self {
            Self::Leaf(leaf) if *leaf == window => return Removal::Detached,
            Self::Leaf(_) => return Removal::Absent,
            Self::Split(split) => split,
        };
        let survivor = match split.first.remove(window) {
            Removal::Collapsed => return Removal::Collapsed,
            Removal::Detached => Side::Second,
            Removal::Absent => match split.second.remove(window) {
                Removal::Detached => Side::First,
                outcome => return outcome,
            },
        };
        if let Self::Split(split) = std::mem::replace(self, Self::Leaf(window)) {
            let Split { first, second, .. } = *split;
            *self = match survivor {
                Side::First => first,
                Side::Second => second,
            };
        }
        Removal::Collapsed
    }
}
