//! The records the module draws: the anchor as the form `me`, and the profiles as rows (prepared for the lists of M3).
//!
//! Pattern: Adapter – the values of the module (`fs_person`) as the `RecordView` of the contract. The profile rows are a Projection
//! of `fs_person::Row` (id, label, state `me`/`account`/`contact`); nothing is stored, and a row is a profile, never a second
//! person. No form serves them yet: `catalog/forms.json` gets `rows` and `single-record` in M3.

use fs_interfaces::module::{RecordView, Value};
use fs_person::{Anchor, Profile, Row, State};

/// The form `me`: one record, the anchor (`id`, `type`). It has nothing to run: the set offers `me.create` while there is no "me".
#[must_use]
pub fn me_record(anchor: &Anchor) -> RecordView {
    let mut view = RecordView::default();
    view.id = anchor.urn();
    view.values = vec![
        Value::new("id", &anchor.urn()),
        Value::new("type", "Person"),
    ];
    view
}

/// The word of a state in the rows (`me`, `account`, `contact`, as in ENTSCHEID person-frontend §1).
#[must_use]
pub fn state_word(state: State) -> &'static str {
    match state {
        State::Me => "me",
        State::Account => "account",
        State::Contact => "contact",
    }
}

/// The rows of `profiles`: id (`urn:uuid:` of the profile), `label` (display name) and `state` (from `describes` and `me`).
#[must_use]
pub fn profile_records(profiles: &[Profile], me: Option<&Anchor>) -> Vec<RecordView> {
    profiles
        .iter()
        .map(|p| {
            let row = Row::project(p, me);
            let mut view = RecordView::default();
            view.id = format!("urn:uuid:{}", row.id);
            view.values = vec![
                Value::new("label", &row.label),
                Value::new("state", state_word(row.state)),
            ];
            view
        })
        .collect()
}
