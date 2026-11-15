use std::fs::{self, Permissions};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::Path;

use tilling_ipc::endpoint::{DirectoryError, SocketFileError};
use tilling_ipc::server::{BindError, Listener};

use crate::support::{Peer, Scratch, accept, bind};

fn mode_of(path: &Path) -> u32 {
    let metadata = fs::symlink_metadata(path)
        .unwrap_or_else(|error| panic!("inspect {}: {error}", path.display()));
    metadata.permissions().mode() & 0o777
}

fn leave_stale_socket(scratch: &Scratch) {
    drop(bind(scratch));
    let abandoned = UnixListener::bind(scratch.socket())
        .unwrap_or_else(|error| panic!("bind abandoned socket: {error}"));
    drop(abandoned);
}

#[test]
fn bind_creates_a_private_directory_and_socket() {
    let scratch = Scratch::new();

    let listener = bind(&scratch);

    assert_eq!(listener.path(), scratch.socket());
    assert_eq!(mode_of(&scratch.root().join("tilling")), 0o700);
    assert_eq!(mode_of(&scratch.root().join("tilling/test")), 0o700);
    assert_eq!(mode_of(&scratch.socket()), 0o600);
}

#[test]
fn second_instance_is_refused_while_the_first_listens() {
    let scratch = Scratch::new();
    let _first = bind(&scratch);

    let second = Listener::bind(&scratch.socket());

    assert!(matches!(second, Err(BindError::InstanceAlive(path)) if path == scratch.socket()));
}

#[test]
fn stale_socket_is_replaced() {
    let scratch = Scratch::new();
    leave_stale_socket(&scratch);

    let listener = bind(&scratch);
    let _peer = Peer::connect(&scratch.socket());

    accept(&listener);
}

#[test]
fn symlink_at_the_socket_path_is_refused_and_kept() {
    let scratch = Scratch::new();
    drop(bind(&scratch));
    let target = scratch.root().join("elsewhere");
    fs::write(&target, b"keep").unwrap();
    symlink(&target, scratch.socket()).unwrap();

    let outcome = Listener::bind(&scratch.socket());

    assert!(matches!(
        outcome,
        Err(BindError::SocketFile(SocketFileError::Symlink(_)))
    ));
    assert_eq!(fs::read(&target).unwrap(), b"keep");
    assert!(
        fs::symlink_metadata(scratch.socket())
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn regular_file_at_the_socket_path_is_refused() {
    let scratch = Scratch::new();
    drop(bind(&scratch));
    fs::write(scratch.socket(), b"").unwrap();

    let outcome = Listener::bind(&scratch.socket());

    assert!(matches!(
        outcome,
        Err(BindError::SocketFile(SocketFileError::NotSocket(_)))
    ));
}

#[test]
fn directory_open_to_other_users_is_refused() {
    let scratch = Scratch::new();
    drop(bind(&scratch));
    let directory = scratch.root().join("tilling/test");
    fs::set_permissions(&directory, Permissions::from_mode(0o755)).unwrap();

    let outcome = Listener::bind(&scratch.socket());

    assert!(matches!(
        outcome,
        Err(BindError::Directory(DirectoryError::Exposed {
            mode: 0o755,
            ..
        }))
    ));
}

#[test]
fn dropping_the_listener_removes_its_own_socket() {
    let scratch = Scratch::new();

    drop(bind(&scratch));

    assert!(fs::symlink_metadata(scratch.socket()).is_err());
}

#[test]
fn dropping_the_listener_keeps_a_socket_that_replaced_it() {
    let scratch = Scratch::new();
    let replaced = bind(&scratch);
    fs::rename(scratch.socket(), scratch.root().join("moved-away")).unwrap();
    let _successor = UnixListener::bind(scratch.socket()).unwrap();

    drop(replaced);

    assert!(fs::symlink_metadata(scratch.socket()).is_ok());
}
