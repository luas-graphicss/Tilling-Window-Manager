use tilling_compositor::Capability;
use tilling_test_support::Client;

use crate::harness::Harness;

const PRIVILEGED: [(Capability, &str); 4] = [
    (Capability::DataControl, "zwlr_data_control_manager_v1"),
    (Capability::InputMethod, "zwp_input_method_manager_v2"),
    (
        Capability::VirtualKeyboard,
        "zwp_virtual_keyboard_manager_v1",
    ),
    (Capability::SessionLock, "ext_session_lock_manager_v1"),
];

fn is_offered(client: &Client, interface: &str) -> bool {
    client
        .globals()
        .iter()
        .any(|global| global.interface == interface)
}

#[test]
fn the_public_socket_offers_no_privileged_global() {
    let harness = Harness::start();
    let client = harness.connect();

    for (_, interface) in PRIVILEGED {
        assert!(!is_offered(&client, interface), "{interface} is public");
    }
}

#[test]
fn a_helper_is_offered_exactly_the_capabilities_it_was_granted() {
    let harness = Harness::start();

    for (granted, _) in PRIVILEGED {
        let helper = harness.connect_with(&[granted]);

        for (capability, interface) in PRIVILEGED {
            assert_eq!(
                is_offered(&helper, interface),
                capability == granted,
                "{interface} for a helper granted {granted:?}"
            );
        }
    }
}

#[test]
fn a_helper_without_capabilities_is_an_ordinary_client() {
    let harness = Harness::start();
    let helper = harness.connect_with(&[]);
    let mut public = harness.connect();
    let window = public.create_toplevel("org.example.Terminal", "shell");

    public.map(window).unwrap();
    harness.published(1);

    for (_, interface) in PRIVILEGED {
        assert!(!is_offered(&helper, interface), "{interface} was granted");
    }
}
