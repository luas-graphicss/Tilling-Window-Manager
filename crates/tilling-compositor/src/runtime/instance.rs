use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

const MAX_NAME_BYTES: usize = 32;
const SHARED_ACCESS_BITS: u32 = 0o077;
const WAYLAND_SOCKET_PREFIX: &str = "tilling-";

#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error("instance name `{0}` must use 1 to {MAX_NAME_BYTES} letters, digits, `-` or `_`")]
    InvalidName(String),
    #[error("runtime directory {} is not an absolute path", .0.display())]
    RelativeRuntimeDirectory(PathBuf),
    #[error("inspect runtime directory {}", .path.display())]
    Inspect {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("runtime directory {} is not a directory", .0.display())]
    NotADirectory(PathBuf),
    #[error("runtime directory {} belongs to another user", .0.display())]
    ForeignOwner(PathBuf),
    #[error("runtime directory {} is accessible by other users", .0.display())]
    SharedAccess(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Instance {
    wayland_socket: PathBuf,
}

impl Instance {
    pub(crate) fn resolve(runtime_dir: &Path, name: &str) -> Result<Self, InstanceError> {
        require_name(name)?;
        require_private_directory(runtime_dir)?;
        Ok(Self {
            wayland_socket: runtime_dir.join(format!("{WAYLAND_SOCKET_PREFIX}{name}")),
        })
    }

    pub(crate) fn wayland_socket(&self) -> &Path {
        &self.wayland_socket
    }
}

fn require_name(name: &str) -> Result<(), InstanceError> {
    let allowed = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_');
    if name.is_empty() || name.len() > MAX_NAME_BYTES || !name.bytes().all(allowed) {
        return Err(InstanceError::InvalidName(name.to_owned()));
    }
    Ok(())
}

fn require_private_directory(path: &Path) -> Result<(), InstanceError> {
    if !path.is_absolute() {
        return Err(InstanceError::RelativeRuntimeDirectory(path.to_path_buf()));
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|source| InstanceError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(InstanceError::NotADirectory(path.to_path_buf()));
    }
    if metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(InstanceError::ForeignOwner(path.to_path_buf()));
    }
    if metadata.permissions().mode() & SHARED_ACCESS_BITS != 0 {
        return Err(InstanceError::SharedAccess(path.to_path_buf()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::Permissions;

    use tempfile::TempDir;

    use super::*;

    fn directory_with_mode(mode: u32) -> TempDir {
        tempfile::Builder::new()
            .permissions(Permissions::from_mode(mode))
            .tempdir()
            .unwrap()
    }

    #[test]
    fn wayland_socket_is_named_after_the_instance() {
        let directory = directory_with_mode(0o700);

        let instance = Instance::resolve(directory.path(), "nested-1").unwrap();

        assert_eq!(
            instance.wayland_socket(),
            directory.path().join("tilling-nested-1")
        );
    }

    #[test]
    fn names_that_could_leave_the_runtime_directory_are_refused() {
        let directory = directory_with_mode(0o700);

        for name in ["", "..", "a/b", "name with space", &"x".repeat(33)] {
            assert!(matches!(
                Instance::resolve(directory.path(), name),
                Err(InstanceError::InvalidName(_))
            ));
        }
    }

    #[test]
    fn directory_open_to_other_users_is_refused() {
        for mode in [0o750, 0o705, 0o777] {
            let directory = directory_with_mode(mode);

            let outcome = Instance::resolve(directory.path(), "main");

            assert!(
                matches!(outcome, Err(InstanceError::SharedAccess(_))),
                "mode {mode:o}"
            );
        }
    }

    #[test]
    fn relative_missing_and_non_directory_paths_are_refused() {
        let directory = directory_with_mode(0o700);
        let file = directory.path().join("file");
        std::fs::write(&file, b"").unwrap();

        assert!(matches!(
            Instance::resolve(Path::new("run/user"), "main"),
            Err(InstanceError::RelativeRuntimeDirectory(_))
        ));
        assert!(matches!(
            Instance::resolve(&directory.path().join("missing"), "main"),
            Err(InstanceError::Inspect { .. })
        ));
        assert!(matches!(
            Instance::resolve(&file, "main"),
            Err(InstanceError::NotADirectory(_))
        ));
    }
}
