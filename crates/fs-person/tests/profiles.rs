//! M1: reading profiles from a real fs-objects store (in memory), with fixed test data (no clock, no randomness): the
//! RFC 9553 cards of the standard text and two more made with the library. One contact, two profiles of the anchor, one of
//! another account. The list never shows a second person.

use fs_lib_jscontact::Card;
use fs_objects::{By, Envelope, Error, ObjectId, Query, Store, StoredObject, TursoStore, Write};
use fs_person::{
    Anchor, CONTENT_TYPE, DESCRIBES, MeRepo, PROFILE_TYPE, ProfileRepo, ProfileType, Row, State,
    StoreMe, StoreProfiles,
};
use fs_types::DescribesType as _;
use serde_json::json;

const NOW: &str = "2026-11-25T10:00:00Z";
const OTHER_ACCOUNT: &str = "urn:uuid:01900000-0000-7000-8000-0000000000aa";

/// The card of RFC 9553 section 2.2 / 2.3 examples: full name, e-mail with context, phone with feature.
fn rfc_public() -> Card {
    serde_json::from_value(json!({
        "@type": "Card", "version": "1.0",
        "uid": "22B2D4E9-5AC6-4F4C-9A0C-8E6A0E1D2C11",
        "kind": "individual",
        "name": {"@type": "Name", "full": "Mr. John Q. Public, Esq."},
        "nicknames": {"n1": {"@type": "Nickname", "name": "Johnny"}},
        "emails": {"e1": {"@type": "EmailAddress", "address": "jqpublic@example.com", "contexts": {"work": true}}},
        "phones": {"p1": {"@type": "Phone", "number": "tel:+1-555-555-5555", "features": {"voice": true}}}
    }))
    .unwrap()
}

/// The card of RFC 9553 with organisation and title (the "Public, Quinn" style example), written down as a contact.
fn rfc_org() -> Card {
    serde_json::from_value(json!({
        "@type": "Card", "version": "1.0",
        "uid": "8626E0D6-CA2D-4E19-8F8B-3E1B8A5C2D33",
        "name": {"@type": "Name", "full": "Anna Beispiel"},
        "organizations": {"o1": {"@type": "Organization", "name": "ABC, Inc."}},
        "titles": {"t1": {"@type": "Title", "name": "Research Scientist", "kind": "title"}}
    }))
    .unwrap()
}

fn made(name: &str, uid: &str, email: &str) -> Card {
    let mut c = Card::individual(name, uid, NOW);
    c.add_email(email);
    c
}

fn id(n: u8) -> ObjectId {
    format!("01900000-0000-7000-8000-0000000000{n:02x}")
        .parse()
        .unwrap()
}

fn profile_object(n: u8, card: &Card, describes: Option<&str>) -> StoredObject {
    let mut e = Envelope::new(PROFILE_TYPE, card.display_name());
    e.id = id(n);
    e.describes = describes.map(str::to_owned);
    StoredObject::new(e, CONTENT_TYPE, card.to_value())
}

struct World {
    repo: StoreProfiles<TursoStore>,
    anchor: Anchor,
}

/// The anchor ("me") and four profiles: Anna (contact), John (describes me), Zoe (describes me), Carl (another account).
async fn world() -> World {
    let me = StoreMe(TursoStore::open_in_memory().await.unwrap());
    let anchor = me.create_me().await.unwrap().anchor;
    let store = me.0;
    let by = By::new(anchor.urn(), anchor.person_ref());
    let urn = anchor.urn();
    let objects = [
        profile_object(1, &rfc_org(), None),
        profile_object(2, &rfc_public(), Some(&urn)),
        profile_object(
            3,
            &made(
                "Zoe Zander",
                "urn:uuid:01900000-0000-7000-8000-000000000003",
                "zoe@example.org",
            ),
            Some(&urn),
        ),
        profile_object(
            4,
            &made(
                "Carl Konto",
                "urn:uuid:01900000-0000-7000-8000-000000000004",
                "carl@example.org",
            ),
            Some(OTHER_ACCOUNT),
        ),
    ];
    let writes = objects
        .into_iter()
        .map(|object| Write::Put {
            object,
            expect: None,
        })
        .collect();
    store.write_by(&by, writes).await.unwrap();
    World {
        repo: StoreProfiles(store),
        anchor,
    }
}

async fn all_rows(w: &World) -> Vec<Row> {
    let n = w.repo.count().await.unwrap();
    let profiles = w.repo.page(0, n).await.unwrap();
    profiles
        .iter()
        .map(|p| Row::project(p, Some(&w.anchor)))
        .collect()
}

async fn persons(w: &World) -> usize {
    w.repo
        .0
        .count(&Query::new().of_type("Person"))
        .await
        .unwrap()
}

/// The page comes back in name order with the cards as stored; `get` gives the same profile by id.
#[tokio::test]
async fn pages_and_single_reads_come_from_the_store() {
    let w = world().await;
    let repo = &w.repo;
    assert_eq!(repo.count().await.unwrap(), 4);
    let page = repo.page(0, 10).await.unwrap();
    let names: Vec<String> = page.iter().map(|p| p.card.display_name()).collect();
    assert_eq!(
        names,
        [
            "Anna Beispiel",
            "Carl Konto",
            "Mr. John Q. Public, Esq.",
            "Zoe Zander"
        ]
    );
    assert_eq!(page[2].card, rfc_public());
    let again = repo.get(id(2)).await.unwrap().unwrap();
    assert_eq!(again, page[2]);
    assert_eq!(repo.get(id(99)).await.unwrap(), None);
}

