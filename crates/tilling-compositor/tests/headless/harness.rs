use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tilling_compositor::{
    BackendKind, Capability, FrameStats, Options, Session, SessionSnapshot, SyntheticInput,
    VirtualOutput,
};
use tilling_config::{ConfigSource, ValidatedConfig};
use tilling_test_support::Client;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const CONTROL_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_LIMIT: Duration = Duration::from_secs(5);
const WAIT_STEP: Duration = Duration::from_millis(2);
const IDLE_WINDOW: Duration = Duration::from_millis(80);
const HELPER_RESTART_DELAY: Duration = Duration::from_millis(10);
const CONFIG_FILE: &str = "config.toml";

pub(crate) struct Harness {
    session: Session,
    runtime_dir: TempDir,
}

impl Harness {
    pub(crate) fn start() -> Self {
        Self::start_with(|_| {})
    }

    pub(crate) fn start_with(adjust: impl FnOnce(&mut Options)) -> Self {
        let config = tilling_config::load(&ConfigSource::BuiltIn)
            .unwrap_or_else(|error| panic!("built-in configuration was rejected: {error}"));
        Self::launch(config, adjust)
    }

    pub(crate) fn start_with_config(text: &str) -> Self {
        let config = tilling_config::parse(text)
            .unwrap_or_else(|error| panic!("test configuration was rejected: {error}"));
        Self::launch(config, |_| {})
    }

    pub(crate) fn start_from_file(text: &str) -> Self {
        let config = tilling_config::parse(text)
            .unwrap_or_else(|error| panic!("test configuration was rejected: {error}"));
        Self::launch(config, |options| {
            let path = options.runtime_dir.join(CONFIG_FILE);
            std::fs::write(&path, text)
                .unwrap_or_else(|error| panic!("test configuration was not written: {error}"));
            options.config_source = ConfigSource::File(path);
            options.helper_restart_delay = HELPER_RESTART_DELAY;
        })
    }

    pub(crate) fn rewrite_config(&self, text: &str) {
        std::fs::write(self.runtime_dir().join(CONFIG_FILE), text)
            .unwrap_or_else(|error| panic!("test configuration was not rewritten: {error}"));
    }

    fn launch(config: ValidatedConfig, adjust: impl FnOnce(&mut Options)) -> Self {
        let runtime_dir = tempfile::Builder::new()
            .permissions(Permissions::from_mode(PRIVATE_DIRECTORY_MODE))
            .tempdir()
            .unwrap_or_else(|error| panic!("runtime directory was not created: {error}"));
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
            runtime_dir,
        }
    }

    pub(crate) fn connect(&self) -> Client {
        Client::connect(self.session.wayland_socket())
            .unwrap_or_else(|error| panic!("test client did not connect: {error}"))
    }

    pub(crate) fn connect_with(&self, capabilities: &[Capability]) -> Client {
        let connection = self
            .session
            .connect_privileged(capabilities)
            .unwrap_or_else(|error| panic!("privileged connection was refused: {error}"));
        Client::from_stream(connection)
            .unwrap_or_else(|error| panic!("privileged client did not connect: {error}"))
    }

    pub(crate) fn control(&self) -> tilling_ipc::client::Client {
        tilling_ipc::client::Client::connect(self.session.control_socket(), CONTROL_TIMEOUT)
            .unwrap_or_else(|error| panic!("control client did not connect: {error}"))
    }

    pub(crate) fn wait_for_exit(&self) {
        let started = Instant::now();
        while self.session.snapshot().is_ok() {
            assert!(
                started.elapsed() < WAIT_LIMIT,
                "timed out waiting for the compositor to stop"
            );
            thread::sleep(WAIT_STEP);
        }
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

    pub(crate) fn idle_frames(&self) -> FrameStats {
        let started = Instant::now();
        let mut seen = self.snapshot().frames;
        loop {
            thread::sleep(IDLE_WINDOW);
            let current = self.snapshot().frames;
            if current == seen {
                return current;
            }
            assert!(
                started.elapsed() < WAIT_LIMIT,
                "timed out waiting for the compositor to stop presenting: {current:?}"
            );
            seen = current;
        }
    }

    pub(crate) fn published(&self, windows: usize) -> SessionSnapshot {
        self.wait_until("the scene shows the expected windows", |snapshot| {
            !snapshot.transaction_pending && snapshot.scene.windows().len() == windows
        })
    }

    pub(crate) fn chord(&self, codes: &[u32]) -> SessionSnapshot {
        for code in codes {
            self.inject(SyntheticInput::Key {
                code: *code,
                pressed: true,
            });
        }
        for code in codes.iter().rev() {
            self.inject(SyntheticInput::Key {
                code: *code,
                pressed: false,
            });
        }
        self.snapshot()
    }

    pub(crate) fn hold(&self, code: u32) -> SessionSnapshot {
        self.inject(SyntheticInput::Key {
            code,
            pressed: true,
        });
        self.snapshot()
    }

    pub(crate) fn lose_host_focus(&self) -> SessionSnapshot {
        self.inject(SyntheticInput::HostFocusLost);
        self.snapshot()
    }

    pub(crate) fn move_pointer(&self, x: f64, y: f64) -> SessionSnapshot {
        self.inject(SyntheticInput::PointerMotion { x, y });
        self.snapshot()
    }

    pub(crate) fn nudge_pointer(&self, dx: f64, dy: f64) -> SessionSnapshot {
        self.inject(SyntheticInput::PointerDelta { dx, dy });
        self.snapshot()
    }

    pub(crate) fn click(&self, code: u32) -> SessionSnapshot {
        self.press(code);
        self.release(code)
    }

    pub(crate) fn press(&self, code: u32) -> SessionSnapshot {
        self.inject(SyntheticInput::PointerButton {
            code,
            pressed: true,
        });
        self.snapshot()
    }

    pub(crate) fn release(&self, code: u32) -> SessionSnapshot {
        self.inject(SyntheticInput::PointerButton {
            code,
            pressed: false,
        });
        self.snapshot()
    }

    pub(crate) fn scroll(&self, horizontal: f64, vertical: f64) -> SessionSnapshot {
        self.inject(SyntheticInput::PointerScroll {
            horizontal,
            vertical,
        });
        self.snapshot()
    }

    pub(crate) fn runtime_dir(&self) -> &Path {
        self.runtime_dir.path()
    }

    pub(crate) fn session(&self) -> &Session {
        &self.session
    }

    pub(crate) fn plug(&self, output: VirtualOutput) {
        self.session
            .plug_output(output)
            .unwrap_or_else(|error| panic!("virtual output was not connected: {error}"));
    }

    pub(crate) fn unplug(&self, name: &str) {
        self.session
            .unplug_output(name)
            .unwrap_or_else(|error| panic!("virtual output was not disconnected: {error}"));
    }

    fn inject(&self, input: SyntheticInput) {
        self.session
            .inject(input)
            .unwrap_or_else(|error| panic!("compositor session refused input: {error}"));
    }
}
