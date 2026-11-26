//! The rights of the person module, for `fs_rpc::Guarded`.
//!
//! Pattern: Strategy – the `RightsRules` of the data kind Person, asked by the wrapper before every change and per record before
//! every read. Honest: the transport admits only the OS user of the service (the owner), so these rules are the second lock, not the
//! first: only a PERSON, on a living connection, acting for THE person of this installation, may do anything. `me.create` and
//! reading `me` are the same right (the owner's); a command that is not a command of this module is NOT refused here, no right is
//! asked for what does not exist: the core answers `unknown-command` 404 (as in planning). That also lets `change.revert` and
//! `change.watch` through to the core, which answers them (`not-revertible`, an empty feed).
//! Limit: under `OsUserIsPerson` a bot that runs as the same OS user counts as a person; the rules can only tell a bot when the
//! port says so (`AllBots`, or a better port later).

use fs_interfaces::module::{Actor, Ask, Caller, Context, HiddenReason, RightsRules, Verdict};

use crate::owner::Owner;

/// The reason when the caller is not proven a person (fail closed: every actor that is not `Person`).
pub const NOT_A_PERSON: &str = "not-a-person";
/// The reason when there is no living connection behind the context.
pub const NO_CONNECTION: &str = "no-connection";
/// The reason when the context acts for another person than the owner of this installation.
pub const NOT_THE_OWNER: &str = "not-the-owner";

/// The rules of the person module: the owner, and only the owner.
#[derive(Clone, Debug)]
pub struct PersonRules {
    owner: Owner,
}

impl PersonRules {
    /// The rules for the installation whose person is `owner`.
    #[must_use]
    pub fn new(owner: Owner) -> Self {
        Self { owner }
    }

    /// Why `who` may not, or `None` if `who` is the owner. `!=`, never `== Bot`: an actor added later is refused too.
    fn refusal(&self, who: &Context) -> Option<&'static str> {
        if who.actor != Actor::Person {
            return Some(NOT_A_PERSON);
        }
        if !matches!(who.caller, Caller::Connection { .. }) {
            return Some(NO_CONNECTION);
        }
        (who.person != self.owner.person()).then_some(NOT_THE_OWNER)
    }
}

impl RightsRules for PersonRules {
    fn may(&self, ask: &Ask<'_>) -> Verdict {
        self.refusal(ask.who).map_or(Verdict::Allow, Verdict::deny)
    }

    fn hidden(&self, who: &Context, _record: &str, _field: &str) -> Option<HiddenReason> {
        self.refusal(who).map(|_| HiddenReason::NoRight)
    }
}
