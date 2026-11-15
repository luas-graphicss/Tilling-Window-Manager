use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use tilling_config::CommandLine;

const RUNTIME_DIRECTORY_VARIABLE: &str = "XDG_RUNTIME_DIR";
const WAYLAND_DISPLAY_VARIABLE: &str = "WAYLAND_DISPLAY";

#[derive(Debug)]
pub(crate) struct Launcher {
    runtime_dir: PathBuf,
    wayland_display: OsString,
    children: Vec<Child>,
}

impl Launcher {
    pub(crate) fn new(runtime_dir: &Path, wayland_socket: &Path) -> Self {
        Self {
            runtime_dir: runtime_dir.to_path_buf(),
            wayland_display: wayland_socket
                .file_name()
                .map_or_else(|| wayland_socket.as_os_str().to_owned(), OsString::from),
            children: Vec::new(),
        }
    }

    pub(crate) fn launch(&mut self, command: &CommandLine) -> io::Result<()> {
        self.reap();
        let child = Command::new(command.program())
            .args(command.arguments())
            .env(RUNTIME_DIRECTORY_VARIABLE, &self.runtime_dir)
            .env(WAYLAND_DISPLAY_VARIABLE, &self.wayland_display)
            .stdin(Stdio::null())
            .spawn()?;
        self.children.push(child);
        Ok(())
    }

    pub(crate) fn reap(&mut self) {
        self.children
            .retain_mut(|child| matches!(child.try_wait(), Ok(None)));
    }
}
