use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use crate::credentials::effective_uid;

#[derive(Debug, thiserror::Error)]
pub enum SocketFileError {
    #[error("inspect {}", .path.display())]
    Inspect {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{} is a symbolic link", .0.display())]
    Symlink(PathBuf),
    #[error("{} is not a socket", .0.display())]
    NotSocket(PathBuf),
    #[error("{} belongs to uid {owner}, expected uid {expected}", .path.display())]
    ForeignOwner {
        path: PathBuf,
        owner: u32,
        expected: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupancy {
    Vacant,
    Stale,
    Listening,
}

pub(crate) fn occupancy(path: &Path) -> Result<Occupancy, SocketFileError> {
    let inspect = |source| SocketFileError::Inspect {
        path: path.to_owned(),
        source,
    };
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Occupancy::Vacant),
        Err(error) => return Err(inspect(error)),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(SocketFileError::Symlink(path.to_owned()));
    }
    if !file_type.is_socket() {
        return Err(SocketFileError::NotSocket(path.to_owned()));
    }
    let expected = effective_uid();
    let owner = metadata.uid();
    if owner != expected {
        return Err(SocketFileError::ForeignOwner {
            path: path.to_owned(),
            owner,
            expected,
        });
    }
    match UnixStream::connect(path) {
        Ok(_) => Ok(Occupancy::Listening),
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => Ok(Occupancy::Stale),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Occupancy::Vacant),
        Err(error) => Err(inspect(error)),
    }
}
