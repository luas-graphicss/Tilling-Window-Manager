use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process};

use crate::support::Instance;

const WAIT_LIMIT: Duration = Duration::from_secs(10);
const WAIT_STEP: Duration = Duration::from_millis(10);
const RECORD: &str = "probe";
const SESSION_WITH_A_HELPER: &str = r#"schema_version = 1

[[helpers]]
name = "probe"
command = ['sh', '-c', 'echo $$ > "$XDG_RUNTIME_DIR/probe"; exec sleep 600']
"#;

fn recorded_pid(record: &Path) -> Option<Pid> {
    let text = fs::read_to_string(record).ok()?;
    Pid::from_raw(text.trim().parse().ok()?)
}

fn is_running(pid: Pid) -> bool {
    Path::new("/proc")
        .join(pid.as_raw_nonzero().to_string())
        .exists()
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
fn termination_signal_ends_a_program_started_by_the_session() {
    let instance = Instance::start_headless_with_config(SESSION_WITH_A_HELPER);
    let record = instance.runtime_dir().join(RECORD);
    let helper = wait_for("the helper to start", || recorded_pid(&record));

    kill_process(helper, Signal::TERM).unwrap();
    let ended = wait_for("the helper to end", || {
        (!is_running(helper)).then_some(true)
    });

    assert!(ended);
}
