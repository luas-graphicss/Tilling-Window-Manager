use std::ffi::OsString;
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

use tilling_ipc::endpoint::SOCKET_VARIABLE;

const RUNTIME_DIRECTORY: &str = "XDG_RUNTIME_DIR";
const WAYLAND_DISPLAY: &str = "WAYLAND_DISPLAY";
const PRIVATE_CONNECTION: &str = "WAYLAND_SOCKET";
const CURRENT_DESKTOP: &str = "XDG_CURRENT_DESKTOP";
const SESSION_TYPE: &str = "XDG_SESSION_TYPE";
const DESKTOP_NAME: &str = "tilling";
const WAYLAND_SESSION: &str = "wayland";

#[derive(Debug, Clone)]
pub(crate) struct SessionEnvironment {
    runtime_dir: PathBuf,
    wayland_display: OsString,
    control_socket: PathBuf,
}

impl SessionEnvironment {
    pub(crate) const SHARED: [&'static str; 4] = [
        WAYLAND_DISPLAY,
        CURRENT_DESKTOP,
        SESSION_TYPE,
        SOCKET_VARIABLE,
    ];

    pub(crate) fn new(runtime_dir: &Path, wayland_socket: &Path, control_socket: &Path) -> Self {
        Self {
            runtime_dir: runtime_dir.to_path_buf(),
            wayland_display: wayland_socket
                .file_name()
                .map_or_else(|| wayland_socket.as_os_str().to_owned(), OsString::from),
            control_socket: control_socket.to_path_buf(),
        }
    }

    pub(crate) fn apply(&self, command: &mut Command) {
        command
            .env(RUNTIME_DIRECTORY, &self.runtime_dir)
            .env(WAYLAND_DISPLAY, &self.wayland_display)
            .env(SOCKET_VARIABLE, &self.control_socket)
            .env(CURRENT_DESKTOP, DESKTOP_NAME)
            .env(SESSION_TYPE, WAYLAND_SESSION)
            .env_remove(PRIVATE_CONNECTION);
    }

    pub(crate) fn offer_private_connection(command: &mut Command, descriptor: RawFd) {
        command.env(PRIVATE_CONNECTION, descriptor.to_string());
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn variable<'a>(command: &'a Command, name: &str) -> Option<Option<&'a OsStr>> {
        command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new(name))
            .map(|(_, value)| value)
    }

    #[test]
    fn programs_are_pointed_at_this_instance_and_not_at_an_inherited_connection() {
        let environment = SessionEnvironment::new(
            Path::new("/run/user/1000"),
            Path::new("/run/user/1000/tilling-main"),
            Path::new("/run/user/1000/tilling/main/ipc.sock"),
        );
        let mut command = Command::new("true");

        environment.apply(&mut command);

        assert_eq!(
            variable(&command, "XDG_RUNTIME_DIR"),
            Some(Some(OsStr::new("/run/user/1000")))
        );
        assert_eq!(
            variable(&command, "WAYLAND_DISPLAY"),
            Some(Some(OsStr::new("tilling-main")))
        );
        assert_eq!(
            variable(&command, "TILLING_SOCKET"),
            Some(Some(OsStr::new("/run/user/1000/tilling/main/ipc.sock")))
        );
        assert_eq!(
            variable(&command, "XDG_CURRENT_DESKTOP"),
            Some(Some(OsStr::new("tilling")))
        );
        assert_eq!(variable(&command, "WAYLAND_SOCKET"), Some(None));
    }

    #[test]
    fn private_connection_is_named_by_its_descriptor() {
        let mut command = Command::new("true");

        SessionEnvironment::offer_private_connection(&mut command, 7);

        assert_eq!(
            variable(&command, "WAYLAND_SOCKET"),
            Some(Some(OsStr::new("7")))
        );
    }
}
