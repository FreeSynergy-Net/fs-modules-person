//! The list of people: the profiles that are people, page by page.
//!
//! Pattern: Specification (`Profile::listed`) over a Repository page – the filter and the paging are ONE step: a page holds
//! `limit` LISTED profiles (never fewer because a group card stood in between), and the cursor is a place in the whole
//! sequence of the store, so the next page starts right after the last profile this one took. The total counts listed profiles.

use fs_objects::Result;

use crate::{Profile, ProfileRepo};

/// How many profiles one read of the store takes.
const BATCH: usize = 100;

/// One page of the list.
#[derive(Debug)]
#[non_exhaustive]
pub struct Listing {
    /// The listed profiles, in name order.
    pub profiles: Vec<Profile>,
    /// The place (counted in ALL profiles, listed or not) the next page starts at; `None` = this is the last page.
    pub next: Option<usize>,
}

/// Up to `limit` listed profiles from the place `from` on (a place a previous page named, or 0).
///
/// # Errors
/// The store fails or a profile does not read as a card.
pub async fn list_page<R: ProfileRepo + ?Sized>(
    repo: &R,
    from: usize,
    limit: usize,
) -> Result<Listing> {
    let (mut taken, mut place, mut after_last) = (Vec::new(), from, from);
    loop {
        let batch = repo.page(place, BATCH).await?;
        let read = batch.len();
        for (i, p) in batch.into_iter().enumerate() {
            if !p.listed() {
                continue;
            }
            if taken.len() == limit {
                // one more listed profile exists: the next page starts right after the last one taken
                return Ok(Listing {
                    profiles: taken,
                    next: Some(after_last),
                });
            }
            taken.push(p);
            after_last = place + i + 1;
        }
        if read < BATCH {
            return Ok(Listing {
                profiles: taken,
                next: None,
            });
        }
        place += read;
    }
}

/// How many profiles the list of people holds (all listed ones; a read of the whole store, in batches).
///
/// # Errors
/// The store fails or a profile does not read as a card.
pub async fn list_total<R: ProfileRepo + ?Sized>(repo: &R) -> Result<usize> {
    let (mut n, mut place) = (0, 0);
    loop {
        let batch = repo.page(place, BATCH).await?;
        let read = batch.len();
        n += batch.iter().filter(|p| p.listed()).count();
        if read < BATCH {
            return Ok(n);
        }
        place += read;
    }
}
