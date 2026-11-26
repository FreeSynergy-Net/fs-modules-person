//! The anchor is born at the first start (`PersonService::open_anchored`, `PersonCore::ensure_me`): once, by the rules, as the
//! anchor itself. Real stores (a file for the double start, memory elsewhere), fixed steps.
//!
//! Pattern: Scenario test (given a folder, when the service starts, then "me" is there once); counter-proofs: a refusing rule
//! stops the birth, a bot is still refused `me.create` after the autostart.
#![cfg(unix)]

mod common;

use std::sync::Arc;

use common::{invoke, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{Ask, Context, HiddenReason, RightsRules, Verdict};
use fs_objects::{ActivityFilter, Store, TursoStore};
use fs_person::{MeRepo, PersonStore};
use fs_person_service::{COMMAND_CREATE, Owner, PersonCore, PersonService};
use fs_rpc::{AllBots, Ports};
use tokio::task::LocalSet;

type Repo = Arc<PersonStore<TursoStore>>;

async fn memory() -> Repo {
    Arc::new(PersonStore(TursoStore::open_in_memory().await.unwrap()))
}

async fn writes(repo: &Repo) -> usize {
    repo.0
        .activities(&ActivityFilter::default())
        .await
        .unwrap()
        .len()
}

/// Strategy (a wrong variant of `RightsRules`): refuses everything.
struct DenyAll;

impl RightsRules for DenyAll {
    fn may(&self, _: &Ask<'_>) -> Verdict {
        Verdict::deny("no-way")
    }
    fn hidden(&self, _: &Context, _: &str, _: &str) -> Option<HiddenReason> {
        Some(HiddenReason::NoRight)
    }
}

/// An empty folder: the start makes "me", with exactly one write.
#[tokio::test(flavor = "current_thread")]
async fn an_empty_folder_gets_its_anchor_at_start() {
    let repo = memory().await;
    assert!(repo.me().await.unwrap().is_none());
    PersonService::open_anchored(repo.clone(), |_| {})
        .await
        .unwrap();
    let me = repo.me().await.unwrap().expect("me after the start");
    assert_eq!(me.person_ref().art, fs_types::PersonArt::Person);
    assert_eq!(writes(&repo).await, 1);
}

/// The second start (same folder, new process in effect): the same person, no second write.
#[tokio::test(flavor = "current_thread")]
async fn the_second_start_finds_the_same_person_and_writes_nothing() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let file = dir.path().join("objects.db");
    let first = {
        let repo: Repo = Arc::new(PersonStore(TursoStore::open(&file).await.unwrap()));
        PersonService::open_anchored(repo.clone(), |_| {})
            .await
            .unwrap();
        assert_eq!(writes(&repo).await, 1);
        repo.me().await.unwrap().unwrap().urn()
    };
    let repo: Repo = Arc::new(PersonStore(TursoStore::open(&file).await.unwrap()));
    PersonService::open_anchored(repo.clone(), |_| {})
        .await
        .unwrap();
    assert_eq!(repo.me().await.unwrap().unwrap().urn(), first);
    assert_eq!(writes(&repo).await, 1, "no second birth, no second entry");
}

/// A folder that has "me": the start changes nothing, and `ensure_me` says `me-exists`.
#[tokio::test(flavor = "current_thread")]
async fn an_existing_anchor_is_left_alone() {
    let repo = memory().await;
    let born = repo.create_me().await.unwrap();
    assert!(born.new);
    let owner = Owner::read(&*repo).await.unwrap();
    let said = PersonCore::new(repo.clone(), owner)
        .ensure_me()
        .await
        .unwrap();
    assert_eq!(
        (said.text.as_str(), said.record),
        ("me-exists", born.anchor.urn())
    );
    assert_eq!(repo.me().await.unwrap().unwrap().urn(), born.anchor.urn());
    assert_eq!(writes(&repo).await, 1);
}

/// `me.create` stays and says `me-exists` after the autostart (over a real socket).
#[tokio::test(flavor = "current_thread")]
async fn me_create_after_the_start_is_me_exists() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let repo = memory().await;
            let service = PersonService::open_anchored(repo.clone(), |_| {})
                .await
                .unwrap();
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let connection = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let again = invoke(&connection.client, COMMAND_CREATE, "", &[]).await;
            assert!(!again.failed, "{again:?}");
            assert_eq!(again.text, "me-exists");
            assert_eq!(again.record, repo.me().await.unwrap().unwrap().urn());
            assert_eq!(writes(&repo).await, 1);
        })
        .await;
}

/// Counter-proof 1: the birth asks the rules; where they refuse, nothing is written (no way round the rights).
#[tokio::test(flavor = "current_thread")]
async fn a_refusing_rule_stops_the_birth() {
    let repo = memory().await;
    let owner = Owner::read(&*repo).await.unwrap();
    let core = PersonCore::with_rules(repo.clone(), owner, Box::new(DenyAll));
    let refused = core.ensure_me().await.unwrap_err();
    assert_eq!(
        (refused.kind.as_str(), refused.status),
        ("form-source-forbidden", 403)
    );
    assert!(repo.me().await.unwrap().is_none());
    assert_eq!(writes(&repo).await, 0);
}

/// Counter-proof 2: after the autostart a stranger (a bot) is still refused `me.create`; the anchor stays as it was.
#[tokio::test(flavor = "current_thread")]
async fn a_stranger_stays_refused_after_the_start() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let repo = memory().await;
            let owner = Owner::read(&*repo).await.unwrap();
            let core = PersonCore::new(repo.clone(), owner.clone());
            core.ensure_me().await.unwrap();
            let service = PersonService::guarding(core, Ports::new(AllBots, owner), |_| {});
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let connection = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let denied = invoke(&connection.client, COMMAND_CREATE, "", &[]).await;
            assert!(denied.failed, "{denied:?}");
            assert_eq!(writes(&repo).await, 1);
        })
        .await;
}
