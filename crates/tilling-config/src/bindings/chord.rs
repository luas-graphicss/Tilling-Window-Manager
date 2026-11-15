use std::fmt;
use std::str::FromStr;

use crate::limits::MAX_KEY_NAME_BYTES;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyChord {
    modifiers: Modifiers,
    key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChordError {
    #[error("key name is missing")]
    MissingKey,
    #[error("`{0}` is not a key name")]
    InvalidKey(String),
    #[error("`{0}` is not a modifier, use Ctrl, Alt, Shift or Super")]
    UnknownModifier(String),
    #[error("modifier `{0}` is repeated")]
    RepeatedModifier(String),
}

impl KeyChord {
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl FromStr for KeyChord {
    type Err = ChordError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = parts
            .pop()
            .filter(|key| !key.is_empty())
            .ok_or(ChordError::MissingKey)?;
        if !is_key_name(key) {
            return Err(ChordError::InvalidKey(key.to_owned()));
        }
        let mut modifiers = Modifiers::default();
        for part in parts {
            let flag = modifiers
                .flag_mut(part)
                .ok_or_else(|| ChordError::UnknownModifier(part.to_owned()))?;
            if std::mem::replace(flag, true) {
                return Err(ChordError::RepeatedModifier(part.to_owned()));
            }
        }
        Ok(Self {
            modifiers,
            key: key.to_owned(),
        })
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = [
            (self.modifiers.control, "Ctrl"),
            (self.modifiers.alt, "Alt"),
            (self.modifiers.shift, "Shift"),
            (self.modifiers.logo, "Super"),
        ];
        for (_, name) in names.iter().filter(|(pressed, _)| *pressed) {
            write!(formatter, "{name}+")?;
        }
        formatter.write_str(&self.key)
    }
}

impl Modifiers {
    fn flag_mut(&mut self, name: &str) -> Option<&mut bool> {
        match name.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(&mut self.control),
            "alt" => Some(&mut self.alt),
            "shift" => Some(&mut self.shift),
            "super" | "logo" => Some(&mut self.logo),
            _ => None,
        }
    }
}

fn is_key_name(key: &str) -> bool {
    key.len() <= MAX_KEY_NAME_BYTES
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_order_and_case_do_not_change_the_chord() {
        let canonical: KeyChord = "Ctrl+Shift+Super+Return".parse().unwrap();
        let shuffled: KeyChord = "super + SHIFT + control + Return".parse().unwrap();

        assert_eq!(canonical, shuffled);
        assert_eq!(shuffled.to_string(), "Ctrl+Shift+Super+Return");
    }

    #[test]
    fn key_names_stay_case_sensitive() {
        let lower: KeyChord = "Super+q".parse().unwrap();
        let upper: KeyChord = "Super+Q".parse().unwrap();

        assert_ne!(lower, upper);
        assert_eq!(lower.key(), "q");
        assert!(lower.modifiers().logo);
    }

    #[test]
    fn malformed_chords_explain_the_problem() {
        let parse = |text: &str| text.parse::<KeyChord>();

        assert_eq!(parse("Super+"), Err(ChordError::MissingKey));
        assert_eq!(parse(""), Err(ChordError::MissingKey));
        assert_eq!(
            parse("Hyper+x"),
            Err(ChordError::UnknownModifier("Hyper".into()))
        );
        assert_eq!(
            parse("Super+Logo+x"),
            Err(ChordError::RepeatedModifier("Logo".into()))
        );
        assert_eq!(
            parse("Super+semi;colon"),
            Err(ChordError::InvalidKey("semi;colon".into()))
        );
    }
}
