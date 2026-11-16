use smithay::input::pointer::PointerHandle;
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::RegionAttributes;
use smithay::wayland::pointer_constraints::{PointerConstraint, with_pointer_constraint};

use crate::runtime::Compositor;
use crate::shell::SurfaceHit;

#[derive(Debug, Clone)]
pub(super) enum Constraint {
    Locked,
    Confined(Option<RegionAttributes>),
}

impl Compositor {
    pub(super) fn held_constraint(
        &self,
        pointer: &PointerHandle<Self>,
        focus: Option<&SurfaceHit>,
        location: Point<f64, Logical>,
    ) -> Option<Constraint> {
        let (surface, origin) = focus?;
        let within = (location - *origin).to_i32_round();
        with_pointer_constraint(surface, pointer, |constraint| {
            let constraint = constraint.filter(|constraint| constraint.is_active())?;
            if !applies_at(constraint.region(), within) {
                return None;
            }
            Some(match &*constraint {
                PointerConstraint::Locked(_) => Constraint::Locked,
                PointerConstraint::Confined(confined) => {
                    Constraint::Confined(confined.region().cloned())
                }
            })
        })
    }

    pub(super) fn constraint_allows(
        &self,
        constraint: Option<&Constraint>,
        focus: Option<&SurfaceHit>,
        target: Point<f64, Logical>,
    ) -> bool {
        match (constraint, focus) {
            (None, _) | (Some(_), None) => true,
            (Some(Constraint::Locked), Some(_)) => false,
            (Some(Constraint::Confined(region)), Some((surface, origin))) => {
                let stays = self
                    .surface_under(target)
                    .is_some_and(|(under, _)| under == *surface);
                stays && applies_at(region.as_ref(), (target - *origin).to_i32_round())
            }
        }
    }

    pub(crate) fn engage_pointer_constraint(&self) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let location = pointer.current_location();
        let Some((surface, origin)) = self.surface_under(location) else {
            return;
        };
        if !self.owns_keyboard_focus(&surface) {
            return;
        }
        let within = (location - origin).to_i32_round();
        with_pointer_constraint(&surface, &pointer, |constraint| {
            let waiting = constraint.filter(|constraint| !constraint.is_active());
            if let Some(constraint) = waiting
                && applies_at(constraint.region(), within)
            {
                constraint.activate();
            }
        });
    }

    pub(crate) fn review_pointer_constraint(&mut self) {
        if !std::mem::take(&mut self.constraint_review_due) {
            return;
        }
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(constrained) = pointer.current_focus() else {
            return;
        };
        if self.owns_keyboard_focus(&constrained) {
            return;
        }
        with_pointer_constraint(&constrained, &pointer, |constraint| {
            if let Some(constraint) = constraint.filter(|constraint| constraint.is_active()) {
                constraint.deactivate();
            }
        });
    }
}

fn applies_at(region: Option<&RegionAttributes>, point: Point<i32, Logical>) -> bool {
    region.is_none_or(|region| region.contains(point))
}
