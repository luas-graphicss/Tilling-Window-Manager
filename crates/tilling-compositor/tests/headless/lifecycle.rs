use crate::harness::Harness;

const CYCLES: usize = 1_000;

#[test]
fn initial_commit_is_configured_with_the_tile_the_window_will_get() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    client.commit(window).unwrap();
    let configure = client.wait_for_configure(window).unwrap();

    assert_eq!((configure.width, configure.height), (1260, 700));
    assert!(configure.activated && configure.tiled && !configure.fullscreen);
    assert_eq!(harness.snapshot().mapped_windows, 0);
}

#[test]
fn first_buffer_maps_and_focuses_the_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    client.map(window).unwrap();
    let snapshot = harness.published(1);

    let placed = snapshot.scene.windows()[0];
    assert_eq!(snapshot.focused, Some(placed.id));
    assert_eq!(
        (placed.content.width(), placed.content.height()),
        (1260, 700)
    );
    assert_eq!(client.unacknowledged_configures(window), 0);
}

#[test]
fn unmapped_window_needs_a_new_initial_configure_before_it_returns() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    let first = client.map(window).unwrap();
    harness.published(1);

    client.unmap(window).unwrap();
    let unmapped = harness.published(0);
    let second = client.map(window).unwrap();
    let remapped = harness.published(1);

    assert_eq!((unmapped.mapped_windows, unmapped.toplevels), (0, 1));
    assert_ne!(first.serial, second.serial);
    assert_eq!((second.width, second.height), (first.width, first.height));
    assert_eq!(remapped.mapped_windows, 1);
}

#[test]
fn windows_destroyed_out_of_order_leave_the_rest_tiled() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let windows: Vec<_> = (0..4)
        .map(|index| client.create_toplevel("org.example.Terminal", &format!("shell {index}")))
        .collect();
    for window in &windows {
        client.map(*window).unwrap();
        client.settle().unwrap();
    }
    harness.published(4);

    client.destroy_toplevel(windows[1]).unwrap();
    client.destroy_toplevel(windows[3]).unwrap();
    client.settle().unwrap();
    let snapshot = harness.published(2);

    let covered: u64 = snapshot
        .scene
        .windows()
        .iter()
        .map(|window| window.frame.area())
        .sum();
    assert_eq!((snapshot.toplevels, snapshot.mapped_windows), (2, 2));
    assert_eq!(covered, (1264 - 8) * 704);
}

#[test]
fn disconnecting_client_takes_its_windows_with_it() {
    let harness = Harness::start();
    let mut staying = harness.connect();
    let kept = staying.create_toplevel("org.example.Editor", "notes");
    staying.map(kept).unwrap();
    let mut leaving = harness.connect();
    let lost = leaving.create_toplevel("org.example.Terminal", "shell");
    leaving.map(lost).unwrap();
    staying.settle().unwrap();
    harness.published(2);

    drop(leaving);
    let snapshot = harness.wait_until("the disconnected client is gone", |snapshot| {
        snapshot.toplevels == 1
    });
    staying.settle().unwrap();
    let settled = harness.published(1);

    assert_eq!(snapshot.mapped_windows, 1);
    assert_eq!(settled.scene.windows()[0].content.width(), 1260);
}

#[test]
fn repeated_map_and_destroy_cycles_do_not_accumulate_resources() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let anchor = client.create_toplevel("org.example.Editor", "anchor");
    client.map(anchor).unwrap();

    for cycle in 0..CYCLES {
        let window = client.create_toplevel("org.example.Terminal", "transient");
        client.map(window).unwrap();
        if cycle % 2 == 0 {
            client.unmap(window).unwrap();
        }
        client.destroy_toplevel(window).unwrap();
    }
    client.settle().unwrap();
    let snapshot = harness.published(1);

    assert_eq!((snapshot.toplevels, snapshot.mapped_windows), (1, 1));
    assert_eq!(snapshot.popups, 0);
    assert_eq!(snapshot.scene.windows()[0].content.width(), 1260);
}

#[test]
fn buffer_larger_than_its_pool_disconnects_only_the_client_that_sent_it() {
    let harness = Harness::start();
    let mut healthy = harness.connect();
    let kept = healthy.create_toplevel("org.example.Editor", "notes");
    healthy.map(kept).unwrap();
    let mut faulty = harness.connect();
    let window = faulty.create_toplevel("org.example.Viewer", "broken");
    faulty.map(window).unwrap();
    healthy.settle().unwrap();
    harness.published(2);

    faulty.attach_buffer_beyond_its_pool(window).unwrap();
    let rejection = faulty.roundtrip().unwrap_err();
    harness.wait_until("the faulty client is gone", |snapshot| {
        snapshot.toplevels == 1
    });
    healthy.settle().unwrap();
    let snapshot = harness.published(1);

    assert_eq!(rejection.protocol_error(), Some(("wl_shm_pool", 1)));
    assert_eq!(snapshot.mapped_windows, 1);
    assert_eq!(snapshot.scene.windows()[0].content.width(), 1260);
}
