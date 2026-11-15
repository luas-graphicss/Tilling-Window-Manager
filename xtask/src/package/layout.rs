use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::Target;
use super::workspace::Workspace;

const EXECUTABLE_MODE: u32 = 0o755;
const REGULAR_MODE: u32 = 0o644;
const BUILD_PROFILE: &str = "release";
const COMPLETED_BINARY: &str = "tillingctl";
const REPOSITORY_FILES: [(&str, &str); 4] = [
    (
        "packaging/sessions/tilling.desktop",
        "usr/share/wayland-sessions/tilling.desktop",
    ),
    ("config/default.toml", "usr/share/tilling/config.toml"),
    ("LICENSE", "usr/share/doc/tilling/copyright"),
    ("README.md", "usr/share/doc/tilling/README.md"),
];
const COMPLETIONS: [(&str, &str); 3] = [
    ("bash", "usr/share/bash-completion/completions/tillingctl"),
    ("zsh", "usr/share/zsh/site-functions/_tillingctl"),
    (
        "fish",
        "usr/share/fish/vendor_completions.d/tillingctl.fish",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub destination: String,
    pub mode: u32,
    pub contents: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("read {}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("run {} to print the {shell} completions", .program.display())]
    Completions {
        program: PathBuf,
        shell: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("{} did not print the {shell} completions", .program.display())]
    CompletionsRefused {
        program: PathBuf,
        shell: &'static str,
    },
}

pub fn collect(
    workspace: &Workspace,
    target: Target,
    binaries: &[&str],
) -> Result<Vec<Entry>, LayoutError> {
    let built = workspace
        .target_directory
        .join(target.triple())
        .join(BUILD_PROFILE);
    let mut entries = Vec::new();
    for name in binaries {
        entries.push(Entry {
            destination: format!("usr/bin/{name}"),
            mode: EXECUTABLE_MODE,
            contents: read(&built.join(name))?,
        });
    }
    for (source, destination) in REPOSITORY_FILES {
        entries.push(Entry {
            destination: destination.to_owned(),
            mode: REGULAR_MODE,
            contents: read(&workspace.root.join(source))?,
        });
    }
    let completed = built.join(COMPLETED_BINARY);
    for (shell, destination) in COMPLETIONS {
        entries.push(Entry {
            destination: destination.to_owned(),
            mode: REGULAR_MODE,
            contents: completions(&completed, shell)?,
        });
    }
    entries.sort_by(|first, second| first.destination.cmp(&second.destination));
    Ok(entries)
}

pub fn read(path: &Path) -> Result<Vec<u8>, LayoutError> {
    fs::read(path).map_err(|source| LayoutError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn completions(program: &Path, shell: &'static str) -> Result<Vec<u8>, LayoutError> {
    let finished = Command::new(program)
        .args(["completions", shell])
        .stdin(Stdio::null())
        .output()
        .map_err(|source| LayoutError::Completions {
            program: program.to_path_buf(),
            shell,
            source,
        })?;
    if !finished.status.success() || finished.stdout.is_empty() {
        return Err(LayoutError::CompletionsRefused {
            program: program.to_path_buf(),
            shell,
        });
    }
    Ok(finished.stdout)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::package) fn entries() -> Vec<Entry> {
        vec![
            Entry {
                destination: "usr/bin/tilling".to_owned(),
                mode: EXECUTABLE_MODE,
                contents: b"\x7fELF compositor".to_vec(),
            },
            Entry {
                destination: "usr/share/tilling/config.toml".to_owned(),
                mode: REGULAR_MODE,
                contents: b"schema_version = 1\n".to_vec(),
            },
        ]
    }

    #[test]
    fn every_destination_stays_inside_the_system_prefix() {
        let destinations = REPOSITORY_FILES
            .iter()
            .chain(COMPLETIONS.iter())
            .map(|(_, destination)| *destination);

        for destination in destinations {
            assert!(destination.starts_with("usr/share/"), "{destination}");
            assert!(!destination.contains(".."), "{destination}");
        }
    }

    #[test]
    fn missing_file_is_reported_with_its_path() {
        let missing = Path::new("/nonexistent/tilling/README.md");

        let failure = read(missing).unwrap_err();

        assert_eq!(failure.to_string(), "read /nonexistent/tilling/README.md");
    }

    #[test]
    fn program_that_cannot_start_is_reported_with_the_shell() {
        let failure = completions(Path::new("/nonexistent/tillingctl"), "fish").unwrap_err();

        assert_eq!(
            failure.to_string(),
            "run /nonexistent/tillingctl to print the fish completions"
        );
    }
}
