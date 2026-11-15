mod directory;
mod instance;
mod runtime_directory;
mod socket_file;

pub use directory::DirectoryError;
pub use instance::{InstanceName, InvalidInstanceName};
pub use runtime_directory::{DiscoveryError, RuntimeDirectory, RuntimeDirectoryError};
pub use socket_file::SocketFileError;

pub(crate) use directory::require_private;
pub(crate) use socket_file::{Occupancy, occupancy};

pub const SOCKET_VARIABLE: &str = "TILLING_SOCKET";
