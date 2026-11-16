# Recovery

What to do when the session does not start, a configuration change goes wrong or the session stops answering.

## The session does not start

`tilling` prints one line with the cause and exits with status 1. Start it from a virtual terminal to read it, and add `--verbose` for the details.

| Message | Cause | Way out |
| --- | --- | --- |
| `load the configuration: line 12, column 3: ...` | The file has a syntax error or an unknown key | Fix that line, or start with the defaults, below |
| `load the configuration: schema_version 2 is not supported, this build reads version 1` | The file was written for another version | Use a file of the version this build reads |
| `load the configuration: ...` followed by a list | Values that do not validate, all reported at once | Fix each entry named |
| `XDG_RUNTIME_DIR is not set` | The login did not create a runtime directory | Log in through logind, or a display manager that does |

To start with the defaults without touching your file, point `--config` at the installed copy:

    tilling --backend drm --config /usr/share/tilling/config.toml

In a checkout the same file is `config/default.toml`.

## A reload went wrong

A reload validates the whole file before anything changes. When it fails, the bindings, rules and layouts that were active stay, and nothing is applied in part.

    tillingctl config reload

prints the reason and exits with status 1. With the key binding, `Super+Shift+c` by default, the reason goes to the log of the compositor.

If a valid file removed the bindings you need, `tillingctl` still works from any terminal that is open, or from another virtual terminal with the socket named:

    tillingctl instances
    tillingctl --instance NAME config reload
    tillingctl --instance NAME quit

## An output change left the screen black

`tillingctl output set` waits 15 seconds for `tillingctl output confirm` and then returns to the previous settings. Wait, and the picture comes back. See [outputs.md](../outputs.md#changing-an-output-while-running).

## The session stops answering

On the DRM backend, `Ctrl+Alt+F1` to `Ctrl+Alt+F12` switch to another virtual terminal as long as the compositor still reads the keyboard. Log in there and end the session:

    tillingctl instances
    tillingctl --instance NAME quit

If the socket does not answer within the timeout, `tillingctl` exits with status 3. Then end the process:

    pkill -TERM -x tilling

When the keyboard is not read either, log in from another machine and run the same command there.

Windows of a session that ends this way close with it. Nothing is saved between sessions except your configuration file.

## Reporting what happened

Run the compositor with `--verbose` and keep its output. Add the output of `tillingctl --json get diagnostics` when the session still answers, the kernel version and the graphics driver. Tilling itself does not log key presses or window titles, but the libraries below it log too, so read the output before you share it.
