# Core protocol

The five globals of `wayland.xml`. Smithay 0.7 does the wire handling for all of them. Each section says what Tilling does with the interface, where a client can get an error and which tests cover it.

## wl_compositor

Version 5, owned by `protocol/compositor.rs`.

Creates surfaces and regions. Every commit imports the attached buffer, hands the root surface of the tree to the shell in `shell/lifecycle.rs` and asks for a frame. Hit testing in `shell/hit_test.rs` honours the input region: a pointer outside it does not enter the surface.

Version 6 adds the preferred buffer scale and transform events. The compositor does not send them, so the global stays at 5.

Errors: none of its own. Smithay raises the role and state errors of `wl_surface`, and no test asserts them.

Tests: `lifecycle::first_buffer_maps_and_focuses_the_window`, `pointer::pointer_outside_the_input_region_does_not_reach_the_window`, `frames::commit_without_new_content_is_presented_without_damage`, `frames::new_content_damages_only_the_window_that_committed_it`.

## wl_subcompositor

Version 1, owned by `protocol/compositor.rs`.

A subsurface belongs to its window. It is drawn and hit-tested with the surface tree of the toplevel and never takes a tile. A synchronized subsurface shows its content when the parent commits. Pointer input is limited to the content area of the window, even where a subsurface extends past it.

Errors: `bad_surface` and `bad_parent` come from Smithay, and no test asserts them.

Tests: `subsurfaces::subsurface_takes_the_pointer_only_after_its_parent_commits`, `subsurfaces::destroyed_subsurface_gives_the_pointer_back_to_its_window`, `subsurfaces::window_destroyed_before_its_subsurface_leaves_the_rest_tiled`.

## wl_shm

Version 2, owned by `protocol/compositor.rs`. The state is created in `runtime/state.rs`.

Shared memory is the only buffer type; there is no dmabuf global. The formats are the two the protocol makes mandatory, `argb8888` and `xrgb8888`. When a client commits a new buffer, the one it replaced is released.

Errors: a buffer that does not fit in its pool is refused with `wl_shm.invalid_stride`, posted on the `wl_shm_pool`. Only the client that sent it is disconnected.

Tests: `lifecycle::buffer_larger_than_its_pool_disconnects_only_the_client_that_sent_it`, `frames::replaced_buffer_is_released_to_the_client`.

## wl_seat

Version 9, owned by `protocol/seat.rs`. The events are handled in `input/`.

One seat, `seat0`, with a keyboard and a pointer. Touch is not advertised.

The keymap, the repeat rate and the repeat delay come from the configuration. A key that triggers a binding is not forwarded to clients; any other key goes to the focused surface. Keys held when the host window loses focus stop counting as modifiers.

The pointer enters the surface under it, searched from the top of the scene down: popups first, then the content of each window. Borders and gaps belong to no client. A click focuses the window under the pointer and is still delivered. The cursor surface a client sets is drawn by the compositor.

Errors: none of its own, and no test asserts the ones Smithay raises.

Tests: `keyboard::unbound_key_reaches_the_focused_client`, `keyboard::shortcut_moves_focus_without_leaking_its_key_to_clients`, `keyboard::key_held_when_the_host_window_loses_focus_no_longer_modifies_shortcuts`, `pointer::pointer_enters_the_surface_under_it_with_surface_local_coordinates`, `pointer::gaps_and_borders_do_not_deliver_pointer_focus`, `pointer::clicking_a_window_focuses_it_and_still_delivers_the_click`, `pointer::scroll_reaches_the_surface_under_the_pointer`, `frames::client_cursor_is_repainted_only_where_it_was_and_where_it_went`.

## wl_output

Version 4, owned by `protocol/output.rs`. Outputs are created in `runtime/outputs.rs`.

One global for each output of the backend: the virtual output in headless mode, the host window in nested mode. It reports mode, scale, position and name. A window enters the output when it is mapped.

Errors: the interface has no request that can fail.

Tests: no test reads the `wl_output` events. `control::queries_describe_the_windows_workspaces_and_outputs_of_the_session` checks the same output through the control socket. The mode change that follows a resize of the host window can only be observed in nested mode.
