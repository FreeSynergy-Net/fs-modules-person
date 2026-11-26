//! What the module says about itself, and the refusals it speaks.
//!
//! Pattern: Adapter – the names of `catalog/forms.json` and `catalog/commands.json` as constants, and a failure of the store or
//! of a call as the `Refusal` of the module contract (`fs_interfaces::module`); the wrapper adds the prefix `form-source-`.

#![allow(clippy::field_reassign_with_default)] // `Refusal` is `#[non_exhaustive]`: built by Default plus assignment
#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`

use fs_interfaces::forms::parse_key;
use fs_interfaces::module::{Refusal, Value};

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "person";
/// The level of `FormSource` this module says: 1 = page and invoke (the wrapper answers the levels above; nothing here is logged
/// for taking back, so `revert` is `not-revertible` and `changes` is empty).
pub const LEVEL: u32 = 1;
/// The form of the one person, as `name@x.y.z`; `catalog/forms.json` declares it as `1.0`.
pub const FORM_ME: &str = "me@1.0.0";
/// The list of the profiles (type `Profile`, scope `all`); `catalog/forms.json` declares it as `1.0.0`.
pub const FORM_ROWS: &str = "rows@1.0.0";
/// One profile as a whole (scope = the id of the profile; the fields repeat per item); `catalog/forms.json` declares it as `1.2.0`.
pub const FORM_RECORD: &str = "single-record@1.2.0";
/// The same profiles as tiles (type `Profile`, scope `all`); `catalog/forms.json` declares it as `1.1.0`. The records are those of `rows`; the
/// catalog maps the preferred value of four columns as facts (`nicknames` is not mapped, so the host never sees it here).
pub const FORM_TILES: &str = "tiles@1.1.0";
/// The scope of the list: every profile that is a person.
pub const SCOPE_ALL: &str = "all";
/// The one command, the capability key of `catalog/commands.json`.
pub const COMMAND_CREATE: &str = "me.create";
/// The most profiles of one page (and the page when the caller names no limit).
pub const PAGE_MAX: u32 = 500;

/// Which of its forms a key names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Served {
    /// `me@1.0`.
    Me,
    /// `rows@1.0`.
    Rows,
    /// `single-record@1.2`.
    Record,
    /// `tiles@1.1`: the records of `Rows`, drawn as tiles.
    Tiles,
}

/// The form a key `name@x.y` or `name@x.y.z` names (the patch does not count), if this module draws it.
pub(crate) fn served(form: &str) -> Option<Served> {
    let (name, v) = parse_key(form)?;
    match (name, v.major, v.minor) {
        ("me", 1, 0) => Some(Served::Me),
        ("rows", 1, 0) => Some(Served::Rows),
        ("tiles", 1, 1) => Some(Served::Tiles),
        ("single-record", 1, 2) => Some(Served::Record),
        _ => None,
    }
}

/// A refusal of `kind` (without the prefix of the wire), with its status and a reason for people.
pub(crate) fn refuse(kind: &str, status: u16, detail: impl Into<String>) -> Refusal {
    let mut r = Refusal::default();
    kind.clone_into(&mut r.kind);
    r.status = status;
    r.detail = detail.into();
    r
}

/// A store that fails or holds something that is no anchor: told with its words, never guessed.
pub(crate) fn broken(e: &impl std::fmt::Display) -> Refusal {
    refuse(
        "internal",
        500,
        format!("the store of the module failed: {e}"),
    )
}

/// `me.create` takes nothing: any value (an `actor`, an `owner`, whatever) is refused with its reason, before anything is written.
pub(crate) fn no_args(args: &[Value]) -> Result<(), Refusal> {
    match args.first() {
        None => Ok(()),
        Some(a) => Err(refuse(
            "bad-arguments",
            400,
            format!(
                "unknown argument {:?}: {COMMAND_CREATE} takes none, nobody names an actor",
                a.path
            ),
        )),
    }
}
