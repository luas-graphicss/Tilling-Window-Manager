# xdg-decoration

`zxdg_decoration_manager_v1` version 1, owned by `protocol/decoration.rs`.

Every toplevel is told to use server-side decoration, whatever mode the client asks for and also when it unsets its preference. The decoration is the border drawn in `render/borders.rs`; there is no title bar. A client that never binds the global decides for itself, and most toolkits then draw their own title bar.

Errors: `already_constructed`, `unconfigured_buffer` and `orphaned` come from Smithay, and no test asserts them.

Tests: `globals::decoration_is_negotiated_to_the_server_side_whatever_the_client_prefers`.
