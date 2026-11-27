//! Which profiles describe "me": the lookup behind `person.update` without an `id` and behind the check of `person.profile.create`.
//!
//! Pattern: Query (one read over the port [`PersonRepo`], no state). Deviation: none.

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`

use fs_interfaces::module::Refusal;
use fs_objects::ObjectId;
use fs_person::PersonRepo;

use crate::forms::{PAGE_MAX, broken, refuse};

/// The ids of all profiles that describe "me" (empty when "me" does not exist yet).
pub(crate) async fn describing_me<R: PersonRepo + ?Sized>(
    repo: &R,
) -> Result<Vec<ObjectId>, Refusal> {
    let Some(me) = repo.me().await.map_err(|e| broken(&e))? else {
        return Ok(Vec::new());
    };
    let urn = me.urn();
    let mut found = Vec::new();
    let mut from = 0;
    loop {
        let page = repo
            .page(from, PAGE_MAX as usize)
            .await
            .map_err(|e| broken(&e))?;
        let n = page.len();
        found.extend(
            page.iter()
                .filter(|p| p.describes.as_deref() == Some(urn.as_str()))
                .map(|p| p.id),
        );
        if n < PAGE_MAX as usize {
            break;
        }
        from += n;
    }
    Ok(found)
}

/// The one profile that describes "me": none or several is refused, the caller names the `id`.
pub(crate) async fn mine<R: PersonRepo + ?Sized>(repo: &R) -> Result<ObjectId, Refusal> {
    if repo.me().await.map_err(|e| broken(&e))?.is_none() {
        return Err(refuse("unknown-record", 404, "\"me\" is not created yet"));
    }
    match describing_me(repo).await?.as_slice() {
        [one] => Ok(*one),
        [] => Err(refuse(
            "unknown-record",
            404,
            "no profile describes \"me\" yet: create one with describes=me",
        )),
        _ => Err(refuse(
            "conflict",
            409,
            "several profiles describe \"me\": name the `id`",
        )),
    }
}
