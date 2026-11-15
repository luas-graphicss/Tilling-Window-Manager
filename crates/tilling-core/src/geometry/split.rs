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
