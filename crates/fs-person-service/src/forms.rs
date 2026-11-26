//! What the module says about itself, and the refusals it speaks.
//!
//! Pattern: Adapter – the names of `catalog/forms.json` and `catalog/commands.json` as constants, and a failure of the store or
//! of a call as the `Refusal` of the module contract (`fs_interfaces::module`); the wrapper adds the prefix `form-source-`.

#![allow(clippy::field_reassign_with_default)] // `Refusal` is `#[non_exhaustive]`: built by Default plus assignment

use fs_interfaces::forms::parse_key;
use fs_interfaces::module::{Refusal, Value};

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "person";
/// The level of `FormSource` this module says: 1 = page and invoke (the wrapper answers the levels above; nothing here is logged
/// for taking back, so `revert` is `not-revertible` and `changes` is empty).
pub const LEVEL: u32 = 1;
/// The form this module draws, as `name@x.y.z`; `catalog/forms.json` declares it as `1.0`.
pub const FORM_ME: &str = "me@1.0.0";
/// The one command, the capability key of `catalog/commands.json`.
pub const COMMAND_CREATE: &str = "me.create";

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

/// Whether `form` is the key of the form this module draws: the name `me`, major 1, minor 0 (`x.y` and `x.y.z`).
pub(crate) fn serves(form: &str) -> bool {
    parse_key(form).is_some_and(|(name, v)| name == "me" && (v.major, v.minor) == (1, 0))
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
