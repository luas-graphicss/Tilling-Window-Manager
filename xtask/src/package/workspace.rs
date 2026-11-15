use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use serde::Deserialize;

use super::Target;
use crate::cargo;

const LOCKFILE_NAME: &str = "Cargo.lock";
const DEVELOPMENT_KIND: &str = "dev";

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("start cargo")]
    Cargo(#[source] io::Error),
    #[error("cargo metadata failed: {0}")]
    MetadataRefused(String),
    #[error("read the output of cargo metadata")]
    Metadata(#[source] serde_json::Error),
    #[error("read {}", .path.display())]
    Lockfile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("read the packages of the lockfile")]
    LockfileSyntax(#[source] toml::de::Error),
    #[error("the workspace has no package named {0}")]
    MissingPackage(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub root: PathBuf,
    pub target_directory: PathBuf,
    pub version: String,
    pub repository: Option<String>,
    pub lockfile: String,
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Component {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub registry_checksum: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
    target_directory: PathBuf,
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    repository: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
struct Node {
    id: String,
    deps: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    pkg: String,
    dep_kinds: Vec<DependencyKind>,
}

#[derive(Debug, Deserialize)]
struct DependencyKind {
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Lockfile {
    #[serde(default, rename = "package")]
    packages: Vec<LockedPackage>,
}

#[derive(Debug, Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
    checksum: Option<String>,
}

pub fn read(target: Target, shipped: &[&'static str]) -> Result<Workspace, WorkspaceError> {
    let finished = cargo::command()
        .args(["metadata", "--format-version", "1", "--locked"])
        .args(["--filter-platform", target.triple()])
        .stdin(Stdio::null())
        .output()
        .map_err(WorkspaceError::Cargo)?;
    if !finished.status.success() {
        let refusal = String::from_utf8_lossy(&finished.stderr).trim().to_owned();
        return Err(WorkspaceError::MetadataRefused(refusal));
    }
    let metadata: Metadata =
        serde_json::from_slice(&finished.stdout).map_err(WorkspaceError::Metadata)?;
    let path = metadata.workspace_root.join(LOCKFILE_NAME);
    let lockfile = std::fs::read_to_string(&path)
        .map_err(|source| WorkspaceError::Lockfile { path, source })?;
    resolve(metadata, lockfile, shipped)
}

fn resolve(
    metadata: Metadata,
    lockfile: String,
    shipped: &[&'static str],
) -> Result<Workspace, WorkspaceError> {
    let locked: Lockfile = toml::from_str(&lockfile).map_err(WorkspaceError::LockfileSyntax)?;
    let checksums: BTreeMap<(&str, &str), &str> = locked
        .packages
        .iter()
        .filter_map(|package| {
            let checksum = package.checksum.as_deref()?;
            Some(((package.name.as_str(), package.version.as_str()), checksum))
        })
        .collect();
    let mut roots = Vec::with_capacity(shipped.len());
    for name in shipped {
        let package = metadata
            .packages
            .iter()
            .find(|package| package.name == *name)
            .ok_or(WorkspaceError::MissingPackage(name))?;
        roots.push(package);
    }
    let Some(main) = roots.first() else {
        return Err(WorkspaceError::MissingPackage("tilling"));
    };
    let built_into_release = closure(&metadata.resolve, roots.iter().map(|root| root.id.as_str()));
    let mut components: Vec<Component> = metadata
        .packages
        .iter()
        .filter(|package| built_into_release.contains(package.id.as_str()))
        .map(|package| Component {
            name: package.name.clone(),
            version: package.version.clone(),
            license: package.license.clone(),
            registry_checksum: checksums
                .get(&(package.name.as_str(), package.version.as_str()))
                .map(|checksum| (*checksum).to_owned()),
        })
        .collect();
    components.sort();
    Ok(Workspace {
        version: main.version.clone(),
        repository: main.repository.clone(),
        root: metadata.workspace_root,
        target_directory: metadata.target_directory,
        lockfile,
        components,
    })
}

fn closure<'graph>(
    resolve: &'graph Resolve,
    roots: impl Iterator<Item = &'graph str>,
) -> BTreeSet<&'graph str> {
    let edges: BTreeMap<&str, &[Dependency]> = resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.deps.as_slice()))
        .collect();
    let mut reached = BTreeSet::new();
    let mut pending: Vec<&str> = roots.collect();
    while let Some(package) = pending.pop() {
        if !reached.insert(package) {
            continue;
        }
        let dependencies = edges.get(package).copied().unwrap_or_default();
        pending.extend(
            dependencies
                .iter()
                .filter(|dependency| is_built_into_release(dependency))
                .map(|dependency| dependency.pkg.as_str()),
        );
    }
    reached
}

fn is_built_into_release(dependency: &Dependency) -> bool {
    dependency
        .dep_kinds
        .iter()
        .any(|kind| kind.kind.as_deref() != Some(DEVELOPMENT_KIND))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    const METADATA: &str = r#"{
        "packages": [
            {"id": "tilling 0.1.0", "name": "tilling", "version": "0.1.0", "license": "GPL-3.0-only", "repository": "https://example.org/tilling"},
            {"id": "tillingctl 0.1.0", "name": "tillingctl", "version": "0.1.0", "license": "GPL-3.0-only", "repository": null},
            {"id": "xtask 0.1.0", "name": "xtask", "version": "0.1.0", "license": "GPL-3.0-only", "repository": null},
            {"id": "serde 1.0.229", "name": "serde", "version": "1.0.229", "license": "MIT OR Apache-2.0", "repository": null},
            {"id": "cc 1.2.0", "name": "cc", "version": "1.2.0", "license": "MIT OR Apache-2.0", "repository": null},
            {"id": "tempfile 3.27.0", "name": "tempfile", "version": "3.27.0", "license": "MIT OR Apache-2.0", "repository": null},
            {"id": "unlicensed 0.3.0", "name": "unlicensed", "version": "0.3.0", "license": null, "repository": null}
        ],
        "resolve": {"nodes": [
            {"id": "tilling 0.1.0", "deps": [
                {"pkg": "serde 1.0.229", "dep_kinds": [{"kind": null}]},
                {"pkg": "tempfile 3.27.0", "dep_kinds": [{"kind": "dev"}]}
            ]},
            {"id": "tillingctl 0.1.0", "deps": [
                {"pkg": "serde 1.0.229", "dep_kinds": [{"kind": null}, {"kind": "dev"}]}
            ]},
            {"id": "serde 1.0.229", "deps": [
                {"pkg": "cc 1.2.0", "dep_kinds": [{"kind": "build"}]},
                {"pkg": "unlicensed 0.3.0", "dep_kinds": [{"kind": null}]}
            ]},
            {"id": "xtask 0.1.0", "deps": []},
            {"id": "cc 1.2.0", "deps": []},
            {"id": "tempfile 3.27.0", "deps": []},
            {"id": "unlicensed 0.3.0", "deps": []}
        ]},
        "target_directory": "/work/target",
        "workspace_root": "/work"
    }"#;
    const LOCKFILE: &str = r#"
version = 4

[[package]]
name = "serde"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aa11"

[[package]]
name = "cc"
version = "1.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bb22"

[[package]]
name = "tilling"
version = "0.1.0"
"#;

    pub(in crate::package) fn workspace() -> Workspace {
        let metadata = serde_json::from_str(METADATA).unwrap();
        resolve(metadata, LOCKFILE.to_owned(), &["tilling", "tillingctl"]).unwrap()
    }

    fn names(workspace: &Workspace) -> Vec<&str> {
        workspace
            .components
            .iter()
            .map(|component| component.name.as_str())
            .collect()
    }

    #[test]
    fn components_are_what_the_shipped_binaries_are_built_from() {
        let workspace = workspace();

        assert_eq!(
            names(&workspace),
            ["cc", "serde", "tilling", "tillingctl", "unlicensed"]
        );
        assert_eq!(workspace.version, "0.1.0");
        assert_eq!(
            workspace.repository.as_deref(),
            Some("https://example.org/tilling")
        );
        assert_eq!(workspace.target_directory, PathBuf::from("/work/target"));
    }

    #[test]
    fn registry_packages_carry_the_checksum_of_the_lockfile() {
        let workspace = workspace();
        let checksum_of = |name: &str| {
            workspace
                .components
                .iter()
                .find(|component| component.name == name)
                .and_then(|component| component.registry_checksum.clone())
        };

        assert_eq!(checksum_of("serde").as_deref(), Some("aa11"));
        assert_eq!(checksum_of("cc").as_deref(), Some("bb22"));
        assert_eq!(checksum_of("tilling"), None);
    }

    #[test]
    fn package_that_is_not_in_the_workspace_is_reported_by_name() {
        let metadata = serde_json::from_str(METADATA).unwrap();

        let outcome = resolve(metadata, LOCKFILE.to_owned(), &["tilling", "tilling-panel"]);

        assert!(matches!(
            outcome,
            Err(WorkspaceError::MissingPackage("tilling-panel"))
        ));
    }
}
