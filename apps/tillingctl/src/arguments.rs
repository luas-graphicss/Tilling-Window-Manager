mod outputs;
mod workspaces;

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use tilling_ipc::endpoint::InstanceName;
use tilling_ipc::{Call, ResourceId};

use crate::topics::{Topic, TopicFilter};

pub use outputs::OutputAction;
pub use workspaces::{LayoutAction, WorkspaceAction, WorkspaceCall, WorkspaceReference};

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
    #[command(about = "Choose how a workspace arranges its windows")]
    Layout {
        #[command(subcommand)]
        action: LayoutAction,
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
    #[command(about = "Show another workspace")]
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    #[command(about = "Change an output and keep the change once it is confirmed")]
    Output {
        #[command(subcommand)]
        action: OutputAction,
    },
    #[command(about = "Manage the configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    #[command(about = "Ask the session to lock")]
    Lock,
    #[command(about = "Stop the compositor")]
    Quit,
    #[command(about = "Print a snapshot followed by every event")]
    Events {
        #[arg(
            long,
            value_enum,
            value_delimiter = ',',
            value_name = "TOPICS",
            help = "Only print these topics, separated by commas"
        )]
        topics: Vec<Topic>,
    },
    #[command(about = "List the running instances")]
    Instances,
    #[command(about = "Print the completion script of a shell")]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ConfigAction {
    #[command(about = "Reload the configuration")]
    Reload,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    ListInstances,
    PrintCompletions(Shell),
    Call(Call),
    CallOnWorkspace(WorkspaceReference, WorkspaceCall),
    FollowEvents(TopicFilter),
}

impl Command {
    pub fn operation(&self) -> Operation {
        match self {
            Self::Get { resource } => Operation::Call(resource.query()),
            Self::Focus { window } => Operation::Call(Call::FocusWindow { window: *window }),
            Self::Move { window, direction } => Operation::Call(Call::MoveWindow {
                window: *window,
                direction: (*direction).into(),
            }),
            Self::Layout {
                action: LayoutAction::Set { layout, workspace },
            } => Operation::CallOnWorkspace(
                workspace.clone().unwrap_or(WorkspaceReference::Active),
                WorkspaceCall::SetLayout((*layout).into()),
            ),
            Self::Resize { window, delta } => Operation::Call(Call::ResizeWindow {
                window: *window,
                delta: *delta,
            }),
            Self::Mode { window, mode } => Operation::Call(Call::SetWindowMode {
                window: *window,
                mode: (*mode).into(),
            }),
            Self::Workspace {
                action: WorkspaceAction::Switch { workspace },
            } => Operation::CallOnWorkspace(workspace.clone(), WorkspaceCall::Switch),
            Self::Output { action } => Operation::Call(action.call()),
            Self::Config {
                action: ConfigAction::Reload,
            } => Operation::Call(Call::ReloadConfig),
            Self::Lock => Operation::Call(Call::RequestLock),
            Self::Quit => Operation::Call(Call::Quit),
            Self::Events { topics } => Operation::FollowEvents(TopicFilter::new(topics)),
            Self::Instances => Operation::ListInstances,
            Self::Completions { shell } => Operation::PrintCompletions(*shell),
        }
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

    fn operation_of(line: &[&str]) -> Operation {
        parse(line).unwrap().command.operation()
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
        assert_eq!(
            arguments.command.operation(),
            Operation::Call(Call::GetWorkspaces)
        );
        assert_eq!(arguments.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn control_commands_carry_their_typed_parameters() {
        assert_eq!(
            operation_of(&["move", "3:1", "left"]),
            Operation::Call(Call::MoveWindow {
                window: WINDOW,
                direction: tilling_ipc::Direction::Left,
            })
        );
        assert_eq!(
            operation_of(&["mode", "3:1", "fullscreen"]),
            Operation::Call(Call::SetWindowMode {
                window: WINDOW,
                mode: tilling_ipc::WindowMode::Fullscreen,
            })
        );
        assert_eq!(
            operation_of(&["config", "reload"]),
            Operation::Call(Call::ReloadConfig)
        );
    }

    #[test]
    fn layout_targets_the_active_workspace_unless_one_is_named() {
        let master_stack = WorkspaceCall::SetLayout(tilling_ipc::Layout::MasterStack);

        assert_eq!(
            operation_of(&["layout", "set", "master-stack"]),
            Operation::CallOnWorkspace(WorkspaceReference::Active, master_stack)
        );
        assert_eq!(
            operation_of(&["layout", "set", "master-stack", "--workspace", "web"]),
            Operation::CallOnWorkspace(WorkspaceReference::Named("web".to_owned()), master_stack)
        );
    }

    #[test]
    fn workspace_switch_takes_a_name_or_an_identifier() {
        assert_eq!(
            operation_of(&["workspace", "switch", "2"]),
            Operation::CallOnWorkspace(
                WorkspaceReference::Named("2".to_owned()),
                WorkspaceCall::Switch
            )
        );
        assert_eq!(
            operation_of(&["workspace", "switch", "3:1"]),
            Operation::CallOnWorkspace(
                WorkspaceReference::Identifier(WINDOW),
                WorkspaceCall::Switch
            )
        );
    }

    #[test]
    fn events_take_a_list_of_topics() {
        assert_eq!(
            operation_of(&["events"]),
            Operation::FollowEvents(TopicFilter::default())
        );
        assert_eq!(
            operation_of(&["events", "--topics", "windows,outputs"]),
            Operation::FollowEvents(TopicFilter::new(&[Topic::Windows, Topic::Outputs]))
        );
        assert_eq!(
            parse(&["events", "--topics", "pointer"])
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn resize_accepts_a_negative_step() {
        assert_eq!(
            operation_of(&["resize", "3:1", "-40"]),
            Operation::Call(Call::ResizeWindow {
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
        assert_eq!(operation_of(&["instances"]), Operation::ListInstances);
    }

    #[test]
    fn completions_name_the_shell_and_need_no_request() {
        assert_eq!(
            operation_of(&["completions", "zsh"]),
            Operation::PrintCompletions(Shell::Zsh)
        );
        assert_eq!(
            parse(&["completions", "tcsh"]).unwrap_err().kind(),
            ErrorKind::InvalidValue
        );
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
