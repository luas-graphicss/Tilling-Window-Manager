use smithay::backend::input::KeyState;
use smithay::input::keyboard::{
    FilterResult, Keycode, Keysym, KeysymHandle, ModifiersState, keysyms,
};
use smithay::utils::SERIAL_COUNTER;
use tilling_config::Action;

use super::bindings::held_modifiers;
use crate::runtime::Compositor;

const FIRST_TERMINAL_SWITCH: u32 = keysyms::KEY_XF86Switch_VT_1;
const LAST_TERMINAL_SWITCH: u32 = keysyms::KEY_XF86Switch_VT_12;

impl Compositor {
    pub(crate) fn handle_key(&mut self, keycode: Keycode, state: KeyState, time: u32) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
        self.input_serials.record(serial);
        let intercepted = keyboard.input(
            self,
            keycode,
            state,
            serial,
            time,
            |compositor, modifiers, handle| {
                compositor.filter_key(keycode, state, modifiers, &handle)
            },
        );
        if let Some(Some(action)) = intercepted {
            self.perform(action);
        }
    }

    pub(crate) fn release_held_keys(&mut self) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let time = self.event_time();
        for keycode in keyboard.pressed_keys() {
            self.handle_key(keycode, KeyState::Released, time);
        }
    }

    fn filter_key(
        &mut self,
        keycode: Keycode,
        state: KeyState,
        modifiers: &ModifiersState,
        handle: &KeysymHandle<'_>,
    ) -> FilterResult<Option<Action>> {
        if state == KeyState::Released {
            return if self.suppressed_keys.remove(&keycode) {
                FilterResult::Intercept(None)
            } else {
                FilterResult::Forward
            };
        }
        if let Some(terminal) = terminal_of(handle.modified_sym()) {
            self.suppressed_keys.insert(keycode);
            self.request_terminal_switch(terminal);
            return FilterResult::Intercept(None);
        }
        if self.lock.is_engaged() {
            return FilterResult::Forward;
        }
        let mut candidates = handle.raw_syms();
        candidates.push(handle.modified_sym());
        let held = held_modifiers(modifiers);
        let Some(action) = self.bindings.lookup(&self.keyboard_mode, held, &candidates) else {
            return FilterResult::Forward;
        };
        let action = action.clone();
        self.suppressed_keys.insert(keycode);
        FilterResult::Intercept(Some(action))
    }
}

fn terminal_of(symbol: Keysym) -> Option<i32> {
    let raw = symbol.raw();
    if !(FIRST_TERMINAL_SWITCH..=LAST_TERMINAL_SWITCH).contains(&raw) {
        return None;
    }
    i32::try_from(raw - FIRST_TERMINAL_SWITCH + 1).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_switch_symbols_name_their_terminal() {
        let first = terminal_of(Keysym::from(keysyms::KEY_XF86Switch_VT_1));
        let seventh = terminal_of(Keysym::from(keysyms::KEY_XF86Switch_VT_7));
        let last = terminal_of(Keysym::from(keysyms::KEY_XF86Switch_VT_12));

        assert_eq!((first, seventh, last), (Some(1), Some(7), Some(12)));
    }

    #[test]
    fn other_symbols_do_not_switch_terminals() {
        assert_eq!(terminal_of(Keysym::from(keysyms::KEY_F1)), None);
        assert_eq!(terminal_of(Keysym::from(keysyms::KEY_XF86AudioMute)), None);
    }
}
