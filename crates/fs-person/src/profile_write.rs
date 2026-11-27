//! Writing profiles: make one, change one, delete one.
//!
//! Pattern: Repository (the write side of the port [`ProfileRepo`](crate::ProfileRepo)) – [`ProfileWrite`] is the port, the
//! free steps `create_in`, `update_in` and `delete_in` over `&S` are shared by every adapter over an fs-objects store (as
//! `profile_store` does for reading). Deviation: none. Deleting is `Write::Delete` of the store, nothing of our own; a profile
//! is never a person, so none of these steps can touch "me" (a step on the id of the anchor finds no profile).

use async_trait::async_trait;
use fs_lib_jscontact::Card;
use fs_lib_jscontact::validate::validate;
use fs_objects::{
    By, Envelope, Error, ObjectId, Outcome, Result, Store, StoredObject, Timestamp, Write,
};

use crate::Profile;
use crate::profile::{CONTENT_TYPE, TYPE};

/// What a new profile (a contact without an account) is made of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewProfile {
    /// The display name (`name.full`); not empty after trimming.
    pub name: String,
    /// One e-mail address, if given.
    pub email: Option<String>,
    /// One phone number, if given.
    pub phone: Option<String>,
    /// Whom the profile describes (an IRI such as `urn:uuid:...`), or `None` for a written-down contact.
    pub describes: Option<String>,
}

impl NewProfile {
    /// A new contact with `name` and nothing else.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

/// A change of a card: the fields that are `Some` are set, the others stay. For `email` and `phone` an empty text removes the
/// field. Only the FIRST e-mail and the FIRST phone (the first id of the card) are the field here; further ones stay as they are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfilePatch {
    /// The new display name; not empty after trimming.
    pub name: Option<String>,
    /// The new first e-mail address; empty = remove it.
    pub email: Option<String>,
    /// The new first phone number; empty = remove it.
    pub phone: Option<String>,
}

impl ProfilePatch {
    /// Whether the patch changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.email.is_none() && self.phone.is_none()
    }
}

/// Pattern: Repository (write side) – makes, changes and deletes profiles; the store is an adapter behind it. `by` names the actor
/// and the owner of what is written.
#[async_trait]
pub trait ProfileWrite: Send + Sync {
    /// Make a profile with a fresh id.
    ///
    /// # Errors
    /// `Invalid` for an empty name or a card that does not validate; the store fails.
    async fn create_profile(&self, by: &By, new: NewProfile) -> Result<Profile>;

    /// Change the profile `id`; `None` if there is no profile with that id (nothing is written).
    ///
    /// # Errors
    /// `Invalid` for an empty patch, an empty name or a card that does not validate; the store fails.
    async fn update_profile(
        &self,
        by: &By,
        id: ObjectId,
        patch: ProfilePatch,
    ) -> Result<Option<Profile>>;

    /// Delete the profile `id`; `false` if there is no profile with that id (nothing is deleted, whatever else has that id).
    ///
    /// # Errors
    /// The store fails.
    async fn delete_profile(&self, by: &By, id: ObjectId) -> Result<bool>;
}

#[async_trait]
impl<R: ProfileWrite + ?Sized> ProfileWrite for &R {
    async fn create_profile(&self, by: &By, new: NewProfile) -> Result<Profile> {
        (**self).create_profile(by, new).await
    }
    async fn update_profile(
        &self,
        by: &By,
        id: ObjectId,
        patch: ProfilePatch,
    ) -> Result<Option<Profile>> {
        (**self).update_profile(by, id, patch).await
    }
    async fn delete_profile(&self, by: &By, id: ObjectId) -> Result<bool> {
        (**self).delete_profile(by, id).await
    }
}

fn utc(t: &Timestamp) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn checked(card: &Card) -> Result<()> {
    validate(card).map_err(|e| Error::Invalid(format!("the card is not valid: {e}")))
}

