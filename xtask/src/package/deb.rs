use std::io;

use super::layout::Entry;
use super::tarball;

const ARCHIVE_MAGIC: &[u8] = b"!<arch>\n";
const FORMAT_VERSION: &[u8] = b"2.0\n";
const PACKAGE_FIELD: &str = "Package:";
const MEMBER_MODE: u32 = 0o100_644;
const CONTROL_MODE: u32 = 0o644;
const ROOT_OWNER: u32 = 0;
const BYTES_PER_KIBIBYTE: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum DebError {
    #[error("the control template must start with the Package field")]
    TemplateWithoutPackage,
    #[error("assemble the archives of the package")]
    Archive(#[from] io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Release<'package> {
    pub version: &'package str,
    pub architecture: &'package str,
    pub modified: u64,
}

pub fn build(entries: &[Entry], template: &str, release: Release<'_>) -> Result<Vec<u8>, DebError> {
    let control = Entry {
        destination: "control".to_owned(),
        mode: CONTROL_MODE,
        contents: control(template, release, installed_kibibytes(entries))?.into_bytes(),
    };
    let members = [
        ("debian-binary", FORMAT_VERSION.to_vec()),
        (
            "control.tar.gz",
            tarball::build(&[control], "", release.modified)?,
        ),
        (
            "data.tar.gz",
            tarball::build(entries, "", release.modified)?,
        ),
    ];
    let mut package = ARCHIVE_MAGIC.to_vec();
    for (name, contents) in &members {
        append_member(&mut package, name, contents, release.modified);
    }
    Ok(package)
}

fn control(template: &str, release: Release<'_>, installed: usize) -> Result<String, DebError> {
    let (package, described) = template.split_once('\n').unwrap_or((template, ""));
    if !package.starts_with(PACKAGE_FIELD) {
        return Err(DebError::TemplateWithoutPackage);
    }
    Ok(format!(
        "{package}\nVersion: {}\nArchitecture: {}\nInstalled-Size: {installed}\n{described}",
        release.version, release.architecture
    ))
}

fn installed_kibibytes(entries: &[Entry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.contents.len())
        .sum::<usize>()
        .div_ceil(BYTES_PER_KIBIBYTE)
}

fn append_member(package: &mut Vec<u8>, name: &str, contents: &[u8], modified: u64) {
    let size = contents.len();
    let header = format!(
        "{name:<16}{modified:<12}{ROOT_OWNER:<6}{ROOT_OWNER:<6}{MEMBER_MODE:<8o}{size:<10}`\n"
    );
    package.extend(header.into_bytes());
    package.extend(contents);
    if size % 2 == 1 {
        package.push(b'\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::layout::tests::entries;
    use crate::package::tarball::tests::unpack;

    const HEADER_BYTES: usize = 60;
    const TEMPLATE: &str =
        "Package: tilling\nDepends: libseat1\nDescription: Tiling Wayland compositor\n";
    const RELEASE: Release<'static> = Release {
        version: "0.1.0",
        architecture: "amd64",
        modified: 1_700_000_000,
    };

    fn members(package: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut unread = package.strip_prefix(ARCHIVE_MAGIC).unwrap();
        let mut members = Vec::new();
        while !unread.is_empty() {
            let (header, rest) = unread.split_at(HEADER_BYTES);
            let header = std::str::from_utf8(header).unwrap();
            assert!(header.ends_with("`\n"), "{header:?}");
            let size: usize = header.get(48..58).unwrap().trim().parse().unwrap();
            let padded = size + size % 2;
            members.push((
                header.get(..16).unwrap().trim().to_owned(),
                rest.get(..size).unwrap().to_vec(),
            ));
            unread = rest.get(padded..).unwrap();
        }
        members
    }

    #[test]
    fn archive_holds_the_three_members_dpkg_reads_in_order() {
        let package = build(&entries(), TEMPLATE, RELEASE).unwrap();
        let members = members(&package);
        let names: Vec<&str> = members.iter().map(|(name, _)| name.as_str()).collect();

        assert_eq!(names, ["debian-binary", "control.tar.gz", "data.tar.gz"]);
        assert_eq!(members.first().unwrap().1, b"2.0\n");
    }

    #[test]
    fn control_names_the_version_architecture_and_installed_size() {
        let package = build(&entries(), TEMPLATE, RELEASE).unwrap();
        let members = members(&package);
        let unpacked = unpack(&members.get(1).unwrap().1);
        let control = unpacked.first().unwrap();

        assert_eq!(control.path, "control");
        assert_eq!(
            String::from_utf8_lossy(&control.contents),
            "Package: tilling\nVersion: 0.1.0\nArchitecture: amd64\nInstalled-Size: 1\n\
             Depends: libseat1\nDescription: Tiling Wayland compositor\n"
        );
    }

    #[test]
    fn data_holds_the_installed_tree_from_the_root() {
        let package = build(&entries(), TEMPLATE, RELEASE).unwrap();
        let members = members(&package);
        let unpacked = unpack(&members.get(2).unwrap().1);
        let paths: Vec<&str> = unpacked.iter().map(|entry| entry.path.as_str()).collect();

        assert_eq!(
            paths,
            [
                "usr/",
                "usr/bin/",
                "usr/share/",
                "usr/share/tilling/",
                "usr/bin/tilling",
                "usr/share/tilling/config.toml",
            ]
        );
    }

    #[test]
    fn template_without_the_package_field_is_refused() {
        let outcome = build(&entries(), "Depends: libseat1\n", RELEASE);

        assert!(matches!(outcome, Err(DebError::TemplateWithoutPackage)));
    }

    #[test]
    fn installed_size_rounds_up_to_whole_kibibytes() {
        let mut large = entries();
        large.first_mut().unwrap().contents = vec![0; 2 * BYTES_PER_KIBIBYTE];

        assert_eq!(installed_kibibytes(&entries()), 1);
        assert_eq!(installed_kibibytes(&large), 3);
        assert_eq!(installed_kibibytes(&[]), 0);
    }
}
