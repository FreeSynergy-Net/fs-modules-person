//! The type catalog of the module as a file: `catalog/types.bin` (ADR 0087 section 4, an addition: the file is the answer of
//! `TypeCatalog.describe` of `fs-types` `types.capnp`, in the same wire form).
//!
//! Pattern: Registry + Published Language – the catalog (`TypeRegistry`, with [`ProfileType`]) leaves the module only in the
//! wire form of `types.capnp`. Deviation: as a file in the package instead of a call, so install and check need no running service.
//! The bytes are canonical (Cap'n Proto canonicalization), so the same catalog is always the same file.

use capnp::message::{Builder, HeapAllocator, ReaderOptions};
use capnp::serialize;
use fs_types::TypeRegistry;
use fs_types::types_capnp::type_catalog::describe_results;

use crate::ProfileType;

/// The most words a catalog file may traverse when it is read (a file of the module is small; this bounds a damaged one).
const TRAVERSAL_LIMIT_WORDS: usize = 1 << 20;

/// The catalog this module describes: the profile type.
///
/// # Panics
/// Never in practice: the one description of the module validates (a test holds it).
#[must_use]
pub fn type_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    registry
        .register::<ProfileType>()
        .expect("the description of the profile validates");
    registry
}

fn message_of(registry: &TypeRegistry) -> Builder<HeapAllocator> {
    let mut message = Builder::new_default();
    let root = message.init_root::<describe_results::Builder<'_>>();
    registry.write(root.init_types(u32::try_from(registry.len()).unwrap_or(u32::MAX)));
    message
}

/// The bytes of `catalog/types.bin`: the answer of `TypeCatalog.describe` of this module, canonical, as one framed message.
///
/// # Panics
/// Never in practice: writing into memory does not fail.
#[must_use]
pub fn type_catalog_bytes() -> Vec<u8> {
    let first = message_of(&type_registry());
    let reader = first
        .get_root_as_reader::<describe_results::Reader<'_>>()
        .expect("the message just written reads");
    let mut canonical = Builder::new_default();
    canonical
        .set_root_canonical(reader)
        .expect("a message just written canonicalizes");
    let mut out = Vec::new();
    serialize::write_message(&mut out, &canonical).expect("writing into memory does not fail");
    out
}

/// The catalog a file of this form says (every description validated).
///
/// # Errors
/// A damaged or too large message, or a description that does not validate.
pub fn read_type_catalog(mut bytes: &[u8]) -> capnp::Result<TypeRegistry> {
    let mut options = ReaderOptions::new();
    options.traversal_limit_in_words(Some(TRAVERSAL_LIMIT_WORDS));
    let message = serialize::read_message(&mut bytes, options)?;
    let root = message.get_root::<describe_results::Reader<'_>>()?;
    TypeRegistry::read(root.get_types()?)
}
