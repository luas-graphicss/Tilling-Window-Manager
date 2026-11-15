use std::ffi::OsString;
use std::path::PathBuf;

use tilling_compositor::{CompositorError, Options};
use tilling_config::{ConfigError, ConfigSource};

use crate::arguments::Arguments;

const RUNTIME_DIRECTORY_VARIABLE: &str = "XDG_RUNTIME_DIR";

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("XDG_RUNTIME_DIR is not set")]
    NoRuntimeDirectory,
    #[error("load the configuration")]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Compositor(#[from] CompositorError),
}

#[derive(Debug, Default)]
pub struct Environment {
    pub runtime_directory: Option<OsString>,
    pub config: tilling_config::Environment,
}

impl Environment {
    pub fn capture() -> Self {
        Self {
            runtime_directory: std::env::var_os(RUNTIME_DIRECTORY_VARIABLE),
            config: tilling_config::Environment::from_process(),
        }
    }
}

pub fn run(arguments: Arguments, environment: &Environment) -> Result<(), LaunchError> {
    let options = prepare(arguments, environment)?;
    Ok(tilling_compositor::run(options)?)
}

fn prepare(arguments: Arguments, environment: &Environment) -> Result<Options, LaunchError> {
    let runtime_directory = environment
        .runtime_directory
        .as_ref()
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from)
        .ok_or(LaunchError::NoRuntimeDirectory)?;
    let source = ConfigSource::resolve(arguments.config, &environment.config);
    let config = tilling_config::load(&source)?;
    let mut options = Options::new(arguments.backend.into(), config, source, runtime_directory);
    if let Some(instance) = arguments.instance {
        options.instance = instance;
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use tilling_compositor::BackendKind;

    use super::*;
    use crate::arguments::Backend;

    fn arguments() -> Arguments {
        Arguments {
            backend: Backend::Headless,
            config: None,
            instance: None,
            verbose: false,
        }
    }

    fn environment(runtime_directory: &str) -> Environment {
        Environment {
            runtime_directory: Some(runtime_directory.into()),
            config: tilling_config::Environment::default(),
        }
    }

    #[test]
    fn missing_or_empty_runtime_directory_is_refused() {
        let unset = prepare(arguments(), &Environment::default());
        let empty = prepare(arguments(), &environment(""));

        assert!(matches!(unset, Err(LaunchError::NoRuntimeDirectory)));
        assert!(matches!(empty, Err(LaunchError::NoRuntimeDirectory)));
    }

    #[test]
    fn built_in_configuration_is_used_when_no_file_is_found() {
        let options = prepare(arguments(), &environment("/run/user/1000")).unwrap();

        assert_eq!(options.backend, BackendKind::Headless);
        assert_eq!(options.config_source, ConfigSource::BuiltIn);
        assert_eq!(options.runtime_dir, PathBuf::from("/run/user/1000"));
        assert_eq!(options.instance, std::process::id().to_string());
    }

    #[test]
    fn chosen_instance_name_replaces_the_process_id() {
        let named = Arguments {
            instance: Some("scenario".to_owned()),
            ..arguments()
        };

        let options = prepare(named, &environment("/run/user/1000")).unwrap();

        assert_eq!(options.instance, "scenario");
    }

    #[test]
    fn unreadable_configuration_file_stops_the_launch() {
        let missing = Arguments {
            config: Some(PathBuf::from("/nonexistent/tilling.toml")),
            ..arguments()
        };

        let outcome = prepare(missing, &environment("/run/user/1000"));

        assert!(matches!(outcome, Err(LaunchError::Config(_))));
    }
}
