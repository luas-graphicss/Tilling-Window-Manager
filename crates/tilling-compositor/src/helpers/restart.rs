use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const RESTARTS_PER_WINDOW: usize = 3;

#[derive(Debug, Default)]
pub(crate) struct RestartBudget {
    spent: Vec<Instant>,
}

impl RestartBudget {
    pub(crate) fn spend(&mut self, now: Instant) -> bool {
        self.spent
            .retain(|earlier| now.saturating_duration_since(*earlier) < WINDOW);
        if self.spent.len() >= RESTARTS_PER_WINDOW {
            return false;
        }
        self.spent.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restarts_in_a_row_are_limited() {
        let mut budget = RestartBudget::default();
        let now = Instant::now();

        let granted: Vec<bool> = (0..5).map(|_| budget.spend(now)).collect();

        assert_eq!(granted, [true, true, true, false, false]);
    }

    #[test]
    fn a_restart_is_granted_again_once_the_window_has_passed() {
        let mut budget = RestartBudget::default();
        let start = Instant::now();
        for _ in 0..RESTARTS_PER_WINDOW {
            budget.spend(start);
        }

        let within = budget.spend(start + WINDOW - Duration::from_secs(1));
        let after = budget.spend(start + WINDOW);

        assert!(!within);
        assert!(after);
    }
}
