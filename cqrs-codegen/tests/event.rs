#![allow(dead_code)]

use cqrs::Event as _;
use cqrs_codegen::Event;

#[test]
fn derives_for_struct() {
    #[derive(
        Default,
        // Event
    )]
    // #[event(name = "test.event")]
    struct TestEvent {
        id: i32,
        data: String,
    };

    #[automatically_derived]
    impl ::cqrs::StaticTypedEvent for TestEvent {
        #[doc = "Type name of [`TestEvent`] event."]
        const EVENT_TYPE: ::cqrs::EventType = "test.event";
    }
    #[automatically_derived]
    impl ::cqrs::Event for TestEvent
    where
        Self: ::cqrs::StaticTypedEvent,
    {
        #[inline(always)]
        fn event_type(&self) -> ::cqrs::EventType {
            <Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE
        }
    }
    #[automatically_derived]
    impl ::cqrs::TypedEvent for TestEvent
    where
        Self: ::cqrs::StaticTypedEvent,
    {
        #[doc = "Type names of [`TestEvent`] events."]
        const EVENT_TYPES: &'static [::cqrs::EventType] =
            &[<Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE];
    }

    assert_eq!(
        <TestEvent as cqrs::StaticTypedEvent>::EVENT_TYPE,
        "test.event"
    );
    assert_eq!(TestEvent::default().event_type(), "test.event");
}

#[test]
fn derives_for_generic_struct() {
    #[derive(
        Default,
        // Event
    )]
    // #[event(name = "test.event.generic")]
    struct TestEventGeneric<ID, Data> {
        id: ID,
        data: Data,
    };

    #[automatically_derived]
    impl<ID, Data> ::cqrs::StaticTypedEvent for TestEventGeneric<ID, Data> {
        #[doc = "Type name of [`TestEventGeneric`] event."]
        const EVENT_TYPE: ::cqrs::EventType = "test.event.generic";
    }
    #[automatically_derived]
    impl<ID, Data> ::cqrs::Event for TestEventGeneric<ID, Data>
    where
        Self: ::cqrs::StaticTypedEvent,
    {
        #[inline(always)]
        fn event_type(&self) -> ::cqrs::EventType {
            <Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE
        }
    }
    #[automatically_derived]
    impl<ID, Data> ::cqrs::TypedEvent for TestEventGeneric<ID, Data>
    where
        Self: ::cqrs::StaticTypedEvent,
    {
        #[doc = "Type names of [`TestEventGeneric`] events."]
        const EVENT_TYPES: &'static [::cqrs::EventType] =
            &[<Self as ::cqrs::StaticTypedEvent>::EVENT_TYPE];
    }

    type TestEvent = TestEventGeneric<i32, String>;

    assert_eq!(
        <TestEvent as cqrs::StaticTypedEvent>::EVENT_TYPE,
        "test.event.generic"
    );
    assert_eq!(TestEvent::default().event_type(), "test.event.generic");
}

#[test]
fn derives_for_enum() {
    #[derive(
        Default,
        // Event
    )]
    // #[event(name = "test.event.1")]
    struct TestEvent1;

    #[automatically_derived] impl :: cqrs :: StaticTypedEvent for TestEvent1
    {
        #[doc = "Type name of [`TestEvent1`] event."] const EVENT_TYPE : :: cqrs
        :: EventType = "test.event.1";
    } #[automatically_derived] impl :: cqrs :: Event for TestEvent1 where Self :
    :: cqrs :: StaticTypedEvent
    {
        #[inline(always)] fn event_type(& self) -> :: cqrs :: EventType
        { < Self as :: cqrs :: StaticTypedEvent > :: EVENT_TYPE }
    } #[automatically_derived] impl :: cqrs :: TypedEvent for TestEvent1 where
        Self : :: cqrs :: StaticTypedEvent
    {
        #[doc = "Type names of [`TestEvent1`] events."] const EVENT_TYPES : &
        'static [:: cqrs :: EventType] = &
            [< Self as :: cqrs :: StaticTypedEvent > :: EVENT_TYPE];
    }

    #[derive(Default, Event)]
    #[event(name = "test.event.2")]
    struct TestEvent2;

    #[derive(Event)]
    enum TestEvent {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
    }

    assert_eq!(
        TestEvent::TestEventTuple(Default::default()).event_type(),
        "test.event.1",
    );
    assert_eq!(
        TestEvent::TestEventStruct {
            event: Default::default()
        }
        .event_type(),
        "test.event.2",
    );
}

#[test]
fn derives_for_generic_enum() {
    #[derive(Default, Event)]
    #[event(name = "test.event.generic.1")]
    struct TestEventGeneric1<ID, Data> {
        id: ID,
        data: Data,
    }

    #[derive(Default, Event)]
    #[event(name = "test.event.generic.2")]
    struct TestEventGeneric2<ID, Data> {
        id: ID,
        data: Data,
    }

    #[derive(Event)]
    enum TestEventGeneric<ID, Data> {
        TestEventTupleGeneric(TestEventGeneric1<ID, Data>),
        TestEventStructGeneric { event: TestEventGeneric2<ID, Data> },
    }

    type TestEvent = TestEventGeneric<i32, String>;

    assert_eq!(
        TestEvent::TestEventTupleGeneric(Default::default()).event_type(),
        "test.event.generic.1",
    );
    assert_eq!(
        TestEvent::TestEventStructGeneric {
            event: Default::default()
        }
        .event_type(),
        "test.event.generic.2",
    );
}
