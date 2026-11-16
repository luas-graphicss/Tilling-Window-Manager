# Tilling

Tilling is a tiling Wayland compositor for Linux, written in Rust on top of Smithay. It arranges the windows of each workspace as tiles, is driven from the keyboard and can be scripted with `tillingctl` over a local socket.

## Status

Version 0.1.0 is the first release, published as a pre-release. It is a development version and is not ready to be the only session on a machine.

What works and is covered by the tests in this repository:

- The `bsp`, `master-stack` and `monocle` layouts, floating and fullscreen windows, a scratchpad and named workspaces, one shown on each output.
- Several outputs with hotplug, fractional scales, rotation and changes that revert unless confirmed.
- A TOML configuration with key bindings, binding modes, window rules and output profiles, reloaded without restarting.
- A versioned control protocol on a local socket and the `tillingctl` client.
- Layer shell for panels and launchers, with the space they reserve kept free of tiles; clipboard, primary selection and drag and drop; activation tokens, idle notification and pointer lock.
- A session lock driven by a locker such as `swaylock`, and helper programs started and stopped with the session, each with only the permissions its entry names.

There are three backends: `headless`, which the tests use, `nested`, a window inside another Wayland session, and `drm`, for a virtual terminal. Only `headless` runs in the automated tests. No machine has a recorded run of `drm` yet; [docs/hardware](docs/hardware/README.md) lists what a person has to check.

What does not exist yet: screen capture, so the portals for screenshots and screen sharing have nothing to use, and XWayland, so X11 applications do not start. Input methods and clipboard managers get their protocols as helpers, and no test types through one yet.

The lock has only run in the tests, with a stand-in locker. Read [docs/security](docs/security/README.md) before leaving a machine unattended in a Tilling session.

### Known problems in 0.1.0

A session used by hand showed four problems that the automated tests of 0.1.0 did not catch. They are fixed on `main`; 0.1.0 itself still has them.

- Programs started from the session, such as the terminal, the launcher, the locker and the helpers, inherited a signal mask that blocks SIGINT and SIGTERM. Ctrl+C and `kill` did nothing to them. They now start with no signal blocked.
- A debug build, which is what `cargo run` produces without `--release`, aborted when a client sent damage with a negative size, and every window of the session went with it. Debug builds now treat the request the way release builds do.
- The `nested` backend drew the session upside down. It is upright now inside weston on software GL; nobody has looked at it on a GPU. An `[[outputs]]` profile that carries `transform = "flipped-180"` as a workaround has to lose it.
- `tillingctl lock` answered `accepted` as soon as the locker process started, even when the locker never covered the screen. It now answers once the session is locked and fails when the locker ends or does not lock within four seconds.

## Requirements

- Linux on x86_64.
- Rust 1.100.0. `rust-toolchain.toml` pins it, so `rustup` installs it on the first build.
- libseat, libinput, libudev, libgbm, libxkbcommon, libwayland and an EGL driver with GLES 2, with their development files and `pkg-config`.

On Ubuntu 24.04, the system the CI builds and tests on:

    sudo apt install libdrm-dev libegl1-mesa-dev libgbm-dev libinput-dev libpixman-1-dev \
        libseat-dev libudev-dev libwayland-dev libxkbcommon-dev pkg-config

On Amazon Linux 2023:

    sudo dnf install wayland-devel libxkbcommon-devel mesa-libgbm-devel mesa-libEGL-devel \
        libdrm-devel systemd-devel libinput-devel libseat-devel pixman-devel clang

## Running

Inside another Wayland session, as a window:

    cargo run --release -p tilling

From a virtual terminal, on the display:

    cargo run --release -p tilling -- --backend drm

From another terminal, `tillingctl` reads and changes the session:

    cargo run -p tillingctl -- get windows
    cargo run -p tillingctl -- layout set master-stack

### Default key bindings

[config/default.toml](config/default.toml) defines them, and your own configuration replaces them.

