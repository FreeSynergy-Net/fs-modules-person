//! Shared by the crud tests: a service with "me" born, the id of a record, the refusal check, the create call.
#![allow(dead_code, unused_macros)]

use fs_interfaces::formsource_capnp::form_source;
use fs_objects::ObjectId;
use fs_person_service::COMMAND_PROFILE_CREATE;

use crate::common::{Answer, invoke};

macro_rules! connected {
    ($source:ident, $repo:ident, $body:block) => {{
        let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
        tokio::task::LocalSet::new()
            .run_until(async {
                let (socket, $repo, _stop) = crate::common::serve(dir.path()).await;
                let connection = fs_rpc::connect::<
                    fs_interfaces::formsource_capnp::form_source::Client,
                >(&socket)
                .await
                .unwrap();
                let $source = &connection.client;
                let born =
                    crate::common::invoke($source, fs_person_service::COMMAND_CREATE, "", &[])
                        .await;
                assert!(!born.failed, "{born:?}");
                $body
            })
            .await;
    }};
}

pub fn id_of(record: &str) -> ObjectId {
    record
        .strip_prefix("urn:uuid:")
        .expect("urn:uuid")
        .parse()
        .unwrap()
}

pub fn refused(a: &Answer, kind: &str) {
    assert!(a.failed, "the call went through: {a:?}");
    assert_eq!(a.kind, kind, "{a:?}");
}

pub async fn create(source: &form_source::Client, pairs: &[(&str, &str)]) -> Answer {
    invoke(source, COMMAND_PROFILE_CREATE, "", pairs).await
}
