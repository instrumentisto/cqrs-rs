use cqrs_codegen::Event;

#[derive(Event)]
#[event(type = "event.1")]
struct Event1;

#[derive(Event)]
#[event(type = "event.2")]
struct Event2;

#[derive(Event)]
enum EnumEvent {
    Event1(Event1),
    Event2(Event2),
}

#[derive(Event)]
enum EnumEventGeneric<E1, E2> {
    Event1(E1),
    Event2(E2),
}

// TODO
#[test]
fn assert_stuff() {
    unimplemented!()
}
