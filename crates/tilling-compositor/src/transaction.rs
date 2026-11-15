use std::collections::BTreeMap;
use std::time::Duration;

use tilling_core::WindowId;

use crate::scene::Scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigureSerial(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Participant {
    AwaitingAck(ConfigureSerial),
    AwaitingCommit,
}

#[derive(Debug)]
struct Pending {
    scene: Scene,
    deadline: Duration,
    participants: BTreeMap<WindowId, Participant>,
}

#[derive(Debug)]
pub(crate) struct Coordinator {
    timeout: Duration,
    pending: Option<Pending>,
}

impl ConfigureSerial {
    fn is_no_older_than(self, other: Self) -> bool {
        self.0.wrapping_sub(other.0) < u32::MAX / 2
    }
}

impl Coordinator {
    pub(crate) const fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            pending: None,
        }
    }

    pub(crate) fn begin(
        &mut self,
        scene: Scene,
        configured: impl IntoIterator<Item = (WindowId, ConfigureSerial)>,
        now: Duration,
    ) -> Option<Duration> {
        let superseded = self.pending.take();
        let mut participants: BTreeMap<WindowId, Participant> = superseded
            .iter()
            .flat_map(|pending| pending.participants.iter())
            .filter(|(window, _)| scene.window(**window).is_some())
            .map(|(window, participant)| (*window, *participant))
            .collect();
        for (window, serial) in configured {
            participants.insert(window, Participant::AwaitingAck(serial));
        }
        let fresh_deadline = now.saturating_add(self.timeout);
        let armed = superseded.is_none().then_some(fresh_deadline);
        let deadline = superseded.map_or(fresh_deadline, |pending| pending.deadline);
        self.pending = Some(Pending {
            scene,
            deadline,
            participants,
        });
        armed
    }

    pub(crate) fn acknowledge(&mut self, window: WindowId, serial: ConfigureSerial) {
        let Some(participant) = self.participant_mut(window) else {
            return;
        };
        if let Participant::AwaitingAck(expected) = *participant
            && serial.is_no_older_than(expected)
        {
            *participant = Participant::AwaitingCommit;
        }
    }

    pub(crate) fn commit(&mut self, window: WindowId) {
        let Some(pending) = &mut self.pending else {
            return;
        };
        if pending.participants.get(&window) == Some(&Participant::AwaitingCommit) {
            pending.participants.remove(&window);
        }
    }

    pub(crate) fn forget(&mut self, window: WindowId) {
        if let Some(pending) = &mut self.pending {
            pending.participants.remove(&window);
            pending.scene.remove(window);
        }
    }

    pub(crate) fn settle(&mut self, now: Duration) -> Option<Scene> {
        let pending = self.pending.as_ref()?;
        if !pending.participants.is_empty() && now < pending.deadline {
            return None;
        }
        self.pending.take().map(|pending| pending.scene)
    }

    pub(crate) const fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    fn participant_mut(&mut self, window: WindowId) -> Option<&mut Participant> {
        self.pending.as_mut()?.participants.get_mut(&window)
    }
}

#[cfg(test)]
mod tests {
    use tilling_core::geometry::{LogicalRect, Scale};
    use tilling_core::layout::Gaps;
    use tilling_core::{Command, DesktopState, Effect, WindowMetadata, WindowMode};

    use super::*;

    const TIMEOUT: Duration = Duration::from_millis(200);

