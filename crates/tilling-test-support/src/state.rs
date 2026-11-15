use std::collections::VecDeque;

use wayland_client::protocol::wl_pointer::WlPointer;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: u32,
    pub pressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonEvent {
    pub code: u32,
    pub pressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollEvent {
    pub horizontal: f64,
    pub vertical: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupConfigure {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PopupIndex(pub(crate) usize);

#[derive(Debug, Default)]
pub(crate) struct PopupRecord {
    pub(crate) proposed: Option<PopupConfigure>,
    pub(crate) configured: Option<(u32, PopupConfigure)>,
    pub(crate) dismissed: bool,
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
pub(crate) struct SeatLog {
    pub(crate) has_keyboard: bool,
    pub(crate) pointer: Option<WlPointer>,
    pub(crate) enter_serial: Option<u32>,
    pub(crate) keyboard_focus: Option<usize>,
    pub(crate) keyboard_popup: Option<usize>,
    pub(crate) keys: Vec<KeyEvent>,
    pub(crate) pointer_focus: Option<usize>,
    pub(crate) pointer_position: Option<(f64, f64)>,
    pub(crate) buttons: Vec<ButtonEvent>,
    pub(crate) button_serial: Option<u32>,
    pub(crate) scrolls: Vec<ScrollEvent>,
}

#[derive(Debug, Default)]
pub(crate) struct ClientState {
    pub(crate) toplevels: Vec<ToplevelRecord>,
    pub(crate) popups: Vec<PopupRecord>,
    pub(crate) seat: SeatLog,
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
