//! What the module offers on its pages (`may`): the set offers a new profile, every record offers change and delete, the sample data
//! offers nothing. The wrapper strikes what a person may not do; here the owner sees all. Each test names what a wrong implementation would get wrong.
//!
//! Pattern: Scenario test (given a store, when the owner reads the list, then the page says what may be run).
#![cfg(unix)]

mod common;
#[macro_use]
mod crud_common;

use common::page_with;
use crud_common::create;
use fs_person_service::{
    COMMAND_DELETE, COMMAND_PROFILE_CREATE, COMMAND_UPDATE, FORM_RECORD, FORM_ROWS,
};

/// An empty list (only "me" is there) still offers `person.profile.create`: the frontend can show "New" without any record; a
/// record offers `person.update` and `person.delete`.
#[tokio::test(flavor = "current_thread")]
async fn the_list_offers_new_even_when_empty_and_change_and_delete_per_record() {
    connected!(source, repo, {
        let empty = page_with(source, FORM_ROWS, "", "", 10).await.unwrap();
        assert_eq!(empty.rows.len(), 0, "an empty database has no rows");
        assert_eq!(empty.may, [COMMAND_PROFILE_CREATE]);
        let made = create(source, &[("name", "Ada")]).await;
        assert!(!made.failed, "{made:?}");
        let list = page_with(source, FORM_ROWS, "", "", 10).await.unwrap();
        assert_eq!(list.rows.len(), 1);
        assert_eq!(list.rows[0].1, [COMMAND_UPDATE, COMMAND_DELETE]);
        assert_eq!(list.may, [COMMAND_PROFILE_CREATE]);
        let one = page_with(source, FORM_RECORD, &made.record, "", 10)
            .await
            .unwrap();
        assert_eq!(one.rows[0].1, [COMMAND_UPDATE, COMMAND_DELETE]);
        let _ = repo;
    });
}
