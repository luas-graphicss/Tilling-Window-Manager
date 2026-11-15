use smithay::utils::{Logical, Point, Rectangle, Size};
use tilling_core::geometry::LogicalRect;

pub(crate) fn rectangle(rect: LogicalRect) -> Rectangle<i32, Logical> {
    Rectangle::new(Point::from((rect.x(), rect.y())), size(rect))
}

pub(crate) fn size(rect: LogicalRect) -> Size<i32, Logical> {
    Size::from((saturate(rect.width()), saturate(rect.height())))
}

pub(crate) fn centered(within: LogicalRect, width: u32, height: u32) -> Option<LogicalRect> {
    let width = width.min(within.width());
    let height = height.min(within.height());
    let x = within
        .x()
        .checked_add_unsigned((within.width() - width) / 2)?;
    let y = within
        .y()
        .checked_add_unsigned((within.height() - height) / 2)?;
    LogicalRect::new(x, y, width, height).ok()
}

pub(crate) fn nearest_inside(
    areas: impl IntoIterator<Item = LogicalRect>,
    location: Point<f64, Logical>,
) -> Option<Point<f64, Logical>> {
    areas
        .into_iter()
        .map(|area| clamped_into(area, location))
        .min_by(|first, second| {
            squared_distance(*first, location).total_cmp(&squared_distance(*second, location))
        })
}

fn clamped_into(area: LogicalRect, location: Point<f64, Logical>) -> Point<f64, Logical> {
    let bounds = rectangle(area).to_f64();
    let last_column = bounds.loc.x + (bounds.size.w - 1.0).max(0.0);
    let last_row = bounds.loc.y + (bounds.size.h - 1.0).max(0.0);
    Point::from((
        location.x.clamp(bounds.loc.x, last_column),
        location.y.clamp(bounds.loc.y, last_row),
    ))
}

fn squared_distance(first: Point<f64, Logical>, second: Point<f64, Logical>) -> f64 {
    let offset = first - second;
    offset.x * offset.x + offset.y * offset.y
}

fn saturate(length: u32) -> i32 {
    i32::try_from(length).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logical(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    #[test]
    fn rectangle_keeps_origin_and_size() {
        let converted = rectangle(logical(-40, 12, 800, 600));

        assert_eq!(converted.loc, Point::from((-40, 12)));
        assert_eq!(converted.size, Size::from((800, 600)));
    }

    #[test]
    fn sizes_beyond_the_protocol_range_saturate() {
        let huge = logical(i32::MIN, 0, u32::MAX, 600);

        assert_eq!(size(huge), Size::from((i32::MAX, 600)));
    }

    #[test]
    fn location_inside_an_area_is_kept() {
        let areas = [logical(0, 0, 1280, 720), logical(1280, 0, 1920, 1080)];
        let inside = Point::from((2000.5, 900.25));

        assert_eq!(nearest_inside(areas, inside), Some(inside));
    }

    #[test]
    fn location_outside_every_area_moves_to_the_closest_edge() {
        let areas = [logical(0, 0, 1280, 720), logical(1280, 0, 1920, 1080)];

        let below_the_first = nearest_inside(areas, Point::from((100.0, 900.0)));
        let right_of_the_second = nearest_inside(areas, Point::from((9000.0, 40.0)));

        assert_eq!(below_the_first, Some(Point::from((100.0, 719.0))));
        assert_eq!(right_of_the_second, Some(Point::from((3199.0, 40.0))));
    }

    #[test]
    fn no_area_leaves_nowhere_to_move_to() {
        assert_eq!(nearest_inside([], Point::from((10.0, 10.0))), None);
    }

    #[test]
    fn centering_clamps_requests_larger_than_the_area() {
        let area = logical(100, 50, 1000, 500);

        assert_eq!(centered(area, 400, 300), Some(logical(400, 150, 400, 300)));
        assert_eq!(centered(area, 4000, 3000), Some(area));
    }
}
