use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use tilling_compositor::BackendKind;

#[derive(Debug, Parser)]
#[command(name = "tilling", version, about = "Tiling Wayland compositor")]
pub struct Arguments {
    #[arg(
        long,
        value_enum,
        default_value_t = Backend::Nested,
        help = "Where the compositor presents its output"
    )]
    pub backend: Backend,
    #[arg(
        long,
        value_name = "PATH",
        help = "Configuration file to load instead of the user configuration"
    )]
    pub config: Option<PathBuf>,
    #[arg(
        long,
        value_name = "NAME",
        help = "Name of this instance, used for its sockets"
    )]
    pub instance: Option<String>,
    #[arg(
        long,
        value_name = "UNIT",
        help = "Unit of the user service manager started with the session and stopped with it"
    )]
    pub session_target: Option<String>,
    #[arg(long, help = "Log debugging details")]
    pub verbose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    Nested,
    Headless,
    Drm,
}

impl From<Backend> for BackendKind {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::Nested => Self::Nested,
            Backend::Headless => Self::Headless,
            Backend::Drm => Self::Drm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_backend_and_discovered_configuration_are_the_defaults() {
        let arguments = Arguments::try_parse_from(["tilling"]).unwrap();

        assert_eq!(arguments.backend, Backend::Nested);
        assert_eq!(arguments.config, None);
        assert_eq!(arguments.instance, None);
        assert_eq!(arguments.session_target, None);
        assert!(!arguments.verbose);
    }

    #[test]
    fn native_session_can_name_the_unit_that_follows_it() {
        let arguments = Arguments::try_parse_from([
            "tilling",
            "--backend",
            "drm",
            "--session-target",
            "tilling-session.target",
        ])
        .unwrap();

        assert_eq!(
            arguments.session_target.as_deref(),
            Some("tilling-session.target")
        );
    }

    #[test]
    fn backend_configuration_and_instance_can_be_chosen() {
        let arguments = Arguments::try_parse_from([
            "tilling",
            "--backend",
            "headless",
            "--config",
            "config/default.toml",
            "--instance",
            "scenario",
        ])
        .unwrap();

        assert_eq!(BackendKind::from(arguments.backend), BackendKind::Headless);
        assert_eq!(arguments.config, Some(PathBuf::from("config/default.toml")));
        assert_eq!(arguments.instance.as_deref(), Some("scenario"));
    }

    #[test]
    fn drm_backend_can_be_chosen_for_a_native_session() {
        let arguments = Arguments::try_parse_from(["tilling", "--backend", "drm"]).unwrap();

        assert_eq!(BackendKind::from(arguments.backend), BackendKind::Drm);
    }

    #[test]
    fn unknown_backend_is_rejected() {
        assert!(Arguments::try_parse_from(["tilling", "--backend", "x11"]).is_err());
    }
}
