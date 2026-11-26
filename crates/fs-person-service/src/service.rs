//! The factory of the capability of one connection.
//!
//! Pattern: Factory – `PersonService` hands every connection fs-rpc accepts the `FormSource` of the one `Guarded<PersonCore>` (rights
//! asked by `PersonRules` before every change and every read, never by this file). The caller is read by the system (`SO_PEERCRED`
//! on Unix, the token of the pipe client on Windows); a connection whose caller the system does not tell is dropped by fs-rpc and
//! reported to `refused`. The caller is admitted and typed (a person or a bot, `ActorOfConnection`), never recorded: nothing a client
//! sends names an actor, and the log has the anchor as the actor of its birth.

use std::rc::Rc;
use std::sync::Arc;

use fs_person::PersonRepo;
use fs_rpc::{
    ActorOfConnection, Caller, CallerError, Guarded, OsUserIsPerson, PerConnection, Ports,
};

use crate::core::PersonCore;
use crate::owner::Owner;

/// The service: one repository behind one guarded core, one `FormSource` per connection.
pub struct PersonService<R: PersonRepo + 'static> {
    guarded: Guarded<PersonCore<R>>,
    refused: Rc<dyn Fn(&CallerError)>,
    /// What lives as long as the service (the heartbeat); dropped with it.
    keep: Vec<Box<dyn std::any::Any>>,
}

impl<R: PersonRepo + 'static> PersonService<R> {
    /// Serve `repo`: "me" is read ONCE here (before the loop serves, never in it), and every admitted connection is a person (the
    /// OS user of the service, `OsUserIsPerson`). `refused` is told when the system did not say who is on a connection.
    ///
    /// # Errors
    /// The store fails or holds something that is no anchor: the service does not start.
    pub async fn open(
        repo: Arc<R>,
        refused: impl Fn(&CallerError) + 'static,
    ) -> fs_objects::Result<Self> {
        Self::open_as(repo, OsUserIsPerson, refused).await
    }

    /// [`open`](Self::open) with another source of "person or bot" (a test that shows a bot is refused).
    ///
    /// # Errors
    /// As [`open`](Self::open).
    pub async fn open_as(
        repo: Arc<R>,
        actor: impl ActorOfConnection + 'static,
        refused: impl Fn(&CallerError) + 'static,
    ) -> fs_objects::Result<Self> {
        let owner = Owner::read(&*repo).await?;
        let core = PersonCore::new(repo, owner.clone());
        Ok(Self::guarding(core, Ports::new(actor, owner), refused))
    }

    /// Serve `core` behind the wrapper with `ports` (the one way `FormSource` leaves this module).
    pub fn guarding(
        core: PersonCore<R>,
        ports: Ports,
        refused: impl Fn(&CallerError) + 'static,
    ) -> Self {
        Self {
            guarded: Guarded::new(core, ports),
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

impl<R: PersonRepo + 'static> PerConnection for PersonService<R> {
    fn bootstrap(&mut self, caller: &Caller) -> capnp::capability::Client {
        self.guarded.connect(caller).client
    }

    fn refused(&mut self, why: &CallerError) {
        (self.refused)(why);
    }
}
