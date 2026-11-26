//! The client of the person service for other modules (feature `client`).
//!
//! Pattern: Facade – declares the parts and re-exports `PersonServiceOwner`; the logic is in `owner`, `program` and `read`.

mod owner;
mod program;
mod read;

pub use owner::PersonServiceOwner;
pub use program::service_program;
pub use read::me_ids;
