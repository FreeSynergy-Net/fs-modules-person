//! The service ends when it is told to, also after a client was connected: SIGTERM and `Control.stop` (ADR 0104: a module service
//! must not outlive its stop). The test proves the END of the process (no SIGKILL).
#![cfg(unix)]

mod common;

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use fs_interfaces::formsource_capnp::form_source;
use tokio::task::LocalSet;

const BIN: &str = env!("CARGO_BIN_EXE_fs-person-service");

/// Starts the binary on a socket in a temporary folder, waits until it answers, connects once and lets go again.
fn started_and_used(dir: &std::path::Path) -> (Child, std::path::PathBuf) {
    let socket = dir.join("p.sock");
    let child = Command::new(BIN)
        .args(["serve", "--socket"])
        .arg(&socket)
        .arg("--data")
        .arg(dir.join("data"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    LocalSet::new().block_on(&runtime, async {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match fs_rpc::connect::<form_source::Client>(&socket).await {
                Ok(c) => {
                    c.client.hello_request().send().promise.await.unwrap();
                    break;
                }
                Err(e) => assert!(Instant::now() < until, "service did not start: {e}"),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    (child, socket)
}

fn ends_within(child: &mut Child, limit: Duration) -> bool {
    let until = Instant::now() + limit;
    while Instant::now() < until {
        if child.try_wait().unwrap().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn dir() -> tempfile::TempDir {
    tempfile::Builder::new().prefix("fs").tempdir().unwrap()
}

#[test]
fn sigterm_ends_the_service_after_a_client() {
    let d = dir();
    let (mut child, _) = started_and_used(d.path());
    Command::new("kill")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    let ended = ends_within(&mut child, Duration::from_secs(5));
    let _ = child.kill();
    assert!(ended, "the service did not end on SIGTERM");
}

#[test]
fn control_stop_ends_the_service_after_a_client() {
    let d = dir();
    let (mut child, socket) = started_and_used(d.path());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    LocalSet::new()
        .block_on(&runtime, fs_rpc::stop_server(&socket))
        .unwrap();
    let ended = ends_within(&mut child, Duration::from_secs(5));
    let _ = child.kill();
    assert!(ended, "the service did not end on Control.stop");
}
