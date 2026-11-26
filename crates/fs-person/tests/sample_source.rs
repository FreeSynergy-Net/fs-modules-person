//! The sample profiles against their one source, `fs_person_directory::sample`: ids, names, addresses, the links to the person
//! and the contact fields of the cards are those of the directory, so there is ONE Amara for calendar, tasks and people. The
//! test is red as soon as one sample row differs (shown with deliberately diverging copies).
//!
//! Pattern: Oracle (the directory sample is the oracle for the ids, names and addresses of the module sample).
//! Written against the public data only (`sample_profiles`, `sample_anchor`), so a copy of a literal cannot slip through.

use fs_lib_jscontact::Card;
use fs_person::{Profile, sample_anchor, sample_profiles};
use fs_person_directory::{
    ExcerptValue, PersonArt, PersonDirectory as _, Resolved, sample, sample_address, sample_card,
    sample_name, sample_ref,
};
use fs_types::{BasicValue as _, ContactValue as _};

fn iri(n: u8) -> String {
    sample_ref(n).unwrap().id.canonical()
}

fn vals(p: &Profile, field: &str) -> Vec<String> {
    p.values(field).unwrap_or_default()
}

/// Everything in which `profiles` differs from the directory sample (empty = the same).
async fn problems(profiles: &[Profile]) -> Vec<String> {
    let (dir, mut out) = (sample(), Vec::new());
    if sample_anchor().urn() != iri(1) {
        out.push(format!(
            "anchor {} is not entry 1 {}",
            sample_anchor().urn(),
            iri(1)
        ));
    }
    for n in [1u8, 2, 3, 4, 5, 6, 10, 12] {
        let Resolved::Found(view) = dir.resolve(&sample_ref(n).unwrap()).await.unwrap() else {
            out.push(format!("entry {n} is not found in the directory"));
            continue;
        };
        let name = sample_name(n).unwrap();
        // a profile of the directory (2, 3, 5, 6) has the id of its entry; a person (1, 4, 10, 12) has cards that describe it
        let entry = matches!(n, 2 | 3 | 5 | 6);
        let found = profiles.iter().find(|p| {
            p.card.display_name() == name
                && if entry {
                    format!("urn:uuid:{}", p.id) == iri(n)
                } else {
                    p.describes.as_deref() == Some(&iri(n))
                }
        });
        let Some(p) = found else {
            out.push(format!(
                "entry {n} ({name}): no sample profile with that id/link and name"
            ));
            continue;
        };
        if entry {
            let want = view.describes.map(|d| d.id.canonical());
            if p.describes != want {
                out.push(format!(
                    "entry {n}: describes {:?}, directory {want:?}",
                    p.describes
                ));
            }
        } else if view.person.art != PersonArt::Person {
            out.push(format!("entry {n} is no person in the directory"));
        }
        let address =
            sample_address(n).or_else(|| sample_address(1).filter(|_| matches!(n, 2 | 3)));
        if let Some(a) = address {
            let mail = a.strip_prefix("mailto:").unwrap();
            if !vals(p, "emails").iter().any(|e| e == mail) {
                out.push(format!(
                    "entry {n}: address {mail} missing in {:?}",
                    vals(p, "emails")
                ));
            }
        }
        // the contact fields of the card, in order, as `sample_card` has them
        let mut want = (Vec::new(), Vec::new(), Vec::new());
        for v in sample_card(n) {
            match v {
                ExcerptValue::Email(e) => want.0.push(e.as_str().to_owned()),
                ExcerptValue::Phone(t) => want.1.push(format!("tel:{}", t.as_str())),
                ExcerptValue::Postal(a) => want.2.push(a.display()),
                _ => {}
            }
        }
        for (field, want) in [
            ("emails", &want.0),
            ("phones", &want.1),
            ("addresses", &want.2),
        ] {
            if !want.is_empty() && &vals(p, field) != want {
                out.push(format!(
                    "entry {n}: {field} {:?}, directory {want:?}",
                    vals(p, field)
                ));
            }
        }
    }
    out
}

/// A copy with `edit` applied to the card (or the id/link) of the profile named `name`.
fn diverge(name: &str, edit: impl FnOnce(&mut Profile)) -> Vec<Profile> {
    let mut all = sample_profiles();
    edit(
        all.iter_mut()
            .find(|p| p.card.display_name() == name)
            .unwrap(),
    );
    all
}

fn recard(p: &mut Profile, f: impl FnOnce(&mut serde_json::Value)) {
    let mut v = p.card.to_value();
    f(&mut v);
    p.card = serde_json::from_value::<Card>(v).unwrap();
}

#[tokio::test]
async fn the_sample_is_the_directory_sample() {
    assert_eq!(problems(&sample_profiles()).await, Vec::<String>::new());
}

/// Entry 1 is the anchor (the very IRI of events and tasks), Tariq is entry 12 (a real account, not a contact), "Amara Z." links
/// to the anchor, "A. Zolenko" describes nobody: the states the directory has.
#[tokio::test]
async fn the_people_are_the_ones_of_the_directory() {
    let all = sample_profiles();
    let anchor = sample_anchor().urn();
    assert_eq!(anchor, "urn:uuid:00000000-0000-7000-8000-000000000001");
    let by = |name: &str| all.iter().find(|p| p.card.display_name() == name).unwrap();
    assert_eq!(by("Amara Z.").describes.as_deref(), Some(anchor.as_str()));
    assert_eq!(by("A. Zolenko").describes, None);
    assert_eq!(by("Tariq Lindqvist").describes, Some(iri(12)));
    assert_eq!(by("Assistent Sol").describes, Some(iri(4)));
    assert_eq!(by("Soo-jin Marchetti").describes, Some(iri(10)));
    assert_eq!(
        by("Amara Zolenko").describes.as_deref(),
        Some(anchor.as_str())
    );
    assert!(
        all.iter().all(|p| format!("urn:uuid:{}", p.id) != anchor),
        "no profile has the id of the person"
    );
}

#[tokio::test]
async fn a_diverging_copy_is_red() {
    let id = diverge("Amara Z.", |p| {
        p.id = "01900000-0000-7000-8000-0000000000a0".parse().unwrap()
    });
    assert!(!problems(&id).await.is_empty(), "another id");
    let link = diverge("A. Zolenko", |p| p.describes = Some(iri(1)));
    assert!(!problems(&link).await.is_empty(), "another link");
    let tariq = diverge("Tariq Lindqvist", |p| p.describes = None);
    assert!(!problems(&tariq).await.is_empty(), "Tariq as a contact");
    let name = diverge("Assistent Sol", |p| {
        recard(p, |v| v["name"]["full"] = "Assistent Soll".into())
    });
    assert!(!problems(&name).await.is_empty(), "another name");
    let mail = diverge("Tariq Lindqvist", |p| {
        recard(p, |v| {
            v["emails"]["e1"]["address"] = "tariq@example.org".into()
        });
    });
    assert!(!problems(&mail).await.is_empty(), "another address");
    let phone = diverge("Amara Zolenko", |p| {
        recard(p, |v| {
            v["phones"]["p1"]["number"] = "tel:+4930000000".into()
        });
    });
    assert!(!problems(&phone).await.is_empty(), "another phone");
}
