use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process};
use tempfile::TempDir;
use tilling_ipc::client::Client;
use tilling_ipc::endpoint::RuntimeDirectory;

const INSTANCE: &str = "cli";
const START_LIMIT: Duration = Duration::from_secs(20);
const EXIT_LIMIT: Duration = Duration::from_secs(10);
const CONTROL_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub struct Instance {
    child: Child,
    runtime_dir: TempDir,
}

pub fn private_runtime_dir() -> TempDir {
    tempfile::Builder::new()
        .permissions(Permissions::from_mode(0o700))
        .tempdir()
        .unwrap_or_else(|error| panic!("runtime directory was not created: {error}"))
}

pub fn tilling(runtime_dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tilling"));
    command
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("HOME")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("DISPLAY")
        .stdin(Stdio::null());
    command
}

impl Instance {
    pub fn start_headless() -> Self {
        let runtime_dir = private_runtime_dir();
        let child = tilling(runtime_dir.path())
            .args(["--backend", "headless", "--instance", INSTANCE])
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|error| panic!("tilling did not start: {error}"));
        Self { child, runtime_dir }
    }

    pub fn wayland_socket(&self) -> PathBuf {
        self.runtime_dir.path().join(format!("tilling-{INSTANCE}"))
    }

    pub fn control_socket(&self) -> PathBuf {
        let directory = RuntimeDirectory::open(self.runtime_dir.path().to_path_buf())
            .unwrap_or_else(|error| panic!("runtime directory was refused: {error}"));
        let name = INSTANCE
            .parse()
            .unwrap_or_else(|error| panic!("instance name was refused: {error}"));
        directory.socket_path(&name)
    }

    pub fn control(&self) -> Client {
        let socket = self.control_socket();
        let started = Instant::now();
        loop {
            match Client::connect(&socket, CONTROL_TIMEOUT) {
                Ok(client) => return client,
                Err(error) => assert!(
                    started.elapsed() < START_LIMIT,
                    "control socket never answered: {error}"
                ),
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    pub fn terminate(&self) {
        let pid = i32::try_from(self.child.id())
            .ok()
            .and_then(Pid::from_raw)
            .unwrap_or_else(|| panic!("child has no usable process id"));
        kill_process(pid, Signal::TERM)
            .unwrap_or_else(|error| panic!("termination signal was not delivered: {error}"));
    }

    pub fn wait_for_exit(&mut self) -> ExitStatus {
        let started = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) => assert!(
                    started.elapsed() < EXIT_LIMIT,
                    "tilling kept running after it was asked to stop"
                ),
                Err(error) => panic!("tilling could not be awaited: {error}"),
            }
            thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
