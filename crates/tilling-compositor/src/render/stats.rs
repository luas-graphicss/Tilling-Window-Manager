use smithay::utils::{Physical, Rectangle};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub presented: u64,
    pub damaged: u64,
    pub damaged_pixels: u64,
}

impl FrameStats {
    pub(crate) fn record(&mut self, damage: Option<&[Rectangle<i32, Physical>]>) {
        self.presented = self.presented.saturating_add(1);
        let Some(damage) = damage else {
            return;
        };
        self.damaged = self.damaged.saturating_add(1);
        self.damaged_pixels = damage.iter().map(pixels).sum();
    }
}

fn pixels(area: &Rectangle<i32, Physical>) -> u64 {
    let covered = i64::from(area.size.w) * i64::from(area.size.h);
    u64::try_from(covered).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: i32, y: i32, width: i32, height: i32) -> Rectangle<i32, Physical> {
        Rectangle::new((x, y).into(), (width, height).into())
    }

    #[test]
    fn frame_without_damage_counts_as_presented_only() {
        let mut stats = FrameStats::default();

        stats.record(Some(&[area(0, 0, 10, 20), area(50, 50, 5, 5)]));
        stats.record(None);

        assert_eq!(
            stats,
            FrameStats {
                presented: 2,
                damaged: 1,
                damaged_pixels: 225
            }
        );
    }
}
