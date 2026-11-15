use std::path::PathBuf;
use std::time::Duration;

use tilling_config::{ConfigSource, ValidatedConfig};

const DEFAULT_OUTPUT_WIDTH: u32 = 1280;
const DEFAULT_OUTPUT_HEIGHT: u32 = 720;
const DEFAULT_TRANSACTION_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Nested,
    Headless,
}

impl BackendKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Headless => "headless",
        }
    }

    const fn output_name(self) -> &'static str {
        match self {
            Self::Nested => "NESTED-1",
            Self::Headless => "HEADLESS-1",
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
                name: backend.output_name().to_owned(),
                width: DEFAULT_OUTPUT_WIDTH,
                height: DEFAULT_OUTPUT_HEIGHT,
            },
            transaction_timeout: DEFAULT_TRANSACTION_TIMEOUT,
        }
    }
}
