use std::thread;
use std::time::Instant;

use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::xdg::shell::client::xdg_popup::XdgPopup;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor, ConstraintAdjustment, Gravity,
};
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;

use crate::buffer;
use crate::client::{Client, ToplevelId, WAIT_LIMIT, WAIT_STEP};
use crate::error::ClientError;
use crate::state::{PopupConfigure, PopupIndex, PopupRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PopupId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupPlacement {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub(crate) struct Popup {
    surface: WlSurface,
    xdg_surface: XdgSurface,
    role: XdgPopup,
    buffer: Option<WlBuffer>,
}

impl Client {
    #[must_use]
    pub fn popup_with_keyboard_focus(&self) -> Option<PopupId> {
        self.state.seat.keyboard_popup.map(PopupId)
    }

    #[must_use]
    pub fn popup_dismissed(&self, popup: PopupId) -> bool {
        self.state
            .popups
            .get(popup.0)
            .is_some_and(|record| record.dismissed)
    }

    pub fn create_popup(
        &mut self,
        parent: ToplevelId,
        placement: PopupPlacement,
    ) -> Result<PopupId, ClientError> {
        let handle = self.queue.handle();
        let index = PopupIndex(self.popups.len());
        let positioner = self.wm_base.create_positioner(&handle, ());
        positioner.set_size(placement.width, placement.height);
        positioner.set_anchor_rect(placement.x, placement.y, 1, 1);
        positioner.set_anchor(Anchor::TopLeft);
        positioner.set_gravity(Gravity::BottomRight);
        positioner
            .set_constraint_adjustment(ConstraintAdjustment::SlideX | ConstraintAdjustment::SlideY);
        let surface = self.compositor.create_surface(&handle, index);
        let xdg_surface = self.wm_base.get_xdg_surface(&surface, &handle, index);
        let parent = self.xdg_surface_of(parent)?;
        let role = xdg_surface.get_popup(Some(parent), &positioner, &handle, index);
        positioner.destroy();
        self.state.popups.push(PopupRecord::default());
        self.popups.push(Some(Popup {
            surface,
            xdg_surface,
            role,
            buffer: None,
        }));
        Ok(PopupId(index.0))
    }

    pub fn grab_popup(&mut self, popup: PopupId) -> Result<(), ClientError> {
        let serial = self.button_serial().ok_or(ClientError::NoButtonSerial)?;
        self.grab_popup_with(popup, serial)
    }

    pub fn grab_popup_with(&mut self, popup: PopupId, serial: u32) -> Result<(), ClientError> {
        self.popup(popup)?.role.grab(&self.seat, serial);
        self.roundtrip()
    }

    pub fn map_popup(&mut self, popup: PopupId) -> Result<PopupConfigure, ClientError> {
        self.popup(popup)?.surface.commit();
        let (serial, configure) = self.wait_for_popup_configure(popup)?;
        let buffer = buffer::create(
            &self.shm,
            &self.queue.handle(),
            configure.width.unsigned_abs(),
            configure.height.unsigned_abs(),
        )?;
        let target = self.popup_mut(popup)?;
        target.xdg_surface.ack_configure(serial);
        target.surface.attach(Some(&buffer), 0, 0);
        target.surface.commit();
        target.buffer = Some(buffer);
        self.roundtrip()?;
        Ok(configure)
    }

    pub fn destroy_popup(&mut self, popup: PopupId) -> Result<(), ClientError> {
        let target = self
            .popups
            .get_mut(popup.0)
            .and_then(Option::take)
            .ok_or(ClientError::DestroyedPopup)?;
        target.role.destroy();
        target.xdg_surface.destroy();
        target.surface.destroy();
        if let Some(buffer) = target.buffer {
            buffer.destroy();
        }
        self.roundtrip()
    }

    fn wait_for_popup_configure(
        &mut self,
        popup: PopupId,
    ) -> Result<(u32, PopupConfigure), ClientError> {
        let started = Instant::now();
        loop {
            self.roundtrip()?;
            let configured = self
                .state
                .popups
                .get_mut(popup.0)
                .and_then(|record| record.configured.take());
            if let Some(configured) = configured {
                return Ok(configured);
            }
            if started.elapsed() > WAIT_LIMIT {
                return Err(ClientError::Timeout("a popup configure"));
            }
            thread::sleep(WAIT_STEP);
        }
    }

    fn popup(&self, popup: PopupId) -> Result<&Popup, ClientError> {
        self.popups
            .get(popup.0)
            .and_then(Option::as_ref)
            .ok_or(ClientError::DestroyedPopup)
    }

    fn popup_mut(&mut self, popup: PopupId) -> Result<&mut Popup, ClientError> {
        self.popups
            .get_mut(popup.0)
            .and_then(Option::as_mut)
            .ok_or(ClientError::DestroyedPopup)
    }
}
