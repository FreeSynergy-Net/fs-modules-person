//! The factory of the capability of one connection.
//!
//! Pattern: Factory – `PersonService` builds, for every connection fs-rpc accepts, a form source over the one repository. The
//! caller is read by the system (`SO_PEERCRED` on Unix, the token of the pipe client on Windows); a connection whose caller the
//! system does not tell is dropped by fs-rpc and reported to `refused`. Nothing a client sends says who it is.

use std::rc::Rc;
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::formsource_capnp::form_source;
use fs_person::MeRepo;
use fs_rpc::{Caller, CallerError, PerConnection};

use crate::source::FormSourceImpl;

/// The service: one repository, one form source per connection.
pub struct PersonService<R> {
    repo: Arc<R>,
    refused: Rc<dyn Fn(&CallerError)>,
    /// What lives as long as the service (the heartbeat); dropped with it.
    keep: Vec<Box<dyn std::any::Any>>,
}

impl<R: MeRepo + 'static> PersonService<R> {
    /// Serve `repo`; `refused` is told when the system did not say who is on a connection (that connection is dropped).
    pub fn new(repo: Arc<R>, refused: impl Fn(&CallerError) + 'static) -> Self {
        Self {
            repo,
            refused: Rc::new(refused),
            keep: Vec::new(),
        }
    }

    /// Keep `thing` alive as long as the service serves.
    #[must_use]
    pub fn keeping(mut self, thing: impl std::any::Any) -> Self {
        self.keep.push(Box::new(thing));
        self
    }
}

impl<R: MeRepo + 'static> PerConnection for PersonService<R> {
    fn bootstrap(&mut self, _caller: &Caller) -> capnp::capability::Client {
        // Admitted callers (the same user, see fs-rpc) may all do what the command does; who they are is not a value of any call.
        let source = FormSourceImpl {
            repo: self.repo.clone(),
        };
        new_client::<form_source::Client, _>(source).client
    }

    fn refused(&mut self, why: &CallerError) {
        (self.refused)(why);
    }
}
