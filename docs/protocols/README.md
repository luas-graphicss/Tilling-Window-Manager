# Wayland protocols

Tilling advertises twenty globals to every client on every backend, two more on the native one, and four only to programs it starts itself. This directory records, for each one, the version it is capped at, the module that owns it, the errors a client can run into and the tests that pin the behaviour.

| Global | Version | Owner | Reference |
| --- | --- | --- | --- |
| `wl_compositor` | 5 | `protocol/compositor.rs` | [core.md](core.md#wl_compositor) |
| `wl_subcompositor` | 1 | `protocol/compositor.rs` | [core.md](core.md#wl_subcompositor) |
| `wl_shm` | 2 | `protocol/compositor.rs` | [core.md](core.md#wl_shm) |
| `wl_seat` | 9 | `protocol/seat.rs` | [core.md](core.md#wl_seat) |
| `wl_output` | 4 | `protocol/output.rs` | [core.md](core.md#wl_output) |
| `xdg_wm_base` | 6 | `protocol/xdg_shell.rs` | [xdg-shell.md](xdg-shell.md) |
| `zxdg_decoration_manager_v1` | 1 | `protocol/decoration.rs` | [xdg-decoration.md](xdg-decoration.md) |
| `wp_viewporter` | 1 | `protocol.rs` | [output-scaling.md](output-scaling.md#wp_viewporter) |
| `wp_fractional_scale_manager_v1` | 1 | `protocol/fractional_scale.rs` | [output-scaling.md](output-scaling.md#wp_fractional_scale_manager_v1) |
| `zxdg_output_manager_v1` | 3 | `protocol/output.rs` | [output-scaling.md](output-scaling.md#zxdg_output_manager_v1) |
| `zwlr_layer_shell_v1` | 4 | `protocol/layer_shell.rs` | [desktop.md](desktop.md#layer-shell) |
| `wl_data_device_manager` | 3 | `protocol/selection.rs` | [desktop.md](desktop.md#clipboard-primary-selection-and-drag-and-drop) |
| `zwp_primary_selection_device_manager_v1` | 1 | `protocol/selection.rs` | [desktop.md](desktop.md#clipboard-primary-selection-and-drag-and-drop) |
| `xdg_activation_v1` | 1 | `protocol/activation.rs` | [desktop.md](desktop.md#activation) |
| `ext_idle_notifier_v1` | 2 | `protocol/idle.rs` | [desktop.md](desktop.md#idle) |
| `zwp_idle_inhibit_manager_v1` | 1 | `protocol/idle.rs` | [desktop.md](desktop.md#idle) |
| `zwp_pointer_constraints_v1` | 1 | `protocol/pointer_constraints.rs` | [desktop.md](desktop.md#pointer-lock) |
| `zwp_relative_pointer_manager_v1` | 1 | `protocol.rs` | [desktop.md](desktop.md#pointer-lock) |
| `wp_cursor_shape_manager_v1` | 2 | `protocol.rs` | [desktop.md](desktop.md#cursor-shape) |
| `zwp_text_input_manager_v3` | 1 | `protocol/text_input.rs` | [desktop.md](desktop.md#text-input-and-input-methods) |
| `wp_presentation` | 2 | `protocol.rs`, native backend only | [native-backend.md](native-backend.md#wp_presentation) |
| `zwp_linux_dmabuf_v1` | 5 | `protocol/dmabuf.rs`, native backend only | [native-backend.md](native-backend.md#zwp_linux_dmabuf_v1) |

Owner paths are relative to `crates/tilling-compositor/src/`. Test names are relative to `crates/tilling-compositor/tests/headless/`: `lifecycle::first_buffer_maps_and_focuses_the_window` is in `lifecycle.rs`. Those tests drive the compositor through a real Wayland socket with the client from `crates/tilling-test-support`.

Nothing else is advertised on the public socket. Session lock, data control, input methods and the virtual keyboard are offered only to programs the compositor starts with that permission; [desktop.md](desktop.md#globals-for-helpers-only) lists them. Screen capture is not implemented, and X11 clients have no XWayland to connect to.

`globals::only_the_implemented_globals_are_advertised` compares the registry a client receives on the headless backend with the rows of the table that are not marked as native only. Run it with:

    cargo test -p tilling-compositor --test headless globals::

`cargo xtask scenario nested-lifecycle` runs it together with the lifecycle, subsurface and popup tests inside a private runtime directory.

## Adding a global

A global goes on the wire only when its requests do something. Smithay has a delegate macro for most protocols, and delegating one is not the same as implementing it.

1. Write the handler in a module under `protocol/` and delegate it in `protocol.rs`.
2. Create the global next to the others in `runtime/state.rs`, at the highest version whose requests and events are all handled.
3. Add the interface and its version to the list in `tests/headless/globals.rs`.
4. Cover the behaviour and the errors a client can trigger with tests that use `tilling-test-support`.
5. Add a section here with the same fields as the existing ones.
