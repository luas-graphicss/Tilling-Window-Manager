use std::collections::VecDeque;
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::credentials::{PeerError, require_same_user};
use crate::document::{DocumentError, decode, encode};
use crate::event::EventFrame;
use crate::framing::FrameError;
use crate::inbound::{Inbound, StreamError};
use crate::request::{Call, Request};
use crate::response::{Outcome, ServerMessage};
use crate::version::{Capability, HandshakeReply, Hello, IncompatibleVersion, PROTOCOL_VERSION};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("connect to {}", .path.display())]
    Connect {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Peer(#[from] PeerError),
    #[error(transparent)]
    Incompatible(#[from] IncompatibleVersion),
    #[error("server did not answer within {} ms", .0.as_millis())]
    TimedOut(Duration),
    #[error("server closed the connection")]
    Closed,
    #[error("server answered request {request_id}, which is not pending")]
    UnexpectedResponse { request_id: u64 },
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("socket failure")]
    Socket(#[from] io::Error),
}

#[derive(Debug)]
pub struct Client {
    stream: UnixStream,
    inbound: Inbound,
    early_events: VecDeque<EventFrame>,
    last_request_id: u64,
    timeout: Duration,
}

impl Client {
    pub fn connect(socket: &Path, timeout: Duration) -> Result<Self, ClientError> {
        let stream = UnixStream::connect(socket).map_err(|source| ClientError::Connect {
            path: socket.to_owned(),
            source,
        })?;
        require_same_user(&stream)?;
        stream.set_write_timeout(Some(timeout))?;
        let mut client = Self {
            stream,
            inbound: Inbound::default(),
            early_events: VecDeque::new(),
            last_request_id: 0,
            timeout,
        };
        client.handshake()?;
        Ok(client)
    }

    pub fn call(
        &mut self,
        call: Call,
        expected_revision: Option<u64>,
    ) -> Result<Outcome, ClientError> {
        self.last_request_id += 1;
        let request_id = self.last_request_id;
        self.write(&Request {
            request_id,
            expected_revision,
            call,
        })?;
        let deadline = Instant::now() + self.timeout;
        loop {
            match self.read(Some(deadline))? {
                ServerMessage::Event(event) => self.early_events.push_back(event),
                ServerMessage::Response(response) if response.request_id == request_id => {
                    return Ok(response.outcome);
                }
                ServerMessage::Response(response) => {
                    return Err(ClientError::UnexpectedResponse {
                        request_id: response.request_id,
                    });
                }
            }
        }
    }

    pub fn next_event(&mut self) -> Result<EventFrame, ClientError> {
        if let Some(event) = self.early_events.pop_front() {
            return Ok(event);
        }
        match self.read(None)? {
            ServerMessage::Event(event) => Ok(event),
            ServerMessage::Response(response) => Err(ClientError::UnexpectedResponse {
                request_id: response.request_id,
            }),
        }
    }

    fn handshake(&mut self) -> Result<(), ClientError> {
        self.write(&Hello {
            protocol: PROTOCOL_VERSION,
            capabilities: Capability::ALL
                .iter()
                .map(|capability| capability.name().to_owned())
                .collect(),
        })?;
        let deadline = Instant::now() + self.timeout;
        match self.read(Some(deadline))? {
            HandshakeReply::Accepted(_) => Ok(()),
            HandshakeReply::Incompatible(incompatible) => Err(incompatible.into()),
        }
    }

    fn write(&mut self, message: &impl Serialize) -> Result<(), ClientError> {
        let frame = encode(message)?;
        self.stream
            .write_all(&frame)
            .map_err(|error| self.classify(error))
    }

    fn read<T: DeserializeOwned>(&mut self, deadline: Option<Instant>) -> Result<T, ClientError> {
        loop {
            if let Some(frame) = self.inbound.pop() {
                return Ok(decode(&frame)?);
            }
            self.stream.set_read_timeout(self.remaining(deadline)?)?;
            match self.inbound.fill(&mut &self.stream) {
                Ok(()) => {}
                Err(StreamError::Closed) => return Err(ClientError::Closed),
                Err(StreamError::Frame(violation)) => return Err(violation.into()),
                Err(StreamError::Socket(error)) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(StreamError::Socket(error)) => return Err(self.classify(error)),
            }
        }
    }

    fn remaining(&self, deadline: Option<Instant>) -> Result<Option<Duration>, ClientError> {
        let Some(deadline) = deadline else {
            return Ok(None);
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ClientError::TimedOut(self.timeout));
        }
        Ok(Some(remaining))
    }

    fn classify(&self, error: io::Error) -> ClientError {
        match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
                ClientError::TimedOut(self.timeout)
            }
            _ => ClientError::Socket(error),
        }
    }
}
