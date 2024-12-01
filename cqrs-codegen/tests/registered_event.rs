#![allow(dead_code)]

use std::any::TypeId;

use cqrs::RegisteredEvent as _;
use cqrs_codegen::{Event, RegisteredEvent};

#[test]
fn derives_for_struct() {
    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event")]
    struct TestEvent {
        id: i32,
        data: String,
    };

    assert_eq!(TestEvent::default().type_id(), TypeId::of::<TestEvent>());
}

#[test]
fn derives_for_generic_struct() {
    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.generic")]
    struct TestEventGeneric<ID, Data>
    where
        ID: 'static,
        Data: 'static,
    {
        id: ID,
        data: Data,
    };

    type TestEvent = TestEventGeneric<i32, String>;

    assert_eq!(TestEvent::default().type_id(), TypeId::of::<TestEvent>());
}

#[test]
fn derives_for_enum() {
    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.1")]
    struct TestEvent1;

    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.2")]
    struct TestEvent2;

    #[derive(Event, RegisteredEvent)]
    enum TestEvent {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
    }

    assert_eq!(
        TestEvent::TestEventTuple(Default::default()).type_id(),
        TypeId::of::<TestEvent1>(),
    );
    assert_eq!(
        TestEvent::TestEventStruct {
            event: Default::default()
        }
        .type_id(),
        TypeId::of::<TestEvent2>(),
    );
}

#[test]
fn derives_for_deeply_nested_enum() {
    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.1")]
    struct TestEvent1;

    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.2")]
    struct TestEvent2;

    #[derive(Event, RegisteredEvent)]
    enum TestEvent {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
    }

    #[derive(Event, RegisteredEvent)]
    enum TestEventNested {
        TestEvent(TestEvent),
    }

    assert_eq!(
        TestEventNested::TestEvent(TestEvent::TestEventTuple(Default::default())).type_id(),
        TypeId::of::<TestEvent1>(),
    );
    assert_eq!(
        TestEventNested::TestEvent(TestEvent::TestEventStruct {
            event: Default::default()
        })
        .type_id(),
        TypeId::of::<TestEvent2>(),
    );
}

#[test]
fn derives_for_generic_enum() {
    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.generic.1")]
    struct TestEventGeneric1<ID, Data>
    where
        ID: 'static,
        Data: 'static,
    {
        id: ID,
        data: Data,
    }

    #[derive(Default, Event, RegisteredEvent)]
    #[event(name = "test.event.generic.2")]
    struct TestEventGeneric2<ID, Data>
    where
        ID: 'static,
        Data: 'static,
    {
        id: ID,
        data: Data,
    }

    #[derive(Event, RegisteredEvent)]
    enum TestEventGeneric<ID, Data>
    where
        ID: 'static,
        Data: 'static,
    {
        TestEventTupleGeneric(TestEventGeneric1<ID, Data>),
        TestEventStructGeneric { event: TestEventGeneric2<ID, Data> },
    }

    type TestEvent = TestEventGeneric<i32, String>;

    assert_eq!(
        TestEvent::TestEventTupleGeneric(Default::default()).type_id(),
        TypeId::of::<TestEventGeneric1<i32, String>>(),
    );
    assert_eq!(
        TestEvent::TestEventStructGeneric {
            event: Default::default()
        }
        .type_id(),
        TypeId::of::<TestEventGeneric2<i32, String>>(),
    );
}
