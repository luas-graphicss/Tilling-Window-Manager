mod buffer;
mod client;
mod decoration;
mod dispatch;
mod error;
mod popup;
mod state;

pub use client::{Client, ToplevelId};
pub use decoration::DecorationMode;
pub use error::ClientError;
pub use popup::{PopupId, PopupPlacement};
pub use state::{
    AdvertisedGlobal, ButtonEvent, KeyEvent, PopupConfigure, ScrollEvent, ToplevelConfigure,
};
pub use wayland_protocols::xdg::shell::client::xdg_toplevel::{ResizeEdge, WmCapabilities};
