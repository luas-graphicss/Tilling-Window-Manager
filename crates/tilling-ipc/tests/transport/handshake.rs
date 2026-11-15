use tilling_ipc::server::ConnectionError;
use tilling_ipc::{
    Capability, HandshakeReply, IncompatibleVersion, PROTOCOL_VERSION, ProtocolVersion, Welcome,
    encode_frame,
};

use crate::support::{Peer, Scratch, accept, bind, eventually, flush, hello, receive};

#[test]
fn compatible_peer_is_welcomed_with_the_capabilities_it_asked_for() {
    let scratch = Scratch::new();
    let listener = bind(&scratch);
    let mut peer = Peer::connect(&scratch.socket());
    let mut connection = accept(&listener);
    assert!(!connection.grants(Capability::Events));

    peer.greet();
    eventually(|| {
        receive(&mut connection, 0);
        connection.grants(Capability::Events).then_some(())
    });
    flush(&mut connection);

    assert_eq!(
        peer.receive::<HandshakeReply>(),
        HandshakeReply::Accepted(Welcome {
            protocol: PROTOCOL_VERSION,
            capabilities: vec![Capability::Events],
        })
    );
}

#[test]
fn other_major_gets_a_refusal_before_the_connection_is_dropped() {
    let scratch = Scratch::new();
    let listener = bind(&scratch);
    let mut peer = Peer::connect(&scratch.socket());
    let mut connection = accept(&listener);
    let refusal = IncompatibleVersion {
        requested: 9,
        supported: PROTOCOL_VERSION.major,
    };

    peer.send(&hello(ProtocolVersion { major: 9, minor: 0 }));
    let failure = eventually(|| connection.receive(8).err());
    drop(connection);

    assert!(matches!(failure, ConnectionError::Incompatible(found) if found == refusal));
    assert_eq!(
        peer.receive::<HandshakeReply>(),
        HandshakeReply::Incompatible(refusal)
    );
}

#[test]
fn garbage_instead_of_hello_closes_the_connection() {
    let scratch = Scratch::new();
    let listener = bind(&scratch);
    let mut peer = Peer::connect(&scratch.socket());
    let mut connection = accept(&listener);

    peer.write(&encode_frame(b"not a hello").unwrap());
    let failure = eventually(|| connection.receive(8).err());

    assert!(matches!(failure, ConnectionError::Document(_)));
}
