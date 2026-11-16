mod checksums;
mod deb;
mod layout;
mod provenance;
mod sbom;
mod tarball;
mod workspace;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::ValueEnum;

use self::deb::{DebError, Release};
use self::layout::LayoutError;
use self::workspace::WorkspaceError;
use crate::cargo;

const SHIPPED_BINARIES: [&str; 2] = ["tilling", "tillingctl"];
const CONTROL_TEMPLATE: &str = "packaging/debian/control";
const CHECKSUMS_NAME: &str = "SHA256SUMS";
const SOURCE_DATE_VARIABLE: &str = "SOURCE_DATE_EPOCH";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    #[value(name = "x86_64-unknown-linux-gnu")]
    X86_64Linux,
}

impl Target {
    pub const fn triple(self) -> &'static str {
        match self {
            Self::X86_64Linux => "x86_64-unknown-linux-gnu",
        }
    }

    pub const fn debian_architecture(self) -> &'static str {
        match self {
            Self::X86_64Linux => "amd64",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub name: String,
    pub contents: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error("start cargo")]
    Cargo(#[source] io::Error),
    #[error("the release build failed")]
    BuildFailed,
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("assemble the distribution archive")]
    Tarball(#[source] io::Error),
    #[error(transparent)]
    Deb(#[from] DebError),
    #[error("encode a document as JSON")]
    Encode(#[from] serde_json::Error),
    #[error("write {}", .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("write the report")]
    Report(#[from] io::Error),
}

pub fn run(target: Target, output: &Path, report: &mut impl Write) -> Result<(), PackageError> {
    let workspace = workspace::read(target, &SHIPPED_BINARIES)?;
    writeln!(
        report,
        "package tilling {} for {}",
        workspace.version,
        target.triple()
    )?;
    build(target)?;
    let entries = layout::collect(&workspace, target, &SHIPPED_BINARIES)?;
    let template = layout::read(&workspace.root.join(CONTROL_TEMPLATE))?;
    let version = workspace.version.as_str();
    let modified = modification_time();
    let release = Release {
        version,
        architecture: target.debian_architecture(),
        modified,
    };
    let distribution = format!("tilling-{version}-{}", target.triple());
    let mut artifacts = vec![
        Artifact {
            contents: tarball::build(&entries, &distribution, modified)
                .map_err(PackageError::Tarball)?,
            name: format!("{distribution}.tar.gz"),
        },
        Artifact {
            name: format!("tilling_{version}_{}.deb", target.debian_architecture()),
            contents: deb::build(&entries, &String::from_utf8_lossy(&template), release)?,
        },
        Artifact {
            name: format!("tilling-{version}.cdx.json"),
            contents: serde_json::to_vec_pretty(&sbom::document(&workspace))?,
        },
        Artifact {
            name: format!("tilling-{version}.licenses.txt"),
            contents: sbom::licenses(&workspace).into_bytes(),
        },
    ];
    let origin = provenance::observe(&workspace.root);
    let described = provenance::document(&workspace, target, &origin, &artifacts);
    artifacts.push(Artifact {
        name: format!("tilling-{version}.provenance.json"),
        contents: serde_json::to_vec_pretty(&described)?,
    });
    artifacts.push(Artifact {
        name: CHECKSUMS_NAME.to_owned(),
        contents: checksums::listing(&artifacts).into_bytes(),
    });
    write(output, &artifacts, report)
}

fn build(target: Target) -> Result<(), PackageError> {
    let mut command = cargo::command();
    command.args([
        "build",
        "--release",
        "--locked",
        "--target",
        target.triple(),
    ]);
    for binary in SHIPPED_BINARIES {
        command.args(["--package", binary]);
    }
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .map_err(PackageError::Cargo)?;
    if status.success() {
        Ok(())
    } else {
        Err(PackageError::BuildFailed)
    }
}

fn write(
    output: &Path,
    artifacts: &[Artifact],
    report: &mut impl Write,
) -> Result<(), PackageError> {
    fs::create_dir_all(output).map_err(|source| PackageError::Write {
        path: output.to_path_buf(),
        source,
    })?;
    for artifact in artifacts {
        let path = output.join(&artifact.name);
        fs::write(&path, &artifact.contents).map_err(|source| PackageError::Write {
            path: path.clone(),
            source,
        })?;
        writeln!(
            report,
            "  wrote {} ({} bytes)",
            path.display(),
            artifact.contents.len()
        )?;
    }
    Ok(())
}

fn modification_time() -> u64 {
    let declared = std::env::var(SOURCE_DATE_VARIABLE)
        .ok()
        .and_then(|epoch| epoch.parse().ok());
    declared.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::Sandbox;

    #[test]
    fn target_is_named_by_its_triple_on_the_command_line() {
        let value = Target::X86_64Linux.to_possible_value().unwrap();

        assert_eq!(value.get_name(), Target::X86_64Linux.triple());
        assert_eq!(Target::X86_64Linux.debian_architecture(), "amd64");
    }

    #[test]
    fn arch_package_has_the_version_of_the_workspace() {
        let manifest = include_str!("../../Cargo.toml");
        let definition = include_str!("../../packaging/arch/PKGBUILD");
        let workspace_version = manifest
            .split_once("[workspace.package]")
            .and_then(|(_, package)| package.split_once("version = \""))
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(version, _)| version);
        let package_version = definition
            .lines()
            .find_map(|line| line.strip_prefix("pkgver="));

        assert!(workspace_version.is_some());
        assert_eq!(package_version, workspace_version);
    }

    #[test]
    fn artifacts_are_written_and_listed_in_the_report() {
        let sandbox = Sandbox::create(None, "package-write").unwrap();
        let output = sandbox.path().join("dist");
        let artifacts = [Artifact {
            name: CHECKSUMS_NAME.to_owned(),
            contents: b"abc".to_vec(),
        }];
        let mut report = Vec::new();

        write(&output, &artifacts, &mut report).unwrap();

        assert_eq!(fs::read(output.join(CHECKSUMS_NAME)).unwrap(), b"abc");
        assert_eq!(
            String::from_utf8(report).unwrap(),
            format!(
                "  wrote {} (3 bytes)\n",
                output.join(CHECKSUMS_NAME).display()
            )
        );
    }
}
