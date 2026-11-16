use smithay::reexports::wayland_server::Client;

use crate::runtime::ClientState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    DataControl,
    InputMethod,
    VirtualKeyboard,
    SessionLock,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Privileges(u8);

impl Capability {
    const fn bit(self) -> u8 {
        match self {
            Self::DataControl => 1,
            Self::InputMethod => 1 << 1,
            Self::VirtualKeyboard => 1 << 2,
            Self::SessionLock => 1 << 3,
        }
    }
}

impl Privileges {
    pub(crate) const fn allows(self, capability: Capability) -> bool {
        self.0 & capability.bit() != 0
    }
}

impl FromIterator<Capability> for Privileges {
    fn from_iter<Granted: IntoIterator<Item = Capability>>(granted: Granted) -> Self {
        let bits = granted
            .into_iter()
            .fold(0, |bits, capability| bits | capability.bit());
        Self(bits)
    }
}

pub(crate) fn client_may(client: &Client, capability: Capability) -> bool {
    client
        .get_data::<ClientState>()
        .is_some_and(|state| state.privileges.allows(capability))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_granted_capabilities_are_allowed() {
        let privileges: Privileges = [Capability::SessionLock, Capability::DataControl]
            .into_iter()
            .collect();

        assert!(privileges.allows(Capability::SessionLock));
        assert!(privileges.allows(Capability::DataControl));
        assert!(!privileges.allows(Capability::InputMethod));
        assert!(!privileges.allows(Capability::VirtualKeyboard));
    }

    #[test]
    fn a_public_connection_holds_no_capability() {
        let privileges = Privileges::default();

        assert!(!privileges.allows(Capability::DataControl));
        assert!(!privileges.allows(Capability::SessionLock));
    }
}
