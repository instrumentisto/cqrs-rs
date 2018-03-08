extern crate cqrs;
extern crate cqrs_memory;
extern crate cqrs_redis;
extern crate cqrs_todo_core;

#[macro_use] extern crate juniper;
extern crate juniper_iron;
#[macro_use] extern crate juniper_codegen;
extern crate chrono;
extern crate base64;
extern crate fnv;
extern crate iron;
extern crate mount;
extern crate hashids;

extern crate redis;
extern crate r2d2_redis;
extern crate r2d2;
extern crate serde;
extern crate serde_json;

mod graphql;
mod store;

use std::sync::Arc;
use cqrs_todo_core::{Event, TodoAggregate};

use cqrs::domain::query::{QueryableSnapshotAggregate, SnapshotAndEventsView};
use cqrs::domain::execute::ViewExecutor;
use cqrs::domain::persist::{PersistableSnapshotAggregate, EventsAndSnapshotWithDecorator};
use cqrs::domain::ident::{AggregateIdProvider};

use r2d2_redis::RedisConnectionManager;

type AggregateId = String;

type EventStore = store::MemoryOrNullEventStore;
type SnapshotStore = store::MemoryOrNullSnapshotStore;

type View =
    SnapshotAndEventsView<
        TodoAggregate,
        Arc<EventStore>,
        Arc<SnapshotStore>,
    >;

type Executor =
    ViewExecutor<
        TodoAggregate,
        View,
    >;

type Commander =
    EventsAndSnapshotWithDecorator<
        TodoAggregate,
        Executor,
        Arc<EventStore>,
        Arc<SnapshotStore>,
        cqrs::trivial::NopEventDecorator<Event>,
    >;

pub enum BackendChoice {
    Memory,
    Null,
    Redis(String)
}

fn create_view(es: &Arc<EventStore>, ss: &Arc<SnapshotStore>) -> View {
    TodoAggregate::snapshot_with_events_view(Arc::clone(&es), Arc::clone(&ss))
}

fn create_commander(es: &Arc<EventStore>, ss: &Arc<SnapshotStore>) -> Commander {
    let executor = cqrs::domain::execute::ViewExecutor::new(create_view(es, ss));
    TodoAggregate::persist_events_and_snapshot(executor, Arc::clone(&es), Arc::clone(&ss))
        .without_decorator()
}

pub fn start_todo_server(event_backend: BackendChoice, snapshot_backend: BackendChoice) -> iron::Listening {
    let es =
        match event_backend {
            BackendChoice::Null =>
                store::MemoryOrNullEventStore::new_null_store(),
            BackendChoice::Memory =>
                store::MemoryOrNullEventStore::new_memory_store(),
            BackendChoice::Redis(conn_str) => {
                let pool = r2d2::Pool::new(RedisConnectionManager::new(conn_str.as_ref()).unwrap()).unwrap();
                let config = cqrs_redis::Config::new("todoql");
                store::MemoryOrNullEventStore::new_redis_store(config, pool)
            }
        };

    let ss =
        match snapshot_backend {
            BackendChoice::Null =>
                store::MemoryOrNullSnapshotStore::new_null_store(),
            BackendChoice::Memory =>
                store::MemoryOrNullSnapshotStore::new_memory_store(),
            BackendChoice::Redis(conn_str) => {
                let pool = r2d2::Pool::new(r2d2_redis::RedisConnectionManager::new(conn_str.as_ref()).unwrap()).unwrap();
                let config = cqrs_redis::Config::new("todoql");
                store::MemoryOrNullSnapshotStore::new_redis_store(config, pool)
            }
        };

    let hashid =
        if let Ok(hashid) = hashids::HashIds::new_with_salt_and_min_length("cqrs".to_string(), 10) {
            hashid
        } else {
            panic!("Failed to generate hashid")
        };

    let id_provider = IdProvider(hashid, Default::default());

    let id = id_provider.new_id();

    helper::prefill(&id, &es, &ss);

    let stream_index = vec![id];

    let es = Arc::new(es);
    let ss = Arc::new(ss);

    let query = create_view(&es, &ss);
    let command = create_commander(&es, &ss);

    let context = graphql::InnerContext::new(
        stream_index,
        query,
        command,
        id_provider,
    );

    let chain = graphql::endpoint::create_chain(context);

    iron::Iron::new(chain).http("0.0.0.0:2777").unwrap()
}

pub struct IdProvider(hashids::HashIds,::std::sync::atomic::AtomicUsize);

impl AggregateIdProvider for IdProvider {
    type AggregateId = String;

    fn new_id(&self) -> Self::AggregateId {
        let next = self.1.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst);
        let duration = ::std::time::SystemTime::now().duration_since(::std::time::UNIX_EPOCH).unwrap();
        self.0.encode(&vec![duration.as_secs() as i64, next as i64])
    }
}

mod helper {
    use chrono::{Duration,Utc,TimeZone};
    use cqrs::{Version, VersionedSnapshot,EventAppend,SnapshotPersist};
    use cqrs_todo_core::{Event, TodoAggregate, TodoData, TodoStatus, domain};

    use super::{AggregateId, EventStore, SnapshotStore};

    pub fn prefill(id: &AggregateId, es: &EventStore, ss: &SnapshotStore) {
        let epoch = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = epoch + Duration::seconds(10000);
        let mut events = Vec::new();
        events.push(Event::Completed);
        events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
        events.push(Event::ReminderUpdated(Some(domain::Reminder::new(reminder_time, epoch).unwrap())));
        events.push(Event::TextUpdated(domain::Description::new("New text").unwrap()));
        events.push(Event::Created(domain::Description::new("Ignored!").unwrap()));
        events.push(Event::ReminderUpdated(None));

        es.append_events(id, &events, None).unwrap();
        ss.persist_snapshot(id, VersionedSnapshot {
            version: Version::from(1),
            snapshot: TodoAggregate::Created(TodoData {
                description: domain::Description::new("Hello!").unwrap(),
                reminder: None,
                status: TodoStatus::NotCompleted,
            })
        }).unwrap();
    }
}
