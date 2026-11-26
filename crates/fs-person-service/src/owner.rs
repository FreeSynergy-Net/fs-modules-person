//! "Me" as the person of every connection: the anchor, named before it is born.
//!
//! Pattern: Strategy (`PersonOfConnection`) + shared state – the module is the source of "me", so there is no `LocalOwner` to ask
//! (it would ask this very service). Every connection the transport admits acts for the one person of this installation:
//! the anchor if it exists, else the anchor that `me.create` will birth (made once, here, and handed to the birth with
//! `MeRepo::create_me_as`). Deviation: before the birth the person of a context is a person that is not stored yet; it is no
//! placeholder, because the birth uses exactly this anchor, and nothing is written for it until `me.create`.

use std::sync::{Arc, Mutex, PoisonError};

use fs_person::{Anchor, MeRepo};
use fs_rpc::{Caller, PersonOfConnection};
use fs_types::PersonRef;

/// The anchor of this installation: the stored one, or the one reserved for the birth. Cheap to clone (all clones share it).
#[derive(Clone, Debug)]
pub struct Owner(Arc<Mutex<Anchor>>);

impl Owner {
    /// Read "me" from `repo` ONCE (before the loop serves): the stored anchor, or a fresh one reserved for `me.create`.
    ///
    /// # Errors
    /// The store fails, or it holds something that is no anchor (the service then does not start).
    pub async fn read<R: MeRepo + ?Sized>(repo: &R) -> fs_objects::Result<Self> {
        let anchor = repo.me().await?.unwrap_or_default();
        Ok(Self(Arc::new(Mutex::new(anchor))))
    }

    fn anchor(&self) -> Anchor {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The anchor to hand to `create_me_as`.
    pub(crate) fn reserved(&self) -> Anchor {
        self.anchor()
    }

    /// The anchor the store says is "me" after `create_me_as` (another handle may have been first): new connections act for it.
    pub(crate) fn settle(&self, stored: &Anchor) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = stored.clone();
    }

    /// The person every connection acts for.
    #[must_use]
    pub fn person(&self) -> PersonRef {
        self.anchor().person_ref()
    }
}

impl PersonOfConnection for Owner {
    fn person(&self, _: &Caller) -> Option<PersonRef> {
        Some(Owner::person(self))
    }
}
