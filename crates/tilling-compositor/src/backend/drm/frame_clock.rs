#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Wait {
    Vblank,
    Throttle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Waiting(Wait),
    Halted,
}

#[derive(Debug, Default)]
pub(super) struct FrameClock {
    phase: Phase,
    redraw_wanted: bool,
}

impl FrameClock {
    pub(super) fn request_redraw(&mut self) -> bool {
        match self.phase {
            Phase::Idle => true,
            Phase::Waiting(_) => {
                self.redraw_wanted = true;
                false
            }
            Phase::Halted => false,
        }
    }

    pub(super) fn wait_for(&mut self, wait: Wait) {
        self.phase = Phase::Waiting(wait);
        self.redraw_wanted = false;
    }

    pub(super) fn finish(&mut self, wait: Wait) -> bool {
        if self.phase != Phase::Waiting(wait) {
            return false;
        }
        self.phase = Phase::Idle;
        std::mem::take(&mut self.redraw_wanted)
    }

    pub(super) fn halt(&mut self) {
        self.phase = Phase::Halted;
        self.redraw_wanted = false;
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_output_draws_as_soon_as_it_is_asked() {
        let mut clock = FrameClock::default();

        assert!(clock.request_redraw());
    }

    #[test]
    fn redraw_asked_during_a_page_flip_waits_for_its_vblank() {
        let mut clock = FrameClock::default();
        clock.wait_for(Wait::Vblank);

        let drawn_at_once = clock.request_redraw();
        let drawn_after_vblank = clock.finish(Wait::Vblank);

        assert!(!drawn_at_once);
        assert!(drawn_after_vblank);
        assert!(clock.request_redraw());
    }

    #[test]
    fn vblank_without_a_pending_redraw_leaves_the_output_idle() {
        let mut clock = FrameClock::default();
        clock.wait_for(Wait::Vblank);

        assert!(!clock.finish(Wait::Vblank));
        assert!(clock.request_redraw());
    }

    #[test]
    fn event_of_another_wait_does_not_end_the_current_one() {
        let mut clock = FrameClock::default();
        clock.wait_for(Wait::Vblank);
        clock.request_redraw();

        let ended_by_stale_timer = clock.finish(Wait::Throttle);

        assert!(!ended_by_stale_timer);
        assert!(!clock.request_redraw());
        assert!(clock.finish(Wait::Vblank));
    }

    #[test]
    fn halted_output_stays_dark_until_it_is_reset() {
        let mut clock = FrameClock::default();
        clock.halt();

        let drawn_while_halted = clock.request_redraw();
        let woken_by_vblank = clock.finish(Wait::Vblank);
        clock.reset();

        assert!(!drawn_while_halted);
        assert!(!woken_by_vblank);
        assert!(clock.request_redraw());
    }

    #[test]
    fn reset_forgets_the_wait_and_the_pending_redraw() {
        let mut clock = FrameClock::default();
        clock.wait_for(Wait::Vblank);
        clock.request_redraw();

        clock.reset();

        assert!(!clock.finish(Wait::Vblank));
        assert!(clock.request_redraw());
    }
}
