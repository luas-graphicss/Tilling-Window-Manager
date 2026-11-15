use std::ffi::OsString;
use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};

use crate::inventory::Inventory;
use crate::operator;
use crate::sandbox::{Sandbox, SandboxError};
use crate::scenario::{Scenario, Step};

const RUNTIME_DIRECTORY_VARIABLE: &str = "XDG_RUNTIME_DIR";
const CARGO_VARIABLE: &str = "CARGO";
const RESULT_PREFIX: &str = "test result:";

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    Sandbox(#[from] SandboxError),
    #[error("start cargo")]
    Cargo(#[source] io::Error),
    #[error("write the report")]
    Report(#[from] io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Passed,
    Incomplete,
    Failed,
}

impl Verdict {
    const fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Incomplete => "incomplete, hardware checks are missing",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepResult {
    Passed { tests: usize },
    NothingRan,
    Failed,
}

pub fn run(
    scenario: Scenario,
    operator: Option<&mut dyn BufRead>,
    report: &mut impl Write,
) -> Result<Verdict, RunError> {
    let sandbox = Sandbox::create(
        std::env::var_os(RUNTIME_DIRECTORY_VARIABLE),
        scenario.name(),
    )?;
    writeln!(report, "scenario {}", scenario.name())?;
    writeln!(report, "  backend: headless")?;
    writeln!(report, "  runtime directory: {}", sandbox.path().display())?;
    let mut verdict = automated(scenario, &sandbox, report)?;
    for behaviour in scenario.needs_host_window() {
        writeln!(
            report,
            "  not observed, needs the nested backend: {behaviour}"
        )?;
    }
    if verdict == Verdict::Passed {
        verdict = on_hardware(scenario, operator, report)?;
    }
    writeln!(report, "result: {}", verdict.label())?;
    Ok(verdict)
}

fn automated(
    scenario: Scenario,
    sandbox: &Sandbox,
    report: &mut impl Write,
) -> Result<Verdict, RunError> {
    let steps = scenario.steps();
    let mut verdict = Verdict::Passed;
    for (index, step) in steps.iter().enumerate() {
        let (result, output) = execute(step, sandbox)?;
        let command = step.arguments().join(" ");
        let position = index + 1;
        let total = steps.len();
        writeln!(
            report,
            "  step {position}/{total}: cargo {command}: {}",
            describe(result)
        )?;
        if !matches!(result, StepResult::Passed { .. }) {
            report.write_all(&output)?;
            verdict = Verdict::Failed;
            break;
        }
    }
    let leftovers = sandbox.leftover_sockets()?;
    for socket in &leftovers {
        writeln!(report, "  socket left behind: {}", socket.display())?;
        verdict = Verdict::Failed;
    }
    Ok(verdict)
}

fn on_hardware(
    scenario: Scenario,
    operator: Option<&mut dyn BufRead>,
    report: &mut impl Write,
) -> Result<Verdict, RunError> {
    let checks = scenario.operator_checks();
    if checks.is_empty() {
        return Ok(Verdict::Passed);
    }
    let Some(answers) = operator else {
        for check in checks {
            writeln!(
                report,
                "  not run, needs an operator on a native session: {}",
                check.expectation
            )?;
        }
        return Ok(Verdict::Incomplete);
    };
    let inventory = Inventory::read();
    for line in inventory.lines() {
        writeln!(report, "  {line}")?;
    }
    if !inventory.has_graphics() {
        writeln!(
            report,
            "  not run, this machine has no graphics device to check"
        )?;
        return Ok(Verdict::Incomplete);
    }
    Ok(operator::review(checks, answers, report)?)
}

fn execute(step: &Step, sandbox: &Sandbox) -> Result<(StepResult, Vec<u8>), RunError> {
    let cargo = std::env::var_os(CARGO_VARIABLE).unwrap_or_else(|| OsString::from("cargo"));
    let mut command = Command::new(cargo);
    command.args(step.arguments()).stdin(Stdio::null());
    sandbox.confine(&mut command);
    let finished = command.output().map_err(RunError::Cargo)?;
    let printed = String::from_utf8_lossy(&finished.stdout);
    let result = match (finished.status.success(), passed_tests(&printed)) {
        (false, _) => StepResult::Failed,
        (true, 0) => StepResult::NothingRan,
        (true, tests) => StepResult::Passed { tests },
    };
    let mut output = finished.stdout;
    output.extend(finished.stderr);
    Ok((result, output))
}

fn describe(result: StepResult) -> String {
    match result {
        StepResult::Passed { tests } => format!("{tests} passed"),
        StepResult::NothingRan => "no test matched".to_owned(),
        StepResult::Failed => "failed".to_owned(),
    }
}

fn passed_tests(printed: &str) -> usize {
    printed
        .lines()
        .filter_map(|line| line.trim().strip_prefix(RESULT_PREFIX))
        .filter_map(|summary| {
            let before = summary.split(" passed").next()?;
            before.split_whitespace().last()?.parse::<usize>().ok()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passed_tests_are_summed_over_every_test_binary() {
        let printed = "\
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.10s

running 12 tests
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
";

        assert_eq!(passed_tests(printed), 15);
    }

    #[test]
    fn filter_that_matches_nothing_counts_no_tests() {
        let printed =
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out\n";

        assert_eq!(passed_tests(printed), 0);
        assert_eq!(passed_tests("error: could not compile"), 0);
    }

    #[test]
    fn missing_hardware_checks_rank_between_success_and_failure() {
        assert!(Verdict::Passed < Verdict::Incomplete);
        assert!(Verdict::Incomplete < Verdict::Failed);
        assert_eq!(
            Verdict::Incomplete.label(),
            "incomplete, hardware checks are missing"
        );
    }

    #[test]
    fn step_results_read_as_a_short_summary() {
        assert_eq!(describe(StepResult::Passed { tests: 7 }), "7 passed");
        assert_eq!(describe(StepResult::NothingRan), "no test matched");
        assert_eq!(describe(StepResult::Failed), "failed");
    }
}
