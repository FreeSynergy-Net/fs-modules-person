//! The three write commands on profiles: `person.profile.create`, `person.update`, `person.delete`.
//!
//! Pattern: Command (one function per command over the port [`PersonRepo`]) + Adapter (the text values of the wire as a checked map,
//! a failure of the store as a `Refusal`). Every value is text; an argument that is not named in `catalog/commands.json` or that comes
//! twice is refused with its reason, before anything is written. The wire name of a command is its capability key. The actor is the
//! connection (`Context::person`), never an argument; whether the caller may is asked by `Guarded` (`PersonRules`) before `apply`.
//! Deviation: none. A profile is never a person: `person.delete` refuses the anchor ("me") with its reason, and `person.update`
//! without an `id` means the one profile that describes "me" (none or several is told, never guessed).

#![allow(clippy::result_large_err)] // `Refusal` is the one refusal type of the contract `ModuleCore`

use std::collections::BTreeMap;

use fs_interfaces::module::{Applied, Context, Invocation, Refusal, Value};
use fs_objects::{By, ObjectId};
use fs_person::{NewProfile, PersonRepo, Profile, ProfilePatch};
use fs_types::BasicValue as _;

use crate::forms::{PAGE_MAX, broken, refuse};
use crate::pages::profile_id;

/// `person.profile.create`: a profile (a contact without an account); the answer is its id.
pub const COMMAND_PROFILE_CREATE: &str = "person.profile.create";
/// `person.update`: change the name, e-mail or phone of a profile (or of "me" without an `id`).
pub const COMMAND_UPDATE: &str = "person.update";
/// `person.delete`: delete a profile, never "me".
pub const COMMAND_DELETE: &str = "person.delete";

const CREATE_ARGS: [&str; 4] = ["name", "email", "phone", "describes"];
const UPDATE_ARGS: [&str; 4] = ["id", "name", "email", "phone"];
const DELETE_ARGS: [&str; 1] = ["id"];

/// The only value of `describes` that is taken: the profile then describes the one person of this installation.
pub const DESCRIBES_ME: &str = "me";

fn bad(why: impl Into<String>) -> Refusal {
    refuse("bad-arguments", 400, why)
}

/// The text values by path; a path that is not in `allowed` or that comes twice is refused with the reason, never dropped.
fn text_args(args: &[Value], allowed: &[&str]) -> Result<BTreeMap<String, String>, Refusal> {
    let mut got = BTreeMap::new();
    for a in args {
        if !allowed.contains(&a.path.as_str()) {
            return Err(bad(format!(
                "unknown argument {:?} (allowed: {})",
                a.path,
                allowed.join(", ")
            )));
        }
        if got.insert(a.path.clone(), a.text.clone()).is_some() {
            return Err(bad(format!("argument {:?} given twice", a.path)));
        }
    }
    Ok(got)
}

/// All three commands are about the set (the id is an argument), and none of them knows a compare-and-set yet.
fn set_level(c: &Invocation) -> Result<(), Refusal> {
    if c.record.as_deref().is_some_and(|r| !r.is_empty()) {
        return Err(bad(format!(
            "{} is about the set, the profile is the argument `id`, not the record",
            c.command
        )));
    }
    if c.expect.is_some() {
        return Err(refuse(
            "unsupported",
            501,
            format!(
                "{} has no compare-and-set yet: `expect` is not taken",
                c.command
            ),
        ));
    }
    Ok(())
}

/// A write needs the store to have its owner: the anchor is born first (`me.create`, or the start of the service).
async fn needs_me<R: PersonRepo + ?Sized>(repo: &R) -> Result<(), Refusal> {
    match repo.me().await.map_err(|e| broken(&e))? {
        Some(_) => Ok(()),
        None => Err(refuse(
            "rejected",
            422,
            "\"me\" is not created yet: create it first (me.create), then profiles",
        )),
    }
}

fn writable<R: PersonRepo + ?Sized>(repo: &R) -> Result<(), Refusal> {
    if repo.is_sample() {
        return Err(refuse(
            "forbidden",
            403,
            "the sample data is read only, nothing is written",
        ));
    }
    Ok(())
}

/// A store that fails is `internal`; a value the card does not take is `rejected` (422), as in planning.
fn store_failed(e: &fs_objects::Error) -> Refusal {
    match e {
        fs_objects::Error::Invalid(m) => refuse("rejected", 422, m.clone()),
        other => broken(other),
    }
}

fn by_of(cx: &Context) -> By {
    By::new(cx.person.id.canonical(), cx.person.clone())
}

