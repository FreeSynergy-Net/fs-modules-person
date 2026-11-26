//! Shared by the tests: a real service in a temporary folder, a page of records, one `invoke`.
#![allow(dead_code)]

use std::sync::Arc;

use fs_interfaces::common_capnp::cursor;
use fs_interfaces::formsource_capnp::form_source;
use fs_objects::TursoStore;
use fs_person::StoreMe;
use fs_person_service::PersonService;
use tokio::sync::oneshot;

pub type Repo = Arc<StoreMe<TursoStore>>;
/// A service over an in-memory store; the repo is returned so a test can look into the store (the log, the rows).
pub async fn serve(dir: &std::path::Path) -> (std::path::PathBuf, Repo, oneshot::Sender<()>) {
    let repo = Arc::new(StoreMe(TursoStore::open_in_memory().await.unwrap()));
    let service = PersonService::open(repo.clone(), |_| {}).await.unwrap();
    let (socket, stop) = serve_service(dir, service).await;
    (socket, repo, stop)
}

/// The same over the service a test built (other ports, other rules): bound on `person.sock` in `dir`.
pub async fn serve_service(
    dir: &std::path::Path,
    service: PersonService<StoreMe<TursoStore>>,
) -> (std::path::PathBuf, oneshot::Sender<()>) {
    let socket = dir.join("person.sock");
    let bound = fs_rpc::bind_socket(&socket).await.unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::task::spawn_local(async move {
        let _ = fs_rpc::serve_callers(bound, service, async {
            let _ = stopped.await;
        })
        .await;
    });
    (socket, stop)
}

/// The records of one page: id, the commands the record may run, the values by field name; and the commands of the set.
/// One record: id, the commands it may run, the values by field name.
pub type Row = (String, Vec<String>, Vec<(String, String)>);

pub struct Page {
    pub rows: Vec<Row>,
    pub may: Vec<String>,
}

pub async fn page(source: &form_source::Client, form: &str) -> Result<Page, capnp::Error> {
    let mut request = source.page_request();
    {
        let mut p = request.get();
        p.set_form(form);
        p.set_limit(100);
        let _: cursor::Builder<'_> = p.init_cursor();
    }
    let answer = request.send().promise.await?;
    let page = answer.get()?.get_page()?;
    let texts = |l: capnp::text_list::Reader<'_>| {
        l.iter()
            .map(|t| t?.to_string().map_err(capnp::Error::from))
            .collect::<capnp::Result<Vec<_>>>()
    };
    let mut rows = Vec::new();
    for r in page.get_records()? {
        let values = r
            .get_values()?
            .iter()
            .map(|v| Ok((v.get_path()?.to_string()?, v.get_text()?.to_string()?)))
            .collect::<capnp::Result<Vec<_>>>()?;
        rows.push((r.get_id()?.to_string()?, texts(r.get_may()?)?, values));
    }
    Ok(Page {
        rows,
        may: texts(page.get_may()?)?,
    })
}

/// The answer of one `invoke`.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub failed: bool,
    pub kind: String,
    pub record: String,
    pub text: String,
}

pub async fn invoke(
    source: &form_source::Client,
    command: &str,
    record: &str,
    pairs: &[(&str, &str)],
) -> Answer {
    let mut request = source.invoke_request();
    {
        let mut p = request.get();
        p.set_command(command);
        p.set_record(record);
        let mut args = p.init_args(u32::try_from(pairs.len()).unwrap());
        for (i, (k, v)) in pairs.iter().enumerate() {
            let mut a = args.reborrow().get(u32::try_from(i).unwrap());
            a.set_path(k);
            a.set_text(v);
        }
    }
    let answer = request.send().promise.await.unwrap();
    let answer = answer.get().unwrap();
    let text = |r: capnp::Result<capnp::text::Reader<'_>>| r.unwrap().to_string().unwrap();
    Answer {
        failed: answer.get_failed(),
        kind: text(answer.get_problem().unwrap().get_type()),
        record: text(answer.get_result().unwrap().get_record()),
        text: text(answer.get_result().unwrap().get_text()),
    }
}

pub fn field<'a>(row: &'a Row, name: &str) -> &'a str {
    row.2
        .iter()
        .find(|(p, _)| p == name)
        .map_or("", |(_, t)| t.as_str())
}
