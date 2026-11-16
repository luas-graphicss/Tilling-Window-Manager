use tilling_compositor::SceneLayer;
use tilling_ipc::client::Client;
use tilling_ipc::{
    Call, Event, EventFrame, Layout, Outcome, Reply, ResourceId, WindowInfo, WindowMode,
    WorkspaceInfo,
};

use crate::harness::Harness;

const SWITCH_LIMIT: usize = 20_000;
const EVENT_LIMIT: usize = 200_000;

fn accepted(outcome: Outcome) -> Reply {
    match outcome {
        Outcome::Success { result } => result,
        refused => panic!("request was refused: {refused:?}"),
    }
}

fn request(control: &mut Client, call: Call) -> Reply {
    let outcome = control
        .call(call, None)
        .unwrap_or_else(|error| panic!("control request failed: {error}"));
    accepted(outcome)
}

fn windows(control: &mut Client) -> Vec<WindowInfo> {
    match request(control, Call::GetWindows) {
        Reply::Windows(windows) => windows,
        other => panic!("unexpected reply to get windows: {other:?}"),
    }
}

fn workspaces(control: &mut Client) -> Vec<WorkspaceInfo> {
    match request(control, Call::GetWorkspaces) {
        Reply::Workspaces(workspaces) => workspaces,
        other => panic!("unexpected reply to get workspaces: {other:?}"),
    }
}

fn events_until(control: &mut Client, wanted: impl Fn(&Event) -> bool) -> Vec<EventFrame> {
    let mut received = Vec::new();
    while received.len() < EVENT_LIMIT {
        let frame = control
            .next_event()
            .unwrap_or_else(|error| panic!("event stream failed: {error}"));
        let found = wanted(&frame.event);
        received.push(frame);
        if found {
            return received;
        }
    }
    panic!("the awaited event never arrived");
}

#[test]
fn queries_describe_the_windows_workspaces_and_outputs_of_the_session() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let editor = client.create_toplevel("org.example.Editor", "notes");
    let terminal = client.create_toplevel("org.example.Terminal", "shell");
    client.map(editor).unwrap();
    client.map(terminal).unwrap();
    client.settle().unwrap();
    harness.published(2);
    let mut control = harness.control();

    let listed = windows(&mut control);
    let workspaces = workspaces(&mut control);
    let outputs = request(&mut control, Call::GetOutputs);

    let described: Vec<_> = listed
        .iter()
        .map(|window| {
            (
                window.app_id.as_deref(),
                window.title.as_deref(),
                window.focused,
            )
        })
        .collect();
    assert_eq!(
        described,
        [
            (Some("org.example.Editor"), Some("notes"), false),
            (Some("org.example.Terminal"), Some("shell"), true),
        ]
    );
    assert_eq!(workspaces.len(), 9);
    assert_eq!(workspaces[0].windows, [listed[0].id, listed[1].id]);
    assert_eq!(workspaces[0].focused_window, Some(listed[1].id));
    let Reply::Outputs(outputs) = outputs else {
        panic!("unexpected reply to get outputs: {outputs:?}");
    };
    assert_eq!(
        (
            outputs[0].name.as_str(),
            outputs[0].width,
            outputs[0].height
        ),
        ("HEADLESS-1", 1280, 720)
    );
    assert_eq!(outputs[0].workspace, workspaces[0].id);
}

#[test]
fn switching_workspace_hides_the_windows_and_reports_the_new_revision() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    let before = harness.published(1);
    let mut control = harness.control();
    let second = workspaces(&mut control)[1].id;

    let switch = Call::SwitchWorkspace { workspace: second };
    let reply = accepted(control.call(switch, Some(before.revision)).unwrap());
    let after = harness.published(0);

    assert!(after.revision > before.revision);
    assert_eq!(
        reply,
        Reply::Accepted {
            revision: after.revision
        }
    );
    assert_eq!(after.mapped_windows, 1);
}

#[test]
fn command_issued_against_a_stale_revision_changes_nothing() {
    let harness = Harness::start();
    let mut control = harness.control();
    let second = workspaces(&mut control)[1].id;
    let current = harness.snapshot().revision;

    let switch = Call::SwitchWorkspace { workspace: second };
    let outcome = control.call(switch, Some(current + 7)).unwrap();

    assert_eq!(
        outcome,
        Outcome::RevisionConflict {
            current_revision: current
        }
    );
    assert_eq!(harness.snapshot().revision, current);
}

#[test]
fn unknown_or_unavailable_requests_are_refused_with_their_reason() {
    let harness = Harness::start_with_config("schema_version = 1\n\n[session]\n");
    let mut control = harness.control();
    let before = harness.snapshot().revision;
    let nowhere = ResourceId {
        index: 40,
        generation: 3,
    };

    let missing = control.call(Call::FocusWindow { window: nowhere }, None);
    let layout = control.call(
        Call::SetLayout {
            workspace: nowhere,
            layout: Layout::Monocle,
        },
        None,
    );
    let lock = control.call(Call::RequestLock, None);

    assert_eq!(
        missing.unwrap(),
        Outcome::NotFound {
            message: "window 40v3 does not exist".to_owned()
        }
    );
    assert!(matches!(layout.unwrap(), Outcome::NotFound { .. }));
    assert_eq!(
        lock.unwrap(),
        Outcome::NotPermitted {
            message: "no locker is configured in `session.locker`".to_owned()
        }
    );
    assert_eq!(harness.snapshot().revision, before);
}

