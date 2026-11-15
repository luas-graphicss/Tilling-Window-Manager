use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion { major: 1, minor: 2 };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Events,
}

impl Capability {
    pub const ALL: [Self; 1] = [Self::Events];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Events => "events",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: ProtocolVersion,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    pub protocol: ProtocolVersion,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("protocol major {requested} is not supported, this endpoint speaks major {supported}")]
pub struct IncompatibleVersion {
    pub requested: u16,
    pub supported: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HandshakeReply {
    Accepted(Welcome),
    Incompatible(IncompatibleVersion),
}

impl ProtocolVersion {
    pub fn common(self, peer: Self) -> Result<Self, IncompatibleVersion> {
        if peer.major != self.major {
            return Err(IncompatibleVersion {
                requested: peer.major,
                supported: self.major,
            });
        }
        Ok(Self {
            major: self.major,
            minor: self.minor.min(peer.minor),
        })
    }
}

pub fn negotiate(hello: &Hello) -> Result<Welcome, IncompatibleVersion> {
    let protocol = PROTOCOL_VERSION.common(hello.protocol)?;
    let requested = |capability: &Capability| {
        hello
            .capabilities
            .iter()
            .any(|name| name == capability.name())
    };
    Ok(Welcome {
        protocol,
        capabilities: Capability::ALL.into_iter().filter(requested).collect(),
    })
}

impl From<Result<Welcome, IncompatibleVersion>> for HandshakeReply {
    fn from(outcome: Result<Welcome, IncompatibleVersion>) -> Self {
        match outcome {
            Ok(welcome) => Self::Accepted(welcome),
            Err(incompatible) => Self::Incompatible(incompatible),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(major: u16, minor: u16, capabilities: &[&str]) -> Hello {
        Hello {
            protocol: ProtocolVersion { major, minor },
            capabilities: capabilities.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    #[test]
    fn matching_major_settles_on_the_lower_minor() {
        let endpoint = ProtocolVersion { major: 1, minor: 3 };
        let older = ProtocolVersion { major: 1, minor: 1 };
        let newer = ProtocolVersion { major: 1, minor: 7 };

        assert_eq!(endpoint.common(older), Ok(older));
        assert_eq!(endpoint.common(newer), Ok(endpoint));
        assert_eq!(
            negotiate(&hello(1, 7, &[])).unwrap().protocol,
            PROTOCOL_VERSION
        );
    }

    #[test]
    fn other_majors_are_refused_instead_of_approximated() {
        for major in [0, 2] {
            assert_eq!(
                negotiate(&hello(major, 0, &["events"])),
                Err(IncompatibleVersion {
                    requested: major,
                    supported: 1,
                })
            );
        }
    }

    #[test]
    fn only_requested_and_known_capabilities_are_granted() {
        let granted = negotiate(&hello(1, 0, &["telepathy", "events"])).unwrap();
        let none = negotiate(&hello(1, 0, &["telepathy"])).unwrap();

        assert_eq!(granted.capabilities, [Capability::Events]);
        assert!(none.capabilities.is_empty());
    }

    #[test]
    fn handshake_reply_carries_either_outcome() {
        let accepted = HandshakeReply::from(negotiate(&hello(1, 0, &["events"])));
        let refused = HandshakeReply::from(negotiate(&hello(9, 0, &[])));

        assert_eq!(
            serde_json::to_string(&accepted).unwrap(),
            r#"{"status":"accepted","protocol":{"major":1,"minor":0},"capabilities":["events"]}"#
        );
        assert_eq!(
            serde_json::to_string(&refused).unwrap(),
            r#"{"status":"incompatible","requested":9,"supported":1}"#
        );
    }
}
