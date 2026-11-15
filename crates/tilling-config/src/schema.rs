use serde::Deserialize;

use crate::bindings::DEFAULT_MODE;
use crate::diagnostics::{ConfigError, syntax_error};

pub(crate) const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConfig {
    schema_version: u32,
    workspaces: Option<Vec<String>>,
    layout: Option<RawLayout>,
    input: Option<RawInput>,
    applications: Option<RawApplications>,
    session: Option<RawSession>,
    bindings: Option<Vec<RawBinding>>,
    #[serde(default)]
    rules: Vec<RawRule>,
    #[serde(default)]
    outputs: Vec<RawOutput>,
    #[serde(default)]
    helpers: Vec<RawHelper>,
}

#[derive(Debug)]
pub(crate) struct Sections {
    pub(crate) workspaces: Vec<String>,
    pub(crate) layout: RawLayout,
    pub(crate) input: RawInput,
    pub(crate) applications: RawApplications,
    pub(crate) session: RawSession,
    pub(crate) bindings: Vec<RawBinding>,
    pub(crate) rules: Vec<RawRule>,
    pub(crate) outputs: Vec<RawOutput>,
    pub(crate) helpers: Vec<RawHelper>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawLayout {
    pub(crate) inner_gap: u32,
    pub(crate) outer_gap: u32,
    #[serde(default)]
    pub(crate) default: RawLayoutKind,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawLayoutKind {
    #[default]
    Bsp,
    MasterStack,
    Monocle,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawInput {
    pub(crate) keyboard: RawKeyboard,
    pub(crate) pointer: RawPointer,
    #[serde(default)]
    pub(crate) touchpad: RawTouchpad,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawKeyboard {
    pub(crate) layout: String,
    #[serde(default)]
    pub(crate) variant: String,
    pub(crate) repeat_rate: u32,
    pub(crate) repeat_delay_ms: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPointer {
    pub(crate) natural_scroll: bool,
    pub(crate) acceleration: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTouchpad {
    pub(crate) tap_to_click: bool,
    pub(crate) natural_scroll: bool,
    pub(crate) disable_while_typing: bool,
}

impl Default for RawTouchpad {
    fn default() -> Self {
        Self {
            tap_to_click: false,
            natural_scroll: false,
            disable_while_typing: true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawApplications {
    pub(crate) terminal: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSession {
    pub(crate) locker: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawHelper {
    pub(crate) name: String,
    pub(crate) command: Vec<String>,
    #[serde(default)]
    pub(crate) capabilities: Vec<RawCapability>,
    #[serde(default)]
    pub(crate) restart: RawRestart,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawCapability {
    DataControl,
    InputMethod,
    VirtualKeyboard,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawRestart {
    #[default]
    Never,
    OnFailure,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawBinding {
    #[serde(default = "default_mode")]
    pub(crate) mode: String,
    pub(crate) keys: String,
    pub(crate) action: RawAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum RawAction {
    Spawn(Vec<String>),
    OpenTerminal,
    CloseWindow,
    Focus(RawDirection),
    Move(RawDirection),
    SwitchWorkspace(String),
    MoveToWorkspace(String),
    FocusOutput(RawDirection),
    MoveToOutput(RawDirection),
    ToggleFullscreen,
    ToggleFloating,
    MoveToScratchpad,
    ToggleScratchpad,
    ToggleSplit,
    Resize(i16),
    SetLayout(RawLayoutKind),
    CycleLayout,
    EnterMode(String),
    ReloadConfig,
    Lock,
    Quit,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRule {
    pub(crate) app_id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) workspace: Option<String>,
    pub(crate) mode: Option<RawRuleMode>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawRuleMode {
    Tiled,
    Floating,
    Fullscreen,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawOutput {
    pub(crate) name: String,
    pub(crate) monitor: Option<String>,
    pub(crate) scale: Option<f64>,
    pub(crate) position: Option<RawPosition>,
    pub(crate) mode: Option<RawMode>,
    pub(crate) transform: Option<RawTransform>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMode {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) refresh: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum RawTransform {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "90")]
    Rotated90,
    #[serde(rename = "180")]
    Rotated180,
    #[serde(rename = "270")]
    Rotated270,
    #[serde(rename = "flipped")]
    Flipped,
    #[serde(rename = "flipped-90")]
    Flipped90,
    #[serde(rename = "flipped-180")]
    Flipped180,
    #[serde(rename = "flipped-270")]
    Flipped270,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Deserialize)]
struct VersionProbe {
    schema_version: u32,
}

impl RawConfig {
    pub(crate) fn complete(self, defaults: Self) -> Option<Sections> {
        Some(Sections {
            workspaces: self.workspaces.or(defaults.workspaces)?,
            layout: self.layout.or(defaults.layout)?,
            input: self.input.or(defaults.input)?,
            applications: self.applications.or(defaults.applications)?,
            session: self.session.or(defaults.session).unwrap_or_default(),
            bindings: self.bindings.or(defaults.bindings)?,
            rules: self.rules,
            outputs: self.outputs,
            helpers: self.helpers,
        })
    }
}

pub(crate) fn decode(text: &str) -> Result<RawConfig, ConfigError> {
    match toml::from_str::<RawConfig>(text) {
        Ok(raw) if raw.schema_version == SCHEMA_VERSION => Ok(raw),
        Ok(raw) => Err(unsupported(raw.schema_version)),
        Err(error) => Err(match toml::from_str::<VersionProbe>(text) {
            Ok(probe) if probe.schema_version != SCHEMA_VERSION => {
                unsupported(probe.schema_version)
            }
            Ok(_) | Err(_) => syntax_error(text, &error),
        }),
    }
}

fn unsupported(found: u32) -> ConfigError {
    ConfigError::UnsupportedSchemaVersion {
        found,
        supported: SCHEMA_VERSION,
    }
}

fn default_mode() -> String {
    DEFAULT_MODE.to_owned()
}
