use serde::Serialize;

use super::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Scale(u32);

impl Scale {
    pub const MIN_FACTOR: f64 = 0.25;
    pub const MAX_FACTOR: f64 = 8.0;
    pub const ONE: Self = Self(Self::DENOMINATOR);
    const DENOMINATOR: u32 = 120;

    pub fn from_factor(factor: f64) -> Result<Self, GeometryError> {
        if !factor.is_finite() || !(Self::MIN_FACTOR..=Self::MAX_FACTOR).contains(&factor) {
            return Err(GeometryError::InvalidScale);
        }
        let numerator = (factor * f64::from(Self::DENOMINATOR)).round();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the factor was range-checked, so the numerator lies between 30 and 960"
        )]
        let numerator = numerator as u32;
        Ok(Self(numerator))
    }

    #[must_use]
    pub fn factor(self) -> f64 {
        f64::from(self.0) / f64::from(Self::DENOMINATOR)
    }

    pub(super) fn apply(self, edge: i64) -> i64 {
        let denominator = i64::from(Self::DENOMINATOR);
        (2 * edge * i64::from(self.0) + denominator).div_euclid(2 * denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_common_fractional_factors() {
        for factor in [0.25, 1.0, 1.25, 1.5, 1.75, 2.0, 8.0] {
            let scale = Scale::from_factor(factor).unwrap();
            assert!((scale.factor() - factor).abs() < 1e-9, "factor {factor}");
        }
    }

    #[test]
    fn rejects_factors_that_are_not_finite_positive_and_bounded() {
        for factor in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            0.2,
            8.5,
        ] {
            assert_eq!(Scale::from_factor(factor), Err(GeometryError::InvalidScale));
        }
    }

    #[test]
    fn rounds_half_edges_toward_positive_infinity_on_both_signs() {
        let scale = Scale::from_factor(1.5).unwrap();

        assert_eq!(scale.apply(1), 2);
        assert_eq!(scale.apply(-1), -1);
        assert_eq!(scale.apply(3), 5);
        assert_eq!(scale.apply(-3), -4);
    }
}
