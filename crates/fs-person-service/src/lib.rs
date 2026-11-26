//! `fs-person-service`: the person module as a service (one process per person) over fs-rpc. It serves the form source of the
//! module `person` as `Guarded<PersonCore>` (rights: `PersonRules`):
//! the command `me.create` and the form `me` (the one person of this installation, or nothing yet).
//!
//! Nobody names the actor: there is no `--actor` and no `actor` argument. The connection only decides who may call (fs-rpc admits
//! the same user, read from the system); the log names the anchor as the actor of its own birth, not the caller (the tie of a user
//! to the anchor comes with fs-identity). The data lives in `$XDG_DATA_HOME/freesynergy/modules/person/`.
//!
//! Pattern: Facade – this file only declares modules and re-exports.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// The generated `Server` trait is async for every method, also those that never wait.
#![allow(clippy::unused_async_trait_impl)]

mod bootstrap;
#[cfg(feature = "client")]
mod client;
mod core;
mod forms;
mod owner;
mod pages;
mod rows;
mod rules;
mod serve;
mod service;

#[cfg(feature = "client")]
pub use client::{PersonServiceOwner, me_ids, service_program};
pub use core::PersonCore;
pub use forms::{
    COMMAND_CREATE, FORM_ME, FORM_RECORD, FORM_ROWS, FORM_TILES, LEVEL, PAGE_MAX, PROVIDER,
    SCOPE_ALL,
};
pub use owner::Owner;
pub use rows::{MEDIA, me_record, profile_record, profile_records, record_fields, state_word};
pub use rules::{NO_CONNECTION, NOT_A_PERSON, NOT_THE_OWNER, PersonRules};
pub use serve::{PROGRAM, SAMPLE_PROGRAM, reject_actor, serve_command, serve_with, socket_program};
pub use service::PersonService;
