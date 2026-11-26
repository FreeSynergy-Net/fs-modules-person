//! The fixed cards of the sample data: built from `fs_person_directory` (the one source of the sample people), not copied.
//!
//! Pattern: Object Mother – one well-known set, built by pure functions: no clock, no randomness, the same ids on every run.
//! Deviation: ids, names, addresses and contact fields (`sample_ref`, `sample_name`, `sample_address`, `sample_card`) are read
//! from the directory sample, so there is ONE Amara for calendar, tasks and people. The directory hands out the card as
//! `ExcerptValue`s (no JSContact), so the RFC 9553 card is built from them, deterministically. Written here is only what the
//! directory does not have: the photo URL of the anchor, John Q. Public (the RFC 9553 example), the place, the device, and the
//! id of a card OF a person (the directory entry of a person is the person itself; the module splits it into the anchor and its
//! cards, so a card of it needs its own id, `card_id`).

use fs_objects::{Envelope, ObjectId, StoredObject};
use fs_person_directory::{ExcerptValue, sample_address, sample_card, sample_name, sample_ref};
use fs_types::{BasicValue as _, ContactValue as _};
use serde_json::{Map, Value, json};

use crate::{Anchor, CONTENT_TYPE, PROFILE_TYPE, Profile};

const CREATED: &str = "2026-11-25T10:00:00Z";

/// The IRI of directory entry `n` (`urn:uuid:00000000-0000-7000-8000-<n>`).
fn iri(n: u8) -> String {
    sample_ref(n)
        .expect("an entry of the directory sample")
        .id
        .canonical()
}

fn object_id(iri: &str) -> ObjectId {
    iri.strip_prefix("urn:uuid:")
        .and_then(|u| u.parse().ok())
        .expect("a urn:uuid")
}

/// The one person of the sample: directory entry 1 (Amara Zolenko, the asker, a real account, art Person). Its id is the IRI
/// the events and tasks samples point at. The directory has no "me" entry of its own: entry 1 is it.
#[must_use]
pub fn sample_anchor() -> Anchor {
    Anchor::with_id(object_id(&iri(1)))
}

/// The id of a card that belongs to a thing the directory has no card entry for (the person of entry `n` = 1, 4, 10, 12, or
/// `n` from 0x20): module-local, so it cannot clash with `sample_ref`.
fn card_id(n: u8) -> ObjectId {
    format!("01900000-0000-7000-8000-0000000000{n:02x}")
        .parse()
        .expect("a fixed UUID")
}

fn card(id: ObjectId, kind: &str, full: &str, rest: &Value) -> Value {
    let mut c = json!({
        "@type": "Card", "version": "1.0", "uid": format!("urn:uuid:{id}"), "kind": kind,
        "created": CREATED, "name": {"@type": "Name", "full": full}
    });
    if let (Some(c), Some(rest)) = (c.as_object_mut(), rest.as_object()) {
        c.extend(rest.clone());
    }
    c
}

/// `{"e1": {...}, "e2": {...}}` with `pref` 1, 2, ... in the order given.
fn prefs(prefix: &str, values: Vec<Value>) -> Value {
    let map: Map<String, Value> = values
        .into_iter()
        .zip(1u32..)
        .map(|(mut v, pref)| {
            v["pref"] = json!(pref);
            (format!("{prefix}{pref}"), v)
        })
        .collect();
    Value::Object(map)
}

/// The contact fields of directory entry `n` as JSContact has them: e-mails (`sample_card`, else `sample_address`), phones,
/// postal address, and the surname as a name component. Exactly what the directory says, nothing more.
fn directory_card(id: ObjectId, kind: &str, n: u8) -> Value {
    let (mut emails, mut phones) = (Vec::new(), Vec::new());
    let (mut rest, mut surname) = (json!({}), None);
    for v in sample_card(n) {
        match v {
            ExcerptValue::Email(e) => emails.push(e.as_str().to_owned()),
            ExcerptValue::Phone(p) => phones.push(p.as_str().to_owned()),
            ExcerptValue::Postal(a) => {
                rest["addresses"] = json!({"a1": {"@type": "Address", "full": a.display()}});
            }
            ExcerptValue::Surname(s) => surname = Some(s.canonical()),
            _ => {}
        }
    }
    if emails.is_empty() {
        emails.extend(
            sample_address(n)
                .and_then(|a| a.strip_prefix("mailto:"))
                .map(str::to_owned),
        );
    }
    if !emails.is_empty() {
        let list = emails
            .iter()
            .map(|a| json!({"@type": "EmailAddress", "address": a}));
        rest["emails"] = prefs("e", list.collect());
    }
    if !phones.is_empty() {
        let list = phones
            .iter()
            .map(|p| json!({"@type": "Phone", "number": format!("tel:{p}"), "features": {"voice": true}}));
        rest["phones"] = prefs("p", list.collect());
    }
    let mut c = card(
        id,
        kind,
        sample_name(n).expect("a name of the directory sample"),
        &rest,
    );
    if let Some(s) = surname {
        c["name"]["components"] =
            json!([{"@type": "NameComponent", "kind": "surname", "value": s}]);
    }
    c
}

