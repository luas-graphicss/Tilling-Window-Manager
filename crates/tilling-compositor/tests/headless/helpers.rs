use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use tilling_ipc::{Call, Outcome};

use crate::harness::Harness;

const WAIT_LIMIT: Duration = Duration::from_secs(5);
const WAIT_STEP: Duration = Duration::from_millis(5);
const QUIET_TIME: Duration = Duration::from_millis(300);
const RUNS_OF_A_FAILING_HELPER: usize = 4;

fn session(helpers: &[String]) -> String {
    format!("schema_version = 1\n\n{}", helpers.concat())
}

fn helper(name: &str, script: &str, settings: &str) -> String {
    format!("[[helpers]]\nname = '{name}'\ncommand = ['sh', '-c', '{script}']\n{settings}\n")
}

fn lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn recorded_pid(path: &Path) -> Option<u32> {
    lines(path).first()?.parse().ok()
}

fn is_running(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

fn wait_for<T>(expectation: &str, probe: impl Fn() -> Option<T>) -> T {
    let started = Instant::now();
    loop {
        if let Some(found) = probe() {
            return found;
        }
        assert!(
            started.elapsed() < WAIT_LIMIT,
            "timed out waiting for {expectation}"
        );
        thread::sleep(WAIT_STEP);
    }
}

#[test]
fn a_helper_runs_for_as_long_as_the_session() {
    let script = r#"echo $$ > "$XDG_RUNTIME_DIR/panel"; exec sleep 600"#;
    let harness = Harness::start_from_file(&session(&[helper("panel", script, "")]));
    let record = harness.runtime_dir().join("panel");

    let pid = wait_for("the panel to start", || recorded_pid(&record));
    let supervised = harness.snapshot().helpers;
    let alive_with_the_session = is_running(pid);
    drop(harness);

    assert_eq!(supervised, 1);
    assert!(alive_with_the_session);
    assert!(!is_running(pid));
}

#[test]
fn a_failing_helper_is_started_again_only_a_few_times() {
    let failing = |name: &str| format!(r#"echo run >> "$XDG_RUNTIME_DIR/{name}"; exit 3"#);
    let finished = r#"echo run >> "$XDG_RUNTIME_DIR/finished""#;
    let restarted = r#"restart = "on-failure""#;
    let harness = Harness::start_from_file(&session(&[
        helper("flaky", &failing("flaky"), restarted),
        helper("finished", finished, restarted),
        helper("unattended", &failing("unattended"), ""),
    ]));
    let runs = |name: &str| lines(&harness.runtime_dir().join(name)).len();

    wait_for("the last restart that is allowed", || {
        (runs("flaky") == RUNS_OF_A_FAILING_HELPER).then_some(())
    });
    thread::sleep(QUIET_TIME);

    assert_eq!(runs("flaky"), RUNS_OF_A_FAILING_HELPER);
    assert_eq!(runs("finished"), 1);
    assert_eq!(runs("unattended"), 1);
    assert_eq!(harness.snapshot().helpers, 0);
}

#[test]
fn reloading_keeps_running_helpers_and_stops_the_removed_ones() {
    let record = |name: &str| format!(r#"echo $$ >> "$XDG_RUNTIME_DIR/{name}"; exec sleep 600"#);
    let panel = helper("panel", &record("panel"), "");
    let clock = helper("clock", &record("clock"), "");
    let harness = Harness::start_from_file(&session(std::slice::from_ref(&panel)));
    let mut control = harness.control();
    let started = |name: &str| lines(&harness.runtime_dir().join(name));
    let pid_of = |name: &str| recorded_pid(&harness.runtime_dir().join(name));
    let panel_pid = wait_for("the panel to start", || pid_of("panel"));

    harness.rewrite_config(&session(&[panel, clock.clone()]));
    let grown = control.call(Call::ReloadConfig, None).unwrap();
    let clock_pid = wait_for("the clock to start", || pid_of("clock"));
    let panel_kept = is_running(panel_pid);
    harness.rewrite_config(&session(&[clock]));
    let shrunk = control.call(Call::ReloadConfig, None).unwrap();
    wait_for("the panel to stop", || {
        (!is_running(panel_pid)).then_some(())
    });

    assert!(matches!(grown, Outcome::Success { .. }));
    assert!(matches!(shrunk, Outcome::Success { .. }));
    assert!(panel_kept);
    assert_eq!(started("panel").len(), 1);
    assert_eq!(started("clock").len(), 1);
    assert!(is_running(clock_pid));
    assert_eq!(harness.snapshot().helpers, 1);
}

#[test]
fn only_a_helper_with_capabilities_inherits_a_private_connection() {
    let trusted = r#"test -S "/proc/self/fd/$WAYLAND_SOCKET" && echo private > "$XDG_RUNTIME_DIR/trusted"; exec sleep 600"#;
    let plain = r#"printf "%s\n" "${WAYLAND_SOCKET:-public}" "$WAYLAND_DISPLAY" "$TILLING_SOCKET" "$XDG_CURRENT_DESKTOP" > "$XDG_RUNTIME_DIR/plain"; exec sleep 600"#;
    let harness = Harness::start_from_file(&session(&[
        helper("trusted", trusted, r#"capabilities = ["data-control"]"#),
        helper("plain", plain, ""),
    ]));
    let recorded = |name: &str, count: usize| {
        let path = harness.runtime_dir().join(name);
        wait_for("the helper to describe its environment", || {
            Some(lines(&path)).filter(|found| found.len() == count)
        })
    };

    let trusted_sees = recorded("trusted", 1);
    let plain_sees = recorded("plain", 4);

    let control_socket = harness.session().control_socket().display().to_string();
    assert_eq!(trusted_sees, ["private"]);
    assert_eq!(
        plain_sees,
        ["public", "tilling-test", control_socket.as_str(), "tilling"]
    );
}
