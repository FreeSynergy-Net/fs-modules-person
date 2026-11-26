//! M3: the list `rows@1.0.0` and the single record `single-record@1.2.0` of the person module over a real socket and the real
//! `Guarded<PersonCore>`: page, order, filter, state column, repeated values, no second person, and a bot sees nothing.
//!
//! Pattern: Scenario test (given a store with the sample profiles, when a client reads, then the records of the forms).
#![cfg(unix)]

mod common;

use common::{Repo, Shown, all, field, page_with, seeded, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_objects::{Query, Store};
use fs_person::{MeRepo, sample_anchor, sample_profiles};
use fs_person_service::{FORM_RECORD, FORM_ROWS, PersonService, record_fields};
use tokio::task::LocalSet;

const FIRST: &str = "urn:uuid:01900000-0000-7000-8000-000000000001"; // Amara Zolenko (describes me)

async fn owner_reads<F, Fut>(repo: Repo, body: F)
where
    F: FnOnce(form_source::Client) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let service = PersonService::open(repo, |_| {}).await.unwrap();
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            body(c.client.clone()).await;
        })
        .await;
}

fn labels(shown: &Shown) -> Vec<&str> {
    shown.rows.iter().map(|r| field(r, "label")).collect()
}

/// The list: people only, in name order, the state of each row, one value per column, the first picture as `media`.
#[tokio::test(flavor = "current_thread")]
async fn the_list_shows_people_with_state_and_one_value_per_column() {
    owner_reads(seeded().await, |source| async move {
        let list = page_with(&source, FORM_ROWS, "all", "", 0).await.unwrap();
        assert_eq!(
            labels(&list),
            [
                "A. Zolenko",
                "Amara Zolenko",
                "Assistent Sol",
                "Mr. John Q. Public, Esq.",
                "Soo-jin Marchetti",
                "Tariq Lindqvist"
            ]
        );
        assert_eq!(list.total, Some(6));
        assert_eq!(list.next, "");
        assert!(
            list.may.is_empty() && list.rows.iter().all(|r| r.1.is_empty()),
            "nothing writes, nothing is offered"
        );
        let states: Vec<_> = list.rows.iter().map(|r| field(r, "state")).collect();
        assert_eq!(
            states,
            ["me", "me", "account", "contact", "account", "contact"]
        );
        let amara = &list.rows[1];
        assert_eq!(
            all(amara, "emails"),
            ["amara@example.org"],
            "ONE e-mail, the preferred one, not both"
        );
        assert_eq!(all(amara, "phones"), ["tel:+493023125000"]);
        assert_eq!(
            field(amara, "media"),
            "https://example.org/sample/amara.jpg"
        );
        let tariq = &list.rows[5];
        assert_eq!(field(tariq, "organizations"), "Tamarind Works Ltd");
        assert_eq!(field(tariq, "titles"), "Research Scientist");
        assert_eq!(field(&list.rows[3], "nicknames"), "Johnny");
        for row in &list.rows {
            for path in ["uid", "prodId", "name", "notes"] {
                assert!(all(row, path).is_empty(), "{path} is not a column");
            }
        }
    })
    .await;
}

/// Filter and paging together over the wire: page size 2 with the device and the group between the people.
#[tokio::test(flavor = "current_thread")]
async fn pages_with_a_cursor_join_to_the_list_and_never_show_a_group() {
    owner_reads(seeded().await, |source| async move {
        let whole = page_with(&source, FORM_ROWS, "", "", 0).await.unwrap();
        let (mut joined, mut cursor, mut pages) = (Vec::new(), String::new(), 0);
        loop {
            let p = page_with(&source, FORM_ROWS, "all", &cursor, 2)
                .await
                .unwrap();
            assert_eq!(
                p.total,
                Some(6),
                "the total counts the listed people, not the page"
            );
            joined.extend(p.rows.iter().map(|r| r.0.clone()));
            pages += 1;
            if p.next.is_empty() {
                break;
            }
            assert_eq!(p.rows.len(), 2);
            cursor = p.next;
        }
        assert_eq!((joined.len(), pages), (6, 3));
        assert_eq!(
            joined,
            whole.rows.iter().map(|r| r.0.clone()).collect::<Vec<_>>()
        );
        for kind in [
            "Quillon Team",
            "Tamarind Works Ltd",
            "Rathaus Beispielstadt",
            "Laptop Amara",
        ] {
            assert!(!labels(&whole).contains(&kind), "{kind} is no person");
        }
        let bad = page_with(&source, FORM_ROWS, "all", "not-a-number", 2)
            .await
            .err()
            .unwrap();
        assert!(
            bad.to_string().contains("form-source-bad-arguments"),
            "{bad}"
        );
        let scope = page_with(&source, FORM_ROWS, "everybody", "", 2)
            .await
            .err()
            .unwrap();
        assert!(
            scope.to_string().contains("form-source-unknown-scope"),
            "{scope}"
        );
    })
    .await;
}

