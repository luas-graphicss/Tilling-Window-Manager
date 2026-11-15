mod arguments;
mod launch;
mod logging;

use std::error::Error;
use std::process::ExitCode;

use clap::Parser;

use crate::arguments::Arguments;
use crate::launch::Environment;

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    if let Err(failure) = logging::start(arguments.verbose) {
        eprintln!("tilling: start logging: {failure}");
    }
    match launch::run(arguments, &Environment::capture()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("tilling: {}", describe(&failure));
            ExitCode::FAILURE
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
