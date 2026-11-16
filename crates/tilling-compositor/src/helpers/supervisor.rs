use std::mem;
use std::process::Child;
use std::time::{Duration, Instant};

use tilling_config::{Helper, RestartPolicy};

use super::environment::SessionEnvironment;
use super::process;
use super::restart::RestartBudget;
use super::services::SessionServices;
use crate::error::cause_chain;

pub(crate) const STOP_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessId(u64);

#[derive(Debug)]
enum Stage {
    Running(ProcessId, Child),
    AwaitingRestart,
    Settled,
}

#[derive(Debug)]
struct Supervised {
    definition: Helper,
    stage: Stage,
    budget: RestartBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockerDuty {
    Idle,
    Requested,
    Guarding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockerReview {
    Settled,
    EndedBeforeLocking,
    MustReturn,
}

#[derive(Debug)]
pub(crate) struct Supervisor {
    environment: SessionEnvironment,
    restart_delay: Duration,
    helpers: Vec<Supervised>,
    leaving: Vec<(ProcessId, Child)>,
    locker: Option<Child>,
    locker_duty: LockerDuty,
    locker_budget: RestartBudget,
    announced: Option<SessionServices>,
    started: u64,
}

impl Supervisor {
    pub(crate) fn new(environment: SessionEnvironment, restart_delay: Duration) -> Self {
        Self {
            environment,
            restart_delay,
            helpers: Vec::new(),
            leaving: Vec::new(),
            locker: None,
            locker_duty: LockerDuty::Idle,
            locker_budget: RestartBudget::default(),
            announced: None,
            started: 0,
        }
    }

    pub(crate) fn announce(&mut self, services: SessionServices) {
        match services.announce(&self.environment) {
            Ok(()) => self.announced = Some(services),
            Err(error) => {
                tracing::warn!(error = %cause_chain(&error), "announce the session to its services");
            }
        }
    }

    pub(crate) const fn environment(&self) -> &SessionEnvironment {
        &self.environment
    }

    pub(crate) const fn restart_delay(&self) -> Duration {
        self.restart_delay
    }

    pub(crate) fn running(&self) -> usize {
        self.helpers
            .iter()
            .filter(|helper| matches!(helper.stage, Stage::Running(..)))
            .count()
    }

    pub(crate) fn missing(&self, wanted: &[Helper]) -> Vec<Helper> {
        wanted
            .iter()
            .filter(|definition| !self.knows(definition))
            .cloned()
            .collect()
    }

    pub(crate) fn retire_all_but(&mut self, wanted: &[Helper]) -> bool {
        let (kept, retired): (Vec<_>, Vec<_>) = mem::take(&mut self.helpers)
            .into_iter()
            .partition(|helper| wanted.contains(&helper.definition));
        self.helpers = kept;
        let already_leaving = self.leaving.len();
        for helper in retired {
            if let Stage::Running(id, child) = helper.stage {
                process::ask_to_stop(&child);
                self.leaving.push((id, child));
            }
        }
        self.leaving.len() > already_leaving
    }

    pub(crate) fn end_lingering(&mut self) {
        for (_, mut child) in self.leaving.drain(..) {
            process::end(&mut child);
        }
    }

    pub(crate) fn next_id(&mut self) -> ProcessId {
        self.started = self.started.saturating_add(1);
        ProcessId(self.started)
    }

    pub(crate) fn admit(&mut self, id: ProcessId, definition: &Helper, child: Child) {
        self.place(definition, Stage::Running(id, child));
    }

    pub(crate) fn give_up(&mut self, definition: &Helper) {
        self.place(definition, Stage::Settled);
    }

    pub(crate) fn departed(&mut self, id: ProcessId, now: Instant) -> Option<Helper> {
        if let Some(index) = self.leaving.iter().position(|(left, _)| *left == id) {
            let (_, mut child) = self.leaving.swap_remove(index);
            process::end(&mut child);
            return None;
        }
        let helper = self
            .helpers
            .iter_mut()
            .find(|helper| matches!(&helper.stage, Stage::Running(running, _) if *running == id))?;
        let Stage::Running(_, child) = &mut helper.stage else {
            return None;
        };
        let succeeded = match child.try_wait() {
            Ok(Some(status)) => status.success(),
            Ok(None) => return None,
            Err(_) => false,
        };
        let restarts = !succeeded
            && helper.definition.restart == RestartPolicy::OnFailure
            && helper.budget.spend(now);
        if restarts {
            tracing::warn!(helper = %helper.definition.name, "helper failed and starts again");
            helper.stage = Stage::AwaitingRestart;
            return Some(helper.definition.clone());
        }
        if !succeeded {
            tracing::warn!(helper = %helper.definition.name, "helper failed and stays stopped");
        }
        helper.stage = Stage::Settled;
        None
    }

    pub(crate) fn awaits_restart(&self, definition: &Helper) -> bool {
        self.helpers.iter().any(|helper| {
            helper.definition == *definition && matches!(helper.stage, Stage::AwaitingRestart)
        })
    }

    pub(crate) fn locker_is_running(&mut self) -> bool {
        let running = self
            .locker
            .as_mut()
            .is_some_and(|child| !process::has_exited(child));
        if !running {
            self.locker = None;
        }
        running
    }

    pub(crate) fn keep_locker(&mut self, child: Child) {
        self.locker = Some(child);
        if self.locker_duty == LockerDuty::Idle {
            self.locker_duty = LockerDuty::Requested;
        }
    }

    pub(crate) fn review_locker(
        &mut self,
        engaged: bool,
        held: bool,
        now: Instant,
    ) -> LockerReview {
        let running = self.locker_is_running();
        let duty_before = self.locker_duty;
        self.locker_duty = match (duty_before, engaged) {
            (LockerDuty::Requested | LockerDuty::Guarding, true) => LockerDuty::Guarding,
            (LockerDuty::Requested, false) if running => LockerDuty::Requested,
            (LockerDuty::Idle, _) | (LockerDuty::Requested | LockerDuty::Guarding, false) => {
                LockerDuty::Idle
            }
        };
        if duty_before == LockerDuty::Requested && self.locker_duty == LockerDuty::Idle {
            return LockerReview::EndedBeforeLocking;
        }
        let abandoned = self.locker_duty == LockerDuty::Guarding && !held && !running;
        if abandoned && self.locker_budget.spend(now) {
            LockerReview::MustReturn
        } else {
            LockerReview::Settled
        }
    }

    fn knows(&self, definition: &Helper) -> bool {
        self.helpers
            .iter()
            .any(|helper| helper.definition == *definition)
    }

    fn place(&mut self, definition: &Helper, stage: Stage) {
        let known = self
            .helpers
            .iter_mut()
            .find(|helper| helper.definition == *definition);
        match known {
            Some(helper) => helper.stage = stage,
            None => self.helpers.push(Supervised {
                definition: definition.clone(),
                stage,
                budget: RestartBudget::default(),
            }),
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        if let Some(services) = self.announced.take()
            && let Err(error) = services.withdraw(&self.environment)
        {
            tracing::warn!(error = %cause_chain(&error), "withdraw the session from its services");
        }
        let helpers = self
            .helpers
            .iter_mut()
            .filter_map(|helper| match &mut helper.stage {
                Stage::Running(_, child) => Some(child),
                Stage::AwaitingRestart | Stage::Settled => None,
            });
        let leaving = self.leaving.iter_mut().map(|(_, child)| child);
        process::stop_all(
            helpers.chain(leaving).chain(self.locker.as_mut()),
            STOP_GRACE,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;
    use std::thread;

    use super::*;

    const POLL: Duration = Duration::from_millis(2);

    fn supervisor() -> Supervisor {
        let directory = Path::new("/nonexistent");
        let environment = SessionEnvironment::new(directory, directory, directory);
        Supervisor::new(environment, Duration::ZERO)
    }

    fn finished_program() -> Child {
        let mut child = Command::new("true").spawn().unwrap();
        while !process::has_exited(&mut child) {
            thread::sleep(POLL);
        }
        child
    }

    #[test]
    fn a_lock_held_by_someone_else_is_not_guarded() {
        let mut supervisor = supervisor();

        let review = supervisor.review_locker(true, false, Instant::now());

        assert_eq!(review, LockerReview::Settled);
    }

    #[test]
    fn a_requested_locker_that_ends_before_the_session_locks_is_reported_once() {
        let mut supervisor = supervisor();
        let now = Instant::now();
        supervisor.keep_locker(finished_program());

        let ended = supervisor.review_locker(false, false, now);
        let afterwards = supervisor.review_locker(false, false, now);

        assert_eq!(ended, LockerReview::EndedBeforeLocking);
        assert_eq!(afterwards, LockerReview::Settled);
    }

    #[test]
    fn a_requested_locker_that_dies_while_locked_must_return_a_few_times() {
        let mut supervisor = supervisor();
        let now = Instant::now();
        supervisor.keep_locker(finished_program());

        let while_held = supervisor.review_locker(true, true, now);
        let abandoned: Vec<LockerReview> = (0..4)
            .map(|_| supervisor.review_locker(true, false, now))
            .collect();

        assert_eq!(while_held, LockerReview::Settled);
        assert_eq!(
            abandoned,
            [
                LockerReview::MustReturn,
                LockerReview::MustReturn,
                LockerReview::MustReturn,
                LockerReview::Settled
            ]
        );
    }

    #[test]
    fn unlocking_ends_the_guard() {
        let mut supervisor = supervisor();
        let now = Instant::now();
        supervisor.keep_locker(finished_program());
        supervisor.review_locker(true, true, now);

        let after_unlock = supervisor.review_locker(false, false, now);
        let locked_by_someone_else = supervisor.review_locker(true, false, now);

        assert_eq!(after_unlock, LockerReview::Settled);
        assert_eq!(locked_by_someone_else, LockerReview::Settled);
    }
}
