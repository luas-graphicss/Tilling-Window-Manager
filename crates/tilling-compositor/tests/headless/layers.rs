use tilling_compositor::SessionSnapshot;
use tilling_core::geometry::LogicalRect;
use tilling_test_support::{Client, ClientError, KeyEvent, LayerRequest, ScreenEdge, ToplevelId};

use crate::harness::Harness;
use crate::keys;

const THICKNESS: u32 = 30;

fn area(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
    LogicalRect::new(x, y, width, height)
        .unwrap_or_else(|error| panic!("invalid test area: {error}"))
}

fn bar(edge: ScreenEdge) -> LayerRequest {
    LayerRequest::Bar {
        edge,
        thickness: THICKNESS,
    }
}

fn terminal(harness: &Harness) -> Result<(Client, ToplevelId), ClientError> {
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window)?;
    harness.published(1);
    Ok((client, window))
}

fn tile_becomes(
    harness: &Harness,
    client: &mut Client,
    expected: LogicalRect,
) -> Result<SessionSnapshot, ClientError> {
    client.settle()?;
    let settled = harness.wait_until("the tile reaches its new frame", |snapshot| {
        let frame = snapshot.scene.windows().first().map(|window| window.frame);
        !snapshot.transaction_pending && frame == Some(expected)
    });
    Ok(settled)
}

#[test]
fn a_bar_on_any_edge_keeps_tiles_out_of_its_exclusive_zone() {
    let cases = [
        (ScreenEdge::Top, (1280, 30), area(8, 38, 1264, 674)),
        (ScreenEdge::Bottom, (1280, 30), area(8, 8, 1264, 674)),
        (ScreenEdge::Left, (30, 720), area(38, 8, 1234, 704)),
        (ScreenEdge::Right, (30, 720), area(8, 8, 1234, 704)),
    ];

    for (edge, bar_size, tile) in cases {
        let harness = Harness::start();
        let (mut client, _) = terminal(&harness).unwrap();
        let mut panel = harness.connect();

        let layer = panel.map_layer(bar(edge)).unwrap();
        let snapshot = tile_becomes(&harness, &mut client, tile).unwrap();

        assert_eq!(panel.layer_size(layer), Some(bar_size), "{edge:?}");
        assert_eq!(snapshot.layers, 1, "{edge:?}");
    }
}

#[test]
fn bars_on_two_edges_reserve_both_zones() {
    let harness = Harness::start();
    let (mut client, _) = terminal(&harness).unwrap();
    let mut panel = harness.connect();

    panel.map_layer(bar(ScreenEdge::Top)).unwrap();
    panel.map_layer(bar(ScreenEdge::Left)).unwrap();
    let snapshot = tile_becomes(&harness, &mut client, area(38, 38, 1234, 674)).unwrap();

    assert_eq!(snapshot.layers, 2);
}

#[test]
fn a_destroyed_bar_returns_its_zone_to_the_tiles() {
    let harness = Harness::start();
    let (mut client, _) = terminal(&harness).unwrap();
    let mut panel = harness.connect();
    let layer = panel.map_layer(bar(ScreenEdge::Top)).unwrap();
    tile_becomes(&harness, &mut client, area(8, 38, 1264, 674)).unwrap();

    panel.destroy_layer(layer).unwrap();
    let snapshot = tile_becomes(&harness, &mut client, area(8, 8, 1264, 704)).unwrap();

    assert_eq!(snapshot.layers, 0);
}

#[test]
fn a_fullscreen_window_covers_the_zone_of_a_bar() {
    let harness = Harness::start();
    let (mut client, window) = terminal(&harness).unwrap();
    let mut panel = harness.connect();
    panel.map_layer(bar(ScreenEdge::Top)).unwrap();
    tile_becomes(&harness, &mut client, area(8, 38, 1264, 674)).unwrap();

    client.set_fullscreen(window, true).unwrap();
    let snapshot = tile_becomes(&harness, &mut client, area(0, 0, 1280, 720)).unwrap();
    harness.move_pointer(640.0, 10.0);
    client.roundtrip().unwrap();
    panel.roundtrip().unwrap();

    assert_eq!(snapshot.layers, 1);
    assert_eq!(client.pointer_focus(), Some(window));
    assert_eq!(panel.pointer_layer(), None);
}

#[test]
fn the_pointer_over_a_bar_belongs_to_the_bar() {
    let harness = Harness::start();
    let (mut client, _) = terminal(&harness).unwrap();
    let mut panel = harness.connect();
    let layer = panel.map_layer(bar(ScreenEdge::Top)).unwrap();
    tile_becomes(&harness, &mut client, area(8, 38, 1264, 674)).unwrap();

    harness.move_pointer(640.0, 10.0);
    panel.roundtrip().unwrap();
    client.roundtrip().unwrap();

    assert_eq!(panel.pointer_layer(), Some(layer));
    assert_eq!(client.pointer_focus(), None);
}

#[test]
fn a_launcher_takes_the_keyboard_and_returns_it_when_it_closes() {
    let harness = Harness::start();
    let (mut client, window) = terminal(&harness).unwrap();
    let mut launcher = harness.connect();

    let menu = launcher
        .map_layer(LayerRequest::Launcher {
            width: 400,
            height: 300,
        })
        .unwrap();
    launcher
        .wait_until("the launcher holds the keyboard", |launcher| {
            launcher.keyboard_layer() == Some(menu)
        })
        .unwrap();
    harness.chord(&[keys::A]);
    launcher.roundtrip().unwrap();
    client.roundtrip().unwrap();
    let typed_into_launcher = launcher.key_events().to_vec();
    let focus_while_open = client.keyboard_focus();
    launcher.destroy_layer(menu).unwrap();
    client
        .wait_until("the window holds the keyboard again", |client| {
            client.keyboard_focus() == Some(window)
        })
        .unwrap();

    let typed = [true, false].map(|pressed| KeyEvent {
        code: keys::A,
        pressed,
    });
    assert_eq!(typed_into_launcher, typed);
    assert_eq!(focus_while_open, None);
    assert!(client.key_events().is_empty());
}

#[test]
fn a_bar_does_not_take_the_keyboard_from_the_focused_window() {
    let harness = Harness::start();
    let (mut client, window) = terminal(&harness).unwrap();
    let mut panel = harness.connect();

    panel.map_layer(bar(ScreenEdge::Bottom)).unwrap();
    tile_becomes(&harness, &mut client, area(8, 8, 1264, 674)).unwrap();
    harness.move_pointer(640.0, 710.0);
    harness.click(keys::BUTTON_LEFT);
    client.roundtrip().unwrap();
    panel.roundtrip().unwrap();

    assert_eq!(client.keyboard_focus(), Some(window));
    assert_eq!(panel.keyboard_layer(), None);
}
