//! The pages of the module: the form `me`, the lists `rows` and `tiles` and the single record.
//!
//! Pattern: Template Method (steps, the part this module owns) – one function per form, each reads through the ports and
//! projects (`rows.rs`); the check of rights stays in `Guarded`. Deviation: before any read each page asks the rules once whether
//! this context may read at all (`RightsRules::hidden` of the whole context), so a caller who may not (a bot, another person) makes
//! no read of the store and sees no total: an unknown id and a known one look the same to them. Under `AllowAll` rules (a test) it
//! reads, as the wrapper would.

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`

use fs_interfaces::module::{Context, HiddenReason, Page, PageQuery, Refusal, RightsRules};
use fs_objects::ObjectId;
use fs_person::{Anchor, PersonRepo, list_page, list_total};

use crate::forms::{PAGE_MAX, SCOPE_ALL, Served, broken, refuse};
use crate::rows::{me_record, profile_record, profile_records};

/// Whether `rules` refuse this context every read (nothing is read for it).
pub(crate) fn refused(rules: &dyn RightsRules, cx: &Context) -> Option<HiddenReason> {
    rules.hidden(cx, "", "")
}

/// The page `q` asks for, from `repo`.
pub(crate) async fn page_of<R: PersonRepo + ?Sized>(
    repo: &R,
    form: Served,
    q: &PageQuery,
) -> Result<Page, Refusal> {
    match form {
        Served::Me => me_page(repo).await,
        Served::Rows | Served::Tiles => rows_page(repo, q).await,
        Served::Record => record_page(repo, &q.scope).await,
    }
}

async fn me_page<R: PersonRepo + ?Sized>(repo: &R) -> Result<Page, Refusal> {
    let me = repo.me().await.map_err(|e| broken(&e))?;
    let mut page = Page::default();
    page.total = Some(u64::from(me.is_some()));
    // the set may "create" while there is no "me"; the one record has nothing to run
    if me.is_none() {
        page.may = vec![crate::forms::COMMAND_CREATE.to_owned()];
    }
    page.records = me.iter().map(me_record).collect();
    Ok(page)
}

fn limit_of(limit: u32) -> usize {
    (match limit {
        0 => PAGE_MAX,
        n => n.min(PAGE_MAX),
    }) as usize
}

/// The place a cursor of this module says (empty = the start); anything else is `bad-arguments`.
fn place_of(cursor: &str) -> Result<usize, Refusal> {
    if cursor.is_empty() {
        return Ok(0);
    }
    cursor
        .parse()
        .map_err(|_| refuse("bad-arguments", 400, "the cursor is not one of this module"))
}

async fn rows_page<R: PersonRepo + ?Sized>(repo: &R, q: &PageQuery) -> Result<Page, Refusal> {
    if !(q.scope.is_empty() || q.scope == SCOPE_ALL) {
        let why = format!(
            "the list of profiles has the scope {SCOPE_ALL:?}, not {:?}",
            q.scope
        );
        return Err(refuse("unknown-scope", 404, why));
    }
    let from = place_of(&q.cursor)?;
    let me = repo.me().await.map_err(|e| broken(&e))?;
    let listing = list_page(repo, from, limit_of(q.limit))
        .await
        .map_err(|e| broken(&e))?;
    let total = list_total(repo).await.map_err(|e| broken(&e))?;
    let mut page = Page::default();
    page.records = profile_records(&listing.profiles, me.as_ref());
    page.total = u64::try_from(total).ok();
    page.next = listing.next.map(|n| n.to_string()).unwrap_or_default();
    Ok(page)
}

/// The id of a profile a scope names: `urn:uuid:<uuid>` (as in the rows) or the bare UUID.
fn profile_id(scope: &str) -> Option<ObjectId> {
    scope
        .strip_prefix("urn:uuid:")
        .unwrap_or(scope)
        .parse()
        .ok()
}

async fn record_page<R: PersonRepo + ?Sized>(repo: &R, scope: &str) -> Result<Page, Refusal> {
    let Some(id) = profile_id(scope) else {
        let why = "the scope of a single profile is the id of the profile (urn:uuid:...)";
        return Err(refuse("unknown-scope", 404, why));
    };
    let me: Option<Anchor> = repo.me().await.map_err(|e| broken(&e))?;
    let found = repo.get(id).await.map_err(|e| broken(&e))?;
    let mut page = Page::default();
    page.records = found
        .iter()
        .map(|p| profile_record(p, me.as_ref()))
        .collect();
    page.total = Some(u64::from(!page.records.is_empty()));
    Ok(page)
}
