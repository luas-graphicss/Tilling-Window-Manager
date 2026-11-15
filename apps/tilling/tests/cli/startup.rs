use tilling_ipc::{Call, Outcome, Reply};

use crate::support::{Instance, private_runtime_dir, tilling};

#[test]
fn headless_instance_answers_its_control_socket_and_stops_on_request() {
    let mut instance = Instance::start_headless();
    let mut control = instance.control();

    let version = control.call(Call::GetVersion, None).unwrap();
    let quit = control.call(Call::Quit, None).unwrap();
    let status = instance.wait_for_exit();

    let Outcome::Success {
        result: Reply::Version(version),
    } = version
    else {
        panic!("unexpected reply to get version: {version:?}");
    };
    assert_eq!(version.compositor, env!("CARGO_PKG_VERSION"));
    assert!(matches!(quit, Outcome::Success { .. }));
    assert!(status.success());
    assert!(!instance.control_socket().exists());
}

#[test]
fn termination_signal_stops_the_instance_and_removes_its_sockets() {
    let mut instance = Instance::start_headless();
    let control = instance.control();
    let sockets_while_running = (
        instance.wayland_socket().exists(),
        instance.control_socket().exists(),
    );

    drop(control);
    instance.terminate();
    let status = instance.wait_for_exit();

    assert_eq!(sockets_while_running, (true, true));
    assert!(status.success());
    assert!(!instance.wayland_socket().exists());
    assert!(!instance.control_socket().exists());
}

#[test]
fn missing_runtime_directory_is_reported_without_starting() {
    let runtime_dir = private_runtime_dir();

    let output = tilling(runtime_dir.path())
        .env_remove("XDG_RUNTIME_DIR")
        .args(["--backend", "headless"])
        .output()
        .unwrap();

    let reported = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(reported.contains("tilling: XDG_RUNTIME_DIR is not set"));
}

#[test]
fn nested_backend_without_a_host_display_fails_with_the_reason() {
    let runtime_dir = private_runtime_dir();

    let output = tilling(runtime_dir.path())
        .args(["--backend", "nested", "--instance", "nested"])
        .output()
        .unwrap();

    let reported = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        reported.contains("tilling: open the host window"),
        "unexpected report: {reported}"
    );
    assert!(!runtime_dir.path().join("tilling-nested").exists());
}
