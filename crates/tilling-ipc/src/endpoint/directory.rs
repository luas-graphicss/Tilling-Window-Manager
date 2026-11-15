use std::fs::Metadata;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::credentials::effective_uid;

const PERMISSION_BITS: u32 = 0o777;
const OTHER_USERS_BITS: u32 = 0o077;

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("inspect {}", .path.display())]
    Inspect {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{} is not a directory", .0.display())]
    NotDirectory(PathBuf),
    #[error("{} belongs to uid {owner}, expected uid {expected}", .path.display())]
    ForeignOwner {
        path: PathBuf,
        owner: u32,
        expected: u32,
    },
    #[error("{} is accessible by other users, its mode is {mode:o}", .path.display())]
    Exposed { path: PathBuf, mode: u32 },
}

pub(crate) fn require_private(path: &Path, metadata: &Metadata) -> Result<(), DirectoryError> {
    if !metadata.is_dir() {
        return Err(DirectoryError::NotDirectory(path.to_owned()));
    }
    let expected = effective_uid();
    let owner = metadata.uid();
    if owner != expected {
        return Err(DirectoryError::ForeignOwner {
            path: path.to_owned(),
            owner,
            expected,
        });
    }
    let mode = metadata.mode() & PERMISSION_BITS;
    if mode & OTHER_USERS_BITS != 0 {
        return Err(DirectoryError::Exposed {
            path: path.to_owned(),
            mode,
        });
    }
    Ok(())
}
