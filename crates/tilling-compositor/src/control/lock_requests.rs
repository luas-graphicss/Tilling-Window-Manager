use std::mem;
use std::time::Instant;

use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use tilling_ipc::{Outcome, Reply, Response, ServerMessage};

use super::peers::PeerId;
use super::refusal::Refusal;
use crate::helpers::LockRequestError;
use crate::runtime::Compositor;

const MAX_AWAITED: usize = 64;

#[derive(Debug)]
struct AwaitedAnswer {
    peer: PeerId,
    request_id: u64,
    deadline: Instant,
}

#[derive(Debug, Default)]
pub(crate) struct LockRequests {
    awaited: Vec<AwaitedAnswer>,
}

impl Compositor {
    pub(super) fn answer_lock_request(
        &mut self,
        peer: PeerId,
        request_id: u64,
    ) -> Result<Option<Reply>, Refusal> {
        self.request_lock()?;
        if self.lock.is_confirmed() {
            return Ok(Some(self.accepted()));
        }
        if self.lock_requests.awaited.len() >= MAX_AWAITED {
            return Err(LockRequestError::Crowded.into());
        }
        let deadline = Instant::now() + self.lock_confirmation;
        self.lock_requests.awaited.push(AwaitedAnswer {
            peer,
            request_id,
            deadline,
        });
        self.expire_lock_requests_later();
        Ok(None)
    }

    pub(crate) fn accept_lock_requests(&mut self) {
        let awaited = mem::take(&mut self.lock_requests.awaited);
        let outcome = Outcome::Success {
            result: self.accepted(),
        };
        self.deliver_lock_outcome(awaited, &outcome);
    }

    pub(crate) fn refuse_lock_requests(&mut self, error: LockRequestError) {
        let awaited = mem::take(&mut self.lock_requests.awaited);
        self.deliver_lock_outcome(awaited, &Refusal::from(error).into());
    }

    fn expire_lock_requests(&mut self, now: Instant) {
        let (expired, awaited): (Vec<_>, Vec<_>) = mem::take(&mut self.lock_requests.awaited)
            .into_iter()
            .partition(|answer| answer.deadline <= now);
        self.lock_requests.awaited = awaited;
        if expired.is_empty() {
            return;
        }
        tracing::warn!(limit = ?self.lock_confirmation, "the locker did not lock the session in time");
        let error = LockRequestError::TimedOut(self.lock_confirmation);
        self.deliver_lock_outcome(expired, &Refusal::from(error).into());
    }

    fn expire_lock_requests_later(&self) {
        let timer = Timer::from_duration(self.lock_confirmation);
        let scheduled = self.loop_handle.insert_source(timer, |_, (), runtime| {
            runtime.compositor.expire_lock_requests(Instant::now());
            TimeoutAction::Drop
        });
        if let Err(error) = scheduled {
            tracing::warn!(error = %error.error, "schedule the limit of a lock request");
        }
    }

    fn deliver_lock_outcome(&mut self, awaited: Vec<AwaitedAnswer>, outcome: &Outcome) {
        for answer in awaited {
            let response = Response {
                request_id: answer.request_id,
                outcome: outcome.clone(),
            };
            self.reply_to(answer.peer, &ServerMessage::Response(response));
            self.flush_peer(answer.peer);
        }
    }
}
