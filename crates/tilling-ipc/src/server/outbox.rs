use std::collections::VecDeque;
use std::io::{self, Write};

use crate::framing::MAX_FRAME_BYTES;

pub const MAX_QUEUED_EVENTS: usize = 256;
pub const MAX_QUEUED_BYTES: usize = 4 * MAX_FRAME_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum Overflow {
    #[error("subscriber has {MAX_QUEUED_EVENTS} undelivered events")]
    Events,
    #[error("peer has {MAX_QUEUED_BYTES} undelivered bytes")]
    Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Priority {
    Reply,
    Event,
    Final,
}

#[derive(Debug)]
struct Queued {
    frame: Vec<u8>,
    priority: Priority,
}

#[derive(Debug, Default)]
pub(super) struct Outbox {
    queue: VecDeque<Queued>,
    written: usize,
    queued_events: usize,
    queued_bytes: usize,
}

impl Outbox {
    pub(super) fn push(&mut self, frame: Vec<u8>, priority: Priority) -> Result<(), Overflow> {
        if priority != Priority::Final {
            self.admit(frame.len(), priority)?;
        }
        if priority == Priority::Event {
            self.queued_events += 1;
        }
        self.queued_bytes += frame.len();
        self.queue.push_back(Queued { frame, priority });
        Ok(())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(super) fn drain_into(&mut self, sink: &mut impl Write) -> io::Result<bool> {
        while let Some(front) = self.queue.front() {
            let unsent = front.frame.get(self.written..).unwrap_or_default();
            match sink.write(unsent) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => self.advance(count),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }

    fn admit(&self, length: usize, priority: Priority) -> Result<(), Overflow> {
        if priority == Priority::Event && self.queued_events >= MAX_QUEUED_EVENTS {
            return Err(Overflow::Events);
        }
        if self.queued_bytes.saturating_add(length) > MAX_QUEUED_BYTES {
            return Err(Overflow::Bytes);
        }
        Ok(())
    }

    fn advance(&mut self, count: usize) {
        self.written += count;
        self.queued_bytes = self.queued_bytes.saturating_sub(count);
        let Some(front) = self.queue.front() else {
            return;
        };
        if self.written < front.frame.len() {
            return;
        }
        if front.priority == Priority::Event {
            self.queued_events = self.queued_events.saturating_sub(1);
        }
        self.queue.pop_front();
        self.written = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Throttled {
        accepted: Vec<u8>,
        budget: usize,
    }

    impl Write for Throttled {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.budget == 0 {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let count = bytes.len().min(self.budget);
            self.accepted.extend_from_slice(&bytes[..count]);
            self.budget -= count;
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct Saturated;

    impl Write for Saturated {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Ok(0)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn sink(budget: usize) -> Throttled {
        Throttled {
            accepted: Vec::new(),
            budget,
        }
    }

    #[test]
    fn partial_writes_resume_where_they_stopped() {
        let mut outbox = Outbox::default();
        outbox.push(b"abcdef".to_vec(), Priority::Reply).unwrap();
        outbox.push(b"ghi".to_vec(), Priority::Reply).unwrap();
        let mut slow = sink(4);

        assert!(!outbox.drain_into(&mut slow).unwrap());
        slow.budget = 100;
        assert!(outbox.drain_into(&mut slow).unwrap());

        assert_eq!(slow.accepted, b"abcdefghi");
        assert!(outbox.is_empty());
    }

    #[test]
    fn events_are_refused_once_the_subscriber_limit_is_queued() {
        let mut outbox = Outbox::default();
        for _ in 0..MAX_QUEUED_EVENTS {
            outbox.push(vec![0; 8], Priority::Event).unwrap();
        }

        assert_eq!(
            outbox.push(vec![0; 8], Priority::Event),
            Err(Overflow::Events)
        );
        assert_eq!(outbox.push(vec![0; 8], Priority::Reply), Ok(()));
        assert_eq!(outbox.push(vec![0; 8], Priority::Final), Ok(()));
    }

    #[test]
    fn delivered_events_free_their_slots() {
        let mut outbox = Outbox::default();
        for _ in 0..MAX_QUEUED_EVENTS {
            outbox.push(vec![0; 8], Priority::Event).unwrap();
        }

        outbox.drain_into(&mut sink(8)).unwrap();

        assert_eq!(outbox.push(vec![0; 8], Priority::Event), Ok(()));
        assert_eq!(
            outbox.push(vec![0; 8], Priority::Event),
            Err(Overflow::Events)
        );
    }

    #[test]
    fn undelivered_bytes_are_bounded_except_for_the_final_frame() {
        let mut outbox = Outbox::default();
        for _ in 0..4 {
            outbox
                .push(vec![0; MAX_FRAME_BYTES], Priority::Reply)
                .unwrap();
        }

        assert_eq!(
            outbox.push(vec![0; 1], Priority::Reply),
            Err(Overflow::Bytes)
        );
        assert_eq!(outbox.push(vec![0; 1], Priority::Final), Ok(()));
    }

    #[test]
    fn sink_that_accepts_nothing_is_an_error_instead_of_a_spin() {
        let mut outbox = Outbox::default();
        outbox.push(b"frame".to_vec(), Priority::Reply).unwrap();

        let failure = outbox.drain_into(&mut Saturated).unwrap_err();

        assert_eq!(failure.kind(), io::ErrorKind::WriteZero);
    }
}
