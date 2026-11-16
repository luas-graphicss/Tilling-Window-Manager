use std::fs;
use std::panic;
use std::path::{Path, PathBuf};

use tilling_fuzz_cases::{configuration, control_stream, desktop};

fn inputs(target: &str) -> Vec<(PathBuf, Vec<u8>)> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus")
        .join(target);
    let mut inputs: Vec<(PathBuf, Vec<u8>)> = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("list {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .map(|path| {
            let bytes =
                fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            (path, bytes)
        })
        .collect();
    inputs.sort();
    assert!(!inputs.is_empty(), "{} has no input", directory.display());
    inputs
}

fn replay(target: &str, check: fn(&[u8])) {
    for (path, input) in inputs(target) {
        let outcome = panic::catch_unwind(|| check(&input));

        assert!(outcome.is_ok(), "{} breaks a property", path.display());
    }
}

#[test]
fn configuration_corpus_breaks_no_property() {
    replay("configuration", configuration::check);
}

#[test]
fn control_stream_corpus_breaks_no_property() {
    replay("control_stream", control_stream::check);
}

#[test]
fn desktop_corpus_breaks_no_property() {
    replay("desktop", desktop::check);
}
