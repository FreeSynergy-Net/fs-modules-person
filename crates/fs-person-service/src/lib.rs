//! `fs-person-service`: the person module as a service (one process per person) over fs-rpc. It serves the form source (level 1)
//! of the module `person`: the command `me.create` and the form `me` (the one person of this installation, or nothing yet).
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

#[cfg(feature = "client")]
mod client;
mod serve;
mod service;
mod source;

#[cfg(feature = "client")]
pub use client::{PersonServiceOwner, service_program};
pub use serve::{PROGRAM, reject_actor, serve_command, serve_with};
pub use service::PersonService;
pub use source::{COMMAND_CREATE, FORM_ME, LEVEL, PROVIDER};