/// One profile as a whole: several e-mails and phones come as the same path repeated, in the order of the card; `describes`
/// is the anchor; `uid` and `prodId` are not sent; a card that is no person is still readable by its id.
#[tokio::test(flavor = "current_thread")]
async fn the_single_record_repeats_the_values_of_a_field() {
    owner_reads(seeded().await, |source| async move {
        let anchor = sample_anchor().urn();
        let one = page_with(&source, FORM_RECORD, FIRST, "", 0).await.unwrap();
        assert_eq!(one.rows.len(), 1);
        let r = &one.rows[0];
        assert_eq!(r.0, FIRST);
        assert_eq!(field(r, "label"), "Amara Zolenko");
        assert_eq!(field(r, "state"), "me");
        assert_eq!(
            all(r, "emails"),
            ["amara@example.org", "amara.zolenko@arbeit.example.org"],
            "two mails, two values, never joined"
        );
        assert_eq!(all(r, "phones"), ["tel:+493023125000", "tel:+491713920001"]);
        assert_eq!(all(r, "addresses"), ["Musterweg 12\n10115 Berlin"]);
        assert_eq!(all(r, "describes"), [anchor.as_str()]);
        assert_eq!(all(r, "kind"), ["individual"]);
        for path in ["uid", "prodId"] {
            assert!(all(r, path).is_empty(), "{path} is not sent");
        }
        let allowed = record_fields();
        for (path, _) in &r.2 {
            assert!(
                ["label", "state", "media"].contains(&path.as_str()) || allowed.contains(path),
                "{path}"
            );
        }
        // the bare UUID names the same profile; an unknown id is an empty page, not an error
        let bare = page_with(&source, FORM_RECORD, &FIRST["urn:uuid:".len()..], "", 0)
            .await
            .unwrap();
        assert_eq!(bare.rows.len(), 1);
        let none = page_with(
            &source,
            FORM_RECORD,
            "urn:uuid:01900000-0000-7000-8000-0000000000ff",
            "",
            0,
        )
        .await
        .unwrap();
        assert_eq!(none.rows.len(), 0);
        let junk = page_with(&source, FORM_RECORD, "no id", "", 0)
            .await
            .err()
            .unwrap();
        assert!(
            junk.to_string().contains("form-source-unknown-scope"),
            "{junk}"
        );
        // the group card is no row of the list but a record of its own id
        let group = page_with(
            &source,
            FORM_RECORD,
            "urn:uuid:01900000-0000-7000-8000-000000000007",
            "",
            0,
        )
        .await
        .unwrap();
        assert_eq!(field(&group.rows[0], "label"), "Quillon Team");
    })
    .await;
}

/// "One person": the anchor is never a row, no profile has its id, reading makes no person, and the stored persons stay one.
#[tokio::test(flavor = "current_thread")]
async fn no_second_person_next_to_the_anchor() {
    let repo = seeded().await;
    let persons = |repo: Repo| async move {
        repo.0
            .query(&Query::new().of_type("Person"))
            .await
            .unwrap()
            .len()
    };
    assert_eq!(persons(repo.clone()).await, 1);
    let kept = repo.clone();
    owner_reads(repo, |source| async move {
        let list = page_with(&source, FORM_ROWS, "all", "", 0).await.unwrap();
        let anchor = sample_anchor().urn();
        assert!(
            list.rows.iter().all(|r| r.0 != anchor),
            "the anchor is no row"
        );
        assert!(
            sample_profiles()
                .iter()
                .all(|p| format!("urn:uuid:{}", p.id) != anchor)
        );
        // the anchor's id is no profile either
        let as_profile = page_with(&source, FORM_RECORD, &anchor, "", 0)
            .await
            .unwrap();
        assert_eq!(as_profile.rows.len(), 0);
        // two profiles describe the anchor and are two rows with the same state, not two people
        let mine = list
            .rows
            .iter()
            .filter(|r| field(r, "state") == "me")
            .count();
        assert_eq!(mine, 2);
    })
    .await;
    assert_eq!(persons(kept.clone()).await, 1);
    assert_eq!(kept.me().await.unwrap(), Some(sample_anchor()));
}
