//! The adapter of [`ProfileRepo`] over an fs-objects store.
//!
//! Pattern: Adapter – `ProfileRepo` on top of `fs_objects::Store`: the AS2 `Profile` envelope with the JSContact card as
//! content becomes a [`Profile`]. Reading half of `fs-people-core` `repo/envelope.rs`; nothing of the program database
//! of fs-people (index fields, import, roles) is taken over. The steps are free functions over `&S`, shared with `PersonStore`.

use async_trait::async_trait;
use fs_objects::{ObjectId, Order, Query, Result, Store};

use crate::profile::TYPE;
use crate::{Profile, ProfileRepo};

/// The profiles in an fs-objects store: the objects of type `Profile`.
pub struct StoreProfiles<S>(pub S);

fn profiles() -> Query {
    Query::new().of_type(TYPE)
}

pub(crate) async fn page_in<S: Store + ?Sized>(
    store: &S,
    offset: usize,
    limit: usize,
) -> Result<Vec<Profile>> {
    let mut q = profiles().ordered(Order::NameAsc);
    q.offset = offset;
    q.limit = Some(limit);
    store
        .query(&q)
        .await?
        .iter()
        .map(Profile::from_object)
        .collect()
}

pub(crate) async fn get_in<S: Store + ?Sized>(store: &S, id: ObjectId) -> Result<Option<Profile>> {
    match store.get(id).await? {
        Some(o) if o.envelope.object_type == TYPE => Profile::from_object(&o).map(Some),
        _ => Ok(None),
    }
}

pub(crate) async fn count_in<S: Store + ?Sized>(store: &S) -> Result<usize> {
    store.count(&profiles()).await
}

#[async_trait]
impl<S: Store> ProfileRepo for StoreProfiles<S> {
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
