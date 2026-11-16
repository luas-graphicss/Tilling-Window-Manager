use tilling_ipc::client::Client;
use tilling_ipc::{Call, Outcome, Reply};

use crate::harness::Harness;
use crate::keys;

const ACTIVE: &str = r#"schema_version = 1
workspaces = ["one", "two"]

[[bindings]]
keys = "Super+a"
action = { enter-mode = "alpha" }

[[bindings]]
mode = "alpha"
keys = "Escape"
action = { enter-mode = "default" }
"#;

const REPLACEMENT: &str = r#"schema_version = 1
workspaces = ["one", "two", "three"]

[[bindings]]
keys = "Super+t"
action = { enter-mode = "beta" }

[[bindings]]
mode = "beta"
keys = "Escape"
action = { enter-mode = "default" }
"#;

const BROKEN_BINDING: &str = r#"
[[bindings]]
keys = "Super+q"
action = { switch-workspace = "nine" }
"#;

const UNFINISHED_TABLE: &str =
    "schema_version = 1\nworkspaces = [\"one\", \"two\", \"three\"]\n\n[layout\n";

fn request(control: &mut Client, call: Call) -> Reply {
    match control.call(call, None) {
        Ok(Outcome::Success { result }) => result,
        other => panic!("control request was not answered: {other:?}"),
    }
}

fn config_revision(control: &mut Client) -> u64 {
    match request(control, Call::GetConfig) {
        Reply::Config(config) => config.revision,
        other => panic!("unexpected reply to get config: {other:?}"),
    }
}

fn workspace_names(control: &mut Client) -> Vec<String> {
    match request(control, Call::GetWorkspaces) {
        Reply::Workspaces(workspaces) => workspaces
            .into_iter()
            .map(|workspace| workspace.name)
            .collect(),
        other => panic!("unexpected reply to get workspaces: {other:?}"),
    }
}

fn refusal(outcome: Outcome) -> String {
    match outcome {
        Outcome::ValidationError { message } => message,
        other => panic!("expected the file to be refused, got {other:?}"),
    }
}

#[test]
fn a_valid_file_replaces_bindings_and_workspaces_in_one_revision() {
    let harness = Harness::start_from_file(ACTIVE);
    let mut control = harness.control();
    let revision = config_revision(&mut control);

    harness.rewrite_config(REPLACEMENT);
    let outcome = control.call(Call::ReloadConfig, None).unwrap();
    let retired_binding = harness.chord(&[keys::SUPER, keys::A]).keyboard_mode;
    let new_binding = harness.chord(&[keys::SUPER, keys::T]).keyboard_mode;

    assert!(matches!(outcome, Outcome::Success { .. }));
    assert_eq!(config_revision(&mut control), revision + 1);
    assert_eq!(workspace_names(&mut control), ["one", "two", "three"]);
    assert_eq!(retired_binding, "default");
    assert_eq!(new_binding, "beta");
}

#[test]
fn a_file_with_one_invalid_setting_is_refused_whole() {
    let harness = Harness::start_from_file(ACTIVE);
    let mut control = harness.control();
    let before = (config_revision(&mut control), workspace_names(&mut control));

    harness.rewrite_config(&format!("{REPLACEMENT}{BROKEN_BINDING}"));
    let outcome = control.call(Call::ReloadConfig, None).unwrap();
    let new_binding = harness.chord(&[keys::SUPER, keys::T]).keyboard_mode;
    let active_binding = harness.chord(&[keys::SUPER, keys::A]).keyboard_mode;

    let message = refusal(outcome);
    assert!(message.contains("bindings[2].action"), "{message}");
    assert!(message.contains("`nine`"), "{message}");
    assert_eq!(
        (config_revision(&mut control), workspace_names(&mut control)),
        before
    );
    assert_eq!(new_binding, "default");
    assert_eq!(active_binding, "alpha");
}

#[test]
fn a_file_that_does_not_parse_names_its_line_and_changes_nothing() {
    let harness = Harness::start_from_file(ACTIVE);
    let mut control = harness.control();
    let before = (config_revision(&mut control), workspace_names(&mut control));

    harness.rewrite_config(UNFINISHED_TABLE);
    let outcome = control.call(Call::ReloadConfig, None).unwrap();
    let active_binding = harness.chord(&[keys::SUPER, keys::A]).keyboard_mode;

    let message = refusal(outcome);
    assert!(message.contains("line 4"), "{message}");
    assert_eq!(
        (config_revision(&mut control), workspace_names(&mut control)),
        before
    );
    assert_eq!(active_binding, "alpha");
}

#[test]
fn a_mode_removed_by_the_new_file_returns_the_keyboard_to_the_default_mode() {
    let harness = Harness::start_from_file(ACTIVE);
    let mut control = harness.control();
    let entered = harness.chord(&[keys::SUPER, keys::A]).keyboard_mode;

    harness.rewrite_config(REPLACEMENT);
    let outcome = control.call(Call::ReloadConfig, None).unwrap();

    assert_eq!(entered, "alpha");
    assert!(matches!(outcome, Outcome::Success { .. }));
    assert_eq!(harness.snapshot().keyboard_mode, "default");
}
