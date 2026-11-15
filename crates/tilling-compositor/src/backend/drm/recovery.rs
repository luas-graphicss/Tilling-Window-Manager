use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Recovery {
    attempts: u32,
    limit: u32,
    first_delay: Duration,
}

impl Recovery {
    pub(super) const fn new(limit: u32, first_delay: Duration) -> Self {
        Self {
            attempts: 0,
            limit,
            first_delay,
        }
    }

    pub(super) fn next_delay(&mut self) -> Option<Duration> {
        if self.attempts >= self.limit {
            return None;
        }
        let delay = self
            .first_delay
            .saturating_mul(2_u32.saturating_pow(self.attempts));
        self.attempts += 1;
        Some(delay)
    }

    pub(super) fn succeeded(&mut self) {
        self.attempts = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST_DELAY: Duration = Duration::from_millis(250);

    #[test]
    fn delays_double_until_the_attempts_run_out() {
        let mut recovery = Recovery::new(3, FIRST_DELAY);

        let delays: Vec<Option<Duration>> = (0..5).map(|_| recovery.next_delay()).collect();

        assert_eq!(
            delays,
            [
                Some(Duration::from_millis(250)),
                Some(Duration::from_millis(500)),
                Some(Duration::from_secs(1)),
                None,
                None
            ]
        );
    }

    #[test]
    fn success_restores_every_attempt() {
        let mut recovery = Recovery::new(2, FIRST_DELAY);
        recovery.next_delay();
        recovery.next_delay();

        recovery.succeeded();

        assert_eq!(recovery.next_delay(), Some(FIRST_DELAY));
    }

    #[test]
    fn huge_backoff_saturates_instead_of_overflowing() {
        let mut recovery = Recovery::new(u32::MAX, Duration::MAX);

        recovery.next_delay();

        assert_eq!(recovery.next_delay(), Some(Duration::MAX));
    }
}