fn profile(id: ObjectId, describes: Option<String>, card: &Value) -> Profile {
    let name = card["name"]["full"].as_str().unwrap_or("");
    let mut e = Envelope::new(PROFILE_TYPE, name);
    e.id = id;
    e.describes = describes;
    Profile::from_object(&StoredObject::new(e, CONTENT_TYPE, card.clone()))
        .expect("a fixed card reads")
}

/// The profile that IS directory entry `n` (a profile, a group or an organisation there): its id is `sample_ref(n)`.
fn entry_profile(n: u8, kind: &str, describes: Option<String>) -> Profile {
    let id = object_id(&iri(n));
    profile(id, describes, &directory_card(id, kind, n))
}

/// A card of the person of directory entry `n`: it describes `sample_ref(n)` (the anchor for entry 1).
fn person_card(n: u8, kind: &str, extra: &Value) -> Profile {
    let id = card_id(n);
    let mut c = directory_card(id, kind, n);
    if let (Some(c), Some(extra)) = (c.as_object_mut(), extra.as_object()) {
        c.extend(extra.clone());
    }
    profile(id, Some(iri(n)), &c)
}

/// The eleven profiles of the sample, in no particular order: seven people and four cards that are no people (group,
/// organisation, place, device).
///
/// From the directory: 1 Amara Zolenko (the full card of the anchor, describes it), 2 "Amara Z." (describes entry 1), 3
/// "A. Zolenko" (describes nobody: a contact), 4 Assistent Sol (AI account), 10 Soo-jin Marchetti and 12 Tariq Lindqvist
/// (real accounts), 5 and 6 (group, organisation). Entries 7 (deleted), 8 (no longer shared), 9 (merged) and 11 (only
/// proposed) have no card here. Not in the directory: John Q. Public (contact), the place, the device.
#[must_use]
pub fn sample_profiles() -> Vec<Profile> {
    let none = json!({});
    // The directory has the photo of entry 1 only as a file reference without bytes (`avatar`), so the URL stays a sample literal.
    let photo = json!({"media": {"m1": {"@type": "Media", "kind": "photo", "uri": "https://example.org/sample/amara.jpg"}}});
    let john_id = card_id(0x20);
    let john = card(
        john_id,
        "individual",
        "Mr. John Q. Public, Esq.",
        &json!({
            "nicknames": {"n1": {"@type": "Nickname", "name": "Johnny"}},
            "organizations": {"o1": {"@type": "Organization", "name": "Example Inc."}},
            "titles": {"t1": {"@type": "Title", "name": "Research Scientist", "kind": "title"}},
            "emails": {"e1": {"@type": "EmailAddress", "address": "jqpublic@example.com", "contexts": {"work": true}}},
            "phones": {"p1": {"@type": "Phone", "number": "tel:+1-555-555-5555", "features": {"voice": true}}}
        }),
    );
    let place = card(card_id(0x21), "location", "Rathaus Beispielstadt", &none);
    let device = card(card_id(0x22), "device", "Laptop Amara", &none);
    vec![
        person_card(1, "individual", &photo),
        entry_profile(2, "individual", Some(iri(1))),
        entry_profile(3, "individual", None),
        person_card(4, "application", &none),
        person_card(10, "individual", &none),
        person_card(12, "individual", &none),
        profile(john_id, None, &john),
        entry_profile(5, "group", None),
        entry_profile(6, "org", None),
        profile(card_id(0x21), None, &place),
        profile(card_id(0x22), Some(iri(1)), &device),
    ]
}
