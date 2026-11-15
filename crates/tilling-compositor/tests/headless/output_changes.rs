use std::thread;
use std::time::Duration;

use tilling_compositor::{SceneWindow, VirtualOutput};
use tilling_ipc::client::Client;
use tilling_ipc::{
    Call, Outcome, OutputChange, OutputInfo, OutputMode, OutputPosition, OutputTransform, Reply,
    ResourceId,
};

use crate::harness::Harness;

const PRIMARY: &str = "HEADLESS-1";
const SPARE: &str = "HEADLESS-2";
const BRIEF: Duration = Duration::from_millis(500);
const NEVER_REACHED: Duration = Duration::from_secs(600);
const SCALED_WIDTH: u32 = 640;

fn harness(confirmation: Duration) -> Harness {
    Harness::start_with(|options| options.output_confirmation = confirmation)
}

fn outputs(control: &mut Client) -> Vec<OutputInfo> {
    match control.call(Call::GetOutputs, None) {
        Ok(Outcome::Success {
            result: Reply::Outputs(outputs),
        }) => outputs,
        other => panic!("unexpected reply to get outputs: {other:?}"),
    }
}

fn output_named(control: &mut Client, name: &str) -> OutputInfo {
    outputs(control)
        .into_iter()
        .find(|output| output.name == name)
        .unwrap_or_else(|| panic!("output {name} is not connected"))
}

fn send(control: &mut Client, call: Call) -> Outcome {
    control
        .call(call, None)
        .unwrap_or_else(|error| panic!("control request was not answered: {error}"))
}

fn change(control: &mut Client, output: ResourceId, change: OutputChange) -> Outcome {
    send(control, Call::ConfigureOutput { output, change })
}

fn scaled(factor: f64) -> OutputChange {
    OutputChange {
        scale: Some(factor),
        ..OutputChange::default()
    }
}

fn with_mode(width: u32, height: u32) -> OutputChange {
    OutputChange {
        mode: Some(OutputMode {
            width,
            height,
            refresh_millihertz: None,
        }),
        ..OutputChange::default()
    }
}

fn is_pending(outcome: &Outcome) -> bool {
    matches!(
        outcome,
        Outcome::Success {
            result: Reply::OutputChangePending { .. }
        }
    )
}

fn is_refused(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::ValidationError { .. })
}

#[test]
fn unconfirmed_change_is_reverted_when_the_deadline_passes() {
    let harness = harness(BRIEF);
    let mut control = harness.control();
    let before = output_named(&mut control, PRIMARY);

    let pending = change(&mut control, before.id, scaled(2.0));
    let during = output_named(&mut control, PRIMARY);
    thread::sleep(BRIEF * 2);
    let after = output_named(&mut control, PRIMARY);

    assert!(matches!(
        pending,
        Outcome::Success {
            result: Reply::OutputChangePending {
                confirm_within_ms: 500,
                ..
            }
        }
    ));
    assert_eq!(
        during,
        OutputInfo {
            width: SCALED_WIDTH,
            height: 360,
            scale: 2.0,
            ..before.clone()
        }
    );
    assert_eq!(after, before);
}

#[test]
fn confirmed_change_outlives_the_deadline() {
    let harness = harness(BRIEF);
    let mut control = harness.control();
    let before = output_named(&mut control, PRIMARY);
    let turned = OutputChange {
        position: Some(OutputPosition { x: -720, y: 40 }),
        transform: Some(OutputTransform::Rotated90),
        ..OutputChange::default()
    };

    change(&mut control, before.id, turned);
    let confirmed = send(&mut control, Call::ConfirmOutputChange);
    thread::sleep(BRIEF * 2);
    let after = output_named(&mut control, PRIMARY);

    assert!(matches!(
        confirmed,
        Outcome::Success {
            result: Reply::Accepted { .. }
        }
    ));
    assert_eq!(
        after,
        OutputInfo {
            x: -720,
            y: 40,
            width: 720,
            height: 1280,
            ..before
        }
    );
}

