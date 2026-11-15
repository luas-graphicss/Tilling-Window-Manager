use tilling_compositor::{CompositorError, SessionSnapshot, VirtualOutput};
use tilling_core::WindowId;
use tilling_core::geometry::LogicalRect;
use tilling_ipc::client::Client;
use tilling_ipc::{Call, Outcome, OutputInfo, Reply, ResourceId, WorkspaceInfo};

use crate::harness::Harness;

const PRIMARY: &str = "HEADLESS-1";
const SPARE: &str = "HEADLESS-2";
const PRIMARY_WIDTH: i32 = 1280;
const HOTPLUG_CYCLES: usize = 50;

fn virtual_output(name: &str, width: u32, height: u32) -> VirtualOutput {
    VirtualOutput {
        name: name.to_owned(),
        width,
        height,
    }
}

fn spare() -> VirtualOutput {
    virtual_output(SPARE, 1920, 1080)
}

fn reply(control: &mut Client, call: Call) -> Reply {
    match control.call(call, None) {
        Ok(Outcome::Success { result }) => result,
        other => panic!("control request was not accepted: {other:?}"),
    }
}

fn outputs(control: &mut Client) -> Vec<OutputInfo> {
    match reply(control, Call::GetOutputs) {
        Reply::Outputs(outputs) => outputs,
        other => panic!("unexpected reply to get outputs: {other:?}"),
    }
}

fn workspaces(control: &mut Client) -> Vec<WorkspaceInfo> {
    match reply(control, Call::GetWorkspaces) {
        Reply::Workspaces(workspaces) => workspaces,
        other => panic!("unexpected reply to get workspaces: {other:?}"),
    }
}

fn output_named(control: &mut Client, name: &str) -> OutputInfo {
    outputs(control)
        .into_iter()
        .find(|output| output.name == name)
        .unwrap_or_else(|| panic!("output {name} is not connected"))
}

fn switch_to(control: &mut Client, workspace: ResourceId) {
    reply(control, Call::SwitchWorkspace { workspace });
}

fn placements(snapshot: &SessionSnapshot) -> Vec<(WindowId, LogicalRect)> {
    snapshot
        .scene
        .windows()
        .iter()
        .map(|window| (window.id, window.frame))
        .collect()
}

#[test]
fn second_output_sits_beside_the_first_and_shows_another_workspace() {
    let harness = Harness::start();
    let mut control = harness.control();

    harness.plug(spare());

    let primary = output_named(&mut control, PRIMARY);
    let added = output_named(&mut control, SPARE);
    assert_eq!((primary.x, primary.y), (0, 0));
    assert_eq!(
        (added.x, added.y, added.width, added.height),
        (PRIMARY_WIDTH, 0, 1920, 1080)
    );
    assert_ne!(primary.workspace, added.workspace);
}

#[test]
fn window_follows_the_active_workspace_to_the_remaining_output() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let mut control = harness.control();
    harness.plug(spare());
    let spare_workspace = output_named(&mut control, SPARE).workspace;
    switch_to(&mut control, spare_workspace);
    let window = client.create_toplevel("org.example.Browser", "docs");
    client.map(window).unwrap();
    client.settle().unwrap();
    let on_spare = harness.published(1);

    harness.unplug(SPARE);
    client.settle().unwrap();
    let on_primary = harness.published(1);

    let before = on_spare.scene.windows()[0];
    let after = on_primary.scene.windows()[0];
    assert!(before.frame.x() >= PRIMARY_WIDTH);
    assert!(after.frame.x() < PRIMARY_WIDTH);
    assert_eq!(after.id, before.id);
    assert_eq!(on_primary.focused, Some(before.id));
    assert_eq!(
        output_named(&mut control, PRIMARY).workspace,
        spare_workspace
    );
}

#[test]
fn fifty_hotplug_cycles_keep_windows_workspaces_and_focus() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let mut control = harness.control();
    let primary_workspace = output_named(&mut control, PRIMARY).workspace;
    harness.plug(spare());
    let spare_workspace = output_named(&mut control, SPARE).workspace;
    switch_to(&mut control, spare_workspace);
    let browser = client.create_toplevel("org.example.Browser", "docs");
    client.map(browser).unwrap();
    switch_to(&mut control, primary_workspace);
    let editor = client.create_toplevel("org.example.Editor", "notes");
    client.map(editor).unwrap();
    client.settle().unwrap();
    let before = harness.published(2);
    let known_workspaces = workspaces(&mut control).len();

    for _ in 0..HOTPLUG_CYCLES {
        harness.unplug(SPARE);
        harness.plug(spare());
    }
    client.settle().unwrap();
    let after = harness.published(2);

    assert_eq!(placements(&after), placements(&before));
    assert_eq!(after.focused, before.focused);
    assert!(after.focused.is_some());
    assert_eq!(after.mapped_windows, 2);
    assert_eq!(workspaces(&mut control).len(), known_workspaces);
    assert_eq!(output_named(&mut control, SPARE).workspace, spare_workspace);
}

#[test]
fn losing_the_last_output_parks_the_windows_until_one_returns() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    client.settle().unwrap();
    let lit = harness.published(1);

    harness.unplug(PRIMARY);
    let dark = harness.published(0);
    let idle = harness.idle_frames();
    harness.plug(virtual_output(PRIMARY, 1280, 720));
    client.settle().unwrap();
    let relit = harness.published(1);

    assert_eq!(dark.focused, None);
    assert_eq!(dark.mapped_windows, 1);
    assert_eq!(harness.snapshot().frames.presented, idle.presented + 1);
    assert_eq!(relit.focused, lit.focused);
    assert_eq!(placements(&relit), placements(&lit));
}

#[test]
fn duplicate_unknown_and_empty_outputs_are_refused() {
    let harness = Harness::start();
    let session = harness.session();

    let duplicate = session.plug_output(virtual_output(PRIMARY, 800, 600));
    let unknown = session.unplug_output("HEADLESS-9");
    let empty = session.plug_output(virtual_output(SPARE, 0, 1080));

    assert!(matches!(duplicate, Err(CompositorError::OutputInUse(name)) if name == PRIMARY));
    assert!(matches!(unknown, Err(CompositorError::UnknownOutput(name)) if name == "HEADLESS-9"));
    assert!(matches!(
        empty,
        Err(CompositorError::OutputSize {
            width: 0,
            height: 1080
        })
    ));
    assert_eq!(harness.published(0).mapped_windows, 0);
}
