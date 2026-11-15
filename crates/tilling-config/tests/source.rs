use std::fs;
use std::path::{Path, PathBuf};

use tilling_config::{ConfigError, ConfigSource, Environment, MAX_CONFIG_BYTES, load};

struct ScratchDirectory(PathBuf);

impl ScratchDirectory {
    fn new(name: &str) -> Self {
        let unique = format!("tilling-config-{}-{name}", std::process::id());
        let path = std::env::temp_dir().join(unique);
        create_directory(&path);
        Self(path)
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn write(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.join(relative);
        if let Some(parent) = path.parent() {
            create_directory(parent);
        }
        fs::write(&path, contents)
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        path
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("remove {}: {error}", self.0.display());
        }
    }
}

fn create_directory(path: &Path) {
    fs::create_dir_all(path).unwrap_or_else(|error| panic!("create {}: {error}", path.display()));
}

#[test]
fn explicit_path_wins_over_the_user_configuration() {
    let home = ScratchDirectory::new("explicit");
    home.write(".config/tilling/config.toml", "schema_version = 1\n");
    let environment = Environment {
        xdg_config_home: None,
        home: Some(home.join("")),
    };
    let explicit = home.join("custom.toml");

    let source = ConfigSource::resolve(Some(explicit.clone()), &environment);

    assert_eq!(source, ConfigSource::File(explicit));
}

#[test]
fn user_configuration_is_found_under_xdg_config_home() {
    let config_home = ScratchDirectory::new("xdg");
    let path = config_home.write("tilling/config.toml", "schema_version = 1\n");
    let environment = Environment {
        xdg_config_home: Some(config_home.join("")),
        home: None,
    };

    let source = ConfigSource::resolve(None, &environment);

    assert_eq!(source, ConfigSource::File(path));
    assert!(load(&source).is_ok());
}

#[test]
fn relative_xdg_config_home_falls_back_to_the_home_directory() {
    let home = ScratchDirectory::new("relative");
    let environment = Environment {
        xdg_config_home: Some(PathBuf::from("relative/config")),
        home: Some(home.join("")),
    };

    assert_eq!(
        environment.user_config_path(),
        Some(home.join(".config/tilling/config.toml"))
    );
}

#[test]
fn absent_user_configuration_selects_the_built_in_defaults() {
    let home = ScratchDirectory::new("absent");
    let environment = Environment {
        xdg_config_home: None,
        home: Some(home.join("")),
    };

    let source = ConfigSource::resolve(None, &environment);

    assert_eq!(source, ConfigSource::BuiltIn);
    assert_eq!(
        ConfigSource::resolve(None, &Environment::default()),
        ConfigSource::BuiltIn
    );
    assert!(load(&source).is_ok());
}

#[test]
fn missing_explicit_file_is_a_read_error() {
    let directory = ScratchDirectory::new("missing");
    let path = directory.join("nowhere.toml");

    let outcome = load(&ConfigSource::File(path.clone()));

    assert!(matches!(outcome, Err(ConfigError::Read { path: reported, .. }) if reported == path));
}

#[test]
fn oversized_and_binary_files_are_rejected() {
    let directory = ScratchDirectory::new("oversized");
    let oversized = directory.write("oversized.toml", "#".repeat(MAX_CONFIG_BYTES + 4096));
    let binary = directory.write("binary.toml", [0xff, 0xfe, 0x00, 0x01]);

    assert!(matches!(
        load(&ConfigSource::File(oversized)),
        Err(ConfigError::TooLarge { .. })
    ));
    assert!(matches!(
        load(&ConfigSource::File(binary)),
        Err(ConfigError::Read { .. })
    ));
}
