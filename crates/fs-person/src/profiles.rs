//! The port of the profiles: read them, page by page and one by one.
//!
//! Pattern: Repository – the module sees only this port; the store behind it (`StoreProfiles`) can be exchanged.
//! Reading only: writing profiles comes with the commands (M2).

use async_trait::async_trait;
use fs_objects::{ObjectId, Result};

use crate::Profile;

/// Pattern: Repository – reads the profiles of this installation; the store is an adapter behind it.
#[async_trait]
pub trait ProfileRepo: Send + Sync {
    /// One page of profiles in name order (case-insensitive, ties by id), starting after `offset` profiles.
    ///
    /// # Errors
    /// The store fails, or a stored profile does not read as a card (a broken store, never skipped silently).
    async fn page(&self, offset: usize, limit: usize) -> Result<Vec<Profile>>;

    /// One profile by its id, or `None` if there is none with that id (or it is not a profile).
    ///
    /// # Errors
    /// The store fails, or the object does not read as a card.
    async fn get(&self, id: ObjectId) -> Result<Option<Profile>>;

    /// How many profiles there are.
    ///
    /// # Errors
    /// The store fails.
    async fn count(&self) -> Result<usize>;
}

#[async_trait]
impl<R: ProfileRepo + ?Sized> ProfileRepo for &R {
    async fn page(&self, offset: usize, limit: usize) -> Result<Vec<Profile>> {
        (**self).page(offset, limit).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Profile>> {
        (**self).get(id).await
    }
    async fn count(&self) -> Result<usize> {
        (**self).count().await
    }
}
