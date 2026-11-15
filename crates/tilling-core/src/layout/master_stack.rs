use serde::Serialize;

use super::{Direction, Placement};
use crate::geometry::{LogicalRect, Orientation, Ratio};
use crate::identity::WindowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(super) struct MasterStack {
    orientation: Orientation,
    ratio: Ratio,
}

enum Step {
    TowardMaster,
    TowardStack,
    EarlierInStack,
    LaterInStack,
}

impl Default for MasterStack {
    fn default() -> Self {
        Self {
            orientation: Orientation::SideBySide,
            ratio: Ratio::HALF,
        }
    }
}

impl MasterStack {
    pub(super) fn arrange(
        &self,
        order: &[WindowId],
        area: LogicalRect,
        gap: u32,
    ) -> Vec<Placement> {
        let Some((master, stack)) = order.split_first() else {
            return Vec::new();
        };
        if stack.is_empty() {
            return vec![Placement {
                window: *master,
                area,
            }];
        }
        let (master_area, stack_area) = area.split(self.orientation, self.ratio, gap);
        let stack_areas = stack_area.divide(self.orientation.toggled(), stack.len(), gap);
        let mut placements = Vec::with_capacity(order.len());
        placements.push(Placement {
            window: *master,
            area: master_area,
        });
        placements.extend(
            stack
                .iter()
                .zip(stack_areas)
                .map(|(window, area)| Placement {
                    window: *window,
                    area,
                }),
        );
        placements
    }

    pub(super) fn neighbor(
        &self,
        order: &[WindowId],
        window: WindowId,
        direction: Direction,
    ) -> Option<WindowId> {
        let position = order.iter().position(|candidate| *candidate == window)?;
        let target = match (position, self.step(direction)) {
            (0, Step::TowardStack) => 1,
            (0, _) | (_, Step::TowardStack) | (1, Step::EarlierInStack) => return None,
            (_, Step::TowardMaster) => 0,
            (position, Step::EarlierInStack) => position - 1,
            (position, Step::LaterInStack) => position + 1,
        };
        order.get(target).copied()
    }

    pub(super) fn toggle_orientation(&mut self, order: &[WindowId], window: WindowId) -> bool {
        if order.len() < 2 || !order.contains(&window) {
            return false;
        }
        self.orientation = self.orientation.toggled();
        true
    }

    pub(super) fn resize(&mut self, order: &[WindowId], window: WindowId, delta: i16) -> bool {
        if order.len() < 2 {
            return false;
        }
        let growth = match order.iter().position(|candidate| *candidate == window) {
            Some(0) => delta,
            Some(_) => delta.saturating_neg(),
            None => return false,
        };
        let ratio = self.ratio.adjusted(growth);
        let changed = ratio != self.ratio;
        self.ratio = ratio;
        changed
    }

