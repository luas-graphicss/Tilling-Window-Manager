mod cargo;
mod checks;
mod inventory;
mod operator;
mod runner;
mod sandbox;
mod scenario;

use std::error::Error;
use std::io::{self, BufRead};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::checks::{CheckError, Profile};
use crate::runner::{RunError, Verdict};
use crate::scenario::Scenario;

const BROKEN_EXIT_CODE: u8 = 2;
const INCOMPLETE_EXIT_CODE: u8 = 3;

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
        #[arg(
            long,
            help = "Ask the operator of a native session to confirm the hardware checks"
        )]
        operator: bool,
    },
    #[command(about = "Run the checks a change has to pass")]
    Check {
        #[arg(
            long,
            value_enum,
            default_value_t = Profile::Quick,
            help = "Quick checks for a change, or every check a release needs"
        )]
        profile: Profile,
    },
}

#[derive(Debug, thiserror::Error)]
enum TaskError {
    #[error(transparent)]
    Scenario(#[from] RunError),
    #[error(transparent)]
    Check(#[from] CheckError),
}

fn main() -> ExitCode {
    match run(Arguments::parse().task) {
        Ok(Verdict::Passed) => ExitCode::SUCCESS,
        Ok(Verdict::Incomplete) => ExitCode::from(INCOMPLETE_EXIT_CODE),
        Ok(Verdict::Failed) => ExitCode::FAILURE,
        Err(failure) => {
            eprintln!("xtask: {}", describe(&failure));
            ExitCode::from(BROKEN_EXIT_CODE)
        }
    }
}

fn run(task: Task) -> Result<Verdict, TaskError> {
    let mut report = io::stdout().lock();
    match task {
        Task::Scenario { name, operator } => {
            let mut answers = io::stdin().lock();
            let operator = operator.then_some(&mut answers as &mut dyn BufRead);
            Ok(runner::run(name, operator, &mut report)?)
        }
        Task::Check { profile } => Ok(checks::run(profile, &mut report)?),
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
