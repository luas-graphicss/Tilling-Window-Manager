# xdg-shell

`xdg_wm_base` version 6, owned by `protocol/xdg_shell.rs`. The behaviour lives in `shell/` and `transaction.rs`.

## Toplevels

The first commit of a toplevel has no buffer and is answered with a configure that carries the tile the window will get. The window is mapped by the commit that brings the first buffer, after that configure is acknowledged. A commit that removes the buffer unmaps it, and the surface needs a new initial configure before it returns.

A toplevel with a parent floats above it instead of taking a tile. Fullscreen is the only capability listed in `wm_capabilities`: the request is honoured, and leaving fullscreen puts the window back in its tile. `set_maximized` is answered with a configure that leaves the state as it was. `set_minimized` and `show_window_menu` are ignored.

Closing a window sends `xdg_toplevel.close` and waits for the client. The compositor does not kill a process that fails to answer.

Tests: `lifecycle::initial_commit_is_configured_with_the_tile_the_window_will_get`, `lifecycle::first_buffer_maps_and_focuses_the_window`, `lifecycle::unmapped_window_needs_a_new_initial_configure_before_it_returns`, `lifecycle::windows_destroyed_out_of_order_leave_the_rest_tiled`, `lifecycle::disconnecting_client_takes_its_windows_with_it`, `lifecycle::repeated_map_and_destroy_cycles_do_not_accumulate_resources`, `tiling::dialog_floats_above_its_parent_instead_of_taking_a_tile`, `tiling::fullscreen_covers_the_output_and_returns_to_the_previous_tile`, `globals::fullscreen_is_the_only_window_management_capability_offered`, `keyboard::close_shortcut_asks_the_client_instead_of_removing_the_window`.

## Configure transactions

A layout change that resizes several windows is shown only when each of them has acknowledged its configure and committed. A client that never answers holds the layout until the transaction deadline and no longer. An acknowledgement of an older serial does not count.

Tests: `transactions::layout_is_published_together_once_every_resized_window_commits`, `transactions::client_that_ignores_configure_only_delays_the_layout_until_the_deadline`, `transactions::destroying_the_awaited_window_releases_the_transaction`, and the unit tests in `transaction.rs`, where `stale_ack_and_commit_before_ack_do_not_complete_a_participant` covers the older serial.

## Popups

A popup is placed by its positioner and slides back inside the output when it would leave it. `grab` gives it the keyboard until a click outside dismisses it with `popup_done`. Destroying a grabbed popup returns the keyboard to its window. `reposition` is answered with a new configure and `repositioned`, and has no test.

Tests: `popups::popup_is_configured_where_the_client_anchored_it`, `popups::popup_that_would_leave_the_output_slides_back_inside`, `popups::grabbed_popup_takes_the_keyboard_until_a_click_outside_dismisses_it`, `popups::destroying_a_grabbed_popup_returns_the_keyboard_to_its_window`.

## Interactive move and resize

`move` and `resize` start a pointer grab on a floating window, which follows the pointer or stretches from the grabbed edges. A tiled window keeps its tile.

Tests: `pointer::floating_window_follows_the_pointer_after_a_move_request`, `pointer::resize_request_stretches_the_grabbed_edges_and_reconfigures_the_client`, `pointer::tiled_window_keeps_its_tile_and_the_pointer_after_a_move_request`.

## Errors

- A buffer committed before the first configure is acknowledged ends the connection with `xdg_surface.not_constructed`. Smithay uses that code where the protocol names `unconfigured_buffer`. The check does not apply again after an unmap, because Smithay keeps the surface marked as configured. Test: `lifecycle::buffer_committed_before_the_first_configure_is_refused`.
- `move` or `resize` with a serial other than the button press the client received is ignored. Test: `pointer::move_request_without_the_serial_of_the_press_is_ignored`.
- `grab` with a serial that is not a recent input event dismisses the popup with `popup_done`. Test: `popups::grab_without_a_recent_input_serial_dismisses_the_popup`.
- The other errors of the protocol, such as a second role on a surface, an incomplete positioner or a defunct role object, are raised by Smithay, and no test asserts them.
