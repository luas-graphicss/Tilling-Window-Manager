use tilling_core::geometry::LogicalRect;

const MIN_FLOATING_WIDTH: u32 = 96;
const MIN_FLOATING_HEIGHT: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Edges {
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) top: bool,
    pub(crate) bottom: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Drag {
    Move,
    Resize(Edges),
}

#[derive(Debug, Clone, Copy)]
enum Anchor {
    Near,
    Far,
    Fixed,
}

impl Drag {
    pub(crate) fn apply(self, initial: LogicalRect, dx: i32, dy: i32) -> Option<LogicalRect> {
        let (x, y, width, height) = match self {
            Self::Move => (
                initial.x().checked_add(dx)?,
                initial.y().checked_add(dy)?,
                initial.width(),
                initial.height(),
            ),
            Self::Resize(edges) => {
                let horizontal = Anchor::of(edges.left, edges.right);
                let vertical = Anchor::of(edges.top, edges.bottom);
                let (x, width) =
                    horizontal.stretch(initial.x(), initial.width(), dx, MIN_FLOATING_WIDTH)?;
                let (y, height) =
                    vertical.stretch(initial.y(), initial.height(), dy, MIN_FLOATING_HEIGHT)?;
                (x, y, width, height)
            }
        };
        LogicalRect::new(x, y, width, height).ok()
    }
}

impl Anchor {
    const fn of(near: bool, far: bool) -> Self {
        match (near, far) {
            (true, _) => Self::Near,
            (false, true) => Self::Far,
            (false, false) => Self::Fixed,
        }
    }

    fn stretch(self, origin: i32, length: u32, delta: i32, minimum: u32) -> Option<(i32, u32)> {
        let floor = i64::from(minimum.min(length));
        let (origin, length, delta) = (i64::from(origin), i64::from(length), i64::from(delta));
        let (origin, length) = match self {
            Self::Near => {
                let moved = delta.min(length - floor);
                (origin + moved, length - moved)
            }
            Self::Far => (origin, (length + delta).max(floor)),
            Self::Fixed => (origin, length),
        };
        Some((i32::try_from(origin).ok()?, u32::try_from(length).ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTTOM_RIGHT: Edges = Edges {
        left: false,
        right: true,
        top: false,
        bottom: true,
    };
    const TOP_LEFT: Edges = Edges {
        left: true,
        right: false,
        top: true,
        bottom: false,
    };

    fn logical(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    #[test]
    fn move_keeps_the_size_and_follows_the_pointer() {
        let moved = Drag::Move.apply(logical(100, 80, 400, 300), -30, 45);

        assert_eq!(moved, Some(logical(70, 125, 400, 300)));
    }

    #[test]
    fn far_edges_grow_without_moving_the_origin() {
        let resized = Drag::Resize(BOTTOM_RIGHT).apply(logical(100, 80, 400, 300), 50, -20);

        assert_eq!(resized, Some(logical(100, 80, 450, 280)));
    }

    #[test]
    fn near_edges_move_the_origin_and_keep_the_opposite_edge_in_place() {
        let resized = Drag::Resize(TOP_LEFT).apply(logical(100, 80, 400, 300), 50, -20);

        assert_eq!(resized, Some(logical(150, 60, 350, 320)));
    }

    #[test]
    fn shrinking_stops_at_the_minimum_size() {
        let from_far = Drag::Resize(BOTTOM_RIGHT).apply(logical(100, 80, 400, 300), -900, -900);
        let from_near = Drag::Resize(TOP_LEFT).apply(logical(100, 80, 400, 300), 900, 900);

        assert_eq!(from_far, Some(logical(100, 80, 96, 64)));
        assert_eq!(from_near, Some(logical(404, 316, 96, 64)));
    }

    #[test]
    fn window_already_below_the_minimum_is_not_forced_to_grow() {
        let resized = Drag::Resize(BOTTOM_RIGHT).apply(logical(0, 0, 40, 30), -10, -10);

        assert_eq!(resized, Some(logical(0, 0, 40, 30)));
    }

    #[test]
    fn move_beyond_the_coordinate_space_is_refused() {
        let moved = Drag::Move.apply(logical(i32::MAX - 20, 0, 10, 10), 15, 0);

        assert_eq!(moved, None);
    }
}
