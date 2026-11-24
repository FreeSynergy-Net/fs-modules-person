//! `FormSource` level 1 of the person module: the form `me` (page) and the command `me.create` (invoke).
//!
//! Pattern: Adapter – the port [`MeRepo`] of `fs-person` as records by the field names of `catalog/forms.json`. The wire name of
//! a command is its capability key of `catalog/commands.json`; `invoke` knows only that one, takes no argument at all (nobody names an actor;
//! the log names the anchor as the actor of its own birth, the connection only admits the caller) and says what it did in `text`: `me-created` or `me-exists`. The levels above 1 (compare-and-set,
//! undo, changes, watch) are not spoken: the generated defaults answer them as unimplemented.

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::forms::parse_key;
use fs_interfaces::formsource_capnp::{form_source, record_view, value};
use fs_person::{Anchor, MeRepo};
use fs_rpc::Problem;

/// Who the service says it is in `hello`.
pub const PROVIDER: &str = "person";
/// The level of `FormSource` this service speaks: 1 = page and invoke.
pub const LEVEL: u32 = 1;
/// The form this module draws, as `name@x.y.z`; `catalog/forms.json` declares it as `1.0`.
pub const FORM_ME: &str = "me@1.0.0";
/// The one command, the capability key of `catalog/commands.json`.
pub const COMMAND_CREATE: &str = "me.create";

const UNKNOWN_FORM: &str = "form-source-unknown-form";
const UNKNOWN_COMMAND: &str = "form-source-unknown-command";
const BAD_ARGS: &str = "form-source-bad-arguments";
const INTERNAL: &str = "form-source-internal";

pub(crate) struct FormSourceImpl<R> {
    pub(crate) repo: Arc<R>,
}

type Reply = Result<(), capnp::Error>;

/// A `Problem` is large: it travels boxed.
type Failure = Box<Problem>;

#[allow(clippy::unnecessary_box_returns)] // a `Problem` is large, every reader of it takes it boxed
fn problem(kind: &str, title: &str, detail: impl Into<String>, status: u16) -> Failure {
    Box::new(Problem::new(kind, title, detail, status))
}

/// A store that fails or holds something that is no anchor: told with its words, never guessed.
#[allow(clippy::unnecessary_box_returns)]
fn broken(e: &impl std::fmt::Display) -> Failure {
    problem(
        INTERNAL,
        "Internal",
        format!("the store of the module failed: {e}"),
        500,
    )
}

/// Whether `form` is the key of the form this module draws: the name `me`, major 1, minor 0 (`x.y` and `x.y.z`).
fn serves(form: &str) -> bool {
    parse_key(form).is_some_and(|(name, v)| name == "me" && (v.major, v.minor) == (1, 0))
}

fn view(mut out: record_view::Builder<'_>, anchor: &Anchor) {
    out.set_id(anchor.urn().as_str());
    let mut values = out.init_values(2);
    for (i, (path, text)) in (0u32..).zip([("id", anchor.urn()), ("type", "Person".to_owned())]) {
        let mut v = values.reborrow().get(i);
        v.set_path(path);
        v.set_text(text.as_str());
    }
}

/// `me.create` takes nothing: any value (an `actor`, an `owner`, whatever) is refused with its reason, before anything is written.
fn no_args(args: capnp::struct_list::Reader<'_, value::Owned>) -> Result<(), Failure> {
    if let Some(a) = args.iter().next() {
        let path = a
            .get_path()
            .ok()
            .and_then(|p| p.to_str().ok())
            .unwrap_or("?");
        let why = format!(
            "unknown argument {path:?}: {COMMAND_CREATE} takes none, nobody names an actor"
        );
        return Err(problem(BAD_ARGS, "Bad arguments", why, 400));
    }
    Ok(())
}

impl<R: MeRepo + 'static> form_source::Server for FormSourceImpl<R> {
    async fn hello(
        self: Rc<Self>,
        _: form_source::HelloParams,
        mut results: form_source::HelloResults,
    ) -> Reply {
        let mut out = results.get();
        out.set_level(LEVEL);
        out.set_provider(PROVIDER);
        Ok(())
    }

    async fn page(
        self: Rc<Self>,
        params: form_source::PageParams,
        mut results: form_source::PageResults,
    ) -> Reply {
        let form = params.get()?.get_form()?.to_str()?;
        if !serves(form) {
            let why = format!("this module draws {FORM_ME}");
            return Err(problem(UNKNOWN_FORM, "Unknown form", why, 404).into_error());
        }
        let me = self.repo.me().await.map_err(|e| broken(&e).into_error())?;
        let mut page = results.get().init_page();
        page.set_total(u64::from(me.is_some()));
        page.set_has_total(true);
        // the set may "create" while there is no "me"; the one record has nothing to run
        let mut may = page.reborrow().init_may(u32::from(me.is_none()));
        if me.is_none() {
            may.set(0, COMMAND_CREATE);
        }
        let mut records = page.init_records(u32::from(me.is_some()));
        if let Some(anchor) = &me {
            view(records.reborrow().get(0), anchor);
        }
        Ok(())
    }

    async fn blob(
        self: Rc<Self>,
        _: form_source::BlobParams,
        _: form_source::BlobResults,
    ) -> Reply {
        Err(problem(
            "form-source-unsupported",
            "Unsupported",
            "this module has no image or file field",
            501,
        )
        .into_error())
    }

    async fn invoke(
        self: Rc<Self>,
        params: form_source::InvokeParams,
        mut results: form_source::InvokeResults,
    ) -> Reply {
        let p = params.get()?;
        let done = match (p.get_command()?.to_str()?, p.get_record()?.to_str()?) {
            (COMMAND_CREATE, "") => match no_args(p.get_args()?) {
                Ok(()) => self.repo.create_me().await.map_err(|e| broken(&e)),
                Err(e) => Err(e),
            },
            (COMMAND_CREATE, _) => {
                let why = format!("{COMMAND_CREATE} is about the set, not about a record");
                Err(problem(BAD_ARGS, "Bad arguments", why, 400))
            }
            (other, _) => Err(problem(
                UNKNOWN_COMMAND,
                "Unknown command",
                format!("no command {other:?}"),
                404,
            )),
        };
        let mut out = results.get();
        match done {
            Ok(made) => {
                let mut r = out.init_result();
                r.set_record(made.anchor.urn().as_str());
                r.set_text(if made.new { "me-created" } else { "me-exists" });
            }
            Err(why) => {
                out.set_failed(true);
                why.write(out.init_problem());
            }
        }
        Ok(())
    }
}
