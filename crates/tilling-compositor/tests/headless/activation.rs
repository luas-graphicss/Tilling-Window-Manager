use tilling_compositor::SessionSnapshot;
use tilling_test_support::{Client, ClientError, ToplevelId};

use crate::harness::Harness;
use crate::keys;

struct Desk {
    harness: Harness,
    background: Client,
    hidden: ToplevelId,
    focused: Client,
    shown: ToplevelId,
}

fn desk() -> Result<Desk, ClientError> {
    let harness = Harness::start();
    let mut background = harness.connect();
    let hidden = background.create_toplevel("org.example.Mail", "inbox");
    background.map(hidden)?;
    harness.published(1);
    let mut focused = harness.connect();
    let shown = focused.create_toplevel("org.example.Browser", "page");
    focused.map(shown)?;
    background.settle()?;
    focused.settle()?;
    harness.published(2);
    Ok(Desk {
        harness,
        background,
        hidden,
        focused,
        shown,
    })
}

fn click_focused_window(harness: &Harness, snapshot: &SessionSnapshot) {
    let focused = snapshot
        .scene
        .windows()
        .iter()
        .find(|window| window.focused);
    let Some(frame) = focused.map(|window| window.frame) else {
        panic!("no window is focused: {snapshot:?}");
    };
    let x = f64::from(frame.x()) + f64::from(frame.width()) / 2.0;
    let y = f64::from(frame.y()) + f64::from(frame.height()) / 2.0;
    harness.move_pointer(x, y);
    harness.click(keys::BUTTON_LEFT);
}

fn clicked_serial(desk: &mut Desk) -> Result<u32, ClientError> {
    let snapshot = desk.harness.snapshot();
    click_focused_window(&desk.harness, &snapshot);
    desk.focused.roundtrip()?;
    desk.focused
        .button_serial()
        .ok_or(ClientError::NoButtonSerial)
}

#[test]
fn a_token_backed_by_a_click_lets_another_window_take_the_keyboard() {
    let mut desk = desk().unwrap();
    let serial = clicked_serial(&mut desk).unwrap();

    let token = desk.focused.request_activation_token(Some(serial)).unwrap();
    desk.background.activate(&token, desk.hidden).unwrap();
    desk.background
        .wait_until("the activated window holds the keyboard", |client| {
            client.keyboard_focus().is_some()
        })
        .unwrap();
    desk.focused.roundtrip().unwrap();

    assert_eq!(desk.background.keyboard_focus(), Some(desk.hidden));
    assert_eq!(desk.focused.keyboard_focus(), None);
}

#[test]
fn a_token_without_an_input_serial_does_not_move_the_keyboard() {
    let mut desk = desk().unwrap();

    let token = desk.background.request_activation_token(None).unwrap();
    desk.background.activate(&token, desk.hidden).unwrap();
    desk.focused.roundtrip().unwrap();

    assert_eq!(desk.focused.keyboard_focus(), Some(desk.shown));
    assert_eq!(desk.background.keyboard_focus(), None);
}

#[test]
fn a_token_requested_by_a_client_without_the_keyboard_is_worthless() {
    let mut desk = desk().unwrap();
    let serial = clicked_serial(&mut desk).unwrap();

    let token = desk
        .background
        .request_activation_token(Some(serial))
        .unwrap();
    desk.background.activate(&token, desk.hidden).unwrap();
    desk.focused.roundtrip().unwrap();

    assert_eq!(desk.focused.keyboard_focus(), Some(desk.shown));
    assert_eq!(desk.background.keyboard_focus(), None);
}

#[test]
fn a_token_is_spent_by_its_first_use() {
    let mut desk = desk().unwrap();
    let serial = clicked_serial(&mut desk).unwrap();
    let token = desk.focused.request_activation_token(Some(serial)).unwrap();
    desk.background.activate(&token, desk.hidden).unwrap();
    desk.background
        .wait_until("the activated window holds the keyboard", |client| {
            client.keyboard_focus().is_some()
        })
        .unwrap();

    desk.focused.activate(&token, desk.shown).unwrap();
    desk.background.roundtrip().unwrap();

    assert_eq!(desk.background.keyboard_focus(), Some(desk.hidden));
    assert_eq!(desk.focused.keyboard_focus(), None);
}

#[test]
fn a_made_up_token_does_not_move_the_keyboard() {
    let mut desk = desk().unwrap();

    desk.background
        .activate("made-up-token", desk.hidden)
        .unwrap();
    desk.focused.roundtrip().unwrap();

    assert_eq!(desk.focused.keyboard_focus(), Some(desk.shown));
}
