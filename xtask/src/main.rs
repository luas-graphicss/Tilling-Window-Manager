mod runner;
mod sandbox;
mod scenario;

use std::error::Error;
use std::io;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::runner::Verdict;
use crate::scenario::Scenario;

const BROKEN_EXIT_CODE: u8 = 2;

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Development tasks of the Tilling workspace")]
struct Arguments {
    #[command(subcommand)]
    task: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    #[command(about = "Run an integration scenario inside a private runtime directory")]
    Scenario {
        #[arg(value_enum)]
        name: Scenario,
    },
}

fn main() -> ExitCode {
    let Task::Scenario { name } = Arguments::parse().task;
    match runner::run(name, &mut io::stdout().lock()) {
        Ok(Verdict::Passed) => ExitCode::SUCCESS,
        Ok(Verdict::Failed) => ExitCode::FAILURE,
        Err(failure) => {
            eprintln!("xtask: {}", describe(&failure));
            ExitCode::from(BROKEN_EXIT_CODE)
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
