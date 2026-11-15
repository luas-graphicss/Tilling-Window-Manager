use clap::ValueEnum;
use tilling_ipc::{Event, Outcome, Reply, Snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Topic {
    Windows,
    Workspaces,
    Outputs,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TopicFilter {
    topics: Vec<Topic>,
}

impl TopicFilter {
    pub fn new(topics: &[Topic]) -> Self {
        Self {
            topics: topics.to_vec(),
        }
    }

    pub fn admits(&self, event: &Event) -> bool {
        match event {
            Event::WindowOpened(_)
            | Event::WindowChanged(_)
            | Event::WindowClosed { .. }
            | Event::FocusChanged { .. } => self.selects(Topic::Windows),
            Event::WorkspaceChanged(_) => self.selects(Topic::Workspaces),
            Event::OutputChanged(_) | Event::OutputRemoved { .. } => self.selects(Topic::Outputs),
            Event::EventsDropped => true,
        }
    }

    pub fn narrow(&self, outcome: Outcome) -> Outcome {
        match outcome {
            Outcome::Success {
                result: Reply::Subscribed(snapshot),
            } => Outcome::Success {
                result: Reply::Subscribed(self.narrow_snapshot(snapshot)),
            },
            other => other,
        }
    }

    fn narrow_snapshot(&self, snapshot: Snapshot) -> Snapshot {
        Snapshot {
            outputs: self.kept(Topic::Outputs, snapshot.outputs),
            workspaces: self.kept(Topic::Workspaces, snapshot.workspaces),
            windows: self.kept(Topic::Windows, snapshot.windows),
            ..snapshot
        }
    }

    fn kept<T>(&self, topic: Topic, entries: Vec<T>) -> Vec<T> {
        if self.selects(topic) {
            entries
        } else {
            Vec::new()
        }
    }

    fn selects(&self, topic: Topic) -> bool {
        self.topics.is_empty() || self.topics.contains(&topic)
    }
}

#[cfg(test)]
mod tests {
    use tilling_ipc::{OutputInfo, ResourceId};

    use super::*;

    const OUTPUT: ResourceId = ResourceId {
        index: 1,
        generation: 0,
    };

    fn output() -> OutputInfo {
        OutputInfo {
            id: OUTPUT,
            name: "eDP-1".to_owned(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale: 1.0,
            workspace: OUTPUT,
        }
    }

    #[test]
    fn no_topics_admit_every_event() {
        let filter = TopicFilter::default();

        assert!(filter.admits(&Event::FocusChanged { window: None }));
        assert!(filter.admits(&Event::OutputRemoved { output: OUTPUT }));
    }

    #[test]
    fn chosen_topics_leave_the_others_out() {
        let filter = TopicFilter::new(&[Topic::Outputs]);

        assert!(filter.admits(&Event::OutputChanged(output())));
        assert!(!filter.admits(&Event::FocusChanged { window: None }));
        assert!(!filter.admits(&Event::WindowClosed { window: OUTPUT }));
    }

    #[test]
    fn lost_events_are_reported_under_every_topic() {
        let filter = TopicFilter::new(&[Topic::Workspaces]);

        assert!(filter.admits(&Event::EventsDropped));
    }

    #[test]
    fn snapshot_keeps_only_the_chosen_topics() {
        let subscribed = Outcome::Success {
            result: Reply::Subscribed(Snapshot {
                sequence: 4,
                revision: 9,
                outputs: vec![output()],
                workspaces: Vec::new(),
                windows: Vec::new(),
            }),
        };
        let refusal = Outcome::NotPermitted {
            message: "events were not negotiated".to_owned(),
        };

        let windows_only = TopicFilter::new(&[Topic::Windows]).narrow(subscribed.clone());

        assert_eq!(
            windows_only,
            Outcome::Success {
                result: Reply::Subscribed(Snapshot {
                    sequence: 4,
                    revision: 9,
                    outputs: Vec::new(),
                    workspaces: Vec::new(),
                    windows: Vec::new(),
                }),
            }
        );
        assert_eq!(
            TopicFilter::new(&[Topic::Outputs]).narrow(subscribed.clone()),
            subscribed
        );
        assert_eq!(TopicFilter::default().narrow(refusal.clone()), refusal);
    }
}
