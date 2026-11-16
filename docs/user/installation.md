# Installing

Tilling ships two programs: `tilling`, the compositor, and `tillingctl`, the client of its control socket. This page covers building the packages, installing them, starting a session and removing everything again.

No package has been installed on a clean Ubuntu or Arch machine yet. The archives are checked by the tests of `xtask` and by listing their contents; treat the steps below as the intended procedure, and report what differs.

## Building the packages

From a checkout, with the build requirements of the [README](../../README.md#requirements):

    cargo xtask package --target x86_64-unknown-linux-gnu

`dist/` then holds:

| File | Use |
| --- | --- |
| `tilling-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | The installed tree under one directory, for any distribution |
| `tilling_0.1.0_amd64.deb` | Package for Debian and Ubuntu |
| `tilling-0.1.0.cdx.json` | CycloneDX list of every crate built into the binaries |
| `tilling-0.1.0.licenses.txt` | License of each of those crates |
| `tilling-0.1.0.provenance.json` | Commit, toolchain, lockfile digest and digest of each artifact |
| `SHA256SUMS` | Digests of the files above |

Check a download before installing it:

    cd dist && sha256sum --check SHA256SUMS

Build the `.deb` on the oldest release you want it to run on. The binaries link against the C library and the graphics libraries of the machine that built them.

## Debian and Ubuntu

    sudo apt install ./tilling_0.1.0_amd64.deb

`apt` pulls the libraries named in `packaging/debian/control`. The terminal, the launcher and the locker of the default configuration, `foot`, `fuzzel` and `swaylock`, are only recommended; install them or name others in your configuration.

## Arch Linux

`packaging/arch/PKGBUILD` builds from the tagged source and installs the same tree:

    cd packaging/arch && makepkg --syncdeps --install

It needs a `v0.1.0` tag on the repository, which does not exist yet.

## Any distribution

The tarball holds `usr/` under one directory. Install it over `/`:

    sudo tar --extract --gzip --file tilling-0.1.0-x86_64-unknown-linux-gnu.tar.gz \
        --directory / --strip-components 1 --no-same-owner

You need libseat, libinput, libudev, libgbm, libxkbcommon, libwayland and an EGL driver with GLES 2 from your distribution.

## What gets installed

| Path | Content |
| --- | --- |
| `/usr/bin/tilling`, `/usr/bin/tillingctl` | The programs |
| `/usr/share/wayland-sessions/tilling.desktop` | Session entry for display managers |
| `/usr/share/systemd/user/tilling-session.target` | Target the user services of the session attach to |
| `/usr/share/xdg-desktop-portal/tilling-portals.conf` | Which portal backend answers which request |
| `/usr/share/tilling/config.toml` | The default configuration, as a starting point |
| `/usr/share/doc/tilling/` | README and license |
| `/usr/share/bash-completion/completions/tillingctl` | Completions for bash |
| `/usr/share/fish/vendor_completions.d/tillingctl.fish` | Completions for fish |
| `/usr/share/zsh/site-functions/_tillingctl` | Completions for zsh |

Debian and Ubuntu do not read `/usr/share/zsh/site-functions`. There, add it to `fpath` before `compinit`, or generate the script where you want it with `tillingctl completions zsh`.

Nothing is written to your home directory. The compositor runs as your user, opens devices through libseat and needs no setuid bit and no system service.

## First start

From a display manager, choose the Tilling session. From a virtual terminal:

    tilling --backend drm

Without a configuration of your own the built-in one applies: `Super+Return` opens `foot`, `Super+d` opens `fuzzel`, `Super+Escape` locks with `swaylock` and `Super+Shift+e` ends the session. To change anything, start from the installed copy:

    mkdir -p ~/.config/tilling
    cp /usr/share/tilling/config.toml ~/.config/tilling/config.toml

If the session does not come up, see [recovery.md](recovery.md). For the locker, panels, input methods and user services, see [session.md](session.md).

## Removing

    sudo apt remove tilling

For the tarball, remove the files it installed:

    tar --list --gzip --file tilling-0.1.0-x86_64-unknown-linux-gnu.tar.gz \
        | grep -v '/$' | cut -d/ -f2- | sed 's|^|/|' | sudo xargs rm --

Neither touches `~/.config/tilling`. Your configuration stays until you delete it.
