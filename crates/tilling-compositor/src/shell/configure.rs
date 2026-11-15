use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::shell::xdg::{ToplevelState, ToplevelStateSet, ToplevelSurface};
use tilling_core::{Effect, Transition, WindowId, WindowMode};

use super::placement::WindowPlan;
use crate::geometry;
use crate::runtime::Compositor;
use crate::scene::{Scene, SceneLayer, SceneWindow};
use crate::transaction::ConfigureSerial;

const TILED_EDGES: [State; 4] = [
    State::TiledLeft,
    State::TiledRight,
    State::TiledTop,
    State::TiledBottom,
];

pub(crate) fn configure_toplevel(
    toplevel: &ToplevelSurface,
    placed: &SceneWindow,
) -> Option<ConfigureSerial> {
    let resized = toplevel.with_pending_state(|state| describe(state, placed));
    let serial = toplevel.send_pending_configure()?;
    resized.then(|| ConfigureSerial(u32::from(serial)))
}

pub(crate) fn opened_window(transition: &Transition) -> Option<WindowId> {
    transition.effects.iter().find_map(|effect| match effect {
        Effect::WindowOpened(window) => Some(*window),
        _ => None,
    })
}

impl Compositor {
    pub(crate) fn send_initial_configure(&mut self, surface: &WlSurface) {
        let Some(managed) = self.windows.get(surface) else {
            return;
        };
        let toplevel = managed.toplevel.clone();
        let preview = self.preview_placement(self.plan_window(managed));
        if let Some(placed) = preview {
            toplevel.with_pending_state(|state| describe(state, &placed));
        }
        toplevel.send_configure();
    }

    fn preview_placement(&self, plan: WindowPlan) -> Option<SceneWindow> {
        if matches!(plan.mode, WindowMode::Floating(_)) {
            return None;
        }
        let mut preview = self.desktop.clone();
        let transition = preview.apply(plan.into_command()).ok()?;
        let window = opened_window(&transition)?;
        Scene::build(&preview, self.config.layout().gaps)
            .window(window)
            .copied()
    }
}

fn describe(state: &mut ToplevelState, placed: &SceneWindow) -> bool {
    let size = geometry::size(placed.content);
    let resized = state.size != Some(size);
    state.size = Some(size);
    switch(&mut state.states, State::Activated, placed.focused);
    switch(
        &mut state.states,
        State::Fullscreen,
        placed.layer == SceneLayer::Fullscreen,
    );
    for edge in TILED_EDGES {
        switch(&mut state.states, edge, placed.layer == SceneLayer::Tiled);
    }
    resized
}

fn switch(states: &mut ToplevelStateSet, state: State, enabled: bool) {
    if enabled {
        states.set(state);
    } else {
        states.unset(state);
    }
}
