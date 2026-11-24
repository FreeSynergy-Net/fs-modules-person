//! The local owner, read from the store of the module `person`.
//!
//! Pattern: Strategy – the variant of `LocalOwner` for the single place (the type has the details and the deviation).

use fs_interfaces::owner::{LocalOwner, OwnerProblem};
use fs_types::PersonRef;

use crate::MeRepo;

/// Pattern: Strategy (the variant of `LocalOwner` for the single place) – reads `me()` of the repository; without "me" it
/// fails loudly with `NoMe`, never with a placeholder.
///
/// Deviation: `LocalOwner` is synchronous, the port is async; every call runs the port on a thread and runtime of its own,
/// so it works inside and outside a runtime (read once per process, ADR 0104 N1).
pub struct StoreOwner<R>(pub R);

impl<R: MeRepo> LocalOwner for StoreOwner<R> {
    fn owner(&self) -> Result<PersonRef, OwnerProblem> {
        let read = std::thread::scope(|s| {
            s.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                rt.block_on(self.0.me()).map_err(|e| e.to_string())
            })
            .join()
            .unwrap_or_else(|_| Err("the reading thread panicked".to_owned()))
        });
        match read {
            Ok(Some(anchor)) => Ok(anchor.person_ref()),
            Ok(None) => Err(OwnerProblem::NoMe {
                hint: "\"me\" is not created yet: create it in the module person".to_owned(),
            }),
            Err(reason) => Err(OwnerProblem::Unreachable { reason }),
        }
    }
}
