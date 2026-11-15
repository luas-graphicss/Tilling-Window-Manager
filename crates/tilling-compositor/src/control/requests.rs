use tilling_config::ConfigSource;
use tilling_core::layout::Direction;
use tilling_core::{Command, ModeChange, WindowId, WindowMode};
use tilling_ipc::{
    Call, Capability, ConfigInfo, Layout, Outcome, PROTOCOL_VERSION, Reply, Request, ResourceId,
    Response, Snapshot, VersionInfo,
};

use super::peers::PeerId;
use super::refusal::Refusal;
use super::{resources, views};
use crate::runtime::Compositor;

const LOCK_UNAVAILABLE: &str = "session locking is not available";
const EVENTS_NOT_NEGOTIATED: &str = "the events capability was not negotiated";
const LAYOUT_UNAVAILABLE: &str = "only the bsp layout is available";
const NOWHERE_TO_FLOAT: &str = "window is not on a workspace shown by an output";

impl Compositor {
    pub(super) fn answer(&mut self, peer: PeerId, request: Request) -> Response {
        let current_revision = self.desktop.revision();
        let outcome = match request.expected_revision {
            Some(expected) if expected != current_revision => {
                Outcome::RevisionConflict { current_revision }
            }
            _ => match self.perform_call(peer, request.call) {
                Ok(result) => Outcome::Success { result },
                Err(refusal) => refusal.into(),
            },
        };
        Response {
            request_id: request.request_id,
            outcome,
        }
    }

    fn perform_call(&mut self, peer: PeerId, call: Call) -> Result<Reply, Refusal> {
        match call {
            Call::GetVersion => Ok(Reply::Version(VersionInfo {
                compositor: env!("CARGO_PKG_VERSION").to_owned(),
                protocol: PROTOCOL_VERSION,
            })),
            Call::GetOutputs => Ok(Reply::Outputs(views::outputs(&self.desktop))),
            Call::GetWorkspaces => Ok(Reply::Workspaces(views::workspaces(&self.desktop))),
            Call::GetWindows => Ok(Reply::Windows(views::windows(&self.desktop))),
            Call::GetConfig => Ok(Reply::Config(self.config_info())),
            Call::GetDiagnostics => Ok(Reply::Diagnostics(views::diagnostics(&self.desktop))),
            Call::FocusWindow { window } => {
                let window = resources::window(&self.desktop, window)?;
                self.accept(Command::FocusWindow { window })
            }
            Call::MoveWindow { window, direction } => {
                let window = resources::window(&self.desktop, window)?;
                let direction = core_direction(direction);
                self.accept(Command::MoveWindow { window, direction })
            }
            Call::SetLayout { workspace, layout } => self.set_layout(workspace, layout),
            Call::ResizeWindow { window, delta } => {
                let window = resources::window(&self.desktop, window)?;
                self.accept(Command::ResizeSplit { window, delta })
            }
            Call::SetWindowMode { window, mode } => self.set_window_mode(window, mode),
            Call::SwitchWorkspace { workspace } => {
                let workspace = resources::workspace(&self.desktop, workspace)?;
                self.accept(Command::SwitchWorkspace { workspace })
            }
            Call::ConfigureOutput { output, change } => self.configure_output(output, change),
            Call::ConfirmOutputChange => self.confirm_output(),
            Call::RevertOutputChange => self.revert_output(),
            Call::ReloadConfig => {
                self.reload()
                    .map_err(|error| Refusal::invalid_because(&error))?;
                Ok(self.accepted())
            }
            Call::RequestLock => Err(Refusal::NotPermitted(LOCK_UNAVAILABLE.to_owned())),
            Call::Quit => {
                self.stop();
                Ok(self.accepted())
            }
            Call::Subscribe => self.subscribe(peer),
        }
    }

    fn accept(&mut self, command: Command) -> Result<Reply, Refusal> {
        self.execute(command)?;
        Ok(self.accepted())
    }

