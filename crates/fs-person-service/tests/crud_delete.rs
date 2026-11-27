//! Delete a profile at the service: `person.delete` through a real socket in a temporary folder. Each test names what a wrong implementation would get wrong.
//!
//! Pattern: Scenario test (given a store, when the owner calls, then the store shows exactly that).
#![cfg(unix)]

mod common;
#[macro_use]
mod crud_common;

use common::invoke;
use crud_common::{create, id_of, refused};
use fs_person::{MeRepo, ProfileRepo};
use fs_person_service::COMMAND_DELETE;

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
        // the record and `id` name different profiles: refused, and nothing is deleted
        refused(
            &invoke(source, COMMAND_DELETE, &b.record, &[("id", &a.record)]).await,
            "form-source-bad-arguments",
        );
        assert_eq!(repo.count().await.unwrap(), 1);
    });
}

/// `Invocation.record` is the profile to delete (as the uid is for `task.delete`), alone or with the same profile as `id`.
#[tokio::test(flavor = "current_thread")]
async fn delete_takes_the_profile_as_the_record() {
    connected!(source, repo, {
        let a = create(source, &[("name", "Anna")]).await;
        let b = create(source, &[("name", "Berta")]).await;
        let gone = invoke(source, COMMAND_DELETE, &a.record, &[]).await;
        assert!(!gone.failed, "{gone:?}");
        assert!(repo.get(id_of(&a.record)).await.unwrap().is_none());
        let same = invoke(source, COMMAND_DELETE, &b.record, &[("id", &b.record)]).await;
        assert!(!same.failed, "{same:?}");
        assert_eq!(repo.count().await.unwrap(), 0);
        refused(
            &invoke(source, COMMAND_DELETE, &a.record, &[]).await,
            "form-source-unknown-record",
        );
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
            assert_eq!(a.record.len(), 0, "no record in the answer");
        }
        assert_eq!(repo.me().await.unwrap(), Some(me));
        assert!(repo.get(id_of(&p.record)).await.unwrap().is_some());
    });
}
