use tilling_ipc::{
    Call, Direction, DocumentError, Event, EventFrame, FrameDecoder, Outcome, OutputInfo,
    PROTOCOL_VERSION, Reply, Request, ResourceId, Response, ServerMessage, Snapshot, VersionInfo,
    WindowInfo, WindowMode, WorkspaceInfo, decode, encode,
};

const WINDOW: ResourceId = ResourceId {
    index: 3,
    generation: 1,
};
const WORKSPACE: ResourceId = ResourceId {
    index: 0,
    generation: 0,
};
const OUTPUT: ResourceId = ResourceId {
    index: 1,
    generation: 4,
};

fn window_info() -> WindowInfo {
    WindowInfo {
        id: WINDOW,
        app_id: Some("org.example.Editor".into()),
        title: None,
        workspace: WORKSPACE,
        mode: WindowMode::Tiled,
        focused: true,
    }
}

fn workspace_info() -> WorkspaceInfo {
    WorkspaceInfo {
        id: WORKSPACE,
        name: "1".into(),
        output: Some(OUTPUT),
        focused_window: Some(WINDOW),
        windows: vec![WINDOW],
    }
}

fn output_info() -> OutputInfo {
    OutputInfo {
        id: OUTPUT,
        name: "eDP-1".into(),
        x: -1920,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.25,
        workspace: WORKSPACE,
    }
}

fn payload_of(frame: &[u8]) -> Vec<u8> {
    let mut frames = FrameDecoder::default().push(frame).unwrap();
    assert_eq!(frames.len(), 1);
    frames.remove(0)
}

fn round_trip<T>(message: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    decode(&payload_of(&encode(message).unwrap())).unwrap()
}

#[test]
fn requests_put_the_method_and_params_beside_the_request_id() {
    let request = Request {
        request_id: 7,
        expected_revision: Some(41),
        call: Call::MoveWindow {
            window: WINDOW,
            direction: Direction::Left,
        },
    };

    let json = String::from_utf8(payload_of(&encode(&request).unwrap())).unwrap();

    assert_eq!(
        json,
        r#"{"request_id":7,"expected_revision":41,"method":"move_window","params":{"window":{"index":3,"generation":1},"direction":"left"}}"#
    );
    assert_eq!(round_trip(&request), request);
}

