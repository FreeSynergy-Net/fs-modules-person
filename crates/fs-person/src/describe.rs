//! The description of a profile: what a generated frontend is told (ADR 0066).
//!
//! Pattern: Registry entry (trait first) – [`ProfileType`] implements `DescribesType` from `fs-types`, so
//! `TypeRegistry::register::<ProfileType>()` puts it in the catalog the module serves. Fields come from the table in
//! `fields.rs`, labels are `person.<field>`.
//!
//! Differences to `fs-people-core` `person/describe.rs` (copy): (1) a newtype, because `Card` (fs-lib-jscontact) and
//! `DescribesType` (fs-types) are both foreign here (orphan rule); (2) the type is called `Profile` (what is stored,
//! ADR 0104), owner program `fs-person`; (3) no commands (create, change, find, delete, check, suggest are write steps
//! or belong to M2/M3) and no kind selectors; (4) one more field, `describes`, a read-only reference to the person.

use fs_types::{BasicType, DescribesType, FieldDescription, TypeDescription};

use crate::fields::{self, Field};

/// The name of the type in the catalog (the AS2 type of what is stored).
pub const PROFILE: &str = "Profile";

/// The module that owns it (ADR 0050).
pub const PROGRAM: &str = "fs-person";

/// The field that says whom the profile describes (AS2 `Profile.describes`); not part of the card, it is in the envelope.
pub const DESCRIBES: &str = "describes";

/// Set by the module itself, shown and never edited (`FieldDescription::read_only`).
const READ_ONLY: [&str; 5] = ["uid", "created", "updated", "prodId", DESCRIBES];

/// `emailDomain` -> `email-domain`: the translation key part of a property name.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn describe_field(f: &Field) -> FieldDescription {
    let mut d = FieldDescription::new(f.name, f.basic).label(&format!("person.{}", kebab(f.name)));
    if f.multiple {
        d = d.multiple();
    }
    if READ_ONLY.contains(&f.name) {
        d = d.read_only();
    }
    if !f.target.is_empty() {
        d = d.target(f.target);
    }
    if !f.choices.is_empty() {
        d = d.choices(f.choices);
    }
    d
}

/// The profile as a data type: its fields are the properties of the RFC 9553 card, plus `describes`.
///
/// Not described (no basic type for it yet): `localizations`, and the members of a sub-object that are no basic type
/// (`contexts`, `pref`, `label`, `features`, `mediaType`).
pub struct ProfileType;

impl DescribesType for ProfileType {
    fn describe() -> TypeDescription {
        let mut t = TypeDescription::new(PROFILE, PROGRAM, "RFC 9553 Card", "person");
        for f in &fields::table() {
            t = t.field(describe_field(f));
        }
        t.field(
            FieldDescription::new(DESCRIBES, BasicType::Reference)
                .label(&format!("person.{DESCRIBES}"))
                .target("Person")
                .read_only(),
        )
    }
}
