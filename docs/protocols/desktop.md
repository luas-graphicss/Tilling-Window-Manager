# Desktop protocols

The globals that panels, launchers, clipboards, lockers and input methods need. Ten are offered to every client. Four are offered only over a connection the compositor opens for a program it starts itself; [docs/user/session.md](../user/session.md) shows how a program gets one.

Smithay 0.7.0 implements the wire side of every protocol on this page. What is listed under "Tests" is what this repository pins; where it says that no test asserts a behaviour, the behaviour is Smithay's and has only been reasoned about.

## Layer shell

`zwlr_layer_shell_v1` version 4, owned by `protocol/layer_shell.rs` and `layers/`.

A layer surface is placed on the output it names, or on the output that shows the active workspace when it names none. The space it reserves with an exclusive zone is taken from the area the layouts tile, on any edge and for several surfaces at once, and comes back when the surface is destroyed. A fullscreen window covers the whole output, reserved space included.

A surface that asks for the keyboard exclusively, as a launcher does, gets it and gives it back to the window that had it when it closes. A surface that does not ask never takes the keyboard. The pointer over a layer surface belongs to that surface.

Every client can create a layer surface. A client can therefore cover the windows or take the keyboard the way a launcher does; see [docs/security](../security/README.md#wayland-clients).

Tests: `layers::a_bar_on_any_edge_keeps_tiles_out_of_its_exclusive_zone`, `layers::bars_on_two_edges_reserve_both_zones`, `layers::a_destroyed_bar_returns_its_zone_to_the_tiles`, `layers::a_fullscreen_window_covers_the_zone_of_a_bar`, `layers::the_pointer_over_a_bar_belongs_to_the_bar`, `layers::a_launcher_takes_the_keyboard_and_returns_it_when_it_closes`, `layers::a_bar_does_not_take_the_keyboard_from_the_focused_window`. The reserved area itself is tested in `tilling-core`: `outputs::reserved_edges_keep_tiles_out_but_not_fullscreen_windows` and the two tests after it.

Popups of layer surfaces, the overlay layer above a fullscreen window and fractional scales have no test.

## Clipboard, primary selection and drag and drop

`wl_data_device_manager` version 3 and `zwp_primary_selection_device_manager_v1` version 1, owned by `protocol/selection.rs`.

The clipboard and the primary selection are offered to the client that holds the keyboard, and only that client can replace them. Content is copied between the two clients through a pipe; the compositor does not keep it, so the clipboard empties when its owner disconnects unless a clipboard manager holds a copy.

A drag is offered to the window under the pointer, shows the icon the client attached, and is cancelled when the target refuses it.

Tests: `selection::clipboard_content_travels_to_the_client_that_takes_the_keyboard`, `selection::the_clipboard_empties_when_its_owner_disconnects`, `selection::a_client_without_the_keyboard_cannot_replace_the_clipboard`, `selection::the_primary_selection_is_offered_to_the_client_that_takes_the_keyboard`, `selection::a_drag_is_offered_to_the_window_under_the_pointer_and_cancelled_when_refused`.

A large transfer, a reader that stops reading and a drop that is accepted have no test.

## Activation

`xdg_activation_v1` version 1, owned by `protocol/activation.rs`.

A token moves the keyboard to another window only when the client that asked for it held the keyboard and named the serial of a real key press or click. A token works once. A token without a serial, a token asked for by a client without the keyboard and a token nobody issued change nothing.

Tests: the five tests of `activation::`.

## Idle

`ext_idle_notifier_v1` version 2 and `zwp_idle_inhibit_manager_v1` version 1, owned by `protocol/idle.rs` and `idle/`.

A client learns when no input arrived for the time it named and when input returns. A window that inhibits idleness keeps the session awake while it is shown, and not while its workspace is hidden.

Tests: the three tests of `idle::`.

Tilling does not blank outputs or suspend the machine by itself. An idle daemon does that and asks for the lock with `tillingctl lock`.

## Pointer lock

`zwp_pointer_constraints_v1` version 1, owned by `protocol/pointer_constraints.rs` and `input/constraints.rs`, and `zwp_relative_pointer_manager_v1` version 1, delegated in `protocol.rs`.

A locked pointer stays where it is and still reports how far the device moved. The lock ends when its window loses the keyboard, so a client cannot keep the pointer after the user switched away.

Tests: the three tests of `pointer_lock::`. Confinement to a region has no test.

## Cursor shape

`wp_cursor_shape_manager_v1` version 2, delegated in `protocol.rs`.

A client names a cursor instead of attaching a buffer. On the native backend the compositor draws the pointer itself. No test asserts which shape is shown.

## Text input and input methods

`zwp_text_input_manager_v3` version 1, owned by `protocol/text_input.rs`.

Text input is offered to every client. It does nothing until an input method is running, and an input method is a helper: see `zwp_input_method_manager_v2` below.

No test types through an input method. `cargo xtask scenario desktop-integration --operator` asks a person to do it.

## Globals for helpers only

| Global | Version | Owner | Offered to |
| --- | --- | --- | --- |
| `ext_session_lock_manager_v1` | 1 | `protocol/session_lock.rs`, `lock/` | The program named in `session.locker` |
| `zwlr_data_control_manager_v1` | 2 | `protocol/selection.rs` | Helpers with `data-control` |
| `zwp_input_method_manager_v2` | 1 | `protocol/text_input.rs` | Helpers with `input-method` |
| `zwp_virtual_keyboard_manager_v1` | 1 | `protocol.rs` | Helpers with `virtual-keyboard` |

`protocol/states.rs` hides each of them from a client that was not granted it, so a client on the public socket does not see them in the registry and cannot bind them.

Tests: `privileges::the_public_socket_offers_no_privileged_global`, `privileges::a_helper_is_offered_exactly_the_capabilities_it_was_granted`, `privileges::a_helper_without_capabilities_is_an_ordinary_client`, and `helpers::only_a_helper_with_capabilities_inherits_a_private_connection` for the way a started program receives its connection.

Data control, input methods and the virtual keyboard have no test beyond being offered and hidden.

### Session lock

While the session is locked, windows receive no input, key bindings do nothing and each output shows only the surface of the locker, or a blank frame until the locker has drawn one. The lock is confirmed to the locker when every output is covered.

The session stays locked when the locker disappears, before or after the confirmation. Another locker can then take the lock over and release it. While a locker holds the lock, a second one is refused. An output plugged while locked shows no window.

Tests: the eight tests of `lock::`, and `locker::` for the locker the compositor starts itself.
