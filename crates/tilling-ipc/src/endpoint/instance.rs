use std::fmt;
use std::str::FromStr;

const MAX_LENGTH: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("instance name must be 1 to {MAX_LENGTH} ASCII letters, digits, '-' or '_'")]
pub struct InvalidInstanceName;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceName(String);

impl InstanceName {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for InstanceName {
    type Err = InvalidInstanceName;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let is_allowed =
            |character: char| character.is_ascii_alphanumeric() || matches!(character, '-' | '_');
        if name.is_empty() || name.len() > MAX_LENGTH || !name.chars().all(is_allowed) {
            return Err(InvalidInstanceName);
        }
        Ok(Self(name.to_owned()))
    }
}

impl fmt::Display for InstanceName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_are_accepted() {
        for name in ["default", "nested-1", "scenario_ipc", "A9"] {
            assert_eq!(name.parse::<InstanceName>().unwrap().as_str(), name);
        }
    }

    #[test]
    fn names_that_could_escape_the_directory_are_rejected() {
        for name in ["", ".", "..", "a/b", "../etc", "name with space", "ação"] {
            assert_eq!(name.parse::<InstanceName>(), Err(InvalidInstanceName));
        }
    }

    #[test]
    fn length_is_limited() {
        assert!("a".repeat(MAX_LENGTH).parse::<InstanceName>().is_ok());
        assert!("a".repeat(MAX_LENGTH + 1).parse::<InstanceName>().is_err());
    }
}
