use std::env;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use tilling_ipc::{Call, Outcome};
use tilling_test_support::Client;

use crate::harness::Harness;

const STAND_IN: &str = "locker::stand_in_locker";
const ROLE_MARKER: &str = "stand-in-locker";
const UNLOCK_SIGNAL: &str = "unlock";
const CRASH_SIGNAL: &str = "crash";
const LOCKER_LOG: &str = "locker.log";
const LAUNCH_NOTICE: &str = "running 1 test";
const STEP: Duration = Duration::from_millis(5);
const WAIT_LIMIT: Duration = Duration::from_secs(5);
const SHORT_CONFIRMATION: Duration = Duration::from_millis(150);

fn session_with_stand_in_locker() -> Harness {
    let program =
        env::current_exe().unwrap_or_else(|error| panic!("test program was not located: {error}"));
    let config = format!(
        "schema_version = 1\n\n[session]\nlocker = ['sh', '-c', 'exec \"$@\" >> \"$XDG_RUNTIME_DIR/{LOCKER_LOG}\" 2>&1', 'locker', '{}', '--exact', '{STAND_IN}', '--nocapture']\n",
        program.display()
    );
    let harness = Harness::start_from_file(&config);
    signal(&harness, ROLE_MARKER);
    harness
}

fn stand_in_directory() -> Option<PathBuf> {
    env::var_os("WAYLAND_SOCKET")?;
    let directory = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    directory.join(ROLE_MARKER).exists().then_some(directory)
}

fn launches(harness: &Harness) -> usize {
    fs::read_to_string(harness.runtime_dir().join(LOCKER_LOG))
        .map(|log| log.matches(LAUNCH_NOTICE).count())
        .unwrap_or_default()
}

fn signal(harness: &Harness, name: &str) {
    fs::write(harness.runtime_dir().join(name), b"")
        .unwrap_or_else(|error| panic!("signal `{name}` was not written: {error}"));
}

fn wait_until_consumed(harness: &Harness, name: &str) {
    let started = Instant::now();
    while harness.runtime_dir().join(name).exists() {
        assert!(
            started.elapsed() < WAIT_LIMIT,
            "timed out waiting for the locker to notice `{name}`"
        );
        thread::sleep(STEP);
    }
}

#[test]
fn stand_in_locker() {
    let Some(directory) = stand_in_directory() else {
        return;
    };
    let mut client = Client::connect_to_environment().unwrap();
    client.lock_session().unwrap();
    client.cover_output().unwrap();
    client
        .wait_until("the lock is confirmed", Client::lock_confirmed)
        .unwrap();
    loop {
        if fs::remove_file(directory.join(CRASH_SIGNAL)).is_ok() {
            std::process::exit(1);
        }
        if directory.join(UNLOCK_SIGNAL).exists() {
            client.unlock_session().unwrap();
            client.roundtrip().unwrap();
            return;
        }
        client.roundtrip().unwrap();
        thread::sleep(STEP);
    }
}

fn session_with_locker(command: &str) -> String {
    format!("schema_version = 1\n\n[session]\nlocker = {command}\n")
}

fn refused_as_invalid(outcome: Outcome) -> String {
    let Outcome::ValidationError { message } = outcome else {
        panic!("expected the lock request to be refused, got {outcome:?}");
    };
    message
}

#[test]
fn a_lock_request_is_answered_once_the_locker_covers_the_session() {
    let harness = session_with_stand_in_locker();
    let mut control = harness.control();

    let requested = control.call(Call::RequestLock, None).unwrap();
    let locked = harness.snapshot();
    let repeated = control.call(Call::RequestLock, None).unwrap();
    signal(&harness, UNLOCK_SIGNAL);
    let unlocked = harness.wait_until("the locker releases the session", |snapshot| {
        !snapshot.locked
    });

    assert!(matches!(requested, Outcome::Success { .. }));
    assert!(matches!(repeated, Outcome::Success { .. }));
    assert!(locked.locked);
    assert!(!unlocked.locked);
    assert_eq!(launches(&harness), 1);
}

#[test]
fn a_locker_that_dies_is_replaced_while_the_session_stays_locked() {
    let harness = session_with_stand_in_locker();
    let mut control = harness.control();
    control.call(Call::RequestLock, None).unwrap();
    harness.wait_until("the locker covers the session", |snapshot| snapshot.locked);

    signal(&harness, CRASH_SIGNAL);
    wait_until_consumed(&harness, CRASH_SIGNAL);
    let after_the_crash = harness.snapshot();
    signal(&harness, UNLOCK_SIGNAL);
    let unlocked = harness.wait_until("the new locker releases the session", |snapshot| {
        !snapshot.locked
    });

    assert!(after_the_crash.locked);
    assert!(!unlocked.locked);
    assert_eq!(launches(&harness), 2);
}

#[test]
fn a_locker_that_cannot_start_is_reported_and_nothing_locks() {
    let harness =
        Harness::start_from_file(&session_with_locker("['/nonexistent/tilling-test-locker']"));
    let mut control = harness.control();

    let outcome = control.call(Call::RequestLock, None).unwrap();

    let message = refused_as_invalid(outcome);
    assert!(
        message.starts_with("start the locker `/nonexistent/tilling-test-locker`"),
        "{message}"
    );
    assert!(!harness.snapshot().locked);
}

#[test]
fn a_locker_that_ends_without_locking_is_reported_and_nothing_locks() {
    let harness = Harness::start_from_file(&session_with_locker("['true']"));
    let mut control = harness.control();

    let outcome = control.call(Call::RequestLock, None).unwrap();

    assert_eq!(
        refused_as_invalid(outcome),
        "the locker ended without locking the session"
    );
    assert!(!harness.snapshot().locked);
}

#[test]
fn a_locker_that_never_locks_is_reported_after_the_limit() {
    let harness =
        Harness::start_from_file_with(&session_with_locker("['sleep', '600']"), |options| {
            options.lock_confirmation = SHORT_CONFIRMATION;
        });
    let mut control = harness.control();

    let outcome = control.call(Call::RequestLock, None).unwrap();

    assert_eq!(
        refused_as_invalid(outcome),
        "the locker did not lock the session within 150ms"
    );
    assert!(!harness.snapshot().locked);
}
