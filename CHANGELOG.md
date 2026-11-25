# Changelog

Version = tag.

## Unreleased

- Pin: fs-rpc rev `0f639470` (was `9aff426a`); fs-interfaces rev `ad56e2f4` (was `a395979b`), the one fs-rpc uses, so there is one `fs-interfaces` in `cargo tree -d`. Written with the help of Claude (Anthropic).
- Auf Deutsch: Pin: fs-rpc rev `0f639470` (war `9aff426a`); fs-interfaces rev `ad56e2f4` (war `a395979b`), derselbe wie in fs-rpc, damit nur ein `fs-interfaces` in `cargo tree -d` steht. Geschrieben mit Hilfe von Claude (Anthropic).
- Pin: fs-rpc rev `9aff426a` (was `1de6cf2a`); fs-interfaces rev `a395979b` (was `e8699fb4`), the one fs-rpc uses, so there is one `fs-interfaces` in `cargo tree -d`. Written with the help of Claude (Anthropic).
- Auf Deutsch: Pin: fs-rpc rev `9aff426a` (war `1de6cf2a`); fs-interfaces rev `a395979b` (war `e8699fb4`), derselbe wie in fs-rpc, damit nur ein `fs-interfaces` in `cargo tree -d` steht. Geschrieben mit Hilfe von Claude (Anthropic).
- Pin: fs-rpc rev `1de6cf2a` (was `9585ed18`); fs-interfaces rev `e8699fb4` (was `83b96e30`), the one fs-rpc uses, so there is one `fs-interfaces` in `cargo tree -d`. Written with the help of Claude (Anthropic).
- Auf Deutsch: Pin: fs-rpc rev `1de6cf2a` (war `9585ed18`); fs-interfaces rev `e8699fb4` (war `83b96e30`), derselbe wie in fs-rpc, damit nur ein `fs-interfaces` in `cargo tree -d` steht. Geschrieben mit Hilfe von Claude (Anthropic).
- Pin: fs-rpc rev `9585ed18` (was `82d2ba19`). Written with the help of Claude (Anthropic).
- Auf Deutsch: Pin: fs-rpc rev `9585ed18` (war `82d2ba19`). Geschrieben mit Hilfe von Claude (Anthropic).
- Pin: fs-rpc rev `82d2ba19` (was `56f5cc39`). Written with the help of Claude (Anthropic).
- Auf Deutsch: Pin: fs-rpc rev `82d2ba19` (war `56f5cc39`). Geschrieben mit Hilfe von Claude (Anthropic).
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
  writes nothing). Nobody names the actor: no `--actor`, any argument of `me.create` is refused; the connection only admits the same user, and the log names the anchor as the actor of its birth (the caller is not recorded). `catalog/commands.json`
  and `catalog/forms.json` filled. Seven tests (P2–P5, P9, error cases, catalog), each with a mutant shown red.
- `fs-person-service` feature `client` (G-4): `PersonServiceOwner: LocalOwner` for other modules. Gets or starts the one service
  (`ensure_single`, `CommandStarter`, program next to the running one or in `/usr/bin`, never on `PATH`), reads the form `me` and gives
  `Ok(PersonRef)` / `NoMe` / `Unreachable { reason }` (`Invalid` for an answer that is no UUIDv7 person). Never creates "me". Runs
  on a thread and runtime of its own (synchronous). Four tests against the real binary: P1 (client, `owner::check` green, two
  questions one id, never creates) and P8 (no program, program that does not serve); three mutants shown red.
- Review fixes of bundle G: rule gate (`check_rust_rules.py`) green (module docs with `Pattern:`, `client/mod.rs` only declares parts), `service_program` never
  falls back to a bare name, texts about the actor corrected, tests for the `Invalid` answers of the client and P4 with `store owner mismatch`.
- Pin: fs-rpc rev `d8944ef9` (heartbeat fix: the end of a service no longer hangs when the folder of the socket is gone before it), version 0.1.4 unchanged. One source with fs-modules-planning; no doubled `fs-*` crate in `cargo tree -d`.
- Re-pin: fs-rpc and fs-trusted-path to rev `0120f9af` (fs-rpc main), version 0.1.4 unchanged; `Cargo.lock` by `cargo update -p fs-rpc -p fs-trusted-path`. One fs-rpc in the graph of fs-starter; no doubled `fs-*` crate in `cargo tree -d`.
- Pins (no Rust change): fs-types `rev 722d3ce0` (v0.0.7; was tag v0.0.6), fs-interfaces `rev 2bb70d82`, fs-objects `rev e38093f1`, fs-rpc and fs-trusted-path `rev 9cbce63c` (version 0.1.4 unchanged); `.cargo/config.toml` names the registry `freesynergy` (fs-types 722d3ce0 needs it). `cargo tree -d` shows no fs-* duplicate. No non_exhaustive wildcard arm was needed. Written with the help of Claude (Anthropic).
- Auf Deutsch: Pins (keine Rust-Aenderung): fs-types `rev 722d3ce0` (v0.0.7; war Tag v0.0.6), fs-interfaces `rev 2bb70d82`, fs-objects `rev e38093f1`, fs-rpc und fs-trusted-path `rev 9cbce63c` (Version 0.1.4 unveraendert); `.cargo/config.toml` nennt die Registry `freesynergy`. `cargo tree -d` zeigt kein fs-* Doppeltes. Kein Wildcard-Arm noetig. Geschrieben mit Hilfe von Claude (Anthropic).
- Licence gate: `fs-person-service` names `version = "0.0.1"` on its path dependency `fs-person` (cargo-deny `wildcard`; `deny.toml` unchanged). Auf Deutsch: Lizenz-Gate: `fs-person-service` nennt `version = "0.0.1"` an der Pfad-Abhaengigkeit `fs-person` (cargo-deny `wildcard`; `deny.toml` unveraendert).
- fs-rpc wave 2 (no Rust change): fs-rpc and fs-trusted-path to `rev 56f5cc39` (branch `unreviewed/rpc-wave-1125`, version 0.1.4), fs-interfaces to `rev 83b96e30`; fs-types `722d3ce0` and fs-objects `e38093f1` unchanged. `.config/nextest.toml` from `unreviewed/nextest-security` (profile `security`). `cargo tree -d` shows no fs-* duplicate. Auf Deutsch: fs-rpc-Welle 2 (keine Rust-Aenderung): fs-rpc und fs-trusted-path auf `rev 56f5cc39` (Zweig `unreviewed/rpc-wave-1125`, Version 0.1.4), fs-interfaces auf `rev 83b96e30`; fs-types und fs-objects unveraendert. `.config/nextest.toml` aus `unreviewed/nextest-security` (Profil `security`). Kein fs-* Doppeltes in `cargo tree -d`.
