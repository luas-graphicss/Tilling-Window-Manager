use smithay::utils::Transform;
use tilling_core::geometry::Scale;
use tilling_ipc::{OutputTransform, Reply, ResourceId};

use super::refusal::Refusal;
use super::resources;
use crate::output::{ModeRequest, OutputChange};
use crate::runtime::Compositor;

impl Compositor {
    pub(super) fn configure_output(
        &mut self,
        output: ResourceId,
        change: tilling_ipc::OutputChange,
    ) -> Result<Reply, Refusal> {
        let output = resources::output(&self.desktop, output)?;
        let change = requested(change)?;
        let window = self
            .try_output_change(output, change)
            .map_err(|error| Refusal::invalid_because(&error))?;
        Ok(Reply::OutputChangePending {
            revision: self.desktop.revision(),
            confirm_within_ms: u64::try_from(window.as_millis()).unwrap_or(u64::MAX),
        })
    }

    pub(super) fn confirm_output(&mut self) -> Result<Reply, Refusal> {
        self.confirm_output_change()
            .map_err(|error| Refusal::invalid_because(&error))?;
        Ok(self.accepted())
    }

    pub(super) fn revert_output(&mut self) -> Result<Reply, Refusal> {
        self.revert_output_change()
            .map_err(|error| Refusal::invalid_because(&error))?;
        Ok(self.accepted())
    }
}

fn requested(change: tilling_ipc::OutputChange) -> Result<OutputChange, Refusal> {
    let scale = change
        .scale
        .map(Scale::from_factor)
        .transpose()
        .map_err(|error| Refusal::invalid_because(&error))?;
    Ok(OutputChange {
        mode: change.mode.map(|mode| ModeRequest {
            width: mode.width,
            height: mode.height,
            refresh_millihertz: mode.refresh_millihertz,
        }),
        position: change.position.map(|position| (position.x, position.y)),
        scale,
        transform: change.transform.map(transform_of),
    })
}

const fn transform_of(requested: OutputTransform) -> Transform {
    match requested {
        OutputTransform::Normal => Transform::Normal,
        OutputTransform::Rotated90 => Transform::_90,
        OutputTransform::Rotated180 => Transform::_180,
        OutputTransform::Rotated270 => Transform::_270,
        OutputTransform::Flipped => Transform::Flipped,
        OutputTransform::Flipped90 => Transform::Flipped90,
        OutputTransform::Flipped180 => Transform::Flipped180,
        OutputTransform::Flipped270 => Transform::Flipped270,
    }
}

#[cfg(test)]
mod tests {
    use tilling_ipc::{OutputMode, OutputPosition};

    use super::*;

    #[test]
    fn request_is_translated_setting_by_setting() {
        let change = tilling_ipc::OutputChange {
            mode: Some(OutputMode {
                width: 1920,
                height: 1080,
                refresh_millihertz: None,
            }),
            position: Some(OutputPosition { x: -1920, y: 0 }),
            scale: Some(1.25),
            transform: Some(OutputTransform::Flipped270),
        };

        assert_eq!(
            requested(change),
            Ok(OutputChange {
                mode: Some(ModeRequest {
                    width: 1920,
                    height: 1080,
                    refresh_millihertz: None,
                }),
                position: Some((-1920, 0)),
                scale: Scale::from_factor(1.25).ok(),
                transform: Some(Transform::Flipped270),
            })
        );
    }

    #[test]
    fn scale_outside_the_supported_range_is_refused() {
        for factor in [0.0, 0.1, 9.0, f64::NAN] {
            let change = tilling_ipc::OutputChange {
                scale: Some(factor),
                ..tilling_ipc::OutputChange::default()
            };

            assert!(matches!(requested(change), Err(Refusal::Invalid(_))));
        }
    }
}
