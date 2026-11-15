use tilling_compositor::{BORDER_WIDTH, SceneLayer};

use crate::harness::Harness;

const OUTPUT_WIDTH: u32 = 1280;
const OUTPUT_HEIGHT: u32 = 720;

#[test]
fn ten_windows_share_the_output_without_overlapping() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let windows: Vec<_> = (0..10)
        .map(|index| client.create_toplevel("org.example.Terminal", &format!("shell {index}")))
        .collect();

    for window in &windows {
        client.map(*window).unwrap();
        client.settle().unwrap();
    }
    let snapshot = harness.published(10);

    let placed = snapshot.scene.windows();
    for (index, window) in placed.iter().enumerate() {
        assert!(!window.frame.is_empty(), "window {index} has no area");
        assert!(
            placed[index + 1..]
                .iter()
                .all(|other| !window.frame.overlaps(&other.frame)),
            "window {index} overlaps another tile"
        );
    }
    for window in &windows {
        let configure = client.latest_configure(*window).unwrap();
        let matches_a_tile = placed.iter().any(|tile| {
            (tile.content.width(), tile.content.height()) == (configure.width, configure.height)
        });
        assert!(matches_a_tile, "configure {configure:?} matches no tile");
    }
    assert_eq!(snapshot.mapped_windows, 10);
    assert_eq!(placed.iter().filter(|window| window.focused).count(), 1);
}

#[test]
fn dialog_floats_above_its_parent_instead_of_taking_a_tile() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let parent = client.create_toplevel("org.example.Editor", "notes");
    client.map(parent).unwrap();
    let dialog = client.create_toplevel("org.example.Editor", "save as");
    client.set_parent(dialog, parent).unwrap();

    let configure = client.map(dialog).unwrap();
    client.settle().unwrap();
    let snapshot = harness.published(2);

    let [tile, floating] = snapshot.scene.windows() else {
        panic!("expected a tile and a dialog: {snapshot:?}");
    };
    assert_eq!((configure.width, configure.height), (0, 0));
    assert_eq!(tile.layer, SceneLayer::Tiled);
    assert_eq!(tile.content.width(), OUTPUT_WIDTH - 2 * (8 + BORDER_WIDTH));
    assert_eq!(floating.layer, SceneLayer::Floating);
    assert_eq!(
        (floating.content.width(), floating.content.height()),
        (640, 480)
    );
    assert!(floating.focused);
}

#[test]
fn fullscreen_covers_the_output_and_returns_to_the_previous_tile() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let first = client.create_toplevel("org.example.Terminal", "shell");
    let second = client.create_toplevel("org.example.Player", "video");
    client.map(first).unwrap();
    client.map(second).unwrap();
    client.settle().unwrap();
    let tiled = harness.published(2);

    client.set_fullscreen(second, true).unwrap();
    client.settle().unwrap();
    let fullscreen = harness.wait_until("the player is fullscreen", |snapshot| {
        !snapshot.transaction_pending
            && snapshot
                .scene
                .windows()
                .iter()
                .any(|window| window.layer == SceneLayer::Fullscreen)
    });
    client.set_fullscreen(second, false).unwrap();
    client.settle().unwrap();
    let restored = harness.wait_until("the player is tiled again", |snapshot| {
        !snapshot.transaction_pending
            && snapshot
                .scene
                .windows()
                .iter()
                .all(|window| window.layer == SceneLayer::Tiled)
    });

    let covering = fullscreen.scene.windows().last().unwrap();
    assert_eq!(
        (covering.content.width(), covering.content.height()),
        (OUTPUT_WIDTH, OUTPUT_HEIGHT)
    );
    assert!(client.latest_configure(second).unwrap().tiled);
    assert_eq!(restored.scene, tiled.scene);
}
