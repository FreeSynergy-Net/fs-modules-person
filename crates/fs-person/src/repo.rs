//! The port of the module: read and create "me".
//!
//! Pattern: Repository – the module sees only this port; the store behind it (`StoreMe`) can be exchanged.

use async_trait::async_trait;
use fs_objects::Result;

use crate::Anchor;

/// What `create_me` answers: the anchor, and whether this call made it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Created {
    /// The one person of this installation.
    pub anchor: Anchor,
    /// `true` if this call created it, `false` if it was there already.
    pub new: bool,
}

/// Pattern: Repository – reads and creates the one person of this installation; the store is an adapter behind it.
#[async_trait]
pub trait MeRepo: Send + Sync {
    /// The anchor, or `None` if "me" is not created yet.
    ///
    /// # Errors
    /// The store fails, or it holds more than one person (a broken store, never guessed).
    async fn me(&self) -> Result<Option<Anchor>>;

    /// Create "me" once. If it exists, the same anchor comes back with `new = false` and nothing is written.
    ///
    /// # Errors
    /// The store fails.
    async fn create_me(&self) -> Result<Created>;
}

#[async_trait]
impl<R: MeRepo + ?Sized> MeRepo for &R {
    async fn me(&self) -> Result<Option<Anchor>> {
        (**self).me().await
    }
    async fn create_me(&self) -> Result<Created> {
        (**self).create_me().await
    }
}
