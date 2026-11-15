mod arguments;
mod commands;
mod output;
mod target;

use std::error::Error;
use std::process::ExitCode;

use clap::Parser;

use crate::arguments::Arguments;
use crate::target::Environment;

const FAILURE_EXIT_CODE: u8 = 3;

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    match commands::run(&arguments, &Environment::capture()) {
        Ok(status) => status.into(),
        Err(failure) => {
            eprintln!("tillingctl: {}", describe(&failure));
            ExitCode::from(FAILURE_EXIT_CODE)
        }
    }
}

fn describe(failure: &dyn Error) -> String {
    let mut description = failure.to_string();
    let mut cause = failure.source();
    while let Some(source) = cause {
        description.push_str(": ");
        description.push_str(&source.to_string());
        cause = source.source();
    }
    description
}
