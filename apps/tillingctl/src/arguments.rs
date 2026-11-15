mod outputs;

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use tilling_ipc::endpoint::InstanceName;
use tilling_ipc::{Call, ResourceId};

pub use outputs::OutputAction;

const DEFAULT_TIMEOUT_MILLISECONDS: u64 = 5000;

#[derive(Debug, Parser)]
#[command(
    name = "tillingctl",
    version,
    about = "Control a running Tilling compositor"
)]
pub struct Arguments {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        conflicts_with = "instance",
        help = "Control socket of the instance to reach"
    )]
    pub socket: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        value_name = "NAME",
        help = "Name of the instance to reach"
    )]
    pub instance: Option<InstanceName>,
    #[arg(long, global = true, help = "Print replies and events as JSON")]
    pub json: bool,
    #[arg(
        long = "timeout",
        global = true,
        value_name = "MILLISECONDS",
        default_value_t = DEFAULT_TIMEOUT_MILLISECONDS,
        value_parser = clap::value_parser!(u64).range(1..),
        help = "How long to wait for the compositor"
    )]
    pub timeout_milliseconds: u64,
    #[arg(
        long,
        global = true,
        value_name = "REVISION",
        help = "Apply the command only while the compositor is at this revision"
    )]
    pub if_revision: Option<u64>,
    #[command(subcommand)]
    pub command: Command,
}

impl Arguments {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }
}

#[derive(Debug, Clone, PartialEq, Subcommand)]
pub enum Command {
    #[command(about = "Query the compositor")]
    Get {
        #[arg(value_enum)]
        resource: Resource,
    },
    #[command(about = "Focus a window")]
    Focus {
        #[arg(value_name = "WINDOW", value_parser = parse_resource_id)]
        window: ResourceId,
    },
    #[command(about = "Move a window inside its layout")]
    Move {
        #[arg(value_name = "WINDOW", value_parser = parse_resource_id)]
        window: ResourceId,
        #[arg(value_enum)]
        direction: Direction,
    },
    #[command(about = "Choose the layout of a workspace")]
    Layout {
        #[arg(value_name = "WORKSPACE", value_parser = parse_resource_id)]
        workspace: ResourceId,
        #[arg(value_enum)]
        layout: Layout,
    },
    #[command(about = "Grow or shrink a window by a signed step")]
    Resize {
        #[arg(value_name = "WINDOW", value_parser = parse_resource_id)]
        window: ResourceId,
        #[arg(allow_negative_numbers = true)]
        delta: i16,
    },
    #[command(about = "Tile, float or fullscreen a window")]
    Mode {
        #[arg(value_name = "WINDOW", value_parser = parse_resource_id)]
        window: ResourceId,
        #[arg(value_enum)]
        mode: Mode,
    },
    #[command(about = "Switch to a workspace")]
    Workspace {
        #[arg(value_name = "WORKSPACE", value_parser = parse_resource_id)]
        workspace: ResourceId,
    },
    #[command(about = "Change an output and keep the change once it is confirmed")]
    Output {
        #[command(subcommand)]
        action: OutputAction,
    },
    #[command(about = "Reload the configuration")]
    Reload,
    #[command(about = "Ask the session to lock")]
    Lock,
    #[command(about = "Stop the compositor")]
    Quit,
    #[command(about = "Print a snapshot followed by every event")]
    Subscribe,
    #[command(about = "List the running instances")]
    Instances,
}

