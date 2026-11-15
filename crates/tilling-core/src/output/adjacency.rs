use crate::geometry::LogicalRect;
use crate::identity::OutputId;
use crate::layout::Direction;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Reach {
    misaligned: bool,
    gap: i64,
    drift: i64,
}

struct Span {
    start: i64,
    end: i64,
}

pub(crate) fn nearest(
    origin: LogicalRect,
    direction: Direction,
    candidates: impl Iterator<Item = (OutputId, LogicalRect)>,
) -> Option<OutputId> {
    candidates
        .filter_map(|(output, area)| Some((reach(origin, area, direction)?, output)))
        .min()
        .map(|(_, output)| output)
}

fn reach(origin: LogicalRect, target: LogicalRect, direction: Direction) -> Option<Reach> {
    let (gap, origin_span, target_span) = match direction {
        Direction::Left => (
            i64::from(origin.x()) - target.right(),
            rows(origin),
            rows(target),
        ),
        Direction::Right => (
            i64::from(target.x()) - origin.right(),
            rows(origin),
            rows(target),
        ),
        Direction::Up => (
            i64::from(origin.y()) - target.bottom(),
            columns(origin),
            columns(target),
        ),
        Direction::Down => (
            i64::from(target.y()) - origin.bottom(),
            columns(origin),
            columns(target),
        ),
    };
    (gap >= 0).then(|| Reach {
        misaligned: origin_span.start >= target_span.end || target_span.start >= origin_span.end,
        gap,
        drift: (origin_span.start + origin_span.end - target_span.start - target_span.end).abs(),
    })
}

fn rows(area: LogicalRect) -> Span {
    Span {
        start: i64::from(area.y()),
        end: area.bottom(),
    }
}

fn columns(area: LogicalRect) -> Span {
    Span {
        start: i64::from(area.x()),
        end: area.right(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Arena;

    fn outputs(count: usize) -> Vec<OutputId> {
        let mut arena: Arena<OutputId, ()> = Arena::new();
        (0..count).map(|_| arena.insert(()).unwrap()).collect()
    }

    fn area(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    #[test]
    fn only_outputs_past_the_edge_are_reachable() {
        let ids = outputs(2);
        let origin = area(0, 0, 1920, 1080);
        let others = [
            (ids[0], area(1920, 0, 1920, 1080)),
            (ids[1], area(0, 1080, 1920, 1080)),
        ];
        let toward = |direction| nearest(origin, direction, others.into_iter());

        assert_eq!(toward(Direction::Right), Some(ids[0]));
        assert_eq!(toward(Direction::Down), Some(ids[1]));
        assert_eq!(toward(Direction::Left), None);
        assert_eq!(toward(Direction::Up), None);
    }

    #[test]
    fn an_aligned_output_wins_over_a_closer_diagonal_one() {
        let ids = outputs(2);
        let origin = area(0, 0, 1000, 1000);
        let others = [
            (ids[0], area(1000, 2000, 500, 500)),
            (ids[1], area(1500, 0, 500, 1000)),
        ];

        let right = nearest(origin, Direction::Right, others.into_iter());

        assert_eq!(right, Some(ids[1]));
    }

    #[test]
    fn the_nearest_aligned_output_wins_and_ties_fall_to_the_best_centered() {
        let ids = outputs(3);
        let origin = area(0, 0, 1000, 1000);
        let stacked = [
            (ids[0], area(3000, 0, 1000, 1000)),
            (ids[1], area(1000, 0, 1000, 1000)),
        ];
        let side_by_side = [
            (ids[0], area(1000, 600, 1000, 1000)),
            (ids[2], area(1000, -100, 1000, 700)),
        ];

        assert_eq!(
            nearest(origin, Direction::Right, stacked.into_iter()),
            Some(ids[1])
        );
        assert_eq!(
            nearest(origin, Direction::Right, side_by_side.into_iter()),
            Some(ids[2])
        );
    }
}
