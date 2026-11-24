//! The one question of the client: which records does the form `me` show?
//!
//! Pattern: Adapter – the page request of `FormSource` (level 1) as a list of record ids; no field of a record is trusted
//! but its id, which the caller then checks.

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;

use crate::FORM_ME;

/// The ids of the records of the form `me` (at most two: more than one is already wrong).
///
/// # Errors
/// The words of the failed call.
pub(super) async fn me_ids(source: &form_source::Client) -> Result<Vec<String>, String> {
    let mut request = source.page_request();
    {
        let mut p = request.get();
        p.set_form(FORM_ME);
        p.set_limit(2);
        let _: cursor::Builder<'_> = p.init_cursor();
    }
    let answer = request.send().promise.await.map_err(|e| e.to_string())?;
    let page = answer
        .get()
        .and_then(form_source::page_results::Reader::get_page)
        .map_err(|e| e.to_string())?;
    let records = page.get_records().map_err(|e| e.to_string())?;
    records
        .iter()
        .map(|r| {
            r.get_id()
                .and_then(|id| Ok(id.to_str()?.to_owned()))
                .map_err(|e| e.to_string())
        })
        .collect()
}
