//! The adapter of [`MeRepo`] over an fs-objects store.
//!
//! Pattern: Adapter – `MeRepo` on top of `fs_objects::Store`; "me" is the single `Person` object, written by `birth`.

use async_trait::async_trait;
use fs_objects::{By, Error, Outcome, Query, Result, Store};

use crate::{Anchor, Created, MeRepo};

/// "Me" in an fs-objects store: the one `Person` object, owned by itself.
pub struct StoreMe<S>(pub S);

#[async_trait]
impl<S: Store> MeRepo for StoreMe<S> {
    async fn me(&self) -> Result<Option<Anchor>> {
        let found = self
            .0
            .query(&Query::new().of_type(crate::anchor::TYPE))
            .await?;
        match found.as_slice() {
            [] => Ok(None),
            [one] => Anchor::from_object(one)
                .map(Some)
                .ok_or_else(|| Error::Invalid("the person in the store is not an anchor".into())),
            _ => Err(Error::Invalid(
                "the store holds more than one person".into(),
            )),
        }
    }

    async fn create_me_as(&self, anchor: Anchor) -> Result<Created> {
        if let Some(found) = self.me().await? {
            return Ok(Created {
                anchor: found,
                new: false,
            });
        }
        let by = By::new(anchor.urn(), anchor.person_ref());
        match self.0.birth(&by, anchor.to_object()).await {
            Ok(Outcome::Done(_)) => Ok(Created { anchor, new: true }),
            Ok(other) => Err(Error::Invalid(format!("birth did not complete: {other:?}"))),
            // Another handle was first: its anchor is the one.
            Err(Error::Invalid(m)) if m.contains("already owned") => {
                let anchor = self.me().await?.ok_or(Error::Invalid(m))?;
                Ok(Created { anchor, new: false })
            }
            Err(e) => Err(e),
        }
    }
}
