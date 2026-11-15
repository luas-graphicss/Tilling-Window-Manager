use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::directory::{DirectoryError, require_private};
use super::instance::InstanceName;
use super::socket_file::{Occupancy, SocketFileError, occupancy};

const NAMESPACE: &str = "tilling";
const SOCKET_FILE: &str = "ipc.sock";

#[derive(Debug, thiserror::Error)]
pub enum RuntimeDirectoryError {
    #[error("XDG_RUNTIME_DIR is not set")]
    Unset,
    #[error("runtime directory {} is not an absolute path", .0.display())]
    Relative(PathBuf),
    #[error(transparent)]
    Directory(#[from] DirectoryError),
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("list instances in {}", .path.display())]
    List {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    SocketFile(#[from] SocketFileError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDirectory(PathBuf);

impl RuntimeDirectory {
    pub fn from_variable(value: Option<OsString>) -> Result<Self, RuntimeDirectoryError> {
        let path = value
            .filter(|value| !value.is_empty())
            .ok_or(RuntimeDirectoryError::Unset)?;
        Self::open(PathBuf::from(path))
    }

    pub fn open(path: PathBuf) -> Result<Self, RuntimeDirectoryError> {
        if !path.is_absolute() {
            return Err(RuntimeDirectoryError::Relative(path));
        }
        let metadata = fs::metadata(&path).map_err(|source| DirectoryError::Inspect {
            path: path.clone(),
            source,
        })?;
        require_private(&path, &metadata)?;
        Ok(Self(path))
    }

    #[must_use]
    pub fn socket_path(&self, instance: &InstanceName) -> PathBuf {
        self.0
            .join(NAMESPACE)
            .join(instance.as_str())
            .join(SOCKET_FILE)
    }

    pub fn live_instances(&self) -> Result<Vec<InstanceName>, DiscoveryError> {
        let root = self.0.join(NAMESPACE);
        let list = |source| DiscoveryError::List {
            path: root.clone(),
            source,
        };
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(list(error)),
        };
        let mut instances = Vec::new();
        for entry in entries {
            let name = entry.map_err(list)?.file_name();
            let Some(instance) = name.to_str().and_then(|name| name.parse().ok()) else {
                continue;
            };
            if occupancy(&self.socket_path(&instance))? == Occupancy::Listening {
                instances.push(instance);
            }
        }
        instances.sort();
        Ok(instances)
    }
}
