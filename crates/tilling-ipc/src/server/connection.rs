use std::io;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::net::UnixStream;

use serde::Deserialize;

use super::outbox::{Outbox, Overflow, Priority};
use crate::document::{DocumentError, decode, encode};
use crate::event::{Event, EventFrame};
use crate::framing::FrameError;
use crate::inbound::{Inbound, StreamError};
use crate::request::Request;
use crate::response::{Outcome, Response, ServerMessage};
use crate::version::{Capability, HandshakeReply, Hello, IncompatibleVersion, negotiate};

const UNSUPPORTED_REQUEST: &str = "request does not match a supported method and its parameters";

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("peer closed the connection")]
    Closed,
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error(transparent)]
    Incompatible(#[from] IncompatibleVersion),
    #[error("socket failure")]
    Socket(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("subscriber stopped reading its events")]
    EventQueueFull,
    #[error("peer stopped reading its replies")]
    Congested,
    #[error(transparent)]
    Document(#[from] DocumentError),
}

#[derive(Debug)]
enum Session {
    AwaitingHello,
    Established { capabilities: Vec<Capability> },
}

#[derive(Deserialize)]
struct Correlation {
    request_id: u64,
}

#[derive(Debug)]
pub struct Connection {
    stream: UnixStream,
    inbound: Inbound,
    outbox: Outbox,
    session: Session,
}

impl Connection {
    pub(super) fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            inbound: Inbound::default(),
            outbox: Outbox::default(),
            session: Session::AwaitingHello,
        }
    }

    pub fn receive(&mut self, capacity: usize) -> Result<Vec<Request>, ConnectionError> {
        if self.inbound.is_empty() {
            self.fill()?;
        }
        if matches!(self.session, Session::AwaitingHello) {
            let Some(frame) = self.inbound.pop() else {
                return Ok(Vec::new());
            };
            self.establish(&frame)?;
        }
        self.take_requests(capacity)
    }

    #[must_use]
    pub fn has_undelivered_requests(&self) -> bool {
        !self.inbound.is_empty()
    }

    pub fn send(&mut self, message: &ServerMessage) -> Result<(), SendError> {
        let priority = match message {
            ServerMessage::Response(_) => Priority::Reply,
            ServerMessage::Event(EventFrame {
                event: Event::EventsDropped,
                ..
            }) => Priority::Final,
            ServerMessage::Event(_) => Priority::Event,
        };
        Ok(self.outbox.push(encode(message)?, priority)?)
    }

    pub fn flush(&mut self) -> Result<bool, ConnectionError> {
        Ok(self.outbox.drain_into(&mut &self.stream)?)
    }

    #[must_use]
    pub fn wants_write(&self) -> bool {
        !self.outbox.is_empty()
    }

    #[must_use]
    pub fn grants(&self, capability: Capability) -> bool {
        match &self.session {
            Session::AwaitingHello => false,
            Session::Established { capabilities } => capabilities.contains(&capability),
        }
    }

    fn fill(&mut self) -> Result<(), ConnectionError> {
        match self.inbound.fill(&mut &self.stream) {
            Err(StreamError::Socket(error)) if is_transient(&error) => Ok(()),
            outcome => Ok(outcome?),
        }
    }

    fn establish(&mut self, frame: &[u8]) -> Result<(), ConnectionError> {
        let hello: Hello = decode(frame)?;
        match negotiate(&hello) {
            Ok(welcome) => {
                let capabilities = welcome.capabilities.clone();
                self.queue_handshake(&HandshakeReply::Accepted(welcome))?;
                self.session = Session::Established { capabilities };
                Ok(())
            }
            Err(incompatible) => {
                self.queue_handshake(&HandshakeReply::Incompatible(incompatible))?;
                self.flush()?;
                Err(incompatible.into())
            }
        }
    }

    fn queue_handshake(&mut self, reply: &HandshakeReply) -> Result<(), ConnectionError> {
        let frame = encode(reply)?;
        self.outbox
            .push(frame, Priority::Final)
            .map_err(|overflow| io::Error::other(overflow).into())
    }

    fn take_requests(&mut self, capacity: usize) -> Result<Vec<Request>, ConnectionError> {
        let mut requests = Vec::new();
        while requests.len() < capacity {
            let Some(frame) = self.inbound.pop() else {
                break;
            };
            match decode(&frame) {
                Ok(request) => requests.push(request),
                Err(mismatch) => self.reject(&frame, mismatch)?,
            }
        }
        Ok(requests)
    }

    fn reject(&mut self, frame: &[u8], mismatch: DocumentError) -> Result<(), ConnectionError> {
        let Ok(Correlation { request_id }) = decode(frame) else {
            return Err(mismatch.into());
        };
        let refusal = ServerMessage::Response(Response {
            request_id,
            outcome: Outcome::ValidationError {
                message: UNSUPPORTED_REQUEST.to_owned(),
            },
        });
        self.send(&refusal).map_err(|_| mismatch.into())
    }
}

impl AsFd for Connection {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.stream.as_fd()
    }
}

impl From<StreamError> for ConnectionError {
    fn from(failure: StreamError) -> Self {
        match failure {
            StreamError::Closed => Self::Closed,
            StreamError::Frame(violation) => Self::Frame(violation),
            StreamError::Socket(error) => Self::Socket(error),
        }
    }
}

impl From<Overflow> for SendError {
    fn from(overflow: Overflow) -> Self {
        match overflow {
            Overflow::Events => Self::EventQueueFull,
            Overflow::Bytes => Self::Congested,
        }
    }
}

fn is_transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}
