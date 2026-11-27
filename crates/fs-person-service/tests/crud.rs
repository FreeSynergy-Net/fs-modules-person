//! Create, change and delete of a profile at the service: `person.profile.create`, `person.update`, `person.delete` through a real
//! socket in a temporary folder, every value a text. Each test names what a wrong implementation would get wrong.
//!
//! Pattern: Scenario test (given a store, when the owner or a stranger calls, then the store shows exactly that) + counter-proof
//! (`AllowAll` rules let a stranger through, so the refusals are the rules).
#![cfg(unix)]

mod common;

use std::sync::Arc;

use common::{Answer, Repo, invoke, seeded, serve, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{Ask, Context, HiddenReason, RightsRules, Verdict};
use fs_objects::{ObjectId, TursoStore};
use fs_person::{Anchor, MeRepo, ProfileRepo, SampleRepo};
use fs_person_service::{
    COMMAND_CREATE, COMMAND_DELETE, COMMAND_PROFILE_CREATE, COMMAND_UPDATE, Owner, PersonCore,
    PersonService,
};
use fs_rpc::{AllBots, FixedPerson, OsUserIsPerson, Ports};
use tokio::task::LocalSet;

macro_rules! connected {
    ($source:ident, $repo:ident, $body:block) => {{
        let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
        LocalSet::new()
            .run_until(async {
                let (socket, $repo, _stop) = serve(dir.path()).await;
                let connection = fs_rpc::connect::<form_source::Client>(&socket)
                    .await
                    .unwrap();
                let $source = &connection.client;
                let born = invoke($source, COMMAND_CREATE, "", &[]).await;
                assert!(!born.failed, "{born:?}");
                $body
            })
            .await;
    }};
}

fn id_of(record: &str) -> ObjectId {
    record
        .strip_prefix("urn:uuid:")
        .expect("urn:uuid")
        .parse()
        .unwrap()
}

fn refused(a: &Answer, kind: &str) {
    assert!(a.failed, "the call went through: {a:?}");
    assert_eq!(a.kind, kind, "{a:?}");
}

async fn create(source: &form_source::Client, pairs: &[(&str, &str)]) -> Answer {
    invoke(source, COMMAND_PROFILE_CREATE, "", pairs).await
}

/// The first profile in an empty database (only "me" is there): made with name, e-mail and phone; the answer is its id; "me" stays
/// the one person; the card has what was given and `describes` nothing.
#[tokio::test(flavor = "current_thread")]
async fn the_first_profile_is_made_in_an_empty_database() {
    connected!(source, repo, {
        assert_eq!(repo.count().await.unwrap(), 0);
        let made = create(
            source,
            &[
                ("name", "Ada Lovelace"),
                ("email", "ada@example.org"),
                ("phone", "+44 20 7946 0000"),
            ],
        )
        .await;
        assert!(!made.failed, "{made:?}");
        assert_eq!(made.text, "profile-created");
        let id = id_of(&made.record);
        assert_eq!(
            uuid::Uuid::parse_str(made.record.strip_prefix("urn:uuid:").unwrap())
                .unwrap()
                .get_version_num(),
            7
        );
        assert_eq!(repo.count().await.unwrap(), 1);
        let p = repo.get(id).await.unwrap().expect("the profile is stored");
        assert_eq!(p.card.display_name(), "Ada Lovelace");
        assert_eq!(p.values("emails").unwrap(), ["ada@example.org"]);
        assert_eq!(p.values("phones").unwrap(), ["+44 20 7946 0000"]);
        assert_eq!(p.describes, None, "a contact describes nobody");
        assert_eq!(p.card.uid, made.record);
        // never a second person: "me" is still the one anchor and is not the new id
        let me = repo.me().await.unwrap().unwrap();
        assert_ne!(me.id, id);
    });
}

/// Only the name is needed; a missing, empty or blank name, an argument that is not named, a doubled one and a record are refused, and
/// nothing is written.
#[tokio::test(flavor = "current_thread")]
async fn create_refuses_what_is_not_a_clean_call_and_writes_nothing() {
    connected!(source, repo, {
        let only_name = create(source, &[("name", "Grace")]).await;
        assert!(!only_name.failed, "{only_name:?}");
        let before = repo.count().await.unwrap();
        for bad in [
            vec![],
            vec![("name", "")],
            vec![("name", "   ")],
            vec![("email", "x@example.org")],
            vec![("name", "A"), ("actor", "urn:x")],
            vec![("name", "A"), ("name", "B")],
            vec![
                ("name", "A"),
                ("describes", "urn:uuid:00000000-0000-7000-8000-000000000001"),
            ],
        ] {
            let a = create(source, &bad).await;
            refused(&a, "form-source-bad-arguments");
        }
        let with_record = invoke(
            source,
            COMMAND_PROFILE_CREATE,
            "urn:uuid:1",
            &[("name", "A")],
        )
        .await;
        refused(&with_record, "form-source-bad-arguments");
        assert_eq!(repo.count().await.unwrap(), before, "a refused call wrote");
    });
}

/// `describes=me` makes the profile of the one person; the profile then is what `person.update` without an id changes.
#[tokio::test(flavor = "current_thread")]
async fn describes_me_and_update_without_id_change_that_profile() {
    connected!(source, repo, {
        let none = invoke(source, COMMAND_UPDATE, "", &[("name", "Nobody")]).await;
        refused(&none, "form-source-unknown-record");
        let mine = create(source, &[("name", "Me Myself"), ("describes", "me")]).await;
        assert!(!mine.failed, "{mine:?}");
        let anchor = repo.me().await.unwrap().unwrap();
        let p = repo.get(id_of(&mine.record)).await.unwrap().unwrap();
        assert_eq!(p.describes.as_deref(), Some(anchor.urn().as_str()));
        let changed = invoke(source, COMMAND_UPDATE, "", &[("name", "Me Renamed")]).await;
        assert!(!changed.failed, "{changed:?}");
        assert_eq!(changed.record, mine.record);
        let p = repo.get(id_of(&mine.record)).await.unwrap().unwrap();
        assert_eq!(p.card.display_name(), "Me Renamed");
        // a second profile of me: the call without an id is no longer clear, and it changes neither
        let second = create(source, &[("name", "Me Again"), ("describes", "me")]).await;
        assert!(!second.failed);
        let unclear = invoke(source, COMMAND_UPDATE, "", &[("name", "X")]).await;
        refused(&unclear, "form-source-conflict");
        for r in [&mine.record, &second.record] {
            let p = repo.get(id_of(r)).await.unwrap().unwrap();
            assert_ne!(p.card.display_name(), "X");
        }
    });
}

/// A change sets the fields given and keeps the rest: the id stays, an empty phone removes it, the second e-mail stays, the name and
/// the first e-mail are replaced, `updated` of the card moves on, the count does not change.
#[tokio::test(flavor = "current_thread")]
async fn update_sets_the_given_fields_and_keeps_the_rest() {
    connected!(source, repo, {
        let made = create(
            source,
            &[
                ("name", "Old Name"),
                ("email", "old@example.org"),
                ("phone", "123"),
            ],
        )
        .await;
        let id = id_of(&made.record);
        // a second e-mail made behind the service, as an import would have
        {
            use fs_objects::{By, Store, Write};
            let mut o = repo.0.get(id).await.unwrap().unwrap();
            let mut card = repo.get(id).await.unwrap().unwrap().card;
            card.add_email("second@example.org");
            o.content = card.to_value();
            let a = repo.me().await.unwrap().unwrap();
            let by = By::new(a.urn(), a.person_ref());
            repo.0
                .write_by(
                    &by,
                    vec![Write::Put {
                        object: o,
                        expect: None,
                    }],
                )
                .await
                .unwrap();
        }
        let before = repo.get(id).await.unwrap().unwrap();
        let count = repo.count().await.unwrap();
        let done = invoke(
            source,
            COMMAND_UPDATE,
            "",
            &[
                ("id", &made.record),
                ("name", "New Name"),
                ("email", "new@example.org"),
                ("phone", ""),
            ],
        )
        .await;
        assert!(!done.failed, "{done:?}");
        assert_eq!(done.record, made.record);
        assert_eq!(done.text, "profile-updated");
        let after = repo.get(id).await.unwrap().unwrap();
        assert_eq!(after.card.display_name(), "New Name");
        assert_eq!(
            after.values("emails").unwrap(),
            ["new@example.org", "second@example.org"]
        );
        assert!(
            after.values("phones").unwrap().is_empty(),
            "the empty phone did not remove it"
        );
        assert_eq!(after.card.uid, before.card.uid);
        assert_eq!(repo.count().await.unwrap(), count);
        // only the name given: the e-mails stay as they were
        let only_name = invoke(
            source,
            COMMAND_UPDATE,
            "",
            &[("id", &made.record), ("name", "Third")],
        )
        .await;
        assert!(!only_name.failed, "{only_name:?}");
        let again = repo.get(id).await.unwrap().unwrap();
        assert_eq!(again.card.display_name(), "Third");
        assert_eq!(
            again.values("emails").unwrap(),
            after.values("emails").unwrap()
        );
        // the name shown in the list (the envelope) follows the card
        let listed = repo.page(0, 10).await.unwrap();
        assert_eq!(listed[0].card.display_name(), "Third");
    });
}

/// An empty change, an empty name, an unknown field, a bad id and an id nobody has are refused; the profile stays as it was.
#[tokio::test(flavor = "current_thread")]
async fn update_refuses_an_empty_patch_and_bad_input() {
    connected!(source, repo, {
        let made = create(
            source,
            &[("name", "Keep Me"), ("email", "keep@example.org")],
        )
        .await;
        let id = id_of(&made.record);
        let before = repo.get(id).await.unwrap().unwrap();
        let nobody = "urn:uuid:01900000-0000-7000-8000-0000000000ff";
        for (pairs, kind) in [
            (
                vec![("id", made.record.as_str())],
                "form-source-bad-arguments",
            ),
            (vec![], "form-source-bad-arguments"),
            (
                vec![("id", made.record.as_str()), ("name", " ")],
                "form-source-bad-arguments",
            ),
            (
                vec![("id", made.record.as_str()), ("nickname", "x")],
                "form-source-bad-arguments",
            ),
            (
                vec![("id", "no-uuid"), ("name", "x")],
                "form-source-bad-arguments",
            ),
            (
                vec![("id", nobody), ("name", "x")],
                "form-source-unknown-record",
            ),
        ] {
            refused(&invoke(source, COMMAND_UPDATE, "", &pairs).await, kind);
        }
        let on_record = invoke(source, COMMAND_UPDATE, &made.record, &[("name", "x")]).await;
        refused(&on_record, "form-source-bad-arguments");
        assert_eq!(
            repo.get(id).await.unwrap().unwrap(),
            before,
            "a refused call changed the profile"
        );
    });
}

/// A profile is deleted by its id and is gone; the others stay; a second delete of it finds nothing; no id and a bad id are refused.
#[tokio::test(flavor = "current_thread")]
async fn delete_removes_that_profile_only() {
    connected!(source, repo, {
        let a = create(source, &[("name", "Anna")]).await;
        let b = create(source, &[("name", "Berta")]).await;
        assert_eq!(repo.count().await.unwrap(), 2);
        let gone = invoke(source, COMMAND_DELETE, "", &[("id", &a.record)]).await;
        assert!(!gone.failed, "{gone:?}");
        assert_eq!(gone.text, "profile-deleted");
        assert!(repo.get(id_of(&a.record)).await.unwrap().is_none());
        assert!(repo.get(id_of(&b.record)).await.unwrap().is_some());
        assert_eq!(repo.count().await.unwrap(), 1);
        refused(
            &invoke(source, COMMAND_DELETE, "", &[("id", &a.record)]).await,
            "form-source-unknown-record",
        );
        refused(
            &invoke(source, COMMAND_DELETE, "", &[]).await,
            "form-source-bad-arguments",
        );
        refused(
            &invoke(source, COMMAND_DELETE, "", &[("id", "x")]).await,
            "form-source-bad-arguments",
        );
        refused(
            &invoke(source, COMMAND_DELETE, &b.record, &[("id", &b.record)]).await,
            "form-source-bad-arguments",
        );
        assert_eq!(repo.count().await.unwrap(), 1);
    });
}

/// "Me" is never deleted, by its urn or its bare id; it stays the one person and the profiles stay.
#[tokio::test(flavor = "current_thread")]
async fn the_anchor_is_not_deletable() {
    connected!(source, repo, {
        let p = create(source, &[("name", "Contact"), ("describes", "me")]).await;
        let me = repo.me().await.unwrap().unwrap();
        for id in [me.urn(), me.id.to_string()] {
            let a = invoke(source, COMMAND_DELETE, "", &[("id", &id)]).await;
            refused(&a, "form-source-rejected");
            assert!(a.record.is_empty());
        }
        assert_eq!(repo.me().await.unwrap(), Some(me));
        assert!(repo.get(id_of(&p.record)).await.unwrap().is_some());
    });
}

/// Pattern: Strategy (the wrong variant of `RightsRules`) - says yes to everything.
struct AllowAll;

impl RightsRules for AllowAll {
    fn may(&self, _: &Ask<'_>) -> Verdict {
        Verdict::Allow
    }
    fn hidden(&self, _: &Context, _: &str, _: &str) -> Option<HiddenReason> {
        None
    }
}

#[derive(Clone, Copy)]
enum Stranger {
    Bot,
    OtherPerson,
}

/// What a stranger gets for the three commands over a store with one profile: whether each was refused, and whether the store is as it was.
async fn stranger_runs(stranger: Stranger, lax: bool) -> ([bool; 3], bool) {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let repo: Repo = seeded().await;
    let kept = repo.page(0, 1).await.unwrap().remove(0);
    let owner = Owner::read(&*repo).await.unwrap();
    let ports = match stranger {
        Stranger::Bot => Ports::new(AllBots, owner.clone()),
        Stranger::OtherPerson => {
            Ports::new(OsUserIsPerson, FixedPerson(Anchor::new().person_ref()))
        }
    };
    let core = if lax {
        PersonCore::with_rules(repo.clone(), owner, Box::new(AllowAll))
    } else {
        PersonCore::new(repo.clone(), owner)
    };
    let service = PersonService::guarding(core, ports, |_| {});
    let (socket, _stop) = serve_service(dir.path(), service).await;
    let c = fs_rpc::connect::<form_source::Client>(&socket)
        .await
        .unwrap();
    let s = &c.client;
    let before = repo.count().await.unwrap();
    let kept_id = format!("urn:uuid:{}", kept.id);
    let made = create(s, &[("name", "Intruder")]).await;
    let changed = invoke(
        s,
        COMMAND_UPDATE,
        "",
        &[("id", &kept_id), ("name", "Hacked")],
    )
    .await;
    let deleted = invoke(s, COMMAND_DELETE, "", &[("id", &kept_id)]).await;
    let forbidden = |a: &Answer| a.failed && a.kind == "form-source-forbidden";
    let untouched = repo.count().await.unwrap() == before
        && repo
            .get(kept.id)
            .await
            .unwrap()
            .is_some_and(|p| p.card.display_name() == kept.card.display_name());
    (
        [forbidden(&made), forbidden(&changed), forbidden(&deleted)],
        untouched,
    )
}

#[tokio::test(flavor = "current_thread")]
async fn a_stranger_may_neither_make_nor_change_nor_delete() {
    LocalSet::new()
        .run_until(async {
            for stranger in [Stranger::Bot, Stranger::OtherPerson] {
                let (refused, untouched) = stranger_runs(stranger, false).await;
                assert_eq!(refused, [true; 3], "a command of a stranger got through");
                assert!(untouched, "a refused command wrote");
            }
        })
        .await;
}

/// Counter-proof: with rules that allow everything the same strangers write, so the refusals above come from `PersonRules`.
#[tokio::test(flavor = "current_thread")]
async fn counter_proof_rules_that_allow_everything_let_a_stranger_write() {
    LocalSet::new()
        .run_until(async {
            for stranger in [Stranger::Bot, Stranger::OtherPerson] {
                let (refused, untouched) = stranger_runs(stranger, true).await;
                assert_eq!(refused, [false; 3], "AllowAll refused a command");
                assert!(!untouched, "AllowAll wrote nothing");
            }
        })
        .await;
}

/// The sample data is read only: the owner's three commands are refused with their reason and the data stays.
#[tokio::test(flavor = "current_thread")]
async fn the_sample_data_is_read_only() {
    LocalSet::new()
        .run_until(async {
            let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
            let repo = Arc::new(SampleRepo::new());
            let owner = Owner::read(&*repo).await.unwrap();
            let core = PersonCore::new(repo.clone(), owner.clone());
            let service = PersonService::guarding(core, Ports::new(OsUserIsPerson, owner), |_| {});
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let s = &c.client;
            let before = repo.count().await.unwrap();
            let first = repo.page(0, 1).await.unwrap().remove(0);
            let id = format!("urn:uuid:{}", first.id);
            refused(
                &create(s, &[("name", "New")]).await,
                "form-source-forbidden",
            );
            refused(
                &invoke(s, COMMAND_UPDATE, "", &[("id", &id), ("name", "X")]).await,
                "form-source-forbidden",
            );
            refused(
                &invoke(s, COMMAND_DELETE, "", &[("id", &id)]).await,
                "form-source-forbidden",
            );
            assert_eq!(repo.count().await.unwrap(), before);
            assert_eq!(repo.get(first.id).await.unwrap().unwrap(), first);
            // `me.create` is not touched by this: it still answers "exists"
            let me = invoke(s, COMMAND_CREATE, "", &[]).await;
            assert_eq!(me.text, "me-exists");
        })
        .await;
}

/// A store over a database made by the service itself (the same module, other adapter) behaves the same: `StoreProfiles` is the
/// second adapter of the port.
#[tokio::test(flavor = "current_thread")]
async fn a_second_adapter_of_the_port_does_the_same() {
    use fs_objects::By;
    use fs_person::{NewProfile, ProfilePatch, ProfileWrite, StoreMe, StoreProfiles};
    let me = StoreMe(TursoStore::open_in_memory().await.unwrap());
    let anchor = me.create_me().await.unwrap().anchor;
    let repo = StoreProfiles(me.0);
    let by = By::new(anchor.urn(), anchor.person_ref());
    let made = repo
        .create_profile(&by, NewProfile::named("Via Adapter"))
        .await
        .unwrap();
    let patch = ProfilePatch {
        phone: Some("555".into()),
        ..ProfilePatch::default()
    };
    let changed = repo
        .update_profile(&by, made.id, patch)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(changed.values("phones").unwrap(), ["555"]);
    assert!(
        repo.update_profile(&by, made.id, ProfilePatch::default())
            .await
            .is_err(),
        "an empty patch is no change"
    );
    assert!(repo.delete_profile(&by, made.id).await.unwrap());
    assert!(
        !repo.delete_profile(&by, made.id).await.unwrap(),
        "the second delete found something"
    );
    assert!(
        !repo.delete_profile(&by, anchor.id).await.unwrap(),
        "the anchor is no profile and is not deleted"
    );
}

/// Before "me" is born the store has no owner: a profile is refused with that reason (not a store error), and nothing is written.
#[tokio::test(flavor = "current_thread")]
async fn without_me_a_profile_is_refused_with_the_reason() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let (socket, repo, _stop) = serve(dir.path()).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let s = &c.client;
            assert!(repo.me().await.unwrap().is_none());
            refused(
                &create(s, &[("name", "Early")]).await,
                "form-source-rejected",
            );
            assert_eq!(repo.count().await.unwrap(), 0);
            // then "me", and the first profile of the empty database is made
            assert!(!invoke(s, COMMAND_CREATE, "", &[]).await.failed);
            assert!(!create(s, &[("name", "Early")]).await.failed);
            assert_eq!(repo.count().await.unwrap(), 1);
        })
        .await;
}
