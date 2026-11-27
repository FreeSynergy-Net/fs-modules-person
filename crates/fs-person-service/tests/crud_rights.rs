//! Who may write: strangers are refused, the sample data is read only. Each test names what a wrong implementation would get wrong.
//!
//! Pattern: Scenario test (given a store, when the owner or a stranger calls, then the store shows exactly that) + counter-proof
//! (`AllowAll` rules let a stranger through, so the refusals are the rules).
#![cfg(unix)]

mod common;
#[macro_use]
mod crud_common;

use std::sync::Arc;

use common::{Answer, Repo, invoke, seeded, serve_service};
use crud_common::{create, refused};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{Ask, Context, HiddenReason, RightsRules, Verdict};
use fs_person::{Anchor, ProfileRepo, SampleRepo};
use fs_person_service::{
    COMMAND_CREATE, COMMAND_DELETE, COMMAND_UPDATE, Owner, PersonCore, PersonService,
};
use fs_rpc::{AllBots, FixedPerson, OsUserIsPerson, Ports};
use tokio::task::LocalSet;

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
