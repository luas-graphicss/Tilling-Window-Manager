use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tilling_compositor::{BackendKind, Options, Session, SessionSnapshot};
use tilling_config::ConfigSource;
use tilling_test_support::Client;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const WAIT_LIMIT: Duration = Duration::from_secs(5);
const WAIT_STEP: Duration = Duration::from_millis(2);

pub(crate) struct Harness {
    session: Session,
    _runtime_dir: TempDir,
}

impl Harness {
    pub(crate) fn start() -> Self {
        Self::start_with(|_| {})
    }

    pub(crate) fn start_with(adjust: impl FnOnce(&mut Options)) -> Self {
        let runtime_dir = tempfile::Builder::new()
            .permissions(Permissions::from_mode(PRIVATE_DIRECTORY_MODE))
            .tempdir()
            .unwrap_or_else(|error| panic!("runtime directory was not created: {error}"));
        let config = tilling_config::load(&ConfigSource::BuiltIn)
            .unwrap_or_else(|error| panic!("built-in configuration was rejected: {error}"));
        let mut options = Options::new(
            BackendKind::Headless,
            config,
            ConfigSource::BuiltIn,
            runtime_dir.path().to_path_buf(),
        );
        options.instance = "test".to_owned();
        adjust(&mut options);
        let session = Session::start(options)
            .unwrap_or_else(|error| panic!("compositor session did not start: {error}"));
        Self {
            session,
            _runtime_dir: runtime_dir,
        }
    }

    pub(crate) fn connect(&self) -> Client {
        Client::connect(self.session.wayland_socket())
            .unwrap_or_else(|error| panic!("test client did not connect: {error}"))
    }

    pub(crate) fn snapshot(&self) -> SessionSnapshot {
        self.session
            .snapshot()
            .unwrap_or_else(|error| panic!("compositor session stopped answering: {error}"))
    }

    pub(crate) fn wait_until(
        &self,
        expectation: &str,
        satisfied: impl Fn(&SessionSnapshot) -> bool,
    ) -> SessionSnapshot {
        let started = Instant::now();
        loop {
            let snapshot = self.snapshot();
            if satisfied(&snapshot) {
                return snapshot;
            }
            assert!(
                started.elapsed() < WAIT_LIMIT,
                "timed out waiting until {expectation}: {snapshot:?}"
            );
            thread::sleep(WAIT_STEP);
        }
    }

    pub(crate) fn published(&self, windows: usize) -> SessionSnapshot {
        self.wait_until("the scene shows the expected windows", |snapshot| {
            !snapshot.transaction_pending && snapshot.scene.windows().len() == windows
        })
    }
}
