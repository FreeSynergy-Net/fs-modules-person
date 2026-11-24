//! P2 to P5 and P9 of the Geburtsstein (ENTSCHEID N2) at the service: `me.create` and the form `me` through a real socket in a
//! temporary folder, every value a text.
#![cfg(unix)]

mod common;

use common::{field, invoke, page, serve};
use fs_interfaces::formsource_capnp::form_source;
use fs_objects::{ActivityFilter, By, Error, Query, Store};
use fs_person::{Anchor, MeRepo};
use fs_person_service::{COMMAND_CREATE, FORM_ME, PROVIDER, reject_actor};
use fs_types::PersonArt;
use tokio::task::LocalSet;

macro_rules! connected {
    ($source:ident, $repo:ident, $body:block) => {{
        let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
        LocalSet::new()
            .run_until(async {
                let (socket, $repo, _stop) = serve(dir.path()).await;
                let connection = fs_rpc::connect::<form_source::Client>(&socket)
                    .await
                    .unwrap();
                let $source = &connection.client;
                $body
            })
            .await;
    }};
}

/// P2: `me.create` answers `urn:uuid:` + UUIDv7 as the record; the form then shows one person, of kind `Person`.
#[tokio::test(flavor = "current_thread")]
async fn p2_me_create_makes_a_v7_person() {
    connected!(source, repo, {
        let hello = source.hello_request().send().promise.await.unwrap();
        assert_eq!(
            hello
                .get()
                .unwrap()
                .get_provider()
                .unwrap()
                .to_str()
                .unwrap(),
            PROVIDER
        );
        let made = invoke(source, COMMAND_CREATE, "", &[]).await;
        assert!(!made.failed, "{made:?}");
        assert_eq!(made.text, "me-created");
        let uuid = made.record.strip_prefix("urn:uuid:").expect("urn:uuid");
        assert_eq!(uuid::Uuid::parse_str(uuid).unwrap().get_version_num(), 7);
        let me = repo.me().await.unwrap().unwrap();
        assert_eq!(me.urn(), made.record);
        assert_eq!(me.person_ref().art, PersonArt::Person);
        let shown = page(source, FORM_ME).await.unwrap();
        assert_eq!(shown.rows.len(), 1);
        assert_eq!(shown.rows[0].0, made.record);
        assert_eq!(field(&shown.rows[0], "type"), "Person");
        assert!(shown.may.is_empty(), "me exists: nothing left to create");
    });
}

/// P3: twice gives the same anchor (`me-exists`), and the log holds exactly one activity.
#[tokio::test(flavor = "current_thread")]
async fn p3_me_create_twice_is_one_anchor_and_one_activity() {
    connected!(source, repo, {
        let first = invoke(source, COMMAND_CREATE, "", &[]).await;
        let again = invoke(source, COMMAND_CREATE, "", &[]).await;
        assert!(!first.failed && !again.failed);
        assert_eq!(
            (first.text.as_str(), again.text.as_str()),
            ("me-created", "me-exists")
        );
        assert_eq!(first.record, again.record);
        let log = repo.0.activities(&ActivityFilter::default()).await.unwrap();
        assert_eq!(log.len(), 1, "{log:?}");
    });
}

/// P4: the row of the anchor is owned by the anchor itself, and the form says "create" only while there is no "me".
#[tokio::test(flavor = "current_thread")]
async fn p4_the_anchor_owns_itself_and_the_form_offers_create_once() {
    connected!(source, repo, {
        let empty = page(source, FORM_ME).await.unwrap();
        assert_eq!(empty.rows.len(), 0);
        assert_eq!(empty.may, [COMMAND_CREATE]);
        let made = invoke(source, COMMAND_CREATE, "", &[]).await;
        let me = repo.me().await.unwrap().unwrap();
        let row = repo.0.get(me.id).await.unwrap().unwrap();
        assert_eq!(row.owner, Some(me.person_ref()));
        assert_eq!(
            row.envelope.id.to_string(),
            made.record["urn:uuid:".len()..]
        );
    });
}

/// P5 (counter-proof): a second self-owned person with another uuid, written straight into the store behind the service, is
/// refused, nothing is written, and the form still shows the first.
#[tokio::test(flavor = "current_thread")]
async fn p5_a_second_birth_is_refused_and_the_form_keeps_the_first() {
    connected!(source, repo, {
        let first = invoke(source, COMMAND_CREATE, "", &[]).await;
        let other = Anchor::new();
        let by = By::new(other.urn(), other.person_ref());
        match repo.0.birth(&by, other.to_object()).await {
            Err(Error::Invalid(m)) => assert!(m.contains("store already owned"), "{m}"),
            other => panic!("expected Invalid, got {other:?}"),
        }
        assert_eq!(repo.0.count(&Query::default()).await.unwrap(), 1);
        let shown = page(source, FORM_ME).await.unwrap();
        assert_eq!(shown.rows[0].0, first.record);
    });
}

