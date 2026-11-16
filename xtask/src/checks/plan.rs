use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Profile {
    Quick,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
    pub subcommand: &'static str,
    pub package: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub arguments: &'static [&'static str],
    pub environment: &'static [(&'static str, &'static str)],
    pub tool: Option<Tool>,
}

const FORMAT: Step = Step::built_in(&["fmt", "--all", "--check"]);
const LINT: Step = Step::built_in(&[
    "clippy",
    "--workspace",
    "--all-targets",
    "--locked",
    "--",
    "-D",
    "warnings",
]);
const TEST: Step = Step::built_in(&["test", "--workspace", "--locked"]);
const TEST_OPTIMIZED: Step = Step::built_in(&["test", "--workspace", "--release", "--locked"]);
const DOCUMENT: Step = Step {
    arguments: &["doc", "--workspace", "--no-deps", "--locked"],
    environment: &[("RUSTDOCFLAGS", "-D warnings")],
    tool: None,
};
const BUILD_OPTIMIZED: Step = Step::built_in(&["build", "--workspace", "--release", "--locked"]);
const AUDIT: Step = Step {
    arguments: &["audit"],
    environment: &[],
    tool: Some(Tool {
        subcommand: "audit",
        package: "cargo-audit",
        version: "0.23.0",
    }),
};
const DENY: Step = Step {
    arguments: &["deny", "check"],
    environment: &[],
    tool: Some(Tool {
        subcommand: "deny",
        package: "cargo-deny",
        version: "0.20.2",
    }),
};

impl Step {
    const fn built_in(arguments: &'static [&'static str]) -> Self {
        Self {
            arguments,
            environment: &[],
            tool: None,
        }
    }
}

impl Profile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Full => "full",
        }
    }

    pub const fn steps(self) -> &'static [Step] {
        match self {
            Self::Quick => &[FORMAT, LINT, TEST],
            Self::Full => &[
                FORMAT,
                LINT,
                TEST,
                TEST_OPTIMIZED,
                DOCUMENT,
                BUILD_OPTIMIZED,
                AUDIT,
                DENY,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_names_match_the_command_line_values() {
        for profile in Profile::value_variants() {
            let value = profile.to_possible_value().unwrap();

            assert_eq!(value.get_name(), profile.name());
        }
    }

    #[test]
    fn full_profile_starts_with_every_quick_step() {
        let quick = Profile::Quick.steps();

        assert_eq!(Profile::Full.steps().get(..quick.len()), Some(quick));
    }

    #[test]
    fn steps_that_read_the_lockfile_refuse_to_change_it() {
        for step in Profile::Full.steps() {
            let resolves_dependencies =
                step.tool.is_none() && step.arguments.first() != Some(&"fmt");

            assert_eq!(
                step.arguments.contains(&"--locked"),
                resolves_dependencies,
                "{:?}",
                step.arguments
            );
        }
    }

    #[test]
    fn only_the_full_profile_needs_tools_outside_the_toolchain() {
        let pinned: Vec<&str> = Profile::Full
            .steps()
            .iter()
            .filter_map(|step| step.tool)
            .map(|tool| tool.package)
            .collect();

        assert_eq!(pinned, ["cargo-audit", "cargo-deny"]);
        assert!(
            Profile::Quick
                .steps()
                .iter()
                .all(|step| step.tool.is_none())
        );
    }
}
