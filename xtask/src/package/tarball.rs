use std::collections::BTreeSet;
use std::io;

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{Builder, EntryType, Header};

use super::layout::Entry;

const DIRECTORY_MODE: u32 = 0o755;
const ROOT_OWNER: u64 = 0;

pub fn build(entries: &[Entry], prefix: &str, modified: u64) -> io::Result<Vec<u8>> {
    let mut archive = Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    let paths: Vec<String> = entries
        .iter()
        .map(|entry| join(prefix, &entry.destination))
        .collect();
    for directory in directories(&paths) {
        let mut header = header(EntryType::Directory, DIRECTORY_MODE, 0, modified);
        archive.append_data(&mut header, format!("{directory}/"), io::empty())?;
    }
    for (entry, path) in entries.iter().zip(&paths) {
        let size = u64::try_from(entry.contents.len()).map_err(io::Error::other)?;
        let mut header = header(EntryType::Regular, entry.mode, size, modified);
        archive.append_data(&mut header, path, entry.contents.as_slice())?;
    }
    archive.into_inner()?.finish()
}

fn header(kind: EntryType, mode: u32, size: u64, modified: u64) -> Header {
    let mut header = Header::new_gnu();
    header.set_entry_type(kind);
    header.set_mode(mode);
    header.set_size(size);
    header.set_mtime(modified);
    header.set_uid(ROOT_OWNER);
    header.set_gid(ROOT_OWNER);
    header
}

fn join(prefix: &str, destination: &str) -> String {
    if prefix.is_empty() {
        return destination.to_owned();
    }
    format!("{prefix}/{destination}")
}

fn directories(paths: &[String]) -> BTreeSet<&str> {
    paths
        .iter()
        .flat_map(|path| {
            path.match_indices('/')
                .filter_map(|(end, _)| path.get(..end))
        })
        .collect()
}

#[cfg(test)]
pub(super) mod tests {
    use std::io::Read;

    use flate2::read::GzDecoder;
    use tar::Archive;

    use super::*;
    use crate::package::layout::tests::entries;

    pub(in crate::package) struct Unpacked {
        pub path: String,
        pub mode: u32,
        pub modified: u64,
        pub owner: u64,
        pub contents: Vec<u8>,
    }

    pub(in crate::package) fn unpack(compressed: &[u8]) -> Vec<Unpacked> {
        let mut archive = Archive::new(GzDecoder::new(compressed));
        archive
            .entries()
            .unwrap()
            .map(|entry| {
                let mut entry = entry.unwrap();
                let mut contents = Vec::new();
                entry.read_to_end(&mut contents).unwrap();
                Unpacked {
                    path: entry.path().unwrap().to_string_lossy().into_owned(),
                    mode: entry.header().mode().unwrap(),
                    modified: entry.header().mtime().unwrap(),
                    owner: entry.header().uid().unwrap(),
                    contents,
                }
            })
            .collect()
    }

    #[test]
    fn directories_come_before_the_files_they_hold() {
        let unpacked = unpack(&build(&entries(), "tilling-0.1.0", 1_700_000_000).unwrap());
        let paths: Vec<&str> = unpacked.iter().map(|entry| entry.path.as_str()).collect();

        assert_eq!(
            paths,
            [
                "tilling-0.1.0/",
                "tilling-0.1.0/usr/",
                "tilling-0.1.0/usr/bin/",
                "tilling-0.1.0/usr/share/",
                "tilling-0.1.0/usr/share/tilling/",
                "tilling-0.1.0/usr/bin/tilling",
                "tilling-0.1.0/usr/share/tilling/config.toml",
            ]
        );
    }

    #[test]
    fn files_keep_their_mode_and_contents_and_belong_to_root() {
        let unpacked = unpack(&build(&entries(), "", 1_700_000_000).unwrap());
        let binary = unpacked
            .iter()
            .find(|entry| entry.path == "usr/bin/tilling")
            .unwrap();
        let directory = unpacked.iter().find(|entry| entry.path == "usr/").unwrap();

        assert_eq!(binary.mode, 0o755);
        assert_eq!(binary.contents, b"\x7fELF compositor");
        assert_eq!(binary.owner, 0);
        assert_eq!(binary.modified, 1_700_000_000);
        assert_eq!(directory.mode, 0o755);
        assert!(directory.contents.is_empty());
    }

    #[test]
    fn same_input_and_time_give_the_same_bytes() {
        let first = build(&entries(), "tilling-0.1.0", 42).unwrap();
        let second = build(&entries(), "tilling-0.1.0", 42).unwrap();

        assert_eq!(first, second);
    }
}
