use tilling_core::geometry::{LogicalRect, Scale};
use tilling_core::layout::Gaps;
use tilling_core::{
    Command, DesktopState, Effect, OutputId, Transition, WindowId, WindowMetadata, WindowMode,
    WorkspaceId,
};

pub fn area(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
    LogicalRect::new(x, y, width, height)
        .unwrap_or_else(|error| panic!("invalid test area: {error}"))
}

pub fn apply(state: &mut DesktopState, command: Command) -> Transition {
    let description = format!("{command:?}");
    let transition = state
        .apply(command)
        .unwrap_or_else(|error| panic!("{description} was rejected: {error}"));
    if let Err(violation) = state.verify() {
        panic!("{description} broke an invariant: {violation}");
    }
    transition
}

pub fn add_output(state: &mut DesktopState, name: &str, area: LogicalRect) -> OutputId {
    let transition = apply(
        state,
        Command::AddOutput {
            name: name.to_owned(),
            area,
            scale: Scale::ONE,
        },
    );
    transition
        .effects
        .iter()
        .find_map(|effect| match effect {
            Effect::OutputAdded(output) => Some(*output),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no output was added"))
}

pub fn desktop_with_output() -> (DesktopState, OutputId) {
    let mut state = DesktopState::new("1");
    let output = add_output(&mut state, "primary", area(0, 0, 1920, 1080));
    (state, output)
}

pub fn metadata(app_id: &str) -> WindowMetadata {
    WindowMetadata {
        app_id: Some(app_id.to_owned()),
        title: Some(format!("{app_id} window")),
    }
}

pub fn open(state: &mut DesktopState, app_id: &str) -> WindowId {
    open_in(state, app_id, None, WindowMode::Tiled)
}

pub fn open_in(
    state: &mut DesktopState,
    app_id: &str,
    workspace: Option<WorkspaceId>,
    mode: WindowMode,
) -> WindowId {
    let transition = apply(
        state,
        Command::OpenWindow {
            metadata: metadata(app_id),
            workspace,
            transient_for: None,
            mode,
        },
    );
    opened_window(&transition)
}

pub fn opened_window(transition: &Transition) -> WindowId {
    transition
        .effects
        .iter()
        .find_map(|effect| match effect {
            Effect::WindowOpened(window) => Some(*window),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no window was opened"))
}

pub fn create_workspace(state: &mut DesktopState, name: &str) -> WorkspaceId {
    let transition = apply(
        state,
        Command::CreateWorkspace {
            name: name.to_owned(),
        },
    );
    transition
        .effects
        .iter()
        .find_map(|effect| match effect {
            Effect::WorkspaceCreated(workspace) => Some(*workspace),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no workspace was created"))
}

pub fn placement_of(state: &DesktopState, window: WindowId, gaps: Gaps) -> LogicalRect {
    state
        .window(window)
        .and_then(|entry| state.placements(entry.workspace(), gaps))
        .and_then(|placements| {
            placements
                .into_iter()
                .find(|placement| placement.window == window)
        })
        .map(|placement| placement.area)
        .unwrap_or_else(|| panic!("window {window} is not placed"))
}
