//! The anchor is born at the first start of the service, by the same birth and the same rules as `me.create`.
//!
//! Pattern: Lazy Initialization (bootstrap) – the first start makes the one thing every other part needs (the anchor, "me"), every
//! later start finds it and writes nothing (the repository answers `me-exists`). Deviation: it is not lazy at first use but at first
//! start, because the module is the source of "me" and nobody else calls `me.create` at installation.
//! No way round the rights: the service asks `RightsRules` for `me.create` as the person of this installation on a living
//! connection (itself), exactly as `Guarded` asks for a caller; a refusal ends the start. The birth is `PersonCore::birth`, the same
//! step `me.create` runs, and the log names the new anchor as the actor of its own birth.

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`

use fs_interfaces::module::{Actor, Applied, Ask, Caller, Context, ModuleCore, Refusal, Verdict};
use fs_person::PersonRepo;
use fs_types::Utc;

use crate::core::PersonCore;
use crate::forms::{COMMAND_CREATE, refuse};

/// The "system id" the service gives itself on its own context (there is no peer: the process is the connection).
pub(crate) const SELF_ID: &str = "fs-person-service";

/// "Now" for `Ask.at` (the clock of the system, never a value from a caller; the person rules do not read it).
fn now() -> Utc {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    Utc::from_millis(ms)
}

impl<R: PersonRepo + 'static> PersonCore<R> {
    /// Make sure the one anchor exists: births it if the folder has none, else changes nothing (`me-exists`, no second write).
    ///
    /// # Errors
    /// The rules refuse the service (`form-source-forbidden` 403, with the reason) or the store fails (`internal` 500).
    pub async fn ensure_me(&self) -> Result<Applied, Refusal> {
        let cx = Context::from_connection(
            Caller::Connection {
                system_id: SELF_ID.into(),
                system_name: Some(SELF_ID.into()),
            },
            Actor::Person,
            self.owner().person(),
        );
        match self
            .rules()
            .may(&Ask::new(&cx, COMMAND_CREATE, None, now()))
        {
            Verdict::Allow => self.birth().await,
            Verdict::Deny { reason } => Err(refuse(
                "form-source-forbidden",
                403,
                format!("the service may not birth its own anchor: {reason}"),
            )),
            // a verdict this code does not know counts as a refusal (fail closed)
            _ => Err(refuse(
                "form-source-forbidden",
                403,
                "the service may not birth its own anchor",
            )),
        }
    }
}
