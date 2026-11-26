//! The steps of `PersonCore` called directly (no wrapper): what it answers where it is narrower than the contract, and the
//! projection of profiles to rows that M3 will serve. Real in-memory store, fixed data.
//!
//! Pattern: Scenario test (given the core, when a step is called, then its answer); the rights are not asked here (`foreign.rs`).

use std::sync::Arc;

use fs_interfaces::module::{
    Actor, Applied, Caller, ChangesQuery, Context, Invocation, ModuleCore, PageQuery, StepRef,
};
use fs_objects::{Envelope, StoredObject, TursoStore};
use fs_person::{Anchor, CONTENT_TYPE, MeRepo, PROFILE_TYPE, Profile, StoreMe};
use fs_person_service::{Owner, PersonCore, profile_records, state_word};
use serde_json::json;

async fn core() -> (
    PersonCore<StoreMe<TursoStore>>,
    Context,
    Arc<StoreMe<TursoStore>>,
) {
    let repo = Arc::new(StoreMe(TursoStore::open_in_memory().await.unwrap()));
    let owner = Owner::read(&*repo).await.unwrap();
    let who = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    let cx = Context::from_connection(who, Actor::Person, owner.person());
    (PersonCore::new(repo.clone(), owner), cx, repo)
}

fn call(command: &str, record: Option<&str>) -> Invocation {
    let mut i = Invocation::default();
    command.clone_into(&mut i.command);
    i.record = record.map(str::to_owned);
    i
}

fn query(form: &str) -> PageQuery {
    let mut q = PageQuery::default();
    form.clone_into(&mut q.form);
    q
}

/// `apply` knows `me.create` and nothing else; `revert` is `not-revertible` (422), `changes` is empty, `blob` is `unsupported`.
#[tokio::test(flavor = "current_thread")]
async fn the_core_is_narrower_than_the_contract_and_says_so() {
    let (core, cx, repo) = core().await;
    let made: Applied = core.apply(&cx, call("me.create", None)).await.unwrap();
    assert_eq!(made.text, "me-created");
    assert_eq!(made.record, repo.me().await.unwrap().unwrap().urn());
    assert!(made.change.is_none());
    let again = core.apply(&cx, call("me.create", None)).await.unwrap();
    assert_eq!(
        (again.text.as_str(), again.record),
        ("me-exists", made.record)
    );
    let unknown = core.apply(&cx, call("me.delete", None)).await.unwrap_err();
    assert_eq!(
        (unknown.kind.as_str(), unknown.status),
        ("unknown-command", 404)
    );
    let on_record = core
        .apply(&cx, call("me.create", Some("urn:uuid:x")))
        .await
        .unwrap_err();
    assert_eq!(on_record.kind, "bad-arguments");
    let taken = core
        .revert(&cx, "any", StepRef::new("", ""))
        .await
        .unwrap_err();
    assert_eq!((taken.kind.as_str(), taken.status), ("not-revertible", 422));
    let none = core.changes(&cx, ChangesQuery::default()).await.unwrap();
    assert_eq!(none.changes.len(), 0);
    let blob = core.blob(&cx, "r", "f").await.unwrap_err();
    assert_eq!(blob.kind, "unsupported");
    let form = core.page(&cx, query("rows@1.0.0")).await.unwrap_err();
    assert_eq!(form.kind, "unknown-form", "the lists come with M3");
}

fn profile(n: u8, name: &str, describes: Option<&str>) -> Profile {
    let card = json!({
        "@type": "Card", "version": "1.0", "uid": format!("uid-{n}"),
        "name": {"@type": "Name", "full": name}
    });
    let mut e = Envelope::new(PROFILE_TYPE, name);
    e.id = format!("01900000-0000-7000-8000-0000000000{n:02x}")
        .parse()
        .unwrap();
    e.describes = describes.map(str::to_owned);
    Profile::from_object(&StoredObject::new(e, CONTENT_TYPE, card)).unwrap()
}

/// The rows of profiles: id, label and state from `describes`; with no anchor nothing is `me`; never a second person.
#[test]
fn profiles_become_rows_by_their_state() {
    let me = Anchor::new();
    let profiles = [
        profile(1, "Anna", Some(&me.urn())),
        profile(
            2,
            "Carl",
            Some("urn:uuid:01900000-0000-7000-8000-0000000000aa"),
        ),
        profile(3, "Zoe", None),
    ];
    let rows = profile_records(&profiles, Some(&me));
    let states: Vec<_> = rows
        .iter()
        .map(|r| {
            r.values
                .iter()
                .find(|v| v.path == "state")
                .unwrap()
                .text
                .as_str()
        })
        .collect();
    assert_eq!(states, ["me", "account", "contact"]);
    assert_eq!(rows[0].id, "urn:uuid:01900000-0000-7000-8000-000000000001");
    assert_eq!(rows[2].values[0].text, "Zoe");
    assert!(
        rows.iter().all(|r| r.id != me.urn()),
        "a row is a profile, not the person"
    );
    let without = profile_records(&profiles, None);
    assert!(
        without
            .iter()
            .all(|r| r.values.iter().any(|v| v.path == "state" && v.text != "me"))
    );
    assert_eq!(state_word(fs_person::State::Me), "me");
}
