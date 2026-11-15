use tilling_ipc::endpoint::InstanceName;
use tilling_ipc::{
    DiagnosticsInfo, Event, EventFrame, Layout, Outcome, OutputInfo, Reply, ResourceId, Snapshot,
    WindowInfo, WindowMode, WorkspaceInfo,
};

const ABSENT: &str = "-";
const MILLISECONDS_PER_SECOND: u64 = 1_000;
const REPLACEMENT: char = '\u{fffd}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
}

pub fn outcome(outcome: &Outcome, format: Format) -> Result<String, serde_json::Error> {
    match format {
        Format::Json => serde_json::to_string(outcome),
        Format::Text => Ok(describe_outcome(outcome)),
    }
}

pub fn event(frame: &EventFrame, format: Format) -> Result<String, serde_json::Error> {
    match format {
        Format::Json => serde_json::to_string(frame),
        Format::Text => Ok(describe_event(frame)),
    }
}

pub fn instances(instances: &[InstanceName], format: Format) -> Result<String, serde_json::Error> {
    let names: Vec<&str> = instances.iter().map(InstanceName::as_str).collect();
    match format {
        Format::Json => serde_json::to_string(&names),
        Format::Text => Ok(names.join("\n")),
    }
}

fn describe_outcome(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Success { result } => describe_reply(result),
        Outcome::ValidationError { message } => format!("invalid request: {}", printable(message)),
        Outcome::RevisionConflict { current_revision } => {
            format!("revision conflict: the compositor is at revision {current_revision}")
        }
        Outcome::NotFound { message } => format!("not found: {}", printable(message)),
        Outcome::NotPermitted { message } => format!("not permitted: {}", printable(message)),
    }
}

fn describe_reply(reply: &Reply) -> String {
    match reply {
        Reply::Version(version) => format!(
            "{} (protocol {}.{})",
            printable(&version.compositor),
            version.protocol.major,
            version.protocol.minor
        ),
        Reply::Outputs(outputs) => lines(outputs.iter().map(describe_output)),
        Reply::Workspaces(workspaces) => lines(workspaces.iter().map(describe_workspace)),
        Reply::Windows(windows) => lines(windows.iter().map(describe_window)),
        Reply::Config(config) => format!(
            "revision {} from {}",
            config.revision,
            config
                .path
                .as_deref()
                .map_or_else(|| "built-in defaults".to_owned(), printable)
        ),
        Reply::Diagnostics(diagnostics) => describe_diagnostics(diagnostics),
        Reply::Accepted { revision } => format!("accepted at revision {revision}"),
        Reply::OutputChangePending {
            revision,
            confirm_within_ms,
        } => format!(
            "changed at revision {revision}, reverted in {} s unless `tillingctl output confirm` runs",
            confirm_within_ms.div_ceil(MILLISECONDS_PER_SECOND)
        ),
        Reply::Subscribed(snapshot) => describe_snapshot(snapshot),
    }
}

fn describe_diagnostics(diagnostics: &DiagnosticsInfo) -> String {
    let desktop = format!(
        "revision {}: {} outputs, {} workspaces, {} windows",
        diagnostics.revision, diagnostics.outputs, diagnostics.workspaces, diagnostics.windows
    );
    let presentation = format!(
        "frames: {} presented, {} with damage",
        diagnostics.frames_presented, diagnostics.frames_damaged
    );
    let input = format!("input devices: {}", diagnostics.input_devices);
    let process = format!(
        "process: {} KiB resident, {} descriptors, {} ms of processor time",
        diagnostics.resident_kib, diagnostics.open_descriptors, diagnostics.cpu_time_ms
    );
    lines([desktop, presentation, input, process].into_iter())
}

fn describe_snapshot(snapshot: &Snapshot) -> String {
    let header = format!("snapshot #{} r{}", snapshot.sequence, snapshot.revision);
    let outputs = snapshot.outputs.iter().map(describe_output);
    let workspaces = snapshot.workspaces.iter().map(describe_workspace);
    let windows = snapshot.windows.iter().map(describe_window);
    lines(
        std::iter::once(header)
            .chain(outputs)
            .chain(workspaces)
            .chain(windows),
    )
}

