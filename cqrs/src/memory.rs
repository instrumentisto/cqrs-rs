use std::hash::BuildHasher;
use std::fmt;
use std::marker::PhantomData;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use void::Void;
use cqrs_core::{Aggregate, EventSource, EventSink, SnapshotSource, SnapshotSink, EventNumber, VersionedEvent, Since, Version, VersionedAggregate, VersionedAggregateView, Precondition};

type EventStream<Event> = RwLock<Vec<Event>>;

#[derive(Debug)]
pub struct EventStore<A, Hasher = DefaultHashBuilder>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, EventStream<VersionedEvent<A::Event>>, Hasher>>,
    _phantom: PhantomData<A>,
}

impl<A, Hasher> Default for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        EventStore {
            inner: RwLock::new(HashMap::default()),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    pub fn with_hasher(hasher: Hasher) -> Self {
        EventStore {
            inner: RwLock::new(HashMap::with_hasher(hasher)),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> EventSource<A> for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    type Events = Vec<Result<VersionedEvent<A::Event>, Void>>;
    type Error = Void;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
        let table = self.inner.read();

        let stream = table.get(id);

        let result =
            stream.map(|stream| {
                let stream = stream.read();
                match (since, max_count) {
                    (Since::BeginningOfStream, None) => {
                        stream.iter()
                            .map(ToOwned::to_owned)
                            .map(Ok)
                            .collect()
                    },
                    (Since::Event(event_number), None) => {
                        stream.iter()
                            .skip(event_number.get() as usize)
                            .map(ToOwned::to_owned)
                            .map(Ok)
                            .collect()
                    },
                    (Since::BeginningOfStream, Some(max_count)) => {
                        stream.iter()
                            .take(max_count.min(usize::max_value() as u64) as usize)
                            .map(ToOwned::to_owned)
                            .map(Ok)
                            .collect()
                    },
                    (Since::Event(event_number), Some(max_count)) => {
                        stream.iter()
                            .skip(event_number.get() as usize)
                            .take(max_count.min(usize::max_value() as u64) as usize)
                            .map(ToOwned::to_owned)
                            .map(Ok)
                            .collect()
                    },
                }
            });

        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreconditionFailed(pub Precondition);

impl From<Precondition> for PreconditionFailed {
    fn from(p: Precondition) -> Self {
        PreconditionFailed(p)
    }
}

impl fmt::Display for PreconditionFailed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "precondition failed: {}", self.0)
    }
}

impl<A, Hasher> EventSink<A> for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    type Error = PreconditionFailed;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let table = self.inner.upgradable_read();

        if table.contains_key(id) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let stream = table.get(id).unwrap().upgradable_read();

            let mut sequence = Version::new(stream.len() as u64).next_event();
            let first_sequence = sequence;

            if let Some(precondition) = precondition {
                precondition.verify(Some(first_sequence.into()))?;
            }

            let stream = &mut RwLockUpgradableReadGuard::upgrade(stream);

            stream.extend(events.into_iter().map(|event| {
                let versioned_event = VersionedEvent {
                    sequence,
                    event: event.to_owned(),
                };
                sequence = sequence.incr();
                versioned_event
            }));

            Ok(first_sequence)
        } else {
            if let Some(precondition) = precondition {
                precondition.verify(None)?;
            }

            let mut sequence = EventNumber::MIN_VALUE;

            let stream = RwLock::new(events.into_iter().map(|event| {
                let versioned_event = VersionedEvent {
                    sequence,
                    event: event.to_owned(),
                };
                sequence = sequence.incr();
                versioned_event
            }).collect());

            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), stream);

            Ok(EventNumber::MIN_VALUE)
        }
    }
}

#[derive(Debug)]
pub struct StateStore<A, Hasher = DefaultHashBuilder>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<VersionedAggregate<A>>, Hasher>>,
    _phantom: PhantomData<A>,
}

impl<A, Hasher> Default for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        StateStore {
            inner: RwLock::new(HashMap::default()),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    pub fn with_hasher(hasher: Hasher) -> Self {
        StateStore {
            inner: RwLock::new(HashMap::with_hasher(hasher)),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> SnapshotSource<A> for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error> where Self: Sized {
        let table = self.inner.read();

        let snapshot =
            table.get(id)
                .map(|data| data.read().to_owned());

        Ok(snapshot)
    }
}

impl<A, Hasher> SnapshotSink<A> for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn persist_snapshot(&self, id: &str, view: VersionedAggregateView<A>) -> Result<(), Self::Error> where Self: Sized {
        let table = self.inner.upgradable_read();

        if table.contains_key(id) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            *table.get(id).unwrap().write() = view.into();
        } else {
            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), RwLock::new(view.into()));
        };

        Ok(())
    }
}

#[path = "memory_tests.rs"]
#[cfg(test)]
pub(crate) mod tests;