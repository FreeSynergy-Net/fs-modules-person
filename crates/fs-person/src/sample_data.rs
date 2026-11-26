//! The fixed cards of the sample data (names and addresses as in `fs_person_directory::sample`, the shared test data of the
//! consumers; its excerpt (`sample_card`) is no `Card`, so the cards are written here as RFC 9553 JSON).
//!
//! Pattern: Object Mother – one well-known set, built by pure functions: no clock, no randomness, the same ids on every run.
//! Phone numbers are the drama numbers of the Bundesnetzagentur (never assigned to a subscriber), as in the directory sample.

use fs_objects::{Envelope, ObjectId, StoredObject};
use serde_json::{Value, json};

use crate::{Anchor, CONTENT_TYPE, PROFILE_TYPE, Profile};

/// The one person of the sample (the asker of the directory sample: Amara Zolenko).
#[must_use]
pub fn sample_anchor() -> Anchor {
    Anchor::with_id(id(0xa0))
}

/// Another account (entry 10 of the directory sample) the profiles of "account" rows hang on.
const SOO_JIN: &str = "urn:uuid:00000000-0000-7000-8000-00000000000a";
/// The AI account (entry 4 of the directory sample).
const SOL: &str = "urn:uuid:00000000-0000-7000-8000-000000000004";
const CREATED: &str = "2026-11-25T10:00:00Z";

fn id(n: u8) -> ObjectId {
    format!("01900000-0000-7000-8000-0000000000{n:02x}")
        .parse()
        .expect("a fixed UUID")
}

fn card(n: u8, kind: &str, full: &str, rest: &Value) -> Value {
    let mut c = json!({
        "@type": "Card", "version": "1.0", "uid": format!("urn:uuid:{}", id(n)), "kind": kind,
        "created": CREATED, "name": {"@type": "Name", "full": full}
    });
    if let (Some(c), Some(rest)) = (c.as_object_mut(), rest.as_object()) {
        c.extend(rest.clone());
    }
    c
}

fn email(address: &str, pref: u32) -> Value {
    json!({"@type": "EmailAddress", "address": address, "pref": pref})
}

fn phone(number: &str, pref: u32) -> Value {
    json!({"@type": "Phone", "number": number, "features": {"voice": true}, "pref": pref})
}

fn profile(n: u8, describes: Option<&str>, card: &Value) -> Profile {
    let name = card["name"]["full"].as_str().unwrap_or("");
    let mut e = Envelope::new(PROFILE_TYPE, name);
    e.id = id(n);
    e.describes = describes.map(str::to_owned);
    Profile::from_object(&StoredObject::new(e, CONTENT_TYPE, card.clone()))
        .expect("a fixed card reads")
}

/// The ten profiles of the sample, in no particular order: six people (two of them describe the anchor, one an AI account, one
/// another account, two are contacts) and four cards that are no people (group, organisation, place, device).
#[must_use]
pub fn sample_profiles() -> Vec<Profile> {
    let me = sample_anchor().urn();
    let amara = card(
        1,
        "individual",
        "Amara Zolenko",
        &json!({
            "emails": {"e1": email("amara@example.org", 1), "e2": email("amara.zolenko@arbeit.example.org", 2)},
            "phones": {"p1": phone("tel:+493023125000", 1), "p2": phone("tel:+491713920001", 2)},
            "addresses": {"a1": {"@type": "Address", "full": "Musterweg 12\n10115 Berlin"}},
            "media": {"m1": {"@type": "Media", "kind": "photo", "uri": "https://example.org/sample/amara.jpg"}}
        }),
    );
    let short = card(
        2,
        "individual",
        "A. Zolenko",
        &json!({"emails": {"e1": email("amara@example.org", 1)}}),
    );
    let sol = card(
        3,
        "application",
        "Assistent Sol",
        &json!({
            "emails": {"e1": email("assistent.sol@example.org", 1)}
        }),
    );
    let soo = card(
        4,
        "individual",
        "Soo-jin Marchetti",
        &json!({
            "emails": {"e1": email("soojin.marchetti@example.org", 1)}
        }),
    );
    let tariq = card(
        5,
        "individual",
        "Tariq Lindqvist",
        &json!({
            "emails": {"e1": email("tariq.lindqvist@example.org", 1)},
            "phones": {"p1": phone("tel:+493023125002", 1)},
            "organizations": {"o1": {"@type": "Organization", "name": "Tamarind Works Ltd"}},
            "titles": {"t1": {"@type": "Title", "name": "Research Scientist", "kind": "title"}}
        }),
    );
    let john = card(
        6,
        "individual",
        "Mr. John Q. Public, Esq.",
        &json!({
            "nicknames": {"n1": {"@type": "Nickname", "name": "Johnny"}},
            "emails": {"e1": {"@type": "EmailAddress", "address": "jqpublic@example.com", "contexts": {"work": true}}},
            "phones": {"p1": {"@type": "Phone", "number": "tel:+1-555-555-5555", "features": {"voice": true}}}
        }),
    );
    let none = json!({});
    vec![
        profile(1, Some(&me), &amara),
        profile(2, Some(&me), &short),
        profile(3, Some(SOL), &sol),
        profile(4, Some(SOO_JIN), &soo),
        profile(5, None, &tariq),
        profile(6, None, &john),
        profile(7, None, &card(7, "group", "Quillon Team", &none)),
        profile(8, None, &card(8, "org", "Tamarind Works Ltd", &none)),
        profile(
            9,
            None,
            &card(9, "location", "Rathaus Beispielstadt", &none),
        ),
        profile(10, Some(&me), &card(10, "device", "Laptop Amara", &none)),
    ]
}
