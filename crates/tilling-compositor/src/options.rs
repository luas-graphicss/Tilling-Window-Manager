use std::path::PathBuf;
use std::time::Duration;

use tilling_config::{ConfigSource, ValidatedConfig};

use crate::helpers::SessionServices;

const DEFAULT_OUTPUT_WIDTH: u32 = 1280;
const DEFAULT_OUTPUT_HEIGHT: u32 = 720;
const DEFAULT_TRANSACTION_TIMEOUT: Duration = Duration::from_millis(200);
const DEFAULT_OUTPUT_CONFIRMATION: Duration = Duration::from_secs(15);
const DEFAULT_HELPER_RESTART_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Nested,
    Headless,
    Drm,
}

impl BackendKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Headless => "headless",
            Self::Drm => "drm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualOutput {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct Options {
    pub backend: BackendKind,
    pub config: ValidatedConfig,
    pub config_source: ConfigSource,
    pub runtime_dir: PathBuf,
    pub instance: String,
    pub output: VirtualOutput,
    pub transaction_timeout: Duration,
    pub output_confirmation: Duration,
    pub helper_restart_delay: Duration,
    pub session_services: Option<SessionServices>,
}

impl Options {
    #[must_use]
    pub fn new(
        backend: BackendKind,
        config: ValidatedConfig,
        config_source: ConfigSource,
        runtime_dir: PathBuf,
    ) -> Self {
        Self {
            backend,
            config,
            config_source,
            runtime_dir,
            instance: std::process::id().to_string(),
            output: VirtualOutput {
                name: format!("{}-1", backend.name().to_ascii_uppercase()),
                width: DEFAULT_OUTPUT_WIDTH,
                height: DEFAULT_OUTPUT_HEIGHT,
            },
            transaction_timeout: DEFAULT_TRANSACTION_TIMEOUT,
            output_confirmation: DEFAULT_OUTPUT_CONFIRMATION,
            helper_restart_delay: DEFAULT_HELPER_RESTART_DELAY,
            session_services: None,
        }
    }
}
