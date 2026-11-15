use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_keyboard::{self, KeyState, WlKeyboard};
use wayland_client::protocol::wl_pointer::{self, ButtonState, WlPointer};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::{self, Capability, WlSeat};
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::{self, WlSurface};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self, XdgToplevel};
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};

use crate::state::{ButtonEvent, ClientState, KeyEvent, ProposedState};

delegate_noop!(ClientState: WlCompositor);
delegate_noop!(ClientState: WlShmPool);
delegate_noop!(ClientState: ignore WlShm);

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

impl Dispatch<WlSurface, usize> for ClientState {
    fn event(
        _state: &mut Self,
        _surface: &WlSurface,
        _event: wl_surface::Event,
        _toplevel: &usize,
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

impl Dispatch<WlSeat, ()> for ClientState {
    fn event(
        state: &mut Self,
        seat: &WlSeat,
        event: wl_seat::Event,
        (): &(),
        _connection: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        else {
            return;
        };
        if capabilities.contains(Capability::Keyboard) && !state.seat.has_keyboard {
            seat.get_keyboard(queue, ());
            state.seat.has_keyboard = true;
        }
        if capabilities.contains(Capability::Pointer) && !state.seat.has_pointer {
            seat.get_pointer(queue, ());
            state.seat.has_pointer = true;
        }
    }
}

impl Dispatch<WlKeyboard, ()> for ClientState {
    fn event(
        state: &mut Self,
        _keyboard: &WlKeyboard,
        event: wl_keyboard::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { surface, .. } => {
                state.seat.keyboard_focus = toplevel_of(&surface);
            }
            wl_keyboard::Event::Leave { .. } => state.seat.keyboard_focus = None,
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(key_state),
                ..
            } => state.seat.keys.push(KeyEvent {
                code: key,
                pressed: key_state == KeyState::Pressed,
            }),
            _ => {}
        }
    }
}

impl Dispatch<WlPointer, ()> for ClientState {
    fn event(
        state: &mut Self,
        _pointer: &WlPointer,
        event: wl_pointer::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                state.seat.pointer_focus = toplevel_of(&surface);
                state.seat.pointer_position = Some((surface_x, surface_y));
            }
            wl_pointer::Event::Leave { .. } => {
                state.seat.pointer_focus = None;
                state.seat.pointer_position = None;
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => state.seat.pointer_position = Some((surface_x, surface_y)),
            wl_pointer::Event::Button {
                button,
                state: WEnum::Value(button_state),
                ..
            } => state.seat.buttons.push(ButtonEvent {
                code: button,
                pressed: button_state == ButtonState::Pressed,
            }),
            _ => {}
        }
    }
}

fn toplevel_of(surface: &WlSurface) -> Option<usize> {
    surface.data::<usize>().copied()
}
