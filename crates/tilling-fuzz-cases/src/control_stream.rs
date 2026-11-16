use serde::Serialize;
use serde::de::DeserializeOwned;
use tilling_ipc::{
    FrameDecoder, FrameError, HandshakeReply, Hello, MAX_FRAME_BYTES, PROTOCOL_VERSION, Request,
    decode, encode, encode_frame, negotiate,
};

type Delivery = (Vec<Vec<u8>>, Result<(), FrameError>);

pub fn check(input: &[u8]) {
    let Some((&split, stream)) = input.split_first() else {
        return;
    };
    let whole = deliver(stream, stream.len().max(1));
    let pieces = deliver(stream, usize::from(split) + 1);
    assert_eq!(
        whole, pieces,
        "frames depend on how the stream was split into reads"
    );
    for frame in &whole.0 {
        check_frame(frame);
    }
}

fn deliver(stream: &[u8], read_size: usize) -> Delivery {
    let mut decoder = FrameDecoder::default();
    let mut frames = Vec::new();
    for read in stream.chunks(read_size) {
        match decoder.push(read) {
            Ok(received) => frames.extend(received),
            Err(_) => break,
        }
    }
    (frames, decoder.finish())
}

fn check_frame(frame: &[u8]) {
    assert!(
        !frame.is_empty() && frame.len() <= MAX_FRAME_BYTES,
        "decoder delivered a frame outside the limits"
    );
    let framed = encode_frame(frame).unwrap_or_default();
    assert_eq!(
        deliver(&framed, framed.len().max(1)),
        (vec![frame.to_vec()], Ok(())),
        "delivered frame does not survive being framed again"
    );
    if let Ok(request) = decode::<Request>(frame) {
        assert!(
            is_accepted_again(&request),
            "accepted request is refused after being encoded again"
        );
    }
    if let Ok(hello) = decode::<Hello>(frame) {
        check_handshake(&hello);
    }
}

fn check_handshake(hello: &Hello) {
    let outcome = negotiate(hello);
    match &outcome {
        Ok(welcome) => {
            assert_eq!(
                welcome.protocol.major, PROTOCOL_VERSION.major,
                "handshake settled on a major this endpoint does not speak"
            );
            assert!(
                welcome.protocol.minor <= PROTOCOL_VERSION.minor.min(hello.protocol.minor),
                "handshake settled on a minor one of the peers does not know"
            );
            assert!(
                welcome.capabilities.iter().all(|capability| hello
                    .capabilities
                    .iter()
                    .any(|name| name == capability.name())),
                "capability was granted without being requested"
            );
        }
        Err(refusal) => assert_ne!(
            refusal.requested, PROTOCOL_VERSION.major,
            "peer with the supported major was refused"
        ),
    }
    assert!(
        is_accepted_again(&HandshakeReply::from(outcome)),
        "handshake reply cannot be read back"
    );
}

fn is_accepted_again<T: Serialize + DeserializeOwned>(message: &T) -> bool {
    let Ok(encoded) = encode(message) else {
        return false;
    };
    let (frames, end) = deliver(&encoded, encoded.len().max(1));
    end.is_ok() && matches!(frames.as_slice(), [payload] if decode::<T>(payload).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(split: u8, payloads: &[&[u8]]) -> Vec<u8> {
        let mut input = vec![split];
        for payload in payloads {
            input.extend(encode_frame(payload).unwrap());
        }
        input
    }

    #[test]
    fn requests_and_handshakes_are_checked_whatever_the_read_size() {
        let request = br#"{"request_id":7,"method":"get_windows"}"#;
        let hello = br#"{"protocol":{"major":1,"minor":9},"capabilities":["events","telepathy"]}"#;

        for split in [0, 1, 6, 255] {
            check(&stream(split, &[request, hello, b"not json"]));
        }
    }

    #[test]
    fn violations_end_the_stream_the_same_way_for_every_read_size() {
        let mut oversized = stream(2, &[b"{}"]);
        oversized.extend([0xff, 0xff, 0xff, 0xff, b'x']);
        let mut truncated = stream(0, &[b"{}"]);
        truncated.extend([0, 0, 0, 9, b'x']);

        check(&oversized);
        check(&truncated);
        check(&[3, 0, 0, 0, 0]);
        check(&[]);
    }

    #[test]
    fn delivery_reports_the_frames_and_how_the_stream_ended() {
        let input = stream(0, &[b"first", b"second"]);
        let framed = input.get(1..).unwrap();

        assert_eq!(
            deliver(framed, 3),
            (vec![b"first".to_vec(), b"second".to_vec()], Ok(()))
        );
        assert_eq!(
            deliver(&[0, 0, 0, 4, b'x'], 1),
            (Vec::new(), Err(FrameError::Truncated { received: 5 }))
        );
    }
}
