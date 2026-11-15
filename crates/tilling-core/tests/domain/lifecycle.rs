use tilling_core::layout::{Direction, Gaps};
use tilling_core::{Command, Effect, ModeChange, RestoreMode, TransitionError, WindowMode};

use crate::scenario::{
    apply, area, create_workspace, desktop_with_output, metadata, open, open_in, opened_window,
    placement_of,
};

#[test]
fn new_window_enters_the_active_workspace_and_takes_focus() {
    let (mut state, _) = desktop_with_output();

    let first = open(&mut state, "terminal");
    let second = open(&mut state, "browser");

    let workspace = state.workspace(state.active_workspace()).unwrap();
    assert_eq!(workspace.windows(), [first, second]);
    assert_eq!(state.focused_window(), Some(second));
}

#[test]
fn window_opened_on_a_hidden_workspace_leaves_the_visible_focus_alone() {
    let (mut state, _) = desktop_with_output();
    let visible = open(&mut state, "terminal");
    let hidden_workspace = create_workspace(&mut state, "2");

    let hidden = open_in(
        &mut state,
        "mail",
        Some(hidden_workspace),
        WindowMode::Tiled,
    );

    assert_eq!(state.focused_window(), Some(visible));
    assert_eq!(state.window(hidden).unwrap().workspace(), hidden_workspace);
    assert_eq!(
        state.workspace(hidden_workspace).unwrap().focused_window(),
        Some(hidden)
    );
}

#[test]
fn closing_the_focused_window_focuses_the_most_recently_focused_one() {
    let (mut state, _) = desktop_with_output();
    let a = open(&mut state, "a");
    let b = open(&mut state, "b");
    let c = open(&mut state, "c");
    apply(&mut state, Command::FocusWindow { window: a });

    let transition = apply(&mut state, Command::CloseWindow { window: a });
    assert!(transition.effects.contains(&Effect::FocusChanged {
        previous: Some(a),
        current: Some(c),
    }));

    apply(&mut state, Command::CloseWindow { window: c });
    assert_eq!(state.focused_window(), Some(b));

    apply(&mut state, Command::CloseWindow { window: b });
    let workspace = state.workspace(state.active_workspace()).unwrap();
    assert_eq!(state.focused_window(), None);
    assert!(workspace.layout().windows().is_empty());
}

#[test]
fn moving_a_window_to_another_workspace_keeps_its_identity_and_properties() {
    let (mut state, _) = desktop_with_output();
    let stays = open(&mut state, "terminal");
    let moves = open(&mut state, "browser");
    let before = state.window(moves).unwrap().clone();
    let source = state.active_workspace();
    let target = create_workspace(&mut state, "2");

    let transition = apply(
        &mut state,
        Command::MoveWindowToWorkspace {
            window: moves,
            workspace: target,
        },
    );

    let after = state.window(moves).unwrap();
    assert_eq!(after.metadata(), before.metadata());
    assert_eq!(after.mode(), before.mode());
    assert_eq!(after.workspace(), target);
    assert_eq!(state.workspace(target).unwrap().windows(), [moves]);
    assert_eq!(state.workspace(source).unwrap().windows(), [stays]);
    assert_eq!(transition.revision, state.revision());
}

#[test]
fn stale_identifiers_are_rejected_without_changing_state() {
    let (mut state, _) = desktop_with_output();
    let closed = open(&mut state, "terminal");
    apply(&mut state, Command::CloseWindow { window: closed });
    let reused_slot = open(&mut state, "browser");
    let before = state.clone();

    let commands = [
        Command::FocusWindow { window: closed },
        Command::CloseWindow { window: closed },
        Command::ToggleSplit { window: closed },
        Command::MoveWindow {
            window: closed,
            direction: Direction::Left,
        },
    ];
    for command in commands {
        assert_eq!(
            state.apply(command),
            Err(TransitionError::UnknownWindow(closed))
        );
    }

    assert_ne!(closed, reused_slot);
    assert_eq!(state, before);
}

#[test]
fn metadata_changes_do_not_affect_identity_or_placement() {
    let (mut state, _) = desktop_with_output();
    let window = open(&mut state, "terminal");
    open(&mut state, "browser");
    let tile = placement_of(&state, window, Gaps::default());

    apply(
        &mut state,
        Command::UpdateWindowMetadata {
            window,
            metadata: metadata("renamed"),
        },
    );

    assert_eq!(
        state.window(window).unwrap().metadata(),
        &metadata("renamed")
    );
    assert_eq!(placement_of(&state, window, Gaps::default()), tile);
}

#[test]
fn fullscreen_returns_a_tiled_window_to_its_tile() {
    let (mut state, output) = desktop_with_output();
    let window = open(&mut state, "video");
    open(&mut state, "terminal");
    let tile = placement_of(&state, window, Gaps::default());
    let enter = ModeChange::EnterFullscreen;
    let leave = ModeChange::LeaveFullscreen;

    apply(
        &mut state,
        Command::ChangeWindowMode {
            window,
            change: enter,
        },
    );
    assert_eq!(
        state.window(window).unwrap().mode(),
        WindowMode::Fullscreen(RestoreMode::Tiled)
    );
    assert_eq!(
        placement_of(&state, window, Gaps::default()),
        state.output(output).unwrap().area()
    );
    assert_eq!(state.focused_window(), Some(window));

    apply(
        &mut state,
        Command::ChangeWindowMode {
            window,
            change: leave,
        },
    );
    assert_eq!(state.window(window).unwrap().mode(), WindowMode::Tiled);
    assert_eq!(placement_of(&state, window, Gaps::default()), tile);
}

