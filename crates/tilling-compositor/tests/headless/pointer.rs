use tilling_core::geometry::LogicalRect;
use tilling_test_support::{
    ButtonEvent, Client, ClientError, InputRegion, ResizeEdge, ScrollEvent, ToplevelId,
};

use crate::harness::Harness;
use crate::keys;

const CLICKABLE_CORNER: InputRegion = InputRegion {
    x: 0,
    y: 0,
    width: 300,
    height: 200,
};

fn floating_window(
    harness: &Harness,
    client: &mut Client,
) -> Result<(ToplevelId, LogicalRect), ClientError> {
    let window = client.create_toplevel("org.example.Dialog", "settings");
    client.map(window)?;
    harness.published(1);
    harness.chord(&[keys::SUPER, keys::SHIFT, keys::SPACE]);
    client.settle()?;
    Ok((window, frame(harness)))
}

fn frame(harness: &Harness) -> LogicalRect {
    harness.published(1).scene.windows()[0].frame
}

fn press_inside(harness: &Harness, client: &mut Client) -> Result<(), ClientError> {
    harness.move_pointer(300.0, 200.0);
    harness.press(keys::BUTTON_LEFT);
    client.roundtrip()
}

fn drag_by(harness: &Harness, client: &mut Client, dx: f64, dy: f64) -> Result<(), ClientError> {
    harness.move_pointer(300.0 + dx, 200.0 + dy);
    harness.release(keys::BUTTON_LEFT);
    client.settle()
}

#[test]
fn pointer_enters_the_surface_under_it_with_surface_local_coordinates() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    harness.move_pointer(110.0, 60.0);
    client.roundtrip().unwrap();
    let entered = (client.pointer_focus(), client.pointer_position());
    harness.move_pointer(310.5, 260.25);
    client.roundtrip().unwrap();

    assert_eq!(entered, (Some(window), Some((100.0, 50.0))));
    assert_eq!(client.pointer_position(), Some((300.5, 250.25)));
}

#[test]
fn gaps_and_borders_do_not_deliver_pointer_focus() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    harness.move_pointer(110.0, 60.0);
    client.roundtrip().unwrap();
    let inside = client.pointer_focus();
    harness.move_pointer(9.0, 9.0);
    client.roundtrip().unwrap();
    let on_border = client.pointer_focus();
    harness.move_pointer(4.0, 4.0);
    client.roundtrip().unwrap();

    assert_eq!(inside, Some(window));
    assert_eq!(on_border, None);
    assert_eq!(client.pointer_focus(), None);
}

#[test]
fn pointer_outside_the_input_region_does_not_reach_the_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);
    client.set_input_region(window, CLICKABLE_CORNER).unwrap();

    harness.move_pointer(110.0, 60.0);
    client.roundtrip().unwrap();
    let inside = client.pointer_focus();
    harness.move_pointer(600.0, 400.0);
    client.roundtrip().unwrap();

    assert_eq!(inside, Some(window));
    assert_eq!(client.pointer_focus(), None);
}

#[test]
fn clicking_a_window_focuses_it_and_still_delivers_the_click() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let left = client.create_toplevel("org.example.Terminal", "left");
    let right = client.create_toplevel("org.example.Terminal", "right");
    client.map(left).unwrap();
    client.map(right).unwrap();
    client.settle().unwrap();
    let before = harness.published(2);

    harness.move_pointer(100.0, 100.0);
    let after = harness.click(keys::BUTTON_LEFT);
    client.settle().unwrap();

    assert_ne!(before.focused, after.focused);
    assert_eq!(client.keyboard_focus(), Some(left));
    assert_eq!(client.pointer_focus(), Some(left));
    assert_eq!(
        client.button_events(),
        [
            ButtonEvent {
                code: keys::BUTTON_LEFT,
                pressed: true
            },
            ButtonEvent {
                code: keys::BUTTON_LEFT,
                pressed: false
            }
        ]
    );
}

#[test]
fn scroll_reaches_the_surface_under_the_pointer() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);

    harness.move_pointer(110.0, 60.0);
    harness.scroll(0.0, 30.0);
    harness.scroll(-15.0, 0.0);
    client.roundtrip().unwrap();

    assert_eq!(
        client.scroll_events(),
        [
            ScrollEvent {
                horizontal: 0.0,
                vertical: 30.0
            },
            ScrollEvent {
                horizontal: -15.0,
                vertical: 0.0
            }
        ]
    );
}

#[test]
fn floating_window_follows_the_pointer_after_a_move_request() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let (window, before) = floating_window(&harness, &mut client).unwrap();

    press_inside(&harness, &mut client).unwrap();
    client.request_move(window).unwrap();
    drag_by(&harness, &mut client, 40.0, 25.0).unwrap();
    let after = frame(&harness);

    assert_eq!((after.x() - before.x(), after.y() - before.y()), (40, 25));
    assert_eq!(
        (after.width(), after.height()),
        (before.width(), before.height())
    );
}

#[test]
fn resize_request_stretches_the_grabbed_edges_and_reconfigures_the_client() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let (window, before) = floating_window(&harness, &mut client).unwrap();

    press_inside(&harness, &mut client).unwrap();
    client
        .request_resize(window, ResizeEdge::BottomRight)
        .unwrap();
    drag_by(&harness, &mut client, -30.0, -20.0).unwrap();
    let after = harness.published(1).scene.windows()[0];

    let configure = client.latest_configure(window).unwrap();
    assert_eq!((after.frame.x(), after.frame.y()), (before.x(), before.y()));
    assert_eq!(
        (after.frame.width(), after.frame.height()),
        (before.width() - 30, before.height() - 20)
    );
    assert_eq!(
        (configure.width, configure.height),
        (after.content.width(), after.content.height())
    );
}

#[test]
fn move_request_without_the_serial_of_the_press_is_ignored() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let (window, before) = floating_window(&harness, &mut client).unwrap();

    press_inside(&harness, &mut client).unwrap();
    let forged = client.button_serial().unwrap().wrapping_add(1000);
    client.request_move_with(window, forged).unwrap();
    drag_by(&harness, &mut client, 40.0, 25.0).unwrap();

    assert_eq!(frame(&harness), before);
}

#[test]
fn tiled_window_keeps_its_tile_and_the_pointer_after_a_move_request() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    let before = frame(&harness);

    press_inside(&harness, &mut client).unwrap();
    client.request_move(window).unwrap();
    drag_by(&harness, &mut client, 40.0, 25.0).unwrap();

    assert_eq!(frame(&harness), before);
    assert_eq!(client.pointer_focus(), Some(window));
    assert_eq!(client.pointer_position(), Some((330.0, 215.0)));
}
