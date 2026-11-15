use tilling_test_support::Client;

use crate::harness::Harness;
use crate::keys;

#[test]
fn a_locked_pointer_stays_put_and_still_reports_its_travel() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let game = client.create_toplevel("org.example.Game", "level");
    client.map(game).unwrap();
    harness.published(1);
    harness.move_pointer(640.0, 360.0);
    client.roundtrip().unwrap();
    let before = client.pointer_position();

    client.lock_pointer(game).unwrap();
    client
        .wait_until("the pointer is locked", Client::pointer_locked)
        .unwrap();
    harness.nudge_pointer(25.0, -10.0);
    client.roundtrip().unwrap();

    assert_eq!(client.relative_travel(), (25.0, -10.0));
    assert_eq!(client.pointer_position(), before);
}

#[test]
fn an_unlocked_pointer_moves_and_reports_its_travel() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let game = client.create_toplevel("org.example.Game", "level");
    client.map(game).unwrap();
    harness.published(1);
    harness.move_pointer(640.0, 360.0);
    client.roundtrip().unwrap();
    let before = client.pointer_position().unwrap();

    harness.nudge_pointer(25.0, -10.0);
    client.roundtrip().unwrap();

    let after = client.pointer_position().unwrap();
    assert_eq!((after.0 - before.0, after.1 - before.1), (25.0, -10.0));
}

#[test]
fn a_pointer_lock_ends_when_its_window_loses_the_keyboard() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let editor = client.create_toplevel("org.example.Editor", "notes");
    client.map(editor).unwrap();
    client.settle().unwrap();
    harness.published(1);
    let game = client.create_toplevel("org.example.Game", "level");
    client.map(game).unwrap();
    client.settle().unwrap();
    let snapshot = harness.published(2);
    let focused = snapshot
        .scene
        .windows()
        .iter()
        .find(|window| window.focused)
        .unwrap()
        .frame;
    let x = f64::from(focused.x()) + f64::from(focused.width()) / 2.0;
    let y = f64::from(focused.y()) + f64::from(focused.height()) / 2.0;
    harness.move_pointer(x, y);
    client.roundtrip().unwrap();
    client.lock_pointer(game).unwrap();
    client
        .wait_until("the pointer is locked", Client::pointer_locked)
        .unwrap();

    harness.chord(&[keys::SUPER, keys::H]);
    client
        .wait_until("the lock ends with the focus change", |client| {
            !client.pointer_locked()
        })
        .unwrap();

    assert_eq!(client.keyboard_focus(), Some(editor));
}