#[test]
fn fullscreen_returns_a_floating_window_to_its_geometry() {
    let (mut state, _) = desktop_with_output();
    let geometry = area(100, 80, 640, 480);
    let window = open_in(&mut state, "dialog", None, WindowMode::Floating(geometry));
    let enter = ModeChange::EnterFullscreen;
    let leave = ModeChange::LeaveFullscreen;

    apply(
        &mut state,
        Command::ChangeWindowMode {
            window,
            change: enter,
        },
    );
    apply(
        &mut state,
        Command::ChangeWindowMode {
            window,
            change: leave,
        },
    );

    assert_eq!(
        state.window(window).unwrap().mode(),
        WindowMode::Floating(geometry)
    );
    assert_eq!(placement_of(&state, window, Gaps::default()), geometry);
}

#[test]
fn focusing_another_window_releases_fullscreen() {
    let (mut state, _) = desktop_with_output();
    let video = open(&mut state, "video");
    let terminal = open(&mut state, "terminal");
    let enter = ModeChange::EnterFullscreen;
    apply(
        &mut state,
        Command::ChangeWindowMode {
            window: video,
            change: enter,
        },
    );

    let transition = apply(&mut state, Command::FocusWindow { window: terminal });

    assert_eq!(state.window(video).unwrap().mode(), WindowMode::Tiled);
    assert!(
        transition
            .effects
            .contains(&Effect::WindowModeChanged(video))
    );
}

#[test]
fn fullscreen_window_rejects_other_mode_changes() {
    let (mut state, _) = desktop_with_output();
    let window = open(&mut state, "video");
    let enter = ModeChange::EnterFullscreen;
    apply(
        &mut state,
        Command::ChangeWindowMode {
            window,
            change: enter,
        },
    );

    let rejected = state.apply(Command::ChangeWindowMode {
        window,
        change: ModeChange::Tile,
    });

    assert_eq!(rejected, Err(TransitionError::FullscreenLocked(window)));
}

#[test]
fn directional_commands_on_an_empty_workspace_change_nothing() {
    let (mut state, _) = desktop_with_output();
    let revision = state.revision();

    let transition = apply(
        &mut state,
        Command::FocusDirection {
            direction: Direction::Right,
        },
    );

    assert!(transition.effects.is_empty());
    assert_eq!(transition.revision, revision);
}

#[test]
fn directional_focus_and_movement_follow_the_layout() {
    let (mut state, _) = desktop_with_output();
    let left = open(&mut state, "left");
    let top_right = open(&mut state, "top-right");
    let bottom_right = open(&mut state, "bottom-right");
    let top_right_tile = placement_of(&state, top_right, Gaps::default());

    apply(
        &mut state,
        Command::FocusDirection {
            direction: Direction::Left,
        },
    );
    assert_eq!(state.focused_window(), Some(left));

    apply(
        &mut state,
        Command::MoveWindow {
            window: left,
            direction: Direction::Right,
        },
    );
    assert_eq!(placement_of(&state, left, Gaps::default()), top_right_tile);
    assert_eq!(state.focused_window(), Some(left));
    assert!(state.window(bottom_right).is_some());
}

#[test]
fn closing_a_parent_clears_transient_links() {
    let (mut state, _) = desktop_with_output();
    let parent = open(&mut state, "editor");
    let transition = apply(
        &mut state,
        Command::OpenWindow {
            metadata: metadata("dialog"),
            workspace: None,
            transient_for: Some(parent),
            mode: WindowMode::Floating(area(10, 10, 300, 200)),
        },
    );
    let dialog = opened_window(&transition);
    assert_eq!(state.window(dialog).unwrap().transient_for(), Some(parent));

    apply(&mut state, Command::CloseWindow { window: parent });

    assert_eq!(state.window(dialog).unwrap().transient_for(), None);
    let orphan = state.apply(Command::OpenWindow {
        metadata: metadata("dialog"),
        workspace: None,
        transient_for: Some(parent),
        mode: WindowMode::Tiled,
    });
    assert_eq!(orphan, Err(TransitionError::UnknownWindow(parent)));
}

#[test]
fn floating_windows_stay_out_of_the_layout() {
    let (mut state, _) = desktop_with_output();
    let tiled = open(&mut state, "terminal");
    let geometry = area(40, 40, 400, 300);
    let floating = open_in(
        &mut state,
        "calculator",
        None,
        WindowMode::Floating(geometry),
    );
    let layout_windows = |state: &tilling_core::DesktopState| {
        let workspace = state.workspace(state.active_workspace()).unwrap();
        workspace.layout().windows()
    };
    assert_eq!(layout_windows(&state), [tiled]);

    apply(
        &mut state,
        Command::ChangeWindowMode {
            window: floating,
            change: ModeChange::Tile,
        },
    );
    assert_eq!(layout_windows(&state), [tiled, floating]);

    apply(
        &mut state,
        Command::ChangeWindowMode {
            window: floating,
            change: ModeChange::Float(geometry),
        },
    );
    assert_eq!(layout_windows(&state), [tiled]);
}

#[test]
fn workspace_names_must_be_unique_and_non_empty() {
    let (mut state, _) = desktop_with_output();
    create_workspace(&mut state, "web");

    let duplicate = state.apply(Command::CreateWorkspace { name: "web".into() });
    let empty = state.apply(Command::CreateWorkspace {
        name: String::new(),
    });

    assert_eq!(
        duplicate,
        Err(TransitionError::DuplicateWorkspaceName("web".into()))
    );
    assert_eq!(empty, Err(TransitionError::EmptyWorkspaceName));
}
