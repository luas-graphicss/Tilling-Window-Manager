use smithay::input::keyboard::XkbConfig;
use tilling_config::{Action, CommandLine, DEFAULT_MODE, ValidatedConfig};
use tilling_core::{Command, ModeChange, WindowId, WindowMode};

use super::BindingTable;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn perform(&mut self, action: Action) {
        match action {
            Action::Spawn(command) => self.launch(&command),
            Action::OpenTerminal => {
                let terminal = self.config.applications().terminal.clone();
                self.launch(&terminal);
            }
            Action::CloseWindow => self.request_close(),
            Action::Focus(direction) => self.run(Command::FocusDirection { direction }),
            Action::Move(direction) => {
                self.run_on_focused(|window| Command::MoveWindow { window, direction });
            }
            Action::SwitchWorkspace(name) => {
                if let Some(workspace) = self.workspace_named(&name) {
                    self.run(Command::SwitchWorkspace { workspace });
                }
            }
            Action::MoveToWorkspace(name) => {
                if let Some(workspace) = self.workspace_named(&name) {
                    self.run_on_focused(|window| Command::MoveWindowToWorkspace {
                        window,
                        workspace,
                    });
                }
            }
            Action::ToggleFullscreen => self.toggle_fullscreen(),
            Action::ToggleFloating => self.toggle_floating(),
            Action::ToggleSplit => self.run_on_focused(|window| Command::ToggleSplit { window }),
            Action::Resize(delta) => {
                self.run_on_focused(|window| Command::ResizeSplit { window, delta });
            }
            Action::EnterMode(mode) => self.keyboard_mode = mode,
            Action::ReloadConfig => self.reload_config(),
            Action::Quit => self.stop(),
        }
    }

    pub(crate) fn run(&mut self, command: Command) {
        if let Err(error) = self.execute(command) {
            tracing::warn!(%error, "reject command");
        }
    }

    fn run_on_focused(&mut self, command: impl FnOnce(WindowId) -> Command) {
        if let Some(window) = self.desktop.focused_window() {
            self.run(command(window));
        }
    }

    fn launch(&mut self, command: &CommandLine) {
        if let Err(error) = self.launcher.launch(command) {
            tracing::warn!(%error, program = command.program(), "launch program");
        }
    }

    fn request_close(&self) {
        let focused = self.desktop.focused_window();
        if let Some(managed) = focused.and_then(|window| self.windows.by_id(window)) {
            managed.toplevel.send_close();
        }
    }

    fn toggle_fullscreen(&mut self) {
        let Some(window) = self.desktop.focused_window() else {
            return;
        };
        let Some(mode) = self.desktop.window(window).map(|state| state.mode()) else {
            return;
        };
        let change = match mode {
            WindowMode::Fullscreen(_) => ModeChange::LeaveFullscreen,
            WindowMode::Tiled | WindowMode::Floating(_) => ModeChange::EnterFullscreen,
        };
        self.run(Command::ChangeWindowMode { window, change });
    }

    fn toggle_floating(&mut self) {
        let Some(window) = self.desktop.focused_window() else {
            return;
        };
        let Some(state) = self.desktop.window(window) else {
            return;
        };
        let change = match state.mode() {
            WindowMode::Floating(_) => Some(ModeChange::Tile),
            WindowMode::Tiled => self
                .windows
                .by_id(window)
                .and_then(|managed| self.floating_area(state.workspace(), managed))
                .map(ModeChange::Float),
            WindowMode::Fullscreen(_) => None,
        };
        if let Some(change) = change {
            self.run(Command::ChangeWindowMode { window, change });
        }
    }

    fn reload_config(&mut self) {
        match tilling_config::load(&self.config_source) {
            Ok(config) => self.adopt_config(config),
            Err(error) => tracing::warn!(%error, "keep the active configuration"),
        }
    }

    fn adopt_config(&mut self, config: ValidatedConfig) {
        self.bindings = BindingTable::compile(config.bindings());
        if !self.bindings.has_mode(&self.keyboard_mode) {
            DEFAULT_MODE.clone_into(&mut self.keyboard_mode);
        }
        self.config = config;
        self.apply_keyboard_config();
        for name in self.config.workspaces().to_vec() {
            if self.workspace_named(&name).is_none() {
                self.run(Command::CreateWorkspace { name });
            }
        }
        self.rearrange();
    }

    fn apply_keyboard_config(&mut self) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let settings = self.config.input().keyboard.clone();
        let layout = XkbConfig {
            layout: &settings.layout,
            variant: &settings.variant,
            ..XkbConfig::default()
        };
        if let Err(error) = keyboard.set_xkb_config(self, layout) {
            tracing::warn!(%error, "keep the active keyboard layout");
        }
        keyboard.change_repeat_info(
            i32::try_from(settings.repeat_rate).unwrap_or(i32::MAX),
            i32::try_from(settings.repeat_delay_ms).unwrap_or(i32::MAX),
        );
    }
}
