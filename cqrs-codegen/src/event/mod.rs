//! Codegen for [`cqrs::Event`] and related (e.g., [`cqrs::VersionedEvent`], etc).

mod common;
mod event;
mod versioned_event;

pub(crate) use event::derive as derive;
pub(crate) use versioned_event::derive as versioned_derive;
