use tilling_test_support::ButtonEvent;

use crate::harness::Harness;
use crate::keys;

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