    fn millis(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    struct Fixture {
        desktop: DesktopState,
    }

    impl Fixture {
        fn new() -> Self {
            let mut desktop = DesktopState::new("1");
            desktop
                .apply(Command::AddOutput {
                    name: "TEST-1".into(),
                    area: LogicalRect::new(0, 0, 800, 600).unwrap(),
                    scale: Scale::ONE,
                })
                .unwrap();
            Self { desktop }
        }

        fn open(&mut self) -> WindowId {
            let transition = self
                .desktop
                .apply(Command::OpenWindow {
                    metadata: WindowMetadata::default(),
                    workspace: None,
                    transient_for: None,
                    mode: WindowMode::Tiled,
                })
                .unwrap();
            transition
                .effects
                .iter()
                .find_map(|effect| match effect {
                    Effect::WindowOpened(window) => Some(*window),
                    _ => None,
                })
                .unwrap()
        }

        fn scene(&self) -> Scene {
            Scene::build(&self.desktop, Gaps::default())
        }
    }

    #[test]
    fn scene_without_participants_is_published_immediately() {
        let mut fixture = Fixture::new();
        fixture.open();
        let mut coordinator = Coordinator::new(TIMEOUT);

        coordinator.begin(fixture.scene(), [], millis(0));

        assert_eq!(coordinator.settle(millis(0)), Some(fixture.scene()));
        assert!(!coordinator.is_pending());
    }

    #[test]
    fn scene_waits_until_every_participant_acks_and_commits() {
        let mut fixture = Fixture::new();
        let (first, second) = (fixture.open(), fixture.open());
        let mut coordinator = Coordinator::new(TIMEOUT);
        let configured = [(first, ConfigureSerial(10)), (second, ConfigureSerial(11))];

        let armed = coordinator.begin(fixture.scene(), configured, millis(0));
        coordinator.acknowledge(first, ConfigureSerial(10));
        coordinator.commit(first);
        coordinator.acknowledge(second, ConfigureSerial(11));

        assert_eq!(armed, Some(TIMEOUT));
        assert_eq!(coordinator.settle(millis(50)), None);
        coordinator.commit(second);
        assert_eq!(coordinator.settle(millis(60)), Some(fixture.scene()));
    }

    #[test]
    fn deadline_publishes_despite_a_client_that_ignores_configure() {
        let mut fixture = Fixture::new();
        let stuck = fixture.open();
        let mut coordinator = Coordinator::new(TIMEOUT);

        coordinator.begin(fixture.scene(), [(stuck, ConfigureSerial(1))], millis(0));

        assert_eq!(coordinator.settle(millis(199)), None);
        assert_eq!(coordinator.settle(millis(200)), Some(fixture.scene()));
    }

    #[test]
    fn stale_ack_and_commit_before_ack_do_not_complete_a_participant() {
        let mut fixture = Fixture::new();
        let window = fixture.open();
        let mut coordinator = Coordinator::new(TIMEOUT);

        coordinator.begin(fixture.scene(), [(window, ConfigureSerial(7))], millis(0));
        coordinator.commit(window);
        coordinator.acknowledge(window, ConfigureSerial(6));
        coordinator.commit(window);

        assert_eq!(coordinator.settle(millis(1)), None);
        coordinator.acknowledge(window, ConfigureSerial(7));
        coordinator.commit(window);
        assert!(coordinator.settle(millis(2)).is_some());
    }

    #[test]
    fn serial_comparison_survives_wrap_around() {
        let mut fixture = Fixture::new();
        let window = fixture.open();
        let mut coordinator = Coordinator::new(TIMEOUT);
        let configured = [(window, ConfigureSerial(u32::MAX))];

        coordinator.begin(fixture.scene(), configured, millis(0));
        coordinator.acknowledge(window, ConfigureSerial(2));
        coordinator.commit(window);

        assert!(coordinator.settle(millis(1)).is_some());
    }

    #[test]
    fn destroyed_window_leaves_the_transaction_and_its_scene() {
        let mut fixture = Fixture::new();
        let (kept, destroyed) = (fixture.open(), fixture.open());
        let mut coordinator = Coordinator::new(TIMEOUT);
        let configured = [(destroyed, ConfigureSerial(3))];

        coordinator.begin(fixture.scene(), configured, millis(0));
        coordinator.forget(destroyed);
        let published = coordinator.settle(millis(1)).unwrap();

        assert!(published.window(kept).is_some());
        assert_eq!(published.window(destroyed), None);
    }

    #[test]
    fn superseding_keeps_the_first_deadline_and_unfinished_participants() {
        let mut fixture = Fixture::new();
        let slow = fixture.open();
        let mut coordinator = Coordinator::new(TIMEOUT);
        coordinator.begin(fixture.scene(), [(slow, ConfigureSerial(1))], millis(0));
        let fast = fixture.open();

        let armed = coordinator.begin(fixture.scene(), [(fast, ConfigureSerial(2))], millis(150));
        coordinator.acknowledge(fast, ConfigureSerial(2));
        coordinator.commit(fast);

        assert_eq!(armed, None);
        assert_eq!(coordinator.settle(millis(199)), None);
        assert_eq!(coordinator.settle(millis(200)), Some(fixture.scene()));
    }
}
