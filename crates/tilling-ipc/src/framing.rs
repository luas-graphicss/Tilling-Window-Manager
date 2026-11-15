pub const MAX_FRAME_BYTES: usize = 1 << 20;

const PREFIX_BYTES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("frame of {length} bytes exceeds the {MAX_FRAME_BYTES} byte limit")]
    TooLarge { length: usize },
    #[error("frame is empty")]
    Empty,
    #[error("stream ended inside a frame after {received} bytes")]
    Truncated { received: usize },
}

pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    let length = checked_length(payload.len())?;
    let mut frame = Vec::with_capacity(PREFIX_BYTES + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

#[derive(Debug, Default)]
pub struct FrameDecoder {
    pending: Vec<u8>,
    failure: Option<FrameError>,
}

impl FrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();
        let mut consumed = 0;
        loop {
            let unread = self.pending.get(consumed..).unwrap_or_default();
            match next_payload(unread) {
                Ok(Some(payload)) => {
                    consumed += PREFIX_BYTES + payload.len();
                    frames.push(payload.to_vec());
                }
                Ok(None) => break,
                Err(failure) => {
                    self.failure = Some(failure);
                    self.pending.clear();
                    return if frames.is_empty() {
                        Err(failure)
                    } else {
                        Ok(frames)
                    };
                }
            }
        }
        self.pending.drain(..consumed);
        Ok(frames)
    }

    pub fn finish(self) -> Result<(), FrameError> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        if self.pending.is_empty() {
            return Ok(());
        }
        Err(FrameError::Truncated {
            received: self.pending.len(),
        })
    }
}

fn next_payload(unread: &[u8]) -> Result<Option<&[u8]>, FrameError> {
    let Some(prefix) = unread.first_chunk::<PREFIX_BYTES>() else {
        return Ok(None);
    };
    let declared = usize::try_from(u32::from_be_bytes(*prefix)).unwrap_or(usize::MAX);
    checked_length(declared)?;
    Ok(unread.get(PREFIX_BYTES..PREFIX_BYTES + declared))
}

fn checked_length(length: usize) -> Result<u32, FrameError> {
    if length == 0 {
        return Err(FrameError::Empty);
    }
    if length > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge { length });
    }
    u32::try_from(length).map_err(|_| FrameError::TooLarge { length })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(payload: &[u8]) -> Vec<u8> {
        encode_frame(payload).unwrap()
    }

    #[test]
    fn prefix_is_a_big_endian_length() {
        assert_eq!(frame(b"{}"), [0, 0, 0, 2, b'{', b'}']);
    }

    #[test]
    fn byte_by_byte_delivery_yields_each_frame_once() {
        let stream = [frame(b"first"), frame(b"second")].concat();
        let mut decoder = FrameDecoder::default();
        let mut frames = Vec::new();

        for byte in stream {
            frames.extend(decoder.push(&[byte]).unwrap());
        }

        assert_eq!(frames, [b"first".to_vec(), b"second".to_vec()]);
        assert_eq!(decoder.finish(), Ok(()));
    }

    #[test]
    fn one_read_can_carry_several_frames_and_a_partial_one() {
        let mut stream = [frame(b"a"), frame(b"bc"), frame(b"def")].concat();
        let tail = stream.split_off(stream.len() - 2);
        let mut decoder = FrameDecoder::default();

        assert_eq!(
            decoder.push(&stream).unwrap(),
            [b"a".to_vec(), b"bc".to_vec()]
        );
        assert_eq!(decoder.push(&tail).unwrap(), [b"def".to_vec()]);
    }

    #[test]
    fn incomplete_prefix_waits_and_then_reports_truncation() {
        let mut decoder = FrameDecoder::default();

        assert!(decoder.push(&[0, 0]).unwrap().is_empty());
        assert!(decoder.push(&[]).unwrap().is_empty());
        assert_eq!(decoder.finish(), Err(FrameError::Truncated { received: 2 }));
    }

    #[test]
    fn truncated_payload_is_reported_when_the_stream_ends() {
        let mut stream = frame(b"complete payload");
        stream.truncate(stream.len() - 3);
        let mut decoder = FrameDecoder::default();

        assert!(decoder.push(&stream).unwrap().is_empty());
        assert_eq!(
            decoder.finish(),
            Err(FrameError::Truncated {
                received: stream.len()
            })
        );
    }

    #[test]
    fn oversized_length_is_rejected_from_the_prefix_alone() {
        let mut decoder = FrameDecoder::default();
        let too_large = FrameError::TooLarge {
            length: MAX_FRAME_BYTES + 1,
        };

        assert_eq!(decoder.push(&[0, 0x10, 0, 1]), Err(too_large));
        assert_eq!(decoder.push(b"more bytes"), Err(too_large));
        assert_eq!(decoder.finish(), Err(too_large));
    }

    #[test]
    fn largest_declared_length_does_not_reserve_memory() {
        let mut decoder = FrameDecoder::default();

        let outcome = decoder.push(&[0xff, 0xff, 0xff, 0xff]);

        assert!(matches!(outcome, Err(FrameError::TooLarge { .. })));
        assert!(decoder.pending.capacity() < MAX_FRAME_BYTES);
    }

    #[test]
    fn frames_before_a_violation_are_still_delivered() {
        let stream = [frame(b"valid"), vec![0, 0, 0, 0]].concat();
        let mut decoder = FrameDecoder::default();

        assert_eq!(decoder.push(&stream).unwrap(), [b"valid".to_vec()]);
        assert_eq!(decoder.push(&[]), Err(FrameError::Empty));
    }

    #[test]
    fn encoder_enforces_the_same_limits() {
        let limit = vec![b' '; MAX_FRAME_BYTES];
        let beyond = vec![b' '; MAX_FRAME_BYTES + 1];

        assert_eq!(encode_frame(&[]), Err(FrameError::Empty));
        assert!(encode_frame(&limit).is_ok());
        assert_eq!(
            encode_frame(&beyond),
            Err(FrameError::TooLarge {
                length: MAX_FRAME_BYTES + 1
            })
        );
    }

    #[test]
    fn frame_at_the_limit_round_trips() {
        let payload = vec![b'x'; MAX_FRAME_BYTES];
        let mut decoder = FrameDecoder::default();

        let frames = decoder.push(&frame(&payload)).unwrap();

        assert_eq!(frames, [payload]);
    }
}
