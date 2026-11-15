use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use smithay::input::keyboard::xkb;
use smithay::reexports::calloop::EventLoop;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::wayland_server::backend::ClientData;
use smithay::reexports::wayland_server::{Display, ListeningSocket};

struct UnprivilegedClient;

impl ClientData for UnprivilegedClient {}

#[test]
fn event_loop_dispatches_sources_into_the_single_owner_state() {
    let mut event_loop: EventLoop<'_, u32> = EventLoop::try_new().unwrap();
    event_loop
        .handle()
        .insert_source(Timer::immediate(), |_deadline, (), fired| {
            *fired += 1;
            TimeoutAction::Drop
        })
        .unwrap();
    let mut fired = 0;

    event_loop
        .dispatch(Duration::from_millis(500), &mut fired)
        .unwrap();

    assert_eq!(fired, 1);
}

#[test]
fn display_accepts_a_client_on_a_private_socket() {
    let directory = std::env::temp_dir().join(format!("tilling-wayland-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("wayland-contract");
    let socket = ListeningSocket::bind_absolute(path.clone()).unwrap();
    let mut display: Display<()> = Display::new().unwrap();

    let _client = UnixStream::connect(&path).unwrap();
    let connection = socket.accept().unwrap().unwrap();
    let inserted = display
        .handle()
        .insert_client(connection, Arc::new(UnprivilegedClient));

    assert!(inserted.is_ok());
    assert!(display.dispatch_clients(&mut ()).is_ok());
    assert!(display.flush_clients().is_ok());
    drop(socket);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn xkb_compiles_the_default_keymap() {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        "us",
        "",
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    );

    assert!(keymap.is_some());
}
