# Architecture

Which crate does what, which one may depend on which, and who owns the state.

## Crates

An arrow reads "depends on".

    tilling            -> tilling-compositor, tilling-config
    tilling-compositor -> tilling-core, tilling-config, tilling-ipc, smithay, calloop
    tilling-config     -> tilling-core
    tillingctl         -> tilling-ipc
    tilling-fuzz-cases -> tilling-core, tilling-config, tilling-ipc
    xtask              -> cargo, nothing of the workspace

| Crate | Responsibility | Must not know about |
| --- | --- | --- |
| `tilling-core` | Windows, workspaces, outputs, layouts and focus as plain data, and the commands that change them | Wayland, sockets, files, threads, the clock, the environment |
| `tilling-config` | The TOML schema, its validation and the diagnostics | The running session. It returns a validated value and applies nothing |
| `tilling-ipc` | Framing, documents, the handshake and the socket of the control protocol | The types of `tilling-core`. It has its own types for what goes on the wire |
| `tilling-compositor` | The Wayland protocols, input, rendering, the backends and the control server | Nothing above it |
| `tilling`, `tillingctl` | Command line, start and exit status | Rules of any kind |
| `tilling-test-support` | A Wayland client for the tests | Compositor internals. It only speaks the protocol |
| `tilling-fuzz-cases` | Properties checked against arbitrary input | The fuzzer. `fuzz/` holds the targets |

`tests/smithay-contract` holds tests of the Smithay features the compositor relies on, so an upgrade that changes one of them fails there first.

## State

One event loop owns everything that changes. There is no state shared behind a lock.

`tilling-core` keeps the desktop: which windows exist, where each one is, what has the focus. A change is a `Command` applied to a `DesktopState`. The result is either a `Transition`, with the new revision and the effects to carry out, or an error that leaves the state exactly as it was. After every accepted command the state is checked against its invariants, and a change that would break one is refused.

Windows, workspaces and outputs are named by identifiers with a generation. An identifier of something that was removed never names what took its place.

The compositor maps those identifiers to Wayland surfaces and outputs, and carries out the effects: it sends configures, moves surfaces, redraws. The core decides and the compositor does.

## Path of an event

    key press, client request, hotplug, control request or timer
      -> checked at the edge: origin, limits, protocol state
      -> turned into a command
      -> applied by the core, invariants checked
      -> effects carried out by the compositor
      -> configures sent, waiting for clients up to 200 ms
      -> scene updated, damaged outputs redrawn
      -> revision and events published on the control socket

Not every event takes the whole path. A pointer motion may only move the cursor, and a frame callback only releases a buffer.

The same commands in the same order give the same state. `crates/tilling-core/tests/domain/generated_sequences.rs` replays generated sequences and compares the result, and the `desktop` fuzz target checks the invariants on arbitrary ones.
