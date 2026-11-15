# Hardware validation

The native backend, `--backend drm`, drives real display controllers and input devices. Unit and headless tests cover the logic around it. They cannot show that a monitor lights up, so this directory records what a person checked on a machine and how to repeat the run.

## Status

No combination has been validated yet. A row without a report is unsupported, however plausible it looks.

| Dimension | Cases to cover | Report |
| --- | --- | --- |
| GPU | Intel with Mesa, AMD with Mesa | none |
| Outputs | one monitor, two monitors, last monitor removed | none |
| Geometry | landscape, portrait, negative origin, scales 1, 1.25, 1.5 and 2 | none |
| Refresh | 60 Hz, mixed rates with 120 Hz or 144 Hz | none |
| Input | keyboard, mouse, touchpad, unplugging during an interaction | none |
| Session | start from a virtual terminal, terminal switch, seat pause and resume | none |
| Rendering | SHM, dmabuf, composition, direct scanout when eligible, composited cursor | none |
| Power | suspend and resume | none |

Session locking does not exist yet. Do not leave a machine unattended on this backend.

## Requirements

- A seat from logind or seatd. Tilling runs as a normal user and opens devices through libseat. It needs no root, no setuid binary and no access to `/dev/input`.
- libseat, libinput, libudev, libgbm and an EGL driver with GLES 2.

## Running

From a virtual terminal:

    cargo run -p tilling -- --backend drm --config config/default.toml

`Ctrl+Alt+F1` to `Ctrl+Alt+F12` switch to another virtual terminal. While the session is away, held keys are released, pointer grabs end and nothing is presented. Coming back takes the displays again. A failure is retried five times with a delay that doubles from 250 ms, and after the last retry the compositor stops instead of spinning.

An output whose frame fails five times in a row stops presenting and keeps its windows.

## Recording a run

Each scenario runs its automated tests, prints the kernel, the graphics drivers and the connectors, and then asks for every check. A check that can blank outputs, leave the terminal or suspend the machine asks before it starts.

    cargo xtask scenario output-hotplug --operator | tee output-hotplug.txt
    cargo xtask scenario output-rollback --operator | tee output-rollback.txt
    cargo xtask scenario fractional-scale --operator | tee fractional-scale.txt
    cargo xtask scenario seat-reactivation --operator | tee seat-reactivation.txt
    cargo xtask scenario renderer-fallback --operator | tee renderer-fallback.txt

Run them from a terminal inside the session and keep the transcripts next to the Mesa version.

| Exit code | Meaning |
| --- | --- |
| 0 | every test passed and every check was confirmed |
| 1 | a test failed or a behaviour was not observed |
| 2 | the scenario could not run |
| 3 | checks were skipped, no operator was present or the machine has no graphics device |

Without `--operator`, which is how a machine without devices runs them, the result is always 3: the headless tests pass and the hardware checks are listed as not run.

## Measurements

Presentation, input and process figures come from the control socket:

    tillingctl --json get diagnostics > before.json
    tillingctl --json get diagnostics > after.json

`frames_presented` and `frames_damaged` count frames over every output, `input_devices` the devices libinput opened, and `resident_kib`, `open_descriptors` and `cpu_time_ms` describe the compositor process. Take one reading before a scenario and one after it. Between two readings of an idle session neither the frames nor the processor time should grow.
