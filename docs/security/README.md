# Security

What Tilling protects today, what it does not, and how to report a problem.

## Reporting a vulnerability

Use "Report a vulnerability" in the Security tab of the repository. Do not open a public issue for a problem that lets one program read the input or the contents of another, reach the control socket of another user or crash the session with ordinary input.

## What is not protected

The session lock has never run on real hardware or with a real locker. The tests lock the headless backend with a stand-in program. Until a run of `cargo xtask scenario session-lock-failures --operator` is recorded in [docs/hardware](../hardware/README.md), do not rely on it to protect an unattended machine.

Screen capture is not implemented, so nothing can record the screen through the compositor. X11 applications cannot run; when XWayland is integrated they will not be isolated from each other the way Wayland clients are.

Tilling has not been reviewed by anyone outside the project, and the hardware backend has no recorded run. Treat everything below as the intended behaviour with the tests that pin it, not as a guarantee.

## Wayland clients

A client is any program that connects to the Wayland socket of the session. It is not trusted with anything beyond its own windows.

- The compositor advertises the globals listed in [docs/protocols](../protocols/README.md) and nothing else. None of those on the public socket lets a client read the windows of another client, capture the screen or inject input. `globals::only_the_implemented_globals_are_advertised` fails when a global is added without being listed.
- The clipboard and the primary selection are offered to the client that holds the keyboard, and only that client can replace them. A client in the background can neither read nor change them.
- Keyboard input goes to the focused window only. A client cannot take the focus by asking: an activation token moves the keyboard only when the client that requested it held the keyboard and named a real key press or click, and it works once.
- A pointer lock ends when its window loses the keyboard.
- Layer shell is open to every client, as it is in most compositors that offer it. A client can put a surface over the windows and, by asking for the keyboard exclusively the way a launcher does, receive what is typed until it closes. Start only programs you trust; this is not a boundary Tilling enforces yet.
- A client that breaks a protocol is disconnected alone. The headless tests cover the errors a client can trigger for each global.
- A client that stops answering does not hold the others: a layout change waits 200 ms for slow clients and then goes on without them.

## Helpers and their permissions

Four protocols give a program power over the whole session: locking it, reading and replacing the clipboard at any time, feeding text as an input method and typing as a keyboard. None of them is on the public socket (`privileges::the_public_socket_offers_no_privileged_global`).

A program receives one only when the compositor starts it: the locker named in `session.locker` gets the session lock, and a `[[helpers]]` entry gets what its `capabilities` list names. The compositor opens a connection for that program alone and hands it over as an inherited descriptor in `WAYLAND_SOCKET`. Nothing is granted by the name of a process, and a program that connects to the public socket by itself is an ordinary client (`privileges::a_helper_without_capabilities_is_an_ordinary_client`). The permissions come from the configuration, which is trusted.

A helper that keeps failing is started again at most three times a minute. Helpers stop with the session: they are asked to end and killed after two seconds.

Screen capture must follow the same rule when it is added. It must not become a global every client can bind.

## Session lock

`tillingctl lock`, or the `lock` action of a binding, starts the locker. While the session is locked no window receives input, key bindings do nothing, and an output shows only what the locker drew, or a blank frame.

The session never unlocks because its locker went away. When a locker the compositor started dies, a new one is started, at most three times a minute; after that the session stays locked until `tillingctl lock` is run again, from another virtual terminal or over the network. A locker started by other means is not replaced.

Tests: `lock::` for the protocol and `locker::` for the locker the compositor starts.

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
- A binding, a helper and the locker each start a program from a list of arguments. No shell is involved, and no window title or application identifier is ever placed in a command.
- Started programs receive `WAYLAND_DISPLAY`, `TILLING_SOCKET`, `XDG_CURRENT_DESKTOP` and `XDG_SESSION_TYPE` of this instance. With `--session-target` the same four variables are handed to the user service manager, and removed from it when the session ends.

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
