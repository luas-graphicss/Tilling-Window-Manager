use std::collections::BTreeSet;

use tilling_config::{
    Action, ConfigError, ConfigSource, HelperCapability, RestartPolicy, ValidatedConfig, load,
    parse,
};

fn valid(text: &str) -> ValidatedConfig {
    parse(text).unwrap_or_else(|error| panic!("configuration was rejected: {error}"))
}

fn diagnostics(text: &str) -> Vec<String> {
    match parse(text) {
        Err(ConfigError::Invalid(diagnostics)) => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.to_string())
            .collect(),
        other => panic!("expected invalid settings, got {other:?}"),
    }
}

#[test]
fn built_in_defaults_name_a_locker_and_bind_the_lock_request() {
    let config = load(&ConfigSource::BuiltIn).unwrap();

    let locker = config.session().locker.as_ref().unwrap();
    let lock_is_bound = config
        .bindings()
        .iter()
        .any(|binding| binding.action == Action::Lock);

    assert_eq!(locker.program(), "swaylock");
    assert!(lock_is_bound);
    assert!(config.session().helpers.is_empty());
}

#[test]
fn file_without_a_session_section_inherits_the_locker() {
    let inherited = valid("schema_version = 1\n");
    let cleared = valid("schema_version = 1\n\n[session]\n");

    assert!(inherited.session().locker.is_some());
    assert_eq!(cleared.session().locker, None);
}

#[test]
fn helpers_keep_their_command_capabilities_and_restart_policy() {
    let config = valid(
        r#"
schema_version = 1

[session]
locker = ["waylock", "--fork-on-lock"]

[[helpers]]
name = "panel"
command = ["waybar", "--log-level", "warning"]

[[helpers]]
name = "input-method"
command = ["fcitx5"]
capabilities = ["input-method", "virtual-keyboard"]
restart = "on-failure"
"#,
    );

    let session = config.session();
    let [panel, input_method] = session.helpers.as_slice() else {
        panic!("expected two helpers: {session:?}");
    };
    let locker = session.locker.as_ref().unwrap();

    assert_eq!(locker.program(), "waylock");
    assert_eq!(locker.arguments(), ["--fork-on-lock"]);
    assert_eq!(panel.name, "panel");
    assert_eq!(panel.command.program(), "waybar");
    assert_eq!(panel.command.arguments(), ["--log-level", "warning"]);
    assert!(panel.capabilities.is_empty());
    assert_eq!(panel.restart, RestartPolicy::Never);
    assert_eq!(
        input_method.capabilities,
        BTreeSet::from([
            HelperCapability::InputMethod,
            HelperCapability::VirtualKeyboard
        ])
    );
    assert_eq!(input_method.restart, RestartPolicy::OnFailure);
}

#[test]
fn invalid_lockers_and_helpers_are_reported_where_they_appear() {
    let problems = diagnostics(
        r#"
schema_version = 1

[session]
locker = []

[[helpers]]
name = "Status Bar"
command = ["waybar"]

[[helpers]]
name = "panel"
command = []

[[helpers]]
name = "panel"
command = ["waybar"]
"#,
    );

    assert_eq!(
        problems,
        [
            "session.locker: must name a program",
            "helpers[0].name: must use lowercase letters, digits, `-` or `_`",
            "helpers[1].command: must name a program",
            "helpers[2].name: `panel` is listed more than once",
        ]
    );
}

#[test]
fn helper_cannot_be_granted_the_session_lock() {
    let outcome = parse(
        r#"
schema_version = 1

[[helpers]]
name = "panel"
command = ["waybar"]
capabilities = ["session-lock"]
"#,
    );

    let Err(ConfigError::Syntax { line, message, .. }) = outcome else {
        panic!("expected a syntax error, got {outcome:?}");
    };
    assert_eq!(line, 7);
    assert!(message.contains("session-lock"), "{message}");
}
