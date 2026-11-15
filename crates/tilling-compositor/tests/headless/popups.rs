use tilling_test_support::{
    Client, ClientError, KeyEvent, PopupConfigure, PopupPlacement, ToplevelId,
};

use crate::harness::Harness;
use crate::keys;

const MENU: PopupPlacement = PopupPlacement {
    x: 20,
    y: 30,
    width: 200,
    height: 100,
};

fn mapped_window(harness: &Harness, client: &mut Client) -> Result<ToplevelId, ClientError> {
    let window = client.create_toplevel("org.example.Editor", "notes");
    client.map(window)?;
    harness.published(1);
    Ok(window)
}

fn press_on_window(harness: &Harness, client: &mut Client) -> Result<(), ClientError> {
    harness.move_pointer(300.0, 200.0);
    harness.press(keys::BUTTON_LEFT);
    client.roundtrip()
}

#[test]
fn popup_is_configured_where_the_client_anchored_it() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = mapped_window(&harness, &mut client).unwrap();

    let popup = client.create_popup(window, MENU).unwrap();
    let configure = client.map_popup(popup).unwrap();
    let snapshot = harness.snapshot();

    assert_eq!(
        configure,
        PopupConfigure {
            x: 20,
            y: 30,
            width: 200,
            height: 100
        }
    );
    assert_eq!(snapshot.popups, 1);
    assert!(!client.popup_dismissed(popup));
}

#[test]
fn popup_that_would_leave_the_output_slides_back_inside() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = mapped_window(&harness, &mut client).unwrap();
    let near_the_corner = PopupPlacement {
        x: 1200,
        y: 650,
        width: 300,
        height: 200,
    };

    let popup = client.create_popup(window, near_the_corner).unwrap();
    let configure = client.map_popup(popup).unwrap();

    assert_eq!(
        configure,
        PopupConfigure {
            x: 970,
            y: 510,
            width: 300,
            height: 200
        }
    );
}

#[test]
fn grabbed_popup_takes_the_keyboard_until_a_click_outside_dismisses_it() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = mapped_window(&harness, &mut client).unwrap();
    press_on_window(&harness, &mut client).unwrap();
    let popup = client.create_popup(window, MENU).unwrap();

    client.grab_popup(popup).unwrap();
    client.map_popup(popup).unwrap();
    let grabbing = client.popup_with_keyboard_focus();
    harness.release(keys::BUTTON_LEFT);
    harness.move_pointer(4.0, 4.0);
    harness.click(keys::BUTTON_LEFT);
    client.roundtrip().unwrap();

    assert_eq!(grabbing, Some(popup));
    assert!(client.popup_dismissed(popup));
    assert_eq!(client.keyboard_focus(), Some(window));
}

#[test]
fn grab_without_a_recent_input_serial_dismisses_the_popup() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = mapped_window(&harness, &mut client).unwrap();
    press_on_window(&harness, &mut client).unwrap();
    let forged = client.button_serial().unwrap().wrapping_add(1000);
    let popup = client.create_popup(window, MENU).unwrap();

    client.grab_popup_with(popup, forged).unwrap();

    assert!(client.popup_dismissed(popup));
    assert_eq!(client.keyboard_focus(), Some(window));
    assert_eq!(client.popup_with_keyboard_focus(), None);
}

#[test]
fn destroying_a_grabbed_popup_returns_the_keyboard_to_its_window() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = mapped_window(&harness, &mut client).unwrap();
    press_on_window(&harness, &mut client).unwrap();
    let popup = client.create_popup(window, MENU).unwrap();
    client.grab_popup(popup).unwrap();
    client.map_popup(popup).unwrap();
    harness.release(keys::BUTTON_LEFT);

    client.destroy_popup(popup).unwrap();
    harness.chord(&[keys::A]);
    client.roundtrip().unwrap();
    let snapshot = harness.snapshot();

    assert_eq!(snapshot.popups, 0);
    assert_eq!(client.keyboard_focus(), Some(window));
    assert!(client.key_events().ends_with(&[
        KeyEvent {
            code: keys::A,
            pressed: true
        },
        KeyEvent {
            code: keys::A,
            pressed: false
        }
    ]));
}
