use std::fs::{self, DirBuilder, Permissions};
use std::io;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use super::connection::Connection;
use crate::credentials::{PeerError, require_same_user};
use crate::endpoint::{DirectoryError, Occupancy, SocketFileError, occupancy, require_private};

const DIRECTORY_MODE: u32 = 0o700;
const SOCKET_MODE: u32 = 0o600;

#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("socket path {} has no parent directory", .0.display())]
    NoDirectory(PathBuf),
    #[error("create socket directory {}", .path.display())]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Directory(#[from] DirectoryError),
    #[error(transparent)]
    SocketFile(#[from] SocketFileError),
    #[error("another instance is listening on {}", .0.display())]
    InstanceAlive(PathBuf),
    #[error("remove stale socket {}", .path.display())]
    RemoveStale {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("bind socket {}", .path.display())]
    Socket {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum AcceptError {
    #[error(transparent)]
    Peer(#[from] PeerError),
    #[error("accept connection")]
    Socket(#[from] io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

impl FileIdentity {
    fn of(path: &Path) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
}

#[derive(Debug)]
pub struct Listener {
    socket: UnixListener,
    path: PathBuf,
    identity: FileIdentity,
}

impl Listener {
    pub fn bind(path: &Path) -> Result<Self, BindError> {
        prepare_directory(path)?;
        match occupancy(path)? {
            Occupancy::Vacant => {}
            Occupancy::Stale => {
                fs::remove_file(path).map_err(|source| BindError::RemoveStale {
                    path: path.to_owned(),
                    source,
                })?;
            }
            Occupancy::Listening => return Err(BindError::InstanceAlive(path.to_owned())),
        }
        let (socket, identity) = open_socket(path).map_err(|source| BindError::Socket {
            path: path.to_owned(),
            source,
        })?;
        Ok(Self {
            socket,
            path: path.to_owned(),
            identity,
        })
    }

    pub fn accept(&self) -> Result<Option<Connection>, AcceptError> {
        let stream = match self.socket.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        require_same_user(&stream)?;
        stream.set_nonblocking(true)?;
        Ok(Some(Connection::new(stream)))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AsFd for Listener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let still_ours = FileIdentity::of(&self.path).is_ok_and(|found| found == self.identity);
        if still_ours {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn prepare_directory(socket: &Path) -> Result<(), BindError> {
    let directory = socket
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| BindError::NoDirectory(socket.to_owned()))?;
    DirBuilder::new()
        .recursive(true)
        .mode(DIRECTORY_MODE)
        .create(directory)
        .map_err(|source| BindError::CreateDirectory {
            path: directory.to_owned(),
            source,
        })?;
    let metadata = fs::symlink_metadata(directory).map_err(|source| DirectoryError::Inspect {
        path: directory.to_owned(),
        source,
    })?;
    Ok(require_private(directory, &metadata)?)
}

fn open_socket(path: &Path) -> io::Result<(UnixListener, FileIdentity)> {
    let socket = UnixListener::bind(path)?;
    fs::set_permissions(path, Permissions::from_mode(SOCKET_MODE))?;
    socket.set_nonblocking(true)?;
    Ok((socket, FileIdentity::of(path)?))
}
