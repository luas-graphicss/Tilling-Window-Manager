use tilling_compositor::SessionSnapshot;
use tilling_test_support::{Client, ClientError, ToplevelId};

use crate::harness::Harness;
use crate::keys;

const TEXT: &str = "text/plain;charset=utf-8";

fn window(
    harness: &Harness,
    title: &str,
    expected_windows: usize,
) -> Result<(Client, ToplevelId), ClientError> {
    let mut client = harness.connect();
    client.watch_selections()?;
    let toplevel = client.create_toplevel("org.example.Editor", title);
    client.map(toplevel)?;
    client.settle()?;
    harness.published(expected_windows);
    Ok((client, toplevel))
}

fn center_of(snapshot: &SessionSnapshot, index: usize) -> (f64, f64) {
    let frame = snapshot.scene.windows()[index].frame;
    let x = f64::from(frame.x()) + f64::from(frame.width()) / 2.0;
    let y = f64::from(frame.y()) + f64::from(frame.height()) / 2.0;
    (x, y)
}

#[test]
fn clipboard_content_travels_to_the_client_that_takes_the_keyboard() {
    let harness = Harness::start();
    let (mut source, _) = window(&harness, "notes", 1).unwrap();
    source.offer_clipboard(TEXT, b"copied text").unwrap();

    let (mut target, _) = window(&harness, "letter", 2).unwrap();
    target
        .wait_until("the clipboard is offered", |target| {
            target.clipboard_mime_types().is_some()
        })
        .unwrap();
    let transfer = target.request_clipboard(TEXT).unwrap();
    source.roundtrip().unwrap();
    let received = transfer.finish().unwrap();

    assert_eq!(target.clipboard_mime_types().unwrap(), [TEXT]);
    assert_eq!(received, b"copied text");
}

#[test]
fn the_clipboard_empties_when_its_owner_disconnects() {
    let harness = Harness::start();
    let (mut source, _) = window(&harness, "notes", 1).unwrap();
    source.offer_clipboard(TEXT, b"copied text").unwrap();
    let (mut target, _) = window(&harness, "letter", 2).unwrap();
    target
        .wait_until("the clipboard is offered", |target| {
            target.clipboard_mime_types().is_some()
        })
        .unwrap();

    drop(source);
    target
        .wait_until("the offer is withdrawn", |target| {
            target.clipboard_mime_types().is_none()
        })
        .unwrap();

    assert!(matches!(
        target.request_clipboard(TEXT),
        Err(ClientError::NoClipboardOffer)
    ));
}

#[test]
fn a_client_without_the_keyboard_cannot_replace_the_clipboard() {
    let harness = Harness::start();
    let (mut background, _) = window(&harness, "notes", 1).unwrap();
    let (mut focused, _) = window(&harness, "letter", 2).unwrap();

    background.offer_clipboard(TEXT, b"injected").unwrap();
    focused.roundtrip().unwrap();

    assert_eq!(focused.clipboard_mime_types(), None);
}

#[test]
fn the_primary_selection_is_offered_to_the_client_that_takes_the_keyboard() {
    let harness = Harness::start();
    let (mut source, _) = window(&harness, "notes", 1).unwrap();
    source.offer_primary_selection(TEXT).unwrap();

    let (mut target, _) = window(&harness, "letter", 2).unwrap();
    target
        .wait_until("the primary selection is offered", |target| {
            target.primary_selection_mime_types().is_some()
        })
        .unwrap();

    assert_eq!(target.primary_selection_mime_types().unwrap(), [TEXT]);
}

#[test]
fn a_drag_is_offered_to_the_window_under_the_pointer_and_cancelled_when_refused() {
    let harness = Harness::start();
    let (mut source, origin) = window(&harness, "notes", 1).unwrap();
    let (mut target, _) = window(&harness, "letter", 2).unwrap();
    source.settle().unwrap();
    let snapshot = harness.published(2);
    let (from, onto) = (center_of(&snapshot, 0), center_of(&snapshot, 1));

    harness.move_pointer(from.0, from.1);
    harness.press(keys::BUTTON_LEFT);
    source.roundtrip().unwrap();
    source.start_drag(origin, TEXT, b"dragged text").unwrap();
    harness.move_pointer(onto.0, onto.1);
    target
        .wait_until("the drag enters the window", |target| {
            target.dragged_mime_types().is_some()
        })
        .unwrap();
    let offered = target.dragged_mime_types().map(<[String]>::to_vec);
    harness.release(keys::BUTTON_LEFT);
    source
        .wait_until("the refused drag is cancelled", |source| {
            source.cancelled_sources() == 1
        })
        .unwrap();
    target.roundtrip().unwrap();

    assert_eq!(offered.unwrap(), [TEXT]);
    assert_eq!(target.dragged_mime_types(), None);
}
