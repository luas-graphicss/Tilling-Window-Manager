# Outputs

How Tilling places monitors, what happens when one is unplugged and how a change made while running is kept or undone.

## Profiles

An `[[outputs]]` entry of the configuration applies when a monitor is connected. `name` is the connector, such as `DP-1` or `eDP-1`. An entry with `monitor` follows that monitor to whatever connector it is plugged into and wins over the entry of the connector.

    [[outputs]]
    name = "desk"
    monitor = "DEL/DELL U2720Q/7RGSN83"
    scale = 1.5
    position = { x = -2560, y = 0 }
    mode = { width = 2560, height = 1440, refresh = 143.9 }
    transform = "90"

The monitor key is `MAKE/MODEL/SERIAL`, read from the EDID. When the EDID carries no serial, or two connected monitors report the same one, the key is `MAKE/MODEL@CONNECTOR`, so identical monitors stay apart. A monitor without a readable EDID matches by connector only. The number an output gets at run time is never part of a profile.

Every setting is optional:

| Setting | Without it |
| --- | --- |
| `scale` | 1. Accepts 0.25 to 8 in steps of 1/120. |
| `position` | To the right of the outputs already placed. Coordinates may be negative. |
| `mode` | The mode the monitor prefers. A size without `refresh` takes the preferred or fastest rate of that size; a rate matches within 0.5 Hz. A mode the monitor does not offer falls back to the preferred one with a warning. |
| `transform` | `normal`. Also `90`, `180`, `270`, `flipped`, `flipped-90`, `flipped-180` and `flipped-270`. |

Each output presents on its own clock. A 60 Hz monitor does not hold back a 144 Hz one.

## Hotplug

Each output shows one workspace. Unplugging an output moves its workspaces to the first output that remains. With no output left they stay parked with their windows, and the compositor stops presenting until a monitor returns. A monitor plugged into the same connector again gets back the workspaces that were on it; nothing else moves.

## Changing an output while running

    tillingctl get outputs
    tillingctl output set 1:0 --mode 1920x1080@60 --scale 1.25
    tillingctl output confirm

`output set` takes any of `--mode WIDTHxHEIGHT[@HZ]`, `--position X,Y`, `--scale FACTOR` and `--transform`. The change applies at once and waits 15 seconds for `output confirm`. Without it, or after `output revert`, the previous settings return, so a mode the monitor cannot show does not leave the session blind. One change waits at a time: another `output set` is refused until the first is confirmed or reverted.

A mode the monitor does not offer, a scale out of range and a change that leaves no usable area are refused before anything changes. On the native backend a mode the display controller rejects is reverted immediately.

A confirmed change lasts until the monitor is unplugged or the compositor stops. Write it to the profile to keep it.

Over the control socket the calls are `configure_output`, `confirm_output_change` and `revert_output_change`, available from protocol 1.1. `configure_output` answers with `output_change_pending` and the time left to confirm.

Tests: `output_changes::` in `crates/tilling-compositor/tests/headless/`, run together with the unit tests of the settings by `cargo xtask scenario output-rollback`.
