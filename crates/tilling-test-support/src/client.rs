use std::os::unix::net::UnixStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, EventQueue};
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::{ResizeEdge, XdgToplevel};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

use crate::buffer;
use crate::error::ClientError;
use crate::state::{
    AdvertisedGlobal, ButtonEvent, ClientState, KeyEvent, ScrollEvent, ToplevelConfigure,
    ToplevelRecord,
};

const DEFAULT_WIDTH: u32 = 640;
const DEFAULT_HEIGHT: u32 = 480;
const WAIT_LIMIT: Duration = Duration::from_secs(5);
const WAIT_STEP: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToplevelId(usize);

#[derive(Debug)]
struct Toplevel {
    surface: WlSurface,
    xdg_surface: XdgSurface,
    role: XdgToplevel,
    buffer: Option<WlBuffer>,
}

#[derive(Debug)]
pub struct Client {
    queue: EventQueue<ClientState>,
    state: ClientState,
    globals: Vec<AdvertisedGlobal>,
    compositor: WlCompositor,
    shm: WlShm,
    wm_base: XdgWmBase,
    seat: WlSeat,
    toplevels: Vec<Option<Toplevel>>,
}

impl Client {
    pub fn connect(socket: &Path) -> Result<Self, ClientError> {
        let stream = UnixStream::connect(socket).map_err(|source| ClientError::Connect {
            path: socket.to_path_buf(),
            source,
        })?;
        let connection = Connection::from_socket(stream).map_err(ClientError::Handshake)?;
        let (list, queue) =
            registry_queue_init::<ClientState>(&connection).map_err(ClientError::Globals)?;
        let handle = queue.handle();
        let bind_error = |interface| move |source| ClientError::Bind { interface, source };
        let compositor = list
            .bind(&handle, 1..=6, ())
            .map_err(bind_error("wl_compositor"))?;
        let shm = list
            .bind(&handle, 1..=1, ())
            .map_err(bind_error("wl_shm"))?;
        let wm_base = list
            .bind(&handle, 1..=6, ())
            .map_err(bind_error("xdg_wm_base"))?;
        let seat = list
            .bind(&handle, 1..=9, ())
            .map_err(bind_error("wl_seat"))?;
        let globals = list
            .contents()
            .clone_list()
            .into_iter()
            .map(|global| AdvertisedGlobal {
                interface: global.interface,
                version: global.version,
            })
            .collect();
        Ok(Self {
            queue,
            state: ClientState::default(),
            globals,
            compositor,
            shm,
            wm_base,
            seat,
            toplevels: Vec::new(),
        })
    }

    #[must_use]
    pub fn keyboard_focus(&self) -> Option<ToplevelId> {
        self.state.seat.keyboard_focus.map(ToplevelId)
    }

    #[must_use]
    pub fn key_events(&self) -> &[KeyEvent] {
        &self.state.seat.keys
    }

    #[must_use]
    pub fn pointer_focus(&self) -> Option<ToplevelId> {
        self.state.seat.pointer_focus.map(ToplevelId)
    }

    #[must_use]
    pub fn pointer_position(&self) -> Option<(f64, f64)> {
        self.state.seat.pointer_position
    }

    #[must_use]
    pub fn button_events(&self) -> &[ButtonEvent] {
        &self.state.seat.buttons
    }

    #[must_use]
    pub fn button_serial(&self) -> Option<u32> {
        self.state.seat.button_serial
    }

    #[must_use]
    pub fn scroll_events(&self) -> &[ScrollEvent] {
        &self.state.seat.scrolls
    }

    #[must_use]
    pub fn globals(&self) -> &[AdvertisedGlobal] {
        &self.globals
    }

    #[must_use]
    pub fn released_buffers(&self) -> usize {
        self.state.released_buffers
    }

