//! The row of the list: a profile as the list shows it, and whom it hangs on.
//!
//! Pattern: Projection – profile -> row, nothing is stored. The state comes from `describes` alone: there is no person
//! record next to the anchor, so a row never is a second person.

use fs_objects::ObjectId;

use crate::preferred::{COLUMNS, preferred};
use crate::{Anchor, Profile, values_of};

/// Whom a profile hangs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// It describes the anchor, the one person of this installation.
    Me,
    /// It describes another (real) account.
    Account,
    /// It describes nobody: a written-down contact.
    Contact,
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Row {
    /// The id of the profile.
    pub id: ObjectId,
    /// The display name of the card (ADR 0032, never empty).
    pub label: String,
    /// Whom the profile hangs on.
    pub state: State,
    /// The first picture of the card (`media`, no sound, no `data:` file), the `preview` of the form.
    pub preview: Option<String>,
    /// The columns the card has a value for, as (field, the one value), in the order of [`COLUMNS`].
    pub columns: Vec<(&'static str, String)>,
}

impl State {
    /// The state of a profile: `Me` if it describes the anchor, `Account` if it describes anything else, else `Contact`.
    /// Without an anchor nothing is `Me`.
    #[must_use]
    pub fn of(profile: &Profile, me: Option<&Anchor>) -> Self {
        match (&profile.describes, me) {
            (None, _) => Self::Contact,
            (Some(d), Some(anchor)) if *d == anchor.urn() => Self::Me,
            (Some(_), _) => Self::Account,
        }
    }
}

impl Row {
    /// The row of a profile.
    #[must_use]
    pub fn project(profile: &Profile, me: Option<&Anchor>) -> Self {
        Self {
            id: profile.id,
            label: profile.card.display_name(),
            state: State::of(profile, me),
            preview: values_of(&profile.card, "media").and_then(|v| v.into_iter().next()),
            columns: COLUMNS
                .iter()
                .filter_map(|c| preferred(&profile.card, c).map(|v| (*c, v)))
                .collect(),
        }
    }
}