/// P9: the actor is the connection (the peer), never an argument: an `actor` value, an `owner` value and a record id are refused
/// with their reason and nothing is written; `--actor` of the program words is refused too.
#[tokio::test(flavor = "current_thread")]
async fn p9_a_given_actor_is_not_heeded() {
    connected!(source, repo, {
        for pairs in [
            &[("actor", "urn:evil")][..],
            &[("owner", "urn:evil")],
            &[("actor", "urn:evil"), ("actor", "urn:evil")],
        ] {
            let answer = invoke(source, COMMAND_CREATE, "", pairs).await;
            assert!(answer.failed, "{pairs:?}");
            assert_eq!(answer.kind, "form-source-bad-arguments");
        }
        let with_record = invoke(source, COMMAND_CREATE, "urn:uuid:x", &[]).await;
        assert_eq!(with_record.kind, "form-source-bad-arguments");
        assert_eq!(repo.me().await.unwrap(), None, "nothing was written");
    });
    assert!(reject_actor(["serve".as_ref(), "--actor".as_ref(), "urn:evil".as_ref()]).is_err());
    assert!(reject_actor(["--actor=urn:evil".as_ref()]).is_err());
    assert!(reject_actor(["serve".as_ref(), "--data".as_ref(), "d".as_ref()]).is_ok());
}

/// Error cases: an unknown command, an unknown form, and a broken store are told with their kind, never guessed.
#[tokio::test(flavor = "current_thread")]
async fn unknown_things_and_a_broken_store_are_told() {
    connected!(source, repo, {
        let unknown = invoke(source, "me.delete", "", &[]).await;
        assert_eq!(unknown.kind, "form-source-unknown-command");
        let prefixed = invoke(source, "person.me.create", "", &[]).await;
        assert_eq!(prefixed.kind, "form-source-unknown-command");
        for key in ["nope@1.0.0", "me@2.0.0", "me", "me@x"] {
            let e = page(source, key)
                .await
                .err()
                .unwrap_or_else(|| panic!("{key}"));
            assert!(
                e.to_string().contains("form-source-unknown-form"),
                "{key}: {e}"
            );
        }
        // two ways of the key are read: x.y.z and x.y
        assert!(page(source, "me@1.0").await.is_ok());
        // a person in the store that is no anchor: the form and the command say so (internal), they do not make a second one
        let a = Anchor::new();
        let mut bad = a.to_object();
        bad.content = serde_json::json!({"type": "Person", "id": "urn:uuid:other"});
        repo.0
            .birth(&By::new(a.urn(), a.person_ref()), bad)
            .await
            .unwrap();
        let e = page(source, FORM_ME).await.err().unwrap();
        assert!(e.to_string().contains("form-source-internal"), "{e}");
        let broken = invoke(source, COMMAND_CREATE, "", &[]).await;
        assert!(broken.failed);
        assert_eq!(broken.kind, "form-source-internal");
    });
}

/// The catalog files and the service say the same: the command and the form name, and every field the page writes.
#[tokio::test(flavor = "current_thread")]
async fn the_catalog_matches_what_the_service_serves() {
    let commands: serde_json::Value =
        serde_json::from_str(include_str!("../../../catalog/commands.json")).unwrap();
    assert_eq!(commands["module"], "person");
    assert_eq!(commands["commands"][0]["capability"], COMMAND_CREATE);
    assert_eq!(commands["commands"][0]["access"], "write");
    let forms: serde_json::Value =
        serde_json::from_str(include_str!("../../../catalog/forms.json")).unwrap();
    assert_eq!(
        format!("{}@1.0.0", forms[0]["name"].as_str().unwrap()),
        FORM_ME
    );
    connected!(source, _repo, {
        invoke(source, COMMAND_CREATE, "", &[]).await;
        let shown = page(source, FORM_ME).await.unwrap();
        for (name, _) in &shown.rows[0].2 {
            assert!(forms[0]["fields"].get(name).is_some(), "{name}");
        }
        for name in forms[0]["fields"].as_object().unwrap().keys() {
            assert!(shown.rows[0].2.iter().any(|(p, _)| p == name), "{name}");
        }
    });
}
