use std::io::{self, BufRead, Write};

use crate::runner::Verdict;
use crate::scenario::Check;

const AFFIRMATIVE: [&str; 2] = ["y", "yes"];
const DISRUPTION_WARNING: &str =
    "this can blank outputs, leave the terminal or suspend the machine, continue?";

pub fn review(
    checks: &[Check],
    answers: &mut dyn BufRead,
    report: &mut impl Write,
) -> io::Result<Verdict> {
    let mut verdict = Verdict::Passed;
    let total = checks.len();
    for (index, check) in checks.iter().enumerate() {
        let position = index + 1;
        writeln!(report, "  check {position}/{total}: {}", check.action)?;
        if check.disruptive && !agreed(DISRUPTION_WARNING, answers, report)? {
            writeln!(report, "    skipped by the operator")?;
            verdict = verdict.max(Verdict::Incomplete);
            continue;
        }
        let question = format!("observed: {}?", check.expectation);
        if agreed(&question, answers, report)? {
            writeln!(report, "    confirmed by the operator")?;
        } else {
            writeln!(report, "    not observed")?;
            verdict = Verdict::Failed;
        }
    }
    Ok(verdict)
}

fn agreed(question: &str, answers: &mut dyn BufRead, report: &mut impl Write) -> io::Result<bool> {
    writeln!(report, "    {question} [y/N]")?;
    report.flush()?;
    let mut answer = String::new();
    answers.read_line(&mut answer)?;
    let answer = answer.trim().to_ascii_lowercase();
    Ok(AFFIRMATIVE.contains(&answer.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECKS: [Check; 2] = [
        Check {
            action: "unplug the last monitor and plug it back",
            expectation: "every window returns",
            disruptive: true,
        },
        Check {
            action: "move the pointer across every output",
            expectation: "the pointer is drawn on each output",
            disruptive: false,
        },
    ];

    fn reviewed(answers: &str) -> (Verdict, String) {
        let mut report = Vec::new();
        let verdict = review(&CHECKS, &mut answers.as_bytes(), &mut report).unwrap();
        (verdict, String::from_utf8(report).unwrap())
    }

    #[test]
    fn checks_confirmed_by_the_operator_pass() {
        let (verdict, report) = reviewed("y\nyes\nY\n");

        assert_eq!(verdict, Verdict::Passed);
        assert_eq!(report.matches("confirmed by the operator").count(), 2);
    }

    #[test]
    fn declined_disruptive_check_leaves_the_scenario_incomplete() {
        let (verdict, report) = reviewed("n\ny\n");

        assert_eq!(verdict, Verdict::Incomplete);
        assert!(report.contains("check 1/2: unplug the last monitor and plug it back"));
        assert!(report.contains("skipped by the operator"));
        assert!(report.contains("confirmed by the operator"));
    }

    #[test]
    fn behaviour_that_was_not_observed_fails_the_scenario() {
        let (verdict, report) = reviewed("y\nno\ny\n");

        assert_eq!(verdict, Verdict::Failed);
        assert!(report.contains("observed: every window returns? [y/N]"));
        assert!(report.contains("not observed"));
    }

    #[test]
    fn missing_answers_never_count_as_confirmation() {
        let (verdict, report) = reviewed("");

        assert_eq!(verdict, Verdict::Failed);
        assert!(!report.contains("confirmed by the operator"));
    }
}