    pub fn create_toplevel(&mut self, app_id: &str, title: &str) -> ToplevelId {
        let handle = self.queue.handle();
        let index = self.toplevels.len();
        let surface = self.compositor.create_surface(&handle, index);
        let xdg_surface = self.wm_base.get_xdg_surface(&surface, &handle, index);
        let role = xdg_surface.get_toplevel(&handle, index);
        role.set_app_id(app_id.to_owned());
        role.set_title(title.to_owned());
        self.state.toplevels.push(ToplevelRecord::default());
        self.toplevels.push(Some(Toplevel {
            surface,
            xdg_surface,
            role,
            buffer: None,
        }));
        ToplevelId(index)
    }

    pub fn set_parent(&mut self, child: ToplevelId, parent: ToplevelId) -> Result<(), ClientError> {
        let parent = &self.toplevel(parent)?.role;
        self.toplevel(child)?.role.set_parent(Some(parent));
        Ok(())
    }

    pub fn set_title(&mut self, toplevel: ToplevelId, title: &str) -> Result<(), ClientError> {
        self.toplevel(toplevel)?.role.set_title(title.to_owned());
        Ok(())
    }

    pub fn set_fullscreen(
        &mut self,
        toplevel: ToplevelId,
        fullscreen: bool,
    ) -> Result<(), ClientError> {
        let role = &self.toplevel(toplevel)?.role;
        if fullscreen {
            role.set_fullscreen(None);
        } else {
            role.unset_fullscreen();
        }
        Ok(())
    }

    pub fn request_move(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        let serial = self.button_serial().ok_or(ClientError::NoButtonSerial)?;
        self.request_move_with(toplevel, serial)
    }

    pub fn request_move_with(
        &mut self,
        toplevel: ToplevelId,
        serial: u32,
    ) -> Result<(), ClientError> {
        self.toplevel(toplevel)?.role._move(&self.seat, serial);
        self.roundtrip()
    }

    pub fn request_resize(
        &mut self,
        toplevel: ToplevelId,
        edge: ResizeEdge,
    ) -> Result<(), ClientError> {
        let serial = self.button_serial().ok_or(ClientError::NoButtonSerial)?;
        self.toplevel(toplevel)?
            .role
            .resize(&self.seat, serial, edge);
        self.roundtrip()
    }

    pub fn commit(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        self.toplevel(toplevel)?.surface.commit();
        Ok(())
    }

    pub fn attach(
        &mut self,
        toplevel: ToplevelId,
        width: u32,
        height: u32,
    ) -> Result<(), ClientError> {
        let buffer = buffer::create(&self.shm, &self.queue.handle(), width, height)?;
        let target = self.toplevel_mut(toplevel)?;
        target.surface.attach(Some(&buffer), 0, 0);
        target.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        if let Some(previous) = target.buffer.replace(buffer) {
            previous.destroy();
        }
        Ok(())
    }

    pub fn acknowledge(
        &mut self,
        toplevel: ToplevelId,
    ) -> Result<Option<ToplevelConfigure>, ClientError> {
        let xdg_surface = self.toplevel(toplevel)?.xdg_surface.clone();
        let Some(record) = self.state.toplevels.get_mut(toplevel.0) else {
            return Err(ClientError::DestroyedToplevel);
        };
        let Some(latest) = record.unacknowledged.pop_back() else {
            return Ok(None);
        };
        record.unacknowledged.clear();
        xdg_surface.ack_configure(latest.serial);
        Ok(Some(latest))
    }

    pub fn map(&mut self, toplevel: ToplevelId) -> Result<ToplevelConfigure, ClientError> {
        self.commit(toplevel)?;
        let configure = self.wait_for_configure(toplevel)?;
        self.present(toplevel, configure)?;
        self.roundtrip()?;
        Ok(configure)
    }

    pub fn unmap(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        let target = self.toplevel_mut(toplevel)?;
        target.surface.attach(None, 0, 0);
        target.surface.commit();
        if let Some(buffer) = target.buffer.take() {
            buffer.destroy();
        }
        self.roundtrip()
    }

