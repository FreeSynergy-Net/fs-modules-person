//! M3: the tiles `tiles@1.1.0` of the person module over a real socket: the same profiles as the list, each fact ONE value.
//!
//! Pattern: Scenario test (given the sample profiles, when a client reads the tiles, then the facts are the preferred values).
#![cfg(unix)]

mod common;

use common::{Repo, all, page_with, seeded, serve_service};
use fs_interfaces::formsource_capnp::form_source;
use fs_person::{preferred, sample_profiles};
use fs_person_service::{FORM_ROWS, FORM_TILES, PersonService};
use tokio::task::LocalSet;

async fn owner_reads<F, Fut>(repo: Repo, body: F)
where
    F: FnOnce(form_source::Client) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let dir = tempfile::Builder::new().prefix("fs").tempdir().unwrap();
    LocalSet::new()
        .run_until(async {
            let service = PersonService::open(repo, |_| {}).await.unwrap();
            let (socket, _stop) = serve_service(dir.path(), service).await;
            let c = fs_rpc::connect::<form_source::Client>(&socket)
                .await
                .unwrap();
            body(c.client.clone()).await;
        })
        .await;
}

/// The tiles: each fact is ONE value, the preferred one (`preferred()`); a profile without a value has none for that fact (no empty value).
#[tokio::test(flavor = "current_thread")]
async fn the_tiles_carry_one_preferred_value_per_fact() {
    owner_reads(seeded().await, |source| async move {
        let tiles = page_with(&source, FORM_TILES, "all", "", 0).await.unwrap();
        let rows = page_with(&source, FORM_ROWS, "all", "", 0).await.unwrap();
        assert_eq!(tiles.total, Some(7));
        assert_eq!(tiles.rows.len(), rows.rows.len());
        assert!(tiles.may.is_empty());
        let profiles = sample_profiles();
        let mut without_organization = 0;
        for (tile, row) in tiles.rows.iter().zip(&rows.rows) {
            assert_eq!(tile.0, row.0, "the same profile in the same place");
            let profile = profiles
                .iter()
                .find(|p| format!("urn:uuid:{}", p.id) == tile.0)
                .unwrap_or_else(|| panic!("{} is no sample profile", tile.0));
            for fact in ["organizations", "titles", "emails", "phones"] {
                let expected: Vec<String> = preferred(&profile.card, fact).into_iter().collect();
                assert_eq!(
                    all(tile, fact),
                    expected,
                    "{} / {fact}: exactly the preferred value",
                    tile.0
                );
                assert_eq!(
                    all(tile, fact),
                    all(row, fact),
                    "{fact}: the tile says what the row says"
                );
            }
            if all(tile, "organizations").is_empty() {
                without_organization += 1;
            }
        }
        assert!(
            without_organization > 0,
            "a profile without organizations has no value for it"
        );
        assert_eq!(all(&tiles.rows[4], "organizations"), ["Example Inc."]);
    })
    .await;
}
