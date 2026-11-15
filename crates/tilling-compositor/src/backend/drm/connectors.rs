use std::collections::HashSet;
use std::io;

use smithay::backend::drm::DrmDevice;
use smithay::reexports::drm::control::{Device as _, connector, crtc};

pub(super) fn connected(drm: &DrmDevice) -> io::Result<Vec<connector::Info>> {
    let resources = drm.resource_handles()?;
    let usable = resources
        .connectors()
        .iter()
        .filter_map(|handle| drm.get_connector(*handle, true).ok())
        .filter(|info| info.state() == connector::State::Connected && !info.modes().is_empty())
        .collect();
    Ok(usable)
}

pub(super) fn free_crtc(
    drm: &DrmDevice,
    connector: &connector::Info,
    taken: &HashSet<crtc::Handle>,
) -> Option<crtc::Handle> {
    let resources = drm.resource_handles().ok()?;
    let current = connector
        .current_encoder()
        .and_then(|encoder| drm.get_encoder(encoder).ok())
        .and_then(|encoder| encoder.crtc());
    let possible = connector
        .encoders()
        .iter()
        .filter_map(|encoder| drm.get_encoder(*encoder).ok())
        .flat_map(|encoder| resources.filter_crtcs(encoder.possible_crtcs()));
    current
        .into_iter()
        .chain(possible)
        .find(|candidate| !taken.contains(candidate))
}
