use std::thread;
use std::time::{Duration, Instant};

use tilling_compositor::SceneLayer;
use tilling_test_support::KeyEvent;

use crate::harness::Harness;
use crate::keys;

fn pressed(code: u32) -> KeyEvent {
    KeyEvent {
        code,
        pressed: true,
    }
}

fn released(code: u32) -> KeyEvent {
    KeyEvent {
        code,
        pressed: false,
    }
}

#[test]
fn unbound_key_reaches_the_focused_client() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    harness.chord(&[keys::A]);
    client.roundtrip().unwrap();

    assert_eq!(client.keyboard_focus(), Some(window));
    assert_eq!(client.key_events(), [pressed(keys::A), released(keys::A)]);
}

#[test]
fn shortcut_moves_focus_without_leaking_its_key_to_clients() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let left = client.create_toplevel("org.example.Terminal", "left");
    let right = client.create_toplevel("org.example.Terminal", "right");
    client.map(left).unwrap();
    client.map(right).unwrap();
    client.settle().unwrap();
    harness.published(2);
    let focused_before = client.keyboard_focus();

    let snapshot = harness.chord(&[keys::SUPER, keys::H]);
    client.settle().unwrap();

    let focused = snapshot.focused.unwrap();
    assert_eq!(focused_before, Some(right));
    assert_eq!(client.keyboard_focus(), Some(left));
    assert_eq!(
        client.key_events(),
        [pressed(keys::SUPER), released(keys::SUPER)]
    );
    assert!(client.latest_configure(left).unwrap().activated);
    assert!(!client.latest_configure(right).unwrap().activated);
    assert!(harness.published(2).scene.window(focused).unwrap().focused);
}

#[test]
fn close_shortcut_asks_the_client_instead_of_removing_the_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    let snapshot = harness.chord(&[keys::SUPER, keys::SHIFT, keys::Q]);
    client.roundtrip().unwrap();

    assert!(client.close_requested(window));
    assert_eq!(snapshot.mapped_windows, 1);
}

#[test]
fn workspace_shortcuts_switch_the_view_and_carry_the_focused_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    let mut focus_after = |codes: &[u32], visible: usize| {
        harness.chord(codes);
        let snapshot = harness.published(visible);
        client.roundtrip().unwrap();
        (snapshot, client.keyboard_focus())
    };
    let (empty, focus_on_empty) = focus_after(&[keys::SUPER, keys::DIGIT_2], 0);
    let (back, focus_back) = focus_after(&[keys::SUPER, keys::DIGIT_1], 1);
    let (sent_away, focus_sent_away) = focus_after(&[keys::SUPER, keys::SHIFT, keys::DIGIT_2], 0);
    let (followed, focus_followed) = focus_after(&[keys::SUPER, keys::DIGIT_2], 1);

    assert_eq!((empty.focused, focus_on_empty), (None, None));
    assert_eq!(focus_back, Some(window));
    assert_eq!((sent_away.mapped_windows, focus_sent_away), (1, None));
    assert_eq!(back.focused, followed.focused);
    assert_eq!(focus_followed, Some(window));
}

#[test]
fn fullscreen_and_floating_shortcuts_toggle_the_focused_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Player", "video");
    client.map(window).unwrap();
    let tiled = harness.published(1);
    let mut layer_after = |codes: &[u32]| {
        harness.chord(codes);
        client.settle().unwrap();
        harness.published(1).scene.windows()[0].layer
    };

    let fullscreen = layer_after(&[keys::SUPER, keys::F]);
    let restored = layer_after(&[keys::SUPER, keys::F]);
    let floating = layer_after(&[keys::SUPER, keys::SHIFT, keys::SPACE]);
    let retiled = layer_after(&[keys::SUPER, keys::SHIFT, keys::SPACE]);

    assert_eq!(
        [fullscreen, restored, floating, retiled],
        [
            SceneLayer::Fullscreen,
            SceneLayer::Tiled,
            SceneLayer::Floating,
            SceneLayer::Tiled
        ]
    );
    assert_eq!(harness.published(1).scene, tiled.scene);
}

#[test]
fn resize_mode_changes_the_split_until_escape_returns_to_the_default_mode() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let left = client.create_toplevel("org.example.Terminal", "left");
    let right = client.create_toplevel("org.example.Terminal", "right");
    client.map(left).unwrap();
    client.map(right).unwrap();
    client.settle().unwrap();
    let even = harness.published(2);

    let entered = harness.chord(&[keys::SUPER, keys::R]);
    harness.chord(&[keys::L]);
    client.settle().unwrap();
    let resized = harness.published(2);
    let left_mode = harness.chord(&[keys::ESCAPE]);
    harness.chord(&[keys::L]);
    client.settle().unwrap();

    let widths = |snapshot: &tilling_compositor::SessionSnapshot| -> Vec<u32> {
        snapshot
            .scene
            .windows()
            .iter()
            .map(|window| window.content.width())
            .collect()
    };
    assert_eq!(entered.keyboard_mode, "resize");
    assert_eq!(left_mode.keyboard_mode, "default");
    assert_eq!(widths(&even), [624, 624]);
    assert_ne!(widths(&resized), widths(&even));
    assert_eq!(widths(&resized).iter().sum::<u32>(), 1248);
    assert_eq!(
        client.key_events(),
        [
            pressed(keys::SUPER),
            released(keys::SUPER),
            pressed(keys::L),
            released(keys::L)
        ]
    );
}

#[test]
fn spawn_binding_starts_the_program_with_the_private_wayland_display() {
    let marker = tempfile::NamedTempFile::new().unwrap();
    let config = format!(
        r#"
schema_version = 1

[[bindings]]
keys = "Super+t"
action = {{ spawn = ["sh", "-c", 'printf %s "$WAYLAND_DISPLAY" > "$1"', "sh", "{}"] }}
"#,
        marker.path().display()
    );
    let harness = Harness::start_with_config(&config);

    harness.chord(&[keys::SUPER, keys::T]);
    let started = Instant::now();
    let mut written = String::new();
    while written.is_empty() && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(5));
        written = std::fs::read_to_string(marker.path()).unwrap();
    }

    assert_eq!(written, "tilling-test");
    assert!(harness.runtime_dir().join(&written).exists());
}

#[test]
fn key_held_when_the_host_window_loses_focus_no_longer_modifies_shortcuts() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let left = client.create_toplevel("org.example.Terminal", "left");
    let right = client.create_toplevel("org.example.Terminal", "right");
    client.map(left).unwrap();
    client.map(right).unwrap();
    client.settle().unwrap();
    let before = harness.published(2);

    harness.hold(keys::SUPER);
    harness.lose_host_focus();
    let after = harness.chord(&[keys::H]);
    client.roundtrip().unwrap();

    assert_eq!(after.focused, before.focused);
    assert_eq!(
        client.key_events(),
        [
            pressed(keys::SUPER),
            released(keys::SUPER),
            pressed(keys::H),
            released(keys::H)
        ]
    );
}
