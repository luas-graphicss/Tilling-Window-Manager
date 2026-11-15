# Wayland protocols

Tilling advertises seven globals. This directory records, for each one, the version it is capped at, the module that owns it, the errors a client can run into and the tests that pin the behaviour.

| Global | Version | Owner | Reference |
| --- | --- | --- | --- |
| `wl_compositor` | 5 | `protocol/compositor.rs` | [core.md](core.md#wl_compositor) |
| `wl_subcompositor` | 1 | `protocol/compositor.rs` | [core.md](core.md#wl_subcompositor) |
| `wl_shm` | 2 | `protocol/compositor.rs` | [core.md](core.md#wl_shm) |
| `wl_seat` | 9 | `protocol/seat.rs` | [core.md](core.md#wl_seat) |
| `wl_output` | 4 | `protocol/output.rs` | [core.md](core.md#wl_output) |
| `xdg_wm_base` | 6 | `protocol/xdg_shell.rs` | [xdg-shell.md](xdg-shell.md) |
| `zxdg_decoration_manager_v1` | 1 | `protocol/decoration.rs` | [xdg-decoration.md](xdg-decoration.md) |

Owner paths are relative to `crates/tilling-compositor/src/`. Test names are relative to `crates/tilling-compositor/tests/headless/`: `lifecycle::first_buffer_maps_and_focuses_the_window` is in `lifecycle.rs`. Those tests drive the compositor through a real Wayland socket with the client from `crates/tilling-test-support`.

Nothing else is advertised. Clipboard and drag and drop (`wl_data_device_manager`), layer shell, input methods and session lock are not implemented, so a client that looks for them finds no global.

`globals::only_the_implemented_globals_are_advertised` compares the registry a client receives with the table above. Run it with:

    cargo test -p tilling-compositor --test headless globals::

`cargo xtask scenario nested-lifecycle` runs it together with the lifecycle, subsurface and popup tests inside a private runtime directory.

## Adding a global

A global goes on the wire only when its requests do something. Smithay has a delegate macro for most protocols, and delegating one is not the same as implementing it.

1. Write the handler in a module under `protocol/` and delegate it in `protocol.rs`.
2. Create the global next to the others in `runtime/state.rs`, at the highest version whose requests and events are all handled.
3. Add the interface and its version to the list in `tests/headless/globals.rs`.
4. Cover the behaviour and the errors a client can trigger with tests that use `tilling-test-support`.
5. Add a section here with the same fields as the existing ones.
