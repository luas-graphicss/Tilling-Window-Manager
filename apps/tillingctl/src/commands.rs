use std::io::{self, Write};
use std::process::ExitCode;

use tilling_ipc::client::{Client, ClientError};
use tilling_ipc::{Call, Event, Outcome};

use crate::arguments::Arguments;
use crate::output::{self, Format};
use crate::target::{self, Environment, TargetError};

const REFUSED_EXIT_CODE: u8 = 1;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("write to standard output")]
    Output(#[from] io::Error),
    #[error("encode output as JSON")]
    Encode(#[from] serde_json::Error),
    #[error("the compositor dropped events, subscribe again for a fresh snapshot")]
    EventsDropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Done,
    Refused,
}

impl From<Status> for ExitCode {
    fn from(status: Status) -> Self {
        match status {
            Status::Done => Self::SUCCESS,
            Status::Refused => Self::from(REFUSED_EXIT_CODE),
        }
    }
}

pub fn run(arguments: &Arguments, environment: &Environment) -> Result<Status, CommandError> {
    let format = if arguments.json {
        Format::Json
    } else {
        Format::Text
    };
    let mut standard_output = io::stdout().lock();
    let Some(call) = arguments.command.call() else {
        let instances = target::running_instances(environment)?;
        print(
            &mut standard_output,
            &output::instances(&instances, format)?,
        )?;
        return Ok(Status::Done);
    };
    let socket = target::resolve(
        arguments.socket.as_ref(),
        arguments.instance.as_ref(),
        environment,
    )?;
    let mut client = Client::connect(&socket, arguments.timeout())?;
    let follows_events = call == Call::Subscribe;
    let outcome = client.call(call, arguments.if_revision)?;
    let status = report(&mut standard_output, &outcome, format)?;
    if status == Status::Done && follows_events {
        follow(&mut client, &mut standard_output, format)?;
    }
    Ok(status)
}

fn report(
    standard_output: &mut impl Write,
    outcome: &Outcome,
    format: Format,
) -> Result<Status, CommandError> {
    let rendered = output::outcome(outcome, format)?;
    if matches!(outcome, Outcome::Success { .. }) {
        print(standard_output, &rendered)?;
        return Ok(Status::Done);
    }
    match format {
        Format::Json => print(standard_output, &rendered)?,
        Format::Text => eprintln!("tillingctl: {rendered}"),
    }
    Ok(Status::Refused)
}

fn follow(
    client: &mut Client,
    standard_output: &mut impl Write,
    format: Format,
) -> Result<(), CommandError> {
    loop {
        let frame = match client.next_event() {
            Ok(frame) => frame,
            Err(ClientError::Closed) => return Ok(()),
            Err(failure) => return Err(failure.into()),
        };
        print(standard_output, &output::event(&frame, format)?)?;
        if frame.event == Event::EventsDropped {
            return Err(CommandError::EventsDropped);
        }
    }
}

fn print(standard_output: &mut impl Write, rendered: &str) -> io::Result<()> {
    if rendered.is_empty() {
        return Ok(());
    }
    writeln!(standard_output, "{rendered}")
}
