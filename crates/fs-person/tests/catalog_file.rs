//! `catalog/types.bin` is the type catalog of the module in the wire form of `types.capnp`: the file equals what the code writes,
//! it reads back to the same description, and the labels and `multiple` are those of the field table.
//!
//! Pattern: Golden file test – the checked-in file is the expectation; `FS_BLESS=1` renews it, nothing else does.

use std::path::PathBuf;

use fs_person::{PROFILE, ProfileType, read_type_catalog, type_catalog_bytes, values_of};
use fs_types::DescribesType as _;

fn file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../catalog/types.bin")
}

#[test]
fn the_file_is_what_the_code_writes() {
    let bytes = type_catalog_bytes();
    if std::env::var_os("FS_BLESS").is_some() {
        std::fs::write(file(), &bytes).unwrap();
    }
    let on_disk =
        std::fs::read(file()).expect("catalog/types.bin is missing: FS_BLESS=1 creates it");
    assert!(
        on_disk == bytes,
        "catalog/types.bin differs from the catalog of the code: FS_BLESS=1 renews it"
    );
    assert_eq!(
        type_catalog_bytes(),
        bytes,
        "canonical: the same catalog is the same bytes"
    );
}

#[test]
fn it_reads_back_to_the_description_of_the_module() {
    let registry = read_type_catalog(&std::fs::read(file()).unwrap()).unwrap();
    assert_eq!(registry.len(), 1);
    let read = registry.get(PROFILE).expect("the profile type");
    let described = ProfileType::describe();
    assert_eq!(read.name, described.name);
    assert_eq!(read.program, described.program);
    assert_eq!(read.standard, described.standard);
    assert_eq!(read.label, described.label);
    assert_eq!(read.fields.len(), described.fields.len());
    for (a, b) in read.fields.iter().zip(&described.fields) {
        assert_eq!(
            (
                &a.name,
                a.basic,
                a.multiple,
                a.read_only,
                &a.label,
                &a.target,
                &a.choices
            ),
            (
                &b.name,
                b.basic,
                b.multiple,
                b.read_only,
                &b.label,
                &b.target,
                &b.choices
            )
        );
    }
}

#[test]
fn every_label_is_person_and_multiple_follows_the_field_table() {
    let registry = read_type_catalog(&type_catalog_bytes()).unwrap();
    let profile = registry.get(PROFILE).unwrap();
    assert_eq!(profile.label, "person");
    // the one table behind the description and `values_of`: a field that repeats is `multiple`
    let multiple: Vec<&str> = profile
        .fields
        .iter()
        .filter(|f| f.multiple)
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(
        multiple,
        [
            "members",
            "relatedTo",
            "nicknames",
            "organizations",
            "titles",
            "pronouns",
            "emails",
            "onlineServices",
            "phones",
            "preferredLanguages",
            "calendars",
            "schedulingAddresses",
            "addresses",
            "cryptoKeys",
            "directories",
            "links",
            "media",
            "anniversaries",
            "notes",
            "personalInfo"
        ]
    );
    for f in &profile.fields {
        assert!(f.label.starts_with("person."), "{}: {}", f.name, f.label);
        // every named field is also known to `values_of` (None only for the fields the card does not hold, e.g. describes)
        if f.name != "describes" && f.name != "media" {
            assert!(
                values_of(&fs_person::sample_profiles()[0].card, &f.name).is_some(),
                "{}",
                f.name
            );
        }
    }
}

#[test]
fn a_damaged_file_is_an_error_not_a_catalog() {
    let mut bytes = type_catalog_bytes();
    bytes.truncate(bytes.len() / 2);
    assert!(read_type_catalog(&bytes).is_err());
    assert!(read_type_catalog(&[]).is_err());
}
