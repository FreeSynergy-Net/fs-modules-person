//! P1 (client) and P8 of the Geburtsstein (ENTSCHEID N2): `PersonServiceOwner` against the REAL service binary (the owner starts
//! it through `ensure_single`), in a temporary folder; and without a service it can start.
#![cfg(unix)]

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::invoke;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::owner::fake::check;
use fs_interfaces::owner::{LocalOwner, OwnerProblem};
use fs_person_service::{COMMAND_CREATE, PersonServiceOwner};
use fs_rpc::{RestartTimes, SingleTimes};
use fs_types::BasicValue as _;

const BIN: &str = env!("CARGO_BIN_EXE_fs-person-service");

/// Stops the service the owner started, also when a test fails: asked to stop, then (a service that had a client does not end
/// on its own, see BERICHT-g4) ended by its pid, found by its socket in the command line.
struct Stop(PathBuf);

impl Drop for Stop {
    fn drop(&mut self) {
        let socket = self.0.clone();
        let _ =
            std::thread::spawn(move || local(async move { fs_rpc::stop_server(&socket).await }))
                .join();
        let want = self.0.to_string_lossy().into_owned();
        for entry in std::fs::read_dir("/proc").unwrap().flatten() {
            let line = std::fs::read(entry.path().join("cmdline")).unwrap_or_default();
            if String::from_utf8_lossy(&line).contains(&want) {
                let _ = std::process::Command::new("kill")
                    .arg("-KILL")
                    .arg(entry.file_name())
                    .status();
            }
        }
    }
}

fn local<T>(work: impl std::future::Future<Output = T>) -> T {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tokio::task::LocalSet::new().block_on(&runtime, work)
}

fn short() -> SingleTimes {
    let ms = Duration::from_millis;
    SingleTimes {
        handshake: RestartTimes {
            call: ms(300),
            stop: ms(500),
            start: ms(800),
        },
        overall: Duration::from_secs(4),
        ..SingleTimes::default()
    }
}

fn owner_in(dir: &Path) -> (PersonServiceOwner, Stop) {
    let socket = dir.join("p.sock");
    let owner = PersonServiceOwner::new(&socket, BIN).with_data(dir.join("data"));
    (owner, Stop(socket))
}

/// What the person does in the module (not the client): `me.create` at the running service.
fn create_me(socket: &Path) -> String {
    local(async {
        let c = fs_rpc::connect::<form_source::Client>(socket)
            .await
            .unwrap();
        let made = invoke(&c.client, COMMAND_CREATE, "", &[]).await;
        assert!(!made.failed, "{made:?}");
        made.record
    })
}

fn tmp() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("fs").tempdir().unwrap()
}

/// P1 (client): the contract of `LocalOwner` (the three rules) holds against the real service; two questions, one id; and the
/// id is the one of the module, which the client did not make.
#[test]
fn p1_client_owner_passes_the_contract() {
    let dir = tmp();
    let (owner, stop) = owner_in(dir.path());
    // The first question starts the service; there is no "me" yet and the client does not make one.
    let first = owner.owner();
    assert!(matches!(first, Err(OwnerProblem::NoMe { .. })), "{first:?}");
    assert!(check(&owner).is_ok(), "NoMe has a reason");
    let urn = create_me(&stop.0);
    let me = owner.owner().expect("me exists now");
    assert_eq!(me.id.canonical(), urn);
    assert_eq!(owner.owner().unwrap(), me, "two questions, one id");
    check(&owner).unwrap();
}

/// The client never creates: asking many times without "me" leaves the form empty.
#[test]
fn p1_client_never_creates() {
    let dir = tmp();
    let (owner, stop) = owner_in(dir.path());
    for _ in 0..3 {
        assert!(matches!(owner.owner(), Err(OwnerProblem::NoMe { .. })));
    }
    let urn = create_me(&stop.0);
    assert_eq!(
        owner.owner().unwrap().id.canonical(),
        urn,
        "the first me.create of the person is the first anchor"
    );
}

/// P8: no program file to start → `Unreachable`, with a reason.
#[test]
fn p8_unstartable_service_is_unreachable() {
    let dir = tmp();
    let owner = PersonServiceOwner::new(dir.path().join("p.sock"), dir.path().join("none"))
        .with_times(short());
    let got = owner.owner();
    let Err(OwnerProblem::Unreachable { reason }) = &got else {
        panic!("{got:?}");
    };
    assert_ne!(reason.trim(), "");
    assert!(check(&owner).is_ok());
}

/// P8: a program that starts but never serves → `Unreachable`, not a hang and not an `Ok`.
#[test]
fn p8_program_that_does_not_serve_is_unreachable() {
    let dir = tmp();
    let owner = PersonServiceOwner::new(dir.path().join("p.sock"), "/bin/true").with_times(short());
    let got = owner.owner();
    let Err(OwnerProblem::Unreachable { reason }) = &got else {
        panic!("{got:?}");
    };
    assert!(reason.contains("p.sock"), "{reason}");
}
