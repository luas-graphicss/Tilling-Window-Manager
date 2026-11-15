use std::ffi::OsString;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use tilling_ipc::endpoint::{
    DirectoryError, InstanceName, RuntimeDirectory, RuntimeDirectoryError,
};
use tilling_ipc::server::Listener;

use crate::support::Scratch;

#[test]
fn runtime_directory_must_be_set_absolute_present_and_private() {
    let scratch = Scratch::new();
    let exposed = scratch.root().join("exposed");
    fs::create_dir(&exposed).unwrap();
    fs::set_permissions(&exposed, Permissions::from_mode(0o750)).unwrap();

    assert!(matches!(
        RuntimeDirectory::from_variable(None),
        Err(RuntimeDirectoryError::Unset)
    ));
    assert!(matches!(
        RuntimeDirectory::from_variable(Some(OsString::new())),
        Err(RuntimeDirectoryError::Unset)
    ));
    assert!(matches!(
        RuntimeDirectory::open(PathBuf::from("run/user")),
        Err(RuntimeDirectoryError::Relative(_))
    ));
    assert!(matches!(
        RuntimeDirectory::open(scratch.root().join("missing")),
        Err(RuntimeDirectoryError::Directory(
            DirectoryError::Inspect { .. }
        ))
    ));
    assert!(matches!(
        RuntimeDirectory::open(exposed),
        Err(RuntimeDirectoryError::Directory(DirectoryError::Exposed {
            mode: 0o750,
            ..
        }))
    ));
    assert!(RuntimeDirectory::from_variable(Some(scratch.root().into())).is_ok());
}

#[test]
fn only_listening_instances_are_discovered() {
    let scratch = Scratch::new();
    let runtime = RuntimeDirectory::open(scratch.root().to_owned()).unwrap();
    let alpha: InstanceName = "alpha".parse().unwrap();
    let beta: InstanceName = "beta".parse().unwrap();
    let stale: InstanceName = "stale".parse().unwrap();
    let before_any_instance = runtime.live_instances().unwrap();

    let _alpha = Listener::bind(&runtime.socket_path(&alpha)).unwrap();
    let _beta = Listener::bind(&runtime.socket_path(&beta)).unwrap();
    drop(Listener::bind(&runtime.socket_path(&stale)).unwrap());
    drop(UnixListener::bind(runtime.socket_path(&stale)).unwrap());
    fs::create_dir(scratch.root().join("tilling/not an instance")).unwrap();

    assert!(before_any_instance.is_empty());
    assert_eq!(runtime.live_instances().unwrap(), [alpha, beta]);
}
