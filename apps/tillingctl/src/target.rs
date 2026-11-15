use std::ffi::OsString;
use std::path::PathBuf;

use tilling_ipc::endpoint::{
    DiscoveryError, InstanceName, RuntimeDirectory, RuntimeDirectoryError, SOCKET_VARIABLE,
};

const RUNTIME_DIRECTORY_VARIABLE: &str = "XDG_RUNTIME_DIR";

#[derive(Debug, thiserror::Error)]
pub enum TargetError {
    #[error(transparent)]
    RuntimeDirectory(#[from] RuntimeDirectoryError),
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error("no instance is running, start tilling or pass --socket")]
    NoInstance,
    #[error("several instances are running ({}), choose one with --instance", list(.0))]
    Ambiguous(Vec<InstanceName>),
}

#[derive(Debug, Default)]
pub struct Environment {
    pub socket: Option<OsString>,
    pub runtime_directory: Option<OsString>,
}

impl Environment {
    pub fn capture() -> Self {
        Self {
            socket: std::env::var_os(SOCKET_VARIABLE),
            runtime_directory: std::env::var_os(RUNTIME_DIRECTORY_VARIABLE),
        }
    }

    fn open_runtime_directory(&self) -> Result<RuntimeDirectory, RuntimeDirectoryError> {
        RuntimeDirectory::from_variable(self.runtime_directory.clone())
    }
}

pub fn resolve(
    socket: Option<&PathBuf>,
    instance: Option<&InstanceName>,
    environment: &Environment,
) -> Result<PathBuf, TargetError> {
    if let Some(socket) = socket {
        return Ok(socket.clone());
    }
    if let Some(instance) = instance {
        return Ok(environment.open_runtime_directory()?.socket_path(instance));
    }
    if let Some(socket) = environment.socket.as_ref().filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(socket));
    }
    let runtime = environment.open_runtime_directory()?;
    let instance = sole(runtime.live_instances()?)?;
    Ok(runtime.socket_path(&instance))
}

pub fn running_instances(environment: &Environment) -> Result<Vec<InstanceName>, TargetError> {
    Ok(environment.open_runtime_directory()?.live_instances()?)
}

fn sole(instances: Vec<InstanceName>) -> Result<InstanceName, TargetError> {
    match <[InstanceName; 1]>::try_from(instances) {
        Ok([only]) => Ok(only),
        Err(none) if none.is_empty() => Err(TargetError::NoInstance),
        Err(several) => Err(TargetError::Ambiguous(several)),
    }
}

fn list(instances: &[InstanceName]) -> String {
    let names: Vec<&str> = instances.iter().map(InstanceName::as_str).collect();
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(name: &str) -> InstanceName {
        name.parse().unwrap()
    }

    #[test]
    fn explicit_socket_wins_over_everything_else() {
        let environment = Environment {
            socket: Some("/from/environment.sock".into()),
            runtime_directory: None,
        };
        let explicit = PathBuf::from("/explicit.sock");

        let target = resolve(Some(&explicit), None, &environment).unwrap();

        assert_eq!(target, explicit);
    }

    #[test]
    fn exported_socket_is_used_when_nothing_is_named() {
        let environment = Environment {
            socket: Some("/from/environment.sock".into()),
            runtime_directory: None,
        };

        let target = resolve(None, None, &environment).unwrap();

        assert_eq!(target, PathBuf::from("/from/environment.sock"));
    }

    #[test]
    fn named_instance_needs_a_runtime_directory() {
        let outcome = resolve(None, Some(&instance("main")), &Environment::default());

        assert!(matches!(
            outcome,
            Err(TargetError::RuntimeDirectory(RuntimeDirectoryError::Unset))
        ));
    }

    #[test]
    fn exactly_one_running_instance_is_required() {
        let ambiguity = sole(vec![instance("alpha"), instance("beta")]).unwrap_err();

        assert!(matches!(sole(Vec::new()), Err(TargetError::NoInstance)));
        assert_eq!(sole(vec![instance("only")]).unwrap(), instance("only"));
        assert_eq!(
            ambiguity.to_string(),
            "several instances are running (alpha, beta), choose one with --instance"
        );
    }
}
