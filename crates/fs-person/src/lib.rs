//! `fs-person`: the person module of FreeSynergy. It knows the one person of this installation: the anchor.
//!
//! Pattern: Repository – the port [`MeRepo`] reads and creates "me"; the store behind it ([`StoreMe`]) is an adapter that
//! can be exchanged without changing the anchor. [`StoreOwner`] hands the anchor to everybody who needs the local owner.
//!
//! No profile data here (names, addresses, links to accounts): they come later, as profiles of the anchor.

mod anchor;
mod owner;
mod repo;
mod store;

pub use anchor::Anchor;
pub use owner::StoreOwner;
pub use repo::{Created, MeRepo};
pub use store::StoreMe;