impl Command {
    pub fn call(&self) -> Option<Call> {
        let call = match *self {
            Self::Get { resource } => resource.query(),
            Self::Focus { window } => Call::FocusWindow { window },
            Self::Move { window, direction } => Call::MoveWindow {
                window,
                direction: direction.into(),
            },
            Self::Layout { workspace, layout } => Call::SetLayout {
                workspace,
                layout: layout.into(),
            },
            Self::Resize { window, delta } => Call::ResizeWindow { window, delta },
            Self::Mode { window, mode } => Call::SetWindowMode {
                window,
                mode: mode.into(),
            },
            Self::Workspace { workspace } => Call::SwitchWorkspace { workspace },
            Self::Output { action } => action.call(),
            Self::Reload => Call::ReloadConfig,
            Self::Lock => Call::RequestLock,
            Self::Quit => Call::Quit,
            Self::Subscribe => Call::Subscribe,
            Self::Instances => return None,
        };
        Some(call)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Resource {
    Version,
    Outputs,
    Workspaces,
    Windows,
    Config,
    Diagnostics,
}

impl Resource {
    fn query(self) -> Call {
        match self {
            Self::Version => Call::GetVersion,
            Self::Outputs => Call::GetOutputs,
            Self::Workspaces => Call::GetWorkspaces,
            Self::Windows => Call::GetWindows,
            Self::Config => Call::GetConfig,
            Self::Diagnostics => Call::GetDiagnostics,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl From<Direction> for tilling_ipc::Direction {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Left => Self::Left,
            Direction::Right => Self::Right,
            Direction::Up => Self::Up,
            Direction::Down => Self::Down,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Layout {
    Bsp,
    MasterStack,
    Monocle,
}

impl From<Layout> for tilling_ipc::Layout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Bsp => Self::Bsp,
            Layout::MasterStack => Self::MasterStack,
            Layout::Monocle => Self::Monocle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    Tiled,
    Floating,
    Fullscreen,
}

impl From<Mode> for tilling_ipc::WindowMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Tiled => Self::Tiled,
            Mode::Floating => Self::Floating,
            Mode::Fullscreen => Self::Fullscreen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected INDEX:GENERATION, for example 3:1")]
pub struct InvalidResourceId;

fn parse_resource_id(text: &str) -> Result<ResourceId, InvalidResourceId> {
    let (index, generation) = text.split_once(':').ok_or(InvalidResourceId)?;
    Ok(ResourceId {
        index: index.parse().map_err(|_| InvalidResourceId)?,
        generation: generation.parse().map_err(|_| InvalidResourceId)?,
    })
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::*;

    const WINDOW: ResourceId = ResourceId {
        index: 3,
        generation: 1,
    };

    fn parse(line: &[&str]) -> Result<Arguments, clap::Error> {
        Arguments::try_parse_from([&["tillingctl"], line].concat())
    }

    fn call_of(line: &[&str]) -> Option<Call> {
        parse(line).unwrap().command.call()
    }

    #[test]
    fn explicit_socket_and_query_follow_the_documented_form() {
        let arguments = parse(&[
            "--socket",
            "/run/user/1000/tilling/test/ipc.sock",
            "get",
            "workspaces",
        ])
        .unwrap();

        assert_eq!(
            arguments.socket,
            Some(PathBuf::from("/run/user/1000/tilling/test/ipc.sock"))
        );
        assert_eq!(arguments.command.call(), Some(Call::GetWorkspaces));
        assert_eq!(arguments.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn control_commands_carry_their_typed_parameters() {
        assert_eq!(
            call_of(&["move", "3:1", "left"]),
            Some(Call::MoveWindow {
                window: WINDOW,
                direction: tilling_ipc::Direction::Left,
            })
        );
        assert_eq!(
            call_of(&["layout", "0:0", "master-stack"]),
            Some(Call::SetLayout {
                workspace: ResourceId {
                    index: 0,
                    generation: 0,
                },
                layout: tilling_ipc::Layout::MasterStack,
            })
        );
        assert_eq!(
            call_of(&["mode", "3:1", "fullscreen"]),
            Some(Call::SetWindowMode {
                window: WINDOW,
                mode: tilling_ipc::WindowMode::Fullscreen,
            })
        );
    }

    #[test]
    fn resize_accepts_a_negative_step() {
        assert_eq!(
            call_of(&["resize", "3:1", "-40"]),
            Some(Call::ResizeWindow {
                window: WINDOW,
                delta: -40,
            })
        );
    }

    #[test]
    fn global_options_may_follow_the_command() {
        let arguments =
            parse(&["quit", "--if-revision", "12", "--json", "--timeout", "250"]).unwrap();

        assert_eq!(arguments.if_revision, Some(12));
        assert!(arguments.json);
        assert_eq!(arguments.timeout(), Duration::from_millis(250));
    }

    #[test]
    fn listing_instances_needs_no_request() {
        assert_eq!(call_of(&["instances"]), None);
    }

    #[test]
    fn malformed_identifiers_are_rejected() {
        for identifier in ["3", "3:", ":1", "3:1:0", "a:b", "4294967296:0"] {
            let failure = parse(&["focus", identifier]).unwrap_err();

            assert_eq!(failure.kind(), ErrorKind::ValueValidation, "{identifier}");
        }
    }

    #[test]
    fn socket_and_instance_cannot_be_combined() {
        let failure =
            parse(&["--socket", "/tmp/ipc.sock", "--instance", "main", "quit"]).unwrap_err();

        assert_eq!(failure.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn instance_names_and_timeouts_are_validated() {
        let bad_instance = parse(&["--instance", "../etc", "quit"]).unwrap_err();
        let no_patience = parse(&["--timeout", "0", "quit"]).unwrap_err();

        assert_eq!(bad_instance.kind(), ErrorKind::ValueValidation);
        assert_eq!(no_patience.kind(), ErrorKind::ValueValidation);
    }
}