#[test]
fn revert_restores_the_previous_settings_at_once() {
    let harness = harness(NEVER_REACHED);
    let mut control = harness.control();
    let before = output_named(&mut control, PRIMARY);

    change(&mut control, before.id, scaled(1.5));
    let reverted = send(&mut control, Call::RevertOutputChange);
    let after = output_named(&mut control, PRIMARY);
    let nothing_left = send(&mut control, Call::RevertOutputChange);

    assert!(matches!(reverted, Outcome::Success { .. }));
    assert_eq!(after, before);
    assert!(is_refused(&nothing_left));
}

#[test]
fn second_change_waits_until_the_first_is_settled() {
    let harness = harness(NEVER_REACHED);
    let mut control = harness.control();
    let output = output_named(&mut control, PRIMARY).id;

    let first = change(&mut control, output, with_mode(1280, 720));
    let while_pending = change(&mut control, output, scaled(1.5));
    send(&mut control, Call::ConfirmOutputChange);
    let after_confirmation = change(&mut control, output, scaled(1.5));

    assert!(is_pending(&first));
    assert!(is_refused(&while_pending));
    assert!(is_pending(&after_confirmation));
}

#[test]
fn unusable_changes_are_refused_and_leave_nothing_pending() {
    let harness = harness(NEVER_REACHED);
    let mut control = harness.control();
    let before = output_named(&mut control, PRIMARY);
    let stranger = ResourceId {
        index: 99,
        generation: 0,
    };

    let unknown_mode = change(&mut control, before.id, with_mode(3840, 2160));
    let tiny_scale = change(&mut control, before.id, scaled(0.1));
    let nothing = change(&mut control, before.id, OutputChange::default());
    let unknown_output = change(&mut control, stranger, scaled(2.0));
    let confirmation = send(&mut control, Call::ConfirmOutputChange);

    assert!(is_refused(&unknown_mode));
    assert!(is_refused(&tiny_scale));
    assert!(is_refused(&nothing));
    assert!(matches!(unknown_output, Outcome::NotFound { .. }));
    assert!(is_refused(&confirmation));
    assert_eq!(output_named(&mut control, PRIMARY), before);
}

#[test]
fn windows_fill_the_changed_area_and_return_with_the_revert() {
    let harness = harness(NEVER_REACHED);
    let mut client = harness.connect();
    let mut control = harness.control();
    let window = client.create_toplevel("org.example.Editor", "notes");
    client.map(window).unwrap();
    client.settle().unwrap();
    let before = harness.published(1);
    let output = output_named(&mut control, PRIMARY).id;

    change(&mut control, output, scaled(2.0));
    client.settle().unwrap();
    let scaled_down = harness.wait_until("the window fits the scaled output", |snapshot| {
        let fits = |window: &SceneWindow| window.frame.width() <= SCALED_WIDTH;
        !snapshot.transaction_pending && snapshot.scene.windows().first().is_some_and(fits)
    });
    send(&mut control, Call::RevertOutputChange);
    client.settle().unwrap();
    let restored = harness.wait_until("the window regains its frame", |snapshot| {
        !snapshot.transaction_pending && snapshot.scene.windows() == before.scene.windows()
    });

    assert_eq!(scaled_down.focused, before.focused);
    assert_eq!(restored.focused, before.focused);
    assert_eq!(restored.mapped_windows, 1);
}

#[test]
fn unplugging_the_output_ends_its_trial() {
    let harness = harness(BRIEF);
    let mut control = harness.control();
    harness.plug(VirtualOutput {
        name: SPARE.to_owned(),
        width: 1920,
        height: 1080,
    });
    let spare = output_named(&mut control, SPARE).id;

    let pending = change(&mut control, spare, scaled(2.0));
    harness.unplug(SPARE);
    let confirmation = send(&mut control, Call::ConfirmOutputChange);
    thread::sleep(BRIEF * 2);

    assert!(is_pending(&pending));
    assert!(is_refused(&confirmation));
    assert_eq!(outputs(&mut control).len(), 1);
}
