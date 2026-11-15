use smithay::output::Output;
use smithay::reexports::wayland_server::backend::GlobalId;
use tilling_core::OutputId;

#[derive(Debug)]
pub(super) struct Registered {
    pub(super) id: OutputId,
    pub(super) output: Output,
    pub(super) global: GlobalId,
}

#[derive(Debug, Default)]
pub(crate) struct OutputRegistry {
    entries: Vec<Registered>,
}

impl OutputRegistry {
    pub(crate) fn id_of(&self, output: &Output) -> Option<OutputId> {
        self.entries
            .iter()
            .find(|registered| registered.output == *output)
            .map(|registered| registered.id)
    }

    pub(crate) fn by_id(&self, id: OutputId) -> Option<&Output> {
        self.entries
            .iter()
            .find(|registered| registered.id == id)
            .map(|registered| &registered.output)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (OutputId, &Output)> {
        self.entries
            .iter()
            .map(|registered| (registered.id, &registered.output))
    }

    pub(super) fn insert(&mut self, registered: Registered) {
        self.entries.push(registered);
    }

    pub(super) fn remove(&mut self, output: &Output) -> Option<Registered> {
        let position = self
            .entries
            .iter()
            .position(|registered| registered.output == *output)?;
        Some(self.entries.remove(position))
    }
}
