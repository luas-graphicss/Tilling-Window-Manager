use std::collections::VecDeque;

use wayland_protocols::xdg::shell::client::xdg_toplevel::State;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvertisedGlobal {
    pub interface: String,
    pub version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelConfigure {
    pub serial: u32,
    pub width: u32,
    pub height: u32,
    pub activated: bool,
    pub fullscreen: bool,
    pub tiled: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ProposedState {
    width: u32,
    height: u32,
    activated: bool,
    fullscreen: bool,
    tiled: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ToplevelRecord {
    proposed: ProposedState,
    pub(crate) unacknowledged: VecDeque<ToplevelConfigure>,
    pub(crate) latest: Option<ToplevelConfigure>,
    pub(crate) close_requested: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ClientState {
    pub(crate) toplevels: Vec<ToplevelRecord>,
    pub(crate) released_buffers: usize,
}

impl ProposedState {
    pub(crate) fn new(width: i32, height: i32, states: &[u8]) -> Self {
        let states: Vec<State> = states
            .as_chunks::<4>()
            .0
            .iter()
            .filter_map(|bytes| State::try_from(u32::from_ne_bytes(*bytes)).ok())
            .collect();
        let tiled_edges = [
            State::TiledLeft,
            State::TiledRight,
            State::TiledTop,
            State::TiledBottom,
        ];
        Self {
            width: u32::try_from(width).unwrap_or(0),
            height: u32::try_from(height).unwrap_or(0),
            activated: states.contains(&State::Activated),
            fullscreen: states.contains(&State::Fullscreen),
            tiled: tiled_edges.iter().all(|edge| states.contains(edge)),
        }
    }
}

impl ToplevelRecord {
    pub(crate) fn propose(&mut self, proposed: ProposedState) {
        self.proposed = proposed;
    }

    pub(crate) fn configure(&mut self, serial: u32) {
        let configure = ToplevelConfigure {
            serial,
            width: self.proposed.width,
            height: self.proposed.height,
            activated: self.proposed.activated,
            fullscreen: self.proposed.fullscreen,
            tiled: self.proposed.tiled,
        };
        self.unacknowledged.push_back(configure);
        self.latest = Some(configure);
    }
}
