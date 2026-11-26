//! The port of the whole module: "me" and the profiles together.
//!
//! Pattern: Repository – the service sees only this port (`MeRepo` + `ProfileRepo`); a real store (`PersonStore`) or the sample
//! data (`SampleRepo`) stands behind it.

use crate::{MeRepo, ProfileRepo};

/// Pattern: Repository – both ports of the module on one object, so the service holds ONE repository (an fs-objects store cannot
/// be cloned for two adapters).
pub trait PersonRepo: MeRepo + ProfileRepo {
    /// Whether the records are sample data and not a real person's (the service then says so in `hello`, ADR `ProviderKind`).
    fn is_sample(&self) -> bool {
        false
    }
}
