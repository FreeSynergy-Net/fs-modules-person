//! The person module as a `ModuleCore` for `fs_rpc::Guarded`: the forms `me`, `rows` (the profiles as a list) and
//! `single-record` (one profile as a whole), and the command `me.create`, and nothing else.
//!
//! Pattern: Template Method (steps) – this delivers only `page` and `blob` (no image or file field); the order
//! check -> change -> report stands once in `Guarded<PersonCore>`. Adapter: the port [`PersonRepo`] (the anchor and the profiles)
//! as the records of `catalog/forms.json` (`pages.rs`, `rows.rs`). Deviation: `apply` knows only `me.create` (it takes no argument:
//! nobody names an actor, the log names the anchor as the actor of its own birth); `revert` answers `not-revertible` (the one
//! birth is not taken back) and `changes` is empty, and there is no feed; the profiles are read only (nothing writes them).
//! Rights are asked by `Guarded` (`rules.rs`) and, once more before any read of the store, by `pages::refused`; the actor is the
//! connection, never an argument.

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`
#![allow(clippy::unused_async_trait_impl)] // the trait is async, some methods have nothing to await

use std::sync::Arc;

use fs_interfaces::module::{
    Applied, Blob, ChangeFeed, ChangePage, ChangesQuery, Context, Invocation, ModuleCore, Page,
    PageQuery, Refusal, RightsRules, SAMPLE_PROVIDER, StepRef,
};
use fs_person::PersonRepo;

use crate::forms::{COMMAND_CREATE, LEVEL, PROVIDER, broken, no_args, refuse, served};
use crate::owner::Owner;
use crate::pages::{page_of, refused};
use crate::rules::PersonRules;

/// The person module over one repository of the anchor and the profiles.
pub struct PersonCore<R> {
    repo: Arc<R>,
    owner: Owner,
    rules: Box<dyn RightsRules>,
}

impl<R: PersonRepo + 'static> PersonCore<R> {
    /// The module over `repo`, whose person is `owner` (read once, see [`Owner::read`]), with the rules of the owner ([`PersonRules`]).
    #[must_use]
    pub fn new(repo: Arc<R>, owner: Owner) -> Self {
        let rules = Box::new(PersonRules::new(owner.clone()));
        Self::with_rules(repo, owner, rules)
    }

    /// The module with other rules (a test that shows what the rules are for).
    #[must_use]
    pub fn with_rules(repo: Arc<R>, owner: Owner, rules: Box<dyn RightsRules>) -> Self {
        Self { repo, owner, rules }
    }

    /// The one birth, shared by `me.create` and the start of the service ([`PersonCore::ensure_me`]): the repository births the
    /// reserved anchor once (the anchor is the actor of its own birth), and says `me-exists` where "me" was there already.
    pub(crate) async fn birth(&self) -> Result<Applied, Refusal> {
        let made = self
            .repo
            .create_me_as(self.owner.reserved())
            .await
            .map_err(|e| broken(&e))?;
        // another handle may have been first: new connections act for the stored anchor
        self.owner.settle(&made.anchor);
        let mut done = Applied::default();
        done.record = made.anchor.urn();
        (if made.new { "me-created" } else { "me-exists" }).clone_into(&mut done.text);
        Ok(done)
    }

    /// The owner of this installation (for the context of the service itself, `bootstrap.rs`).
    pub(crate) fn owner(&self) -> &Owner {
        &self.owner
    }
}

impl<R: PersonRepo + 'static> ModuleCore for PersonCore<R> {
    fn level(&self) -> u32 {
        LEVEL
    }

    /// `person`, or `fake` (`SAMPLE_PROVIDER`) when the repository is the sample data: a host shows it as sample data, never as real.
    fn provider(&self) -> &str {
        if self.repo.is_sample() {
            SAMPLE_PROVIDER
        } else {
            PROVIDER
        }
    }

    fn rules(&self) -> &dyn RightsRules {
        self.rules.as_ref()
    }

    async fn page(&self, cx: &Context, q: PageQuery) -> Result<Page, Refusal> {
        let Some(form) = served(&q.form) else {
            let why = "this module draws me@1.0, rows@1.0 and single-record@1.2";
            return Err(refuse("unknown-form", 404, why));
        };
        if refused(self.rules(), cx).is_some() {
            return Ok(Page::default());
        }
        page_of(&*self.repo, form, &q).await
    }

    async fn blob(&self, _cx: &Context, _record: &str, _field: &str) -> Result<Blob, Refusal> {
        Err(refuse(
            "unsupported",
            501,
            "this module has no image or file field",
        ))
    }

    async fn apply(&self, _cx: &Context, c: Invocation) -> Result<Applied, Refusal> {
        match (c.command.as_str(), c.record.as_deref().unwrap_or("")) {
            (COMMAND_CREATE, "") => {
                no_args(&c.args)?;
                self.birth().await
            }
            (COMMAND_CREATE, _) => Err(refuse(
                "bad-arguments",
                400,
                format!("{COMMAND_CREATE} is about the set, not about a record"),
            )),
            (other, _) => Err(refuse(
                "unknown-command",
                404,
                format!("no command {other:?}"),
            )),
        }
    }

    async fn revert(
        &self,
        _cx: &Context,
        _change: &str,
        _step: StepRef,
    ) -> Result<Applied, Refusal> {
        Err(refuse(
            "not-revertible",
            422,
            "the birth of the one person is not taken back",
        ))
    }

    async fn changes(&self, _cx: &Context, _q: ChangesQuery) -> Result<ChangePage, Refusal> {
        Ok(ChangePage::default())
    }

    fn feed(&self, _sink: ChangeFeed) {
        // No change feed: the one birth is told by the answer of `me.create`.
    }
}
