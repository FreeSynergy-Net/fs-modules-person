//! The anchor: the one person of this installation.
//!
//! Pattern: Value Object – the anchor is its id (a UUIDv7); two anchors with the same id are the same person.

use fs_objects::{Envelope, ObjectId, StoredObject};
use fs_types::{BasicValue as _, PersonArt, PersonRef, Reference};
use serde_json::json;

/// The AS2 type of the anchor.
pub const TYPE: &str = "Person";

/// The one person of this installation. Nothing describes it; names and addresses are profiles (later).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Anchor {
    /// The id, a UUIDv7 (ADR 0027).
    pub id: ObjectId,
}

impl Anchor {
    /// A new anchor with a fresh id.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: ObjectId::new(),
        }
    }

    /// The anchor of a stored object, or `None` if the object is not a person with that form.
    #[must_use]
    pub fn from_object(o: &StoredObject) -> Option<Self> {
        let form = &o.content;
        let same = form["type"] == TYPE && form["id"] == format!("urn:uuid:{}", o.envelope.id);
        (o.envelope.object_type == TYPE && same).then_some(Self { id: o.envelope.id })
    }

    /// The reference every other module uses: `urn:uuid:<id>`, kind `Person`, no `abzug` (ADR 0085:59).
    ///
    /// # Panics
    /// Never: `urn:uuid:` and a UUID are a valid reference.
    #[must_use]
    pub fn person_ref(&self) -> PersonRef {
        PersonRef {
            id: Reference::parse(&self.urn()).expect("urn:uuid is a valid reference"),
            art: PersonArt::Person,
            abzug: None,
        }
    }

    /// The id as `urn:uuid:<uuid>`.
    #[must_use]
    pub fn urn(&self) -> String {
        format!("urn:uuid:{}", self.id)
    }

    /// The AS2 form: `{"type":"Person","id":"urn:uuid:…"}` and nothing else.
    #[must_use]
    pub fn as2(&self) -> serde_json::Value {
        json!({ "type": TYPE, "id": self.urn() })
    }

    /// The stored object of the anchor (the envelope carries no personal data; its name is the urn).
    #[must_use]
    pub fn to_object(&self) -> StoredObject {
        let mut e = Envelope::new(TYPE, self.urn());
        e.id = self.id;
        StoredObject::new(e, "application/activity+json", self.as2())
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::new()
    }
}