    pub(super) fn accepted(&self) -> Reply {
        Reply::Accepted {
            revision: self.desktop.revision(),
        }
    }

    fn config_info(&self) -> ConfigInfo {
        let path = match &self.config_source {
            ConfigSource::File(path) => Some(path.display().to_string()),
            ConfigSource::BuiltIn => None,
        };
        ConfigInfo {
            revision: self.config_revision,
            path,
        }
    }

    fn set_layout(&self, workspace: ResourceId, layout: Layout) -> Result<Reply, Refusal> {
        resources::workspace(&self.desktop, workspace)?;
        match layout {
            Layout::Bsp => Ok(self.accepted()),
            Layout::MasterStack | Layout::Monocle => {
                Err(Refusal::Invalid(LAYOUT_UNAVAILABLE.to_owned()))
            }
        }
    }

    fn set_window_mode(
        &mut self,
        window: ResourceId,
        mode: tilling_ipc::WindowMode,
    ) -> Result<Reply, Refusal> {
        let window = resources::window(&self.desktop, window)?;
        for change in self.mode_changes(window, mode)? {
            self.execute(Command::ChangeWindowMode { window, change })?;
        }
        Ok(self.accepted())
    }

    fn mode_changes(
        &self,
        window: WindowId,
        requested: tilling_ipc::WindowMode,
    ) -> Result<Vec<ModeChange>, Refusal> {
        let current = self
            .desktop
            .window(window)
            .map(|state| state.mode())
            .ok_or_else(|| Refusal::NotFound(format!("window {window} does not exist")))?;
        let (leave, restored) = match current {
            WindowMode::Fullscreen(restore) => {
                (Some(ModeChange::LeaveFullscreen), WindowMode::from(restore))
            }
            WindowMode::Tiled | WindowMode::Floating(_) => (None, current),
        };
        let settle = match (requested, restored) {
            (tilling_ipc::WindowMode::Fullscreen, _) => {
                return Ok(vec![ModeChange::EnterFullscreen]);
            }
            (tilling_ipc::WindowMode::Tiled, WindowMode::Tiled)
            | (tilling_ipc::WindowMode::Floating, WindowMode::Floating(_)) => None,
            (tilling_ipc::WindowMode::Tiled, _) => Some(ModeChange::Tile),
            (tilling_ipc::WindowMode::Floating, _) => Some(self.float_change(window)?),
        };
        Ok(leave.into_iter().chain(settle).collect())
    }

    fn float_change(&self, window: WindowId) -> Result<ModeChange, Refusal> {
        let workspace = self.desktop.window(window).map(|state| state.workspace());
        workspace
            .zip(self.windows.by_id(window))
            .and_then(|(workspace, managed)| self.floating_area(workspace, managed))
            .map(ModeChange::Float)
            .ok_or_else(|| Refusal::Invalid(NOWHERE_TO_FLOAT.to_owned()))
    }

    fn subscribe(&mut self, peer: PeerId) -> Result<Reply, Refusal> {
        let subscriber = self
            .control
            .get_mut(peer)
            .filter(|peer| peer.connection.grants(Capability::Events))
            .ok_or_else(|| Refusal::NotPermitted(EVENTS_NOT_NEGOTIATED.to_owned()))?;
        subscriber.subscribed = true;
        Ok(Reply::Subscribed(Snapshot {
            sequence: self.control.sequence(),
            revision: self.desktop.revision(),
            outputs: views::outputs(&self.desktop),
            workspaces: views::workspaces(&self.desktop),
            windows: views::windows(&self.desktop),
        }))
    }
}

fn core_direction(direction: tilling_ipc::Direction) -> Direction {
    match direction {
        tilling_ipc::Direction::Left => Direction::Left,
        tilling_ipc::Direction::Right => Direction::Right,
        tilling_ipc::Direction::Up => Direction::Up,
        tilling_ipc::Direction::Down => Direction::Down,
    }
}
