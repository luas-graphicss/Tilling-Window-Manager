use std::fmt;
use std::marker::PhantomData;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub(crate) struct Key {
    index: u32,
    generation: u32,
}

pub(crate) trait Identifier: Copy + Eq {
    fn from_key(key: Key) -> Self;
    fn key(self) -> Key;
}

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(Key);

        impl Identifier for $name {
            fn from_key(key: Key) -> Self {
                Self(key)
            }

            fn key(self) -> Key {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}v{}", self.0.index, self.0.generation)
            }
        }
    };
}

identifier!(WindowId);
identifier!(WorkspaceId);
identifier!(OutputId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("identifier space is exhausted")]
pub struct CapacityExhausted;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Arena<I, T> {
    slots: Vec<Slot<T>>,
    vacant: Vec<u32>,
    #[serde(skip)]
    marker: PhantomData<fn() -> I>,
}

impl<I: Identifier, T> Arena<I, T> {
    pub(crate) fn new() -> Self {
        Self {
            slots: Vec::new(),
            vacant: Vec::new(),
            marker: PhantomData,
        }
    }

    pub(crate) fn with_initial(value: T) -> (Self, I) {
        let arena = Self {
            slots: vec![Slot {
                generation: 0,
                value: Some(value),
            }],
            vacant: Vec::new(),
            marker: PhantomData,
        };
        let key = Key {
            index: 0,
            generation: 0,
        };
        (arena, I::from_key(key))
    }

    pub(crate) fn insert(&mut self, value: T) -> Result<I, CapacityExhausted> {
        if let Some(index) = self.vacant.pop()
            && let Some(slot) = self.slots.get_mut(position(index))
        {
            slot.value = Some(value);
            let key = Key {
                index,
                generation: slot.generation,
            };
            return Ok(I::from_key(key));
        }
        let index = u32::try_from(self.slots.len()).map_err(|_| CapacityExhausted)?;
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        let key = Key {
            index,
            generation: 0,
        };
        Ok(I::from_key(key))
    }

    pub(crate) fn remove(&mut self, id: I) -> Option<T> {
        let key = id.key();
        let slot = self.slots.get_mut(position(key.index))?;
        if slot.generation != key.generation {
            return None;
        }
        let value = slot.value.take()?;
        if let Some(next_generation) = slot.generation.checked_add(1) {
            slot.generation = next_generation;
            self.vacant.push(key.index);
        }
        Some(value)
    }

    pub(crate) fn get(&self, id: I) -> Option<&T> {
        let key = id.key();
        let slot = self.slots.get(position(key.index))?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_ref()
    }

    pub(crate) fn get_mut(&mut self, id: I) -> Option<&mut T> {
        let key = id.key();
        let slot = self.slots.get_mut(position(key.index))?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_mut()
    }

    pub(crate) fn contains(&self, id: I) -> bool {
        self.get(id).is_some()
    }

    pub(crate) fn len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.value.is_some())
            .count()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(position, slot)| {
                let index = u32::try_from(position).ok()?;
                let key = Key {
                    index,
                    generation: slot.generation,
                };
                Some((I::from_key(key), slot.value.as_ref()?))
            })
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots.iter_mut().filter_map(|slot| slot.value.as_mut())
    }
}

fn position(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_slot_rejects_the_previous_generation() {
        let mut arena: Arena<WindowId, &str> = Arena::new();
        let first = arena.insert("first").unwrap();
        assert_eq!(arena.remove(first), Some("first"));

        let second = arena.insert("second").unwrap();

        assert_ne!(first, second);
        assert_eq!(arena.get(first), None);
        assert_eq!(arena.remove(first), None);
        assert_eq!(arena.get(second), Some(&"second"));
    }

    #[test]
    fn removing_twice_yields_nothing() {
        let mut arena: Arena<WindowId, u8> = Arena::new();
        let id = arena.insert(7).unwrap();

        assert_eq!(arena.remove(id), Some(7));
        assert_eq!(arena.remove(id), None);
        assert_eq!(arena.len(), 0);
    }

    #[test]
    fn iteration_follows_slot_order() {
        let mut arena: Arena<WorkspaceId, char> = Arena::new();
        let a = arena.insert('a').unwrap();
        let b = arena.insert('b').unwrap();
        let c = arena.insert('c').unwrap();
        arena.remove(b);
        let d = arena.insert('d').unwrap();

        let order: Vec<_> = arena.iter().collect();

        assert_eq!(order, vec![(a, &'a'), (d, &'d'), (c, &'c')]);
    }

    #[test]
    fn exhausted_generation_retires_the_slot() {
        let (mut arena, id): (Arena<OutputId, u8>, OutputId) = Arena::with_initial(1);
        arena.slots[0].generation = u32::MAX;
        let last = OutputId::from_key(Key {
            index: id.key().index,
            generation: u32::MAX,
        });

        assert_eq!(arena.remove(last), Some(1));
        let replacement = arena.insert(2).unwrap();

        assert_eq!(arena.get(last), None);
        assert_eq!(replacement.key().index, 1);
    }

    #[test]
    fn identifiers_display_index_and_generation() {
        let mut arena: Arena<WindowId, ()> = Arena::new();
        let first = arena.insert(()).unwrap();
        arena.remove(first);
        let second = arena.insert(()).unwrap();

        assert_eq!(first.to_string(), "0v0");
        assert_eq!(second.to_string(), "0v1");
    }
}
