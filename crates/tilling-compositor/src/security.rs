mod privileges;

pub use privileges::Capability;
pub(crate) use privileges::{Privileges, client_may};