#[test]
fn layout_set_over_the_socket_rearranges_the_workspace_and_is_reported() {
    let harness = Harness::start();
    let mut client = harness.connect();
    for title in ["editor", "shell", "logs"] {
        let window = client.create_toplevel("org.example.Terminal", title);
        client.map(window).unwrap();
        client.settle().unwrap();
    }
    harness.published(3);
    let mut control = harness.control();
    let first = workspaces(&mut control)[0].id;
    let set_layout = |layout| Call::SetLayout {
        workspace: first,
        layout,
    };

    request(&mut control, set_layout(Layout::Monocle));
    client.settle().unwrap();
    let monocle = harness.published(1);
    let reported = workspaces(&mut control);
    request(&mut control, set_layout(Layout::Bsp));
    client.settle().unwrap();
    let bsp = harness.published(3);

    assert!(monocle.scene.windows()[0].focused);
    assert_eq!(
        (reported[0].layout, reported[0].active),
        (Layout::Monocle, true)
    );
    assert_eq!(
        (reported[1].layout, reported[1].active),
        (Layout::Bsp, false)
    );
    assert_eq!(bsp.scene.windows().len(), 3);
}

#[test]
fn window_mode_set_over_the_socket_reaches_the_scene() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);
    let mut control = harness.control();
    let id = windows(&mut control)[0].id;
    let set_mode = |mode| Call::SetWindowMode { window: id, mode };

    request(&mut control, set_mode(WindowMode::Fullscreen));
    client.settle().unwrap();
    let fullscreen = harness.published(1);
    request(&mut control, set_mode(WindowMode::Floating));
    client.settle().unwrap();
    let floating = harness.published(1);
    request(&mut control, set_mode(WindowMode::Tiled));
    client.settle().unwrap();
    let tiled = harness.published(1);

    assert_eq!(fullscreen.scene.windows()[0].layer, SceneLayer::Fullscreen);
    assert_eq!(floating.scene.windows()[0].layer, SceneLayer::Floating);
    assert_eq!(tiled.scene.windows()[0].layer, SceneLayer::Tiled);
    assert_eq!(windows(&mut control)[0].mode, WindowMode::Tiled);
}

#[test]
fn subscriber_gets_a_snapshot_and_then_every_change_in_order() {
    let harness = Harness::start();
    let mut control = harness.control();
    let subscribed = request(&mut control, Call::Subscribe);
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    client.map(window).unwrap();
    harness.published(1);
    let received = events_until(&mut control, |event| {
        matches!(event, Event::FocusChanged { window: Some(_) })
    });

    let Reply::Subscribed(snapshot) = subscribed else {
        panic!("unexpected reply to subscribe: {subscribed:?}");
    };
    let sequences: Vec<u64> = received.iter().map(|frame| frame.sequence).collect();
    let expected: Vec<u64> = (snapshot.sequence + 1..).take(received.len()).collect();
    assert!(snapshot.windows.is_empty());
    assert_eq!(sequences, expected);
    assert!(received.iter().any(|frame| matches!(
        &frame.event,
        Event::WindowOpened(opened) if opened.title.as_deref() == Some("shell")
    )));
    assert!(
        received
            .iter()
            .all(|frame| frame.revision > snapshot.revision)
    );
}

#[test]
fn subscriber_that_stops_reading_is_cut_off_while_others_are_still_served() {
    let harness = Harness::start();
    let mut stalled = harness.control();
    request(&mut stalled, Call::Subscribe);
    let mut active = harness.control();
    let listed = workspaces(&mut active);
    let targets = [listed[1].id, listed[0].id];

    let mut switches = 0;
    while harness.snapshot().control_subscribers == 1 {
        assert!(switches < SWITCH_LIMIT, "subscriber was never cut off");
        let workspace = targets[switches % targets.len()];
        request(&mut active, Call::SwitchWorkspace { workspace });
        switches += 1;
    }
    let received = events_until(&mut stalled, |event| *event == Event::EventsDropped);

    let in_order = received
        .windows(2)
        .all(|pair| pair[0].sequence < pair[1].sequence);
    assert!(in_order);
    assert_eq!(harness.snapshot().control_peers, 2);
    assert_eq!(workspaces(&mut active).len(), 9);
}

#[test]
fn peer_that_disconnects_is_forgotten() {
    let harness = Harness::start();
    let mut staying = harness.control();
    let leaving = harness.control();
    workspaces(&mut staying);
    harness.wait_until("both peers are connected", |snapshot| {
        snapshot.control_peers == 2
    });

    drop(leaving);
    let snapshot = harness.wait_until("the closed peer is gone", |snapshot| {
        snapshot.control_peers == 1
    });

    assert_eq!(snapshot.control_subscribers, 0);
    assert_eq!(workspaces(&mut staying).len(), 9);
}

#[test]
fn quit_request_is_answered_before_the_compositor_stops() {
    let harness = Harness::start();
    let mut control = harness.control();

    let reply = request(&mut control, Call::Quit);
    harness.wait_for_exit();

    assert!(matches!(reply, Reply::Accepted { .. }));
    assert!(control.call(Call::GetVersion, None).is_err());
}
