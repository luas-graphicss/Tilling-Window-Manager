use smithay::output::{Mode, Output, Scale as AdvertisedScale};
use smithay::utils::{Point, Transform};
use tilling_core::geometry::{LogicalRect, Scale};

use super::placement::logical_area;
use crate::error::CompositorError;

const REFRESH_TOLERANCE_MILLIHERTZ: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeRequest {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) refresh_millihertz: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OutputChange {
    pub(crate) mode: Option<ModeRequest>,
    pub(crate) position: Option<(i32, i32)>,
    pub(crate) scale: Option<Scale>,
    pub(crate) transform: Option<Transform>,
}

impl OutputChange {
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutputSettings {
    pub(super) mode: Mode,
    pub(super) position: (i32, i32),
    pub(super) scale: Scale,
    pub(super) transform: Transform,
}

impl OutputSettings {
    pub(super) fn of(output: &Output, scale: Scale) -> Option<Self> {
        let location = output.current_location();
        Some(Self {
            mode: output.current_mode()?,
            position: (location.x, location.y),
            scale,
            transform: output.current_transform(),
        })
    }

    pub(super) fn changed_by(self, change: OutputChange, offered: &[Mode]) -> Option<Self> {
        let mode = match change.mode {
            Some(wanted) => offered_mode(offered, wanted)?,
            None => self.mode,
        };
        Some(Self {
            mode,
            position: change.position.unwrap_or(self.position),
            scale: change.scale.unwrap_or(self.scale),
            transform: change.transform.unwrap_or(self.transform),
        })
    }

    pub(super) fn area(self) -> Result<LogicalRect, CompositorError> {
        logical_area(self.position, self.mode.size, self.transform, self.scale)
    }

    pub(super) fn apply_to(self, output: &Output) {
        output.change_current_state(
            Some(self.mode),
            Some(self.transform),
            Some(AdvertisedScale::Fractional(self.scale.factor())),
            Some(Point::from(self.position)),
        );
    }
}

fn offered_mode(offered: &[Mode], wanted: ModeRequest) -> Option<Mode> {
    let sized = offered.iter().copied().filter(|mode| {
        u32::try_from(mode.size.w) == Ok(wanted.width)
            && u32::try_from(mode.size.h) == Ok(wanted.height)
    });
    match wanted.refresh_millihertz {
        Some(rate) => sized
            .filter_map(|mode| {
                let distance = u32::try_from(mode.refresh).ok()?.abs_diff(rate);
                Some((mode, distance))
            })
            .filter(|(_, distance)| *distance <= REFRESH_TOLERANCE_MILLIHERTZ)
            .min_by_key(|(_, distance)| *distance)
            .map(|(mode, _)| mode),
        None => sized.max_by_key(|mode| mode.refresh),
    }
}

#[cfg(test)]
mod tests {
    use smithay::utils::Size;

    use super::*;

    fn mode(width: i32, height: i32, refresh: i32) -> Mode {
        Mode {
            size: Size::from((width, height)),
            refresh,
        }
    }

    fn offered() -> [Mode; 4] {
        [
            mode(2560, 1440, 59_951),
            mode(2560, 1440, 143_912),
            mode(1920, 1080, 60_000),
            mode(1920, 1080, 59_940),
        ]
    }

    fn current() -> OutputSettings {
        OutputSettings {
            mode: mode(2560, 1440, 59_951),
            position: (0, 0),
            scale: Scale::ONE,
            transform: Transform::Normal,
        }
    }

    fn with_mode(width: u32, height: u32, refresh_millihertz: Option<u32>) -> Option<Mode> {
        let change = OutputChange {
            mode: Some(ModeRequest {
                width,
                height,
                refresh_millihertz,
            }),
            ..OutputChange::default()
        };
        current()
            .changed_by(change, &offered())
            .map(|settings| settings.mode)
    }

    #[test]
    fn change_keeps_every_setting_it_does_not_name() {
        let larger = Scale::from_factor(1.5).unwrap();
        let change = OutputChange {
            scale: Some(larger),
            ..OutputChange::default()
        };

        let changed = current().changed_by(change, &offered());

        assert_eq!(
            changed,
            Some(OutputSettings {
                scale: larger,
                ..current()
            })
        );
        assert!(OutputChange::default().is_empty());
        assert!(!change.is_empty());
    }

    #[test]
    fn requested_rate_picks_the_nearest_offered_mode_of_that_size() {
        assert_eq!(
            with_mode(2560, 1440, Some(144_000)),
            Some(mode(2560, 1440, 143_912))
        );
        assert_eq!(
            with_mode(1920, 1080, Some(59_940)),
            Some(mode(1920, 1080, 59_940))
        );
        assert_eq!(with_mode(1920, 1080, None), Some(mode(1920, 1080, 60_000)));
    }

    #[test]
    fn mode_that_is_not_offered_changes_nothing() {
        assert_eq!(with_mode(3840, 2160, None), None);
        assert_eq!(with_mode(1920, 1080, Some(120_000)), None);
    }

    #[test]
    fn area_follows_position_scale_and_rotation() {
        let turned = OutputSettings {
            position: (-960, 40),
            scale: Scale::from_factor(1.5).unwrap(),
            transform: Transform::_90,
            ..current()
        };

        assert_eq!(
            turned.area().unwrap(),
            LogicalRect::new(-960, 40, 960, 1707).unwrap()
        );
    }
}
