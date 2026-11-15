pub const MAX_CONFIG_BYTES: usize = 256 * 1024;

pub(crate) const MAX_WORKSPACES: usize = 64;
pub(crate) const MAX_BINDINGS: usize = 512;
pub(crate) const MAX_RULES: usize = 256;
pub(crate) const MAX_OUTPUTS: usize = 16;
pub(crate) const MAX_NAME_BYTES: usize = 64;
pub(crate) const MAX_KEY_NAME_BYTES: usize = 32;
pub(crate) const MAX_COMMAND_ARGUMENTS: usize = 64;
pub(crate) const MAX_ARGUMENT_BYTES: usize = 1_024;
pub(crate) const MAX_PATTERN_BYTES: usize = 256;
pub(crate) const MAX_COMPILED_PATTERN_BYTES: usize = 64 * 1024;
pub(crate) const MAX_GAP: u32 = 256;
pub(crate) const MAX_RESIZE_STEP: u16 = 4_000;
pub(crate) const MAX_REPEAT_RATE: u32 = 100;
pub(crate) const MIN_REPEAT_DELAY_MS: u32 = 100;
pub(crate) const MAX_REPEAT_DELAY_MS: u32 = 5_000;
