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
            ("wl_compositor", 5),
            ("wl_output", 4),
            ("wl_seat", 9),
            ("wl_shm", 2),
            ("wl_subcompositor", 1),
            ("xdg_wm_base", 6),
            ("zxdg_decoration_manager_v1", 1),
        ]
    );
}