fn describe_event(frame: &EventFrame) -> String {
    let change = match &frame.event {
        Event::WindowOpened(window) => format!("opened {}", describe_window(window)),
        Event::WindowChanged(window) => format!("changed {}", describe_window(window)),
        Event::WindowClosed { window } => format!("closed window {}", identifier(*window)),
        Event::FocusChanged { window } => format!("focused window {}", optional(*window)),
        Event::WorkspaceChanged(workspace) => format!("changed {}", describe_workspace(workspace)),
        Event::OutputChanged(output) => format!("changed {}", describe_output(output)),
        Event::OutputRemoved { output } => format!("removed output {}", identifier(*output)),
        Event::EventsDropped => "events dropped".to_owned(),
    };
    format!("#{} r{} {change}", frame.sequence, frame.revision)
}

fn describe_output(output: &OutputInfo) -> String {
    format!(
        "output {}  {}  {}x{}{:+}{:+}  scale {}  workspace {}",
        identifier(output.id),
        printable(&output.name),
        output.width,
        output.height,
        output.x,
        output.y,
        output.scale,
        identifier(output.workspace)
    )
}

fn describe_workspace(workspace: &WorkspaceInfo) -> String {
    format!(
        "workspace {}  {}  {}  {}  output {}  focused {}  windows {}",
        identifier(workspace.id),
        printable(&workspace.name),
        layout_name(workspace.layout),
        if workspace.active {
            "active"
        } else {
            "inactive"
        },
        optional(workspace.output),
        optional(workspace.focused_window),
        workspace.windows.len()
    )
}

fn describe_window(window: &WindowInfo) -> String {
    format!(
        "window {}  {}  {}  workspace {}  app {}  title {}",
        identifier(window.id),
        mode_name(window.mode),
        if window.focused {
            "focused"
        } else {
            "unfocused"
        },
        identifier(window.workspace),
        window.app_id.as_deref().map_or_else(absent, printable),
        window.title.as_deref().map_or_else(absent, printable)
    )
}

fn layout_name(layout: Layout) -> &'static str {
    match layout {
        Layout::Bsp => "bsp",
        Layout::MasterStack => "master-stack",
        Layout::Monocle => "monocle",
    }
}

fn mode_name(mode: WindowMode) -> &'static str {
    match mode {
        WindowMode::Tiled => "tiled",
        WindowMode::Floating => "floating",
        WindowMode::Fullscreen => "fullscreen",
    }
}

fn identifier(resource: ResourceId) -> String {
    format!("{}:{}", resource.index, resource.generation)
}

fn optional(resource: Option<ResourceId>) -> String {
    resource.map_or_else(absent, identifier)
}

fn absent() -> String {
    ABSENT.to_owned()
}

fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                REPLACEMENT
            } else {
                character
            }
        })
        .collect()
}

