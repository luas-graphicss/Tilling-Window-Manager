use std::collections::VecDeque;
use std::io::{self, Read};

use crate::framing::{FrameDecoder, FrameError};

const READ_CHUNK_BYTES: usize = 16 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StreamError {
    #[error("peer closed the connection")]
    Closed,
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("socket failure")]
    Socket(#[from] io::Error),
}

#[derive(Debug, Default)]
pub(crate) struct Inbound {
    decoder: FrameDecoder,
    frames: VecDeque<Vec<u8>>,
}

impl Inbound {
    pub(crate) fn fill(&mut self, source: &mut impl Read) -> Result<(), StreamError> {
        let mut chunk = [0_u8; READ_CHUNK_BYTES];
        let length = source.read(&mut chunk)?;
        if length == 0 {
            return Err(self.end_of_stream());
        }
        let frames = self.decoder.push(chunk.get(..length).unwrap_or_default())?;
        self.decoder.push(&[])?;
        self.frames.extend(frames);
        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Option<Vec<u8>> {
        self.frames.pop_front()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    fn end_of_stream(&mut self) -> StreamError {
        match std::mem::take(&mut self.decoder).finish() {
            Ok(()) => StreamError::Closed,
            Err(failure) => failure.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::encode_frame;

    #[test]
    fn frames_split_across_reads_are_queued_in_order() {
        let stream = [
            encode_frame(b"first").unwrap(),
            encode_frame(b"second").unwrap(),
        ]
        .concat();
        let (head, tail) = stream.split_at(7);
        let mut inbound = Inbound::default();

        inbound.fill(&mut &*head).unwrap();
        assert!(inbound.is_empty());
        inbound.fill(&mut &*tail).unwrap();

        assert_eq!(inbound.pop(), Some(b"first".to_vec()));
        assert_eq!(inbound.pop(), Some(b"second".to_vec()));
        assert_eq!(inbound.pop(), None);
    }

    #[test]
    fn end_of_stream_between_frames_is_an_orderly_close() {
        let mut inbound = Inbound::default();

        let outcome = inbound.fill(&mut io::empty());

        assert!(matches!(outcome, Err(StreamError::Closed)));
    }

    #[test]
    fn end_of_stream_inside_a_frame_is_a_truncation() {
        let mut inbound = Inbound::default();
        inbound.fill(&mut &[0_u8, 0, 0, 9, b'{'][..]).unwrap();

        let outcome = inbound.fill(&mut io::empty());

        assert!(matches!(
            outcome,
            Err(StreamError::Frame(FrameError::Truncated { received: 5 }))
        ));
    }

    #[test]
    fn violation_discards_the_frames_that_shared_its_read() {
        let stream = [encode_frame(b"valid").unwrap(), vec![0, 0, 0, 0]].concat();
        let mut inbound = Inbound::default();

        let outcome = inbound.fill(&mut &*stream);

        assert!(matches!(
            outcome,
            Err(StreamError::Frame(FrameError::Empty))
        ));
        assert!(inbound.is_empty());
    }
}
