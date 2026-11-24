# Changelog

Version = tag.

## Unreleased

- Birth: local repository with an empty workspace (`fs-person`, `fs-person-service`), empty `catalog/`
  (`commands.json`, `forms.json`), CI copied from `fs-modules-planning` (`check.yml`, `windows.yml`), licences MIT OR Apache-2.0.
  No domain code yet.
- `fs-person` (G-2): `Anchor` (UUIDv7, `person_ref()`, AS2 form `{"type":"Person","id":"urn:uuid:…"}`), port `MeRepo` (`me()`, `create_me()`),
  adapter `StoreMe` over fs-objects (the one self-owned write is `birth`), `StoreOwner: LocalOwner` (no "me" → `NoMe`). Eight tests,
  among them the counter-proofs (second birth refused, self-ownership refused in `write_by`). Pins: fs-interfaces `e6f69213`,
  fs-objects `f6409f2a`, fs-types tag v0.0.6 (the one the other two use).