#[test]
fn requests_without_params_omit_them() {
    let request: Request = decode(br#"{"request_id":1,"method":"get_windows"}"#).unwrap();

    assert_eq!(
        request,
        Request {
            request_id: 1,
            expected_revision: None,
            call: Call::GetWindows,
        }
    );
    assert_eq!(
        serde_json::to_string(&request).unwrap(),
        r#"{"request_id":1,"method":"get_windows"}"#
    );
}

#[test]
fn every_call_survives_a_round_trip() {
    let calls = [
        Call::GetVersion,
        Call::GetOutputs,
        Call::GetWorkspaces,
        Call::GetWindows,
        Call::GetConfig,
        Call::GetDiagnostics,
        Call::FocusWindow { window: WINDOW },
        Call::MoveWindow {
            window: WINDOW,
            direction: Direction::Down,
        },
        Call::SetLayout {
            workspace: WORKSPACE,
            layout: tilling_ipc::Layout::Bsp,
        },
        Call::ResizeWindow {
            window: WINDOW,
            delta: -250,
        },
        Call::SetWindowMode {
            window: WINDOW,
            mode: WindowMode::Fullscreen,
        },
        Call::SwitchWorkspace {
            workspace: WORKSPACE,
        },
        Call::ReloadConfig,
        Call::RequestLock,
        Call::Quit,
        Call::Subscribe,
    ];

    for (request_id, call) in (0_u64..).zip(calls) {
        let request = Request {
            request_id,
            expected_revision: None,
            call,
        };
        assert_eq!(round_trip(&request), request);
    }
}

#[test]
fn unknown_methods_and_missing_params_do_not_decode() {
    let unknown = decode::<Request>(br#"{"request_id":1,"method":"run_shell","params":{}}"#);
    let incomplete = decode::<Request>(br#"{"request_id":1,"method":"focus_window"}"#);
    let untagged = decode::<Request>(br#"{"request_id":1}"#);

    assert!(matches!(unknown, Err(DocumentError::Mismatch(_))));
    assert!(matches!(incomplete, Err(DocumentError::Mismatch(_))));
    assert!(matches!(untagged, Err(DocumentError::Mismatch(_))));
}

#[test]
fn responses_distinguish_every_outcome() {
    let outcomes = [
        (
            Outcome::Success {
                result: Reply::Accepted { revision: 42 },
            },
            r#"{"type":"response","request_id":9,"status":"success","result":{"kind":"accepted","data":{"revision":42}}}"#,
        ),
        (
            Outcome::ValidationError {
                message: "delta is out of range".into(),
            },
            r#"{"type":"response","request_id":9,"status":"validation_error","message":"delta is out of range"}"#,
        ),
        (
            Outcome::RevisionConflict {
                current_revision: 43,
            },
            r#"{"type":"response","request_id":9,"status":"revision_conflict","current_revision":43}"#,
        ),
        (
            Outcome::NotFound {
                message: "window 3v1 does not exist".into(),
            },
            r#"{"type":"response","request_id":9,"status":"not_found","message":"window 3v1 does not exist"}"#,
        ),
        (
            Outcome::NotPermitted {
                message: "session is locked".into(),
            },
            r#"{"type":"response","request_id":9,"status":"not_permitted","message":"session is locked"}"#,
        ),
    ];

    for (outcome, expected) in outcomes {
        let message = ServerMessage::Response(Response {
            request_id: 9,
            outcome,
        });
        assert_eq!(serde_json::to_string(&message).unwrap(), expected);
        assert_eq!(round_trip(&message), message);
    }
}

#[test]
fn query_replies_survive_a_round_trip() {
    let replies = [
        Reply::Version(VersionInfo {
            compositor: "0.1.0".into(),
            protocol: PROTOCOL_VERSION,
        }),
        Reply::Outputs(vec![output_info()]),
        Reply::Workspaces(vec![workspace_info()]),
        Reply::Windows(vec![window_info()]),
        Reply::Config(tilling_ipc::ConfigInfo {
            revision: 2,
            path: None,
        }),
        Reply::Diagnostics(tilling_ipc::DiagnosticsInfo {
            revision: 2,
            outputs: 1,
            workspaces: 9,
            windows: 3,
        }),
        Reply::Subscribed(Snapshot {
            sequence: 0,
            revision: 2,
            outputs: vec![output_info()],
            workspaces: vec![workspace_info()],
            windows: vec![window_info()],
        }),
    ];

    for result in replies {
        let message = ServerMessage::Response(Response {
            request_id: 1,
            outcome: Outcome::Success { result },
        });
        assert_eq!(round_trip(&message), message);
    }
}

#[test]
fn events_carry_sequence_and_revision() {
    let message = ServerMessage::Event(EventFrame {
        sequence: 12,
        revision: 42,
        event: Event::FocusChanged {
            window: Some(WINDOW),
        },
    });

    assert_eq!(
        serde_json::to_string(&message).unwrap(),
        r#"{"type":"event","sequence":12,"revision":42,"event":"focus_changed","data":{"window":{"index":3,"generation":1}}}"#
    );
    assert_eq!(round_trip(&message), message);
}

#[test]
fn every_event_survives_a_round_trip() {
    let events = [
        Event::WindowOpened(window_info()),
        Event::WindowChanged(window_info()),
        Event::WindowClosed { window: WINDOW },
        Event::FocusChanged { window: None },
        Event::WorkspaceChanged(workspace_info()),
        Event::OutputChanged(output_info()),
        Event::OutputRemoved { output: OUTPUT },
        Event::EventsDropped,
    ];

    for (sequence, event) in (0_u64..).zip(events) {
        let message = ServerMessage::Event(EventFrame {
            sequence,
            revision: 5,
            event,
        });
        assert_eq!(round_trip(&message), message);
    }
}
