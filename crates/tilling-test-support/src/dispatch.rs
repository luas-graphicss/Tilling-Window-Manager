use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self, XdgToplevel};
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};

use crate::state::{ClientState, ProposedState};

delegate_noop!(ClientState: WlCompositor);
delegate_noop!(ClientState: WlShmPool);
delegate_noop!(ClientState: ignore WlShm);
delegate_noop!(ClientState: ignore WlSurface);

impl Dispatch<WlRegistry, GlobalListContents> for ClientState {
    fn event(
        _state: &mut Self,
        _registry: &WlRegistry,
        _event: wl_registry::Event,
        _contents: &GlobalListContents,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for ClientState {
    fn event(
        state: &mut Self,
        _buffer: &WlBuffer,
        event: wl_buffer::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            state.released_buffers += 1;
        }
    }
}

impl Dispatch<XdgWmBase, ()> for ClientState {
    fn event(
        _state: &mut Self,
        wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, usize> for ClientState {
    fn event(
        state: &mut Self,
        _surface: &XdgSurface,
        event: xdg_surface::Event,
        toplevel: &usize,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event
            && let Some(record) = state.toplevels.get_mut(*toplevel)
        {
            record.configure(serial);
        }
    }
}

impl Dispatch<XdgToplevel, usize> for ClientState {
    fn event(
        state: &mut Self,
        _toplevel: &XdgToplevel,
        event: xdg_toplevel::Event,
        toplevel: &usize,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let Some(record) = state.toplevels.get_mut(*toplevel) else {
            return;
        };
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => record.propose(ProposedState::new(width, height, &states)),
            xdg_toplevel::Event::Close => record.close_requested = true,
            _ => {}
        }
    }
}