fn done(outcome: Outcome) -> Result<()> {
    match outcome {
        Outcome::Done(_) => Ok(()),
        other => Err(Error::Invalid(format!(
            "the write did not complete: {other:?}"
        ))),
    }
}

fn name_of(text: &str) -> Result<&str> {
    match text.trim() {
        "" => Err(Error::Invalid("the name of a profile is empty".into())),
        name => Ok(name),
    }
}

pub(crate) async fn create_in<S: Store + ?Sized>(
    store: &S,
    by: &By,
    new: NewProfile,
) -> Result<Profile> {
    let name = name_of(&new.name)?;
    let mut e = Envelope::new(TYPE, name);
    e.describes = new.describes.clone();
    let mut card = Card::individual(name, format!("urn:uuid:{}", e.id), &utc(&e.published));
    for email in new.email.iter().filter(|t| !t.trim().is_empty()) {
        card.add_email(email.trim());
    }
    for phone in new.phone.iter().filter(|t| !t.trim().is_empty()) {
        card.add_phone(phone.trim());
    }
    checked(&card)?;
    let object = StoredObject::new(e, CONTENT_TYPE, card.to_value());
    let made = Profile::from_object(&object)?;
    let write = Write::Put {
        object,
        expect: None,
    };
    done(store.write_by(by, vec![write]).await?)?;
    Ok(made)
}

/// The first value of a card's map by key order (`e1`, `e2` ...), as the field of the patch.
fn first_key<T>(map: &fs_lib_jscontact::card::Map<T>) -> Option<String> {
    map.keys().next().cloned()
}

fn apply(card: &mut Card, patch: &ProfilePatch, now: &str) -> Result<()> {
    if let Some(name) = &patch.name {
        let name = name_of(name)?;
        let n = card.name.get_or_insert_with(Default::default);
        n.full = Some(name.to_owned());
        n.components.clear();
    }
    if let Some(email) = &patch.email {
        match (first_key(&card.emails), email.trim()) {
            (Some(k), "") => {
                card.emails.remove(&k);
            }
            (Some(k), text) => {
                if let Some(e) = card.emails.get_mut(&k) {
                    text.clone_into(&mut e.address);
                }
            }
            (None, "") => {}
            (None, text) => {
                card.add_email(text);
            }
        }
    }
    if let Some(phone) = &patch.phone {
        match (first_key(&card.phones), phone.trim()) {
            (Some(k), "") => {
                card.phones.remove(&k);
            }
            (Some(k), text) => {
                if let Some(p) = card.phones.get_mut(&k) {
                    text.clone_into(&mut p.number);
                }
            }
            (None, "") => {}
            (None, text) => {
                card.add_phone(text);
            }
        }
    }
    card.updated = Some(now.to_owned());
    checked(card)
}

pub(crate) async fn update_in<S: Store + ?Sized>(
    store: &S,
    by: &By,
    id: ObjectId,
    patch: ProfilePatch,
) -> Result<Option<Profile>> {
    if patch.is_empty() {
        return Err(Error::Invalid("the change names no field".into()));
    }
    let Some(mut object) = store.get(id).await? else {
        return Ok(None);
    };
    if object.envelope.object_type != TYPE {
        return Ok(None);
    }
    let mut profile = Profile::from_object(&object)?;
    let now: Timestamp = std::time::SystemTime::now().into();
    apply(&mut profile.card, &patch, &utc(&now))?;
    object.envelope.name = profile.card.display_name();
    object.content = profile.card.to_value();
    let changed = Profile::from_object(&object)?;
    let write = Write::Put {
        object,
        expect: None,
    };
    done(store.write_by(by, vec![write]).await?)?;
    Ok(Some(changed))
}

pub(crate) async fn delete_in<S: Store + ?Sized>(store: &S, by: &By, id: ObjectId) -> Result<bool> {
    match store.get(id).await? {
        Some(o) if o.envelope.object_type == TYPE => {
            done(
                store
                    .write_by(by, vec![Write::Delete { id, expect: None }])
                    .await?,
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
