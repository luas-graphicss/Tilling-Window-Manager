mod backend;
mod desktop;
mod error;
mod geometry;
mod options;
mod protocol;
mod runtime;
mod scene;
mod shell;
mod transaction;

pub use error::CompositorError;
pub use options::{BackendKind, Options, VirtualOutput};
pub use runtime::instance::InstanceError;
pub use runtime::run;
pub use runtime::session::{Session, SessionSnapshot};
pub use scene::{BORDER_WIDTH, Scene, SceneLayer, SceneWindow};
