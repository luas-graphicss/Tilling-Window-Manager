use std::ffi::OsString;
use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt};
use std::path::{Path, PathBuf};
use std::process::Command;

const PRIVATE_MODE: u32 = 0o700;
const OUTER_SESSION_VARIABLES: [&str; 4] = [
    "TILLING_SOCKET",
    "WAYLAND_DISPLAY",
    "WAYLAND_SOCKET",
    "DISPLAY",
];

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("create private runtime directory {}", .path.display())]
    Create {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("inspect {}", .path.display())]
    Inspect {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn create(runtime_directory: Option<OsString>, label: &str) -> Result<Self, SandboxError> {
        let parent = runtime_directory
            .map(PathBuf::from)
            .filter(|directory| directory.is_absolute() && directory.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        let root = parent.join(format!("tilling-{label}-{}", std::process::id()));
        DirBuilder::new()
            .mode(PRIVATE_MODE)
            .create(&root)
            .map_err(|source| SandboxError::Create {
                path: root.clone(),
                source,
            })?;
        Ok(Self { root })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn confine(&self, command: &mut Command) {
        command
            .env("XDG_RUNTIME_DIR", &self.root)
            .env("TMPDIR", &self.root);
        for variable in OUTER_SESSION_VARIABLES {
            command.env_remove(variable);
        }
    }

    pub fn leftover_sockets(&self) -> Result<Vec<PathBuf>, SandboxError> {
        let mut sockets = Vec::new();
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            let inspect = |source| SandboxError::Inspect {
                path: directory.clone(),
                source,
            };
            for entry in fs::read_dir(&directory).map_err(inspect)? {
                let entry = entry.map_err(inspect)?;
                let kind = entry.file_type().map_err(inspect)?;
                if kind.is_dir() {
                    pending.push(entry.path());
                } else if kind.is_socket() {
                    sockets.push(entry.path());
                }
            }
        }
        sockets.sort();
        Ok(sockets)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!("xtask: remove {}: {error}", self.root.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    use super::*;

    fn sandbox(label: &str) -> Sandbox {
        Sandbox::create(None, label).unwrap()
    }

    #[test]
    fn directory_is_private_and_removed_when_the_run_ends() {
        let created = sandbox("private");
        let root = created.path().to_path_buf();
        let mode = fs::metadata(&root).unwrap().permissions().mode() & 0o777;

        drop(created);

        assert_eq!(mode, PRIVATE_MODE);
        assert!(!root.exists());
    }

    #[test]
    fn child_only_sees_the_private_directory_and_no_outer_display() {
        let created = sandbox("confined");
        let mut command = Command::new("true");
        command.env("WAYLAND_DISPLAY", "wayland-0");

        created.confine(&mut command);

        let changed: Vec<_> = command.get_envs().collect();
        let private = Some(created.path().as_os_str());
        assert!(changed.contains(&("XDG_RUNTIME_DIR".as_ref(), private)));
        assert!(changed.contains(&("TMPDIR".as_ref(), private)));
        assert!(changed.contains(&("WAYLAND_DISPLAY".as_ref(), None)));
        assert!(changed.contains(&("TILLING_SOCKET".as_ref(), None)));
    }

    #[test]
    fn sockets_left_in_nested_directories_are_found() {
        let created = sandbox("leftovers");
        let nested = created.path().join("tilling/test");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("notes.txt"), b"").unwrap();
        let listener = UnixListener::bind(nested.join("ipc.sock")).unwrap();

        let found = created.leftover_sockets().unwrap();

        drop(listener);
        assert_eq!(found, [nested.join("ipc.sock")]);
    }
}
