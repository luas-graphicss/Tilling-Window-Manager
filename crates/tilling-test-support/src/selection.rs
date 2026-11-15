use std::io::{ErrorKind, PipeReader, Read, Write};
use std::os::fd::AsFd;
use std::thread;
use std::time::Instant;

use rustix::fs::{OFlags, fcntl_setfl};
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::WlDataDeviceManager;
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::wl_data_source::{self, WlDataSource};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop, event_created_child};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1;
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_device_v1::{
    self, ZwpPrimarySelectionDeviceV1,
};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_offer_v1::{
    self, ZwpPrimarySelectionOfferV1,
};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1;

use crate::client::{Client, ToplevelId, WAIT_LIMIT, WAIT_STEP};
use crate::error::ClientError;
use crate::state::ClientState;

const TRANSFER_CHUNK_BYTES: usize = 4096;

#[derive(Debug, Default)]
pub(crate) struct SelectionLog {
    device: Option<WlDataDevice>,
    offered: Option<(WlDataSource, Vec<u8>)>,
    cancelled_sources: usize,
    introduced: Vec<(WlDataOffer, Vec<String>)>,
    clipboard: Option<WlDataOffer>,
    dragged_over: Option<WlDataOffer>,
    primary_device: Option<ZwpPrimarySelectionDeviceV1>,
    primary_source: Option<ZwpPrimarySelectionSourceV1>,
    primary_introduced: Vec<(ZwpPrimarySelectionOfferV1, Vec<String>)>,
    primary: Option<ZwpPrimarySelectionOfferV1>,
}

#[derive(Debug)]
pub struct PendingTransfer {
    incoming: PipeReader,
}

delegate_noop!(ClientState: WlDataDeviceManager);
delegate_noop!(ClientState: ZwpPrimarySelectionDeviceManagerV1);
delegate_noop!(ClientState: ignore ZwpPrimarySelectionSourceV1);

impl Dispatch<WlDataDevice, ()> for ClientState {
    fn event(
        state: &mut Self,
        _device: &WlDataDevice,
        event: wl_data_device::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let log = &mut state.selection;
        match event {
            wl_data_device::Event::DataOffer { id } => log.introduced.push((id, Vec::new())),
            wl_data_device::Event::Selection { id } => log.clipboard = id,
            wl_data_device::Event::Enter { id, .. } => log.dragged_over = id,
            wl_data_device::Event::Leave => log.dragged_over = None,
            _ => {}
        }
    }

    event_created_child!(ClientState, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}

impl Dispatch<WlDataOffer, ()> for ClientState {
    fn event(
        state: &mut Self,
        offer: &WlDataOffer,
        event: wl_data_offer::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let wl_data_offer::Event::Offer { mime_type } = event else {
            return;
        };
        let introduced = &mut state.selection.introduced;
        if let Some((_, mime_types)) = introduced.iter_mut().find(|(known, _)| known == offer) {
            mime_types.push(mime_type);
        }
    }
}

impl Dispatch<WlDataSource, ()> for ClientState {
    fn event(
        state: &mut Self,
        _source: &WlDataSource,
        event: wl_data_source::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_source::Event::Send { fd, .. } => {
                if let Some((_, content)) = &state.selection.offered {
                    let mut outgoing = std::fs::File::from(fd);
                    let _delivered = outgoing.write_all(content).is_ok();
                }
            }
            wl_data_source::Event::Cancelled => state.selection.cancelled_sources += 1,
            _ => {}
        }
    }
}

impl Dispatch<ZwpPrimarySelectionDeviceV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        _device: &ZwpPrimarySelectionDeviceV1,
        event: zwp_primary_selection_device_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let log = &mut state.selection;
        match event {
            zwp_primary_selection_device_v1::Event::DataOffer { offer } => {
                log.primary_introduced.push((offer, Vec::new()));
            }
            zwp_primary_selection_device_v1::Event::Selection { id } => log.primary = id,
            _ => {}
        }
    }

    event_created_child!(ClientState, ZwpPrimarySelectionDeviceV1, [
        zwp_primary_selection_device_v1::EVT_DATA_OFFER_OPCODE => (ZwpPrimarySelectionOfferV1, ()),
    ]);
}

impl Dispatch<ZwpPrimarySelectionOfferV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        offer: &ZwpPrimarySelectionOfferV1,
        event: zwp_primary_selection_offer_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let zwp_primary_selection_offer_v1::Event::Offer { mime_type } = event else {
            return;
        };
        let introduced = &mut state.selection.primary_introduced;
        if let Some((_, mime_types)) = introduced.iter_mut().find(|(known, _)| known == offer) {
            mime_types.push(mime_type);
        }
    }
}

