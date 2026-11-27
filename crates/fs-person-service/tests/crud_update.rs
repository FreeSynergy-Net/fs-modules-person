//! Change a profile at the service: `person.update` (and the `expect` of all three write commands) through a real socket in a temporary folder. Each test names what a wrong implementation would get wrong.
//!
//! Pattern: Scenario test (given a store, when the owner calls, then the store shows exactly that).
#![cfg(unix)]

mod common;
#[macro_use]
mod crud_common;

use common::{invoke, invoke_expect};
use crud_common::{create, id_of, refused};
use fs_person::{MeRepo, ProfileRepo};
use fs_person_service::{COMMAND_DELETE, COMMAND_PROFILE_CREATE, COMMAND_UPDATE};

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
        // the record and the argument `id` name different profiles: refused, whichever is right
        let other = create(source, &[("name", "Other")]).await;
        let clash = invoke(
            source,
            COMMAND_UPDATE,
            &made.record,
            &[("id", &other.record), ("name", "x")],
        )
        .await;
        refused(&clash, "form-source-bad-arguments");
        let unreadable = invoke(source, COMMAND_UPDATE, "no-uuid", &[("name", "x")]).await;
        refused(&unreadable, "form-source-bad-arguments");
        assert_eq!(
            repo.get(id).await.unwrap().unwrap(),
            before,
            "a refused call changed the profile"
        );
    });
}

/// `Invocation.record` is the profile (as the uid is for `task.edit`); the same profile as `id` too is fine, a profile that is not there is
/// `unknown-record`.
#[tokio::test(flavor = "current_thread")]
async fn update_takes_the_profile_as_the_record() {
    connected!(source, repo, {
        let made = create(source, &[("name", "By Record")]).await;
        let id = id_of(&made.record);
        let done = invoke(source, COMMAND_UPDATE, &made.record, &[("name", "Renamed")]).await;
        assert!(!done.failed, "{done:?}");
        assert_eq!(
            (done.record.as_str(), done.text.as_str()),
            (made.record.as_str(), "profile-updated")
        );
        assert_eq!(
            repo.get(id).await.unwrap().unwrap().card.display_name(),
            "Renamed"
        );
        // the record as the bare id, and the same profile twice (record and `id`)
        let bare = invoke(
            source,
            COMMAND_UPDATE,
            &id.to_string(),
            &[("id", &made.record), ("phone", "1")],
        )
        .await;
        assert!(!bare.failed, "{bare:?}");
        assert_eq!(
            repo.get(id)
                .await
                .unwrap()
                .unwrap()
                .values("phones")
                .unwrap(),
            ["1"]
        );
        let nobody = "urn:uuid:01900000-0000-7000-8000-0000000000ff";
        refused(
            &invoke(source, COMMAND_UPDATE, nobody, &[("name", "x")]).await,
            "form-source-unknown-record",
        );
    });
}

/// None of the three commands knows a compare-and-set: an `expect` is refused with 501 (`unsupported`), never silently dropped, and
/// nothing is written.
#[tokio::test(flavor = "current_thread")]
async fn an_expect_is_refused_at_all_three_commands() {
    connected!(source, repo, {
        let made = create(source, &[("name", "Stay")]).await;
        let id = id_of(&made.record);
        let before = repo.get(id).await.unwrap().unwrap();
        let count = repo.count().await.unwrap();
        let calls = [
            (COMMAND_PROFILE_CREATE, "", vec![("name", "New")]),
            (
                COMMAND_UPDATE,
                made.record.as_str(),
                vec![("name", "Changed")],
            ),
            (COMMAND_DELETE, made.record.as_str(), vec![]),
        ];
        for (command, record, pairs) in calls {
            let a = invoke_expect(source, command, record, &pairs, "1").await;
            refused(&a, "form-source-unsupported");
        }
        assert_eq!(
            repo.get(id).await.unwrap().unwrap(),
            before,
            "a refused call wrote"
        );
        assert_eq!(repo.count().await.unwrap(), count);
    });
}
