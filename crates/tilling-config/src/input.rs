use crate::diagnostics::Report;
use crate::limits::{MAX_NAME_BYTES, MAX_REPEAT_DELAY_MS, MAX_REPEAT_RATE, MIN_REPEAT_DELAY_MS};
use crate::schema::RawInput;

#[derive(Debug, Clone, PartialEq)]
pub struct InputConfig {
    pub keyboard: KeyboardConfig,
    pub pointer: PointerConfig,
    pub touchpad: TouchpadConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardConfig {
    pub layout: String,
    pub variant: String,
    pub repeat_rate: u32,
    pub repeat_delay_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerConfig {
    pub natural_scroll: bool,
    pub acceleration: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchpadConfig {
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    pub disable_while_typing: bool,
}

pub(crate) fn validate(raw: RawInput, report: &mut Report) -> InputConfig {
    let keyboard = KeyboardConfig {
        layout: raw.keyboard.layout,
        variant: raw.keyboard.variant,
        repeat_rate: raw.keyboard.repeat_rate,
        repeat_delay_ms: raw.keyboard.repeat_delay_ms,
    };
    let pointer = PointerConfig {
        natural_scroll: raw.pointer.natural_scroll,
        acceleration: raw.pointer.acceleration,
    };
    let touchpad = TouchpadConfig {
        tap_to_click: raw.touchpad.tap_to_click,
        natural_scroll: raw.touchpad.natural_scroll,
        disable_while_typing: raw.touchpad.disable_while_typing,
    };
    if keyboard.layout.is_empty() || !is_xkb_name(&keyboard.layout) {
        report.reject(
            "input.keyboard.layout",
            "must be an XKB layout name such as `us` or `us,br`",
        );
    }
    if !is_xkb_name(&keyboard.variant) {
        report.reject(
            "input.keyboard.variant",
            "must be an XKB variant name such as `intl`",
        );
    }
    if keyboard.repeat_rate > MAX_REPEAT_RATE {
        report.reject(
            "input.keyboard.repeat_rate",
            format!("must not exceed {MAX_REPEAT_RATE} keys per second"),
        );
    }
    if !(MIN_REPEAT_DELAY_MS..=MAX_REPEAT_DELAY_MS).contains(&keyboard.repeat_delay_ms) {
        report.reject(
            "input.keyboard.repeat_delay_ms",
            format!("must be between {MIN_REPEAT_DELAY_MS} and {MAX_REPEAT_DELAY_MS}"),
        );
    }
    if !(-1.0..=1.0).contains(&pointer.acceleration) {
        report.reject(
            "input.pointer.acceleration",
            "must be a number between -1.0 and 1.0",
        );
    }
    InputConfig {
        keyboard,
        pointer,
        touchpad,
    }
}

fn is_xkb_name(name: &str) -> bool {
    name.len() <= MAX_NAME_BYTES
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b','))
}
