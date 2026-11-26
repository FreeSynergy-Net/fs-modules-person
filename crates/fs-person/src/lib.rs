//! `fs-person`: the person module of FreeSynergy. It knows the one person of this installation (the anchor) and the
//! profiles (contact cards, RFC 9553) that describe it or hang on nobody.
//!
//! Pattern: Repository – the ports [`MeRepo`] (reads and creates "me") and [`ProfileRepo`] (reads profiles); the stores behind
//! them ([`StoreMe`], [`StoreProfiles`]) are adapters that can be exchanged. [`StoreOwner`] hands the anchor to everybody
//! who needs the local owner. [`Row`] is a projection of a profile for the list.
//!
//! Profiles are read only here (names, addresses, links to accounts). There is never a second person next to the anchor.

mod anchor;
mod describe;
mod fields;
mod owner;
mod profile;
mod profile_store;
mod profiles;
mod repo;
mod row;
mod store;
mod values;

pub use anchor::Anchor;
pub use describe::{DESCRIBES, PROFILE, PROGRAM, ProfileType};
pub use owner::StoreOwner;
pub use profile::{CONTENT_TYPE, Profile, TYPE as PROFILE_TYPE};
pub use profile_store::StoreProfiles;
pub use profiles::ProfileRepo;
pub use repo::{Created, MeRepo};
pub use row::{Row, State};
pub use store::StoreMe;
pub use values::values_of;
