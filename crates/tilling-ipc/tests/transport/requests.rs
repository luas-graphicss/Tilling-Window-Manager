use tilling_ipc::server::ConnectionError;
use tilling_ipc::{
    Call, FrameError, Outcome, Request, Response, ServerMessage, encode, encode_frame,
};

use crate::support::{established, eventually, flush, receive, request};

fn ids(requests: &[Request]) -> Vec<u64> {
    requests.iter().map(|request| request.request_id).collect()
}

#[test]
fn request_delivered_byte_by_byte_arrives_once() {
    let mut session = established();
    let frame = encode(&request(1, Call::GetVersion)).unwrap();
    let mut delivered = Vec::new();

    for byte in frame {
        session.peer.write(&[byte]);
        delivered.extend(session.connection.receive(8).unwrap());
    }

    assert_eq!(delivered, [request(1, Call::GetVersion)]);
}

#[test]
fn capacity_limits_each_delivery_and_keeps_the_rest_queued() {
    let mut session = established();
    for request_id in 1..=5 {
        session.peer.send(&request(request_id, Call::GetWindows));
    }

    let first = receive(&mut session.connection, 2);
    let pending_after_first = session.connection.has_undelivered_requests();
    let second = session.connection.receive(2).unwrap();
    let third = session.connection.receive(2).unwrap();

    assert!(pending_after_first);
    assert_eq!(ids(&first), [1, 2]);
    assert_eq!(ids(&second), [3, 4]);
    assert_eq!(ids(&third), [5]);
    assert!(!session.connection.has_undelivered_requests());
}

#[test]
fn unsupported_method_is_answered_and_the_session_continues() {
    let mut session = established();
    let unsupported = br#"{"request_id":7,"method":"evaluate","params":{"code":"1"}}"#;
    session.peer.write(&encode_frame(unsupported).unwrap());
    session.peer.send(&request(8, Call::Quit));

    let delivered = receive(&mut session.connection, 1);
    flush(&mut session.connection);

    assert_eq!(delivered, [request(8, Call::Quit)]);
    assert!(matches!(
        session.peer.receive::<ServerMessage>(),
        ServerMessage::Response(Response {
            request_id: 7,
            outcome: Outcome::ValidationError { .. },
        })
    ));
}

#[test]
fn document_without_request_id_closes_the_connection() {
    let mut session = established();

    session
        .peer
        .write(&encode_frame(br#"{"method":"quit"}"#).unwrap());
    let failure = eventually(|| session.connection.receive(8).err());

    assert!(matches!(failure, ConnectionError::Document(_)));
}

#[test]
fn hang_up_is_reported_as_closed() {
    let mut session = established();

    drop(session.peer);
    let failure = eventually(|| session.connection.receive(8).err());

    assert!(matches!(failure, ConnectionError::Closed));
}

#[test]
fn hang_up_inside_a_frame_is_reported_as_truncation() {
    let mut session = established();

    session.peer.write(&[0, 0, 0, 9, b'{']);
    drop(session.peer);
    let failure = eventually(|| session.connection.receive(8).err());

    assert!(matches!(
        failure,
        ConnectionError::Frame(FrameError::Truncated { received: 5 })
    ));
}

#[test]
fn oversized_frame_is_rejected_from_its_prefix() {
    let mut session = established();

    session.peer.write(&[0, 0x10, 0, 1]);
    let failure = eventually(|| session.connection.receive(8).err());

    assert!(matches!(
        failure,
        ConnectionError::Frame(FrameError::TooLarge { .. })
    ));
}
