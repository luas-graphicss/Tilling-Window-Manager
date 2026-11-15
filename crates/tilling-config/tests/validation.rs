use tilling_config::{
    Action, ConfigError, ConfigSource, DEFAULT_MODE, MAX_CONFIG_BYTES, RuleMode, RuleOutcome,
    ValidatedConfig, load, parse,
};
use tilling_core::WindowMetadata;
use tilling_core::geometry::Scale;

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

fn syntax_error(text: &str) -> (usize, String) {
    match parse(text) {
        Err(ConfigError::Syntax { line, message, .. }) => (line, message),
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

fn metadata(app_id: Option<&str>, title: Option<&str>) -> WindowMetadata {
    WindowMetadata {
        app_id: app_id.map(str::to_owned),
        title: title.map(str::to_owned),
    }
}

#[test]
fn built_in_defaults_are_valid_and_can_open_a_terminal() {
    let config = load(&ConfigSource::BuiltIn).unwrap();

    let opens_terminal = config
        .bindings()
        .iter()
        .any(|binding| binding.mode == DEFAULT_MODE && binding.action == Action::OpenTerminal);
    assert!(opens_terminal);
    assert_eq!(config.applications().terminal.program(), "foot");
    assert_eq!(config.workspaces().len(), 9);
    assert!(config.rules().is_empty());
    assert!(config.outputs().is_empty());
}

#[test]
fn file_with_only_a_version_inherits_every_default_section() {
    let defaults = load(&ConfigSource::BuiltIn).unwrap();

    let config = valid("schema_version = 1\n");

    assert_eq!(config.bindings(), defaults.bindings());
    assert_eq!(config.layout(), defaults.layout());
    assert_eq!(config.input(), defaults.input());
}

#[test]
fn declared_sections_replace_the_defaults() {
    let config = valid(
        r#"
schema_version = 1
workspaces = ["web", "code"]

[layout]
inner_gap = 0
outer_gap = 4

[[bindings]]
keys = "Super+w"
action = { switch-workspace = "web" }
"#,
    );

    assert_eq!(config.workspaces(), ["web", "code"]);
    assert_eq!(config.layout().gaps.outer, 4);
    assert_eq!(config.bindings().len(), 1);
    assert_eq!(
        config.bindings()[0].action,
        Action::SwitchWorkspace("web".into())
    );
}

#[test]
fn empty_file_reports_the_missing_version() {
    let (line, message) = syntax_error("");

    assert_eq!(line, 1);
    assert!(message.contains("schema_version"), "{message}");
}

#[test]
fn malformed_toml_reports_the_offending_line() {
    let (line, _) = syntax_error("schema_version = 1\n\n[layout\ninner_gap = 1\n");

    assert_eq!(line, 3);
}

#[test]
fn unknown_keys_are_rejected_where_they_appear() {
    let text = "schema_version = 1\n\n[layout]\ninner_gap = 1\nouter_gap = 1\nborder = 2\n";

    let (line, message) = syntax_error(text);

    assert_eq!(line, 6);
    assert!(message.contains("border"), "{message}");
}

#[test]
fn unsupported_versions_take_precedence_over_unknown_keys() {
    let outcome = parse("future_section = true\nschema_version = 2\n");

    assert!(matches!(
        outcome,
        Err(ConfigError::UnsupportedSchemaVersion {
            found: 2,
            supported: 1
        })
    ));
}

#[test]
fn oversized_text_is_rejected_before_parsing() {
    let text = "#".repeat(MAX_CONFIG_BYTES + 1);

    assert!(matches!(
        parse(&text),
        Err(ConfigError::TooLarge {
            limit: MAX_CONFIG_BYTES
        })
    ));
}

#[test]
fn duplicate_bindings_name_both_entries() {
    let problems = diagnostics(
        r#"
schema_version = 1

[[bindings]]
keys = "Super+Shift+q"
action = "close-window"

[[bindings]]
keys = "shift+super+q"
action = "quit"
"#,
    );

    assert_eq!(
        problems,
        ["bindings[1].keys: `Shift+Super+q` is already bound in mode `default` by bindings[0]"]
    );
}

#[test]
fn the_same_keys_can_be_bound_in_different_modes() {
    let config = valid(
        r#"
schema_version = 1

[[bindings]]
keys = "Super+r"
action = { enter-mode = "resize" }

[[bindings]]
mode = "resize"
keys = "Super+r"
action = { enter-mode = "default" }
"#,
    );

    assert_eq!(config.bindings().len(), 2);
}

#[test]
fn every_invalid_binding_is_reported_in_one_pass() {
    let problems = diagnostics(
        r#"
schema_version = 1

[[bindings]]
keys = "Hyper+x"
action = "quit"

[[bindings]]
keys = "Super+1"
action = { switch-workspace = "missing" }

[[bindings]]
keys = "Super+2"
action = { enter-mode = "nowhere" }

[[bindings]]
keys = "Super+3"
action = { spawn = [] }

[[bindings]]
keys = "Super+4"
action = { resize = 0 }
"#,
    );

    assert_eq!(
        problems,
        [
            "bindings[0].keys: `Hyper` is not a modifier, use Ctrl, Alt, Shift or Super",
            "bindings[1].action: workspace `missing` is not listed in `workspaces`",
            "bindings[2].action: mode `nowhere` has no bindings",
            "bindings[3].action: must name a program",
            "bindings[4].action: resize step must be a non-zero value within 4000 basis points",
        ]
    );
}

#[test]
fn spawn_keeps_the_program_and_its_arguments_separate() {
    let config = valid(
        r#"
schema_version = 1

[[bindings]]
keys = "Super+p"
action = { spawn = ["notify-send", "$HOME; echo injected", "second argument"] }
"#,
    );

    let Action::Spawn(command) = &config.bindings()[0].action else {
        panic!("expected a spawn action");
    };
    assert_eq!(command.program(), "notify-send");
    assert_eq!(
        command.arguments(),
        ["$HOME; echo injected", "second argument"]
    );
}

#[test]
fn patterns_beyond_the_limits_are_rejected() {
    let long_pattern = "a".repeat(257);
    let text = format!(
        r#"
schema_version = 1

[[rules]]
app_id = "{long_pattern}"
mode = "floating"

[[rules]]
title = '(?:\w{{50}}){{50}}'
mode = "floating"

[[rules]]
app_id = "("
mode = "floating"
"#
    );

    let problems = diagnostics(&text);

    assert_eq!(problems.len(), 3, "{problems:?}");
    assert_eq!(
        problems[0],
        "rules[0].app_id: pattern must not exceed 256 bytes"
    );
    assert!(problems[1].starts_with("rules[1].title: "), "{problems:?}");
    assert!(problems[1].contains("size limit"), "{problems:?}");
    assert!(problems[2].starts_with("rules[2].app_id: "), "{problems:?}");
}

#[test]
fn rules_need_a_matcher_an_outcome_and_a_known_workspace() {
    let problems = diagnostics(
        r#"
schema_version = 1

[[rules]]
workspace = "2"

[[rules]]
app_id = "^mpv$"

[[rules]]
app_id = "^mpv$"
workspace = "42"
"#,
    );

    assert_eq!(
        problems,
        [
            "rules[0]: must match on `app_id` or `title`",
            "rules[1]: must set `workspace` or `mode`",
            "rules[2].workspace: workspace `42` is not listed in `workspaces`",
        ]
    );
}

#[test]
fn first_matching_rule_wins_and_follows_metadata_changes() {
    let config = valid(
        r#"
schema_version = 1

[[rules]]
app_id = '^org\.mozilla\.firefox$'
workspace = "2"

[[rules]]
title = "(?i)picture-in-picture"
mode = "floating"
"#,
    );
    let rules = config.rules();
    let browser = RuleOutcome {
        workspace: Some("2".into()),
        mode: None,
    };
    let overlay = RuleOutcome {
        workspace: None,
        mode: Some(RuleMode::Floating),
    };

    let both = metadata(Some("org.mozilla.firefox"), Some("Picture-in-Picture"));
    let renamed = metadata(Some("org.mozilla.nightly"), Some("Picture-in-Picture"));
    let untitled = metadata(Some("org.mozilla.nightly"), None);

    assert_eq!(rules.len(), 2);
    assert_eq!(rules.resolve(&both), Some(&browser));
    assert_eq!(rules.resolve(&renamed), Some(&overlay));
    assert_eq!(rules.resolve(&untitled), None);
    assert_eq!(rules.resolve(&WindowMetadata::default()), None);
}

#[test]
fn outputs_accept_fractional_scales_and_positions() {
    let config = valid(
        r#"
schema_version = 1

[[outputs]]
name = "eDP-1"
scale = 1.5
position = { x = -1920, y = 0 }

[[outputs]]
name = "HDMI-A-1"
"#,
    );

    let [laptop, external] = config.outputs() else {
        panic!("expected two outputs");
    };
    assert_eq!(laptop.scale, Some(Scale::from_factor(1.5).unwrap()));
    assert_eq!(laptop.position.map(|position| position.x), Some(-1920));
    assert_eq!(external.scale, None);
}

#[test]
fn invalid_outputs_and_input_settings_are_reported() {
    let problems = diagnostics(
        r#"
schema_version = 1

[input.keyboard]
layout = ""
repeat_rate = 500
repeat_delay_ms = 5

[input.pointer]
natural_scroll = true
acceleration = nan

[[outputs]]
name = "eDP-1"
scale = 0.1

[[outputs]]
name = "eDP-1"
scale = inf
"#,
    );

    assert_eq!(
        problems,
        [
            "input.keyboard.layout: must be an XKB layout name such as `us` or `us,br`",
            "input.keyboard.repeat_rate: must not exceed 100 keys per second",
            "input.keyboard.repeat_delay_ms: must be between 100 and 5000",
            "input.pointer.acceleration: must be a number between -1.0 and 1.0",
            "outputs[0].scale: scale must be a finite factor between 0.25 and 8",
            "outputs[1].name: `eDP-1` is configured more than once",
            "outputs[1].scale: scale must be a finite factor between 0.25 and 8",
        ]
    );
}

#[test]
fn workspace_lists_must_be_unique_and_non_empty() {
    let duplicated = diagnostics("schema_version = 1\nworkspaces = [\"a\", \"a\", \"\"]\n");
    let empty = diagnostics("schema_version = 1\nworkspaces = []\n");

    assert!(duplicated.contains(&"workspaces[1]: `a` is listed more than once".to_owned()));
    assert!(duplicated.contains(&"workspaces[2]: must not be empty".to_owned()));
    assert!(empty.contains(&"workspaces: must list between 1 and 64 names".to_owned()));
}
