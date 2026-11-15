use tilling_core::layout::{Direction, Gaps};
use tilling_core::{Command, DesktopState, ModeChange, TransitionError, WindowId, WindowMode};

use crate::scenario::{
    add_output, apply, area, create_workspace, desktop_with_output, open, open_in, placement_of,
};

fn stash(state: &mut DesktopState, window: WindowId) {
    let area = area(100, 100, 400, 300);
    apply(state, Command::StashWindow { window, area });
}

fn toggle(state: &mut DesktopState) -> Option<WindowId> {
    apply(state, Command::ToggleScratchpad);
    state.focused_window()
}

#[test]
fn stashed_window_leaves_its_workspace_and_gives_up_focus() {
    let (mut state, _) = desktop_with_output();
    let stays = open(&mut state, "editor");
    let hidden = open(&mut state, "terminal");

    stash(&mut state, hidden);

    let workspace = state.workspace(state.active_workspace()).unwrap();
    assert_eq!(workspace.windows(), [stays]);
    assert_eq!(state.focused_window(), Some(stays));
    assert!(state.is_stashed(hidden) && !state.is_stashed(stays));
    assert_eq!(
        placement_of(&state, stays, Gaps::default()),
        area(0, 0, 1920, 1080)
    );
}

#[test]
fn toggling_shows_the_stashed_window_floating_and_hides_it_again() {
    let (mut state, _) = desktop_with_output();
    let tile = open(&mut state, "editor");
    let dropdown = open(&mut state, "terminal");
    stash(&mut state, dropdown);

    assert_eq!(toggle(&mut state), Some(dropdown));
    assert_eq!(
        state.window(dropdown).unwrap().mode(),
        WindowMode::Floating(area(100, 100, 400, 300))
    );
    assert!(!state.is_stashed(dropdown));

    assert_eq!(toggle(&mut state), Some(tile));
    assert!(state.is_stashed(dropdown));

    assert_eq!(toggle(&mut state), Some(dropdown));
}

#[test]
fn stashed_window_comes_back_on_the_active_workspace() {
    let (mut state, _) = desktop_with_output();
    let dropdown = open(&mut state, "terminal");
    stash(&mut state, dropdown);
    let other = create_workspace(&mut state, "2");
    apply(&mut state, Command::SwitchWorkspace { workspace: other });

    assert_eq!(toggle(&mut state), Some(dropdown));

    assert_eq!(state.window(dropdown).unwrap().workspace(), other);
    assert_eq!(state.workspace(other).unwrap().windows(), [dropdown]);
}

#[test]
fn stashed_windows_take_turns_oldest_first() {
    let (mut state, _) = desktop_with_output();
    let first = open(&mut state, "terminal");
    let second = open(&mut state, "notes");
    stash(&mut state, first);
    stash(&mut state, second);

    let shown = toggle(&mut state);
    let hidden = toggle(&mut state);
    let next = toggle(&mut state);

    assert_eq!((shown, hidden, next), (Some(first), None, Some(second)));
}

#[test]
fn floating_window_keeps_its_own_area_in_the_scratchpad() {
    let (mut state, _) = desktop_with_output();
    let own_area = area(700, 200, 500, 500);
    let floating = open_in(&mut state, "player", None, WindowMode::Floating(own_area));

    stash(&mut state, floating);
    toggle(&mut state);

    assert_eq!(
        state.window(floating).unwrap().mode(),
        WindowMode::Floating(own_area)
    );
}

#[test]
fn summoned_window_is_centered_on_an_output_that_does_not_contain_it() {
    let (mut state, _) = desktop_with_output();
    add_output(&mut state, "secondary", area(1920, 0, 1280, 720));
    let dropdown = open(&mut state, "terminal");
    stash(&mut state, dropdown);
    apply(
        &mut state,
        Command::FocusOutput {
            direction: Direction::Right,
        },
    );

    toggle(&mut state);

    assert_eq!(
        state.window(dropdown).unwrap().mode(),
        WindowMode::Floating(area(2360, 210, 400, 300))
    );
}

#[test]
fn commands_on_a_stashed_window_are_rejected() {
    let (mut state, _) = desktop_with_output();
    open(&mut state, "editor");
    let hidden = open(&mut state, "terminal");
    stash(&mut state, hidden);
    let workspace = state.active_workspace();
    let before = state.clone();

    let commands = [
        Command::FocusWindow { window: hidden },
        Command::ToggleSplit { window: hidden },
        Command::ResizeSplit {
            window: hidden,
            delta: 500,
        },
        Command::MoveWindow {
            window: hidden,
            direction: Direction::Left,
        },
        Command::MoveWindowToWorkspace {
            window: hidden,
            workspace,
        },
        Command::MoveWindowToOutput {
            window: hidden,
            direction: Direction::Left,
        },
        Command::ChangeWindowMode {
            window: hidden,
            change: ModeChange::Tile,
        },
        Command::StashWindow {
            window: hidden,
            area: area(0, 0, 10, 10),
        },
    ];
    for command in commands {
        assert_eq!(
            state.apply(command),
            Err(TransitionError::WindowStashed(hidden))
        );
    }

    assert_eq!(state, before);
}

#[test]
fn closing_a_stashed_window_forgets_it() {
    let (mut state, _) = desktop_with_output();
    let stays = open(&mut state, "editor");
    let hidden = open(&mut state, "terminal");
    stash(&mut state, hidden);

    apply(&mut state, Command::CloseWindow { window: hidden });
    let transition = apply(&mut state, Command::ToggleScratchpad);

    assert!(transition.effects.is_empty());
    assert_eq!(state.focused_window(), Some(stays));
    assert!(state.window(hidden).is_none());
}

#[test]
fn tiling_a_summoned_window_takes_it_out_of_the_scratchpad() {
    let (mut state, _) = desktop_with_output();
    open(&mut state, "editor");
    let dropdown = open(&mut state, "terminal");
    stash(&mut state, dropdown);
    toggle(&mut state);

    apply(
        &mut state,
        Command::ChangeWindowMode {
            window: dropdown,
            change: ModeChange::Tile,
        },
    );
    let transition = apply(&mut state, Command::ToggleScratchpad);

    assert!(transition.effects.is_empty());
    assert_eq!(state.focused_window(), Some(dropdown));
    assert_eq!(state.window(dropdown).unwrap().mode(), WindowMode::Tiled);
}

#[test]
fn fullscreen_window_is_stashed_with_the_area_it_floats_in() {
    let (mut state, _) = desktop_with_output();
    let player = open(&mut state, "player");
    apply(
        &mut state,
        Command::ChangeWindowMode {
            window: player,
            change: ModeChange::EnterFullscreen,
        },
    );

    stash(&mut state, player);
    toggle(&mut state);

    assert_eq!(
        state.window(player).unwrap().mode(),
        WindowMode::Floating(area(100, 100, 400, 300))
    );
}
