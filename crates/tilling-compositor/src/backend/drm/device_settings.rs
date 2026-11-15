use smithay::reexports::input::{Device, DeviceConfigError, DeviceConfigResult};
use tilling_config::InputConfig;

type Refusal = (&'static str, DeviceConfigError);

pub(super) fn apply(device: &mut Device, input: &InputConfig) {
    let mut refused: Vec<Refusal> = Vec::new();
    if device.config_tap_finger_count() > 0 {
        let touchpad = input.touchpad;
        refused.extend(refusal(
            "tap to click",
            device.config_tap_set_enabled(touchpad.tap_to_click),
        ));
        refused.extend(refusal(
            "natural scroll",
            device.config_scroll_set_natural_scroll_enabled(inverts_on_device(input)),
        ));
        if device.config_dwt_is_available() {
            refused.extend(refusal(
                "disable while typing",
                device.config_dwt_set_enabled(touchpad.disable_while_typing),
            ));
        }
    }
    if device.config_accel_is_available() {
        refused.extend(refusal(
            "pointer speed",
            device.config_accel_set_speed(input.pointer.acceleration),
        ));
    }
    for (setting, error) in refused {
        tracing::warn!(
            device = device.name(),
            setting,
            ?error,
            "input device refused a setting"
        );
    }
}

fn inverts_on_device(input: &InputConfig) -> bool {
    input.touchpad.natural_scroll != input.pointer.natural_scroll
}

fn refusal(setting: &'static str, outcome: DeviceConfigResult) -> Option<Refusal> {
    outcome.err().map(|error| (setting, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(pointer_natural: bool, touchpad_natural: bool) -> tilling_config::ValidatedConfig {
        let text = format!(
            "schema_version = 1\n\n[input.keyboard]\nlayout = \"us\"\nrepeat_rate = 25\nrepeat_delay_ms = 600\n\n[input.pointer]\nnatural_scroll = {pointer_natural}\nacceleration = 0.0\n\n[input.touchpad]\ntap_to_click = false\nnatural_scroll = {touchpad_natural}\ndisable_while_typing = true\n"
        );
        tilling_config::parse(&text).unwrap()
    }

    #[test]
    fn touchpad_inverts_on_the_device_only_when_it_differs_from_the_seat() {
        let inverted: Vec<bool> = [(false, false), (false, true), (true, false), (true, true)]
            .into_iter()
            .map(|(pointer, touchpad)| inverts_on_device(input(pointer, touchpad).input()))
            .collect();

        assert_eq!(inverted, [false, true, true, false]);
    }
}
