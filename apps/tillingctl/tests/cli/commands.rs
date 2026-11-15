use tilling_ipc::{
    Event, EventFrame, Outcome, PROTOCOL_VERSION, Reply, Request, ServerMessage, Snapshot,
    VersionInfo,
};

use crate::support::{Runtime, reply, standard_error, standard_output};

fn version(request: &Request) -> Vec<ServerMessage> {
    let result = Reply::Version(VersionInfo {
        compositor: "tilling-test".to_owned(),
        protocol: PROTOCOL_VERSION,
    });
    vec![reply(request, Outcome::Success { result })]
}

fn missing(request: &Request) -> Vec<ServerMessage> {
    let message = "window 9:9 does not exist".to_owned();
    vec![reply(request, Outcome::NotFound { message })]
}

fn accept_at_expected_revision(request: &Request) -> Vec<ServerMessage> {
    let revision = request.expected_revision.unwrap_or_default();
    let result = Reply::Accepted { revision };
    vec![reply(request, Outcome::Success { result })]
}

fn snapshot_then_overflow(request: &Request) -> Vec<ServerMessage> {
    let result = Reply::Subscribed(Snapshot {
        sequence: 10,
        revision: 3,
        outputs: Vec::new(),
        workspaces: Vec::new(),
        windows: Vec::new(),
    });
    let event = |sequence, event| {
        ServerMessage::Event(EventFrame {
            sequence,
            revision: 3,
            event,
        })
    };
    vec![
        reply(request, Outcome::Success { result }),
        event(11, Event::FocusChanged { window: None }),
        event(12, Event::EventsDropped),
    ]
}

#[test]
fn query_prints_the_reply_of_the_only_running_instance() {
    let runtime = Runtime::new();
    let _instance = runtime.start("main", version);

    let output = runtime
        .tillingctl()
        .args(["get", "version"])
        .output()
        .unwrap();

    assert_eq!(standard_output(&output), "tilling-test (protocol 1.2)\n");
    assert_eq!(standard_error(&output), "");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn refusal_goes_to_standard_error_with_its_own_exit_code() {
    let runtime = Runtime::new();
    let _instance = runtime.start("main", missing);

    let output = runtime
        .tillingctl()
        .args(["focus", "9:9"])
        .output()
        .unwrap();

    assert_eq!(standard_output(&output), "");
    assert_eq!(
        standard_error(&output),
        "tillingctl: not found: window 9:9 does not exist\n"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn json_refusal_stays_on_standard_output() {
    let runtime = Runtime::new();
    let _instance = runtime.start("main", missing);

    let output = runtime
        .tillingctl()
        .args(["--json", "focus", "9:9"])
        .output()
        .unwrap();

    assert_eq!(
        standard_output(&output),
        "{\"status\":\"not_found\",\"message\":\"window 9:9 does not exist\"}\n"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn revision_precondition_reaches_the_compositor() {
    let runtime = Runtime::new();
    let _instance = runtime.start("main", accept_at_expected_revision);

    let output = runtime
        .tillingctl()
        .args(["reload", "--if-revision", "12"])
        .output()
        .unwrap();

    assert_eq!(standard_output(&output), "accepted at revision 12\n");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn subscription_prints_until_events_are_dropped() {
    let runtime = Runtime::new();
    let _instance = runtime.start("main", snapshot_then_overflow);

    let output = runtime.tillingctl().arg("subscribe").output().unwrap();

    assert_eq!(
        standard_output(&output),
        "snapshot #10 r3\n#11 r3 focused window -\n#12 r3 events dropped\n"
    );
    assert!(standard_error(&output).contains("dropped events"));
    assert_eq!(output.status.code(), Some(3));
}
