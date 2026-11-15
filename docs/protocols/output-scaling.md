# Output scaling

Three globals let a client draw at the scale of its output and learn the logical geometry of every output. An output scale is a multiple of 1/120 between 0.25 and 8.

## wp_viewporter

Version 1, delegated in `protocol.rs`.

Smithay applies the source rectangle and the destination size of a viewport when it builds the render elements of a surface. A client that draws at a fractional scale uses it to map its buffer onto the logical size.

Errors: `viewport_exists`, `bad_value`, `bad_size`, `out_of_buffer` and `no_surface` come from Smithay, and no test asserts them.

Tests: `globals::only_the_implemented_globals_are_advertised`.

## wp_fractional_scale_manager_v1

Version 1, owned by `protocol/fractional_scale.rs`.

A surface is told the scale of the output that shows its window, or of the active output while the window is not placed yet. The preferred scale is sent again when the window is laid out on an output with another scale.

Errors: `fractional_scale_exists` comes from Smithay, and no test asserts it.

Tests: `scale::window_is_told_the_fractional_scale_of_its_output`, `scale::window_that_changes_output_is_told_the_new_scale`.

## zxdg_output_manager_v1

Version 3, owned by `protocol/output.rs`.

Reports the logical position and size of each output, which differ from the mode of `wl_output` on a scaled or rotated output. Smithay sends the update whenever the state of the output changes, including a change made with `tillingctl output set`.

Errors: none.

Tests: `globals::only_the_implemented_globals_are_advertised`.
