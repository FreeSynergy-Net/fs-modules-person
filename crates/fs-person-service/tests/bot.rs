//! M3: a bot sees nothing of the lists, not even a total, and not the difference between an unknown id and a known one; with rules
//! that allow everything the same bot reads (the counter-proof: the refusal is `PersonRules`, not the wrapper or the transport).
//!
//! Pattern: Scenario test + Decorator-free counter-proof (`PersonCore::with_rules`).
#![cfg(unix)]

mod common;

use common::{page_with, seeded, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{Ask, Context, HiddenReason, RightsRules, Verdict};
use fs_person_service::{FORM_RECORD, FORM_ROWS, Owner, PersonCore, PersonService};
use fs_rpc::{AllBots, Ports};
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

const FIRST: &str = "urn:uuid:01900000-0000-7000-8000-000000000001"; // Amara Zolenko (describes me)

/// A bot, or a connection for another person, sees no row and no total on either form, not even to tell an unknown id from a
/// known one. Counter-proof: with rules that allow everything the same bot reads the list.
#[tokio::test(flavor = "current_thread")]
async fn a_bot_sees_nothing_and_with_lax_rules_it_would() {
    for lax in [false, true] {
        let repo = seeded().await;
        let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
        LocalSet::new()
            .run_until(async {
                let owner = Owner::read(&*repo).await.unwrap();
                let core = if lax {
                    PersonCore::with_rules(repo.clone(), owner.clone(), Box::new(AllowAll))
                } else {
                    PersonCore::new(repo.clone(), owner.clone())
                };
                let service = PersonService::guarding(core, Ports::new(AllBots, owner), |_| {});
                let (socket, _stop) = serve_service(dir.path(), service).await;
                let c = fs_rpc::connect::<form_source::Client>(&socket)
                    .await
                    .unwrap();
                let list = page_with(&c.client, FORM_ROWS, "all", "", 0).await.unwrap();
                let known = page_with(&c.client, FORM_RECORD, FIRST, "", 0)
                    .await
                    .unwrap();
                let unknown = page_with(
                    &c.client,
                    FORM_RECORD,
                    "urn:uuid:01900000-0000-7000-8000-0000000000ff",
                    "",
                    0,
                )
                .await
                .unwrap();
                if lax {
                    assert_eq!(list.rows.len(), 7, "AllowAll lets the bot read");
                    assert_eq!(known.rows.len(), 1);
                } else {
                    for shown in [&list, &known, &unknown] {
                        assert!(shown.rows.is_empty() && shown.may.is_empty());
                        assert_eq!((shown.total, shown.next.as_str()), (None, ""));
                    }
                }
            })
            .await;
    }
}
