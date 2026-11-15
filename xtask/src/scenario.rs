use clap::ValueEnum;

const COMPOSITOR: &str = "tilling-compositor";
const HEADLESS: Target = Target::Integration("headless");

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Scenario {
    NestedLifecycle,
    NestedInput,
    ConfigureTimeout,
    IpcBackpressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Library,
    Integration(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub package: &'static str,
    pub target: Target,
    pub filters: &'static [&'static str],
}

impl Scenario {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NestedLifecycle => "nested-lifecycle",
            Self::NestedInput => "nested-input",
            Self::ConfigureTimeout => "configure-timeout",
            Self::IpcBackpressure => "ipc-backpressure",
        }
    }

    pub const fn steps(self) -> &'static [Step] {
        match self {
            Self::NestedLifecycle => &[
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["lifecycle::", "popups::", "tiling::", "backends::"],
                },
                Step {
                    package: "tilling",
                    target: Target::Integration("cli"),
                    filters: &[],
                },
            ],
            Self::NestedInput => &[Step {
                package: COMPOSITOR,
                target: HEADLESS,
                filters: &["keyboard::", "pointer::", "frames::"],
            }],
            Self::ConfigureTimeout => &[
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &["transaction::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["transactions::"],
                },
            ],
            Self::IpcBackpressure => &[
                Step {
                    package: "tilling-ipc",
                    target: Target::Integration("transport"),
                    filters: &["backpressure::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["control::"],
                },
                Step {
                    package: "tillingctl",
                    target: Target::Integration("cli"),
                    filters: &[],
                },
            ],
        }
    }

    pub const fn needs_host_window(self) -> &'static [&'static str] {
        match self {
            Self::NestedLifecycle => &[
                "resizing the host window reconfigures the output",
                "closing the host window stops the instance",
                "frames reach the host window through GLES",
            ],
            Self::NestedInput => &[
                "host keyboard, pointer and scroll events reach the seat",
                "the host cursor follows the cursor requested by the client",
            ],
            Self::ConfigureTimeout | Self::IpcBackpressure => &[],
        }
    }
}

impl Step {
    pub fn arguments(&self) -> Vec<&'static str> {
        let mut arguments = vec!["test", "--package", self.package];
        match self.target {
            Target::Library => arguments.push("--lib"),
            Target::Integration(name) => arguments.extend(["--test", name]),
        }
        if !self.filters.is_empty() {
            arguments.push("--");
            arguments.extend(self.filters);
        }
        arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_names_match_the_command_line_values() {
        for scenario in Scenario::value_variants() {
            let value = scenario.to_possible_value().unwrap();

            assert_eq!(value.get_name(), scenario.name());
            assert!(!scenario.steps().is_empty());
        }
    }

    #[test]
    fn step_selects_one_test_target_and_its_filters() {
        let filtered = Step {
            package: "tilling-compositor",
            target: Target::Integration("headless"),
            filters: &["keyboard::", "pointer::"],
        };
        let whole_library = Step {
            package: "tilling-ipc",
            target: Target::Library,
            filters: &[],
        };

        assert_eq!(
            filtered.arguments(),
            [
                "test",
                "--package",
                "tilling-compositor",
                "--test",
                "headless",
                "--",
                "keyboard::",
                "pointer::"
            ]
        );
        assert_eq!(
            whole_library.arguments(),
            ["test", "--package", "tilling-ipc", "--lib"]
        );
    }
}
