use super::Scenario;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub action: &'static str,
    pub expectation: &'static str,
    pub disruptive: bool,
}

impl Scenario {
    pub const fn operator_checks(self) -> &'static [Check] {
        match self {
            Self::NestedLifecycle
            | Self::NestedInput
            | Self::ConfigureTimeout
            | Self::IpcBackpressure
            | Self::ConfigReloadAtomicity => &[],
            Self::OutputHotplug => &[
                Check {
                    action: "unplug a monitor that shows windows, then plug it back",
                    expectation: "its windows move to the remaining monitor and return with it",
                    disruptive: true,
                },
                Check {
                    action: "unplug the last monitor, wait ten seconds and plug it back",
                    expectation: "the session idles without a monitor and every window returns",
                    disruptive: true,
                },
            ],
            Self::OutputRollback => &[
                Check {
                    action: "run `tillingctl output set OUTPUT --mode` with another offered mode and do not confirm",
                    expectation: "the previous mode returns after fifteen seconds",
                    disruptive: true,
                },
                Check {
                    action: "run `tillingctl output set OUTPUT --scale 1.5`, then `tillingctl output confirm`",
                    expectation: "the scale is still 1.5 after fifteen seconds",
                    disruptive: false,
                },
            ],
            Self::FractionalScale => &[
                Check {
                    action: "set the scales 1.25, 1.5 and 2 on one output with `tillingctl output set`",
                    expectation: "borders, cursor, popups and clicks line up with the windows at each scale",
                    disruptive: false,
                },
                Check {
                    action: "turn the output with `tillingctl output set OUTPUT --transform 90`",
                    expectation: "the layout fills the turned area and the pointer follows the turned axes",
                    disruptive: false,
                },
            ],
            Self::SeatReactivation => &[
                Check {
                    action: "hold a key, switch to another virtual terminal and come back",
                    expectation: "no key repeats, no input arrives twice and every output lights up again",
                    disruptive: true,
                },
                Check {
                    action: "suspend the machine and resume it",
                    expectation: "outputs and input devices work again without restarting a client",
                    disruptive: true,
                },
                Check {
                    action: "unplug the mouse while dragging a floating window",
                    expectation: "the drag ends and the next pointer device works at once",
                    disruptive: false,
                },
            ],
            Self::RendererFallback => &[
                Check {
                    action: "move the pointer across every output",
                    expectation: "the pointer is drawn on each output, also where no cursor plane exists",
                    disruptive: false,
                },
                Check {
                    action: "make a dmabuf client fullscreen, then cover part of it with a floating window",
                    expectation: "the client is shown both alone and composed under the floating window",
                    disruptive: false,
                },
            ],
            Self::ReleaseJourney => &[
                Check {
                    action: "install the package on a clean machine and start the Tilling session from the display manager",
                    expectation: "the session comes up and `Super+Return` opens the configured terminal",
                    disruptive: true,
                },
                Check {
                    action: "open three windows and go through the layouts with `Super+space`",
                    expectation: "each layout shows the windows without overlap and focus follows `Super+h` and `Super+l`",
                    disruptive: false,
                },
                Check {
                    action: "put a syntax error in the configuration file and press `Super+Shift+c`",
                    expectation: "every binding still works and `tillingctl config reload` names the broken line",
                    disruptive: false,
                },
                Check {
                    action: "press `Super+Shift+e`",
                    expectation: "the session ends, every client exits and the display manager returns",
                    disruptive: true,
                },
                Check {
                    action: "remove the package",
                    expectation: "no file of the package remains and `~/.config/tilling` is untouched",
                    disruptive: false,
                },
            ],
            Self::DesktopIntegration => &[
                Check {
                    action: "start a panel on the top edge, open two windows and make one fullscreen",
                    expectation: "the windows tile below the panel and the fullscreen window covers it",
                    disruptive: false,
                },
                Check {
                    action: "copy text in one application, paste it in another and drag a file between two windows",
                    expectation: "the text arrives and the drop lands on the window under the pointer",
                    disruptive: false,
                },
                Check {
                    action: "type in a text field with an input method started as a helper with `input-method`",
                    expectation: "the composition shows at the text cursor and the committed text arrives once",
                    disruptive: false,
                },
                Check {
                    action: "start a game that locks the pointer and switch to another workspace",
                    expectation: "the pointer moves freely again as soon as the game loses the keyboard",
                    disruptive: false,
                },
            ],
            Self::SessionLockFailures => &[
                Check {
                    action: "run `tillingctl lock` over a fullscreen window with an open menu",
                    expectation: "every output shows only the locker and no key reaches an application",
                    disruptive: false,
                },
                Check {
                    action: "kill the locker from another virtual terminal",
                    expectation: "the session stays covered and a new locker asks for the password",
                    disruptive: true,
                },
                Check {
                    action: "plug a monitor while the session is locked",
                    expectation: "the new monitor shows the locker and never a window",
                    disruptive: true,
                },
                Check {
                    action: "suspend the machine while locked and resume it",
                    expectation: "the locker is on screen before any window can be seen",
                    disruptive: true,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::ValueEnum;

    use super::*;

    #[test]
    fn scenarios_that_need_hardware_name_what_the_operator_must_see() {
        let hardware = [
            Scenario::OutputHotplug,
            Scenario::OutputRollback,
            Scenario::FractionalScale,
            Scenario::SeatReactivation,
            Scenario::RendererFallback,
            Scenario::ReleaseJourney,
            Scenario::DesktopIntegration,
            Scenario::SessionLockFailures,
        ];

        for scenario in Scenario::value_variants() {
            let checks = scenario.operator_checks();

            assert_eq!(
                !checks.is_empty(),
                hardware.contains(scenario),
                "{}",
                scenario.name()
            );
            assert!(checks.iter().all(|check| !check.expectation.is_empty()));
        }
    }

    #[test]
    fn actions_that_blank_outputs_or_leave_the_terminal_are_marked_disruptive() {
        let disruptive = |scenario: Scenario| {
            scenario
                .operator_checks()
                .iter()
                .filter(|check| check.disruptive)
                .count()
        };

        assert_eq!(disruptive(Scenario::OutputHotplug), 2);
        assert_eq!(disruptive(Scenario::SeatReactivation), 2);
        assert_eq!(disruptive(Scenario::OutputRollback), 1);
        assert_eq!(disruptive(Scenario::FractionalScale), 0);
        assert_eq!(disruptive(Scenario::ReleaseJourney), 2);
        assert_eq!(disruptive(Scenario::DesktopIntegration), 0);
        assert_eq!(disruptive(Scenario::SessionLockFailures), 3);
    }
}
