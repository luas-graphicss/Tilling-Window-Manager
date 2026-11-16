use std::ffi::OsString;
use std::process::Command;

const CARGO_VARIABLE: &str = "CARGO";

pub fn command() -> Command {
    Command::new(std::env::var_os(CARGO_VARIABLE).unwrap_or_else(|| OsString::from("cargo")))
}
