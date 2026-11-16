use tilling_compositor::Capability;
use tilling_test_support::{Client, ClientError, KeyEvent, ToplevelId};

use crate::harness::Harness;
use crate::keys;

fn terminal(harness: &Harness) -> Result<(Client, ToplevelId), ClientError> {
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window)?;
    harness.published(1);
    Ok((client, window))
}

fn locker(harness: &Harness) -> Client {
    harness.connect_with(&[Capability::SessionLock])
}

fn lock(harness: &Harness) -> Result<Client, ClientError> {
    let mut locker = locker(harness);
    locker.lock_session()?;
    locker.cover_output()?;
    locker.wait_until("the lock is confirmed", Client::lock_confirmed)?;
    locker.wait_until(
        "the lock surface holds the keyboard",
        Client::keyboard_on_lock_surface,
    )?;
    Ok(locker)
}

#[test]
fn locking_gives_the_input_to_the_locker_and_takes_it_from_windows() {
    let harness = Harness::start();
    let (mut client, window) = terminal(&harness).unwrap();
    harness.move_pointer(640.0, 360.0);
    client.roundtrip().unwrap();
    let pointer_before = client.pointer_focus();

    let mut locker = lock(&harness).unwrap();
    harness.chord(&[keys::A]);
    harness.move_pointer(600.0, 300.0);
    client.roundtrip().unwrap();
    locker.roundtrip().unwrap();

    let typed = [true, false].map(|pressed| KeyEvent {
        code: keys::A,
        pressed,
    });
    assert_eq!(pointer_before, Some(window));
    assert!(harness.snapshot().locked);
    assert_eq!(client.keyboard_focus(), None);
    assert_eq!(client.pointer_focus(), None);
    assert!(client.key_events().is_empty());
    assert_eq!(locker.key_events(), typed);
}

#[test]
fn the_lock_surface_is_asked_to_cover_its_whole_output() {
    let harness = Harness::start();
    let mut locker = locker(&harness);

    locker.lock_session().unwrap();
    let size = locker.cover_output().unwrap();

    assert_eq!(size, (1280, 720));
}

#[test]
fn shortcuts_do_nothing_while_the_session_is_locked() {
    let harness = Harness::start();
    let (mut client, window) = terminal(&harness).unwrap();
    let _locker = lock(&harness).unwrap();

    harness.chord(&[keys::SUPER, keys::SHIFT, keys::Q]);
    harness.chord(&[keys::SUPER, keys::DIGIT_2]);
    client.roundtrip().unwrap();
    let snapshot = harness.snapshot();

    assert!(!client.close_requested(window));
    assert_eq!(snapshot.scene.windows().len(), 1);
    assert!(snapshot.locked);
}

#[test]
fn the_session_stays_locked_when_its_locker_dies() {
    let harness = Harness::start();
    let (mut client, _) = terminal(&harness).unwrap();

    drop(lock(&harness).unwrap());
    harness.chord(&[keys::A]);
    harness.move_pointer(640.0, 360.0);
    client.roundtrip().unwrap();

    assert!(harness.snapshot().locked);
    assert_eq!(client.keyboard_focus(), None);
    assert_eq!(client.pointer_focus(), None);
    assert!(client.key_events().is_empty());
}

#[test]
fn a_locker_that_dies_before_the_confirmation_leaves_the_session_locked() {
    let harness = Harness::start();
    let (mut client, _) = terminal(&harness).unwrap();
    let mut dying = locker(&harness);

    dying.lock_session().unwrap();
    drop(dying);
    client.roundtrip().unwrap();
    let snapshot = harness.wait_until("the session is locked", |snapshot| snapshot.locked);

    assert!(snapshot.locked);
    assert_eq!(client.keyboard_focus(), None);
}

#[test]
fn a_new_locker_takes_over_a_session_left_locked_and_unlocks_it() {
    let harness = Harness::start();
    let (mut client, window) = terminal(&harness).unwrap();
    drop(lock(&harness).unwrap());

    let mut successor = lock(&harness).unwrap();
    successor.unlock_session().unwrap();
    harness.wait_until("the session is unlocked", |snapshot| !snapshot.locked);
    client
        .wait_until("the window holds the keyboard again", |client| {
            client.keyboard_focus() == Some(window)
        })
        .unwrap();
}

#[test]
fn a_second_locker_is_refused_while_the_first_holds_the_session() {
    let harness = Harness::start();
    let _holder = lock(&harness).unwrap();
    let mut intruder = locker(&harness);

    intruder.lock_session().unwrap();
    intruder
        .wait_until("the second lock is refused", Client::lock_refused)
        .unwrap();

    assert!(!intruder.lock_confirmed());
    assert!(harness.snapshot().locked);
}

#[test]
fn an_output_plugged_while_locked_shows_no_window_to_the_pointer() {
    let harness = Harness::start();
    let (mut client, _) = terminal(&harness).unwrap();
    let _locker = lock(&harness).unwrap();

    harness.plug(tilling_compositor::VirtualOutput {
        name: "LATE-1".to_owned(),
        width: 800,
        height: 600,
    });
    harness.move_pointer(1500.0, 200.0);
    harness.move_pointer(640.0, 360.0);
    client.roundtrip().unwrap();

    assert!(harness.snapshot().locked);
    assert_eq!(client.pointer_focus(), None);
}
