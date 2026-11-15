use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::thread;
use std::time::Duration;

use tilling_ipc::client::{Client, ClientError};
use tilling_ipc::{
    Call, Event, EventFrame, HandshakeReply, IncompatibleVersion, Outcome, PROTOCOL_VERSION, Reply,
    Request, Response, ServerMessage, VersionInfo, encode,
};

use crate::support::{PATIENCE, Scratch, bind, hello, serve};

const SHORT_PATIENCE: Duration = Duration::from_millis(200);

fn version() -> Outcome {
    Outcome::Success {
        result: Reply::Version(VersionInfo {
            compositor: "tilling-test".to_owned(),
            protocol: PROTOCOL_VERSION,
        }),
    }
}

fn focus_changed(sequence: u64) -> ServerMessage {
    ServerMessage::Event(EventFrame {
        sequence,
        revision: 1,
        event: Event::FocusChanged { window: None },
    })
}

fn answer(request: &Request) -> Vec<ServerMessage> {
    vec![ServerMessage::Response(Response {
        request_id: request.request_id,
        outcome: version(),
    })]
}

fn stay_silent(_: &Request) -> Vec<ServerMessage> {
    Vec::new()
}

fn announce_then_answer(request: &Request) -> Vec<ServerMessage> {
    let mut messages = vec![focus_changed(1), focus_changed(2)];
    messages.extend(answer(request));
    messages
}

#[test]
fn call_returns_the_outcome_of_its_own_request() {
    let scratch = Scratch::new();
    let _server = serve(bind(&scratch), answer);

    let mut client = Client::connect(&scratch.socket(), PATIENCE).unwrap();

    assert_eq!(client.call(Call::GetVersion, None).unwrap(), version());
    assert_eq!(client.call(Call::GetVersion, Some(3)).unwrap(), version());
}

#[test]
fn silent_server_times_out_the_call() {
    let scratch = Scratch::new();
    let _server = serve(bind(&scratch), stay_silent);
    let mut client = Client::connect(&scratch.socket(), SHORT_PATIENCE).unwrap();

    let outcome = client.call(Call::GetVersion, None);

    assert!(matches!(outcome, Err(ClientError::TimedOut(waited)) if waited == SHORT_PATIENCE));
}

#[test]
fn events_that_precede_the_response_are_kept_in_order() {
    let scratch = Scratch::new();
    let _server = serve(bind(&scratch), announce_then_answer);
    let mut client = Client::connect(&scratch.socket(), PATIENCE).unwrap();

    let outcome = client.call(Call::Subscribe, None).unwrap();
    let sequences = [
        client.next_event().unwrap().sequence,
        client.next_event().unwrap().sequence,
    ];

    assert_eq!(outcome, version());
    assert_eq!(sequences, [1, 2]);
}

#[test]
fn server_of_another_major_is_reported_as_incompatible() {
    let scratch = Scratch::new();
    let socket = scratch.root().join("other-major.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let refusal = IncompatibleVersion {
        requested: PROTOCOL_VERSION.major,
        supported: 2,
    };
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut greeting = vec![0_u8; encode(&hello(PROTOCOL_VERSION)).unwrap().len()];
        stream.read_exact(&mut greeting).unwrap();
        let reply = encode(&HandshakeReply::Incompatible(refusal)).unwrap();
        stream.write_all(&reply).unwrap();
    });

    let outcome = Client::connect(&socket, PATIENCE);
    server.join().unwrap();

    assert!(matches!(outcome, Err(ClientError::Incompatible(found)) if found == refusal));
}

#[test]
fn missing_socket_names_the_path_it_tried() {
    let scratch = Scratch::new();

    let outcome = Client::connect(&scratch.socket(), PATIENCE);

    assert!(matches!(outcome, Err(ClientError::Connect { path, .. }) if path == scratch.socket()));
}
