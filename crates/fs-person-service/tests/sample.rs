//! M3: the sample provider over a real socket: the same forms from fixed test data, `hello` says `fake`, nothing is written, and
//! the rights are the same as for a store: the sample does not let a bot or a stranger read.
//!
//! Pattern: Scenario test + Oracle (the store seeded with the sample is the oracle of the list the sample provider serves).
#![cfg(unix)]

mod common;

use std::sync::Arc;

use common::{all, field, invoke, page_with, seeded, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::SAMPLE_PROVIDER;
use fs_person::{Anchor, SampleRepo, sample_anchor};
use fs_person_service::{
    COMMAND_CREATE, FORM_ME, FORM_RECORD, FORM_ROWS, Owner, PersonCore, PersonService,
};
use fs_rpc::{AllBots, FixedPerson, OsUserIsPerson, Ports};
use tokio::task::LocalSet;

enum Asker {
    Owner,
    Bot,
    Other,
}

/// What a client of the sample service gets: the ids of the list, the single record of the first profile, `me`, the provider word.
struct Seen {
    list: Vec<String>,
    one: usize,
    me: usize,
    provider: String,
}

async fn ask(asker: Asker) -> Seen {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let repo = Arc::new(SampleRepo::new());
    LocalSet::new()
        .run_until(async {
            let owner = Owner::read(&*repo).await.unwrap();
            let ports = match asker {
                Asker::Owner => Ports::new(OsUserIsPerson, owner.clone()),
                Asker::Bot => Ports::new(AllBots, owner.clone()),
                Asker::Other => Ports::new(OsUserIsPerson, FixedPerson(Anchor::new().person_ref())),
            };
            let core = PersonCore::new(repo.clone(), owner);
            let service = PersonService::guarding(core, ports, |_| {});
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let s = &c.client;
            let hello = s.hello_request().send().promise.await.unwrap();
            let provider = hello
                .get()
                .unwrap()
                .get_provider()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let list = page_with(s, FORM_ROWS, "all", "", 0).await.unwrap();
            let one = page_with(
                s,
                FORM_RECORD,
                "urn:uuid:01900000-0000-7000-8000-000000000001",
                "",
                0,
            )
            .await
            .unwrap();
            let me = page_with(s, FORM_ME, "", "", 0).await.unwrap();
            Seen {
                list: list.rows.iter().map(|r| r.0.clone()).collect(),
                one: one.rows.len(),
                me: me.rows.len(),
                provider,
            }
        })
        .await
}

/// The sample serves the list, the record and `me`, says `fake`, and is the same on every run.
#[tokio::test(flavor = "current_thread")]
async fn the_sample_serves_the_forms_and_says_it_is_sample_data() {
    let a = ask(Asker::Owner).await;
    assert_eq!(a.provider, SAMPLE_PROVIDER);
    assert_eq!((a.list.len(), a.one, a.me), (6, 1, 1));
    let b = ask(Asker::Owner).await;
    assert_eq!(a.list, b.list, "the same ids on every run");
}

/// The store seeded with the sample serves the very list the sample provider does (the sample is no second truth about the order).
#[tokio::test(flavor = "current_thread")]
async fn the_sample_list_is_the_list_of_a_store_with_the_sample() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let repo = seeded().await;
    let from_store = LocalSet::new()
        .run_until(async {
            let service = PersonService::open(repo, |_| {}).await.unwrap();
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let list = page_with(&c.client, FORM_ROWS, "all", "", 0).await.unwrap();
            list.rows
        })
        .await;
    let ids: Vec<_> = from_store.iter().map(|r| r.0.clone()).collect();
    assert_eq!(ids, ask(Asker::Owner).await.list);
    assert_eq!(field(&from_store[1], "label"), "Amara Zolenko");
    assert_eq!(all(&from_store[1], "emails"), ["amara@example.org"]);
}

/// The sample is no way round the rights: a bot and a connection for another person see nothing of it.
#[tokio::test(flavor = "current_thread")]
async fn the_sample_does_not_get_round_the_rights() {
    for asker in [Asker::Bot, Asker::Other] {
        let s = ask(asker).await;
        assert_eq!(
            s.provider, SAMPLE_PROVIDER,
            "hello is not a read of records"
        );
        assert_eq!((s.list.len(), s.one, s.me), (0, 0, 0));
    }
}

/// `me.create` on the sample: "me" is there, nothing is made (it answers `me-exists` with the sample anchor).
#[tokio::test(flavor = "current_thread")]
async fn me_create_on_the_sample_makes_nothing() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let repo = Arc::new(SampleRepo::new());
            let service = PersonService::open(repo, |_| {}).await.unwrap();
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let made = invoke(&c.client, COMMAND_CREATE, "", &[]).await;
            assert!(!made.failed, "{made:?}");
            assert_eq!(
                (made.text.as_str(), made.record),
                ("me-exists", sample_anchor().urn())
            );
        })
        .await;
}
