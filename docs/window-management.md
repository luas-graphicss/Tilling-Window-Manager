# Window management

How a workspace arranges its tiles, where the scratchpad keeps windows, and how focus and windows cross from one output to another.

## Layouts

Each workspace has its own layout. `default` in the `[layout]` section chooses the one new workspaces start with; without it they start with `bsp`. Reloading the configuration changes the layout of workspaces created afterwards and leaves the existing ones alone.

    [layout]
    inner_gap = 8
    outer_gap = 8
    default = "master-stack"

| Layout | Arrangement |
| --- | --- |
| `bsp` | A new window splits the focused tile in two, turning the split at each level. |
| `master-stack` | The first tile takes half of the output and the others share the other half in equal parts. |
| `monocle` | The tile focused last covers the whole area and hides the other tiles. |

The three layouts read the same tree of tiles, so changing the layout loses nothing: the splits and ratios made in `bsp` are back when the workspace returns to it. A new tile always enters the order right after the focused one. Floating and fullscreen windows do not depend on the layout; in `monocle` a floating window stays above the tile in front.

The directional actions keep their meaning in every layout:

| Action | `bsp` | `master-stack` | `monocle` |
| --- | --- | --- | --- |
| `focus`, `move` | The tile across the nearest split in that direction. | Between the master and the stack, or to the previous and next tile of the stack. Moving a stacked tile toward the master swaps them. | The next or previous tile, wrapping around. |
| `resize` | The ratio of the split that holds the tile. | The share of the master. | Nothing. |
| `toggle-split` | Turns the split of the tile. | Puts the master above a row of tiles, or back to the side. | Nothing. |

`cycle-layout` goes from `bsp` to `master-stack` to `monocle` and back. The default bindings are `Super+space` to cycle and `Super+b`, `Super+t` and `Super+m` to pick one.

    [[bindings]]
    keys = "Super+space"
    action = "cycle-layout"

    [[bindings]]
    keys = "Super+m"
    action = { set-layout = "monocle" }

## Scratchpad

`move-to-scratchpad` (`Super+Shift+minus`) takes the focused window off its workspace and hides it. `toggle-scratchpad` (`Super+minus`) brings the window stashed for the longest time to the active workspace, floating and focused. Pressed again while that window has the focus, it hides the window again, and with several windows stashed they take turns.

A window that was tiled comes back floating in the middle of the output at the size it had. A floating window keeps its place, unless the output it comes back on does not contain it, in which case it is centered there. Tiling a window that came from the scratchpad, with `toggle-floating`, makes it an ordinary tile.

A stashed window keeps running but is not drawn and gets no frame callbacks. `tillingctl get windows` lists it as `stashed`, and requests to focus, move, resize or change its mode are refused until it is shown.

## Outputs

`focus-output` and `move-to-output` take a direction. The target is an output that starts past that edge of the current one. An output aligned with the current one wins over a diagonal one, then the nearest wins, then the one whose center is closest. Without such an output nothing happens.

`focus-output` activates the workspace the target shows. `move-to-output` sends the focused window to that workspace and leaves the focus where it was. A floating window the new output does not contain is centered on it. The default bindings are `Super+Ctrl+h`, `j`, `k` and `l` to focus and the same keys with `Shift` to move.

## From the command line

    tillingctl get workspaces
    tillingctl layout set master-stack
    tillingctl layout set monocle --workspace 2
    tillingctl workspace switch 2
    tillingctl config reload
    tillingctl events --topics windows,outputs

`layout set` applies to the active workspace unless `--workspace` names another. A workspace is given by its name or by its `INDEX:GENERATION` identifier, and `tillingctl` asks the compositor for the list to find it. A workspace that does not exist ends with exit status 1 before anything changes.

`events` prints a snapshot and then one line per event until it is interrupted. `--topics` limits both to `windows`, `workspaces` or `outputs`; focus changes belong to `windows`. Lost events are reported under every topic, with exit status 3.

Over the control socket, `set_layout` takes `bsp`, `master_stack` or `monocle`. From protocol 1.2 each workspace carries `layout` and `active`, and each window carries `stashed`. A client reading an older compositor sees `bsp`, `false` and `false`.

Tests: `layouts::`, `scratchpad::` and `outputs::output_shortcuts` in `crates/tilling-compositor/tests/headless/`, and `layouts::`, `scratchpad::` and `navigation::` in `crates/tilling-core/tests/domain/`.
