use std::collections::BTreeMap;

use smithay::reexports::calloop::RegistrationToken;
use tilling_ipc::server::Connection;

pub(super) const MAX_PEERS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PeerId(u64);

#[derive(Debug)]
pub(super) struct Peer {
    pub(super) connection: Connection,
    pub(super) reader: RegistrationToken,
    pub(super) writer: Option<RegistrationToken>,
    pub(super) subscribed: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ControlPeers {
    peers: BTreeMap<PeerId, Peer>,
    last_id: u64,
    sequence: u64,
}

impl ControlPeers {
    pub(crate) fn len(&self) -> usize {
        self.peers.len()
    }

    pub(crate) fn subscriber_count(&self) -> usize {
        self.peers.values().filter(|peer| peer.subscribed).count()
    }

    pub(super) fn is_full(&self) -> bool {
        self.peers.len() >= MAX_PEERS
    }

    pub(super) fn reserve(&mut self) -> PeerId {
        self.last_id += 1;
        PeerId(self.last_id)
    }

    pub(super) fn insert(&mut self, id: PeerId, connection: Connection, reader: RegistrationToken) {
        let peer = Peer {
            connection,
            reader,
            writer: None,
            subscribed: false,
        };
        self.peers.insert(id, peer);
    }

    pub(super) fn get_mut(&mut self, id: PeerId) -> Option<&mut Peer> {
        self.peers.get_mut(&id)
    }

    pub(super) fn remove(&mut self, id: PeerId) -> Option<Peer> {
        self.peers.remove(&id)
    }

    pub(super) fn subscribers(&self) -> Vec<PeerId> {
        self.peers
            .iter()
            .filter(|(_, peer)| peer.subscribed)
            .map(|(id, _)| *id)
            .collect()
    }

    pub(super) fn sequence(&self) -> u64 {
        self.sequence
    }

    pub(super) fn advance_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }
}
