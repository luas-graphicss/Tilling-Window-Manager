mod node;

use serde::Serialize;

use self::node::{Node, Removal, Side, Split};
use super::{Direction, LayoutError, Placement};
use crate::geometry::{LogicalRect, Orientation, Ratio};
use crate::identity::WindowId;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(super) struct BspTree {
    root: Option<Node>,
}

impl BspTree {
    pub const MAX_WINDOWS: usize = 1_024;

    #[must_use]
    pub fn windows(&self) -> Vec<WindowId> {
        let mut windows = Vec::new();
        if let Some(root) = &self.root {
            root.collect_windows(&mut windows);
        }
        windows
    }

    #[must_use]
    pub fn contains(&self, window: WindowId) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| root.path_to(window, &mut Vec::new()))
    }

    #[must_use]
    pub fn arrange(&self, area: LogicalRect, gap: u32) -> Vec<Placement> {
        let mut placements = Vec::new();
        if let Some(root) = &self.root {
            root.arrange(area, gap, &mut placements);
        }
        placements
    }

    #[must_use]
    pub fn neighbor(&self, window: WindowId, direction: Direction) -> Option<WindowId> {
        let root = self.root.as_ref()?;
        let mut path = Vec::new();
        if !root.path_to(window, &mut path) {
            return None;
        }
        let (orientation, origin) = crossing(direction);
        (0..path.len()).rev().find_map(|depth| {
            let (ancestors, descent) = path.split_at(depth);
            let Node::Split(split) = root.descend(ancestors) else {
                return None;
            };
            let crosses = split.orientation == orientation && descent.first() == Some(&origin);
            crosses.then(|| {
                split
                    .child(origin.opposite())
                    .nearest_window(orientation, origin)
            })
        })
    }

    pub(crate) fn insert(
        &mut self,
        anchor: Option<WindowId>,
        window: WindowId,
    ) -> Result<(), LayoutError> {
        let Some(root) = self.root.as_mut() else {
            self.root = Some(Node::Leaf(window));
            return Ok(());
        };
        let mut windows = Vec::new();
        root.collect_windows(&mut windows);
        if windows.contains(&window) {
            return Err(LayoutError::AlreadyTiled(window));
        }
        if windows.len() >= Self::MAX_WINDOWS {
            return Err(LayoutError::Full {
                limit: Self::MAX_WINDOWS,
            });
        }
        let mut path = Vec::new();
        if !anchor.is_some_and(|anchor| root.path_to(anchor, &mut path)) {
            root.trace_last_leaf(&mut path);
        }
        let orientation = orientation_below(root, &path);
        let target = root.descend_mut(&path);
        let anchored = std::mem::replace(target, Node::Leaf(window));
        *target = Node::Split(Box::new(Split {
            orientation,
            ratio: Ratio::HALF,
            first: anchored,
            second: Node::Leaf(window),
        }));
        Ok(())
    }

    pub(crate) fn remove(&mut self, window: WindowId) -> bool {
        let Some(root) = self.root.as_mut() else {
            return false;
        };
        match root.remove(window) {
            Removal::Absent => false,
            Removal::Collapsed => true,
            Removal::Detached => {
                self.root = None;
                true
            }
        }
    }

    pub(crate) fn swap(&mut self, first: WindowId, second: WindowId) -> bool {
        if first == second || !self.contains(first) || !self.contains(second) {
            return false;
        }
        if let Some(root) = self.root.as_mut() {
            root.swap(first, second);
        }
        true
    }

    pub(crate) fn toggle_orientation(&mut self, window: WindowId) -> bool {
        let Some((split, _)) = self.parent_split_mut(window) else {
            return false;
        };
        split.orientation = split.orientation.toggled();
        true
    }

    pub(crate) fn resize(&mut self, window: WindowId, delta: i16) -> bool {
        let Some((split, side)) = self.parent_split_mut(window) else {
            return false;
        };
        let growth = match side {
            Side::First => delta,
            Side::Second => delta.saturating_neg(),
        };
        let ratio = split.ratio.adjusted(growth);
        let changed = ratio != split.ratio;
        split.ratio = ratio;
        changed
    }

    fn parent_split_mut(&mut self, window: WindowId) -> Option<(&mut Split, Side)> {
        let root = self.root.as_mut()?;
        let mut path = Vec::new();
        if !root.path_to(window, &mut path) {
            return None;
        }
        let (side, ancestors) = path.split_last()?;
        match root.descend_mut(ancestors) {
            Node::Split(split) => Some((split.as_mut(), *side)),
            Node::Leaf(_) => None,
        }
    }
}

