//! The whole module over ONE fs-objects store: "me" and the profiles.
//!
//! Pattern: Adapter – [`PersonStore`] is `MeRepo` and `ProfileRepo` on one store. `TursoStore` cannot be cloned, so the two
//! adapters `StoreMe` and `StoreProfiles` cannot each own it; this one owns it once and both ports read the same objects
//! (the anchor and the profiles that describe it). Deviation: none, it only joins the free steps the two adapters also use.

use async_trait::async_trait;
use fs_objects::{ObjectId, Result, Store};

use crate::profile_store::{count_in, get_in, page_in};
use crate::store::{create_in, me_in};
use crate::{Anchor, Created, MeRepo, PersonRepo, Profile, ProfileRepo};

/// "Me" and the profiles in one store.
pub struct PersonStore<S>(pub S);

#[async_trait]
impl<S: Store> MeRepo for PersonStore<S> {
    async fn me(&self) -> Result<Option<Anchor>> {
        me_in(&self.0).await
    }

    async fn create_me_as(&self, anchor: Anchor) -> Result<Created> {
        create_in(&self.0, anchor).await
    }
}

#[async_trait]
impl<S: Store> ProfileRepo for PersonStore<S> {
    async fn page(&self, offset: usize, limit: usize) -> Result<Vec<Profile>> {
        page_in(&self.0, offset, limit).await
    }

    async fn get(&self, id: ObjectId) -> Result<Option<Profile>> {
        get_in(&self.0, id).await
    }

    async fn count(&self) -> Result<usize> {
        count_in(&self.0).await
    }
}

impl<S: Store> PersonRepo for PersonStore<S> {}
