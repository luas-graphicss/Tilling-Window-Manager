use std::time::Duration;

use crate::harness::Harness;
use crate::keys;

const TIMEOUT: Duration = Duration::from_millis(60);

#[test]
fn a_watcher_learns_when_input_stops_and_when_it_returns() {
    let harness = Harness::start();
    let mut watcher = harness.connect();

    let watch = watcher.watch_idleness(TIMEOUT).unwrap();
    watcher
        .wait_until("the session is reported idle", |watcher| {
            watcher.is_idle(watch)
        })
        .unwrap();
    harness.chord(&[keys::A]);
    watcher
        .wait_until("the returning input is reported", |watcher| {
            !watcher.is_idle(watch)
        })
        .unwrap();
    watcher
        .wait_until("the session is reported idle again", |watcher| {
            watcher.is_idle(watch)
        })
        .unwrap();
}

#[test]
fn a_shown_window_that_inhibits_idleness_keeps_the_session_awake() {
    let harness = Harness::start();
    let mut player = harness.connect();
    let video = player.create_toplevel("org.example.Player", "film");
    player.map(video).unwrap();
    harness.published(1);
    player.inhibit_idleness(video).unwrap();
    let mut watcher = harness.connect();

    let obeying = watcher.watch_idleness(TIMEOUT).unwrap();
    let input_only = watcher.watch_input_idleness(TIMEOUT).unwrap();
    watcher
        .wait_until("the lack of input is reported", |watcher| {
            watcher.is_idle(input_only)
        })
        .unwrap();
    let idle_while_inhibited = watcher.is_idle(obeying);
    player.allow_idleness().unwrap();
    watcher
        .wait_until("idleness is reported once nothing inhibits it", |watcher| {
            watcher.is_idle(obeying)
        })
        .unwrap();

    assert!(!idle_while_inhibited);
}

#[test]
fn an_inhibitor_on_a_hidden_workspace_does_not_keep_the_session_awake() {
    let harness = Harness::start();
    let mut player = harness.connect();
    let video = player.create_toplevel("org.example.Player", "film");
    player.map(video).unwrap();
    harness.published(1);
    player.inhibit_idleness(video).unwrap();
    harness.chord(&[keys::SUPER, keys::DIGIT_2]);
    harness.published(0);
    let mut watcher = harness.connect();

    let watch = watcher.watch_idleness(TIMEOUT).unwrap();
    watcher
        .wait_until("the session is reported idle", |watcher| {
            watcher.is_idle(watch)
        })
        .unwrap();
}
