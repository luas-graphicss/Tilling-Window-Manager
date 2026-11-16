use std::marker::PhantomData;

use serde::Serialize;

use super::split::divide_evenly;
use super::{GeometryError, Insets, Orientation, Ratio, Scale};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Logical;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Physical;

pub type LogicalRect = Rect<Logical>;
pub type PhysicalRect = Rect<Physical>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Rect<Unit> {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    #[serde(skip)]
    unit: PhantomData<Unit>,
}

impl<Unit> Rect<Unit> {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Result<Self, GeometryError> {
        let rect = Self {
            x,
            y,
            width,
            height,
            unit: PhantomData,
        };
        if i32::try_from(rect.right()).is_err() || i32::try_from(rect.bottom()).is_err() {
            return Err(GeometryError::OutOfBounds);
        }
        Ok(rect)
    }

    #[must_use]
    pub const fn x(&self) -> i32 {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> i32 {
        self.y
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    #[must_use]
    pub fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    #[must_use]
    pub fn area(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && i64::from(self.x) < other.right()
            && i64::from(other.x) < self.right()
            && i64::from(self.y) < other.bottom()
            && i64::from(other.y) < self.bottom()
    }

    #[must_use]
    pub fn shrink(&self, margin: u32) -> Self {
        let horizontal = margin.min(self.width / 2);
        let vertical = margin.min(self.height / 2);
        Self {
            x: self.x.saturating_add_unsigned(horizontal),
            y: self.y.saturating_add_unsigned(vertical),
            width: self.width - 2 * horizontal,
            height: self.height - 2 * vertical,
            unit: PhantomData,
        }
    }

    #[must_use]
    pub fn inset(&self, insets: Insets) -> Option<Self> {
        let horizontal = insets.left.checked_add(insets.right)?;
        let vertical = insets.top.checked_add(insets.bottom)?;
        let inner = Self {
            x: self.x.checked_add_unsigned(insets.left)?,
            y: self.y.checked_add_unsigned(insets.top)?,
            width: self.width.checked_sub(horizontal)?,
            height: self.height.checked_sub(vertical)?,
            unit: PhantomData,
        };
        (!inner.is_empty()).then_some(inner)
    }

    #[must_use]
    pub fn fitted_within(&self, bounds: &Self) -> Self {
        if bounds.contains(self) {
            return Self { ..*self };
        }
        let width = self.width.min(bounds.width);
        let height = self.height.min(bounds.height);
        Self {
            x: bounds.x.saturating_add_unsigned((bounds.width - width) / 2),
            y: bounds
                .y
                .saturating_add_unsigned((bounds.height - height) / 2),
            width,
            height,
            unit: PhantomData,
        }
    }

    #[must_use]
    pub fn split(&self, orientation: Orientation, ratio: Ratio, gap: u32) -> (Self, Self) {
        match orientation {
            Orientation::SideBySide => {
                let division = ratio.divide(self.width, gap);
                let first = Self {
                    width: division.first,
                    ..*self
                };
                let second = Self {
                    x: self.x.saturating_add_unsigned(division.offset),
                    width: division.second,
                    ..*self
                };
                (first, second)
            }
            Orientation::Stacked => {
                let division = ratio.divide(self.height, gap);
                let first = Self {
                    height: division.first,
                    ..*self
                };
                let second = Self {
                    y: self.y.saturating_add_unsigned(division.offset),
                    height: division.second,
                    ..*self
                };
                (first, second)
            }
        }
    }

    #[must_use]
    pub fn divide(&self, orientation: Orientation, parts: usize, gap: u32) -> Vec<Self> {
        let length = match orientation {
            Orientation::SideBySide => self.width,
            Orientation::Stacked => self.height,
        };
        divide_evenly(length, parts, gap)
            .into_iter()
            .map(|segment| match orientation {
                Orientation::SideBySide => Self {
                    x: self.x.saturating_add_unsigned(segment.offset),
                    width: segment.length,
                    ..*self
                },
                Orientation::Stacked => Self {
                    y: self.y.saturating_add_unsigned(segment.offset),
                    height: segment.length,
                    ..*self
                },
            })
            .collect()
    }
}

impl Rect<Logical> {
    pub fn to_physical(&self, scale: Scale) -> Result<Rect<Physical>, GeometryError> {
        let left = scale.apply(i64::from(self.x));
        let top = scale.apply(i64::from(self.y));
        let width = scale.apply(self.right()) - left;
        let height = scale.apply(self.bottom()) - top;
        Rect::new(
            i32::try_from(left).map_err(|_| GeometryError::OutOfBounds)?,
            i32::try_from(top).map_err(|_| GeometryError::OutOfBounds)?,
            u32::try_from(width).map_err(|_| GeometryError::OutOfBounds)?,
            u32::try_from(height).map_err(|_| GeometryError::OutOfBounds)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logical(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    #[test]
    fn rejects_edges_beyond_the_coordinate_space() {
        assert_eq!(
            LogicalRect::new(i32::MAX, 0, 1, 1),
            Err(GeometryError::OutOfBounds)
        );
        assert_eq!(
            LogicalRect::new(0, 1, 1, u32::MAX),
            Err(GeometryError::OutOfBounds)
        );
        assert!(LogicalRect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX).is_ok());
    }

    #[test]
    fn shrinking_never_inverts_a_small_rectangle() {
        let shrunk = logical(10, 10, 5, 100).shrink(8);

        assert_eq!(shrunk, logical(12, 18, 1, 84));
        assert_eq!(logical(0, 0, 0, 0).shrink(8), logical(0, 0, 0, 0));
    }

    #[test]
    fn fitting_keeps_a_contained_rectangle_and_centers_any_other() {
        let bounds = logical(1920, 0, 1280, 720);

        assert_eq!(
            logical(2000, 100, 400, 300).fitted_within(&bounds),
            logical(2000, 100, 400, 300)
        );
        assert_eq!(
            logical(100, 100, 400, 300).fitted_within(&bounds),
            logical(2360, 210, 400, 300)
        );
        assert_eq!(logical(0, 0, 4000, 300).fitted_within(&bounds), {
            logical(1920, 210, 1280, 300)
        });
    }

    #[test]
    fn split_halves_tile_the_original_rectangle() {
        let whole = logical(-300, -200, 1001, 777);
        for orientation in [Orientation::SideBySide, Orientation::Stacked] {
            for gap in [0, 7] {
                let (first, second) = whole.split(orientation, Ratio::HALF, gap);
                let gap_area = match orientation {
                    Orientation::SideBySide => u64::from(gap) * u64::from(whole.height()),
                    Orientation::Stacked => u64::from(gap) * u64::from(whole.width()),
                };

                assert!(whole.contains(&first) && whole.contains(&second));
                assert!(!first.overlaps(&second));
                assert_eq!(first.area() + second.area() + gap_area, whole.area());
            }
        }
    }

    #[test]
    fn even_parts_tile_the_original_rectangle() {
        let whole = logical(-300, -200, 1001, 777);
        for orientation in [Orientation::SideBySide, Orientation::Stacked] {
            for count in 1..7 {
                let parts = whole.divide(orientation, count, 0);
                let covered: u64 = parts.iter().map(Rect::area).sum();

                assert_eq!(parts.len(), count);
                assert_eq!(covered, whole.area());
                for (index, part) in parts.iter().enumerate() {
                    assert!(whole.contains(part));
                    assert!(parts[index + 1..].iter().all(|other| !part.overlaps(other)));
                }
            }
        }
    }

    #[test]
    fn even_parts_keep_the_gap_between_them() {
        let parts = logical(0, 10, 100, 90).divide(Orientation::Stacked, 3, 6);

        assert_eq!(
            parts,
            [
                logical(0, 10, 100, 26),
                logical(0, 42, 100, 26),
                logical(0, 74, 100, 26),
            ]
        );
    }

    #[test]
    fn gap_is_dropped_when_it_does_not_fit() {
        let (first, second) = logical(0, 0, 4, 10).split(Orientation::SideBySide, Ratio::HALF, 4);

        assert_eq!(first.width() + second.width(), 4);
        assert_eq!(second.x(), 2);
    }

    #[test]
    fn fractional_scaling_keeps_neighbours_adjacent() {
        let whole = logical(-1921, -37, 1921, 1081);
        for factor in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let scale = Scale::from_factor(factor).unwrap();
            for orientation in [Orientation::SideBySide, Orientation::Stacked] {
                let (first, second) = whole.split(orientation, Ratio::HALF, 0);
                let physical_whole = whole.to_physical(scale).unwrap();
                let physical_first = first.to_physical(scale).unwrap();
                let physical_second = second.to_physical(scale).unwrap();

                assert!(
                    !physical_first.overlaps(&physical_second),
                    "factor {factor}"
                );
                assert_eq!(
                    physical_first.area() + physical_second.area(),
                    physical_whole.area(),
                    "factor {factor}"
                );
            }
        }
    }

    #[test]
    fn scaling_reports_results_outside_the_coordinate_space() {
        let scale = Scale::from_factor(8.0).unwrap();

        assert_eq!(
            logical(i32::MAX - 10, 0, 10, 10).to_physical(scale),
            Err(GeometryError::OutOfBounds)
        );
    }
}
