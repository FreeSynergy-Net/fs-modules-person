//! The words of the module: `texts/en.mf2` and `texts/de.mf2` pair up, and cover exactly the labels of the type catalog.
//!
//! Pattern: Coverage test – the labels come from `ProfileType::describe()` (the one source), the words are the sources of
//! `pack-texts`; a missing word or a word without a label is red.

use std::collections::BTreeSet;

use fs_components_mf2::{Source, check_pairs};
use fs_person::ProfileType;
use fs_types::DescribesType as _;

const EN: &str = include_str!("../../../texts/en.mf2");
const DE: &str = include_str!("../../../texts/de.mf2");

fn ids(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter(|l| !l.starts_with(['#', ' ', '\t']) && !l.trim().is_empty())
        .filter_map(|l| l.split_once(" = ").map(|(id, _)| id.to_owned()))
        .collect()
}

fn labels() -> BTreeSet<String> {
    let d = ProfileType::describe();
    let mut out = BTreeSet::from([d.label.clone()]);
    for f in d.fields {
        out.extend(f.choices.iter().map(|c| format!("{}.{c}", f.label)));
        out.insert(f.label);
    }
    out
}

#[test]
fn the_two_languages_pair_up() {
    let sources = [
        Source {
            language: "en",
            text: EN,
        },
        Source {
            language: "de",
            text: DE,
        },
    ];
    assert_eq!(check_pairs(&sources), Vec::<String>::new());
}

#[test]
fn every_label_has_a_word_in_both_languages_and_no_word_is_left_over() {
    let labels = labels();
    for (language, text) in [("en", EN), ("de", DE)] {
        let words = ids(text);
        let missing: Vec<_> = labels.difference(&words).collect();
        let left_over: Vec<_> = words.difference(&labels).collect();
        assert!(missing.is_empty(), "{language}: no word for {missing:?}");
        assert!(
            left_over.is_empty(),
            "{language}: word without a label {left_over:?}"
        );
    }
}
