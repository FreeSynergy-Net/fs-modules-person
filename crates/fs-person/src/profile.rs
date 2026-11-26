//! A profile: a contact card that describes (or does not describe) a person.
//!
//! Pattern: Value Object – a profile is its id and its card; the card is the RFC 9553 wire form (fs-lib-jscontact), the
//! `describes` link is the AS2 `Profile.describes` of the envelope. A profile is never a person (CLAUDE.md §8, ENTSCHEID
//! person-frontend §2): the only person is the anchor.

use fs_lib_jscontact::Card;
use fs_objects::{Error, ObjectId, Result, StoredObject};

use crate::describe::DESCRIBES;
use crate::values::values_of;

/// The AS2 type of a stored profile.
pub const TYPE: &str = "Profile";

/// The media type of its content: the JSContact card as JSON.
pub const CONTENT_TYPE: &str = "application/jscontact+json";

/// The kinds of card (RFC 9553 section 2.1.4) that are no person and so are not in the list of people.
pub const UNLISTED_KINDS: [&str; 4] = ["group", "org", "location", "device"];

/// One stored profile.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Profile {
    /// The object id of the profile (not of a person).
    pub id: ObjectId,
    /// The card.
    pub card: Card,
    /// Whom it describes (`Profile.describes`, an IRI such as `urn:uuid:…`), or `None` for a written-down contact.
    pub describes: Option<String>,
    /// Whether the envelope marks it sensitive (ADR 0029).
    pub sensitive: bool,
}

impl Profile {
    /// The profile of a stored object.
    ///
    /// # Errors
    /// `Invalid` if the object is no `Profile` or its content is no card (the fault of the data, never guessed).
    pub fn from_object(o: &StoredObject) -> Result<Self> {
        if o.envelope.object_type != TYPE {
            return Err(Error::Invalid(format!(
                "the object is a {}, not a {TYPE}",
                o.envelope.object_type
            )));
        }
        let card: Card = serde_json::from_value(o.content.clone())
            .map_err(|e| Error::Invalid(format!("the content of a profile is no card: {e}")))?;
        Ok(Self {
            id: o.envelope.id,
            card,
            describes: o.envelope.describes.clone(),
            sensitive: !o.envelope.sensitive.is_empty(),
        })
    }

    /// Whether the list of PEOPLE shows this profile (ENTSCHEID person-frontend section 2): a card of the kind `group`, `org`,
    /// `location` or `device` is not a person (groups have one source, `fs-groups`); `application` (an AI, ADR 0091) and every
    /// other kind, or no kind, is listed. The profile stays readable by its id.
    #[must_use]
    pub fn listed(&self) -> bool {
        !self
            .card
            .kind
            .as_deref()
            .is_some_and(|k| UNLISTED_KINDS.contains(&k))
    }

    /// The values of a field as a form shows them: the table of the type (`describes` is the one field that is not on the
    /// card). `None` for a name that is no field.
    #[must_use]
    pub fn values(&self, field: &str) -> Option<Vec<String>> {
        if field == DESCRIBES {
            return Some(self.describes.iter().cloned().collect());
        }
        values_of(&self.card, field)
    }
}
