//! The sample service is never the real "me": it has a socket name of its own, and the client refuses a service whose `hello` says `fake`.
//!
//! Pattern: Scenario test (given the sample service and a store service, when the client asks for "me", then only the store answers).
#![cfg(unix)]

mod common;

use std::sync::Arc;

use common::{seeded, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_person::{SampleRepo, sample_anchor};
use fs_person_service::{PROGRAM, PersonService, SAMPLE_PROGRAM, me_ids, socket_program};
use tokio::task::LocalSet;

/// `--sample` binds `person-sample.sock`, never `person.sock` (the one a host and planning look at for the real "me").
#[test]
fn the_sample_has_a_socket_of_its_own() {
    assert_eq!(socket_program(false), PROGRAM);
    assert_eq!(socket_program(true), SAMPLE_PROGRAM);
    assert_ne!(PROGRAM, SAMPLE_PROGRAM);
    let (real, sample) = (
        fs_rpc::default_socket(PROGRAM),
        fs_rpc::default_socket(SAMPLE_PROGRAM),
    );
    assert_ne!(real, sample, "two sockets");
}

/// The client reads "me" from the store service and refuses the sample service (it names `fake`), with the reason.
#[tokio::test(flavor = "current_thread")]
async fn the_client_refuses_the_sample_as_the_real_me() {
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    let (a, b) = (dir.path().join("sample"), dir.path().join("real"));
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    LocalSet::new()
        .run_until(async {
            let sample = PersonService::open(Arc::new(SampleRepo::new()), |_| {})
                .await
                .unwrap();
            let (socket, _s) = serve_service(&a, sample).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            let refused = me_ids(&c.client).await.unwrap_err();
            assert!(refused.contains("sample data"), "{refused}");
            let real = PersonService::open(seeded().await, |_| {}).await.unwrap();
            let (socket, _r) = serve_service(&b, real).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            assert_eq!(
                me_ids(&c.client).await.unwrap(),
                [sample_anchor().urn()],
                "the real one is read"
            );
        })
        .await;
}
