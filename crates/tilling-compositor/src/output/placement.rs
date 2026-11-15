use smithay::utils::{Logical, Physical, Size, Transform};
use tilling_config::{OutputConfig, OutputTransform};
use tilling_core::geometry::{LogicalRect, Scale};

use crate::error::CompositorError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Arrangement {
    pub(super) scale: Scale,
    pub(super) transform: Transform,
    pub(super) area: LogicalRect,
}

impl Arrangement {
    pub(super) fn plan(
        profile: Option<&OutputConfig>,
        size: Size<i32, Physical>,
        suggested_scale: f64,
        occupied: impl IntoIterator<Item = LogicalRect>,
    ) -> Result<Self, CompositorError> {
        let scale = scale_for(profile, suggested_scale);
        let transform = profile
            .and_then(|profile| profile.transform)
            .map_or(Transform::Normal, transform_of);
        let position = profile
            .and_then(|profile| profile.position)
            .map_or_else(|| beside(occupied), |position| (position.x, position.y));
        let area = logical_area(position, size, transform, scale)?;
        Ok(Self {
            scale,
            transform,
            area,
        })
    }
}

pub(super) fn scale_for(profile: Option<&OutputConfig>, suggested_scale: f64) -> Scale {
    profile
        .and_then(|profile| profile.scale)
        .or_else(|| Scale::from_factor(suggested_scale).ok())
        .unwrap_or(Scale::ONE)
}

pub(super) fn logical_area(
    position: (i32, i32),
    size: Size<i32, Physical>,
    transform: Transform,
    scale: Scale,
) -> Result<LogicalRect, CompositorError> {
    let oriented = transform.transform_size(size);
    let logical: Size<i32, Logical> = oriented.to_f64().to_logical(scale.factor()).to_i32_round();
    let unusable = || CompositorError::OutputSize {
        width: size.w.unsigned_abs(),
        height: size.h.unsigned_abs(),
    };
    let width = u32::try_from(logical.w).map_err(|_| unusable())?;
    let height = u32::try_from(logical.h).map_err(|_| unusable())?;
    let area = LogicalRect::new(position.0, position.1, width, height).map_err(|_| unusable())?;
    if area.is_empty() {
        return Err(unusable());
    }
    Ok(area)
}

fn beside(occupied: impl IntoIterator<Item = LogicalRect>) -> (i32, i32) {
    let right_edge = occupied
        .into_iter()
        .map(|area| area.x().saturating_add_unsigned(area.width()))
        .max()
        .unwrap_or(0);
    (right_edge, 0)
}

const fn transform_of(configured: OutputTransform) -> Transform {
    match configured {
        OutputTransform::Normal => Transform::Normal,
        OutputTransform::Rotated90 => Transform::_90,
        OutputTransform::Rotated180 => Transform::_180,
        OutputTransform::Rotated270 => Transform::_270,
        OutputTransform::Flipped => Transform::Flipped,
        OutputTransform::Flipped90 => Transform::Flipped90,
        OutputTransform::Flipped180 => Transform::Flipped180,
        OutputTransform::Flipped270 => Transform::Flipped270,
    }
}

#[cfg(test)]
mod tests {
    use tilling_config::ValidatedConfig;

    use super::*;

    const QHD: (i32, i32) = (2560, 1440);

    fn logical(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    fn profile(settings: &str) -> ValidatedConfig {
        let text = format!("schema_version = 1\n\n[[outputs]]\nname = \"DP-1\"\n{settings}\n");
        tilling_config::parse(&text).unwrap()
    }

    fn planned(settings: &str, occupied: &[LogicalRect]) -> Arrangement {
        let config = profile(settings);
        Arrangement::plan(
            config.outputs().first(),
            Size::from(QHD),
            1.0,
            occupied.iter().copied(),
        )
        .unwrap()
    }

    #[test]
    fn fractional_scales_round_the_logical_size_to_whole_pixels() {
        let sizes: Vec<(u32, u32)> = ["1.0", "1.25", "1.5", "2.0"]
            .into_iter()
            .map(|factor| planned(&format!("scale = {factor}"), &[]).area)
            .map(|area| (area.width(), area.height()))
            .collect();

        assert_eq!(
            sizes,
            [(2560, 1440), (2048, 1152), (1707, 960), (1280, 720)]
        );
    }

    #[test]
    fn quarter_turns_swap_the_logical_axes() {
        let portrait = planned("transform = \"90\"\nscale = 1.5", &[]);
        let upside_down = planned("transform = \"180\"", &[]);

        assert_eq!(portrait.transform, Transform::_90);
        assert_eq!(portrait.area, logical(0, 0, 960, 1707));
        assert_eq!(upside_down.area, logical(0, 0, 2560, 1440));
    }

    #[test]
    fn configured_position_may_lie_left_of_the_origin() {
        let left = planned("position = { x = -2560, y = -200 }", &[]);

        assert_eq!(left.area, logical(-2560, -200, 2560, 1440));
    }

    #[test]
    fn unplaced_output_goes_to_the_right_of_the_occupied_area() {
        let occupied = [logical(-1920, 0, 1920, 1080), logical(0, 0, 1280, 1024)];

        assert_eq!(planned("", &[]).area, logical(0, 0, 2560, 1440));
        assert_eq!(planned("", &occupied).area, logical(1280, 0, 2560, 1440));
    }

    #[test]
    fn suggested_scale_applies_only_without_a_configured_one() {
        let unconfigured = Arrangement::plan(None, Size::from(QHD), 2.0, []).unwrap();
        let out_of_range = Arrangement::plan(None, Size::from(QHD), 0.0, []).unwrap();

        assert_eq!(unconfigured.scale, Scale::from_factor(2.0).unwrap());
        assert_eq!(out_of_range.scale, Scale::ONE);
        assert_eq!(
            planned("scale = 1.5", &[]).scale,
            Scale::from_factor(1.5).unwrap()
        );
    }

    #[test]
    fn size_that_scales_down_to_nothing_is_refused() {
        let outcome = Arrangement::plan(None, Size::from((0, 1440)), 1.0, []);

        assert!(matches!(
            outcome,
            Err(CompositorError::OutputSize {
                width: 0,
                height: 1440
            })
        ));
    }
}
