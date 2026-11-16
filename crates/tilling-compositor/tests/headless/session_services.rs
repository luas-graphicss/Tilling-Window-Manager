use std::fs;
use std::path::Path;

use tempfile::TempDir;
use tilling_compositor::SessionServices;

use crate::harness::Harness;

const TARGET: &str = "tilling-session.target";
const SHARED: &str = "WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE TILLING_SOCKET";

fn recording_manager(directory: &Path) -> SessionServices {
    let script = directory.join("manager");
    let log = directory.join("requests");
    let text = format!(
        "echo \"$* from $WAYLAND_DISPLAY on $XDG_CURRENT_DESKTOP\" >> '{}'\n",
        log.display()
    );
    fs::write(&script, text)
        .unwrap_or_else(|error| panic!("service manager was not written: {error}"));
    SessionServices {
        manager: vec!["sh".to_owned(), script.display().to_string()],
        target: TARGET.to_owned(),
    }
}

fn requests(directory: &Path) -> Vec<String> {
    fs::read_to_string(directory.join("requests"))
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[test]
fn the_session_is_announced_to_its_services_and_withdrawn_when_it_ends() {
    let records = TempDir::new().unwrap();
    let services = recording_manager(records.path());
    let origin = "from tilling-test on tilling";

    let harness = Harness::start_with(|options| options.session_services = Some(services));
    let while_running = requests(records.path());
    drop(harness);
    let after_the_end = requests(records.path());

    assert_eq!(
        while_running,
        [
            format!("--user import-environment {SHARED} {origin}"),
            format!("--user start --no-block {TARGET} {origin}"),
        ]
    );
    assert_eq!(
        after_the_end[2..],
        [
            format!("--user stop --no-block {TARGET} {origin}"),
            format!("--user unset-environment {SHARED} {origin}"),
        ]
    );
}

#[test]
fn a_service_manager_that_is_missing_does_not_stop_the_session() {
    let services = SessionServices {
        manager: vec!["/nonexistent/tilling-service-manager".to_owned()],
        target: TARGET.to_owned(),
    };

    let harness = Harness::start_with(|options| options.session_services = Some(services));
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.map(window).unwrap();

    assert_eq!(harness.published(1).scene.windows().len(), 1);
}
