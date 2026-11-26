//! The adapter of [`ProfileRepo`] over an fs-objects store.
//!
//! Pattern: Adapter – `ProfileRepo` on top of `fs_objects::Store`: the AS2 `Profile` envelope with the JSContact card as
//! content becomes a [`Profile`]. Reading half of `fs-people-core` `repo/envelope.rs`; nothing of the program database
//! of fs-people (index fields, import, roles) is taken over.

use async_trait::async_trait;
use fs_objects::{ObjectId, Order, Query, Result, Store};

use crate::profile::TYPE;
use crate::{Profile, ProfileRepo};

/// The profiles in an fs-objects store: the objects of type `Profile`.
pub struct StoreProfiles<S>(pub S);

fn profiles() -> Query {
    Query::new().of_type(TYPE)
}

#[async_trait]
impl<S: Store> ProfileRepo for StoreProfiles<S> {
    async fn page(&self, offset: usize, limit: usize) -> Result<Vec<Profile>> {
        let mut q = profiles().ordered(Order::NameAsc);
        q.offset = offset;
        q.limit = Some(limit);
        self.0
            .query(&q)
            .await?
            .iter()
            .map(Profile::from_object)
            .collect()
    }

    async fn get(&self, id: ObjectId) -> Result<Option<Profile>> {
        match self.0.get(id).await? {
            Some(o) if o.envelope.object_type == TYPE => Profile::from_object(&o).map(Some),
            _ => Ok(None),
        }
    }

    async fn count(&self) -> Result<usize> {
        self.0.count(&profiles()).await
    }
}
