mod action;
mod chord;

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use self::action::References;
use crate::diagnostics::Report;
use crate::limits::{MAX_BINDINGS, MAX_KEY_NAME_BYTES};
use crate::schema::RawBinding;

pub use action::Action;
pub use chord::{ChordError, KeyChord, Modifiers};

pub const DEFAULT_MODE: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub mode: String,
    pub chord: KeyChord,
    pub action: Action,
}

pub(crate) fn validate(
    raw: Vec<RawBinding>,
    workspaces: &[String],
    report: &mut Report,
) -> Vec<Binding> {
    if raw.len() > MAX_BINDINGS {
        report.reject(
            "bindings",
            format!("must not list more than {MAX_BINDINGS} entries"),
        );
    }
    let modes: BTreeSet<String> = raw
        .iter()
        .take(MAX_BINDINGS)
        .map(|binding| binding.mode.clone())
        .chain([DEFAULT_MODE.to_owned()])
        .collect();
    let references = References {
        workspaces,
        modes: &modes,
    };
    let mut first_use: BTreeMap<(String, KeyChord), usize> = BTreeMap::new();
    let mut bindings = Vec::new();
    for (index, entry) in raw.into_iter().take(MAX_BINDINGS).enumerate() {
        if !is_mode_name(&entry.mode) {
            report.reject(
                format!("bindings[{index}].mode"),
                "must use lowercase letters, digits, `-` or `_`",
            );
        }
        let chord = report.accept(
            format!("bindings[{index}].keys"),
            entry.keys.parse::<KeyChord>(),
        );
        let action = report.accept(
            format!("bindings[{index}].action"),
            action::validate(entry.action, &references),
        );
        let (Some(chord), Some(action)) = (chord, action) else {
            continue;
        };
        match first_use.entry((entry.mode.clone(), chord.clone())) {
            Entry::Occupied(first) => report.reject(
                format!("bindings[{index}].keys"),
                format!(
                    "`{chord}` is already bound in mode `{}` by bindings[{}]",
                    entry.mode,
                    first.get()
                ),
            ),
            Entry::Vacant(slot) => {
                slot.insert(index);
                bindings.push(Binding {
                    mode: entry.mode,
                    chord,
                    action,
                });
            }
        }
    }
    bindings
}

fn is_mode_name(mode: &str) -> bool {
    !mode.is_empty()
        && mode.len() <= MAX_KEY_NAME_BYTES
        && mode.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}
