use std::path::Path;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use smithay::reexports::calloop::EventLoop;
use smithay::reexports::calloop::channel::{self, Event, Sender};
use tilling_core::WindowId;

use super::Runtime;
use super::instance::Instance;
use crate::error::CompositorError;
use crate::input::SyntheticInput;
use crate::options::Options;
use crate::render::FrameStats;
use crate::scene::Scene;

const THREAD_NAME: &str = "tilling-compositor";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub revision: u64,
    pub scene: Scene,
    pub focused: Option<WindowId>,
    pub toplevels: usize,
    pub mapped_windows: usize,
    pub popups: usize,
    pub transaction_pending: bool,
    pub keyboard_mode: String,
    pub control_peers: usize,
    pub control_subscribers: usize,
    pub frames: FrameStats,
}

#[derive(Debug)]
pub struct Session {
    instance: Instance,
    requests: Sender<Request>,
    thread: Option<JoinHandle<Result<(), CompositorError>>>,
}

#[derive(Debug)]
enum Request {
    Snapshot(mpsc::Sender<SessionSnapshot>),
    Inject(SyntheticInput),
    Stop,
}

type Startup = Result<(Instance, Sender<Request>), CompositorError>;

impl Session {
    pub fn start(options: Options) -> Result<Self, CompositorError> {
        let (announce, startup) = mpsc::channel::<Startup>();
        let thread = thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || serve(options, &announce))
            .map_err(CompositorError::SessionThread)?;
        let (instance, requests) = startup
            .recv()
            .map_err(|_| CompositorError::SessionClosed)??;
        Ok(Self {
            instance,
            requests,
            thread: Some(thread),
        })
    }

    #[must_use]
    pub fn wayland_socket(&self) -> &Path {
        self.instance.wayland_socket()
    }

    #[must_use]
    pub fn control_socket(&self) -> &Path {
        self.instance.control_socket()
    }

    pub fn snapshot(&self) -> Result<SessionSnapshot, CompositorError> {
        let (reply, snapshot) = mpsc::channel();
        self.requests
            .send(Request::Snapshot(reply))
            .map_err(|_| CompositorError::SessionClosed)?;
        snapshot.recv().map_err(|_| CompositorError::SessionClosed)
    }

    pub fn inject(&self, input: SyntheticInput) -> Result<(), CompositorError> {
        self.requests
            .send(Request::Inject(input))
            .map_err(|_| CompositorError::SessionClosed)
    }

    pub fn stop(mut self) -> Result<(), CompositorError> {
        self.shut_down()
    }

    fn shut_down(&mut self) -> Result<(), CompositorError> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        let stop_requested = self.requests.send(Request::Stop).is_ok();
        let outcome = thread.join().map_err(|_| CompositorError::SessionClosed)?;
        if stop_requested {
            outcome
        } else {
            outcome.and(Err(CompositorError::SessionClosed))
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Err(error) = self.shut_down() {
            tracing::warn!(%error, "stop compositor session");
        }
    }
}

fn serve(options: Options, announce: &mpsc::Sender<Startup>) -> Result<(), CompositorError> {
    let (mut event_loop, mut runtime, startup) = match prepare(options) {
        Ok(prepared) => prepared,
        Err(error) => {
            return announce
                .send(Err(error))
                .map_err(|_| CompositorError::SessionClosed);
        }
    };
    announce
        .send(Ok(startup))
        .map_err(|_| CompositorError::SessionClosed)?;
    runtime.serve(&mut event_loop)
}

type Prepared = (
    EventLoop<'static, Runtime>,
    Runtime,
    (Instance, Sender<Request>),
);

fn prepare(options: Options) -> Result<Prepared, CompositorError> {
    let event_loop = EventLoop::try_new().map_err(CompositorError::EventLoop)?;
    let (runtime, instance) = Runtime::launch(&event_loop, options)?;
    let (requests, incoming) = channel::channel();
    event_loop
        .handle()
        .insert_source(incoming, |event, (), runtime: &mut Runtime| match event {
            Event::Msg(Request::Snapshot(reply)) => {
                if reply.send(runtime.compositor.snapshot()).is_err() {
                    tracing::debug!("snapshot requester went away");
                }
            }
            Event::Msg(Request::Inject(input)) => runtime.compositor.inject(input),
            Event::Msg(Request::Stop) | Event::Closed => runtime.compositor.stop(),
        })
        .map_err(|error| CompositorError::EventSource(error.error))?;
    Ok((event_loop, runtime, (instance, requests)))
}
