use smithay::output::Output;
use smithay::reexports::wayland_protocols::ext::session_lock::v1::server::ext_session_lock_v1::ExtSessionLockV1;
use smithay::reexports::wayland_server::Resource;
use smithay::wayland::session_lock::{LockSurface, SessionLocker};

#[derive(Debug, Default)]
enum Stage {
    #[default]
    Unlocked,
    Curtaining(Vec<Output>),
    Locked,
}

#[derive(Debug, Default)]
pub(crate) struct SessionLock {
    stage: Stage,
    holder: Option<ExtSessionLockV1>,
    unconfirmed: Option<SessionLocker>,
    surfaces: Vec<(Output, LockSurface)>,
}

impl SessionLock {
    pub(crate) const fn is_engaged(&self) -> bool {
        !matches!(self.stage, Stage::Unlocked)
    }

    pub(crate) fn is_held(&self) -> bool {
        self.holder.as_ref().is_some_and(Resource::is_alive)
    }

    pub(crate) fn is_confirmed(&self) -> bool {
        self.is_held() && self.unconfirmed.is_none()
    }

    pub(crate) fn engage(&mut self, uncovered: Vec<Output>) {
        if !self.is_engaged() {
            self.stage = Stage::Curtaining(uncovered);
        }
        self.confirm_when_covered();
    }

    pub(crate) fn hold(&mut self, confirmation: SessionLocker) {
        self.holder = Some(confirmation.ext_session_lock().clone());
        self.unconfirmed = Some(confirmation);
        self.surfaces.clear();
        self.confirm_when_covered();
    }

    pub(crate) fn output_covered(&mut self, output: &Output) {
        if let Stage::Curtaining(uncovered) = &mut self.stage {
            uncovered.retain(|pending| pending != output);
        }
        self.confirm_when_covered();
    }

    pub(crate) fn release(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn show(&mut self, output: Output, surface: LockSurface) {
        self.surfaces.retain(|(_, shown)| shown.alive());
        self.surfaces.push((output, surface));
    }

    pub(crate) fn surface_on(&self, output: &Output) -> Option<&LockSurface> {
        self.surfaces()
            .find(|(shown_on, _)| *shown_on == output)
            .map(|(_, surface)| surface)
    }

    pub(crate) fn surfaces(&self) -> impl Iterator<Item = (&Output, &LockSurface)> {
        self.surfaces
            .iter()
            .filter(|(_, surface)| surface.alive())
            .map(|(output, surface)| (output, surface))
    }

    fn confirm_when_covered(&mut self) {
        if let Stage::Curtaining(uncovered) = &self.stage
            && uncovered.is_empty()
        {
            self.stage = Stage::Locked;
        }
        if matches!(self.stage, Stage::Locked)
            && let Some(confirmation) = self.unconfirmed.take()
        {
            confirmation.lock();
        }
    }
}
