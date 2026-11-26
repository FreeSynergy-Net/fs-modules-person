//! The ONE value a list column shows: the preferred one of a field, else the first.
//!
//! Pattern: Strategy table reader – the same field names as `values_of`; only the five columns of the list have a rule. E-mail,
//! phone and nickname carry `pref` (RFC 9553 section 1.4.4: 1 is the most preferred); an organisation or a title has none, so the
//! first (the map is ordered by id) counts. Nothing is joined: a column shows one value, the whole set is the single record's.

use fs_lib_jscontact::card::{Card, Map};

use crate::values_of;

/// The columns of the list, in the order the form `rows` names them.
pub const COLUMNS: [&str; 5] = ["organizations", "titles", "emails", "phones", "nicknames"];

fn best<T>(
    items: &Map<T>,
    pref: impl Fn(&T) -> Option<u32>,
    text: impl Fn(&T) -> Option<String>,
) -> Option<String> {
    items
        .values()
        .filter_map(|o| text(o).map(|t| (pref(o).unwrap_or(u32::MAX), t)))
        .min_by_key(|(p, _)| *p)
        .map(|(_, t)| t)
}

/// The one value of `field` a column shows, or `None` if the card has none (or `field` is no column).
#[must_use]
pub fn preferred(card: &Card, field: &str) -> Option<String> {
    match field {
        "emails" => best(&card.emails, |o| o.pref, |o| Some(o.address.clone())),
        "phones" => best(&card.phones, |o| o.pref, |o| Some(o.number.clone())),
        "nicknames" => best(&card.nicknames, |o| o.pref, |o| Some(o.name.clone())),
        "organizations" | "titles" => values_of(card, field)?.into_iter().next(),
        _ => None,
    }
}
