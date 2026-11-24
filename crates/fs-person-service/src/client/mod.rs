//! `PersonServiceOwner`: the local owner, asked from the person service by another module.
//!
//! Pattern: Strategy (the variant of `LocalOwner` for other modules) over a Proxy of the service – it gets or starts the one
//! service of this person (`ensure_single`, ADR 0104 E9), reads the form `me` and gives the person back. It never creates "me":
//! no "me" is `NoMe`, so the person is told to make it where it belongs, in the module person.
//!
//! Deviation from `Worker::call` (fs-rpc): the owner is read once per process, so no connection is kept; every call runs on a
//! thread and runtime of its own (works inside and outside a runtime), as `StoreOwner` does.

mod read;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::owner::{LocalOwner, OwnerProblem};
use fs_rpc::{CommandStarter, Expected, SingleTimes, ensure_single};
use fs_types::{BasicValue, PersonArt, PersonRef, Reference};

use crate::PROGRAM;

/// The name of the program of the service, as it tells in `Control.version`.
const SERVICE_PROGRAM: &str = "fs-person-service";

/// Where the service is and how it is started.
pub struct PersonServiceOwner {
    socket: PathBuf,
    program: PathBuf,
    data: Option<PathBuf>,
    times: SingleTimes,
}

impl PersonServiceOwner {
    /// The service on `socket`, started from the program file `program` if none runs.
    #[must_use]
    pub fn new(socket: impl Into<PathBuf>, program: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            program: program.into(),
            data: None,
            times: SingleTimes::default(),
        }
    }

    /// The usual place: `default_socket("person")` and the program next to `running`, then in `/usr/bin` (never on `PATH`: a
    /// planted file must not become the server). `None` if the system has no place for the socket.
    #[must_use]
    pub fn standard(running: &Path) -> Option<Self> {
        let socket = fs_rpc::default_socket(PROGRAM)?;
        let usr_bin = if cfg!(windows) { "" } else { "/usr/bin" };
        let name = if cfg!(windows) {
            "fs-person-service.exe"
        } else {
            SERVICE_PROGRAM
        };
        let program = [
            running.parent().map(|d| d.join(name)),
            Some(Path::new(usr_bin).join(name)),
        ]
        .into_iter()
        .flatten()
        .find(|p| std::fs::metadata(p).is_ok_and(|m| m.is_file()))
        .unwrap_or_else(|| PathBuf::from(name));
        Some(Self::new(socket, program))
    }

    /// Start the service with this data folder (`--data`), not the one of the user.
    #[must_use]
    pub fn with_data(mut self, data: impl Into<PathBuf>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Other limits for getting the service (tests make them short).
    #[must_use]
    pub fn with_times(mut self, times: SingleTimes) -> Self {
        self.times = times;
        self
    }

    async fn ask(&self) -> Result<Vec<String>, OwnerProblem> {
        let down = |why: String| OwnerProblem::Unreachable {
            reason: format!("{}: {why}", self.socket.display()),
        };
        let mut args: Vec<&OsStr> = vec![
            "serve".as_ref(),
            "--socket".as_ref(),
            self.socket.as_os_str(),
        ];
        if let Some(data) = &self.data {
            args.extend(["--data".as_ref(), data.as_os_str()]);
        }
        let starter = CommandStarter::new(&self.program, args);
        let expected = Expected::new(env!("CARGO_PKG_VERSION")).with_program(SERVICE_PROGRAM);
        let single = ensure_single::<form_source::Client>(
            &self.socket,
            &expected,
            &starter,
            &self.times,
            Some(&self.program),
        )
        .await
        .map_err(|e| down(fs_rpc::RpcError::from(&e).code().as_str().to_owned()))?;
        read::me_ids(&single.connection.client).await.map_err(down)
    }
}

/// The reference of the person, or why the answer is no person.
fn person(ids: &[String]) -> Result<PersonRef, OwnerProblem> {
    let bad = |why: &str| OwnerProblem::Invalid {
        reason: why.to_owned(),
    };
    let [id] = ids else {
        return if ids.is_empty() {
            Err(OwnerProblem::NoMe {
                hint: "\"me\" is not created yet: create it in the module person".to_owned(),
            })
        } else {
            Err(bad("the form me shows more than one person"))
        };
    };
    let uuid = id
        .strip_prefix("urn:uuid:")
        .and_then(|u| uuid::Uuid::parse_str(u).ok());
    if uuid.is_none_or(|u| u.get_version_num() != 7) {
        return Err(bad("the id of the person is not a urn:uuid of version 7"));
    }
    let id = Reference::parse(id).map_err(|_| bad("the id of the person is no reference"))?;
    Ok(PersonRef {
        id,
        art: PersonArt::Person,
        abzug: None,
    })
}

impl LocalOwner for PersonServiceOwner {
    fn owner(&self) -> Result<PersonRef, OwnerProblem> {
        let read = std::thread::scope(|s| {
            s.spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| OwnerProblem::Unreachable {
                        reason: format!("runtime: {e}"),
                    })?;
                tokio::task::LocalSet::new().block_on(&runtime, self.ask())
            })
            .join()
            .unwrap_or_else(|_| {
                Err(OwnerProblem::Unreachable {
                    reason: "the reading thread panicked".to_owned(),
                })
            })
        });
        person(&read?)
    }
}
