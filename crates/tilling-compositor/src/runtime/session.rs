use std::io;
use std::os::unix::net::UnixStream;
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
use crate::options::{BackendKind, Options, VirtualOutput};
use crate::render::FrameStats;
use crate::scene::Scene;
use crate::security::Capability;

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
    pub layers: usize,
    pub locked: bool,
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
    PlugOutput(VirtualOutput, mpsc::Sender<TopologyChange>),
    UnplugOutput(String, mpsc::Sender<TopologyChange>),
    ConnectPrivileged(Vec<Capability>, mpsc::Sender<io::Result<UnixStream>>),
    Stop,
}

type TopologyChange = Result<(), CompositorError>;

type Startup = Result<(Instance, Sender<Request>), CompositorError>;

impl Session {
    pub fn start(options: Options) -> Result<Self, CompositorError> {
        if options.backend == BackendKind::Nested {
            return Err(CompositorError::MainThreadBackend(options.backend.name()));
        }
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

    pub fn plug_output(&self, output: VirtualOutput) -> Result<(), CompositorError> {
        self.change_topology(|reply| Request::PlugOutput(output, reply))
    }

    pub fn unplug_output(&self, name: &str) -> Result<(), CompositorError> {
        self.change_topology(|reply| Request::UnplugOutput(name.to_owned(), reply))
    }

    pub fn connect_privileged(
        &self,
        capabilities: &[Capability],
    ) -> Result<UnixStream, CompositorError> {
        let (reply, connection) = mpsc::channel();
        self.requests
            .send(Request::ConnectPrivileged(capabilities.to_vec(), reply))
            .map_err(|_| CompositorError::SessionClosed)?;
        connection
            .recv()
            .map_err(|_| CompositorError::SessionClosed)?
            .map_err(CompositorError::PrivilegedClient)
    }

    pub fn stop(mut self) -> Result<(), CompositorError> {
        self.shut_down()
    }

    fn change_topology(
        &self,
        request: impl FnOnce(mpsc::Sender<TopologyChange>) -> Request,
    ) -> Result<(), CompositorError> {
        let (reply, outcome) = mpsc::channel();
        self.requests
            .send(request(reply))
            .map_err(|_| CompositorError::SessionClosed)?;
        outcome.recv().map_err(|_| CompositorError::SessionClosed)?
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

fn report_topology_change(reply: &mpsc::Sender<TopologyChange>, outcome: TopologyChange) {
    if reply.send(outcome).is_err() {
        tracing::debug!("output requester went away");
    }
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
            Event::Msg(Request::PlugOutput(output, reply)) => {
                let outcome = runtime
                    .backend
                    .plug_virtual_output(&output, &mut runtime.compositor);
                report_topology_change(&reply, outcome);
            }
            Event::Msg(Request::UnplugOutput(name, reply)) => {
                let outcome = runtime
                    .backend
                    .unplug_virtual_output(&name, &mut runtime.compositor);
                report_topology_change(&reply, outcome);
            }
            Event::Msg(Request::ConnectPrivileged(capabilities, reply)) => {
                let privileges = capabilities.into_iter().collect();
                let connection = runtime.compositor.connect_privileged(privileges);
                if reply.send(connection).is_err() {
                    tracing::debug!("privileged client went away");
                }
            }
            Event::Msg(Request::Stop) | Event::Closed => runtime.compositor.stop(),
        })
        .map_err(|error| CompositorError::EventSource(error.error))?;
    Ok((event_loop, runtime, (instance, requests)))
}
