use crate::applications::{self, Applications};
use crate::bindings::{self, Binding};
use crate::diagnostics::{ConfigError, Report};
use crate::input::{self, InputConfig};
use crate::layout::{self, LayoutConfig};
use crate::names::validate_workspaces;
use crate::outputs::{self, OutputConfig};
use crate::rules::{self, WindowRules};
use crate::schema::Sections;
use crate::session::{self, SessionConfig};

#[derive(Debug)]
pub struct ValidatedConfig {
    workspaces: Vec<String>,
    layout: LayoutConfig,
    input: InputConfig,
    applications: Applications,
    session: SessionConfig,
    bindings: Vec<Binding>,
    rules: WindowRules,
    outputs: Vec<OutputConfig>,
}

impl ValidatedConfig {
    #[must_use]
    pub fn workspaces(&self) -> &[String] {
        &self.workspaces
    }

    #[must_use]
    pub fn layout(&self) -> LayoutConfig {
        self.layout
    }

    #[must_use]
    pub fn input(&self) -> &InputConfig {
        &self.input
    }

    #[must_use]
    pub fn applications(&self) -> &Applications {
        &self.applications
    }

    #[must_use]
    pub fn session(&self) -> &SessionConfig {
        &self.session
    }

    #[must_use]
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    #[must_use]
    pub fn rules(&self) -> &WindowRules {
        &self.rules
    }

    #[must_use]
    pub fn outputs(&self) -> &[OutputConfig] {
        &self.outputs
    }
}

pub(crate) fn validate(sections: Sections) -> Result<ValidatedConfig, ConfigError> {
    let mut report = Report::default();
    let workspaces = validate_workspaces(sections.workspaces, &mut report);
    let layout = layout::validate(&sections.layout, &mut report);
    let input = input::validate(sections.input, &mut report);
    let bindings = bindings::validate(sections.bindings, &workspaces, &mut report);
    let rules = rules::validate(sections.rules, &workspaces, &mut report);
    let outputs = outputs::validate(sections.outputs, &mut report);
    let session = session::validate(sections.session, sections.helpers, &mut report);
    let Some(applications) = applications::validate(sections.applications, &mut report) else {
        return Err(report.into_error());
    };
    report.finish(ValidatedConfig {
        workspaces,
        layout,
        input,
        applications,
        session,
        bindings,
        rules,
        outputs,
    })
}
