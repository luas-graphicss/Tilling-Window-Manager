use tilling_core::geometry::{LogicalRect, Scale};
use tilling_core::layout::{Direction, Gaps, LayoutKind};
use tilling_core::{
    Command, DesktopState, ModeChange, OutputId, RestoreMode, Transition, TransitionError,
    WindowId, WindowMetadata, WindowMode, WorkspaceId,
};

use crate::random::Random;
use crate::scenario::area;

const SEEDS: [u64; 4] = [0x0001, 0x5EED, 0x00C0_FFEE, 0x7111_1E57];
const STEPS_PER_SEED: usize = 3_000;
const REPLAY_STEPS: usize = 2_000;
const WINDOW_BUDGET: usize = 48;
const WORKSPACE_BUDGET: usize = 8;
const OUTPUT_BUDGET: usize = 3;
const SCALE_FACTORS: [f64; 4] = [1.0, 1.25, 1.5, 2.0];
const DIRECTIONS: [Direction; 4] = [
    Direction::Left,
    Direction::Right,
    Direction::Up,
    Direction::Down,
];
const LAYOUTS: [LayoutKind; 3] = [
    LayoutKind::Bsp,
    LayoutKind::MasterStack,
    LayoutKind::Monocle,
];

type Outcome = Result<Transition, TransitionError>;

#[test]
fn generated_sequences_preserve_every_invariant() {
    for seed in SEEDS {
        run(seed, STEPS_PER_SEED);
    }
}

#[test]
fn replaying_a_sequence_reproduces_the_serialized_state() {
    let (first_state, first_outcomes) = run(SEEDS[0], REPLAY_STEPS);
    let (second_state, second_outcomes) = run(SEEDS[0], REPLAY_STEPS);

    assert_eq!(first_outcomes, second_outcomes);
    assert_eq!(
        serde_json::to_string(&first_state).unwrap(),
        serde_json::to_string(&second_state).unwrap()
    );
}

fn run(seed: u64, steps: usize) -> (DesktopState, Vec<Outcome>) {
    let mut random = Random::new(seed);
    let mut state = DesktopState::new("1");
    let mut retired = Vec::new();
    let mut outcomes = Vec::with_capacity(steps);
    for step in 0..steps {
        let command = generate(&mut random, &state, &retired);
        let before = state.clone();
        let outcome = state.apply(command.clone());
        if let Command::CloseWindow { window } = command {
            retired.push(window);
        }
        if let Err(failure) = check(&before, &state, &outcome) {
            panic!("seed {seed:#x}, step {step}, {command:?}: {failure}");
        }
        outcomes.push(outcome);
    }
    (state, outcomes)
}

fn check(before: &DesktopState, state: &DesktopState, outcome: &Outcome) -> Result<(), String> {
    state.verify().map_err(|violation| violation.to_string())?;
    let changed = state != before;
    match outcome {
        Ok(transition) if !changed && !transition.effects.is_empty() => {
            return Err("unchanged state reported effects".to_owned());
        }
        Ok(transition) if !changed && transition.revision != before.revision() => {
            return Err("unchanged state advanced the revision".to_owned());
        }
        Ok(transition) if changed && transition.revision != before.revision() + 1 => {
            return Err("revision did not advance by one".to_owned());
        }
        Err(_) if changed => {
            return Err("rejected command changed the state".to_owned());
        }
        Ok(_) | Err(_) => {}
    }
    for (workspace, _) in state.workspaces() {
        check_placements(state, workspace, Gaps::default())?;
        check_placements(
            state,
            workspace,
            Gaps {
                inner: 6,
                outer: 10,
            },
        )?;
    }
    Ok(())
}

fn check_placements(
    state: &DesktopState,
    workspace_id: WorkspaceId,
    gaps: Gaps,
) -> Result<(), String> {
    let workspace = state
        .workspace(workspace_id)
        .ok_or("workspace disappeared")?;
    let placements = state
        .placements(workspace_id, gaps)
        .ok_or("live workspace has no placements")?;
    let Some(bounds) = workspace
        .output()
        .and_then(|output| state.output(output))
        .map(|output| output.area())
    else {
        return if placements.is_empty() {
            Ok(())
        } else {
            Err("parked workspace placed windows".to_owned())
        };
    };
    let mut placed: Vec<WindowId> = placements
        .iter()
        .map(|placement| placement.window)
        .collect();
    placed.sort_unstable();
    placed.dedup();
    let tiled = workspace.layout().windows().len();
    let shown_tiles = match workspace.layout().kind() {
        LayoutKind::Bsp | LayoutKind::MasterStack => tiled,
        LayoutKind::Monocle => tiled.min(1),
    };
    let expected = workspace.windows().len() - tiled + shown_tiles;
    if placed.len() != expected || placed.len() != placements.len() {
        return Err("windows are not placed exactly once".to_owned());
    }
    let tiles: Vec<LogicalRect> = placements
        .iter()
        .filter(|placement| is_tiled(state, placement.window))
        .map(|placement| placement.area)
        .collect();
    for (index, tile) in tiles.iter().enumerate() {
        if !bounds.contains(tile) {
            return Err(format!("tile {tile:?} leaves output {bounds:?}"));
        }
        if tiles
            .iter()
            .skip(index + 1)
            .any(|other| tile.overlaps(other))
        {
            return Err(format!("tile {tile:?} overlaps another tile"));
        }
    }
    let covered: u64 = tiles.iter().map(LogicalRect::area).sum();
    let every_leaf_is_visible = workspace.layout().windows().len() == tiles.len();
    if gaps == Gaps::default()
        && every_leaf_is_visible
        && !tiles.is_empty()
        && covered != bounds.area()
    {
        return Err(format!("tiles cover {covered} of {} units", bounds.area()));
    }
    Ok(())
}