impl PendingTransfer {
    pub fn finish(mut self) -> Result<Vec<u8>, ClientError> {
        let started = Instant::now();
        let mut received = Vec::new();
        let mut chunk = [0; TRANSFER_CHUNK_BYTES];
        loop {
            match self.incoming.read(&mut chunk) {
                Ok(0) => return Ok(received),
                Ok(length) => received.extend_from_slice(&chunk[..length]),
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    if started.elapsed() > WAIT_LIMIT {
                        return Err(ClientError::Timeout("the end of a data transfer"));
                    }
                    thread::sleep(WAIT_STEP);
                }
                Err(error) => return Err(ClientError::Transfer(error)),
            }
        }
    }
}

impl Client {
    pub fn watch_selections(&mut self) -> Result<(), ClientError> {
        if self.state.selection.device.is_none() {
            let manager: WlDataDeviceManager = self.bind(1..=3, "wl_data_device_manager")?;
            let device = manager.get_data_device(&self.seat, &self.queue.handle(), ());
            self.state.selection.device = Some(device);
        }
        if self.state.selection.primary_device.is_none() {
            let manager: ZwpPrimarySelectionDeviceManagerV1 =
                self.bind(1..=1, "zwp_primary_selection_device_manager_v1")?;
            let device = manager.get_device(&self.seat, &self.queue.handle(), ());
            self.state.selection.primary_device = Some(device);
        }
        self.roundtrip()
    }

    pub fn offer_clipboard(&mut self, mime_type: &str, content: &[u8]) -> Result<(), ClientError> {
        self.watch_selections()?;
        let source = self.data_source(mime_type)?;
        if let Some(device) = &self.state.selection.device {
            device.set_selection(Some(&source), self.input_serial());
        }
        self.state.selection.offered = Some((source, content.to_vec()));
        self.roundtrip()
    }

    pub fn offer_primary_selection(&mut self, mime_type: &str) -> Result<(), ClientError> {
        self.watch_selections()?;
        let manager: ZwpPrimarySelectionDeviceManagerV1 =
            self.bind(1..=1, "zwp_primary_selection_device_manager_v1")?;
        let source = manager.create_source(&self.queue.handle(), ());
        source.offer(mime_type.to_owned());
        if let Some(device) = &self.state.selection.primary_device {
            device.set_selection(Some(&source), self.input_serial());
        }
        self.state.selection.primary_source = Some(source);
        self.roundtrip()
    }

    pub fn start_drag(
        &mut self,
        origin: ToplevelId,
        mime_type: &str,
        content: &[u8],
    ) -> Result<(), ClientError> {
        self.watch_selections()?;
        let serial = self.button_serial().ok_or(ClientError::NoButtonSerial)?;
        let source = self.data_source(mime_type)?;
        let surface = self.surface_of(origin)?;
        if let Some(device) = &self.state.selection.device {
            device.start_drag(Some(&source), surface, None, serial);
        }
        self.state.selection.offered = Some((source, content.to_vec()));
        self.roundtrip()
    }

    pub fn request_clipboard(&mut self, mime_type: &str) -> Result<PendingTransfer, ClientError> {
        let offer = self
            .state
            .selection
            .clipboard
            .as_ref()
            .ok_or(ClientError::NoClipboardOffer)?;
        let (incoming, outgoing) = std::io::pipe().map_err(ClientError::Pipe)?;
        fcntl_setfl(&incoming, OFlags::NONBLOCK)
            .map_err(|error| ClientError::Pipe(error.into()))?;
        offer.receive(mime_type.to_owned(), outgoing.as_fd());
        drop(outgoing);
        self.roundtrip()?;
        Ok(PendingTransfer { incoming })
    }

    #[must_use]
    pub fn clipboard_mime_types(&self) -> Option<&[String]> {
        let log = &self.state.selection;
        mime_types_of(&log.introduced, log.clipboard.as_ref())
    }

    #[must_use]
    pub fn primary_selection_mime_types(&self) -> Option<&[String]> {
        let log = &self.state.selection;
        mime_types_of(&log.primary_introduced, log.primary.as_ref())
    }

    #[must_use]
    pub fn dragged_mime_types(&self) -> Option<&[String]> {
        let log = &self.state.selection;
        mime_types_of(&log.introduced, log.dragged_over.as_ref())
    }

    #[must_use]
    pub fn cancelled_sources(&self) -> usize {
        self.state.selection.cancelled_sources
    }

    fn data_source(&self, mime_type: &str) -> Result<WlDataSource, ClientError> {
        let manager: WlDataDeviceManager = self.bind(1..=3, "wl_data_device_manager")?;
        let source = manager.create_data_source(&self.queue.handle(), ());
        source.offer(mime_type.to_owned());
        Ok(source)
    }

    fn input_serial(&self) -> u32 {
        self.button_serial().unwrap_or_default()
    }
}

fn mime_types_of<'log, Offer: PartialEq>(
    introduced: &'log [(Offer, Vec<String>)],
    current: Option<&Offer>,
) -> Option<&'log [String]> {
    let current = current?;
    introduced
        .iter()
        .find(|(known, _)| known == current)
        .map(|(_, mime_types)| mime_types.as_slice())
}
