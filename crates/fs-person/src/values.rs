//! The values of a field as a form shows them.
//!
//! Pattern: Strategy table reader – the same rows `describe` uses. Copy of `fs-people-core` `person/values.rs`.

use fs_lib_jscontact::card::Card;

use crate::fields;

/// The values of a field of the description as a form shows them (empty when the card has none, `None` for a name that is
/// no field of the card).
#[must_use]
pub fn values_of(card: &Card, field: &str) -> Option<Vec<String>> {
    fields::table()
        .iter()
        .find(|f| f.name == field)
        .map(|f| (f.read)(card).into_iter().map(|(_, v)| v).collect())
}
