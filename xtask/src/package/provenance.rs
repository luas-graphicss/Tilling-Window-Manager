use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::workspace::Workspace;
use super::{Artifact, Target, checksums};
use crate::cargo;

const DISTRIBUTED_APPLICATION: &str = "tilling";
const BUILD_PROFILE: &str = "release";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub commit: Option<String>,
    pub rustc: Option<String>,
    pub cargo: Option<String>,
}

pub fn observe(root: &Path) -> Origin {
    Origin {
        commit: printed(Command::new("git").args(["rev-parse", "HEAD"]), root),
        rustc: printed(Command::new("rustc").arg("--version"), root),
        cargo: printed(cargo::command().arg("--version"), root),
    }
}

pub fn document(
    workspace: &Workspace,
    target: Target,
    origin: &Origin,
    artifacts: &[Artifact],
) -> Value {
    json!({
        "package": DISTRIBUTED_APPLICATION,
        "version": workspace.version,
        "target": target.triple(),
        "profile": BUILD_PROFILE,
        "source": {
            "repository": workspace.repository,
            "commit": origin.commit,
            "lockfile_sha256": checksums::sha256(workspace.lockfile.as_bytes()),
        },
        "toolchain": {
            "rustc": origin.rustc,
            "cargo": origin.cargo,
        },
        "artifacts": artifacts
            .iter()
            .map(|artifact| json!({
                "name": artifact.name,
                "bytes": artifact.contents.len(),
                "sha256": checksums::sha256(&artifact.contents),
            }))
            .collect::<Vec<Value>>(),
    })
}

fn printed(command: &mut Command, directory: &Path) -> Option<String> {
    let finished = command
        .current_dir(directory)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&finished.stdout).trim().to_owned();
    (finished.status.success() && !line.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::workspace::tests::workspace;

    fn origin() -> Origin {
        Origin {
            commit: None,
            rustc: Some("rustc 1.100.0".to_owned()),
            cargo: Some("cargo 1.100.0".to_owned()),
        }
    }

    #[test]
    fn document_ties_each_artifact_to_the_source_and_the_toolchain() {
        let artifacts = [Artifact {
            name: "tilling_0.1.0_amd64.deb".to_owned(),
            contents: b"abc".to_vec(),
        }];

        let document = document(&workspace(), Target::X86_64Linux, &origin(), &artifacts);

        assert_eq!(document["target"], "x86_64-unknown-linux-gnu");
        assert_eq!(
            document["source"]["repository"],
            "https://example.org/tilling"
        );
        assert_eq!(document["toolchain"]["rustc"], "rustc 1.100.0");
        assert_eq!(document["artifacts"][0]["bytes"], 3);
        assert_eq!(
            document["artifacts"][0]["sha256"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            document["source"]["lockfile_sha256"].as_str().map(str::len),
            Some(64)
        );
    }

    #[test]
    fn build_outside_a_checkout_records_no_commit() {
        let document = document(&workspace(), Target::X86_64Linux, &origin(), &[]);

        assert!(document["source"]["commit"].is_null());
    }

    #[test]
    fn command_that_cannot_run_reports_nothing() {
        let outcome = printed(
            &mut Command::new("/nonexistent/tilling-toolchain"),
            Path::new("/"),
        );

        assert_eq!(outcome, None);
    }
}
