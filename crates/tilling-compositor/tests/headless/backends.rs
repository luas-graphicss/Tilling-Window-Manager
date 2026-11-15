use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use tilling_compositor::{BackendKind, CompositorError, Options, Session};
use tilling_config::ConfigSource;

#[test]
fn nested_backend_is_refused_away_from_the_main_thread() {
    let runtime_dir = tempfile::Builder::new()
        .permissions(Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = tilling_config::load(&ConfigSource::BuiltIn).unwrap();
    let options = Options::new(
        BackendKind::Nested,
        config,
        ConfigSource::BuiltIn,
        runtime_dir.path().to_path_buf(),
    );

    let outcome = Session::start(options);

    assert!(matches!(
        outcome,
        Err(CompositorError::MainThreadBackend("nested"))
    ));
    assert_eq!(std::fs::read_dir(runtime_dir.path()).unwrap().count(), 0);
}