fn id_of(text: &str) -> Result<ObjectId, Refusal> {
    profile_id(text).ok_or_else(|| bad("`id` is the id of a profile (urn:uuid:...)"))
}

fn made(profile: &Profile, text: &str) -> Applied {
    let mut done = Applied::default();
    done.record = format!("urn:uuid:{}", profile.id);
    text.clone_into(&mut done.text);
    done
}

/// `person.profile.create`: `name` (required) and optional `email`, `phone`, `describes` (only `me`).
pub(crate) async fn create<R: PersonRepo + ?Sized>(
    repo: &R,
    cx: &Context,
    c: &Invocation,
) -> Result<Applied, Refusal> {
    let mut got = text_args(&c.args, &CREATE_ARGS)?;
    set_level(c)?;
    let name = got.remove("name").unwrap_or_default();
    if name.trim().is_empty() {
        return Err(bad("`name` is missing or empty"));
    }
    let describes = match got.remove("describes").as_deref() {
        None | Some("") => None,
        Some(DESCRIBES_ME) => Some(
            repo.me()
                .await
                .map_err(|e| broken(&e))?
                .ok_or_else(|| refuse("unknown-record", 404, "\"me\" is not created yet"))?
                .urn(),
        ),
        Some(other) => {
            return Err(bad(format!(
                "`describes` takes only {DESCRIBES_ME:?}, not {other:?}"
            )));
        }
    };
    writable(repo)?;
    needs_me(repo).await?;
    let new = NewProfile {
        name,
        email: got.remove("email").filter(|t| !t.is_empty()),
        phone: got.remove("phone").filter(|t| !t.is_empty()),
        describes,
    };
    let profile = repo
        .create_profile(&by_of(cx), new)
        .await
        .map_err(|e| store_failed(&e))?;
    Ok(made(&profile, "profile-created"))
}

/// The one profile that describes "me": none or several is refused, the caller names the `id`.
async fn mine<R: PersonRepo + ?Sized>(repo: &R) -> Result<ObjectId, Refusal> {
    let me = repo
        .me()
        .await
        .map_err(|e| broken(&e))?
        .ok_or_else(|| refuse("unknown-record", 404, "\"me\" is not created yet"))?;
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
    match found.as_slice() {
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

/// `person.update`: `id` (none = the profile of "me") and at least one of `name`, `email`, `phone`; empty `email` or `phone` removes it.
pub(crate) async fn update<R: PersonRepo + ?Sized>(
    repo: &R,
    cx: &Context,
    c: &Invocation,
) -> Result<Applied, Refusal> {
    let mut got = text_args(&c.args, &UPDATE_ARGS)?;
    set_level(c)?;
    let patch = ProfilePatch {
        name: got.remove("name"),
        email: got.remove("email"),
        phone: got.remove("phone"),
    };
    if patch.is_empty() {
        return Err(bad("the change names no field (name, email, phone)"));
    }
    if patch.name.as_deref().is_some_and(|n| n.trim().is_empty()) {
        return Err(bad("`name` is empty: a profile needs a name"));
    }
    let id = match got.remove("id").as_deref() {
        None | Some("") => None,
        Some(text) => Some(id_of(text)?),
    };
    writable(repo)?;
    let id = match id {
        Some(id) => id,
        None => mine(repo).await?,
    };
    let profile = repo
        .update_profile(&by_of(cx), id, patch)
        .await
        .map_err(|e| store_failed(&e))?
        .ok_or_else(|| refuse("unknown-record", 404, format!("no profile urn:uuid:{id}")))?;
    Ok(made(&profile, "profile-updated"))
}

/// `person.delete`: `id` (required). "Me" is never deleted.
pub(crate) async fn delete<R: PersonRepo + ?Sized>(
    repo: &R,
    cx: &Context,
    c: &Invocation,
) -> Result<Applied, Refusal> {
    let mut got = text_args(&c.args, &DELETE_ARGS)?;
    set_level(c)?;
    let id = match got.remove("id").as_deref() {
        None | Some("") => return Err(bad("`id` is missing")),
        Some(text) => id_of(text)?,
    };
    if let Some(me) = repo.me().await.map_err(|e| broken(&e))?
        && me.id == id
    {
        return Err(refuse(
            "rejected",
            422,
            "the one person of this installation (me) is never deleted",
        ));
    }
    writable(repo)?;
    let gone = repo
        .delete_profile(&by_of(cx), id)
        .await
        .map_err(|e| store_failed(&e))?;
    if !gone {
        return Err(refuse(
            "unknown-record",
            404,
            format!("no profile urn:uuid:{id}"),
        ));
    }
    let mut done = Applied::default();
    "profile-deleted".clone_into(&mut done.text);
    Ok(done)
}
