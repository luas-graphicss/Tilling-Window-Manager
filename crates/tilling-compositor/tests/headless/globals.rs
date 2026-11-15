use tilling_test_support::{DecorationMode, WmCapabilities};

use crate::harness::Harness;

#[test]
fn only_the_implemented_globals_are_advertised() {
    let harness = Harness::start();
    let client = harness.connect();

    let mut advertised: Vec<(&str, u32)> = client
        .globals()
        .iter()
        .map(|global| (global.interface.as_str(), global.version))
        .collect();
    advertised.sort_unstable();

    assert_eq!(
        advertised,
        [
            ("ext_idle_notifier_v1", 2),
            ("wl_compositor", 5),
            ("wl_data_device_manager", 3),
            ("wl_output", 4),
            ("wl_seat", 9),
            ("wl_shm", 2),
            ("wl_subcompositor", 1),
            ("wp_cursor_shape_manager_v1", 2),
            ("wp_fractional_scale_manager_v1", 1),
            ("wp_viewporter", 1),
            ("xdg_activation_v1", 1),
            ("xdg_wm_base", 6),
            ("zwlr_layer_shell_v1", 4),
            ("zwp_idle_inhibit_manager_v1", 1),
            ("zwp_pointer_constraints_v1", 1),
            ("zwp_primary_selection_device_manager_v1", 1),
            ("zwp_relative_pointer_manager_v1", 1),
            ("zwp_text_input_manager_v3", 1),
            ("zxdg_decoration_manager_v1", 1),
            ("zxdg_output_manager_v1", 3),
        ]
    );
}

#[test]
fn fullscreen_is_the_only_window_management_capability_offered() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    client.commit(window).unwrap();
    client.wait_for_configure(window).unwrap();

    assert_eq!(
        client.window_management_capabilities(window),
        Some(&[WmCapabilities::Fullscreen][..])
    );
}

#[test]
fn decoration_is_negotiated_to_the_server_side_whatever_the_client_prefers() {
    let harness = Harness::start();
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    client
        .request_decoration(window, DecorationMode::ClientSide)
        .unwrap();
    client.map(window).unwrap();

    assert_eq!(
        client.decoration_mode(window),
        Some(DecorationMode::ServerSide)
    );
}
