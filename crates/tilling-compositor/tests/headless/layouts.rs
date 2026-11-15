use tilling_compositor::SessionSnapshot;
use tilling_core::geometry::LogicalRect;
use tilling_test_support::{Client, ToplevelId};

use crate::harness::Harness;
use crate::keys;

const MONOCLE_BY_DEFAULT: &str =
    "schema_version = 1\n\n[layout]\ninner_gap = 8\nouter_gap = 8\ndefault = \"monocle\"\n";

fn frame(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
    LogicalRect::new(x, y, width, height)
        .unwrap_or_else(|error| panic!("invalid test frame: {error}"))
}

fn frames(snapshot: &SessionSnapshot) -> Vec<LogicalRect> {
    snapshot
        .scene
        .windows()
        .iter()
        .map(|window| window.frame)
        .collect()
}

fn open(client: &mut Client, titles: &[&str]) -> Vec<ToplevelId> {
    titles
        .iter()
        .map(|title| {
            let window = client.create_toplevel("org.example.Terminal", title);
            client
                .map(window)
                .and_then(|_| client.settle())
                .unwrap_or_else(|error| panic!("window {title} was not mapped: {error}"));
            window
        })
        .collect()
}

fn only_the_front_window_is_shown(snapshot: &SessionSnapshot, mapped: usize) -> bool {
    !snapshot.transaction_pending
        && snapshot.mapped_windows == mapped
        && matches!(snapshot.scene.windows(), [front] if front.focused)
}

#[test]
fn layout_shortcut_cycles_through_master_stack_and_monocle_back_to_bsp() {
    let harness = Harness::start();
    let mut client = harness.connect();
    open(&mut client, &["editor", "shell", "logs", "notes"]);
    let bsp = harness.published(4);
    let mut cycle = |visible: usize| {
        harness.chord(&[keys::SUPER, keys::SPACE]);
        client.settle().unwrap();
        harness.published(visible)
    };

    let master_stack = cycle(4);
    let monocle = cycle(1);
    let restored = cycle(4);

    assert_eq!(
        frames(&master_stack),
        [
            frame(8, 8, 628, 704),
            frame(644, 8, 628, 230),
            frame(644, 246, 628, 229),
            frame(644, 483, 628, 229),
        ]
    );
    assert_eq!(frames(&monocle), [frame(8, 8, 1264, 704)]);
    assert!(monocle.scene.windows()[0].focused);
    assert_eq!(restored.scene, bsp.scene);
}

#[test]
fn configured_default_layout_is_used_from_the_first_window() {
    let harness = Harness::start_with_config(MONOCLE_BY_DEFAULT);
    let mut client = harness.connect();

    let windows = open(&mut client, &["editor", "shell"]);
    let snapshot = harness.wait_until("only the newest window is shown", |snapshot| {
        only_the_front_window_is_shown(snapshot, 2)
    });
    client.roundtrip().unwrap();

    assert_eq!(frames(&snapshot), [frame(8, 8, 1264, 704)]);
    assert_eq!(client.keyboard_focus(), Some(windows[1]));
}

#[test]
fn focus_shortcut_brings_the_next_monocle_window_to_the_front() {
    let harness = Harness::start_with_config(MONOCLE_BY_DEFAULT);
    let mut client = harness.connect();
    let windows = open(&mut client, &["editor", "shell", "logs"]);
    let newest = harness.wait_until("only the newest window is shown", |snapshot| {
        only_the_front_window_is_shown(snapshot, 3)
    });
    let hidden_before = newest.scene.windows()[0].id;

    harness.chord(&[keys::SUPER, keys::L]);
    client.settle().unwrap();
    let cycled = harness.wait_until("the first window is brought to the front", |snapshot| {
        only_the_front_window_is_shown(snapshot, 3)
            && snapshot.scene.windows()[0].id != hidden_before
    });
    client.roundtrip().unwrap();

    assert_eq!(frames(&cycled), [frame(8, 8, 1264, 704)]);
    assert_eq!(client.keyboard_focus(), Some(windows[0]));
}
