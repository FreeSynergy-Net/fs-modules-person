//! The rights of the person module (M2): a caller that is not the owner (a bot, or a connection that acts for another person) is
//! refused `me.create` and reads nothing, over a real socket and the real `Guarded<PersonCore>`. The counter-proof runs the same
//! module with rules "everything is allowed" (a wrong Strategy): the same call then gets through, so the refusal comes from
//! `PersonRules`, not from the wrapper or the transport.
//!
//! Pattern: Scenario test (given a service, when a stranger calls, then 403 and nothing is written or shown) + Decorator-free
//! counter-proof (`PersonCore::with_rules`).
#![cfg(unix)]

mod common;

use std::sync::Arc;

use common::{Repo, invoke, page, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{Ask, Context, HiddenReason, RightsRules, Verdict};
use fs_objects::TursoStore;
use fs_person::{Anchor, MeRepo, PersonStore};
use fs_person_service::{COMMAND_CREATE, FORM_ME, Owner, PersonCore, PersonRules, PersonService};
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

/// Who calls: a bot of the service's own user, or a person that acts for somebody else.
#[derive(Clone, Copy)]
enum Stranger {
    Bot,
    OtherPerson,
}

struct Outcome {
    create_refused: bool,
    read_refused: bool,
}

/// One service for `stranger` (with the rules of the module, or with `AllowAll`): can it create "me", and does it see "me"?
async fn run(stranger: Stranger, lax: bool) -> Outcome {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let repo: Repo = Arc::new(PersonStore(TursoStore::open_in_memory().await.unwrap()));
    // an owner exists: reading is the question; creating is a second probe on a fresh store below
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
    let connection = fs_rpc::connect::<form_source::Client>(&socket)
        .await
        .unwrap();
    let source = &connection.client;
    let made = invoke(source, COMMAND_CREATE, "", &[]).await;
    let create_refused =
        made.failed && made.kind == "form-source-forbidden" && repo.me().await.unwrap().is_none();
    // the person is made behind the service (as the owner would have): what does the stranger read?
    repo.create_me().await.unwrap();
    let shown = page(source, FORM_ME).await.unwrap();
    Outcome {
        create_refused,
        read_refused: shown.rows.is_empty() && shown.may.is_empty(),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_bot_cannot_create_me_and_reads_nothing() {
    LocalSet::new()
        .run_until(async {
            let o = run(Stranger::Bot, false).await;
            assert!(o.create_refused, "me.create of a bot got through");
            assert!(o.read_refused, "a bot read the person");
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_connection_for_another_person_cannot_create_me_and_reads_nothing() {
    LocalSet::new()
        .run_until(async {
            let o = run(Stranger::OtherPerson, false).await;
            assert!(o.create_refused, "me.create for another person got through");
            assert!(o.read_refused, "another person read the person");
        })
        .await;
}

/// Counter-proof: with rules that allow everything the same two strangers create and read; so the refusals above are the rules.
#[tokio::test(flavor = "current_thread")]
async fn counter_proof_rules_that_allow_everything_let_a_stranger_in() {
    LocalSet::new()
        .run_until(async {
            for stranger in [Stranger::Bot, Stranger::OtherPerson] {
                let o = run(stranger, true).await;
                assert!(!o.create_refused, "AllowAll refused me.create");
                assert!(!o.read_refused, "AllowAll hid the person");
            }
        })
        .await;
}

/// The refusal writes nothing: a stranger's `me.create` leaves the store without a person.
#[tokio::test(flavor = "current_thread")]
async fn a_refused_create_writes_nothing() {
    LocalSet::new()
        .run_until(async {
            let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
            let repo: Repo = Arc::new(PersonStore(TursoStore::open_in_memory().await.unwrap()));
            let owner = Owner::read(&*repo).await.unwrap();
            let core = PersonCore::new(repo.clone(), owner.clone());
            let service = PersonService::guarding(core, Ports::new(AllBots, owner), |_| {});
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let made = invoke(&c.client, COMMAND_CREATE, "", &[]).await;
            assert_eq!(made.kind, "form-source-forbidden");
            assert_eq!(repo.me().await.unwrap(), None);
            let shown = page(&c.client, FORM_ME).await.unwrap();
            assert!(
                shown.rows.is_empty() && shown.may.is_empty(),
                "a bot is offered `create`"
            );
        })
        .await;
}

/// The owner path: the person of the context is the anchor that the birth makes, so a connection made BEFORE the birth and one made
/// after it both read "me" (the rules compare the person of the context with the stored anchor).
#[tokio::test(flavor = "current_thread")]
async fn the_reserved_anchor_is_the_one_that_is_born() {
    LocalSet::new()
        .run_until(async {
            let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
            let repo: Repo = Arc::new(PersonStore(TursoStore::open_in_memory().await.unwrap()));
            let owner = Owner::read(&*repo).await.unwrap();
            let reserved = owner.person();
            let core = PersonCore::new(repo.clone(), owner.clone());
            let service = PersonService::guarding(core, Ports::new(OsUserIsPerson, owner), |_| {});
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let before = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let made = invoke(&before.client, COMMAND_CREATE, "", &[]).await;
            assert_eq!(made.text, "me-created");
            assert_eq!(repo.me().await.unwrap().unwrap().person_ref(), reserved);
            let after = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            for c in [&before, &after] {
                let shown = page(&c.client, FORM_ME).await.unwrap();
                assert_eq!(shown.rows.len(), 1);
                assert_eq!(shown.rows[0].0, made.record);
            }
        })
        .await;
}

/// `PersonRules` on their own: a bot, a connection-less context and another person are refused, the owner is not; `hidden` agrees.
#[test]
fn the_rules_know_the_owner_and_nobody_else() {
    use fs_interfaces::module::{Actor, Caller};
    use fs_types::Utc;

    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let repo = PersonStore(rt.block_on(TursoStore::open_in_memory()).unwrap());
    let owner = rt.block_on(Owner::read(&repo)).unwrap();
    let rules = PersonRules::new(owner.clone());
    let conn = || Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    let cx = |actor, person| Context::from_connection(conn(), actor, person);
    let at = Utc::from_millis(0);
    let asked = |c: &Context, command: &str| rules.may(&Ask::new(c, command, None, at));
    let me = cx(Actor::Person, owner.person());
    for command in [COMMAND_CREATE, "form.read", "change.revert", "me.delete"] {
        assert!(asked(&me, command).is_allowed(), "{command}");
    }
    assert!(rules.hidden(&me, "r", "f").is_none());
    let bot = cx(Actor::Bot, owner.person());
    let stranger = cx(Actor::Person, Anchor::new().person_ref());
    for c in [&bot, &stranger] {
        assert!(!asked(c, COMMAND_CREATE).is_allowed());
        assert!(!asked(c, "form.read").is_allowed());
        assert_eq!(rules.hidden(c, "r", "f"), Some(HiddenReason::NoRight));
    }
}
