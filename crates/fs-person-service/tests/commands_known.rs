//! Every command key of `catalog/commands.json` is on the capability list of fs-interfaces.
//!
//! Pattern: Contract test – the file is the declaration, `fs_interfaces::capabilities` (the list the host reads) is the
//! contract. A key the list does not know makes the host refuse the module, so it turns red here, not at install time.

use fs_interfaces::capabilities::{is_known, is_well_formed};
use serde_json::Value;

fn keys() -> Vec<String> {
    let file: Value = serde_json::from_str(include_str!("../../../catalog/commands.json"))
        .expect("catalog/commands.json is not valid JSON");
    file["commands"]
        .as_array()
        .expect("catalog/commands.json: `commands` is not an array")
        .iter()
        .map(|c| {
            c["capability"]
                .as_str()
                .expect("catalog/commands.json: a command has no string `capability`")
                .to_owned()
        })
        .collect()
}

#[test]
fn every_command_key_is_on_the_capability_list() {
    let keys = keys();
    assert!(!keys.is_empty(), "the catalog names no command");
    for key in &keys {
        assert!(
            is_well_formed(key),
            "`{key}` is not a well-formed capability name"
        );
        assert!(
            is_known(key),
            "`{key}` is not on the capability list of fs-interfaces"
        );
    }
}
