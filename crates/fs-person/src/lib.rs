//! `fs-person`: the person module of FreeSynergy. It knows the one person of this installation (the anchor) and the
//! profiles (contact cards, RFC 9553) that describe it or hang on nobody.
//!
//! Pattern: Repository – the ports [`MeRepo`] (reads and creates "me"), [`ProfileRepo`] (reads profiles) and both together
//! ([`PersonRepo`]); the stores behind them ([`StoreMe`], [`StoreProfiles`], [`PersonStore`], the sample data [`SampleRepo`])
//! are adapters that can be exchanged. [`StoreOwner`] hands the anchor to everybody
//! who needs the local owner. [`Row`] is a projection of a profile for the list.
//!
//! Profiles are written by [`ProfileWrite`] (make, change, delete a contact) and read by [`ProfileRepo`]. There is never a second person next to the anchor.

mod anchor;
mod catalog_file;
mod describe;
mod fields;
mod list;
mod owner;
mod person_repo;
mod person_store;
mod preferred;
mod profile;
mod profile_store;
mod profile_write;
mod profiles;
mod repo;
mod row;
mod sample;
mod sample_data;
mod store;
mod values;

pub use anchor::Anchor;
pub use catalog_file::{read_type_catalog, type_catalog_bytes, type_registry};
pub use describe::{DESCRIBES, PROFILE, PROGRAM, ProfileType};
pub use list::{Listing, list_page, list_total};
pub use owner::StoreOwner;
pub use person_repo::PersonRepo;
pub use person_store::PersonStore;
pub use preferred::{COLUMNS, preferred};
pub use profile::{CONTENT_TYPE, Profile, TYPE as PROFILE_TYPE, UNLISTED_KINDS};
pub use profile_store::StoreProfiles;
pub use profile_write::{NewProfile, ProfilePatch, ProfileWrite};
pub use profiles::ProfileRepo;
pub use repo::{Created, MeRepo};
pub use row::{Row, State};
pub use sample::SampleRepo;
pub use sample_data::{sample_anchor, sample_profiles};
pub use store::StoreMe;
pub use values::values_of;
