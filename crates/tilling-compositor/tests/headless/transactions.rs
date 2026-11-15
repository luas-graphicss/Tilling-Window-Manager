use std::time::{Duration, Instant};

use crate::harness::Harness;

const DEADLINE: Duration = Duration::from_millis(600);

#[test]
fn layout_is_published_together_once_every_resized_window_commits() {
    let harness =
        Harness::start_with(|options| options.transaction_timeout = Duration::from_secs(30));
    let mut client = harness.connect();
    let first = client.create_toplevel("org.example.Terminal", "shell");
    client.map(first).unwrap();
    let alone = harness.published(1);
    let second = client.create_toplevel("org.example.Editor", "notes");

    client.map(second).unwrap();
    let waiting = harness.snapshot();
    client.settle().unwrap();
    let together = harness.published(2);

    assert!(waiting.transaction_pending);
    assert_eq!(waiting.scene, alone.scene);
    assert_eq!(waiting.mapped_windows, 2);
    assert!(
        together
            .scene
            .windows()
            .iter()
            .all(|window| window.content.width() == 624)
    );
}

#[test]
fn client_that_ignores_configure_only_delays_the_layout_until_the_deadline() {
    let harness = Harness::start_with(|options| options.transaction_timeout = DEADLINE);
    let mut stuck = harness.connect();
    let unresponsive = stuck.create_toplevel("org.example.Frozen", "not responding");
    stuck.map(unresponsive).unwrap();
    harness.published(1);
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    let started = Instant::now();
    client.map(window).unwrap();
    let waiting = harness.snapshot();
    let published = harness.published(2);
    let waited = started.elapsed();

    assert!(waiting.transaction_pending);
    assert_eq!(waiting.scene.windows().len(), 1);
    assert!(waited >= DEADLINE, "published after only {waited:?}");
    assert!(waited < DEADLINE * 4, "published after {waited:?}");
    assert_eq!(published.mapped_windows, 2);
}

#[test]
fn destroying_the_awaited_window_releases_the_transaction() {
    let harness =
        Harness::start_with(|options| options.transaction_timeout = Duration::from_secs(30));
    let mut stuck = harness.connect();
    let unresponsive = stuck.create_toplevel("org.example.Frozen", "not responding");
    stuck.map(unresponsive).unwrap();
    harness.published(1);
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    let waiting = harness.snapshot();

    drop(stuck);
    client.settle().unwrap();
    let published = harness.published(1);

    assert!(waiting.transaction_pending);
    assert_eq!(published.scene.windows()[0].content.width(), 1260);
}
