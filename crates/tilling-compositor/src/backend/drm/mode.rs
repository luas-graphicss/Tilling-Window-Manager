use tilling_config::ModeConfig;

const REFRESH_TOLERANCE_MILLIHERTZ: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Candidate {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) refresh_millihertz: u32,
    pub(super) preferred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Selection {
    Requested(usize),
    Default(usize),
}

impl Selection {
    pub(super) const fn index(self) -> usize {
        match self {
            Self::Requested(index) | Self::Default(index) => index,
        }
    }
}

pub(super) fn select(candidates: &[Candidate], wanted: Option<ModeConfig>) -> Option<Selection> {
    wanted
        .and_then(|wanted| requested(candidates, wanted))
        .map(Selection::Requested)
        .or_else(|| default_mode(candidates).map(Selection::Default))
}

pub(super) fn default_mode(candidates: &[Candidate]) -> Option<usize> {
    candidates
        .iter()
        .position(|candidate| candidate.preferred)
        .or_else(|| (!candidates.is_empty()).then_some(0))
}

fn requested(candidates: &[Candidate], wanted: ModeConfig) -> Option<usize> {
    let sized = candidates.iter().enumerate().filter(|(_, candidate)| {
        candidate.width == wanted.width && candidate.height == wanted.height
    });
    let chosen = match wanted.refresh_millihertz {
        Some(rate) => sized
            .map(|(index, candidate)| (index, candidate.refresh_millihertz.abs_diff(rate)))
            .filter(|(_, distance)| *distance <= REFRESH_TOLERANCE_MILLIHERTZ)
            .min_by_key(|(_, distance)| *distance),
        None => sized
            .max_by_key(|(_, candidate)| (candidate.preferred, candidate.refresh_millihertz))
            .map(|(index, _)| (index, 0)),
    };
    chosen.map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn mode(width: u32, height: u32, refresh_millihertz: u32, preferred: bool) -> Candidate {
        Candidate {
            width,
            height,
            refresh_millihertz,
            preferred,
        }
    }

    const OFFERED: [Candidate; 5] = [
        mode(2560, 1440, 59_951, true),
        mode(2560, 1440, 143_912, false),
        mode(1920, 1080, 60_000, false),
        mode(1920, 1080, 59_940, false),
        mode(1280, 720, 60_000, false),
    ];

    const fn wanted(
        width: u32,
        height: u32,
        refresh_millihertz: Option<u32>,
    ) -> Option<ModeConfig> {
        Some(ModeConfig {
            width,
            height,
            refresh_millihertz,
        })
    }

    #[test]
    fn preferred_mode_is_used_without_a_request() {
        let unmarked = [
            mode(1024, 768, 60_000, false),
            mode(800, 600, 60_000, false),
        ];

        assert_eq!(select(&OFFERED, None), Some(Selection::Default(0)));
        assert_eq!(select(&unmarked, None), Some(Selection::Default(0)));
        assert_eq!(select(&[], None), None);
    }

    #[test]
    fn requested_rate_picks_the_nearest_mode_of_that_size() {
        let high_rate = select(&OFFERED, wanted(2560, 1440, Some(144_000)));
        let ntsc = select(&OFFERED, wanted(1920, 1080, Some(59_940)));
        let exact = select(&OFFERED, wanted(1920, 1080, Some(60_000)));

        assert_eq!(high_rate, Some(Selection::Requested(1)));
        assert_eq!(ntsc, Some(Selection::Requested(3)));
        assert_eq!(exact, Some(Selection::Requested(2)));
    }

    #[test]
    fn size_without_a_rate_keeps_the_preferred_or_fastest_mode() {
        let native = select(&OFFERED, wanted(2560, 1440, None));
        let full_hd = select(&OFFERED, wanted(1920, 1080, None));

        assert_eq!(native, Some(Selection::Requested(0)));
        assert_eq!(full_hd, Some(Selection::Requested(2)));
    }

    #[test]
    fn mode_that_is_not_offered_falls_back_to_the_default() {
        let unknown_size = select(&OFFERED, wanted(3840, 2160, None));
        let unreachable_rate = select(&OFFERED, wanted(1920, 1080, Some(120_000)));

        assert_eq!(unknown_size, Some(Selection::Default(0)));
        assert_eq!(unreachable_rate, Some(Selection::Default(0)));
    }
}
