use tilling_core::layout::Direction;
use tilling_core::{Command, DesktopState, OutputId, WindowMode, WorkspaceId};

use crate::scenario::{add_output, apply, area, desktop_with_output, open, open_in};

fn side_by_side() -> (DesktopState, OutputId, OutputId) {
    let (mut state, left) = desktop_with_output();
    let right = add_output(&mut state, "secondary", area(1920, 0, 1920, 1080));
    (state, left, right)
}

fn shown_on(state: &DesktopState, output: OutputId) -> WorkspaceId {
    state
        .output(output)
        .unwrap_or_else(|| panic!("output {output} does not exist"))
        .workspace()
}

#[test]
fn focus_moves_to_the_workspace_of_the_adjacent_output() {
    let (mut state, left, right) = side_by_side();
    let mut focus = |direction| {
        apply(&mut state, Command::FocusOutput { direction });
        state.active_workspace()
    };

    let moved = focus(Direction::Right);
    let at_the_edge = focus(Direction::Right);
    let nothing_above = focus(Direction::Up);
    let back = focus(Direction::Left);

    assert_eq!(moved, shown_on(&state, right));
    assert_eq!(at_the_edge, moved);
    assert_eq!(nothing_above, moved);
    assert_eq!(back, shown_on(&state, left));
}

#[test]
fn focusing_an_output_focuses_the_window_it_shows() {
    let (mut state, _, right) = side_by_side();
    let near = open(&mut state, "editor");
    let far_workspace = shown_on(&state, right);
    let far = open_in(
        &mut state,
        "browser",
        Some(far_workspace),
        WindowMode::Tiled,
    );

    apply(
        &mut state,
        Command::FocusOutput {
            direction: Direction::Right,
        },
    );

    assert_ne!(near, far);
    assert_eq!(state.focused_window(), Some(far));
}

#[test]
fn window_moves_to_the_adjacent_output_while_focus_stays() {
    let (mut state, left, right) = side_by_side();
    let stays = open(&mut state, "editor");
    let moves = open(&mut state, "browser");

    apply(
        &mut state,
        Command::MoveWindowToOutput {
            window: moves,
            direction: Direction::Right,
        },
    );
    let revision = state.revision();
    apply(
        &mut state,
        Command::MoveWindowToOutput {
            window: moves,
            direction: Direction::Right,
        },
    );

    assert_eq!(
        state.window(moves).unwrap().workspace(),
        shown_on(&state, right)
    );
    assert_eq!(state.active_workspace(), shown_on(&state, left));
    assert_eq!(state.focused_window(), Some(stays));
    assert_eq!(state.revision(), revision);
}

#[test]
fn floating_window_is_centered_on_the_output_it_moves_to() {
    let (mut state, _, _) = side_by_side();
    let floating = open_in(
        &mut state,
        "player",
        None,
        WindowMode::Floating(area(100, 100, 400, 300)),
    );

    apply(
        &mut state,
        Command::MoveWindowToOutput {
            window: floating,
            direction: Direction::Right,
        },
    );

    assert_eq!(
        state.window(floating).unwrap().mode(),
        WindowMode::Floating(area(2680, 390, 400, 300))
    );
}

#[test]
fn without_an_adjacent_output_nothing_changes() {
    let (mut state, _) = desktop_with_output();
    let window = open(&mut state, "editor");
    let before = state.clone();

    apply(
        &mut state,
        Command::FocusOutput {
            direction: Direction::Left,
        },
    );
    apply(
        &mut state,
        Command::MoveWindowToOutput {
            window,
            direction: Direction::Down,
        },
    );

    assert_eq!(state, before);
}