    fn step(&self, direction: Direction) -> Step {
        match (self.orientation, direction) {
            (Orientation::SideBySide, Direction::Left) | (Orientation::Stacked, Direction::Up) => {
                Step::TowardMaster
            }
            (Orientation::SideBySide, Direction::Right)
            | (Orientation::Stacked, Direction::Down) => Step::TowardStack,
            (Orientation::SideBySide, Direction::Up) | (Orientation::Stacked, Direction::Left) => {
                Step::EarlierInStack
            }
            (Orientation::SideBySide, Direction::Down)
            | (Orientation::Stacked, Direction::Right) => Step::LaterInStack,
        }
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

    fn areas(layout: &MasterStack, order: &[WindowId]) -> Vec<LogicalRect> {
        layout
            .arrange(order, area(0, 0, 1000, 900), 0)
            .into_iter()
            .map(|placement| placement.area)
            .collect()
    }

    #[test]
    fn a_single_window_fills_the_whole_area() {
        let order = windows(1);

        assert_eq!(
            areas(&MasterStack::default(), &order),
            [area(0, 0, 1000, 900)]
        );
        assert!(areas(&MasterStack::default(), &[]).is_empty());
    }

    #[test]
    fn master_takes_one_side_and_the_stack_shares_the_other() {
        let order = windows(4);

        assert_eq!(
            areas(&MasterStack::default(), &order),
            [
                area(0, 0, 500, 900),
                area(500, 0, 500, 300),
                area(500, 300, 500, 300),
                area(500, 600, 500, 300),
            ]
        );
    }

    #[test]
    fn toggling_puts_the_master_above_a_row_of_windows() {
        let order = windows(3);
        let mut layout = MasterStack::default();

        assert!(layout.toggle_orientation(&order, order[2]));

        assert_eq!(
            areas(&layout, &order),
            [
                area(0, 0, 1000, 450),
                area(0, 450, 500, 450),
                area(500, 450, 500, 450),
            ]
        );
    }

    #[test]
    fn gaps_separate_the_master_and_every_stacked_window() {
        let order = windows(3);

        let placements = MasterStack::default().arrange(&order, area(0, 0, 1010, 910), 10);
        let placed: Vec<LogicalRect> = placements.iter().map(|placement| placement.area).collect();

        assert_eq!(
            placed,
            [
                area(0, 0, 500, 910),
                area(510, 0, 500, 450),
                area(510, 460, 500, 450),
            ]
        );
    }

    #[test]
    fn tiles_cover_the_area_without_overlapping() {
        let order = windows(13);
        let whole = area(-733, 211, 1367, 911);

        let placements = MasterStack::default().arrange(&order, whole, 0);

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

    #[test]
    fn neighbors_follow_the_master_and_stack_positions() {
        let order = windows(4);
        let layout = MasterStack::default();
        let neighbor = |index: usize, direction| layout.neighbor(&order, order[index], direction);

        assert_eq!(neighbor(0, Direction::Right), Some(order[1]));
        assert_eq!(neighbor(3, Direction::Left), Some(order[0]));
        assert_eq!(neighbor(1, Direction::Down), Some(order[2]));
        assert_eq!(neighbor(3, Direction::Up), Some(order[2]));
        assert_eq!(neighbor(0, Direction::Left), None);
        assert_eq!(neighbor(0, Direction::Down), None);
        assert_eq!(neighbor(1, Direction::Up), None);
        assert_eq!(neighbor(3, Direction::Down), None);
        assert_eq!(neighbor(2, Direction::Right), None);
    }

    #[test]
    fn neighbors_turn_with_the_orientation() {
        let order = windows(3);
        let mut layout = MasterStack::default();
        layout.toggle_orientation(&order, order[0]);
        let neighbor = |index: usize, direction| layout.neighbor(&order, order[index], direction);

        assert_eq!(neighbor(0, Direction::Down), Some(order[1]));
        assert_eq!(neighbor(2, Direction::Up), Some(order[0]));
        assert_eq!(neighbor(1, Direction::Right), Some(order[2]));
        assert_eq!(neighbor(1, Direction::Left), None);
    }

    #[test]
    fn resizing_grows_the_side_of_the_resized_window() {
        let order = windows(3);
        let mut layout = MasterStack::default();

        assert!(layout.resize(&order, order[0], 1_000));
        assert_eq!(areas(&layout, &order)[0].width(), 600);

        assert!(layout.resize(&order, order[2], 2_000));
        assert_eq!(areas(&layout, &order)[0].width(), 400);

        assert!(!layout.resize(&order, order[1], 0));
    }

    #[test]
    fn a_lone_or_unknown_window_changes_nothing() {
        let order = windows(3);
        let mut layout = MasterStack::default();

        assert!(!layout.resize(&order[..1], order[0], 500));
        assert!(!layout.toggle_orientation(&order[..1], order[0]));
        assert!(!layout.resize(&order[..2], order[2], 500));
        assert!(!layout.toggle_orientation(&order[..2], order[2]));
        assert_eq!(layout, MasterStack::default());
    }
}
