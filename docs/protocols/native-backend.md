# Native backend

Two globals exist only with `--backend drm`, where a display controller and a GPU are present. The headless and nested backends do not advertise them.

## wp_presentation

Version 2, delegated in `protocol.rs`. Feedback is collected in `backend/drm/feedback.rs`.

The clock is `CLOCK_MONOTONIC`. The feedback of a surface is answered at the vblank of the frame that showed it, with the timestamp and the sequence of the page flip and the refresh interval of the mode. A frame that is never shown discards its feedback.

Errors: none.

Tests: none automated, because a headless frame has no vblank. Checked by hand with the scenarios in [../hardware/README.md](../hardware/README.md).

## zwp_linux_dmabuf_v1

Version 5, owned by `protocol/dmabuf.rs`.

Advertises the formats and modifiers the primary GPU renders from, with that GPU as the main device of the default feedback. The renderer imports a buffer before the client is told that it exists. A buffer the renderer refuses fails with `failed`.

Errors: `invalid_format`, `invalid_dimensions`, `out_of_bounds`, `incomplete`, `plane_idx`, `plane_set`, `already_used` and `invalid_wl_buffer` come from Smithay, and no test asserts them.

Tests: none automated, because importing needs a GPU.
