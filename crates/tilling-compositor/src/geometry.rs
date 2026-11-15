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
    fn centering_clamps_requests_larger_than_the_area() {
        let area = logical(100, 50, 1000, 500);

        assert_eq!(centered(area, 400, 300), Some(logical(400, 150, 400, 300)));
        assert_eq!(centered(area, 4000, 3000), Some(area));
    }
}
