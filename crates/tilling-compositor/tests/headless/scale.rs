use tilling_compositor::VirtualOutput;
use tilling_core::geometry::LogicalRect;

use crate::harness::Harness;

const PRIMARY: &str = "HEADLESS-1";
const SPARE: &str = "HEADLESS-2";
const WIRE_SCALE_1_25: u32 = 150;
const WIRE_SCALE_1_5: u32 = 180;
const WIRE_SCALE_2: u32 = 240;

fn configured(outputs: &str) -> Harness {
    Harness::start_with_config(&format!("schema_version = 1\n{outputs}"))
}

fn logical(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
    LogicalRect::new(x, y, width, height)
        .unwrap_or_else(|error| panic!("invalid test area: {error}"))
}

#[test]
fn fractional_output_lays_out_windows_in_logical_pixels() {
    let harness = configured("\n[[outputs]]\nname = \"HEADLESS-1\"\nscale = 1.5\n");
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    let configure = client.map(window).unwrap();
    client.settle().unwrap();
    let placed = harness.published(1).scene.windows()[0];

    assert_eq!(placed.frame, logical(8, 8, 837, 464));
    assert_eq!(placed.content, logical(10, 10, 833, 460));
    assert_eq!((configure.width, configure.height), (833, 460));
}

#[test]
fn rotated_fractional_output_swaps_the_logical_axes() {
    let harness =
        configured("\n[[outputs]]\nname = \"HEADLESS-1\"\nscale = 1.25\ntransform = \"90\"\n");
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");

    client.map(window).unwrap();
    client.settle().unwrap();
    let placed = harness.published(1).scene.windows()[0];

    assert_eq!(placed.frame, logical(8, 8, 560, 1008));
}

#[test]
fn window_is_told_the_fractional_scale_of_its_output() {
    let harness = configured("\n[[outputs]]\nname = \"HEADLESS-1\"\nscale = 1.5\n");
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.watch_preferred_scale(window).unwrap();

    client.map(window).unwrap();
    client.settle().unwrap();

    assert_eq!(client.preferred_scale(window), Some(WIRE_SCALE_1_5));
}

#[test]
fn window_that_changes_output_is_told_the_new_scale() {
    let harness = configured(
        "\n[[outputs]]\nname = \"HEADLESS-1\"\nscale = 2.0\n\n[[outputs]]\nname = \"HEADLESS-2\"\nscale = 1.25\n",
    );
    let mut client = harness.connect();
    let window = client.create_toplevel("org.example.Terminal", "shell");
    client.watch_preferred_scale(window).unwrap();
    client.map(window).unwrap();
    client.settle().unwrap();
    let on_primary = client.preferred_scale(window);

    harness.plug(VirtualOutput {
        name: SPARE.to_owned(),
        width: 1920,
        height: 1080,
    });
    harness.unplug(PRIMARY);
    client.settle().unwrap();
    harness.published(1);
    client.roundtrip().unwrap();

    assert_eq!(on_primary, Some(WIRE_SCALE_2));
    assert_eq!(client.preferred_scale(window), Some(WIRE_SCALE_1_25));
}