fn lines(entries: impl Iterator<Item = String>) -> String {
    entries.collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use tilling_ipc::{PROTOCOL_VERSION, VersionInfo};

    use super::*;

    const WINDOW: ResourceId = ResourceId {
        index: 3,
        generation: 1,
    };
    const WORKSPACE: ResourceId = ResourceId {
        index: 0,
        generation: 0,
    };

    fn window(title: &str) -> WindowInfo {
        WindowInfo {
            id: WINDOW,
            app_id: Some("org.example.Editor".to_owned()),
            title: Some(title.to_owned()),
            workspace: WORKSPACE,
            mode: WindowMode::Tiled,
            focused: true,
        }
    }

    fn text(outcome: &Outcome) -> String {
        super::outcome(outcome, Format::Text).unwrap()
    }

    #[test]
    fn version_names_the_compositor_and_its_protocol() {
        let reply = Outcome::Success {
            result: Reply::Version(VersionInfo {
                compositor: "tilling 0.1.0".to_owned(),
                protocol: PROTOCOL_VERSION,
            }),
        };

        assert_eq!(text(&reply), "tilling 0.1.0 (protocol 1.2)");
    }

    #[test]
    fn workspaces_name_their_layout_and_whether_they_are_active() {
        let reply = Outcome::Success {
            result: Reply::Workspaces(vec![WorkspaceInfo {
                id: WORKSPACE,
                name: "web".to_owned(),
                output: None,
                focused_window: Some(WINDOW),
                windows: vec![WINDOW],
                layout: Layout::MasterStack,
                active: true,
            }]),
        };

        assert_eq!(
            text(&reply),
            "workspace 0:0  web  master-stack  active  output -  focused 3:1  windows 1"
        );
    }

    #[test]
    fn windows_are_listed_one_per_line() {
        let reply = Outcome::Success {
            result: Reply::Windows(vec![window("Notes"), window("Draft")]),
        };

        assert_eq!(
            text(&reply),
            "window 3:1  tiled  focused  workspace 0:0  app org.example.Editor  title Notes\n\
             window 3:1  tiled  focused  workspace 0:0  app org.example.Editor  title Draft"
        );
    }

    #[test]
    fn control_characters_from_clients_never_reach_the_terminal() {
        let reply = Outcome::Success {
            result: Reply::Windows(vec![window("evil\u{1b}[2J\ntitle")]),
        };

        let rendered = text(&reply);

        assert!(rendered.ends_with("title evil\u{fffd}[2J\u{fffd}title"));
        assert!(!rendered.contains(['\u{1b}', '\n']));
    }

    #[test]
    fn pending_output_change_says_how_long_it_waits_for_confirmation() {
        let reply = Outcome::Success {
            result: Reply::OutputChangePending {
                revision: 12,
                confirm_within_ms: 14_200,
            },
        };

        assert_eq!(
            text(&reply),
            "changed at revision 12, reverted in 15 s unless `tillingctl output confirm` runs"
        );
    }

    #[test]
    fn diagnostics_report_presentation_input_and_process_usage() {
        let reply = Outcome::Success {
            result: Reply::Diagnostics(DiagnosticsInfo {
                revision: 4,
                outputs: 2,
                workspaces: 9,
                windows: 3,
                input_devices: 2,
                frames_presented: 1200,
                frames_damaged: 310,
                resident_kib: 48_212,
                open_descriptors: 37,
                cpu_time_ms: 950,
            }),
        };

        assert_eq!(
            text(&reply),
            "revision 4: 2 outputs, 9 workspaces, 3 windows\n\
             frames: 1200 presented, 310 with damage\n\
             input devices: 2\n\
             process: 48212 KiB resident, 37 descriptors, 950 ms of processor time"
        );
    }

    #[test]
    fn empty_lists_render_as_nothing() {
        let reply = Outcome::Success {
            result: Reply::Windows(Vec::new()),
        };

        assert_eq!(text(&reply), "");
    }

    #[test]
    fn refusals_say_why() {
        let conflict = Outcome::RevisionConflict {
            current_revision: 9,
        };
        let missing = Outcome::NotFound {
            message: "window 3:1 does not exist".to_owned(),
        };

        assert_eq!(
            text(&conflict),
            "revision conflict: the compositor is at revision 9"
        );
        assert_eq!(text(&missing), "not found: window 3:1 does not exist");
    }

    #[test]
    fn events_carry_their_sequence_and_revision() {
        let frame = EventFrame {
            sequence: 41,
            revision: 7,
            event: Event::FocusChanged { window: None },
        };

        assert_eq!(
            event(&frame, Format::Text).unwrap(),
            "#41 r7 focused window -"
        );
    }

    #[test]
    fn json_output_is_the_protocol_document() {
        let reply = Outcome::Success {
            result: Reply::Accepted { revision: 4 },
        };

        assert_eq!(
            outcome(&reply, Format::Json).unwrap(),
            r#"{"status":"success","result":{"kind":"accepted","data":{"revision":4}}}"#
        );
    }
}
