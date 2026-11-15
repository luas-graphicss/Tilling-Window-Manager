use smithay::output::{Mode, Output, PhysicalProperties, Scale as AdvertisedScale, Subpixel};
use smithay::utils::{Logical, Physical, Point, Size, Transform};
use tilling_core::geometry::{LogicalRect, Scale};
use tilling_core::{Command, Effect, OutputId};

use super::Compositor;
use crate::error::CompositorError;

const MAKE: &str = "Tilling";
const MODEL: &str = "Virtual output";

#[derive(Debug, Default)]
pub(crate) struct OutputRegistry {
    entries: Vec<(OutputId, Output)>,
}

impl OutputRegistry {
    pub(crate) fn primary(&self) -> Option<&Output> {
        self.entries.first().map(|(_, output)| output)
    }

    pub(crate) fn id_of(&self, output: &Output) -> Option<OutputId> {
        self.entries
            .iter()
            .find(|(_, known)| known == output)
            .map(|(id, _)| *id)
    }

    fn insert(&mut self, id: OutputId, output: Output) {
        self.entries.push((id, output));
    }
}

impl Compositor {
    pub(crate) fn attach_output(
        &mut self,
        name: &str,
        size: Size<i32, Physical>,
        refresh_millihertz: i32,
        host_scale: f64,
    ) -> Result<Output, CompositorError> {
        let configured = self
            .config
            .outputs()
            .iter()
            .find(|output| output.name == name);
        let scale = self.output_scale(name, host_scale);
        let position = configured
            .and_then(|output| output.position)
            .map_or((0, 0), |position| (position.x, position.y));
        let area = logical_area(position, size, scale)?;
        let output = Output::new(
            name.to_owned(),
            PhysicalProperties {
                size: Size::from((0, 0)),
                subpixel: Subpixel::Unknown,
                make: MAKE.to_owned(),
                model: MODEL.to_owned(),
            },
        );
        let mode = Mode {
            size,
            refresh: refresh_millihertz,
        };
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(AdvertisedScale::Fractional(scale.factor())),
            Some(Point::from(position)),
        );
        output.set_preferred(mode);
        output.create_global::<Self>(&self.display_handle);
        let transition = self.desktop.apply(Command::AddOutput {
            name: name.to_owned(),
            area,
            scale,
        })?;
        let added = transition.effects.iter().find_map(|effect| match effect {
            Effect::OutputAdded(id) => Some(*id),
            _ => None,
        });
        if let Some(id) = added {
            self.outputs.insert(id, output.clone());
        }
        Ok(output)
    }

    #[cfg(feature = "nested")]
    pub(crate) fn resize_output(
        &mut self,
        output: &Output,
        size: Size<i32, Physical>,
        refresh_millihertz: i32,
        host_scale: f64,
    ) {
        let Some(id) = self.outputs.id_of(output) else {
            return;
        };
        let scale = self.output_scale(&output.name(), host_scale);
        let position = output.current_location();
        let area = match logical_area((position.x, position.y), size, scale) {
            Ok(area) => area,
            Err(error) => {
                tracing::warn!(%error, "keep the previous output size");
                return;
            }
        };
        let mode = Mode {
            size,
            refresh: refresh_millihertz,
        };
        let advertised = AdvertisedScale::Fractional(scale.factor());
        output.change_current_state(Some(mode), None, Some(advertised), None);
        output.set_preferred(mode);
        self.run(Command::ConfigureOutput {
            output: id,
            area,
            scale,
        });
        self.demand_frame();
    }

    fn output_scale(&self, name: &str, host_scale: f64) -> Scale {
        self.config
            .outputs()
            .iter()
            .find(|output| output.name == name)
            .and_then(|output| output.scale)
            .or_else(|| Scale::from_factor(host_scale).ok())
            .unwrap_or(Scale::ONE)
    }
}

fn logical_area(
    position: (i32, i32),
    size: Size<i32, Physical>,
    scale: Scale,
) -> Result<LogicalRect, CompositorError> {
    let logical: Size<i32, Logical> = size.to_f64().to_logical(scale.factor()).to_i32_round();
    let unusable = || CompositorError::OutputSize {
        width: size.w.unsigned_abs(),
        height: size.h.unsigned_abs(),
    };
    let width = u32::try_from(logical.w).map_err(|_| unusable())?;
    let height = u32::try_from(logical.h).map_err(|_| unusable())?;
    let area = LogicalRect::new(position.0, position.1, width, height).map_err(|_| unusable())?;
    if area.is_empty() {
        return Err(unusable());
    }
    Ok(area)
}
