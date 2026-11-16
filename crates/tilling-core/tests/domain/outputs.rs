use tilling_core::geometry::{Insets, Scale};
use tilling_core::layout::Gaps;
use tilling_core::{Command, DesktopState, Effect, ModeChange, TransitionError, WindowMode};

use crate::scenario::{add_output, apply, area, desktop_with_output, open, open_in, placement_of};

#[test]
fn losing_every_output_parks_workspaces_without_dropping_windows() {
    let (mut state, output) = desktop_with_output();
    let window = open(&mut state, "terminal");
    let workspace = state.active_workspace();

    let transition = apply(&mut state, Command::RemoveOutput { output });

    assert!(transition.effects.contains(&Effect::FocusChanged {
        previous: Some(window),
        current: None,
    }));
    assert_eq!(state.workspace(workspace).unwrap().output(), None);
    assert_eq!(state.workspace(workspace).unwrap().windows(), [window]);
    assert_eq!(
        state.placements(workspace, Gaps::default()),
        Some(Vec::new())
    );

    let replacement = add_output(&mut state, "replacement", area(0, 0, 1280, 720));

    assert_eq!(state.output(replacement).unwrap().workspace(), workspace);
    assert_eq!(state.focused_window(), Some(window));
}

#[test]
fn removed_output_hands_the_active_workspace_to_the_remaining_output() {
    let (mut state, primary) = desktop_with_output();
    let secondary = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));
    let workspace = state.output(secondary).unwrap().workspace();
    apply(&mut state, Command::SwitchWorkspace { workspace });
    let window = open(&mut state, "browser");

    apply(&mut state, Command::RemoveOutput { output: secondary });

    assert_eq!(state.output(primary).unwrap().workspace(), workspace);
    assert_eq!(state.workspace(workspace).unwrap().output(), Some(primary));
    assert_eq!(state.focused_window(), Some(window));
}

#[test]
fn each_output_shows_its_own_workspace() {
    let (mut state, primary) = desktop_with_output();

    let secondary = add_output(&mut state, "secondary", area(-1280, 0, 1280, 1024));

    let first = state.output(primary).unwrap().workspace();
    let second = state.output(secondary).unwrap().workspace();
    assert_ne!(first, second);
    assert_eq!(state.workspace(second).unwrap().name(), "2");
    assert_eq!(state.workspace(second).unwrap().output(), Some(secondary));
    assert_eq!(state.active_workspace(), first);
}

#[test]
fn tiles_fill_the_output_minus_the_gaps() {
    let (mut state, _) = desktop_with_output();
    let first = open(&mut state, "terminal");
    let second = open(&mut state, "browser");
    let gaps = Gaps {
        inner: 10,
        outer: 20,
    };

    assert_eq!(placement_of(&state, first, gaps), area(20, 20, 935, 1040));
    assert_eq!(placement_of(&state, second, gaps), area(965, 20, 935, 1040));
}

#[test]
fn reconfiguring_an_output_rearranges_its_workspace() {
    let (mut state, output) = desktop_with_output();
    let window = open(&mut state, "terminal");
    let resized = area(-640, -360, 1280, 720);
    let scale = Scale::from_factor(1.5).unwrap();

    apply(
        &mut state,
        Command::ConfigureOutput {
            output,
            area: resized,
            scale,
        },
    );

    assert_eq!(placement_of(&state, window, Gaps::default()), resized);
    assert_eq!(state.output(output).unwrap().scale(), scale);
}

#[test]
fn invalid_output_commands_are_rejected() {
    let (mut state, output) = desktop_with_output();
    apply(&mut state, Command::RemoveOutput { output });
    let before = state.clone();

    let empty = state.apply(Command::AddOutput {
        name: "empty".into(),
        area: area(0, 0, 0, 1080),
        scale: Scale::ONE,
    });
    let stale = state.apply(Command::RemoveOutput { output });

    assert_eq!(empty, Err(TransitionError::EmptyOutputArea));
    assert_eq!(stale, Err(TransitionError::UnknownOutput(output)));
    assert_eq!(state, before);
}

#[test]
fn desktop_without_outputs_has_no_visible_focus() {
    let mut state = DesktopState::new("1");

    let window = open(&mut state, "terminal");

    let workspace = state.workspace(state.active_workspace()).unwrap();
    assert_eq!(workspace.focused_window(), Some(window));
    assert_eq!(state.focused_window(), None);
}

