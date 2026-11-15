use regex::{Regex, RegexBuilder};
use tilling_core::WindowMetadata;

use crate::diagnostics::Report;
use crate::limits::{MAX_COMPILED_PATTERN_BYTES, MAX_PATTERN_BYTES, MAX_RULES};
use crate::schema::{RawRule, RawRuleMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleMode {
    Tiled,
    Floating,
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleOutcome {
    pub workspace: Option<String>,
    pub mode: Option<RuleMode>,
}

#[derive(Debug, Clone)]
struct WindowRule {
    app_id: Option<Regex>,
    title: Option<Regex>,
    outcome: RuleOutcome,
}

#[derive(Debug, Clone, Default)]
pub struct WindowRules {
    rules: Vec<WindowRule>,
}

#[derive(Debug, thiserror::Error)]
enum PatternError {
    #[error("pattern must not exceed {MAX_PATTERN_BYTES} bytes")]
    TooLong,
    #[error("{0}")]
    Rejected(#[from] regex::Error),
}

impl WindowRules {
    #[must_use]
    pub fn resolve(&self, metadata: &WindowMetadata) -> Option<&RuleOutcome> {
        self.rules
            .iter()
            .find(|rule| rule.matches(metadata))
            .map(|rule| &rule.outcome)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl WindowRule {
    fn matches(&self, metadata: &WindowMetadata) -> bool {
        field_matches(self.app_id.as_ref(), metadata.app_id.as_deref())
            && field_matches(self.title.as_ref(), metadata.title.as_deref())
    }
}

fn field_matches(pattern: Option<&Regex>, value: Option<&str>) -> bool {
    match (pattern, value) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(pattern), Some(value)) => pattern.is_match(value),
    }
}

pub(crate) fn validate(
    raw: Vec<RawRule>,
    workspaces: &[String],
    report: &mut Report,
) -> WindowRules {
    if raw.len() > MAX_RULES {
        report.reject(
            "rules",
            format!("must not list more than {MAX_RULES} entries"),
        );
    }
    let rules = raw
        .into_iter()
        .take(MAX_RULES)
        .enumerate()
        .filter_map(|(index, rule)| validate_rule(index, rule, workspaces, report))
        .collect();
    WindowRules { rules }
}

fn validate_rule(
    index: usize,
    raw: RawRule,
    workspaces: &[String],
    report: &mut Report,
) -> Option<WindowRule> {
    let before = report.len();
    if raw.app_id.is_none() && raw.title.is_none() {
        report.reject(
            format!("rules[{index}]"),
            "must match on `app_id` or `title`",
        );
    }
    if raw.workspace.is_none() && raw.mode.is_none() {
        report.reject(format!("rules[{index}]"), "must set `workspace` or `mode`");
    }
    if let Some(workspace) = &raw.workspace
        && !workspaces.contains(workspace)
    {
        report.reject(
            format!("rules[{index}].workspace"),
            format!("workspace `{workspace}` is not listed in `workspaces`"),
        );
    }
    let app_id = compile_field(format!("rules[{index}].app_id"), raw.app_id, report);
    let title = compile_field(format!("rules[{index}].title"), raw.title, report);
    (report.len() == before).then(|| WindowRule {
        app_id,
        title,
        outcome: RuleOutcome {
            workspace: raw.workspace,
            mode: raw.mode.map(RuleMode::from),
        },
    })
}

fn compile_field(field: String, pattern: Option<String>, report: &mut Report) -> Option<Regex> {
    report.accept(field, compile(&pattern?))
}

fn compile(pattern: &str) -> Result<Regex, PatternError> {
    if pattern.len() > MAX_PATTERN_BYTES {
        return Err(PatternError::TooLong);
    }
    let regex = RegexBuilder::new(pattern)
        .size_limit(MAX_COMPILED_PATTERN_BYTES)
        .build()?;
    Ok(regex)
}

impl From<RawRuleMode> for RuleMode {
    fn from(raw: RawRuleMode) -> Self {
        match raw {
            RawRuleMode::Tiled => Self::Tiled,
            RawRuleMode::Floating => Self::Floating,
            RawRuleMode::Fullscreen => Self::Fullscreen,
        }
    }
}
