//! Where the program file of the service is looked for.
//!
//! Pattern: Specification – a fixed rule (next to the running program, then `/usr/bin`, never `PATH` and never a relative path)
//! that says where the server may come from; a planted file in a folder of the search path must not become the server.

use std::path::{Path, PathBuf};

/// The program file of the service for the program `running` (an absolute path): the regular file next to `running`, else in
/// `/usr/bin` (Unix); if there is none, the place next to `running`, so that the start fails visibly (`Unreachable`) and never
/// falls back to a search on `PATH` or in the working folder. `None` if `running` is not an absolute path with a folder.
#[must_use]
pub fn service_program(running: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "fs-person-service.exe"
    } else {
        "fs-person-service"
    };
    if !running.is_absolute() {
        return None;
    }
    let next = running.parent()?.join(name);
    let usr_bin = (!cfg!(windows)).then(|| Path::new("/usr/bin").join(name));
    [Some(next.clone()), usr_bin]
        .into_iter()
        .flatten()
        .find(|p| std::fs::metadata(p).is_ok_and(|m| m.is_file()))
        .or(Some(next))
}
