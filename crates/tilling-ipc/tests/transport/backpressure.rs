use std::thread;

use tilling_ipc::server::{MAX_QUEUED_BYTES, MAX_QUEUED_EVENTS, SendError};
use tilling_ipc::{Event, EventFrame, Outcome, Response, ServerMessage};

use crate::support::{established, flush};

const BULK_BYTES: usize = 512 * 1024;

fn event(sequence: u64) -> ServerMessage {
    ServerMessage::Event(EventFrame {
        sequence,
        revision: 1,
        event: Event::FocusChanged { window: None },
    })
}

fn events_dropped(sequence: u64) -> ServerMessage {
    ServerMessage::Event(EventFrame {
        sequence,
        revision: 1,
        event: Event::EventsDropped,
    })
}

fn bulky_reply(request_id: u64) -> ServerMessage {
    ServerMessage::Response(Response {
        request_id,
        outcome: Outcome::NotFound {
            message: "x".repeat(BULK_BYTES),
        },
    })
}

#[test]
fn subscriber_that_stops_reading_overflows_at_the_event_limit() {
    let mut session = established();
    let sequences = || (0_u64..).take(MAX_QUEUED_EVENTS);
    for sequence in sequences() {
        session.connection.send(&event(sequence)).unwrap();
    }

    let overflow = session.connection.send(&event(1000));
    let marker = session.connection.send(&events_dropped(1000));
    flush(&mut session.connection);

    assert!(matches!(overflow, Err(SendError::EventQueueFull)));
    assert!(marker.is_ok());
    for sequence in sequences() {
        assert_eq!(session.peer.receive::<ServerMessage>(), event(sequence));
    }
    assert_eq!(
        session.peer.receive::<ServerMessage>(),
        events_dropped(1000)
    );
}

#[test]
fn stalled_peer_makes_flush_return_instead_of_blocking() {
    let mut session = established();
    for request_id in 1..=4 {
        session.connection.send(&bulky_reply(request_id)).unwrap();
    }

    let drained_while_stalled = session.connection.flush().unwrap();
    let wanted_write = session.connection.wants_write();
    let delivered = thread::scope(|scope| {
        let reader = scope.spawn(|| {
            (0..4)
                .map(|_| session.peer.receive::<ServerMessage>())
                .collect::<Vec<_>>()
        });
        flush(&mut session.connection);
        reader.join().unwrap()
    });

    assert!(!drained_while_stalled);
    assert!(wanted_write);
    assert_eq!(delivered, (1..=4).map(bulky_reply).collect::<Vec<_>>());
    assert!(!session.connection.wants_write());
}

#[test]
fn replies_to_a_peer_that_never_reads_are_bounded() {
    let mut session = established();

    let accepted = (1_u64..)
        .map(|request_id| session.connection.send(&bulky_reply(request_id)))
        .take_while(Result::is_ok)
        .count();
    let refusal = session.connection.send(&bulky_reply(0));

    assert!(accepted > 0);
    assert!(accepted * BULK_BYTES <= MAX_QUEUED_BYTES);
    assert!(matches!(refusal, Err(SendError::Congested)));
}
