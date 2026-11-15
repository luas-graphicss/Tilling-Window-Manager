use std::error::Error;

use tilling_config::{ConfigError, MAX_CONFIG_BYTES, ValidatedConfig};
use tilling_core::WindowMetadata;

const PROBED_FIELD_CHARS: usize = 48;

pub fn check(input: &[u8]) {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    match tilling_config::parse(text) {
        Ok(config) => {
            assert!(
                text.len() <= MAX_CONFIG_BYTES,
                "configuration beyond the size limit was accepted"
            );
            assert!(
                !config.workspaces().is_empty(),
                "accepted configuration has no workspace"
            );
            probe_rules(&config, text);
        }
        Err(rejection) => assert!(
            !describe(&rejection).is_empty(),
            "rejection carries no description"
        ),
    }
}

fn probe_rules(config: &ValidatedConfig, text: &str) {
    let leading: String = text.chars().take(PROBED_FIELD_CHARS).collect();
    let trailing: String = text.chars().rev().take(PROBED_FIELD_CHARS).collect();
    let probes = [
        WindowMetadata::default(),
        WindowMetadata {
            app_id: Some(leading),
            title: Some(trailing),
        },
    ];
    for metadata in &probes {
        let Some(outcome) = config.rules().resolve(metadata) else {
            continue;
        };
        assert!(
            outcome
                .workspace
                .as_ref()
                .is_none_or(|name| config.workspaces().contains(name)),
            "rule sends windows to a workspace that does not exist"
        );
    }
}

fn describe(rejection: &ConfigError) -> String {
    let mut description = rejection.to_string();
    let mut cause = rejection.source();
    while let Some(source) = cause {
        description.push_str(": ");
        description.push_str(&source.to_string());
        cause = source.source();
    }
    description
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_and_rejected_documents_are_both_checked() {
        check(b"schema_version = 1\n");
        check(b"schema_version = 7\n");
        check(b"[layout\n");
        check(&[0xff, 0xfe]);
    }

    #[test]
    fn rules_are_resolved_against_text_taken_from_the_document() {
        let document = "schema_version = 1\n\n[[rules]]\napp_id = \"schema\"\nworkspace = \"2\"\n";
        let config = tilling_config::parse(document).unwrap();

        probe_rules(&config, document);

        assert_eq!(config.rules().len(), 1);
    }

    #[test]
    fn rejections_name_the_cause() {
        let rejection = tilling_config::parse("schema_version = 7\n").unwrap_err();

        assert!(describe(&rejection).contains("schema_version 7"));
    }
}
