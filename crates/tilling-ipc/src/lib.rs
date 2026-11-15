pub mod client;
pub mod endpoint;
pub mod server;

mod credentials;
mod document;
mod event;
mod framing;
mod inbound;
mod request;
mod resource;
mod response;
mod version;

pub use credentials::{ForeignPeer, PeerError};
pub use document::{DocumentError, MAX_DEPTH, decode, encode};
pub use event::{Event, EventFrame};
pub use framing::{FrameDecoder, FrameError, MAX_FRAME_BYTES, encode_frame};
pub use request::{Call, Request};
pub use resource::{
    Direction, Layout, OutputChange, OutputMode, OutputPosition, OutputTransform, ResourceId,
    WindowMode,
};
pub use response::{
    ConfigInfo, DiagnosticsInfo, Outcome, OutputInfo, Reply, Response, ServerMessage, Snapshot,
    VersionInfo, WindowInfo, WorkspaceInfo,
};
pub use version::{
    Capability, HandshakeReply, Hello, IncompatibleVersion, PROTOCOL_VERSION, ProtocolVersion,
    Welcome, negotiate,
};