#[test]
fn reconnected_output_gets_back_the_workspace_it_was_showing() {
    let (mut state, primary) = desktop_with_output();
    let secondary = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));
    let shown = state.output(secondary).unwrap().workspace();
    let window = open_in(&mut state, "browser", Some(shown), WindowMode::Tiled);

    apply(&mut state, Command::RemoveOutput { output: secondary });
    let while_away = state.workspace(shown).unwrap().output();
    let returned = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));

    assert_eq!(while_away, Some(primary));
    assert_eq!(state.output(returned).unwrap().workspace(), shown);
    assert_eq!(state.workspace(shown).unwrap().output(), Some(returned));
    assert_eq!(state.workspace(shown).unwrap().windows(), [window]);
}

#[test]
fn repeated_hotplug_keeps_the_same_workspaces_and_windows() {
    let (mut state, _) = desktop_with_output();
    let mut secondary = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));
    let shown = state.output(secondary).unwrap().workspace();
    let window = open_in(&mut state, "browser", Some(shown), WindowMode::Tiled);
    let workspaces = state.workspaces().count();

    for _ in 0..50 {
        apply(&mut state, Command::RemoveOutput { output: secondary });
        secondary = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));
    }

    assert_eq!(state.workspaces().count(), workspaces);
    assert_eq!(state.output(secondary).unwrap().workspace(), shown);
    assert_eq!(state.workspace(shown).unwrap().windows(), [window]);
}

#[test]
fn reserved_edges_keep_tiles_out_but_not_fullscreen_windows() {
    let (mut state, output) = desktop_with_output();
    let window = open(&mut state, "terminal");
    let edges = Insets {
        top: 30,
        left: 50,
        ..Insets::default()
    };

    let transition = apply(&mut state, Command::ReserveOutputEdges { output, edges });
    let tiled = placement_of(&state, window, Gaps::default());
    apply(
        &mut state,
        Command::ChangeWindowMode {
            window,
            change: ModeChange::EnterFullscreen,
        },
    );
    let fullscreen = placement_of(&state, window, Gaps::default());

    assert_eq!(transition.effects, [Effect::OutputConfigured(output)]);
    assert_eq!(tiled, area(50, 30, 1870, 1050));
    assert_eq!(fullscreen, area(0, 0, 1920, 1080));
}

#[test]
fn edges_that_leave_no_room_are_refused_and_the_previous_ones_stay() {
    let (mut state, output) = desktop_with_output();
    let kept = Insets {
        bottom: 40,
        ..Insets::default()
    };
    apply(
        &mut state,
        Command::ReserveOutputEdges {
            output,
            edges: kept,
        },
    );

    let refused = state.apply(Command::ReserveOutputEdges {
        output,
        edges: Insets {
            left: 1000,
            right: 920,
            ..Insets::default()
        },
    });

    assert_eq!(refused, Err(TransitionError::OutputFullyReserved(output)));
    assert_eq!(state.output(output).unwrap().reserved_edges(), kept);
    assert_eq!(
        state.output(output).unwrap().work_area(),
        area(0, 0, 1920, 1040)
    );
}

#[test]
fn output_too_small_for_its_reserved_edges_drops_them() {
    let (mut state, output) = desktop_with_output();
    let edges = Insets {
        top: 600,
        ..Insets::default()
    };
    apply(&mut state, Command::ReserveOutputEdges { output, edges });

    apply(
        &mut state,
        Command::ConfigureOutput {
            output,
            area: area(0, 0, 800, 600),
            scale: Scale::ONE,
        },
    );

    assert_eq!(
        state.output(output).unwrap().reserved_edges(),
        Insets::default()
    );
    assert_eq!(
        state.output(output).unwrap().work_area(),
        area(0, 0, 800, 600)
    );
}

#[test]
fn workspace_shown_elsewhere_is_not_taken_by_the_reconnected_output() {
    let (mut state, primary) = desktop_with_output();
    let secondary = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));
    let moved = state.output(secondary).unwrap().workspace();
    apply(&mut state, Command::RemoveOutput { output: secondary });
    apply(&mut state, Command::SwitchWorkspace { workspace: moved });

    let returned = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));

    assert_eq!(state.output(primary).unwrap().workspace(), moved);
    assert_ne!(state.output(returned).unwrap().workspace(), moved);
}
