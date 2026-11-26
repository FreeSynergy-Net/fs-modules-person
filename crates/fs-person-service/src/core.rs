//! The person module as a `ModuleCore` for `fs_rpc::Guarded`: the form `me` and the command `me.create`, and nothing else.
//!
//! Pattern: Template Method (steps) – this delivers only `page` and `blob` (the form `me`; no image or file field); the order
//! check -> change -> report stands once in `Guarded<PersonCore>`. Adapter: the port [`MeRepo`] as the records of `catalog/forms.json`.
//! Deviation: `apply` knows only `me.create` (it takes no argument: nobody names an actor, the log names the anchor as the actor
//! of its own birth); `revert` answers `not-revertible` (the one birth is not taken back) and `changes` is empty, and there is no
//! feed. Rights are NOT asked here (`rules.rs`): this core is safe only behind `Guarded`; the actor is the connection, never an argument.

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`
#![allow(clippy::unused_async_trait_impl)] // the trait is async, some methods have nothing to await

use std::sync::Arc;

use fs_interfaces::module::{
    Applied, Blob, ChangeFeed, ChangePage, ChangesQuery, Context, Invocation, ModuleCore, Page,
    PageQuery, Refusal, RightsRules, StepRef,
};
use fs_person::MeRepo;

use crate::forms::{COMMAND_CREATE, FORM_ME, LEVEL, PROVIDER, broken, no_args, refuse, serves};
use crate::owner::Owner;
use crate::rows::me_record;
use crate::rules::PersonRules;

/// The person module over one repository of "me".
pub struct PersonCore<R> {
    repo: Arc<R>,
    owner: Owner,
    rules: Box<dyn RightsRules>,
}

impl<R: MeRepo + 'static> PersonCore<R> {
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
}

impl<R: MeRepo + 'static> ModuleCore for PersonCore<R> {
    fn level(&self) -> u32 {
        LEVEL
    }

    fn provider(&self) -> &str {
        PROVIDER
    }

    fn rules(&self) -> &dyn RightsRules {
        self.rules.as_ref()
    }

    async fn page(&self, _cx: &Context, q: PageQuery) -> Result<Page, Refusal> {
        if !serves(&q.form) {
            let why = format!("this module draws {FORM_ME}");
            return Err(refuse("unknown-form", 404, why));
        }
        let me = self.repo.me().await.map_err(|e| broken(&e))?;
        let mut page = Page::default();
        page.total = Some(u64::from(me.is_some()));
        // the set may "create" while there is no "me"; the one record has nothing to run
        if me.is_none() {
            page.may = vec![COMMAND_CREATE.to_owned()];
        }
        page.records = me.iter().map(me_record).collect();
        Ok(page)
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
                let made = self
                    .repo
                    .create_me_as(self.owner.reserved())
                    .await
                    .map_err(|e| broken(&e))?;
                // another handle may have been first: new connections act for the stored anchor
                self.owner.settle(&made.anchor);
                let mut done = Applied::default();
                done.record = made.anchor.urn();
                done.text = (if made.new { "me-created" } else { "me-exists" }).to_owned();
                Ok(done)
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
