//! The form source level `catalog/commands.json` declares is the level of the contract the module is built against.
//!
//! Pattern: Contract test – the file is the declaration (it becomes `Requires: freesynergy-form-source >= N` of the package),
//! `fs_interfaces::forms::FORM_SOURCE_LEVEL` is the contract. The packager cannot compare them, so this test does (drift
//! protection: a pin lift that raises the constant turns it red until the file follows).

use serde_json::Value;

fn declared_level() -> u64 {
    let file: Value = serde_json::from_str(include_str!("../../../catalog/commands.json"))
        .expect("catalog/commands.json is not valid JSON");
    file["form_source_level"]
        .as_u64()
        .expect("catalog/commands.json: `form_source_level` is not a whole number")
}

#[test]
fn declared_form_source_level_is_the_level_of_the_contract() {
    let declared = declared_level();
    assert!(declared >= 1, "the level must be >= 1");
    assert_eq!(declared, u64::from(fs_interfaces::forms::FORM_SOURCE_LEVEL));
}
