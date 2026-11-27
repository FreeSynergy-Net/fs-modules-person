//! Create a profile at the service: `person.profile.create` through a real socket in a temporary folder, every value a text. Each test names what a wrong implementation would get wrong.
//!
//! Pattern: Scenario test (given a store, when the owner calls, then the store shows exactly that).
#![cfg(unix)]

mod common;
#[macro_use]
mod crud_common;

use common::{invoke, serve};
use crud_common::{create, id_of, refused};
use fs_interfaces::formsource_capnp::form_source;
use fs_objects::TursoStore;
use fs_person::{MeRepo, NewProfile, ProfileRepo, ProfileWrite};
use fs_person_service::COMMAND_CREATE;
use fs_person_service::{COMMAND_PROFILE_CREATE, COMMAND_UPDATE};
use tokio::task::LocalSet;

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
        // (the service refuses to make it, see `a_second_profile_of_me_is_refused`; here it is made behind the service)
        let by = fs_objects::By::new(anchor.urn(), anchor.person_ref());
        let second = repo
            .create_profile(
                &by,
                NewProfile {
                    name: "Me Again".into(),
                    describes: Some(anchor.urn()),
                    ..NewProfile::default()
                },
            )
            .await
            .unwrap();
        let second_record = format!("urn:uuid:{}", second.id);
        let unclear = invoke(source, COMMAND_UPDATE, "", &[("name", "X")]).await;
        refused(&unclear, "form-source-conflict");
        for r in [&mine.record, &second_record] {
            let p = repo.get(id_of(r)).await.unwrap().unwrap();
            assert_ne!(p.card.display_name(), "X");
        }
    });
}

/// `describes=me` twice: the second is refused when it is made (422 with the reason, not only later at `person.update`), and nothing is written.
#[tokio::test(flavor = "current_thread")]
async fn a_second_profile_of_me_is_refused() {
    connected!(source, repo, {
        let first = create(source, &[("name", "Me Myself"), ("describes", "me")]).await;
        assert!(!first.failed, "{first:?}");
        let second = create(source, &[("name", "Me Again"), ("describes", "me")]).await;
        refused(&second, "form-source-rejected");
        assert_eq!(repo.count().await.unwrap(), 1, "the refused call wrote");
        // a contact without `describes` is still fine
        assert!(!create(source, &[("name", "Contact")]).await.failed);
    });
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
