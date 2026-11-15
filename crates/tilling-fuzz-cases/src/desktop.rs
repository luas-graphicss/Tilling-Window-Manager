mod script;

use tilling_core::layout::Gaps;
use tilling_core::{Command, DesktopState, Transition, TransitionError};

use self::script::Script;

const INITIAL_WORKSPACE: &str = "1";
const PADDED_GAPS: Gaps = Gaps {
    inner: 6,
    outer: 10,
};

pub fn check(input: &[u8]) {
    run(input);
}

fn run(input: &[u8]) -> DesktopState {
    let mut script = Script::new(input);
    let mut state = DesktopState::new(INITIAL_WORKSPACE);
    let mut closed = Vec::new();
    while let Some(command) = script.next_command(&state, &closed) {
        let before = state.clone();
        let outcome = state.apply(command.clone());
        if let Command::CloseWindow { window } = &command {
            closed.push(*window);
        }
        check_step(&before, &state, &outcome, &command);
    }
    state
}

fn check_step(
    before: &DesktopState,
    state: &DesktopState,
    outcome: &Result<Transition, TransitionError>,
    command: &Command,
) {
    let verified = state.verify();
    assert!(
        verified.is_ok(),
        "{command:?} broke an invariant: {verified:?}"
    );
    let changed = state != before;
    match outcome {
        Ok(transition) if changed => assert_eq!(
            Some(transition.revision),
            before.revision().checked_add(1),
            "{command:?} changed the state without advancing the revision by one"
        ),
        Ok(transition) => assert!(
            transition.effects.is_empty() && transition.revision == before.revision(),
            "{command:?} left the state alone and still reported a change"
        ),
        Err(_) => assert!(
            !changed,
            "{command:?} was rejected and still changed the state"
        ),
    }
    for (workspace, _) in state.workspaces() {
        for gaps in [Gaps::default(), PADDED_GAPS] {
            assert!(
                state.placements(workspace, gaps).is_some(),
                "{command:?} left a workspace that cannot be placed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::script::tests::{ADD_OUTPUT, OPEN_TILED_WINDOW};
    use super::*;

    #[test]
    fn scripted_commands_reach_the_state() {
        let input = [ADD_OUTPUT, OPEN_TILED_WINDOW, OPEN_TILED_WINDOW].concat();

        let state = run(&input);

        assert_eq!(state.outputs().count(), 1);
        assert_eq!(state.windows().count(), 2);
        assert_eq!(state.revision(), 3);
    }

    #[test]
    fn input_that_ends_inside_a_command_is_ignored_from_there() {
        let mut input = [ADD_OUTPUT, OPEN_TILED_WINDOW].concat();
        input.extend(ADD_OUTPUT.iter().take(5));

        let state = run(&input);

        assert_eq!(state.outputs().count(), 1);
        assert_eq!(state.windows().count(), 1);
    }

    #[test]
    fn empty_input_leaves_the_initial_desktop() {
        assert_eq!(run(&[]), DesktopState::new(INITIAL_WORKSPACE));
    }
}
