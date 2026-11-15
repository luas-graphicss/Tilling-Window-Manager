use tilling_core::geometry::LogicalRect;
use tilling_core::layout::{Direction, Gaps, LayoutKind, Placement};
use tilling_core::{Command, DesktopState, Effect, WindowId, WindowMode};

use crate::scenario::{
    apply, area, create_workspace, desktop_with_output, open, open_in, placement_of,
};

const NO_GAPS: Gaps = Gaps { inner: 0, outer: 0 };

fn set_layout(state: &mut DesktopState, layout: LayoutKind) {
    let workspace = state.active_workspace();
    apply(state, Command::SetLayout { workspace, layout });
}

fn placed(state: &DesktopState, gaps: Gaps) -> Vec<Placement> {
    state
        .placements(state.active_workspace(), gaps)
        .unwrap_or_else(|| panic!("active workspace does not exist"))
}

fn tile(state: &DesktopState, window: WindowId) -> LogicalRect {
    placement_of(state, window, NO_GAPS)
}

#[test]
fn master_stack_gives_the_first_window_half_of_the_output() {
    let (mut state, _) = desktop_with_output();
    let master = open(&mut state, "editor");
    let upper = open(&mut state, "terminal");
    let lower = open(&mut state, "logs");

    set_layout(&mut state, LayoutKind::MasterStack);

    assert_eq!(tile(&state, master), area(0, 0, 960, 1080));
    assert_eq!(tile(&state, upper), area(960, 0, 960, 540));
    assert_eq!(tile(&state, lower), area(960, 540, 960, 540));
}

#[test]
fn changing_the_layout_keeps_focus_and_restores_the_bsp_arrangement() {
    let (mut state, _) = desktop_with_output();
    let windows = [
        open(&mut state, "editor"),
        open(&mut state, "terminal"),
        open(&mut state, "logs"),
    ];
    apply(
        &mut state,
        Command::ResizeSplit {
            window: windows[0],
            delta: 1_500,
        },
    );
    let arranged = windows.map(|window| tile(&state, window));
    let workspace = state.active_workspace();

    let transition = apply(
        &mut state,
        Command::SetLayout {
            workspace,
            layout: LayoutKind::MasterStack,
        },
    );
    set_layout(&mut state, LayoutKind::Monocle);
    set_layout(&mut state, LayoutKind::Bsp);

    assert_eq!(transition.effects, [Effect::WorkspaceArranged(workspace)]);
    assert_eq!(state.focused_window(), Some(windows[2]));
    assert_eq!(windows.map(|window| tile(&state, window)), arranged);
}

#[test]
fn setting_the_layout_a_workspace_already_uses_changes_nothing() {
    let (mut state, _) = desktop_with_output();
    open(&mut state, "editor");
    let workspace = state.active_workspace();
    let revision = state.revision();

    let transition = apply(
        &mut state,
        Command::SetLayout {
            workspace,
            layout: LayoutKind::Bsp,
        },
    );

    assert!(transition.effects.is_empty());
    assert_eq!(state.revision(), revision);
}

#[test]
fn monocle_places_only_the_focused_tile() {
    let (mut state, _) = desktop_with_output();
    let first = open(&mut state, "editor");
    open(&mut state, "terminal");
    let last = open(&mut state, "logs");
    let gaps = Gaps {
        inner: 8,
        outer: 10,
    };

    set_layout(&mut state, LayoutKind::Monocle);
    assert_eq!(
        placed(&state, gaps),
        [Placement {
            window: last,
            area: area(10, 10, 1900, 1060),
        }]
    );

    apply(
        &mut state,
        Command::FocusDirection {
            direction: Direction::Right,
        },
    );
    assert_eq!(state.focused_window(), Some(first));
    assert_eq!(
        placed(&state, gaps),
        [Placement {
            window: first,
            area: area(10, 10, 1900, 1060),
        }]
    );

    apply(
        &mut state,
        Command::FocusDirection {
            direction: Direction::Left,
        },
    );
    assert_eq!(state.focused_window(), Some(last));
}

#[test]
fn monocle_keeps_the_front_tile_under_a_focused_floating_window() {
    let (mut state, _) = desktop_with_output();
    open(&mut state, "editor");
    let front = open(&mut state, "terminal");
    set_layout(&mut state, LayoutKind::Monocle);
    let floating_area = area(100, 100, 400, 300);

    let floating = open_in(
        &mut state,
        "dialog",
        None,
        WindowMode::Floating(floating_area),
    );

    assert_eq!(state.focused_window(), Some(floating));
    assert_eq!(
        placed(&state, NO_GAPS),
        [
            Placement {
                window: front,
                area: area(0, 0, 1920, 1080),
            },
            Placement {
                window: floating,
                area: floating_area,
            },
        ]
    );
}