#[tokio::test]
async fn a_page_is_a_window_of_the_order() {
    let w = world().await;
    let repo = &w.repo;
    let second = repo.page(1, 2).await.unwrap();
    let ids: Vec<ObjectId> = second.iter().map(|p| p.id).collect();
    assert_eq!(ids, [id(4), id(2)]);
    assert_eq!(repo.page(4, 5).await.unwrap().len(), 0);
}

/// The anchor is no profile: `get` of its id says `None`, and it is not counted.
#[tokio::test]
async fn the_anchor_is_not_a_profile() {
    let w = world().await;
    let repo = &w.repo;
    assert_eq!(repo.get(w.anchor.id).await.unwrap(), None);
    assert_eq!(repo.count().await.unwrap(), 4);
}

/// `state` comes from `describes` alone: the contact hangs on nobody, two profiles hang on the anchor, one on an account.
#[tokio::test]
async fn the_state_is_me_account_or_contact() {
    let w = world().await;
    let rows = all_rows(&w).await;
    let state = |n: u8| rows.iter().find(|r| r.id == id(n)).unwrap().state;
    assert_eq!(state(1), State::Contact);
    assert_eq!(state(2), State::Me);
    assert_eq!(state(3), State::Me);
    assert_eq!(state(4), State::Account);
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Anna Beispiel",
            "Carl Konto",
            "Mr. John Q. Public, Esq.",
            "Zoe Zander"
        ]
    );
}

/// Red if the projection makes a second person: reading all rows adds nothing to the store, there is still exactly the
/// one anchor, every `me` row describes exactly it, and no row is `me` for another account or without an anchor.
#[tokio::test]
async fn the_list_never_makes_a_second_person() {
    let w = world().await;
    let before = w.repo.0.count(&Query::default()).await.unwrap();
    assert_eq!(persons(&w).await, 1);
    let rows = all_rows(&w).await;
    assert_eq!(persons(&w).await, 1, "a second person appeared");
    assert_eq!(
        w.repo.0.count(&Query::default()).await.unwrap(),
        before,
        "the projection wrote"
    );
    let repo = &w.repo;
    for row in rows.iter().filter(|r| r.state == State::Me) {
        let p = repo.get(row.id).await.unwrap().unwrap();
        assert_eq!(p.describes.as_deref(), Some(w.anchor.urn().as_str()));
    }
    // Without an anchor nothing is `me`, however the profile is written.
    let p = repo.get(id(2)).await.unwrap().unwrap();
    assert_eq!(Row::project(&p, None).state, State::Account);
    // A profile of another account is never `me` for this anchor.
    let c = repo.get(id(4)).await.unwrap().unwrap();
    assert_eq!(Row::project(&c, Some(&w.anchor)).state, State::Account);
    assert_eq!(rows.len(), 4, "one row per profile, none for the person");
}

#[tokio::test]
async fn the_values_follow_the_field_table() {
    let w = world().await;
    let p = w.repo.get(id(2)).await.unwrap().unwrap();
    assert_eq!(
        p.values("emails"),
        Some(vec!["jqpublic@example.com".to_owned()])
    );
    assert_eq!(p.values("nicknames"), Some(vec!["Johnny".to_owned()]));
    assert_eq!(
        p.values("name"),
        Some(vec!["Mr. John Q. Public, Esq.".to_owned()])
    );
    assert_eq!(p.values("titles"), Some(Vec::new()));
    assert_eq!(p.values(DESCRIBES), Some(vec![w.anchor.urn()]));
    assert_eq!(p.values("no-such-field"), None);
    let org = w.repo.get(id(1)).await.unwrap().unwrap();
    assert_eq!(
        org.values("organizations"),
        Some(vec!["ABC, Inc.".to_owned()])
    );
    assert_eq!(org.values(DESCRIBES), Some(Vec::new()));
}

#[test]
fn the_type_catalog_names_the_labels_person_field() {
    let t = ProfileType::describe();
    assert_eq!(t.name, "Profile");
    assert!(t.commands.is_empty(), "reading only: no commands yet");
    let label = |name: &str| {
        t.fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.label.clone())
    };
    assert_eq!(label("emails").as_deref(), Some("person.emails"));
    assert_eq!(
        label("preferredLanguages").as_deref(),
        Some("person.preferred-languages")
    );
    assert_eq!(label(DESCRIBES).as_deref(), Some("person.describes"));
    assert!(t.fields.iter().all(|f| f.label.starts_with("person.")));
    let describes = t.fields.iter().find(|f| f.name == DESCRIBES).unwrap();
    assert!(describes.read_only);
}

/// A profile whose content is no card is an error, never skipped and never an empty card.
#[tokio::test]
async fn a_broken_profile_is_an_error() {
    let w = world().await;
    let by = By::new(w.anchor.urn(), w.anchor.person_ref());
    let mut e = Envelope::new(PROFILE_TYPE, "broken");
    e.id = id(9);
    let bad = StoredObject::new(e, CONTENT_TYPE, json!({"@type": "Card", "nicknames": 5}));
    w.repo
        .0
        .write_by(
            &by,
            vec![Write::Put {
                object: bad,
                expect: None,
            }],
        )
        .await
        .unwrap();
    let repo = &w.repo;
    assert!(matches!(repo.get(id(9)).await, Err(Error::Invalid(_))));
    assert!(matches!(repo.page(0, 10).await, Err(Error::Invalid(_))));
}
