use clap::{Subcommand, ValueEnum};
use tilling_ipc::{Call, OutputChange, OutputMode, OutputPosition, OutputTransform, ResourceId};

use super::parse_resource_id;

const MILLIHERTZ_PER_HERTZ: f64 = 1_000.0;
const MAX_REFRESH_HERTZ: f64 = 1_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Subcommand)]
pub enum OutputAction {
    #[command(about = "Apply a change that is reverted unless it is confirmed")]
    Set {
        #[arg(value_name = "OUTPUT", value_parser = parse_resource_id)]
        output: ResourceId,
        #[arg(long, value_name = "WIDTHxHEIGHT[@HZ]", value_parser = parse_mode)]
        mode: Option<OutputMode>,
        #[arg(
            long,
            value_name = "X,Y",
            value_parser = parse_position,
            allow_hyphen_values = true
        )]
        position: Option<OutputPosition>,
        #[arg(long, value_name = "FACTOR")]
        scale: Option<f64>,
        #[arg(long, value_enum)]
        transform: Option<Transform>,
    },
    #[command(about = "Keep the change that is waiting for confirmation")]
    Confirm,
    #[command(about = "Undo the change that is waiting for confirmation")]
    Revert,
}

impl OutputAction {
    pub fn call(self) -> Call {
        match self {
            Self::Set {
                output,
                mode,
                position,
                scale,
                transform,
            } => Call::ConfigureOutput {
                output,
                change: OutputChange {
                    mode,
                    position,
                    scale,
                    transform: transform.map(OutputTransform::from),
                },
            },
            Self::Confirm => Call::ConfirmOutputChange,
            Self::Revert => Call::RevertOutputChange,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Transform {
    Normal,
    #[value(name = "90")]
    Rotated90,
    #[value(name = "180")]
    Rotated180,
    #[value(name = "270")]
    Rotated270,
    Flipped,
    #[value(name = "flipped-90")]
    Flipped90,
    #[value(name = "flipped-180")]
    Flipped180,
    #[value(name = "flipped-270")]
    Flipped270,
}

impl From<Transform> for OutputTransform {
    fn from(transform: Transform) -> Self {
        match transform {
            Transform::Normal => Self::Normal,
            Transform::Rotated90 => Self::Rotated90,
            Transform::Rotated180 => Self::Rotated180,
            Transform::Rotated270 => Self::Rotated270,
            Transform::Flipped => Self::Flipped,
            Transform::Flipped90 => Self::Flipped90,
            Transform::Flipped180 => Self::Flipped180,
            Transform::Flipped270 => Self::Flipped270,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected WIDTHxHEIGHT or WIDTHxHEIGHT@HZ, for example 1920x1080@60")]
pub struct InvalidMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected X,Y, for example -1920,0")]
pub struct InvalidPosition;

fn parse_mode(text: &str) -> Result<OutputMode, InvalidMode> {
    let (size, rate) = match text.split_once('@') {
        Some((size, rate)) => (size, Some(rate)),
        None => (text, None),
    };
    let (width, height) = size.split_once('x').ok_or(InvalidMode)?;
    Ok(OutputMode {
        width: width.parse().map_err(|_| InvalidMode)?,
        height: height.parse().map_err(|_| InvalidMode)?,
        refresh_millihertz: rate.map(parse_refresh).transpose()?,
    })
}

fn parse_refresh(text: &str) -> Result<u32, InvalidMode> {
    let hertz: f64 = text.parse().map_err(|_| InvalidMode)?;
    if !hertz.is_finite() || hertz <= 0.0 || hertz > MAX_REFRESH_HERTZ {
        return Err(InvalidMode);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the rate was range-checked, so the product lies between 0 and 1000000"
    )]
    let millihertz = (hertz * MILLIHERTZ_PER_HERTZ).round() as u32;
    Ok(millihertz)
}

fn parse_position(text: &str) -> Result<OutputPosition, InvalidPosition> {
    let (x, y) = text.split_once(',').ok_or(InvalidPosition)?;
    Ok(OutputPosition {
        x: x.parse().map_err(|_| InvalidPosition)?,
        y: y.parse().map_err(|_| InvalidPosition)?,
    })
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use clap::error::ErrorKind;

    use super::*;
    use crate::arguments::{Arguments, Operation};

    const OUTPUT: ResourceId = ResourceId {
        index: 1,
        generation: 0,
    };

    fn parse(line: &[&str]) -> Result<Arguments, clap::Error> {
        Arguments::try_parse_from([&["tillingctl", "output"], line].concat())
    }

    fn call_of(line: &[&str]) -> Option<Call> {
        match parse(line).unwrap().command.operation() {
            Operation::Call(call) => Some(call),
            _ => None,
        }
    }

    #[test]
    fn set_carries_only_the_settings_given_on_the_command_line() {
        let scale_only = call_of(&["set", "1:0", "--scale", "1.5"]);

        assert_eq!(
            scale_only,
            Some(Call::ConfigureOutput {
                output: OUTPUT,
                change: OutputChange {
                    scale: Some(1.5),
                    ..OutputChange::default()
                },
            })
        );
    }

    #[test]
    fn set_reads_mode_position_and_rotation() {
        let whole = call_of(&[
            "set",
            "1:0",
            "--mode",
            "2560x1440@143.912",
            "--position",
            "-2560,0",
            "--transform",
            "flipped-90",
        ]);

        assert_eq!(
            whole,
            Some(Call::ConfigureOutput {
                output: OUTPUT,
                change: OutputChange {
                    mode: Some(OutputMode {
                        width: 2560,
                        height: 1440,
                        refresh_millihertz: Some(143_912),
                    }),
                    position: Some(OutputPosition { x: -2560, y: 0 }),
                    scale: None,
                    transform: Some(OutputTransform::Flipped90),
                },
            })
        );
    }

    #[test]
    fn mode_without_a_rate_leaves_the_rate_to_the_compositor() {
        assert_eq!(
            parse_mode("1920x1080"),
            Ok(OutputMode {
                width: 1920,
                height: 1080,
                refresh_millihertz: None,
            })
        );
    }

    #[test]
    fn confirm_and_revert_take_no_parameters() {
        assert_eq!(call_of(&["confirm"]), Some(Call::ConfirmOutputChange));
        assert_eq!(call_of(&["revert"]), Some(Call::RevertOutputChange));
    }

    #[test]
    fn malformed_modes_positions_and_rotations_are_rejected() {
        let rejected = [
            ["--mode", "1920"],
            ["--mode", "1920x"],
            ["--mode", "1920x1080@0"],
            ["--mode", "1920x1080@fast"],
            ["--position", "100"],
            ["--position", "left,top"],
            ["--transform", "45"],
        ];

        for [option, value] in rejected {
            let failure = parse(&["set", "1:0", option, value]).unwrap_err();

            assert!(
                matches!(
                    failure.kind(),
                    ErrorKind::ValueValidation | ErrorKind::InvalidValue
                ),
                "{option} {value}"
            );
        }
    }
}
