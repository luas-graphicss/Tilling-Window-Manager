use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1,
};

use crate::buffer;
use crate::client::Client;
use crate::error::ClientError;
use crate::state::ClientState;

const NAMESPACE: &str = "tilling-test";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerIndex(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerRequest {
    Bar { edge: ScreenEdge, thickness: u32 },
    Launcher { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayerConfigure {
    serial: u32,
    width: u32,
    height: u32,
}

#[derive(Debug)]
pub(crate) struct LayerRecord {
    surface: WlSurface,
    role: ZwlrLayerSurfaceV1,
    buffer: Option<WlBuffer>,
    configure: Option<LayerConfigure>,
    closed: bool,
}

delegate_noop!(ClientState: ZwlrLayerShellV1);

impl Dispatch<ZwlrLayerSurfaceV1, LayerIndex> for ClientState {
    fn event(
        state: &mut Self,
        _role: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        layer: &LayerIndex,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let Some(record) = state.layers.get_mut(layer.0).and_then(Option::as_mut) else {
            return;
        };
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                record.configure = Some(LayerConfigure {
                    serial,
                    width,
                    height,
                });
            }
            zwlr_layer_surface_v1::Event::Closed => record.closed = true,
            _ => {}
        }
    }
}

impl LayerRequest {
    const fn level(self) -> Layer {
        match self {
            Self::Bar { .. } => Layer::Top,
            Self::Launcher { .. } => Layer::Overlay,
        }
    }

    fn describe(self, role: &ZwlrLayerSurfaceV1) {
        match self {
            Self::Bar { edge, thickness } => {
                let reserved = i32::try_from(thickness).unwrap_or(i32::MAX);
                let (anchor, width, height) = match edge {
                    ScreenEdge::Top => (Anchor::Top | Anchor::Left | Anchor::Right, 0, thickness),
                    ScreenEdge::Bottom => {
                        (Anchor::Bottom | Anchor::Left | Anchor::Right, 0, thickness)
                    }
                    ScreenEdge::Left => (Anchor::Left | Anchor::Top | Anchor::Bottom, thickness, 0),
                    ScreenEdge::Right => {
                        (Anchor::Right | Anchor::Top | Anchor::Bottom, thickness, 0)
                    }
                };
                role.set_anchor(anchor);
                role.set_size(width, height);
                role.set_exclusive_zone(reserved);
            }
            Self::Launcher { width, height } => {
                role.set_size(width, height);
                role.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
            }
        }
    }
}

impl Client {
    pub fn map_layer(&mut self, request: LayerRequest) -> Result<LayerId, ClientError> {
        let shell: ZwlrLayerShellV1 = self.bind(1..=4, "zwlr_layer_shell_v1")?;
        let handle = self.queue.handle();
        let index = self.state.layers.len();
        let surface = self.compositor.create_surface(&handle, LayerIndex(index));
        let role = shell.get_layer_surface(
            &surface,
            None,
            request.level(),
            NAMESPACE.to_owned(),
            &handle,
            LayerIndex(index),
        );
        request.describe(&role);
        surface.commit();
        self.state.layers.push(Some(LayerRecord {
            surface,
            role,
            buffer: None,
            configure: None,
            closed: false,
        }));
        self.present_layer(index)?;
        Ok(LayerId(index))
    }

    pub fn destroy_layer(&mut self, layer: LayerId) -> Result<(), ClientError> {
        let record = self
            .state
            .layers
            .get_mut(layer.0)
            .and_then(Option::take)
            .ok_or(ClientError::DestroyedLayer)?;
        record.role.destroy();
        record.surface.destroy();
        if let Some(buffer) = record.buffer {
            buffer.destroy();
        }
        self.roundtrip()
    }

    #[must_use]
    pub fn layer_size(&self, layer: LayerId) -> Option<(u32, u32)> {
        let configure = self.layer(layer.0)?.configure?;
        Some((configure.width, configure.height))
    }

    #[must_use]
    pub fn layer_closed(&self, layer: LayerId) -> bool {
        self.layer(layer.0).is_some_and(|record| record.closed)
    }

    #[must_use]
    pub fn keyboard_layer(&self) -> Option<LayerId> {
        self.state.seat.keyboard_layer.map(LayerId)
    }

    #[must_use]
    pub fn pointer_layer(&self) -> Option<LayerId> {
        self.state.seat.pointer_layer.map(LayerId)
    }

    fn present_layer(&mut self, index: usize) -> Result<(), ClientError> {
        let configure = self.wait_for("a layer surface configure", |state| {
            state.layers.get(index)?.as_ref()?.configure
        })?;
        let handle = self.queue.handle();
        let buffer = buffer::create(&self.shm, &handle, configure.width, configure.height)?;
        let record = self
            .state
            .layers
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or(ClientError::DestroyedLayer)?;
        record.role.ack_configure(configure.serial);
        record.surface.attach(Some(&buffer), 0, 0);
        record.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        record.surface.commit();
        record.buffer = Some(buffer);
        self.roundtrip()
    }

    fn layer(&self, index: usize) -> Option<&LayerRecord> {
        self.state.layers.get(index)?.as_ref()
    }
}
