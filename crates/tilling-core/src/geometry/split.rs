use serde::Serialize;

use super::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Orientation {
    SideBySide,
    Stacked,
}

impl Orientation {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::SideBySide => Self::Stacked,
            Self::Stacked => Self::SideBySide,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Ratio(u16);

pub(super) struct Division {
    pub(super) first: u32,
    pub(super) offset: u32,
    pub(super) second: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Segment {
    pub(super) offset: u32,
    pub(super) length: u32,
}

pub(super) fn divide_evenly(length: u32, parts: usize, gap: u32) -> Vec<Segment> {
    let Some(count) = u32::try_from(parts).ok().filter(|count| *count > 0) else {
        return Vec::new();
    };
    let joints = count - 1;
    let gap = if u64::from(gap) * u64::from(joints) < u64::from(length) {
        gap
    } else {
        0
    };
    let available = length - gap * joints;
    let share = available / count;
    let widened = available % count;
    let mut offset = 0_u32;
    (0..count)
        .map(|index| {
            let segment = Segment {
                offset,
                length: share + u32::from(index < widened),
            };
            offset = offset.saturating_add(segment.length).saturating_add(gap);
            segment
        })
        .collect()
}

impl Ratio {
    pub const MIN_BASIS_POINTS: u16 = 1_000;
    pub const MAX_BASIS_POINTS: u16 = 9_000;
    pub const HALF: Self = Self(5_000);
    const WHOLE: u64 = 10_000;

    pub fn from_basis_points(basis_points: u16) -> Result<Self, GeometryError> {
        if (Self::MIN_BASIS_POINTS..=Self::MAX_BASIS_POINTS).contains(&basis_points) {
            Ok(Self(basis_points))
        } else {
            Err(GeometryError::InvalidRatio)
        }
    }

    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }

    #[must_use]
    pub fn adjusted(self, delta: i16) -> Self {
        let shifted = self.0.saturating_add_signed(delta);
        Self(shifted.clamp(Self::MIN_BASIS_POINTS, Self::MAX_BASIS_POINTS))
    }

    pub(super) fn divide(self, length: u32, gap: u32) -> Division {
        let gap = if gap < length { gap } else { 0 };
        let available = length - gap;
        let share = u64::from(available) * u64::from(self.0) / Self::WHOLE;
        let first = u32::try_from(share).unwrap_or(available);
        Division {
            first,
            offset: first + gap,
            second: available - first,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ratios_outside_the_supported_range() {
        assert_eq!(
            Ratio::from_basis_points(999),
            Err(GeometryError::InvalidRatio)
        );
        assert_eq!(
            Ratio::from_basis_points(9_001),
            Err(GeometryError::InvalidRatio)
        );
        assert_eq!(Ratio::from_basis_points(5_000), Ok(Ratio::HALF));
    }

    #[test]
    fn adjustment_saturates_at_the_bounds() {
        assert_eq!(Ratio::HALF.adjusted(i16::MAX).basis_points(), 9_000);
        assert_eq!(Ratio::HALF.adjusted(i16::MIN).basis_points(), 1_000);
        assert_eq!(Ratio::HALF.adjusted(250).basis_points(), 5_250);
    }

    #[test]
    fn even_division_accounts_for_every_unit_of_length() {
        for length in 0..120 {
            for parts in 1..9 {
                for gap in [0, 3, 50] {
                    let segments = divide_evenly(length, parts, gap);
                    let covered: u32 = segments.iter().map(|segment| segment.length).sum();
                    let joints = u32::try_from(parts).unwrap() - 1;
                    let spacing = segments
                        .last()
                        .map_or(0, |last| last.offset + last.length - covered);
                    let longest = segments.iter().map(|segment| segment.length).max();
                    let shortest = segments.iter().map(|segment| segment.length).min();

                    assert_eq!(segments.len(), parts);
                    assert_eq!(covered + spacing, length);
                    assert!(spacing == gap * joints || spacing == 0);
                    assert!(longest.unwrap() - shortest.unwrap() <= 1);
                }
            }
        }
    }

    #[test]
    fn even_division_into_no_parts_is_empty() {
        assert_eq!(divide_evenly(100, 0, 4), Vec::new());
    }

    #[test]
    fn division_accounts_for_every_unit_of_length() {
        for length in 0..200 {
            for gap in 0..12 {
                for basis_points in [1_000, 3_333, 5_000, 6_180, 9_000] {
                    let ratio = Ratio::from_basis_points(basis_points).unwrap();
                    let division = ratio.divide(length, gap);
                    let applied_gap = division.offset - division.first;

                    assert_eq!(division.first + applied_gap + division.second, length);
                    assert!(applied_gap == gap || applied_gap == 0);
                }
            }
        }
    }
}
