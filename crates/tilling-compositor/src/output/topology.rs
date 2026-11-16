use std::time::Duration;

#[cfg(feature = "nested")]
use smithay::output::Mode;
use smithay::output::{Output, PhysicalProperties, Scale as AdvertisedScale};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Point;
use smithay::wayland::fractional_scale::with_fractional_scale;
use tilling_core::{Command, Effect, OutputId, Transition, WindowId};

use super::description::OutputDescription;
use super::placement::Arrangement;
#[cfg(feature = "nested")]
use super::placement::{logical_area, scale_for};
use super::profile;
use super::registry::Registered;
use crate::error::CompositorError;
use crate::runtime::Compositor;

const GLOBAL_LINGER: Duration = Duration::from_secs(5);

impl Compositor {
    pub(crate) fn attach_output(
        &mut self,
        description: OutputDescription,
    ) -> Result<Output, CompositorError> {
        let profile = profile::find(
            self.config.outputs(),
            &description.name,
            description.monitor_key.as_deref(),
        );
        let occupied = self.desktop.outputs().map(|(_, output)| output.area());
        let arrangement = Arrangement::plan(
            profile,
            description.mode.size,
            description.suggested_scale,
            occupied,
        )?;
        let output = Output::new(
            description.name.clone(),
            PhysicalProperties {
                size: description.physical_size,
                subpixel: description.subpixel,
                make: description.make,
                model: description.model,
            },
        );
        let area = arrangement.area;
        output.change_current_state(
            Some(description.mode),
            Some(arrangement.transform),
            Some(AdvertisedScale::Fractional(arrangement.scale.factor())),
            Some(Point::from((area.x(), area.y()))),
        );
        output.set_preferred(description.mode);
        let transition = self.execute(Command::AddOutput {
            name: description.name,
            area,
            scale: arrangement.scale,
        })?;
        if let Some(id) = added_output(&transition) {
            let global = output.create_global::<Self>(&self.display_handle);
            self.outputs.insert(Registered {
                id,
                output: output.clone(),
                global,
            });
        }
        self.demand_frame();
        Ok(output)
    }

    pub(crate) fn detach_output(&mut self, output: &Output) {
        let Some(registered) = self.outputs.remove(output) else {
            return;
        };
        Self::close_layers(output);
        self.forget_output_trial(registered.id);
        self.retire_global(registered.global);
        self.run(Command::RemoveOutput {
            output: registered.id,
        });
        self.confine_pointer();
        self.demand_frame();
    }

    #[cfg(feature = "nested")]
    pub(crate) fn resize_output(&mut self, output: &Output, mode: Mode, suggested_scale: f64) {
        let Some(id) = self.outputs.id_of(output) else {
            return;
        };
        let profile = profile::find(self.config.outputs(), &output.name(), None);
        let scale = scale_for(profile, suggested_scale);
        let position = output.current_location();
        let transform = output.current_transform();
        let area = match logical_area((position.x, position.y), mode.size, transform, scale) {
            Ok(area) => area,
            Err(error) => {
                tracing::warn!(%error, "keep the previous output size");
                return;
            }
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

    #[cfg(feature = "drm")]
    pub(crate) fn output_profile(
        &self,
        name: &str,
        monitor_key: Option<&str>,
    ) -> Option<&tilling_config::OutputConfig> {
        profile::find(self.config.outputs(), name, monitor_key)
    }

    pub(crate) fn output_showing(&self, window: WindowId) -> Option<&Output> {
        let workspace = self.desktop.window(window)?.workspace();
        let owner = self.desktop.workspace(workspace)?.output()?;
        self.outputs.by_id(owner)
    }

    pub(crate) fn announce_surface_outputs(&self) {
        for placed in self.scene.windows() {
            let Some(managed) = self.windows.by_id(placed.id) else {
                continue;
            };
            let scale = self
                .outputs
                .by_id(placed.output)
                .map(|output| output.current_scale().fractional_scale());
            managed.window.with_surfaces(|surface, states| {
                for (id, output) in self.outputs.iter() {
                    if id == placed.output {
                        output.enter(surface);
                    } else {
                        output.leave(surface);
                    }
                }
                if let Some(scale) = scale {
                    with_fractional_scale(states, |fractional| {
                        fractional.set_preferred_scale(scale);
                    });
                }
            });
        }
    }

    pub(crate) fn preferred_scale_of(&self, toplevel: &WlSurface) -> Option<f64> {
        let shown_on = self
            .windows
            .id_of(toplevel)
            .and_then(|window| self.output_showing(window));
        let output = shown_on.or_else(|| self.active_output())?;
        Some(output.current_scale().fractional_scale())
    }

    pub(crate) fn active_output(&self) -> Option<&Output> {
        let active = self.desktop.workspace(self.desktop.active_workspace())?;
        let owner = active.output().and_then(|owner| self.outputs.by_id(owner));
        owner.or_else(|| self.outputs.iter().next().map(|(_, output)| output))
    }

    fn retire_global(&mut self, global: GlobalId) {
        self.display_handle.disable_global::<Self>(global.clone());
        let lingering = global.clone();
        let timer = Timer::from_duration(GLOBAL_LINGER);
        let armed = self
            .loop_handle
            .insert_source(timer, move |_, (), runtime| {
                let display = &runtime.compositor.display_handle;
                display.remove_global::<Self>(lingering.clone());
                TimeoutAction::Drop
            });
        if let Err(error) = armed {
            tracing::warn!(error = %error.error, "remove the output global at once");
            self.display_handle.remove_global::<Self>(global);
        }
    }
}

fn added_output(transition: &Transition) -> Option<OutputId> {
    transition.effects.iter().find_map(|effect| match effect {
        Effect::OutputAdded(id) => Some(*id),
        _ => None,
    })
}
