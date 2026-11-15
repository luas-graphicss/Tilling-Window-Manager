use super::{Direction, Placement};
use crate::geometry::LogicalRect;
use crate::identity::WindowId;

pub(super) fn arrange(
    order: &[WindowId],
    front: Option<WindowId>,
    area: LogicalRect,
) -> Vec<Placement> {
    front
        .filter(|window| order.contains(window))
        .map(|window| Placement { window, area })
        .into_iter()
        .collect()
}

pub(super) fn neighbor(
    order: &[WindowId],
    window: WindowId,
    direction: Direction,
) -> Option<WindowId> {
    let position = order.iter().position(|candidate| *candidate == window)?;
    if order.len() < 2 {
        return None;
    }
    let target = match direction {
        Direction::Right | Direction::Down => (position + 1) % order.len(),
        Direction::Left | Direction::Up => (position + order.len() - 1) % order.len(),
    };
    order.get(target).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Arena;

    fn windows(count: usize) -> Vec<WindowId> {
        let mut arena: Arena<WindowId, ()> = Arena::new();
        (0..count).map(|_| arena.insert(()).unwrap()).collect()
    }

    fn area() -> LogicalRect {
        LogicalRect::new(0, 0, 800, 600).unwrap()
    }

    #[test]
    fn only_the_front_window_is_placed_over_the_whole_area() {
        let order = windows(3);

        let placements = arrange(&order, Some(order[1]), area());

        assert_eq!(
            placements,
            [Placement {
                window: order[1],
                area: area(),
            }]
        );
    }

    #[test]
    fn nothing_is_placed_without_a_tiled_front_window() {
        let order = windows(3);

        assert!(arrange(&order[..2], Some(order[2]), area()).is_empty());
        assert!(arrange(&order, None, area()).is_empty());
    }

    #[test]
    fn neighbors_cycle_through_the_windows_in_order() {
        let order = windows(3);

        assert_eq!(neighbor(&order, order[0], Direction::Right), Some(order[1]));
        assert_eq!(neighbor(&order, order[2], Direction::Down), Some(order[0]));
        assert_eq!(neighbor(&order, order[0], Direction::Left), Some(order[2]));
        assert_eq!(neighbor(&order, order[1], Direction::Up), Some(order[0]));
    }

    #[test]
    fn a_lone_or_unknown_window_has_no_neighbor() {
        let order = windows(2);

        assert_eq!(neighbor(&order[..1], order[0], Direction::Right), None);
        assert_eq!(neighbor(&order[..1], order[1], Direction::Right), None);
    }
}
