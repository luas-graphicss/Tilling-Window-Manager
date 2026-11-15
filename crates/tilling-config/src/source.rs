use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::diagnostics::ConfigError;
use crate::limits::MAX_CONFIG_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    File(PathBuf),
    BuiltIn,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    pub xdg_config_home: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

impl Environment {
    #[must_use]
    pub fn from_process() -> Self {
        Self {
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            home: std::env::var_os("HOME").map(PathBuf::from),
        }
    }

    #[must_use]
    pub fn user_config_path(&self) -> Option<PathBuf> {
        let config_home = self
            .xdg_config_home
            .as_deref()
            .filter(|path| path.is_absolute())
            .map(Path::to_path_buf)
            .or_else(|| self.home.as_ref().map(|home| home.join(".config")))?;
        Some(config_home.join("tilling").join("config.toml"))
    }
}

impl ConfigSource {
    #[must_use]
    pub fn resolve(explicit: Option<PathBuf>, environment: &Environment) -> Self {
        explicit
            .or_else(|| environment.user_config_path().filter(|path| path.is_file()))
            .map_or(Self::BuiltIn, Self::File)
    }
}

pub(crate) fn read(path: &Path) -> Result<String, ConfigError> {
    let failure = |source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    };
    let file = File::open(path).map_err(failure)?;
    let limit = u64::try_from(MAX_CONFIG_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut text = String::new();
    file.take(limit)
        .read_to_string(&mut text)
        .map_err(failure)?;
    Ok(text)
}