    pub fn destroy_toplevel(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        let target = self
            .toplevels
            .get_mut(toplevel.0)
            .and_then(Option::take)
            .ok_or(ClientError::DestroyedToplevel)?;
        target.role.destroy();
        target.xdg_surface.destroy();
        target.surface.destroy();
        if let Some(buffer) = target.buffer {
            buffer.destroy();
        }
        self.roundtrip()
    }

    pub fn follow_configures(&mut self) -> Result<usize, ClientError> {
        self.roundtrip()?;
        let mapped: Vec<ToplevelId> = self
            .toplevels
            .iter()
            .enumerate()
            .filter(|(_, toplevel)| toplevel.as_ref().is_some_and(|it| it.buffer.is_some()))
            .map(|(index, _)| ToplevelId(index))
            .collect();
        let mut followed = 0;
        for toplevel in mapped {
            if let Some(configure) = self.acknowledge(toplevel)? {
                self.attach_for(toplevel, configure)?;
                self.commit(toplevel)?;
                followed += 1;
            }
        }
        self.roundtrip()?;
        Ok(followed)
    }

    pub fn settle(&mut self) -> Result<(), ClientError> {
        let started = Instant::now();
        while self.follow_configures()? > 0 {
            if started.elapsed() > WAIT_LIMIT {
                return Err(ClientError::Timeout("configures to stop arriving"));
            }
        }
        Ok(())
    }

    pub fn wait_for_configure(
        &mut self,
        toplevel: ToplevelId,
    ) -> Result<ToplevelConfigure, ClientError> {
        let started = Instant::now();
        loop {
            self.roundtrip()?;
            let pending = self
                .state
                .toplevels
                .get(toplevel.0)
                .and_then(|record| record.unacknowledged.back());
            if let Some(configure) = pending {
                return Ok(*configure);
            }
            if started.elapsed() > WAIT_LIMIT {
                return Err(ClientError::Timeout("a toplevel configure"));
            }
            thread::sleep(WAIT_STEP);
        }
    }

    pub fn roundtrip(&mut self) -> Result<(), ClientError> {
        self.queue
            .roundtrip(&mut self.state)
            .map(drop)
            .map_err(ClientError::Dispatch)
    }

    #[must_use]
    pub fn latest_configure(&self, toplevel: ToplevelId) -> Option<ToplevelConfigure> {
        self.state.toplevels.get(toplevel.0)?.latest
    }

    #[must_use]
    pub fn unacknowledged_configures(&self, toplevel: ToplevelId) -> usize {
        self.state
            .toplevels
            .get(toplevel.0)
            .map_or(0, |record| record.unacknowledged.len())
    }

    #[must_use]
    pub fn close_requested(&self, toplevel: ToplevelId) -> bool {
        self.state
            .toplevels
            .get(toplevel.0)
            .is_some_and(|record| record.close_requested)
    }

    fn present(
        &mut self,
        toplevel: ToplevelId,
        configure: ToplevelConfigure,
    ) -> Result<(), ClientError> {
        self.acknowledge(toplevel)?;
        self.attach_for(toplevel, configure)?;
        self.commit(toplevel)
    }

    fn attach_for(
        &mut self,
        toplevel: ToplevelId,
        configure: ToplevelConfigure,
    ) -> Result<(), ClientError> {
        let (width, height) = match (configure.width, configure.height) {
            (0, _) | (_, 0) => (DEFAULT_WIDTH, DEFAULT_HEIGHT),
            size => size,
        };
        self.attach(toplevel, width, height)
    }

    fn toplevel(&self, toplevel: ToplevelId) -> Result<&Toplevel, ClientError> {
        self.toplevels
            .get(toplevel.0)
            .and_then(Option::as_ref)
            .ok_or(ClientError::DestroyedToplevel)
    }

    fn toplevel_mut(&mut self, toplevel: ToplevelId) -> Result<&mut Toplevel, ClientError> {
        self.toplevels
            .get_mut(toplevel.0)
            .and_then(Option::as_mut)
            .ok_or(ClientError::DestroyedToplevel)
    }
}
