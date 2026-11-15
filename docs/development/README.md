# Development

How to check a change, run the scenarios, fuzz the parsers and build the packages. The build requirements are in the [README](../../README.md#requirements).

## Checks

    cargo xtask check

runs, in this order, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`. Run it before every commit.

    cargo xtask check --profile full

adds the tests in release mode, `cargo doc` with warnings denied, the release build of the workspace, `cargo audit` and `cargo deny check`. Run it before a release and after changing a dependency.

Each step is reported as `passed`, `failed` with its output, or `not run`. Tests run inside a private runtime directory with `WAYLAND_DISPLAY` and `TILLING_SOCKET` removed, so they cannot reach the session you are working in, and a socket left behind fails the run.

| Exit status | Meaning |
| --- | --- |
| 0 | every step passed |
| 1 | a step failed |
| 2 | the checks could not start |
| 3 | a pinned tool is missing or has another version |

The two tools outside the toolchain are pinned in `xtask/src/checks/plan.rs`:

    cargo install cargo-audit --version 0.23.0 --locked
    cargo install cargo-deny --version 0.20.2 --locked

`deny.toml` lists the licenses accepted in the dependency graph and the advisories that are ignored, each with its reason. An advisory is only ignored after checking that it cannot apply; [docs/security](../security/README.md#dependencies) keeps that analysis.

## Scenarios

    cargo xtask scenario output-hotplug
    cargo xtask scenario output-hotplug --operator

A scenario groups the tests of one behaviour and, with `--operator`, asks a person in a native session to confirm what tests cannot see. [docs/hardware](../hardware/README.md#recording-a-run) explains how to record a run.

    cargo xtask scenario release-journey --operator

covers a whole session with an installed package: start from the display manager, work with windows, survive a broken configuration, quit and uninstall.

## Fuzzing

Three targets live in `fuzz/`:

| Target | Input | Properties |
| --- | --- | --- |
| `configuration` | A configuration document | Never panics, respects the size limit, rules only name workspaces that exist |
| `control_stream` | One byte that sets the read size, then a stream of frames | The frames do not depend on how the stream is split, and an accepted request or handshake is accepted again after being encoded |
| `desktop` | A script of window, workspace and output commands | Every invariant holds after each command, a rejected command changes nothing and the revision advances by exactly one per change |

The properties are ordinary functions in `crates/tilling-fuzz-cases`. Its tests replay every file under `fuzz/corpus/` on the stable toolchain, so `cargo test` covers the corpus without a fuzzer.

Running a target needs a nightly toolchain and `cargo-fuzz`:

    rustup toolchain install nightly --profile minimal
    cargo install cargo-fuzz --version 0.13.2 --locked
    cargo +nightly fuzz run desktop -- -max_total_time=600

When a run fails, libFuzzer writes the input to `fuzz/artifacts/<target>/`. Copy it to `fuzz/corpus/<target>/` under a name that says what broke, fix the cause, and commit both: from then on `cargo test` replays it.

The last run before this page was written was 91 seconds per target on one machine: 58,196 executions of `configuration`, 998,426 of `control_stream` and 161,501 of `desktop`, without a failure. That is a smoke run. The 24 hours per target that a release asks for have not been done.

## Packaging

    cargo xtask package --target x86_64-unknown-linux-gnu

builds `tilling` and `tillingctl` in release mode with the lockfile and writes the artifacts described in [docs/user/installation.md](../user/installation.md#building-the-packages) to `dist/`. `--output` chooses another directory.

- The installed tree is defined once, in `xtask/src/package/layout.rs`. The tarball, the `.deb` and the Arch package all take it from there.
- `packaging/debian/control` holds the fields of the `.deb` that do not change with a build. `xtask` adds the version, the architecture and the installed size.
- The SBOM lists the crates the two binaries are built from, build dependencies included and test dependencies left out, each with the checksum the lockfile pins.
- File times in the archives come from `SOURCE_DATE_EPOCH` when it is set, and from the clock otherwise. Two builds have not been compared byte for byte, so nothing here claims reproducible builds.

## Releasing

1. `cargo xtask check --profile full` passes on the commit to release.
2. The scenarios of [docs/hardware](../hardware/README.md) have a recorded run for each supported row.
3. `cargo xtask package` runs on a clean checkout of that commit, and the packages install, start a session and uninstall on a clean machine of each supported distribution.
4. The version in `Cargo.toml` and in `packaging/arch/PKGBUILD` is the one being released, and `sha256sums` in the `PKGBUILD` holds the digest of the tagged source archive instead of `SKIP`.
5. The tag and the release are created by the maintainer. `SHA256SUMS`, the SBOM and the provenance file are published next to the packages.

Steps 2 and 3 have not been done for any version.
