use std::collections::VecDeque;

use smithay::utils::Serial;

const REMEMBERED_SERIALS: usize = 16;

#[derive(Debug, Default)]
pub(crate) struct InputSerials {
    recent: VecDeque<Serial>,
}

impl InputSerials {
    pub(crate) fn record(&mut self, serial: Serial) {
        if self.recent.len() == REMEMBERED_SERIALS {
            self.recent.pop_front();
        }
        self.recent.push_back(serial);
    }

    pub(crate) fn contains(&self, serial: Serial) -> bool {
        self.recent.contains(&serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_most_recent_serials_authorize_a_request() {
        let mut serials = InputSerials::default();

        for value in 1..=20 {
            serials.record(Serial::from(value));
        }

        assert!(serials.contains(Serial::from(20)));
        assert!(serials.contains(Serial::from(5)));
        assert!(!serials.contains(Serial::from(4)));
        assert!(!serials.contains(Serial::from(21)));
    }
}
