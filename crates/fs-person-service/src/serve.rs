//! `fs-person-service serve`: the options, the data folder, the store, the serving.
//!
//! Pattern: Template method over fs-rpc `run_until_signal_callers` – registers the signals, binds the socket (a second server on
//! the same socket does not start: it is served by somebody else, so one service per person), opens the store, serves until a
//! signal or `Control.stop`. The data lives in `$XDG_DATA_HOME/freesynergy/modules/person/` (0700). Returns the exit code.

use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::sync::Arc;

use fs_objects::TursoStore;
use fs_person::{PersonRepo, PersonStore, SampleRepo};
use fs_rpc::{
    ServerIdentity, ServerOptions, default_data_dir, prepare_data_dir, run_until_signal_callers,
};

use crate::service::PersonService;

/// The name of the module for the socket (`$XDG_RUNTIME_DIR/freesynergy/person.sock`, `default_socket("person")`), for
/// `Control.version` and for the data folder `modules/person`.
pub const PROGRAM: &str = "person";

const USAGE: &str = "fs-person-service [serve] [--socket PATH] [--data DIR] [--instance DOMAIN] [--sample]\nDefaults: $XDG_RUNTIME_DIR/freesynergy/person.sock (person-sample.sock with --sample), $XDG_DATA_HOME/freesynergy/modules/person\nServes the form source of the module person (forms me, rows, single-record; command me.create); nobody names an actor, the connection (same user) only decides who may call.\n--sample serves fixed test data instead of the store (for frontends and tests; the same rights, hello says fake, its own socket person-sample.sock).";

/// The name of the sample service for its socket (`$XDG_RUNTIME_DIR/freesynergy/person-sample.sock`): another file than the one of the
/// real module, so the sample "me" is never found where the real "me" is looked for.
pub const SAMPLE_PROGRAM: &str = "person-sample";

/// The socket name of the service: the real one, or the sample one.
#[must_use]
pub const fn socket_program(sample: bool) -> &'static str {
    if sample { SAMPLE_PROGRAM } else { PROGRAM }
}

/// There is no `--actor`: the words that name one are refused with the reason (fs-rpc would take it; this module does not).
///
/// # Errors
/// A word is `--actor` or `--actor=…`.
pub fn reject_actor<'a>(args: impl IntoIterator<Item = &'a OsStr>) -> Result<(), String> {
    let named = args.into_iter().any(|a| {
        let a = a.to_string_lossy();
        a == "--actor" || a.starts_with("--actor=")
    });
    if named {
        return Err("there is no --actor: nobody names an actor, neither on the command line nor in a message".into());
    }
    Ok(())
}

/// The words of `serve` with the data folder of this module where none was given; a leading `serve` is dropped.
fn words(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = args.into_iter().collect();
    if args.first().is_some_and(|a| a == "serve") {
        args.remove(0);
    }
    if !args.iter().any(|a| a == "--data")
        && let Some(dir) = default_data_dir(&format!("modules/{PROGRAM}"))
    {
        args.push("--data".into());
        args.push(dir.into_os_string());
    }
    args
}

/// Run the server with `args`; the exit code: 0 after a signal or `--help`, 2 for a mistake in the words, 1 when serving failed.
#[must_use]
pub fn serve_command(args: impl IntoIterator<Item = OsString>) -> i32 {
    serve_with(args, &mut io::stderr())
}

/// The words without `--sample`, and whether it was there (the one word of this module that fs-rpc does not know).
fn take_sample(args: Vec<OsString>) -> (bool, Vec<OsString>) {
    let sample = args.iter().any(|a| a == "--sample");
    let rest = args.into_iter().filter(|a| a != "--sample").collect();
    (sample, rest)
}

/// [`serve_command`] with the words for the person written to `err` (a test reads them).
#[must_use]
pub fn serve_with(args: impl IntoIterator<Item = OsString>, err: &mut dyn Write) -> i32 {
    let (sample, args) = take_sample(words(args));
    let options = reject_actor(args.iter().map(OsString::as_os_str))
        .and_then(|()| ServerOptions::parse(socket_program(sample), args));
    let options = match options {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return 0;
        }
        Err(problem) => {
            let _ = writeln!(err, "fs-person-service: {problem}");
            return 2;
        }
    };
    let (socket, data) = (options.socket, options.data);
    let beat_socket = socket.clone();
    let served = if sample {
        // fixed test data: no folder, no store, the same rights (`hello` says `fake`)
        run(&socket, &beat_socket, || async {
            Ok(Arc::new(SampleRepo::new()))
        })
    } else {
        run(&socket, &beat_socket, || async move {
            let data = prepare_data_dir(&data)?;
            let store = TursoStore::open(data.join("objects.db"))
                .await
                .map_err(io::Error::other)?;
            Ok(Arc::new(PersonStore(store)))
        })
    };
    match served {
        Ok(()) => 0,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => {
            let _ = writeln!(err, "fs-person-service: {e}");
            0
        }
        Err(e) => {
            let _ = writeln!(err, "fs-person-service: {e}");
            1
        }
    }
}

/// Serve the repository `open` makes: "me" is read here, once, before the loop serves, and born if the folder has none ([`PersonService::open_anchored`]); a failing birth does not start.
fn run<R, F, Fut>(
    socket: &std::path::Path,
    beat_socket: &std::path::Path,
    open: F,
) -> io::Result<()>
where
    R: PersonRepo + 'static,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = io::Result<Arc<R>>>,
{
    let identity = ServerIdentity::new("fs-person-service", env!("CARGO_PKG_VERSION"));
    run_until_signal_callers(socket, identity, || async move {
        let repo = open().await?;
        let service = PersonService::open_anchored(repo, |why| {
            eprintln!(
                "fs-person-service: connection dropped, the system did not say who it is: {why:?}"
            );
        })
        .await
        .map_err(io::Error::other)?;
        keep_beat(service, beat_socket)
    })
}

/// The heartbeat of the service (ADR 0104, a duty of every module service): started once the socket is bound, ticked every second
/// by the loop that serves, so a loop that stands still is told apart from a healthy one. fs-rpc has it on Unix only.
#[cfg(unix)]
fn keep_beat<R: PersonRepo + 'static>(
    service: PersonService<R>,
    socket: &std::path::Path,
) -> io::Result<PersonService<R>> {
    let beat = std::rc::Rc::new(fs_rpc::Heartbeat::start(socket)?);
    let tick = beat.clone();
    let ticker = tokio::task::spawn_local(async move {
        let mut every = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            every.tick().await;
            tick.tick();
        }
    });
    Ok(service.keeping(Beating { beat, ticker }))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // the same signature as on Unix
fn keep_beat<R: PersonRepo + 'static>(
    service: PersonService<R>,
    _: &std::path::Path,
) -> io::Result<PersonService<R>> {
    Ok(service)
}

#[cfg(unix)]
struct Beating {
    beat: std::rc::Rc<fs_rpc::Heartbeat>,
    ticker: tokio::task::JoinHandle<()>,
}

#[cfg(unix)]
impl Drop for Beating {
    fn drop(&mut self) {
        self.ticker.abort();
        let _ = &self.beat;
    }
}
