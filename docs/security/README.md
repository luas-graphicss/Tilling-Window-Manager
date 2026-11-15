# Security

What Tilling protects today, what it does not, and how to report a problem.

## Reporting a vulnerability

Use "Report a vulnerability" in the Security tab of the repository. Do not open a public issue for a problem that lets one program read the input or the contents of another, reach the control socket of another user or crash the session with ordinary input.

## What is not protected

There is no session lock. `tillingctl lock` is refused, and anyone at the keyboard reaches every window. Do not leave a machine unattended in a Tilling session.

Tilling has not been reviewed by anyone outside the project, and the hardware backend has no recorded run. Treat everything below as the intended behaviour with the tests that pin it, not as a guarantee.

## Wayland clients

A client is any program that connects to the Wayland socket of the session. It is not trusted with anything beyond its own windows.

- The compositor advertises the globals listed in [docs/protocols](../protocols/README.md) and nothing else. None of them lets a client read the contents of another client, capture the screen, read the clipboard or inject input. `globals::only_the_implemented_globals_are_advertised` fails when a global is added without being listed.
- Keyboard input goes to the focused window only. A client cannot take the focus by asking; there is no activation protocol.
- A client that breaks a protocol is disconnected alone. The headless tests cover the errors a client can trigger for each global.
- A client that stops answering does not hold the others: a layout change waits 200 ms for slow clients and then goes on without them.

Because clipboard, layer shell, input methods and capture are not implemented, the question of which clients may use them is still open. They must not be added as globals every client can bind.

## Control socket

`tillingctl` and any other tool talk to the compositor through `$XDG_RUNTIME_DIR/tilling/<instance>/ipc.sock`.

- The directory is created with mode `0700` and the socket with `0600`. A directory that belongs to another user or that other users can enter is refused, and so is a symlink or a file of another user where the socket should be.
- The kernel reports the user of each peer, and a peer of another user is refused when it connects, before any request is read. `tillingctl` checks the compositor the same way.
- A frame is at most 1 MiB and a document at most 16 levels deep. Both limits apply before the document is parsed.
- At most 256 events and 4 MiB wait for a peer that does not read. A subscriber past the limit is told that events were dropped and has to subscribe again.

Everything that runs as your user can use the socket, and that includes ending the session. The socket separates users, not programs of the same user.

## Configuration

The configuration is trusted: it belongs to the user and can start programs.

- A file is at most 256 KiB. Unknown keys, unknown versions and out-of-range values are refused, and a refused file changes nothing.
- Patterns of window rules use a regular expression engine without backtracking, with limits on the size of the pattern and of the compiled program.
- A binding starts a program from a list of arguments. No shell is involved, and no window title or application identifier is ever placed in a command.

## Memory safety

Every crate of the workspace forbids `unsafe` through the workspace lints, and none contains an `unsafe` block. The unsafe code that runs is in Smithay and the libraries below it.

## Fuzzing

The configuration parser, the control stream and the window state have fuzz targets with a committed corpus; [docs/development](../development/README.md#fuzzing) describes them and the length of the runs done so far.

## Dependencies

`cargo audit` and `cargo deny check` run in the full checks. On the lockfile of this commit they report no vulnerability and two warnings, both about `cgmath` 0.18.0, which only arrives through Smithay 0.7.0:

| Advisory | Finding | Why it stays |
| --- | --- | --- |
| RUSTSEC-2026-0196 | `cgmath` is unmaintained | No Smithay release drops it. Ignored in `deny.toml` with this reason, to be removed when Smithay moves on. |
| RUSTSEC-2026-0197 | `Matrix::swap_columns` is unsound when both indices are equal | Smithay 0.7.0 uses `Matrix3` and `Vector2` for the projection of the renderer and never calls `swap_columns`, and no other crate in the graph does. |

Repeat the second check after every Smithay upgrade:

    grep -rn "swap_columns" ~/.cargo/registry/src/*/smithay-*/src
