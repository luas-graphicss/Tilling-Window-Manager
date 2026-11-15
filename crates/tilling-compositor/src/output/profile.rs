use tilling_config::OutputConfig;

pub(super) fn find<'config>(
    configured: &'config [OutputConfig],
    name: &str,
    monitor_key: Option<&str>,
) -> Option<&'config OutputConfig> {
    let for_monitor = monitor_key.and_then(|key| {
        configured
            .iter()
            .find(|entry| entry.monitor.as_deref() == Some(key))
    });
    for_monitor.or_else(|| {
        configured
            .iter()
            .find(|entry| entry.monitor.is_none() && entry.name == name)
    })
}

#[cfg(test)]
mod tests {
    use tilling_config::ValidatedConfig;
    use tilling_core::geometry::Scale;

    use super::*;

    fn configured() -> ValidatedConfig {
        tilling_config::parse(
            r#"
schema_version = 1

[[outputs]]
name = "DP-1"
scale = 2.0

[[outputs]]
name = "desk"
monitor = "DEL/DELL U2720Q/7RGSN83"
scale = 1.5
"#,
        )
        .unwrap()
    }

    fn scale_of(profile: Option<&OutputConfig>) -> Option<Scale> {
        profile.and_then(|profile| profile.scale)
    }

    #[test]
    fn monitor_key_wins_over_the_connector_it_is_plugged_into() {
        let config = configured();

        let profile = find(config.outputs(), "DP-1", Some("DEL/DELL U2720Q/7RGSN83"));

        assert_eq!(scale_of(profile), Scale::from_factor(1.5).ok());
    }

    #[test]
    fn connector_name_applies_to_monitors_without_their_own_profile() {
        let config = configured();

        let unknown_monitor = find(config.outputs(), "DP-1", Some("AOC/2460G5/1"));
        let no_edid = find(config.outputs(), "DP-1", None);

        assert_eq!(scale_of(unknown_monitor), Scale::from_factor(2.0).ok());
        assert_eq!(scale_of(no_edid), Scale::from_factor(2.0).ok());
    }

    #[test]
    fn monitor_profile_is_not_matched_by_its_label() {
        let config = configured();

        assert_eq!(find(config.outputs(), "desk", None), None);
        assert_eq!(find(config.outputs(), "HDMI-A-1", None), None);
    }
}