fn is_tiled(state: &DesktopState, window: WindowId) -> bool {
    state
        .window(window)
        .is_some_and(|entry| entry.mode() == WindowMode::Tiled)
}

fn generate(random: &mut Random, state: &DesktopState, retired: &[WindowId]) -> Command {
    let windows: Vec<WindowId> = state.windows().map(|(id, _)| id).collect();
    let workspaces: Vec<WorkspaceId> = state.workspaces().map(|(id, _)| id).collect();
    let outputs: Vec<OutputId> = state.outputs().map(|(id, _)| id).collect();
    let stale = (random.below(20) == 0)
        .then(|| random.pick(retired))
        .flatten();
    let window = stale.or_else(|| random.pick(&windows));
    let workspace = random.pick(&workspaces);
    let direction = random.pick(&DIRECTIONS).unwrap_or(Direction::Left);
    let layout = random.pick(&LAYOUTS).unwrap_or_default();
    match (random.below(20), window, workspace) {
        (0..=4, _, _) if windows.len() < WINDOW_BUDGET => open_window(random, &windows, workspace),
        (0..=6, Some(window), _) => Command::CloseWindow { window },
        (7, Some(window), _) => Command::FocusWindow { window },
        (8, _, _) => Command::FocusDirection { direction },
        (9, Some(window), _) => Command::MoveWindow { window, direction },
        (10, Some(window), Some(workspace)) => Command::MoveWindowToWorkspace { window, workspace },
        (11, Some(window), _) => Command::ChangeWindowMode {
            window,
            change: mode_change(random),
        },
        (12, Some(window), _) => Command::ToggleSplit { window },
        (13, Some(window), _) => Command::ResizeSplit {
            window,
            delta: resize_delta(random),
        },
        (14, _, Some(workspace)) => Command::SwitchWorkspace { workspace },
        (15, _, _) if workspaces.len() < WORKSPACE_BUDGET => Command::CreateWorkspace {
            name: format!("generated-{}", random.next_u64()),
        },
        (16, _, _) => output_command(random, &outputs),
        (17, Some(window), _) => Command::UpdateWindowMetadata {
            window,
            metadata: metadata(random),
        },
        (18, _, Some(workspace)) => Command::SetLayout { workspace, layout },
        (19, _, _) => Command::SetDefaultLayout { layout },
        _ => open_window(random, &windows, workspace),
    }
}

fn open_window(
    random: &mut Random,
    windows: &[WindowId],
    workspace: Option<WorkspaceId>,
) -> Command {
    let mode = match random.below(6) {
        0 => WindowMode::Floating(random_area(random)),
        1 => WindowMode::Fullscreen(RestoreMode::Tiled),
        _ => WindowMode::Tiled,
    };
    Command::OpenWindow {
        metadata: metadata(random),
        workspace: workspace.filter(|_| random.below(3) == 0),
        transient_for: random.pick(windows).filter(|_| random.below(5) == 0),
        mode,
    }
}

fn output_command(random: &mut Random, outputs: &[OutputId]) -> Command {
    let area = random_area(random);
    let scale = random
        .pick(&SCALE_FACTORS)
        .and_then(|factor| Scale::from_factor(factor).ok())
        .unwrap_or(Scale::ONE);
    match random.pick(outputs) {
        Some(output) if random.below(3) == 0 => Command::RemoveOutput { output },
        Some(output) if outputs.len() >= OUTPUT_BUDGET || random.below(2) == 0 => {
            Command::ConfigureOutput {
                output,
                area,
                scale,
            }
        }
        _ => Command::AddOutput {
            name: format!("generated-{}", random.next_u64()),
            area,
            scale,
        },
    }
}

fn mode_change(random: &mut Random) -> ModeChange {
    match random.below(4) {
        0 => ModeChange::Tile,
        1 => ModeChange::Float(random_area(random)),
        2 => ModeChange::EnterFullscreen,
        _ => ModeChange::LeaveFullscreen,
    }
}

fn resize_delta(random: &mut Random) -> i16 {
    i16::try_from(random.below(4_001)).unwrap_or(0) - 2_000
}

fn random_area(random: &mut Random) -> LogicalRect {
    let x = i32::try_from(random.below(4_000)).unwrap_or(0) - 2_000;
    let y = i32::try_from(random.below(4_000)).unwrap_or(0) - 2_000;
    let width = u32::try_from(random.below(3_840)).unwrap_or(0) + 1;
    let height = u32::try_from(random.below(2_160)).unwrap_or(0) + 1;
    area(x, y, width, height)
}

fn metadata(random: &mut Random) -> WindowMetadata {
    WindowMetadata {
        app_id: Some(format!("app-{}", random.below(5))),
        title: (random.below(3) != 0).then(|| format!("title {}", random.next_u64())),
    }
}
