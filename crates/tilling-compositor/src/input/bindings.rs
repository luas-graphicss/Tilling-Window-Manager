use std::collections::BTreeMap;

use smithay::input::keyboard::{Keysym, ModifiersState, xkb};
use tilling_config::{Action, Binding, Modifiers};

#[derive(Debug)]
struct Shortcut {
    modifiers: Modifiers,
    keysym: Keysym,
    action: Action,
}

#[derive(Debug, Default)]
pub(crate) struct BindingTable {
    modes: BTreeMap<String, Vec<Shortcut>>,
}

impl BindingTable {
    pub(crate) fn compile(bindings: &[Binding]) -> Self {
        let mut modes: BTreeMap<String, Vec<Shortcut>> = BTreeMap::new();
        for binding in bindings {
            let Some(keysym) = keysym_named(binding.chord.key()) else {
                tracing::warn!(chord = %binding.chord, "ignore binding with unknown key name");
                continue;
            };
            modes
                .entry(binding.mode.clone())
                .or_default()
                .push(Shortcut {
                    modifiers: binding.chord.modifiers(),
                    keysym,
                    action: binding.action.clone(),
                });
        }
        Self { modes }
    }

    pub(crate) fn lookup(
        &self,
        mode: &str,
        modifiers: Modifiers,
        candidates: &[Keysym],
    ) -> Option<&Action> {
        self.modes
            .get(mode)?
            .iter()
            .find(|shortcut| {
                shortcut.modifiers == modifiers && candidates.contains(&shortcut.keysym)
            })
            .map(|shortcut| &shortcut.action)
    }

    pub(crate) fn has_mode(&self, mode: &str) -> bool {
        self.modes.contains_key(mode)
    }
}

pub(crate) fn held_modifiers(state: &ModifiersState) -> Modifiers {
    Modifiers {
        control: state.ctrl,
        alt: state.alt,
        shift: state.shift,
        logo: state.logo,
    }
}

fn keysym_named(name: &str) -> Option<Keysym> {
    [xkb::KEYSYM_NO_FLAGS, xkb::KEYSYM_CASE_INSENSITIVE]
        .into_iter()
        .map(|flags| xkb::keysym_from_name(name, flags))
        .find(|keysym| keysym.raw() != xkb::keysyms::KEY_NoSymbol)
}

#[cfg(test)]
mod tests {
    use tilling_core::layout::Direction;

    use super::*;

    const SUPER: Modifiers = Modifiers {
        control: false,
        alt: false,
        shift: false,
        logo: true,
    };

    fn binding(mode: &str, keys: &str, action: Action) -> Binding {
        Binding {
            mode: mode.to_owned(),
            chord: keys.parse().unwrap(),
            action,
        }
    }

    fn keysym(name: &str) -> Keysym {
        keysym_named(name).unwrap()
    }

    #[test]
    fn chord_matches_only_with_the_same_modifiers() {
        let table = BindingTable::compile(&[binding("default", "Super+h", Action::CloseWindow)]);
        let shifted = Modifiers {
            shift: true,
            ..SUPER
        };

        assert_eq!(
            table.lookup("default", SUPER, &[keysym("h")]),
            Some(&Action::CloseWindow)
        );
        assert_eq!(table.lookup("default", shifted, &[keysym("h")]), None);
        assert_eq!(
            table.lookup("default", Modifiers::default(), &[keysym("h")]),
            None
        );
    }

    #[test]
    fn shifted_chord_matches_the_unshifted_symbol_of_the_key() {
        let shifted = Modifiers {
            shift: true,
            ..SUPER
        };
        let table = BindingTable::compile(&[binding(
            "default",
            "Super+Shift+1",
            Action::MoveToWorkspace("1".into()),
        )]);

        let action = table.lookup("default", shifted, &[keysym("1"), keysym("exclam")]);

        assert_eq!(action, Some(&Action::MoveToWorkspace("1".into())));
    }

    #[test]
    fn modes_keep_separate_shortcuts() {
        let table = BindingTable::compile(&[
            binding("default", "Super+l", Action::Focus(Direction::Right)),
            binding("resize", "l", Action::Resize(500)),
        ]);

        assert_eq!(
            table.lookup("resize", Modifiers::default(), &[keysym("l")]),
            Some(&Action::Resize(500))
        );
        assert_eq!(
            table.lookup("default", Modifiers::default(), &[keysym("l")]),
            None
        );
        assert!(table.has_mode("resize") && !table.has_mode("launch"));
    }

    #[test]
    fn binding_with_an_unknown_key_name_is_left_out() {
        let table = BindingTable::compile(&[
            binding("default", "Super+NotAKey", Action::Quit),
            binding("default", "Super+Return", Action::OpenTerminal),
        ]);

        assert_eq!(
            table.lookup("default", SUPER, &[keysym("Return")]),
            Some(&Action::OpenTerminal)
        );
        assert_eq!(keysym_named("NotAKey"), None);
    }
}