| Keys | Action |
| --- | --- |
| `Super+Return` | Open the terminal, `foot` unless you change it |
| `Super+d` | Open the launcher, `fuzzel` |
| `Super+Shift+q` | Close the focused window |
| `Super+h`, `j`, `k`, `l` | Focus the window to the left, below, above, to the right |
| `Super+Shift+h`, `j`, `k`, `l` | Move the focused window in that direction |
| `Super+Ctrl+h`, `j`, `k`, `l` | Focus the output in that direction |
| `Super+Ctrl+Shift+h`, `j`, `k`, `l` | Move the focused window to that output |
| `Super+f` | Toggle fullscreen |
| `Super+Shift+space` | Toggle floating |
| `Super+Shift+minus`, `Super+minus` | Send the window to the scratchpad, bring one back |
| `Super+space` | Cycle the layout |
| `Super+b`, `Super+t`, `Super+m` | Choose `bsp`, `master-stack`, `monocle` |
| `Super+e` | Turn the split of the tile |
| `Super+r` | Enter the resize mode: `h` and `l` resize, `Escape` leaves |
| `Super+1` to `Super+9` | Show that workspace |
| `Super+Shift+1` to `Super+Shift+9` | Move the focused window to that workspace |
| `Super+Shift+c` | Reload the configuration |
| `Super+Escape` | Lock the session with `swaylock` |
| `Super+Shift+e` | End the session |

## Configuration

Tilling reads the file given with `--config`, or `$XDG_CONFIG_HOME/tilling/config.toml` (`~/.config/tilling/config.toml` when the variable is not set), or its built-in defaults when neither exists. [config/default.toml](config/default.toml) is the built-in file; copy it to start your own.

A file that does not validate stops the start and prints the line and the reason. A reload, with `Super+Shift+c` or `tillingctl config reload`, that does not validate keeps the configuration that was active. [docs/user/recovery.md](docs/user/recovery.md) covers getting back from a broken file.

## Installing

    cargo xtask package --target x86_64-unknown-linux-gnu

builds the release binaries and writes a tarball, a `.deb`, checksums, an SBOM and a build record to `dist/`. [docs/user/installation.md](docs/user/installation.md) covers installing them, the session entry for display managers and removing the package.

The release page carries the source of each version, without packages. The CI builds the same files on every push to `main` and keeps them as the `tilling-packages` artifact of the run.

## Documentation

| Where | What |
| --- | --- |
| [docs/user](docs/user/installation.md) | Installing, first start, recovery |
| [docs/user/session.md](docs/user/session.md) | Locker, helpers, permissions, user services and portals |
| [docs/window-management.md](docs/window-management.md) | Layouts, scratchpad, moving between outputs |
| [docs/outputs.md](docs/outputs.md) | Output profiles, hotplug, confirmed changes |
| [docs/protocols](docs/protocols/README.md) | Wayland globals, versions and their tests |
| [docs/hardware](docs/hardware/README.md) | What to check on a real machine and how to record it |
| [docs/security](docs/security/README.md) | What is protected, what is not, reporting a vulnerability |
| [docs/compatibility](docs/compatibility/README.md) | Versioning of the configuration and the protocol, supported platforms |
| [docs/architecture](docs/architecture/README.md) | Crates and who owns which state |
| [docs/development](docs/development/README.md) | Checks, scenarios, fuzzing, packaging, continuous integration |

## Development

    cargo xtask check
    cargo xtask check --profile full

The first formats, lints and tests. The second adds the optimized tests, the documentation, the release build, `cargo audit` and `cargo deny`.

GitHub Actions runs three jobs on every pull request and on every push to `main`: the full profile, every scenario without an operator, and the packages, which are installed in a clean Ubuntu 24.04 container where a headless session has to start and answer `tillingctl`. [docs/development](docs/development/README.md#continuous-integration) has the details.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
