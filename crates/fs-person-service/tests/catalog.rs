//! M3: `catalog/forms.json` says what `schema/forms.txt` of fs-interfaces says about the forms it names (a red test where they
//! drift apart), and what the service draws is what the catalog declares.
//!
//! Pattern: Contract test – the file is the declaration, `fs_interfaces::forms::catalog()` (the reader of forms.txt) is the
//! contract. Every channel the module maps exists, a required one is mapped, the count and the art of every field fit.

use std::collections::BTreeSet;

use fs_interfaces::forms::{Art, Channel, find_key};
use fs_person::{DESCRIBES, ProfileType};
use fs_person_service::{FORM_RECORD, FORM_ROWS, record_fields};
use fs_types::{BasicType, DescribesType as _};
use serde_json::Value;

fn catalog() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("../../../catalog/forms.json"))
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn entry(name: &str) -> Value {
    catalog()
        .into_iter()
        .find(|f| f["name"] == name && f["type"] == "Profile")
        .unwrap_or_else(|| panic!("no entry {name} of type Profile"))
}

/// The key `name@x.y.z` the entry declares.
fn key_of(f: &Value) -> String {
    format!(
        "{}@{}",
        f["name"].as_str().unwrap(),
        f["version"].as_str().unwrap()
    )
}

/// The word of `fields` a basic type of the module is written as (the vocabulary of the forms: `Phone`, `Name` ... are text).
fn word(b: BasicType) -> &'static str {
    match b {
        BasicType::Email => "email",
        BasicType::Date => "date",
        BasicType::DateTime => "date-time",
        BasicType::Link => "link",
        BasicType::Image => "image",
        BasicType::Choice => "choice",
        BasicType::Keywords => "keywords",
        BasicType::Reference => "reference",
        BasicType::Integer | BasicType::Whole => "number",
        BasicType::Flag => "bool",
        _ => "text",
    }
}

/// The name of the basic as `Art::allowed_basics` writes it.
fn basic_name(word: &str) -> &'static str {
    match word {
        "email" => "Email",
        "date" => "Date",
        "date-time" => "DateTime",
        "link" => "Link",
        "image" => "Image",
        "choice" => "Choice",
        "keywords" => "Keywords",
        "reference" => "Reference",
        "number" => "Number",
        "bool" => "Bool",
        _ => "Text",
    }
}

fn mapped(f: &Value, channel: &str) -> Vec<String> {
    match &f["map"][channel] {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a.iter().map(|s| s.as_str().unwrap().to_owned()).collect(),
        _ => vec![],
    }
}

fn check_entry(f: &Value) {
    let key = key_of(f);
    let form = find_key(&key).unwrap_or_else(|| panic!("{key} is no form of schema/forms.txt"));
    let map = f["map"].as_object().unwrap();
    for (name, _) in map {
        let channel: &Channel = form
            .channel(name)
            .unwrap_or_else(|| panic!("{key}: `{name}` is no channel of the form"));
        assert!(
            !channel.accepts(Art::Command),
            "{key}: `{name}` is a command, nothing writes"
        );
        let fields = mapped(f, name);
        assert!(
            channel.count.allows(fields.len()),
            "{key}: `{name}` maps {} fields",
            fields.len()
        );
        if channel.accepts(Art::Records) {
            continue;
        }
        for field in &fields {
            let kind = f["fields"][field]
                .as_str()
                .unwrap_or_else(|| panic!("{key}: field `{field}` has no type"));
            assert!(
                channel.accepts_basic(basic_name(kind)),
                "{key}: `{field}` ({kind}) does not fit `{name}`"
            );
        }
    }
    for c in form.required() {
        assert!(
            map.contains_key(&c.name),
            "{key}: required channel `{}` is not mapped",
            c.name
        );
    }
}

#[test]
fn the_catalog_agrees_with_forms_txt() {
    let forms = catalog();
    assert_eq!(forms.len(), 3, "me, rows, single-record");
    // `me` is the form of this module's own contract (scope own-contract): forms.txt does not know it, so nothing is compared
    assert!(find_key("me@1.0.0").is_none());
    for f in forms.iter().filter(|f| f["name"] != "me") {
        check_entry(f);
    }
}

#[test]
fn the_service_names_the_versions_the_catalog_declares() {
    assert_eq!(key_of(&entry("rows")), FORM_ROWS);
    assert_eq!(key_of(&entry("single-record")), FORM_RECORD);
    // nothing writes a profile: no command channel of the two list forms is mapped, `open` and `actions` neither
    for name in ["rows", "single-record"] {
        let f = entry(name);
        for channel in ["open", "actions", "edit", "create", "remove"] {
            assert!(
                f["map"].get(channel).is_none(),
                "{name}: `{channel}` writes"
            );
        }
    }
}

#[test]
fn the_list_has_the_columns_and_the_record_has_the_fields_of_the_decision() {
    let rows = entry("rows");
    assert_eq!(
        mapped(&rows, "columns"),
        ["organizations", "titles", "emails", "phones", "nicknames"]
    );
    assert_eq!(mapped(&rows, "items"), ["Profile"]);
    assert_eq!(mapped(&rows, "state"), ["state"]);
    assert_eq!(mapped(&rows, "preview"), ["media"]);
    let record = entry("single-record");
    // the table of the type without uid and prodId (and without the picture, which is the preview), plus describes
    assert_eq!(mapped(&record, "fields"), record_fields());
    let table: BTreeSet<String> = ProfileType::describe()
        .fields
        .into_iter()
        .map(|f| f.name)
        .collect();
    let sent: BTreeSet<String> = record_fields().into_iter().collect();
    assert!(sent.contains(DESCRIBES));
    let left_out: Vec<_> = table.difference(&sent).cloned().collect();
    assert_eq!(left_out, ["media", "prodId", "uid"]);
    // the declared type of every field is the type the module describes it with
    for f in ProfileType::describe().fields {
        if let Some(declared) = record["fields"][&f.name].as_str() {
            assert_eq!(declared, word(f.basic), "{}", f.name);
        }
    }
}
