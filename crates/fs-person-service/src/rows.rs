//! The records the module draws: the anchor as the form `me`, and the profiles as rows and as one single record.
//!
//! Pattern: Adapter – the values of the module (`fs_person`) as the `RecordView` of the contract. Both profile views are a
//! Projection (the module sends the superset in standard values, the frontend chooses): `fs_person::Row` for the list (id, label,
//! state `me`/`account`/`contact`, the first picture, one value per column), the field table of the type for the single record
//! (one `Value` per item, the same path repeated). Nothing is stored, and a row is a profile, never a second person.

use fs_interfaces::module::{RecordView, Value};
use fs_person::{Anchor, Profile, ProfileType, Row, State};
use fs_types::DescribesType as _;

/// The picture of a profile (the `preview` channel of both forms).
pub const MEDIA: &str = "media";

/// The fields the single record sends that are not the card's own table row: the table minus `uid` and `prodId` (the module's
/// bookkeeping, ENTSCHEID person-frontend section 1) and minus the picture (it is the `preview`), plus `describes`.
const NOT_SENT: [&str; 3] = ["uid", "prodId", MEDIA];

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

/// The word of a state in the rows (`me`, `account`, `contact`, as in ENTSCHEID person-frontend section 1).
#[must_use]
pub fn state_word(state: State) -> &'static str {
    match state {
        State::Me => "me",
        State::Account => "account",
        State::Contact => "contact",
    }
}

/// The names of the fields the single record sends, in the order of the type: what `catalog/forms.json` lists as `fields`.
#[must_use]
pub fn record_fields() -> Vec<String> {
    ProfileType::describe()
        .fields
        .into_iter()
        .map(|f| f.name)
        .filter(|n| !NOT_SENT.contains(&n.as_str()))
        .collect()
}

fn head(row: &Row) -> RecordView {
    let mut view = RecordView::default();
    view.id = format!("urn:uuid:{}", row.id);
    view.values = vec![
        Value::new("label", &row.label),
        Value::new("state", state_word(row.state)),
    ];
    if let Some(p) = &row.preview {
        view.values.push(Value::new(MEDIA, p));
    }
    view
}

/// The rows of `profiles`: id (`urn:uuid:` of the profile), `label`, `state` (from `describes` and `me`), `media` (the first
/// picture) and the columns the card has a value for (one value each, the preferred one).
#[must_use]
pub fn profile_records(profiles: &[Profile], me: Option<&Anchor>) -> Vec<RecordView> {
    profiles
        .iter()
        .map(|p| {
            let row = Row::project(p, me);
            let mut view = head(&row);
            view.values
                .extend(row.columns.iter().map(|(c, v)| Value::new(c, v)));
            view
        })
        .collect()
}

/// The single record of `profile`: the head of its row, then every field of [`record_fields`] with one `Value` per item, all
/// items of a field under the same path in the order of the card (`emails` twice for two e-mails); a field without value is left out.
#[must_use]
pub fn profile_record(profile: &Profile, me: Option<&Anchor>) -> RecordView {
    let mut view = head(&Row::project(profile, me));
    for field in record_fields() {
        for text in profile.values(&field).unwrap_or_default() {
            view.values.push(Value::new(&field, &text));
        }
    }
    view
}
