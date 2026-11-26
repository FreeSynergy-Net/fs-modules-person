//! The one question of the client: which records does the form `me` show?
//!
//! Pattern: Adapter – the page request of `FormSource` (level 1) as a list of record ids; no field of a record is trusted
//! but its id, which the caller then checks. Before it, `hello`: a service that says `fake` serves sample data, and its
//! "me" is never taken as the real person (the answer is a refusal, not a guess).

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::SAMPLE_PROVIDER;

use crate::FORM_ME;

/// The ids of the records of the form `me` (at most two: more than one is already wrong).
///
/// # Errors
/// The words of the failed call, or that the service serves sample data (`hello` says `fake`): no real person.
pub async fn me_ids(source: &form_source::Client) -> Result<Vec<String>, String> {
    let hello = source
        .hello_request()
        .send()
        .promise
        .await
        .map_err(|e| e.to_string())?;
    let provider = hello
        .get()
        .and_then(|h| h.get_provider())
        .and_then(|p| Ok(p.to_str()?.to_owned()))
        .map_err(|e| e.to_string())?;
    if provider == SAMPLE_PROVIDER {
        return Err(format!(
            "the service serves sample data (hello says {SAMPLE_PROVIDER}): its \"me\" is no real person"
        ));
    }
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
