# Compatibility

Which platforms Tilling is meant for, what has been checked, and how the versions of the programs, the configuration and the control protocol relate.

## Platforms

| Platform | State |
| --- | --- |
| Linux on x86_64 with Intel or AMD graphics and Mesa | The target. No hardware run is recorded yet, see [docs/hardware](../hardware/README.md) |
| NVIDIA, machines that mix graphics vendors, aarch64 | Not supported and not tested. A result on Intel or AMD says nothing about them |
| Other operating systems | Out of scope |

| Distribution | Package | State |
| --- | --- | --- |
| Ubuntu LTS | `.deb` from `cargo xtask package` | Built and inspected. Not yet installed on a clean machine |
| Arch Linux | `packaging/arch/PKGBUILD` | Written. Never built, it needs a release tag |
| Any | Tarball from `cargo xtask package` | Built, extracted and listed. Not yet used to start a session |

No application is on a list of checked applications yet. The automated tests use the client in `crates/tilling-test-support`, not real toolkits.

A combination that is not in these tables is unknown, not broken. A bug is something that fails on a platform listed as the target.

## Not planned for 1.0

X11 applications need XWayland, which is not integrated yet. A standalone X11 window manager, HDR and color management, variable refresh rate, VR, plugins loaded into the compositor, more than one seat, tablets and multi-finger gestures are outside what 1.0 sets out to do.

## Versions

### Programs

`tilling` and `tillingctl` share one version and follow SemVer. Before 1.0 a breaking change raises the minor version.

### Configuration

Every file states `schema_version`. This build reads version 1.

- A file with another version is refused, naming both versions, before any other problem in it is reported.
- Unknown keys are refused instead of ignored, so a typo does not silently disable a setting.
- A key with a default may be added inside version 1; files written before it keep working. `default` in `[layout]` arrived that way.
- Removing a key, renaming it or changing what a value means needs version 2 and a written migration.
- Tilling never rewrites your file.

`unsupported_versions_take_precedence_over_unknown_keys`, `unknown_keys_are_rejected_where_they_appear` and `file_with_only_a_version_inherits_every_default_section` in `crates/tilling-config/tests/validation.rs` pin these rules.

### Control protocol

The protocol has a major and a minor version, now 1.2, exchanged in the handshake.

- A peer with another major is refused with both numbers. Nothing is approximated.
- With the same major, both sides settle on the lower minor.
- A minor version only adds: new methods, and new fields that an older peer may leave out. A reader fills a missing field with a documented default, so `tillingctl` 1.2 reads a compositor that speaks 1.0.
- Removing a method or a field, or changing its meaning, needs major 2.

`crates/tilling-ipc/src/version.rs` and `crates/tilling-ipc/tests/wire_format.rs` pin the handshake and the documents; `workspace_from_an_older_compositor_reads_as_inactive_bsp` is the test of reading an older peer.

### Stored state

Tilling stores nothing between sessions. The arrangement of windows is lost when the session ends, and there is no state file to migrate.
