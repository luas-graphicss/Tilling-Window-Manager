use tilling_core::{Effect, Transition};
use tilling_ipc::server::SendError;
use tilling_ipc::{Event, EventFrame, ServerMessage};

use super::peers::PeerId;
use super::{resources, views};
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn publish(&mut self, transition: &Transition) {
        if self.control.subscriber_count() == 0 {
            return;
        }
        let mut events: Vec<Event> = Vec::with_capacity(transition.effects.len());
        for effect in &transition.effects {
            if let Some(event) = self.describe(*effect)
                && !events.contains(&event)
            {
                events.push(event);
            }
        }
        for event in events {
            self.broadcast(transition.revision, event);
        }
    }

    fn describe(&self, effect: Effect) -> Option<Event> {
        let desktop = &self.desktop;
        match effect {
            Effect::OutputAdded(id) | Effect::OutputConfigured(id) => desktop
                .output(id)
                .map(|output| Event::OutputChanged(views::output(id, output))),
            Effect::OutputRemoved(id) => Some(Event::OutputRemoved {
                output: resources::output_id(id),
            }),
            Effect::WorkspaceCreated(id)
            | Effect::WorkspaceActivated(id)
            | Effect::WorkspaceArranged(id)
            | Effect::WorkspaceShown { workspace: id, .. } => desktop
                .workspace(id)
                .map(|workspace| Event::WorkspaceChanged(views::workspace(desktop, id, workspace))),
            Effect::WindowOpened(id) => desktop
                .window(id)
                .map(|window| Event::WindowOpened(views::window(desktop, id, window))),
            Effect::WindowMoved { window: id, .. }
            | Effect::WindowMetadataChanged(id)
            | Effect::WindowModeChanged(id) => desktop
                .window(id)
                .map(|window| Event::WindowChanged(views::window(desktop, id, window))),
            Effect::WindowClosed(id) => Some(Event::WindowClosed {
                window: resources::window_id(id),
            }),
            Effect::FocusChanged { current, .. } => Some(Event::FocusChanged {
                window: current.map(resources::window_id),
            }),
        }
    }

    fn broadcast(&mut self, revision: u64, event: Event) {
        let message = ServerMessage::Event(EventFrame {
            sequence: self.control.advance_sequence(),
            revision,
            event,
        });
        for subscriber in self.control.subscribers() {
            self.notify(subscriber, &message, revision);
        }
    }

    fn notify(&mut self, id: PeerId, message: &ServerMessage, revision: u64) {
        let Some(peer) = self.control.get_mut(id) else {
            return;
        };
        match peer.connection.send(message) {
            Ok(()) => {}
            Err(SendError::EventQueueFull | SendError::Congested) => {
                self.end_subscription(id, revision);
            }
            Err(SendError::Document(error)) => tracing::warn!(%error, "encode control event"),
        }
        self.flush_peer(id);
    }

    fn end_subscription(&mut self, id: PeerId, revision: u64) {
        let sequence = self.control.advance_sequence();
        let Some(peer) = self.control.get_mut(id) else {
            return;
        };
        peer.subscribed = false;
        let notice = ServerMessage::Event(EventFrame {
            sequence,
            revision,
            event: Event::EventsDropped,
        });
        self.reply_to(id, &notice);
    }
}
