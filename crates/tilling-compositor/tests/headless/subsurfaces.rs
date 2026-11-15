use tilling_test_support::SubsurfacePlacement;

use crate::harness::Harness;

const OVERLAY: SubsurfacePlacement = SubsurfacePlacement {
    x: 200,
    y: 100,
    width: 120,
    height: 80,
};

#[test]
fn subsurface_takes_the_pointer_only_after_its_parent_commits() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Player", "video");
    client.map(window).unwrap();
    harness.published(1);
    let overlay = client.create_subsurface(window, OVERLAY).unwrap();
    client.roundtrip().unwrap();

    harness.move_pointer(240.0, 130.0);
    client.roundtrip().unwrap();
    let before_parent_commit = (client.subsurface_under_pointer(), client.pointer_focus());
    client.commit(window).unwrap();
    client.roundtrip().unwrap();
    harness.move_pointer(241.0, 131.0);
    client.roundtrip().unwrap();

    assert_eq!(before_parent_commit, (None, Some(window)));
    assert_eq!(client.subsurface_under_pointer(), Some(overlay));
    assert_eq!(client.pointer_focus(), None);
    assert_eq!(client.pointer_position(), Some((31.0, 21.0)));
}

#[test]
fn destroyed_subsurface_gives_the_pointer_back_to_its_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Player", "video");
    client.map(window).unwrap();
    harness.published(1);
    let overlay = client.create_subsurface(window, OVERLAY).unwrap();
    client.commit(window).unwrap();
    client.roundtrip().unwrap();
    harness.move_pointer(240.0, 130.0);
    client.roundtrip().unwrap();
    let over_overlay = client.subsurface_under_pointer();

    client.destroy_subsurface(overlay).unwrap();
    harness.move_pointer(241.0, 131.0);
    client.roundtrip().unwrap();

    assert_eq!(over_overlay, Some(overlay));
    assert_eq!(client.subsurface_under_pointer(), None);
    assert_eq!(client.pointer_focus(), Some(window));
    assert_eq!(client.pointer_position(), Some((231.0, 121.0)));
}

#[test]
fn window_destroyed_before_its_subsurface_leaves_the_rest_tiled() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let kept = client.create_toplevel("org.example.Editor", "notes");
    client.map(kept).unwrap();
    let parent = client.create_toplevel("org.example.Player", "video");
    client.map(parent).unwrap();
    client.settle().unwrap();
    harness.published(2);
    let overlay = client.create_subsurface(parent, OVERLAY).unwrap();
    client.commit(parent).unwrap();
    client.roundtrip().unwrap();

    client.destroy_toplevel(parent).unwrap();
    client.destroy_subsurface(overlay).unwrap();
    client.settle().unwrap();
    let snapshot = harness.published(1);

    assert_eq!(snapshot.mapped_windows, 1);
    assert_eq!(snapshot.scene.windows()[0].content.width(), 1260);
}
