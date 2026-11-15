use std::io::{self, Write};
use std::process::ExitCode;

use tilling_ipc::client::{Client, ClientError};
use tilling_ipc::{Call, Event, Outcome, Reply, ResourceId};

use crate::arguments::{Arguments, Operation, WorkspaceReference};
use crate::completions;
use crate::output::{self, Format};
use crate::target::{self, Environment, TargetError};
use crate::topics::TopicFilter;

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
    #[error("the compositor did not list its workspaces when asked")]
    WorkspacesNotListed,
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
    match arguments.command.operation() {
        Operation::ListInstances => {
            let instances = target::running_instances(environment)?;
            let listed = output::instances(&instances, format)?;
            print(&mut standard_output, &listed)?;
            Ok(Status::Done)
        }
        Operation::PrintCompletions(shell) => {
            completions::print(shell, &mut standard_output)?;
            Ok(Status::Done)
        }
        Operation::Call(call) => {
            let mut client = connect(arguments, environment)?;
            let outcome = client.call(call, arguments.if_revision)?;
            report(&mut standard_output, &outcome, format)
        }
        Operation::CallOnWorkspace(reference, call) => {
            let mut client = connect(arguments, environment)?;
            let outcome = match find_workspace(&mut client, &reference)? {
                Ok(workspace) => client.call(call.on(workspace), arguments.if_revision)?,
                Err(refusal) => refusal,
            };
            report(&mut standard_output, &outcome, format)
        }
        Operation::FollowEvents(topics) => {
            let mut client = connect(arguments, environment)?;
            let subscribed = client.call(Call::Subscribe, arguments.if_revision)?;
            let status = report(&mut standard_output, &topics.narrow(subscribed), format)?;
            if status == Status::Done {
                follow(&mut client, &mut standard_output, format, &topics)?;
            }
            Ok(status)
        }
    }
}

fn connect(arguments: &Arguments, environment: &Environment) -> Result<Client, CommandError> {
    let socket = target::resolve(
        arguments.socket.as_ref(),
        arguments.instance.as_ref(),
        environment,
    )?;
    Ok(Client::connect(&socket, arguments.timeout())?)
}

fn find_workspace(
    client: &mut Client,
    reference: &WorkspaceReference,
) -> Result<Result<ResourceId, Outcome>, CommandError> {
    match client.call(Call::GetWorkspaces, None)? {
        Outcome::Success {
            result: Reply::Workspaces(workspaces),
        } => Ok(reference
            .find(&workspaces)
            .ok_or_else(|| Outcome::NotFound {
                message: reference.missing(),
            })),
        Outcome::Success { .. } => Err(CommandError::WorkspacesNotListed),
        refusal => Ok(Err(refusal)),
    }
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
    topics: &TopicFilter,
) -> Result<(), CommandError> {
    loop {
        let frame = match client.next_event() {
            Ok(frame) => frame,
            Err(ClientError::Closed) => return Ok(()),
            Err(failure) => return Err(failure.into()),
        };
        if !topics.admits(&frame.event) {
            continue;
        }
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
