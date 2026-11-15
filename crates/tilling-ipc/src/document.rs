use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::framing::{FrameError, encode_frame};

pub const MAX_DEPTH: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("payload is not valid UTF-8")]
    InvalidUtf8(#[source] std::str::Utf8Error),
    #[error("document nests deeper than {MAX_DEPTH} levels")]
    TooDeep,
    #[error("document does not match the protocol")]
    Mismatch(#[source] serde_json::Error),
    #[error("message cannot be serialized")]
    Unserializable(#[source] serde_json::Error),
    #[error(transparent)]
    Frame(#[from] FrameError),
}

pub fn decode<T: DeserializeOwned>(payload: &[u8]) -> Result<T, DocumentError> {
    let text = std::str::from_utf8(payload).map_err(DocumentError::InvalidUtf8)?;
    if exceeds_depth(text) {
        return Err(DocumentError::TooDeep);
    }
    serde_json::from_str(text).map_err(DocumentError::Mismatch)
}

pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, DocumentError> {
    let payload = serde_json::to_vec(message).map_err(DocumentError::Unserializable)?;
    Ok(encode_frame(&payload)?)
}

fn exceeds_depth(text: &str) -> bool {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in text.bytes() {
        if in_string {
            match byte {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'[' | b'{' if depth == MAX_DEPTH => return true,
            b'[' | b'{' => depth += 1,
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn nested(levels: usize) -> String {
        format!("{}{}", "[".repeat(levels), "]".repeat(levels))
    }

    #[test]
    fn invalid_utf8_is_rejected_before_parsing() {
        let outcome = decode::<Value>(&[b'"', 0xff, 0xfe, b'"']);

        assert!(matches!(outcome, Err(DocumentError::InvalidUtf8(_))));
    }

    #[test]
    fn nesting_is_limited_to_the_documented_depth() {
        assert!(decode::<Value>(nested(MAX_DEPTH).as_bytes()).is_ok());
        assert!(matches!(
            decode::<Value>(nested(MAX_DEPTH + 1).as_bytes()),
            Err(DocumentError::TooDeep)
        ));
        assert!(matches!(
            decode::<Value>(nested(100_000).as_bytes()),
            Err(DocumentError::TooDeep)
        ));
    }

    #[test]
    fn brackets_inside_strings_do_not_count_as_nesting() {
        let text = format!(r#"{{"title":"{}\"[[\\"}}"#, "[{".repeat(64));

        assert!(decode::<Value>(text.as_bytes()).is_ok());
    }

    #[test]
    fn malformed_json_is_a_protocol_mismatch() {
        let outcome = decode::<Value>(b"{\"request_id\":");

        assert!(matches!(outcome, Err(DocumentError::Mismatch(_))));
    }

    #[test]
    fn encoded_messages_are_complete_frames() {
        let frame = encode(&[1, 2, 3]).unwrap();

        assert_eq!(
            frame,
            [0, 0, 0, 7, b'[', b'1', b',', b'2', b',', b'3', b']']
        );
    }
}
