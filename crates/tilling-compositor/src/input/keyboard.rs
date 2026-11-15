use smithay::backend::input::KeyState;
use smithay::input::keyboard::{FilterResult, Keycode, KeysymHandle, ModifiersState};
use smithay::utils::SERIAL_COUNTER;
use tilling_config::Action;

use super::bindings::held_modifiers;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn handle_key(&mut self, keycode: Keycode, state: KeyState, time: u32) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
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
