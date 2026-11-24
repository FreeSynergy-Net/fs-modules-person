//! `fs-person-service`: the person module as a service (one process per person) over fs-rpc. It serves the form source (level 1)
//! of the module `person`: the command `me.create` and the form `me` (the one person of this installation, or nothing yet).
//!
//! The actor of a request is the user of the connection (fs-rpc reads it from the system), never a word of a message or of the
//! command line: there is no `--actor` and no `actor` argument. The data lives in `$XDG_DATA_HOME/freesynergy/modules/person/`.
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
pub use client::PersonServiceOwner;
pub use serve::{PROGRAM, reject_actor, serve_command, serve_with};
pub use service::PersonService;
pub use source::{COMMAND_CREATE, FORM_ME, LEVEL, PROVIDER};
