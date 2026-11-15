use std::time::Duration;

use smithay::backend::drm::{DrmEventMetadata, DrmEventTime};
use smithay::backend::renderer::element::{
    RenderElementStates, default_primary_scanout_output_compare,
};
use smithay::desktop::utils::{
    OutputPresentationFeedback, surface_presentation_feedback_flags_from_states,
    surface_primary_scanout_output, update_surface_primary_scanout_output,
};
use smithay::output::Output;
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback::Kind;
use smithay::utils::{Clock, Monotonic};
use smithay::wayland::presentation::Refresh;

use crate::runtime::Compositor;

pub(super) fn collect(
    compositor: &Compositor,
    output: &Output,
    states: &RenderElementStates,
) -> OutputPresentationFeedback {
    let mut feedback = OutputPresentationFeedback::new(output);
    let shown = compositor.outputs.id_of(output);
    let windows = compositor
        .scene
        .windows()
        .iter()
        .filter(|placed| Some(placed.output) == shown)
        .filter_map(|placed| compositor.windows.by_id(placed.id));
    for managed in windows {
        managed.window.with_surfaces(|surface, data| {
            update_surface_primary_scanout_output(
                surface,
                output,
                data,
                states,
                default_primary_scanout_output_compare,
            );
        });
        managed.window.take_presentation_feedback(
            &mut feedback,
            surface_primary_scanout_output,
            |surface, _| surface_presentation_feedback_flags_from_states(surface, states),
        );
    }
    feedback
}

pub(super) fn present(
    feedback: &mut OutputPresentationFeedback,
    metadata: Option<DrmEventMetadata>,
    refresh_interval: Duration,
) {
    let sequence = metadata.map_or(0, |metadata| u64::from(metadata.sequence));
    let refresh = Refresh::fixed(refresh_interval);
    match metadata.map(|metadata| metadata.time) {
        Some(DrmEventTime::Monotonic(stamp)) => feedback.presented::<_, Monotonic>(
            stamp,
            refresh,
            sequence,
            Kind::Vsync | Kind::HwClock | Kind::HwCompletion,
        ),
        Some(DrmEventTime::Realtime(_)) | None => {
            let now = Clock::<Monotonic>::new().now();
            feedback.presented(now, refresh, sequence, Kind::Vsync);
        }
    }
}
