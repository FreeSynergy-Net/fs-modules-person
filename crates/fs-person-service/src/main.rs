//! The program `fs-person-service`: the service of the person module. All of it is in the library.
//!
//! Pattern: Facade – one call. The module is the source of "me" itself, so there is no `LocalOwner` to ask first.

fn main() {
    std::process::exit(fs_person_service::serve_command(
        std::env::args_os().skip(1),
    ));
}
