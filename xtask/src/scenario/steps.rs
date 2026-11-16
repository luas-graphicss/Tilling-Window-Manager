use super::{Scenario, Step, Target};

const COMPOSITOR: &str = "tilling-compositor";
const HEADLESS: Target = Target::Integration("headless");

impl Scenario {
    pub const fn steps(self) -> &'static [Step] {
        match self {
            Self::NestedLifecycle => &[
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &[
                        "globals::",
                        "lifecycle::",
                        "subsurfaces::",
                        "popups::",
                        "tiling::",
                        "backends::",
                    ],
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
            Self::OutputHotplug => &[
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &["output::profile::", "output::placement::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["outputs::"],
                },
            ],
            Self::OutputRollback => &[
                Step {
                    package: "tilling-ipc",
                    target: Target::Integration("wire_format"),
                    filters: &["output_change"],
                },
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &["output::settings::", "control::outputs::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["output_changes::"],
                },
            ],
            Self::FractionalScale => &[
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &["output::placement::", "geometry::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["scale::"],
                },
            ],
            Self::SeatReactivation => &[
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &[
                        "backend::drm::recovery::",
                        "backend::drm::frame_clock::",
                        "input::keyboard::",
                    ],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["keyboard::"],
                },
            ],
            Self::RendererFallback => &[Step {
                package: COMPOSITOR,
                target: Target::Library,
                filters: &[
                    "render::pointer_sprite::",
                    "backend::drm::recovery::",
                    "backend::drm::frame_clock::",
                ],
            }],
            Self::ReleaseJourney => &[
                Step {
                    package: "tilling",
                    target: Target::Integration("cli"),
                    filters: &[],
                },
                Step {
                    package: "tillingctl",
                    target: Target::Integration("cli"),
                    filters: &[],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &[
                        "lifecycle::",
                        "tiling::",
                        "layouts::",
                        "scratchpad::",
                        "control::",
                    ],
                },
            ],
            Self::DesktopIntegration => &[
                Step {
                    package: "tilling-core",
                    target: Target::Integration("domain"),
                    filters: &["outputs::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &["helpers::", "security::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &[
                        "globals::",
                        "layers::",
                        "selection::",
                        "activation::",
                        "idle::",
                        "pointer_lock::",
                        "privileges::",
                        "helpers::",
                    ],
                },
            ],
            Self::SessionLockFailures => &[
                Step {
                    package: COMPOSITOR,
                    target: Target::Library,
                    filters: &["helpers::supervisor::"],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["lock::", "locker::"],
                },
            ],
            Self::ConfigReloadAtomicity => &[
                Step {
                    package: "tilling-config",
                    target: Target::Integration("validation"),
                    filters: &[],
                },
                Step {
                    package: "tilling-config",
                    target: Target::Integration("session"),
                    filters: &[],
                },
                Step {
                    package: COMPOSITOR,
                    target: HEADLESS,
                    filters: &["reload::", "helpers::reloading"],
                },
            ],
        }
    }
}
