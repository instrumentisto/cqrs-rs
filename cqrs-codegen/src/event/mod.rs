//! Codegen for [`cqrs::Event`] and related (e.g., [`cqrs::VersionedEvent`], etc).

mod common;
mod event;
mod versioned_event;

pub(crate) use event::derive as derive_event;
pub(crate) use versioned_event::derive as derive_versioned_event;
