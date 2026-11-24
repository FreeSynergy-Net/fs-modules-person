//! P1 (`StoreOwner`) to P7 of the Geburtsstein (ENTWURF-stufe3-4 N2): the anchor, `MeRepo` and `StoreOwner` over a real store.

use fs_interfaces::owner::{LocalOwner, OwnerProblem, fake::check};
use fs_objects::{
    ActivityFilter, By, Error, OpenOptions, Outcome, Query, Store, StoredObject, TursoStore, Write,
};
use fs_person::{Anchor, MeRepo, StoreMe, StoreOwner};
use fs_types::PersonArt;

async fn store() -> TursoStore {
    TursoStore::open_in_memory().await.unwrap()
}

fn put(object: StoredObject) -> Write {
    Write::Put {
        object,
        expect: None,
    }
}

fn invalid(r: Result<Outcome, Error>, word: &str) {
    match r {
        Err(Error::Invalid(m)) => assert!(m.contains(word), "{m}"),
        other => panic!("expected Invalid({word}), got {other:?}"),
    }
}

/// P1: the contract check of every `LocalOwner` is green for `StoreOwner`, with and without "me".
#[tokio::test]
async fn p1_store_owner_passes_the_contract_check() {
    let me = StoreMe(store().await);
    assert_eq!(check(&StoreOwner(&me)), Ok(()));
    me.create_me().await.unwrap();
    assert_eq!(check(&StoreOwner(&me)), Ok(()));
}

/// P2: `create_me` makes a `urn:uuid` UUIDv7 of kind `Person`; the AS2 form has the two members and nothing else.
#[tokio::test]
async fn p2_create_me_makes_a_v7_person() {
    let me = StoreMe(store().await);
    let made = me.create_me().await.unwrap();
    assert!(made.new);
    let p = made.anchor.person_ref();
    assert_eq!(p.art, PersonArt::Person);
    assert_eq!(p.abzug, None);
    let urn = made.anchor.urn();
    assert!(urn.starts_with("urn:uuid:"));
    assert_eq!(made.anchor.id.as_uuid().get_version_num(), 7);
    let form = made.anchor.as2();
    assert_eq!(form, serde_json::json!({"type": "Person", "id": urn}));
    assert_eq!(form.as_object().unwrap().len(), 2);
    assert_eq!(me.me().await.unwrap(), Some(made.anchor));
}

/// P3: `create_me` twice gives the same anchor, `new = false` the second time, and the log has exactly one activity.
#[tokio::test]
async fn p3_create_me_twice_is_the_same_anchor_and_one_activity() {
    let me = StoreMe(store().await);
    let first = me.create_me().await.unwrap();
    let again = me.create_me().await.unwrap();
    assert!(first.new && !again.new);
    assert_eq!(first.anchor, again.anchor);
    let log = me.0.activities(&ActivityFilter::default()).await.unwrap();
    assert_eq!(log.len(), 1, "{log:?}");
}

/// P4: the owner of the anchor row is the anchor itself, and the store keeps it as its owner (a reopened file needs no owner).
#[tokio::test]
async fn p4_the_anchor_owns_itself_and_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("person.db");
    let anchor = {
        let me = StoreMe(TursoStore::open(&path).await.unwrap());
        let a = me.create_me().await.unwrap().anchor;
        let row = me.0.get(a.id).await.unwrap().unwrap();
        assert_eq!(row.owner, Some(a.person_ref()));
        a
    };
    // The store owner IS the anchor: opened for another person, it refuses; opened for the anchor, it opens.
    let other = Anchor::new().person_ref();
    let wrong = TursoStore::open_with(&path, OpenOptions::default().with_owner(other)).await;
    assert!(
        matches!(&wrong, Err(Error::Invalid(m)) if m.contains("store owner mismatch")),
        "{:?}",
        wrong.err()
    );
    TursoStore::open_with(
        &path,
        OpenOptions::default().with_owner(anchor.person_ref()),
    )
    .await
    .unwrap();
    let reopened = TursoStore::open(&path).await.unwrap();
    let by = By::new(anchor.urn(), anchor.person_ref());
    let note = fs_objects::Envelope::new("Note", "n");
    let note = StoredObject::new(note, "application/json", serde_json::json!({}));
    let id = note.envelope.id;
    reopened.write_by(&by, vec![put(note)]).await.unwrap();
    assert_eq!(
        reopened.get(id).await.unwrap().unwrap().owner,
        Some(anchor.person_ref())
    );
}

/// P5 (counter-proof): a second self-owned person, with another uuid, is refused and nothing is written.
#[tokio::test]
async fn p5_a_second_birth_with_another_uuid_is_refused() {
    let me = StoreMe(store().await);
    let first = me.create_me().await.unwrap().anchor;
    let other = Anchor::new();
    let by = By::new(other.urn(), other.person_ref());
    invalid(
        me.0.birth(&by, other.to_object()).await,
        "store already owned",
    );
    assert_eq!(me.0.count(&Query::default()).await.unwrap(), 1);
    assert_eq!(me.me().await.unwrap(), Some(first));
}

/// P6: an object whose owner is its own id is refused by `write_by`; self-ownership is `birth` only.
#[tokio::test]
async fn p6_self_ownership_in_write_by_is_refused() {
    let me = StoreMe(store().await);
    let mine = me.create_me().await.unwrap().anchor;
    let other = Anchor::new();
    let by = By::new(mine.urn(), other.person_ref());
    invalid(
        me.0.write_by(&by, vec![put(other.to_object())]).await,
        "self-ownership",
    );
    assert_eq!(me.0.count(&Query::default()).await.unwrap(), 1);
}

/// P7: without "me" `me()` is empty and `StoreOwner` says `NoMe` (with a hint), never a placeholder; a broken store is no "me".
#[tokio::test]
async fn p7_without_me_there_is_no_owner() {
    let me = StoreMe(store().await);
    assert_eq!(me.me().await.unwrap(), None);
    let problem = StoreOwner(&me).owner().unwrap_err();
    assert_eq!(problem.code(), OwnerProblem::NO_ME);
    assert_ne!(problem.reason(), "");
}

#[tokio::test]
async fn a_person_that_is_no_anchor_is_an_error_not_an_owner() {
    let me = StoreMe(store().await);
    let a = Anchor::new();
    let mut bad = a.to_object();
    bad.content = serde_json::json!({"type": "Person", "id": "urn:uuid:other"});
    let by = By::new(a.urn(), a.person_ref());
    me.0.birth(&by, bad).await.unwrap();
    assert!(me.me().await.is_err());
    assert_eq!(
        StoreOwner(&me).owner().unwrap_err().code(),
        OwnerProblem::UNREACHABLE
    );
}
