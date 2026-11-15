use tilling_compositor::SceneLayer;
use tilling_ipc::{Call, Outcome, Reply};

use crate::harness::Harness;
use crate::keys;

#[test]
fn scratchpad_shortcuts_hide_the_focused_window_and_bring_it_back_floating() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let editor = client.create_toplevel("org.example.Editor", "notes");
    let terminal = client.create_toplevel("org.example.Terminal", "shell");
    client.map(editor).unwrap();
    client.map(terminal).unwrap();
    client.settle().unwrap();
    harness.published(2);
    let mut after = |codes: &[u32], visible: usize| {
        harness.chord(codes);
        client.settle().unwrap();
        let snapshot = harness.published(visible);
        client.roundtrip().unwrap();
        (snapshot, client.keyboard_focus())
    };

    let (hidden, focus_while_hidden) = after(&[keys::SUPER, keys::SHIFT, keys::MINUS], 1);
    let (shown, focus_while_shown) = after(&[keys::SUPER, keys::MINUS], 2);
    let (hidden_again, _) = after(&[keys::SUPER, keys::MINUS], 1);

    let [tile, dropdown] = shown.scene.windows() else {
        panic!("expected the tile and the summoned window: {shown:?}");
    };
    assert_eq!(focus_while_hidden, Some(editor));
    assert_eq!(focus_while_shown, Some(terminal));
    assert_eq!(hidden.mapped_windows, 2);
    assert_eq!(tile.layer, SceneLayer::Tiled);
    assert_eq!(dropdown.layer, SceneLayer::Floating);
    assert!(dropdown.focused);
    assert_eq!(hidden_again.scene, hidden.scene);
}

#[test]
fn stashed_window_is_reported_and_cannot_be_focused_over_the_socket() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();
    harness.published(1);
    let mut control = harness.control();

    harness.chord(&[keys::SUPER, keys::SHIFT, keys::MINUS]);
    harness.published(0);
    let listed = control.call(Call::GetWindows, None).unwrap();
    let Outcome::Success {
        result: Reply::Windows(windows),
    } = listed
    else {
        panic!("unexpected reply to get windows: {listed:?}");
    };
    let focus = control.call(
        Call::FocusWindow {
            window: windows[0].id,
        },
        None,
    );

    assert!(windows[0].stashed);
    assert!(matches!(focus.unwrap(), Outcome::ValidationError { .. }));
}