fn crossing(direction: Direction) -> (Orientation, Side) {
    match direction {
        Direction::Left => (Orientation::SideBySide, Side::Second),
        Direction::Right => (Orientation::SideBySide, Side::First),
        Direction::Up => (Orientation::Stacked, Side::Second),
        Direction::Down => (Orientation::Stacked, Side::First),
    }
}

fn orientation_below(root: &Node, path: &[Side]) -> Orientation {
    let Some((_, ancestors)) = path.split_last() else {
        return Orientation::SideBySide;
    };
    match root.descend(ancestors) {
        Node::Split(parent) => parent.orientation.toggled(),
        Node::Leaf(_) => Orientation::SideBySide,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Arena;

    fn windows(count: usize) -> Vec<WindowId> {
        let mut arena: Arena<WindowId, ()> = Arena::new();
        (0..count).map(|_| arena.insert(()).unwrap()).collect()
    }

    fn area(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    fn tree_of(ids: &[WindowId]) -> BspTree {
        let mut tree = BspTree::default();
        let mut anchor = None;
        for id in ids {
            tree.insert(anchor, *id).unwrap();
            anchor = Some(*id);
        }
        tree
    }

    fn area_of(tree: &BspTree, window: WindowId) -> LogicalRect {
        tree.arrange(area(0, 0, 1000, 800), 0)
            .into_iter()
            .find(|placement| placement.window == window)
            .map(|placement| placement.area)
            .unwrap()
    }

    #[test]
    fn first_window_fills_the_whole_area() {
        let ids = windows(1);
        let tree = tree_of(&ids);

        assert_eq!(area_of(&tree, ids[0]), area(0, 0, 1000, 800));
    }

    #[test]
    fn insertion_splits_the_anchor_and_alternates_orientation() {
        let ids = windows(3);
        let tree = tree_of(&ids);

        assert_eq!(area_of(&tree, ids[0]), area(0, 0, 500, 800));
        assert_eq!(area_of(&tree, ids[1]), area(500, 0, 500, 400));
        assert_eq!(area_of(&tree, ids[2]), area(500, 400, 500, 400));
    }

    #[test]
    fn unknown_anchor_falls_back_to_the_last_window() {
        let ids = windows(4);
        let mut tree = tree_of(&ids[..2]);

        tree.insert(Some(ids[3]), ids[2]).unwrap();

        assert_eq!(tree.windows(), ids[..3]);
        assert_eq!(area_of(&tree, ids[2]), area(500, 400, 500, 400));
    }

    #[test]
    fn removing_the_only_window_empties_the_tree() {
        let ids = windows(1);
        let mut tree = tree_of(&ids);

        assert!(tree.remove(ids[0]));
        assert!(tree.windows().is_empty());
        assert!(tree.arrange(area(0, 0, 10, 10), 0).is_empty());
    }

    #[test]
    fn removing_a_root_child_promotes_the_sibling_subtree() {
        let ids = windows(3);
        let mut tree = tree_of(&ids);

        assert!(tree.remove(ids[0]));

        assert_eq!(area_of(&tree, ids[1]), area(0, 0, 1000, 400));
        assert_eq!(area_of(&tree, ids[2]), area(0, 400, 1000, 400));
    }

    #[test]
    fn removing_an_inner_window_collapses_its_split() {
        let ids = windows(3);
        let mut tree = tree_of(&ids);

        assert!(tree.remove(ids[1]));

        assert_eq!(area_of(&tree, ids[0]), area(0, 0, 500, 800));
        assert_eq!(area_of(&tree, ids[2]), area(500, 0, 500, 800));
    }

    #[test]
    fn removing_an_unknown_window_changes_nothing() {
        let ids = windows(3);
        let mut tree = tree_of(&ids[..2]);
        let before = tree.clone();

        assert!(!tree.remove(ids[2]));
        assert_eq!(tree, before);
    }

    #[test]
    fn rejects_duplicates_and_windows_beyond_capacity() {
        let ids = windows(BspTree::MAX_WINDOWS + 1);
        let mut tree = tree_of(&ids[..BspTree::MAX_WINDOWS]);

        assert_eq!(
            tree.insert(None, ids[0]),
            Err(LayoutError::AlreadyTiled(ids[0]))
        );
        assert_eq!(
            tree.insert(None, ids[BspTree::MAX_WINDOWS]),
            Err(LayoutError::Full {
                limit: BspTree::MAX_WINDOWS
            })
        );
    }

    #[test]
    fn neighbors_cross_the_nearest_matching_split() {
        let ids = windows(3);
        let tree = tree_of(&ids);

        assert_eq!(tree.neighbor(ids[0], Direction::Right), Some(ids[1]));
        assert_eq!(tree.neighbor(ids[1], Direction::Left), Some(ids[0]));
        assert_eq!(tree.neighbor(ids[2], Direction::Left), Some(ids[0]));
        assert_eq!(tree.neighbor(ids[1], Direction::Down), Some(ids[2]));
        assert_eq!(tree.neighbor(ids[2], Direction::Up), Some(ids[1]));
        assert_eq!(tree.neighbor(ids[0], Direction::Left), None);
        assert_eq!(tree.neighbor(ids[0], Direction::Up), None);
    }

    #[test]
    fn swapping_exchanges_the_two_tiles() {
        let ids = windows(3);
        let mut tree = tree_of(&ids);
        let (first, last) = (area_of(&tree, ids[0]), area_of(&tree, ids[2]));

        assert!(tree.swap(ids[0], ids[2]));

        assert_eq!(area_of(&tree, ids[0]), last);
        assert_eq!(area_of(&tree, ids[2]), first);
        assert!(!tree.swap(ids[0], ids[0]));
    }

    #[test]
    fn resizing_grows_the_window_on_either_side_of_its_split() {
        let ids = windows(2);
        let mut tree = tree_of(&ids);

        assert!(tree.resize(ids[0], 1_000));
        assert_eq!(area_of(&tree, ids[0]).width(), 600);

        assert!(tree.resize(ids[1], 2_000));
        assert_eq!(area_of(&tree, ids[1]).width(), 600);

        assert!(!tree.resize(ids[1], 0));
    }

    #[test]
    fn toggling_turns_a_side_by_side_split_into_a_stack() {
        let ids = windows(2);
        let mut tree = tree_of(&ids);

        assert!(tree.toggle_orientation(ids[1]));

        assert_eq!(area_of(&tree, ids[0]), area(0, 0, 1000, 400));
        assert_eq!(area_of(&tree, ids[1]), area(0, 400, 1000, 400));
    }

    #[test]
    fn a_single_window_has_no_split_to_adjust() {
        let ids = windows(1);
        let mut tree = tree_of(&ids);

        assert!(!tree.toggle_orientation(ids[0]));
        assert!(!tree.resize(ids[0], 500));
    }

    #[test]
    fn tiles_cover_the_area_without_overlapping() {
        let ids = windows(17);
        let tree = tree_of(&ids);
        let whole = area(-733, 211, 1367, 911);

        let placements = tree.arrange(whole, 0);

        let covered: u64 = placements
            .iter()
            .map(|placement| placement.area.area())
            .sum();
        assert_eq!(covered, whole.area());
        for (index, placement) in placements.iter().enumerate() {
            assert!(whole.contains(&placement.area));
            for other in &placements[index + 1..] {
                assert!(!placement.area.overlaps(&other.area));
            }
        }
    }
}
