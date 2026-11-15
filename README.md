# Tilling

Tilling is a tiling Wayland compositor for Linux, written in Rust on top of Smithay. It arranges the windows of each workspace as tiles, is driven from the keyboard and can be scripted with `tillingctl` over a local socket.

## Status

Version 0.1.0 is a development version. It is not ready to be the only session on a machine.

What works and is covered by the tests in this repository:

- The `bsp`, `master-stack` and `monocle` layouts, floating and fullscreen windows, a scratchpad and named workspaces, one shown on each output.
- Several outputs with hotplug, fractional scales, rotation and changes that revert unless confirmed.
- A TOML configuration with key bindings, binding modes, window rules and output profiles, reloaded without restarting.
- A versioned control protocol on a local socket and the `tillingctl` client.

There are three backends: `headless`, which the tests use, `nested`, a window inside another Wayland session, and `drm`, for a virtual terminal. Only `headless` runs in the automated tests. No machine has a recorded run of `drm` yet; [docs/hardware](docs/hardware/README.md) lists what a person has to check.

What does not exist yet: session locking, clipboard and drag and drop, layer shell (so panels, launchers and notification daemons that need it do not start), input methods, screen capture and XWayland. Without a lock, do not leave a machine unattended in a Tilling session.

## Requirements

- Linux on x86_64.
- Rust 1.100.0. `rust-toolchain.toml` pins it, so `rustup` installs it on the first build.
- libseat, libinput, libudev, libgbm, libxkbcommon, libwayland and an EGL driver with GLES 2, with their development files and `pkg-config`.

On Amazon Linux 2023, where the checks of this repository were last run, this is enough:

    sudo dnf install wayland-devel libxkbcommon-devel mesa-libgbm-devel mesa-libEGL-devel \
        libdrm-devel systemd-devel libinput-devel libseat-devel pixman-devel clang

## Running

Inside another Wayland session, as a window:

    cargo run -p tilling

From a virtual terminal, on the display:

    cargo run --release -p tilling -- --backend drm

`Super+Return` opens the terminal named in the configuration, `foot` unless you change it. `Super+Shift+q` closes the focused window and `Super+Shift+e` ends the session. From another terminal:

    cargo run -p tillingctl -- get windows
    cargo run -p tillingctl -- layout set master-stack

## Configuration

Tilling reads the file given with `--config`, or `$XDG_CONFIG_HOME/tilling/config.toml` (`~/.config/tilling/config.toml` when the variable is not set), or its built-in defaults when neither exists. [config/default.toml](config/default.toml) is the built-in file; copy it to start your own.

A file that does not validate stops the start and prints the line and the reason. A reload, with `Super+Shift+c` or `tillingctl config reload`, that does not validate keeps the configuration that was active. [docs/user/recovery.md](docs/user/recovery.md) covers getting back from a broken file.

## Installing

    cargo xtask package --target x86_64-unknown-linux-gnu

builds the release binaries and writes a tarball, a `.deb`, checksums, an SBOM and a build record to `dist/`. [docs/user/installation.md](docs/user/installation.md) covers installing them, the session entry for display managers and removing the package.

## Documentation

| Where | What |
| --- | --- |
| [docs/user](docs/user/installation.md) | Installing, first start, recovery |
| [docs/window-management.md](docs/window-management.md) | Layouts, scratchpad, moving between outputs |
| [docs/outputs.md](docs/outputs.md) | Output profiles, hotplug, confirmed changes |
| [docs/protocols](docs/protocols/README.md) | Wayland globals, versions and their tests |
| [docs/hardware](docs/hardware/README.md) | What to check on a real machine and how to record it |
| [docs/security](docs/security/README.md) | What is protected, what is not, reporting a vulnerability |
| [docs/compatibility](docs/compatibility/README.md) | Versioning of the configuration and the protocol, supported platforms |
| [docs/architecture](docs/architecture/README.md) | Crates and who owns which state |
| [docs/development](docs/development/README.md) | Checks, scenarios, fuzzing, packaging |

## Development

    cargo xtask check
    cargo xtask check --profile full

The first formats, lints and tests. The second adds the optimized tests, the documentation, the release build, `cargo audit` and `cargo deny`.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
