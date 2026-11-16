use std::time::Duration;

use smithay::reexports::calloop::RegistrationToken;
use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::ext_idle_notification_v1::ExtIdleNotificationV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::IsAlive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InhibitorPolicy {
    Obeyed,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Idle,
    Waiting(Duration),
    Held,
}

#[derive(Debug)]
struct Watch {
    notification: ExtIdleNotificationV1,
    timeout: Duration,
    policy: InhibitorPolicy,
    idle: bool,
}

#[derive(Debug, Default)]
pub(crate) struct IdleTracker {
    last_activity: Duration,
    watches: Vec<Watch>,
    inhibitors: Vec<WlSurface>,
    timer: Option<RegistrationToken>,
}

impl IdleTracker {
    pub(crate) fn watch(
        &mut self,
        notification: ExtIdleNotificationV1,
        timeout: Duration,
        policy: InhibitorPolicy,
    ) {
        self.watches.push(Watch {
            notification,
            timeout,
            policy,
            idle: false,
        });
    }

    pub(crate) fn forget(&mut self, notification: &ExtIdleNotificationV1) {
        self.watches
            .retain(|watch| watch.notification != *notification);
    }

    pub(crate) fn inhibit(&mut self, surface: WlSurface) {
        self.inhibitors.push(surface);
    }

    pub(crate) fn uninhibit(&mut self, surface: &WlSurface, now: Duration) {
        self.inhibitors.retain(|inhibitor| inhibitor != surface);
        self.last_activity = now;
    }

    pub(crate) fn inhibitors(&self) -> impl Iterator<Item = &WlSurface> {
        self.inhibitors.iter().filter(|surface| surface.alive())
    }

    pub(crate) fn resume(&mut self, now: Duration) {
        self.last_activity = now;
        for watch in &mut self.watches {
            if std::mem::take(&mut watch.idle) {
                watch.notification.resumed();
            }
        }
    }

    pub(crate) fn advance(&mut self, now: Duration, inhibited: bool) -> Option<Duration> {
        let quiet = now.saturating_sub(self.last_activity);
        let mut next_check = None;
        for watch in self.watches.iter_mut().filter(|watch| !watch.idle) {
            match verdict(quiet, watch.timeout, watch.policy, inhibited) {
                Verdict::Idle => {
                    watch.idle = true;
                    watch.notification.idled();
                }
                Verdict::Waiting(remaining) => {
                    next_check = Some(
                        next_check.map_or(remaining, |soonest: Duration| soonest.min(remaining)),
                    );
                }
                Verdict::Held => next_check = next_check.or(Some(watch.timeout)),
            }
        }
        next_check
    }

    pub(crate) const fn is_timed(&self) -> bool {
        self.timer.is_some()
    }

    pub(crate) const fn take_timer(&mut self) -> Option<RegistrationToken> {
        self.timer.take()
    }

    pub(crate) const fn set_timer(&mut self, timer: RegistrationToken) {
        self.timer = Some(timer);
    }
}

fn verdict(
    quiet: Duration,
    timeout: Duration,
    policy: InhibitorPolicy,
    inhibited: bool,
) -> Verdict {
    if inhibited && policy == InhibitorPolicy::Obeyed {
        return Verdict::Held;
    }
    match timeout.checked_sub(quiet) {
        Some(remaining) if !remaining.is_zero() => Verdict::Waiting(remaining),
        _ => Verdict::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(300);

    #[test]
    fn a_watch_waits_for_the_rest_of_its_timeout() {
        let quiet = Duration::from_secs(120);

        let outcome = verdict(quiet, TIMEOUT, InhibitorPolicy::Obeyed, false);

        assert_eq!(outcome, Verdict::Waiting(Duration::from_secs(180)));
    }

    #[test]
    fn a_watch_turns_idle_once_its_timeout_has_passed() {
        let exactly = verdict(TIMEOUT, TIMEOUT, InhibitorPolicy::Obeyed, false);
        let later = verdict(TIMEOUT * 2, TIMEOUT, InhibitorPolicy::Obeyed, false);

        assert_eq!((exactly, later), (Verdict::Idle, Verdict::Idle));
    }

    #[test]
    fn an_inhibitor_holds_only_the_watches_that_obey_it() {
        let obeying = verdict(TIMEOUT * 2, TIMEOUT, InhibitorPolicy::Obeyed, true);
        let ignoring = verdict(TIMEOUT * 2, TIMEOUT, InhibitorPolicy::Ignored, true);

        assert_eq!((obeying, ignoring), (Verdict::Held, Verdict::Idle));
    }
}
