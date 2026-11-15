use std::time::Duration;

use smithay::desktop::utils::send_frames_surface_tree;
use smithay::output::Output;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::utils::SERIAL_COUNTER;
use tilling_config::ValidatedConfig;
use tilling_core::{
    Command, DesktopState, Effect, Transition, TransitionError, WindowId, WorkspaceId,
};

use crate::error::CompositorError;
use crate::runtime::Compositor;
use crate::runtime::session::SessionSnapshot;
use crate::scene::Scene;
use crate::shell::configure_toplevel;

pub(crate) fn initial_desktop(config: &ValidatedConfig) -> Result<DesktopState, CompositorError> {
    let (first, rest) = config
        .workspaces()
        .split_first()
        .ok_or(CompositorError::NoWorkspaces)?;
    let mut desktop = DesktopState::new(first.as_str());
    for name in rest {
        desktop.apply(Command::CreateWorkspace { name: name.clone() })?;
    }
    Ok(desktop)
}

impl Compositor {
    pub(crate) fn execute(&mut self, command: Command) -> Result<Transition, TransitionError> {
        let transition = self.desktop.apply(command)?;
        self.react(&transition);
        Ok(transition)
    }

    pub(crate) fn react(&mut self, transition: &Transition) {
        if transition.effects.is_empty() {
            return;
        }
        for effect in &transition.effects {
            match *effect {
                Effect::FocusChanged { current, .. } => self.focus_keyboard(current),
                Effect::WindowClosed(window) => self.retire_window(window),
                Effect::OutputAdded(_)
                | Effect::OutputConfigured(_)
                | Effect::OutputRemoved(_)
                | Effect::WorkspaceCreated(_)
                | Effect::WorkspaceShown { .. }
                | Effect::WorkspaceActivated(_)
                | Effect::WorkspaceArranged(_)
                | Effect::WindowOpened(_)
                | Effect::WindowMoved { .. }
                | Effect::WindowMetadataChanged(_)
                | Effect::WindowModeChanged(_) => {}
            }
        }
        self.publish(transition);
        self.rearrange();
    }

    pub(crate) fn settle(&mut self) {
        if let Some(scene) = self.transactions.settle(self.elapsed()) {
            self.scene = scene;
            self.demand_frame();
        }
    }

    pub(crate) fn send_frame_callbacks(&self, output: &Output) {
        let elapsed = self.elapsed();
        for placed in self.scene.windows() {
            if let Some(managed) = self.windows.by_id(placed.id) {
                managed
                    .window
                    .send_frame(output, elapsed, Some(Duration::ZERO), |_, _| {
                        Some(output.clone())
                    });
            }
        }
        if let Some(cursor) = self.cursor_surface() {
            send_frames_surface_tree(cursor, output, elapsed, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }
    }

    pub(crate) fn workspace_named(&self, name: &str) -> Option<WorkspaceId> {
        self.desktop
            .workspaces()
            .find(|(_, workspace)| workspace.name() == name)
            .map(|(id, _)| id)
    }

    pub(crate) fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            revision: self.desktop.revision(),
            scene: self.scene.clone(),
            focused: self.desktop.focused_window(),
            toplevels: self.windows.len(),
            mapped_windows: self.windows.mapped_len(),
            popups: self.xdg_shell_state.popup_surfaces().len(),
            transaction_pending: self.transactions.is_pending(),
            keyboard_mode: self.keyboard_mode.clone(),
            control_peers: self.control.len(),
            control_subscribers: self.control.subscriber_count(),
            frames: self.frames,
        }
    }

    pub(crate) fn rearrange(&mut self) {
        let target = Scene::build(&self.desktop, self.config.layout().gaps);
        let configured: Vec<_> = target
            .windows()
            .iter()
            .filter_map(|placed| {
                let managed = self.windows.by_id(placed.id)?;
                Some((placed.id, configure_toplevel(&managed.toplevel, placed)?))
            })
            .collect();
        let now = self.elapsed();
        if let Some(deadline) = self.transactions.begin(target, configured, now) {
            self.wake_at_deadline(deadline.saturating_sub(now));
        }
        self.settle();
    }

    fn wake_at_deadline(&mut self, delay: Duration) {
        let timer = Timer::from_duration(delay);
        let armed = self.loop_handle.insert_source(timer, |_, (), runtime| {
            runtime.compositor.settle();
            TimeoutAction::Drop
        });
        if let Err(error) = armed {
            tracing::warn!(error = %error.error, "arm transaction deadline");
        }
    }

    fn retire_window(&mut self, window: WindowId) {
        self.transactions.forget(window);
        if self.scene.remove(window) {
            self.demand_frame();
        }
    }

    fn focus_keyboard(&mut self, window: Option<WindowId>) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let surface = window.and_then(|id| self.windows.surface_of(id)).cloned();
        keyboard.set_focus(self, surface, SERIAL_COUNTER.next_serial());
    }
}
