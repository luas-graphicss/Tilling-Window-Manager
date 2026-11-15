use std::thread;
use std::time::Duration;

use crate::harness::Harness;

const OBSERVATION: Duration = Duration::from_millis(250);
const CURSOR_SIZE: u32 = 16;

#[test]
fn still_scene_is_not_presented_again() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    let settled = harness.idle_frames();
    thread::sleep(OBSERVATION);
    let later = harness.snapshot().frames;

    assert!(settled.damaged > 0);
    assert_eq!(later, settled);
}

#[test]
fn commit_without_new_content_is_presented_without_damage() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);
    let before = harness.idle_frames();

    client.commit(window).unwrap();
    client.roundtrip().unwrap();
    let after = harness.idle_frames();

    assert!(after.presented > before.presented);
    assert_eq!(after.damaged, before.damaged);
}

#[test]
fn replaced_buffer_is_released_to_the_client() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    let content = harness.published(1).scene.windows()[0].content;
    harness.idle_frames();
    client.roundtrip().unwrap();
    let released_before = client.released_buffers();

    client
        .replace_buffer(window, content.width(), content.height())
        .unwrap();
    harness.idle_frames();
    client.roundtrip().unwrap();

    assert_eq!(client.released_buffers(), released_before + 1);
}

#[test]
fn new_content_damages_only_the_window_that_committed_it() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let left = client.create_toplevel("org.example.Editor", "notes");
    let right = client.create_toplevel("org.example.Terminal", "shell");
    client.map(left).unwrap();
    client.map(right).unwrap();
    client.settle().unwrap();
    let content = harness.published(2).scene.windows()[0].content;
    let before = harness.idle_frames();

    client
        .attach(left, content.width(), content.height())
        .unwrap();
    client.commit(left).unwrap();
    client.roundtrip().unwrap();
    let after = harness.idle_frames();

    assert_eq!(after.damaged, before.damaged + 1);
    assert_eq!(after.damaged_pixels, content.area());
}

#[test]
fn client_cursor_is_repainted_only_where_it_was_and_where_it_went() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);
    harness.move_pointer(300.0, 200.0);
    client.roundtrip().unwrap();
    let hidden = harness.idle_frames();

    client.set_cursor(CURSOR_SIZE, 4).unwrap();
    let shown = harness.idle_frames();
    harness.move_pointer(600.0, 400.0);
    let moved = harness.idle_frames();

    assert!(shown.damaged > hidden.damaged);
    assert_eq!(moved.damaged, shown.damaged + 1);
    assert_eq!(
        moved.damaged_pixels,
        2 * u64::from(CURSOR_SIZE * CURSOR_SIZE)
    );
}
