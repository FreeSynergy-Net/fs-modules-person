//! M3: the list of people over a real fs-objects store (in memory) and over the sample data: filter and paging are one step,
//! the sample comes in the order of the store, the preferred value of a column, and the sample is the same on every run.
//!
//! Pattern: Scenario test (given a store with the sample profiles, when the list is read page by page, then the pages join to
//! the whole list); Oracle (the store is the oracle for the order of the sample repository).

use fs_lib_jscontact::Card;
use fs_objects::{By, Envelope, Store, StoredObject, TursoStore, Write};
use fs_person::{
    CONTENT_TYPE, MeRepo, PROFILE_TYPE, PersonStore, Profile, ProfileRepo, SampleRepo, list_page,
    list_total, preferred, sample_anchor, sample_profiles,
};
use serde_json::json;

fn object_of(p: &Profile) -> StoredObject {
    let mut e = Envelope::new(PROFILE_TYPE, p.card.display_name());
    e.id = p.id;
    e.describes.clone_from(&p.describes);
    StoredObject::new(e, CONTENT_TYPE, p.card.to_value())
}

/// A store with the anchor of the sample and its ten profiles.
async fn seeded() -> PersonStore<TursoStore> {
    let repo = PersonStore(TursoStore::open_in_memory().await.unwrap());
    let anchor = sample_anchor();
    repo.create_me_as(anchor.clone()).await.unwrap();
    let by = By::new(anchor.urn(), anchor.person_ref());
    let writes = sample_profiles()
        .iter()
        .map(|p| Write::Put {
            object: object_of(p),
            expect: None,
        })
        .collect();
    repo.0.write_by(&by, writes).await.unwrap();
    repo
}

fn names(profiles: &[Profile]) -> Vec<String> {
    profiles.iter().map(|p| p.card.display_name()).collect()
}

/// Every page of the list with `limit` listed profiles each, and the number of pages.
async fn all_pages<R: ProfileRepo>(repo: &R, limit: usize) -> (Vec<String>, usize) {
    let (mut out, mut from, mut pages) = (Vec::new(), 0, 0);
    loop {
        let page = list_page(repo, from, limit).await.unwrap();
        pages += 1;
        assert!(page.profiles.len() <= limit);
        assert!(
            !page.profiles.is_empty(),
            "a page of the list is never empty"
        );
        out.extend(names(&page.profiles));
        match page.next {
            Some(n) => from = n,
            None => return (out, pages),
        }
    }
}

/// Group, organisation, place and device are no people; an AI (`application`) is.
#[tokio::test]
async fn the_list_holds_the_six_people_in_name_order() {
    let repo = seeded().await;
    let whole = list_page(&repo, 0, 100).await.unwrap();
    assert!(whole.next.is_none());
    assert_eq!(
        names(&whole.profiles),
        [
            "A. Zolenko",
            "Amara Zolenko",
            "Assistent Sol",
            "Mr. John Q. Public, Esq.",
            "Soo-jin Marchetti",
            "Tariq Lindqvist"
        ]
    );
    assert_eq!(list_total(&repo).await.unwrap(), 6);
    assert_eq!(
        repo.count().await.unwrap(),
        10,
        "the four others are stored, only not listed"
    );
    assert!(
        whole
            .profiles
            .iter()
            .any(|p| p.card.kind.as_deref() == Some("application"))
    );
}

/// Filter and paging together: a page holds `limit` LISTED profiles even where an unlisted card stands between them (the device
/// "Laptop Amara" sits between "Assistent Sol" and "Mr. John"), and the pages join to the whole list.
#[tokio::test]
async fn pages_of_any_size_join_to_the_whole_list() {
    let repo = seeded().await;
    let (whole, _) = all_pages(&repo, 100).await;
    assert_eq!(whole.len(), 6);
    for (limit, pages) in [(1, 6), (2, 3), (3, 2), (4, 2), (6, 1)] {
        let (joined, n) = all_pages(&repo, limit).await;
        assert_eq!(joined, whole, "limit {limit}");
        assert_eq!(n, pages, "limit {limit}");
    }
    let first = list_page(&repo, 0, 3).await.unwrap();
    assert_eq!(
        first.next,
        Some(3),
        "right after Assistent Sol, the device is the next stored card"
    );
}

/// The sample repository is in the order the store gives, page by page (the store is the oracle), and the same on every run.
#[tokio::test]
async fn the_sample_is_in_the_order_of_the_store_and_always_the_same() {
    let (store, sample, again) = (seeded().await, SampleRepo::new(), SampleRepo::new());
    for (offset, limit) in [(0, 10), (0, 3), (3, 3), (9, 5), (10, 5)] {
        let (a, b) = (
            store.page(offset, limit).await.unwrap(),
            sample.page(offset, limit).await.unwrap(),
        );
        assert_eq!(a, b, "{offset}+{limit}");
        assert_eq!(b, again.page(offset, limit).await.unwrap());
    }
    assert_eq!(sample.count().await.unwrap(), 10);
    assert_eq!(sample.me().await.unwrap(), Some(sample_anchor()));
    // nothing is made: the sample is "me" already
    let made = sample.create_me().await.unwrap();
    assert_eq!((made.new, made.anchor), (false, sample_anchor()));
    assert!(fs_person::PersonRepo::is_sample(&sample));
    assert!(!fs_person::PersonRepo::is_sample(&store));
    let id = sample.page(0, 1).await.unwrap()[0].id;
    assert_eq!(sample.get(id).await.unwrap().unwrap().id, id);
}

fn card(v: serde_json::Value) -> Card {
    serde_json::from_value(v).unwrap()
}

/// A column shows ONE value: the preferred one (`pref` 1 is the best), else the first; nothing is joined.
#[test]
fn a_column_shows_the_preferred_value_or_the_first() {
    let c = card(json!({
        "@type": "Card", "version": "1.0", "uid": "u",
        "emails": {"e1": {"address": "second@example.org", "pref": 2}, "e2": {"address": "first@example.org", "pref": 1}},
        "phones": {"p1": {"number": "tel:+1"}, "p2": {"number": "tel:+2"}},
        "nicknames": {"n1": {"name": "No pref"}, "n2": {"name": "Pref", "pref": 5}},
        "organizations": {"o1": {"name": "ABC"}, "o2": {"name": "XYZ"}}
    }));
    assert_eq!(
        preferred(&c, "emails").as_deref(),
        Some("first@example.org")
    );
    assert_eq!(
        preferred(&c, "phones").as_deref(),
        Some("tel:+1"),
        "no pref: the first"
    );
    assert_eq!(
        preferred(&c, "nicknames").as_deref(),
        Some("Pref"),
        "a pref beats none"
    );
    assert_eq!(preferred(&c, "organizations").as_deref(), Some("ABC"));
    assert_eq!(preferred(&c, "titles"), None);
    assert_eq!(preferred(&c, "links"), None, "no column");
}
