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
- `fs-person-service` (G-3): the service over fs-rpc (binary `fs-person-service [serve]`, socket `default_socket("person")`, data in
  `$XDG_DATA_HOME/freesynergy/modules/person/`, heartbeat, one service per person by the bound socket). FormSource level 1: form
  `me@1.0.0` (one record or none) and command `me.create` (record = `urn:uuid:…`, text `me-created` / `me-exists`, a second call
  writes nothing). The actor is the connection only: no `--actor`, any argument of `me.create` is refused. `catalog/commands.json`
  and `catalog/forms.json` filled. Seven tests (P2–P5, P9, error cases, catalog), each with a mutant shown red.
- `fs-person-service` feature `client` (G-4): `PersonServiceOwner: LocalOwner` for other modules. Gets or starts the one service
  (`ensure_single`, `CommandStarter`, program next to the running one or in `/usr/bin`, never on `PATH`), reads the form `me` and gives
  `Ok(PersonRef)` / `NoMe` / `Unreachable { reason }` (`Invalid` for an answer that is no UUIDv7 person). Never creates "me". Runs
  on a thread and runtime of its own (synchronous). Four tests against the real binary: P1 (client, `owner::check` green, two
  questions one id, never creates) and P8 (no program, program that does not serve); three mutants shown red.
