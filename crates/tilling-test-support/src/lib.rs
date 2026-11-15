mod buffer;
mod client;
mod dispatch;
mod error;
mod state;

pub use client::{Client, ToplevelId};
pub use error::ClientError;
pub use state::{AdvertisedGlobal, ButtonEvent, KeyEvent, ScrollEvent, ToplevelConfigure};
pub use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;
