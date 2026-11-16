# The session

What runs next to the compositor: the locker, the helpers, the user services and the portals. Everything here is configuration; [installation.md](installation.md) covers getting to a first start.

None of this has run on a real machine yet. The tests start, restart and stop stand-in programs on the headless backend; treat the examples with real programs as the intended setup.

## Locking

    tillingctl lock

starts the program named in `session.locker`, and so does the `lock` action, bound to `Super+Escape` in the default configuration:

    [session]
    locker = ["swaylock"]

The locker has to speak `ext-session-lock-v1`; `swaylock` and `waylock` do. It is the only program that receives that protocol. The request is refused when no locker is named, and an empty `[session]` table names none.

`tillingctl lock` answers `accepted` once the locker covers every output. It answers with an error and a status other than 0 when the locker cannot be started, when it ends without locking, and when it has not locked after four seconds; in the last case the locker keeps running. A locker that ends without locking is also written to the log, whichever way the lock was asked for.

When the locker dies while the session is locked, the session stays locked and a new locker is started. To lock after a time without input, run an idle daemon as a helper and let it call `tillingctl lock`:

    [[helpers]]
    name = "idle"
    command = ["swayidle", "-w", "timeout", "300", "tillingctl lock"]

## Helpers

A helper is a program started with the session and stopped with it:

    [[helpers]]
    name = "panel"
    command = ["waybar"]
    restart = "on-failure"

| Key | Meaning |
| --- | --- |
| `name` | Lowercase letters, digits, `-` and `_`, unique |
| `command` | The program and its arguments. No shell is involved |
| `capabilities` | Protocols only this program may use. Empty when left out |
| `restart` | `never`, the default, or `on-failure` |

With `on-failure`, a helper that ends with an error is started again after a second, at most three times a minute. A helper that ends without an error is not started again.

Reloading the configuration leaves a running helper alone when its entry did not change, starts the entries that are new and stops the ones that are gone. Changing an entry stops the old program and starts the new one.

When the session ends, helpers are asked to stop and killed after two seconds. Programs started from key bindings are not helpers: they are not restarted and not stopped.

### Permissions

| Capability | Lets the program | Typical program |
| --- | --- | --- |
| `data-control` | Read and replace the clipboard at any time | A clipboard manager |
| `input-method` | Compose text for the focused window | `fcitx5`, `ibus` |
| `virtual-keyboard` | Type as a keyboard | An input method, an on-screen keyboard |

    [[helpers]]
    name = "input-method"
    command = ["fcitx5"]
    capabilities = ["input-method", "virtual-keyboard"]

A program gets these protocols only when the compositor starts it from such an entry. The same program started from a terminal is an ordinary client and does not see them. The session lock cannot be given to a helper; it belongs to `session.locker`.

## Environment

Every program the compositor starts, from a binding, as a helper or as the locker, receives:

| Variable | Value |
| --- | --- |
| `WAYLAND_DISPLAY` | The socket of this instance |
| `TILLING_SOCKET` | Its control socket, so `tillingctl` reaches this instance |
| `XDG_CURRENT_DESKTOP` | `tilling` |
| `XDG_SESSION_TYPE` | `wayland` |

## User services

The session entry for display managers starts the compositor with `--session-target tilling-session.target`. The compositor then hands the four variables above to the user service manager with `systemctl --user import-environment`, starts the target, and at the end of the session stops it and removes the variables again.

`tilling-session.target` is bound to `graphical-session.target`. A user service that should run in the session, such as a notification daemon or a portal, needs:

    [Unit]
    PartOf=graphical-session.target
    After=graphical-session.target

    [Install]
    WantedBy=tilling-session.target

Without the option the compositor leaves the service manager alone, on every backend. A nested instance started the usual way therefore does not replace the environment of the session it runs in. Pass the option yourself to get the behaviour of the session entry from a virtual terminal.

Services that D-Bus starts without systemd do not see the variables. Where that matters, run `dbus-update-activation-environment` with the four names as a helper.

## Portals

`/usr/share/xdg-desktop-portal/tilling-portals.conf` selects the GTK backend for file choosers, settings and the like, and no backend for screenshots, screen sharing and remote desktop: the compositor cannot capture the screen yet, and an application that asks is told so instead of waiting for a backend that cannot answer.

Install `xdg-desktop-portal` and `xdg-desktop-portal-gtk` to use it. Sandboxed applications open files through it.

## What the default configuration expects

| Program | Used for | Binding |
| --- | --- | --- |
| `foot` | Terminal | `Super+Return` |
| `fuzzel` | Launcher | `Super+d` |
| `swaylock` | Locker | `Super+Escape` |

None of them is required to start the session. A binding whose program is missing does nothing, and `tillingctl lock` reports the locker it could not start.
