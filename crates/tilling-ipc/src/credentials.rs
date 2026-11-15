use std::io;
use std::os::fd::AsFd;

use rustix::net::sockopt::socket_peercred;
use rustix::process::geteuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("peer runs as uid {peer}, this socket only serves uid {owner}")]
pub struct ForeignPeer {
    pub peer: u32,
    pub owner: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error(transparent)]
    Foreign(#[from] ForeignPeer),
    #[error("read peer credentials")]
    Unavailable(#[from] io::Error),
}

pub(crate) fn effective_uid() -> u32 {
    geteuid().as_raw()
}

pub(crate) fn require_same_user(socket: impl AsFd) -> Result<(), PeerError> {
    let credentials = socket_peercred(socket).map_err(io::Error::from)?;
    Ok(authorize(credentials.uid.as_raw(), effective_uid())?)
}

fn authorize(peer: u32, owner: u32) -> Result<(), ForeignPeer> {
    if peer == owner {
        return Ok(());
    }
    Err(ForeignPeer { peer, owner })
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream;

    use super::*;

    #[test]
    fn same_user_is_authorized() {
        assert_eq!(authorize(1000, 1000), Ok(()));
    }

    #[test]
    fn another_user_is_denied_even_when_it_is_root() {
        assert_eq!(
            authorize(0, 1000),
            Err(ForeignPeer {
                peer: 0,
                owner: 1000
            })
        );
        assert_eq!(
            authorize(1001, 1000),
            Err(ForeignPeer {
                peer: 1001,
                owner: 1000
            })
        );
    }

    #[test]
    fn kernel_reports_the_peer_of_a_local_socket() {
        let (near, far) = UnixStream::pair().unwrap();

        assert!(require_same_user(&near).is_ok());
        assert!(require_same_user(&far).is_ok());
    }
}
