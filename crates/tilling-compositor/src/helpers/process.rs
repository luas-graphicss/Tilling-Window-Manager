use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use rustix::io::{FdFlags, fcntl_setfd};
use rustix::process::{Pid, Signal, kill_process};
use tilling_config::CommandLine;

use super::environment::SessionEnvironment;

const EXIT_POLL: Duration = Duration::from_millis(5);

static SPAWNING: Mutex<()> = Mutex::new(());

pub(crate) fn spawn(
    command: &CommandLine,
    environment: &SessionEnvironment,
    connection: Option<UnixStream>,
) -> io::Result<Child> {
    let mut process = Command::new(command.program());
    process.args(command.arguments()).stdin(Stdio::null());
    spawn_command(process, environment, connection)
}

pub(crate) fn spawn_command(
    mut process: Command,
    environment: &SessionEnvironment,
    connection: Option<UnixStream>,
) -> io::Result<Child> {
    environment.apply(&mut process);
    let _exclusive = SPAWNING.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(connection) = connection else {
        return process.spawn();
    };
    fcntl_setfd(&connection, FdFlags::empty())?;
    SessionEnvironment::offer_private_connection(&mut process, connection.as_raw_fd());
    let child = process.spawn();
    drop(connection);
    child
}

pub(crate) fn finish_within(child: &mut Child, limit: Duration) -> io::Result<bool> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if Instant::now() >= deadline {
            end(child);
            return Err(io::ErrorKind::TimedOut.into());
        }
        thread::sleep(EXIT_POLL);
    }
}

pub(crate) fn has_exited(child: &mut Child) -> bool {
    !matches!(child.try_wait(), Ok(None))
}

pub(crate) fn ask_to_stop(child: &Child) {
    if let Err(error) = kill_process(Pid::from_child(child), Signal::TERM) {
        tracing::debug!(%error, "ask a helper to stop");
    }
}

pub(crate) fn end(child: &mut Child) {
    if has_exited(child) {
        return;
    }
    if let Err(error) = child.kill().and_then(|()| child.wait().map(drop)) {
        tracing::warn!(%error, "kill a helper");
    }
}

pub(crate) fn stop_all<'a>(children: impl IntoIterator<Item = &'a mut Child>, grace: Duration) {
    let mut pending: Vec<&mut Child> = children.into_iter().collect();
    pending.retain_mut(|child| !has_exited(child));
    for child in &pending {
        ask_to_stop(child);
    }
    let deadline = Instant::now() + grace;
    while !pending.is_empty() && Instant::now() < deadline {
        thread::sleep(EXIT_POLL);
        pending.retain_mut(|child| !has_exited(child));
    }
    for child in pending {
        end(child);
    }
}
