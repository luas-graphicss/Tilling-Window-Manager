mod plan;

use std::io::{self, Write};
use std::process::Stdio;

pub use plan::Profile;
use plan::{Step, Tool};

use crate::cargo;
use crate::runner::Verdict;
use crate::sandbox::{Sandbox, SandboxError};

const RUNTIME_DIRECTORY_VARIABLE: &str = "XDG_RUNTIME_DIR";
const SANDBOX_LABEL: &str = "check";

#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error(transparent)]
    Sandbox(#[from] SandboxError),
    #[error("start cargo")]
    Cargo(#[source] io::Error),
    #[error("write the report")]
    Report(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StepResult {
    Passed,
    Failed { output: Vec<u8> },
    ToolMissing(Tool),
    ToolMismatch { tool: Tool, installed: String },
}

pub fn run(profile: Profile, report: &mut impl Write) -> Result<Verdict, CheckError> {
    let sandbox = Sandbox::create(std::env::var_os(RUNTIME_DIRECTORY_VARIABLE), SANDBOX_LABEL)?;
    writeln!(report, "checks {}", profile.name())?;
    let mut verdict = Verdict::Passed;
    for step in profile.steps() {
        write!(report, "  cargo {}: ", step.arguments.join(" "))?;
        report.flush()?;
        let result = execute(step, &sandbox)?;
        writeln!(report, "{}", describe(&result))?;
        if let StepResult::Failed { output } = &result {
            report.write_all(output)?;
        }
        verdict = verdict.max(rank(&result));
    }
    for socket in sandbox.leftover_sockets()? {
        writeln!(report, "  socket left behind: {}", socket.display())?;
        verdict = Verdict::Failed;
    }
    writeln!(report, "result: {}", summarize(verdict))?;
    Ok(verdict)
}

fn execute(step: &Step, sandbox: &Sandbox) -> Result<StepResult, CheckError> {
    if let Some(tool) = step.tool {
        match installed_version(tool)? {
            None => return Ok(StepResult::ToolMissing(tool)),
            Some(installed) if installed != tool.version => {
                return Ok(StepResult::ToolMismatch { tool, installed });
            }
            Some(_) => {}
        }
    }
    let mut command = cargo::command();
    command
        .args(step.arguments)
        .envs(step.environment.iter().copied())
        .stdin(Stdio::null());
    sandbox.confine(&mut command);
    let finished = command.output().map_err(CheckError::Cargo)?;
    if finished.status.success() {
        return Ok(StepResult::Passed);
    }
    let mut output = finished.stdout;
    output.extend(finished.stderr);
    Ok(StepResult::Failed { output })
}

fn installed_version(tool: Tool) -> Result<Option<String>, CheckError> {
    let finished = cargo::command()
        .args([tool.subcommand, "--version"])
        .stdin(Stdio::null())
        .output()
        .map_err(CheckError::Cargo)?;
    if !finished.status.success() {
        return Ok(None);
    }
    Ok(reported_version(&String::from_utf8_lossy(&finished.stdout)))
}

fn reported_version(printed: &str) -> Option<String> {
    printed.split_whitespace().last().map(str::to_owned)
}

fn describe(result: &StepResult) -> String {
    match result {
        StepResult::Passed => "passed".to_owned(),
        StepResult::Failed { .. } => "failed".to_owned(),
        StepResult::ToolMissing(tool) => format!(
            "not run, install it with `cargo install {} --version {} --locked`",
            tool.package, tool.version
        ),
        StepResult::ToolMismatch { tool, installed } => format!(
            "not run, {} {installed} is installed and {} is pinned",
            tool.package, tool.version
        ),
    }
}

const fn rank(result: &StepResult) -> Verdict {
    match result {
        StepResult::Passed => Verdict::Passed,
        StepResult::ToolMissing(_) | StepResult::ToolMismatch { .. } => Verdict::Incomplete,
        StepResult::Failed { .. } => Verdict::Failed,
    }
}

const fn summarize(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Passed => "passed",
        Verdict::Incomplete => "incomplete, a pinned tool did not run",
        Verdict::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DENY: Tool = Tool {
        subcommand: "deny",
        package: "cargo-deny",
        version: "0.20.2",
    };

    #[test]
    fn version_is_the_last_word_a_tool_prints() {
        assert_eq!(
            reported_version("cargo-deny 0.20.2\n").as_deref(),
            Some("0.20.2")
        );
        assert_eq!(
            reported_version("cargo-audit-audit 0.23.0\n").as_deref(),
            Some("0.23.0")
        );
        assert_eq!(reported_version("\n"), None);
    }

    #[test]
    fn missing_tool_says_how_to_install_the_pinned_version() {
        assert_eq!(
            describe(&StepResult::ToolMissing(DENY)),
            "not run, install it with `cargo install cargo-deny --version 0.20.2 --locked`"
        );
    }

    #[test]
    fn other_version_of_a_tool_is_named_beside_the_pinned_one() {
        let mismatch = StepResult::ToolMismatch {
            tool: DENY,
            installed: "0.18.3".to_owned(),
        };

        assert_eq!(
            describe(&mismatch),
            "not run, cargo-deny 0.18.3 is installed and 0.20.2 is pinned"
        );
    }

    #[test]
    fn step_that_did_not_run_leaves_the_checks_incomplete() {
        let failed = StepResult::Failed { output: Vec::new() };

        assert_eq!(rank(&StepResult::Passed), Verdict::Passed);
        assert_eq!(rank(&StepResult::ToolMissing(DENY)), Verdict::Incomplete);
        assert_eq!(rank(&failed), Verdict::Failed);
        assert_eq!(
            summarize(Verdict::Incomplete),
            "incomplete, a pinned tool did not run"
        );
    }
}