#[test]
fn closing_the_front_tile_reveals_the_previously_focused_one() {
    let (mut state, _) = desktop_with_output();
    let first = open(&mut state, "editor");
    let second = open(&mut state, "terminal");
    let third = open(&mut state, "logs");
    set_layout(&mut state, LayoutKind::Monocle);
    apply(&mut state, Command::FocusWindow { window: first });

    apply(&mut state, Command::CloseWindow { window: first });

    assert_eq!(state.focused_window(), Some(third));
    assert_eq!(
        placed(&state, NO_GAPS),
        [Placement {
            window: third,
            area: area(0, 0, 1920, 1080),
        }]
    );
    assert!(state.window(second).is_some());
}

#[test]
fn resizing_and_toggling_adjust_the_master_area() {
    let (mut state, _) = desktop_with_output();
    let master = open(&mut state, "editor");
    let stacked = open(&mut state, "terminal");
    set_layout(&mut state, LayoutKind::MasterStack);

    apply(
        &mut state,
        Command::ResizeSplit {
            window: master,
            delta: 1_000,
        },
    );
    assert_eq!(tile(&state, master), area(0, 0, 1152, 1080));

    apply(
        &mut state,
        Command::ResizeSplit {
            window: stacked,
            delta: 2_000,
        },
    );
    assert_eq!(tile(&state, master), area(0, 0, 768, 1080));

    apply(&mut state, Command::ToggleSplit { window: stacked });
    assert_eq!(tile(&state, master), area(0, 0, 1920, 432));
    assert_eq!(tile(&state, stacked), area(0, 432, 1920, 648));
}

#[test]
fn monocle_has_no_split_to_resize_or_toggle() {
    let (mut state, _) = desktop_with_output();
    open(&mut state, "editor");
    let window = open(&mut state, "terminal");
    set_layout(&mut state, LayoutKind::Monocle);
    let before = state.clone();

    let resized = apply(
        &mut state,
        Command::ResizeSplit {
            window,
            delta: 1_000,
        },
    );
    let toggled = apply(&mut state, Command::ToggleSplit { window });

    assert!(resized.effects.is_empty() && toggled.effects.is_empty());
    assert_eq!(state, before);
}

#[test]
fn moving_a_stacked_window_toward_the_master_swaps_them() {
    let (mut state, _) = desktop_with_output();
    let master = open(&mut state, "editor");
    open(&mut state, "terminal");
    let promoted = open(&mut state, "logs");
    set_layout(&mut state, LayoutKind::MasterStack);

    apply(
        &mut state,
        Command::MoveWindow {
            window: promoted,
            direction: Direction::Left,
        },
    );

    assert_eq!(tile(&state, promoted), area(0, 0, 960, 1080));
    assert_eq!(tile(&state, master), area(960, 540, 960, 540));
    assert_eq!(state.focused_window(), Some(promoted));
}

#[test]
fn focus_moves_between_the_master_and_the_stack() {
    let (mut state, _) = desktop_with_output();
    let master = open(&mut state, "editor");
    let upper = open(&mut state, "terminal");
    let lower = open(&mut state, "logs");
    set_layout(&mut state, LayoutKind::MasterStack);
    let mut focus = |direction| {
        apply(&mut state, Command::FocusDirection { direction });
        state.focused_window()
    };

    assert_eq!(focus(Direction::Up), Some(upper));
    assert_eq!(focus(Direction::Left), Some(master));
    assert_eq!(focus(Direction::Left), Some(master));
    assert_eq!(focus(Direction::Right), Some(upper));
    assert_eq!(focus(Direction::Down), Some(lower));
    assert_eq!(focus(Direction::Down), Some(lower));
}

#[test]
fn new_workspaces_start_with_the_default_layout() {
    let (mut state, _) = desktop_with_output();
    let existing = state.active_workspace();
    let revision = state.revision();

    let transition = apply(
        &mut state,
        Command::SetDefaultLayout {
            layout: LayoutKind::MasterStack,
        },
    );
    let created = create_workspace(&mut state, "2");

    assert!(transition.effects.is_empty());
    assert_eq!(transition.revision, revision + 1);
    let kind = |workspace| state.workspace(workspace).unwrap().layout().kind();
    assert_eq!(kind(existing), LayoutKind::Bsp);
    assert_eq!(kind(created), LayoutKind::MasterStack);
}
