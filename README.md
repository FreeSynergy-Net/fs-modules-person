# FreeSynergy person modules

FreeSynergy person module: the one person of an installation (the anchor, an AS2 `Person` with its `urn:uuid` id), served as a
module service with its form source. Profile data (names, addresses, links to accounts) comes later, as profiles of this anchor.

**Release: version 0.0.1 is the first shipped module `person` (RPM `fs-module-person`, built by CI from the tag with `fs-blueprint package module`).** Working version, not yet for end users.

**Status: `fs-person` and `fs-person-service` exist.** The anchor (`Anchor`), the port `MeRepo`, its adapter `StoreMe` over fs-objects and `StoreOwner`
(a `LocalOwner` of fs-interfaces) are in. The service serves the form source as `Guarded<PersonCore>` (rights: `PersonRules`, only the owner as a person): the command `me.create` and the forms `me`, `rows` (the people as a list), `tiles` (the same people as tiles), `single-record` (one profile as a whole), all read only except `me.create`; `fs-person-service --sample` serves fixed test data (hello says `fake`); nobody names an actor (no `--actor`, no argument); the connection only admits the same user, and the log names the anchor as the actor of its own birth. The feature `client` of `fs-person-service` gives other modules `PersonServiceOwner` (a `LocalOwner`: gets or starts the service, reads the form `me`, never creates it).

**Writing profiles:** the commands `person.profile.create` (`name`, optional `email`, `phone`, `describes=me`), `person.update` (`id`, without `id` the profile that describes me; `name`, `email`, `phone`) and `person.delete` (`id`; "me" is never deleted) are in the service and in `catalog/commands.json`; port `ProfileWrite`. The sample data is read only.

## Crates

- `fs-person`: the module (the anchor, the profiles, `MeRepo`, `ProfileRepo`, `PersonRepo` with the adapter `PersonStore` and the sample data `SampleRepo`, `StoreOwner`; the type catalog as a file, `catalog_file.rs`, with `catalog/types.bin` and the words `texts/en.mf2`, `texts/de.mf2`), without fs-rpc.
- `fs-person-service`: the module as a service over fs-rpc, one process per person. Binary `fs-person-service [serve]`, socket `default_socket("person")`, data in `$XDG_DATA_HOME/freesynergy/modules/person/`.

## About the AI help

This repository is written with the help of Claude (Anthropic), an AI. The commits say so (`Co-Authored-By`). Every change is checked
by tests and by a second review before it lands on `main`.

Licence: MIT OR Apache-2.0.

## Auf Deutsch

FreeSynergy-Personenmodul: die eine Person einer Installation (der Anker, ein AS2-`Person` mit seiner `urn:uuid`-Kennung), als
Modul-Dienst mit seiner Formquelle. Profildaten (Namen, Adressen, Verweise auf Konten) kommen später als Profile dieses Ankers.

**Release: Version 0.0.1 ist das erste ausgelieferte Modul `person` (RPM `fs-module-person`, von der CI aus dem Tag mit `fs-blueprint package module` gebaut).** Arbeitsversion, noch nicht für Endnutzer.

**Stand: `fs-person` und `fs-person-service` stehen.** Anker (`Anchor`), Port `MeRepo`, Adapter `StoreMe` über fs-objects und `StoreOwner`

**Profile schreiben:** die Befehle `person.profile.create` (`name`, optional `email`, `phone`, `describes=me`), `person.update` (`id`, ohne `id` das Profil, das mich beschreibt; `name`, `email`, `phone`) und `person.delete` (`id`; „ich“ wird nie gelöscht) stehen im Dienst und in `catalog/commands.json`; Port `ProfileWrite`. Beispieldaten sind nur lesbar.
(ein `LocalOwner` aus fs-interfaces) sind drin. Der Dienst (`fs-person-service`) steht: Befehl `me.create` und Formen `me`, `rows` (die Personen als Liste), `tiles` (dieselben Personen als Kacheln), `single-record` (ein Profil als Ganzes), alles nur lesend außer `me.create`; `fs-person-service --sample` bedient feste Testdaten (hello sagt `fake`); niemand nennt einen Akteur (kein `--actor`, kein Argument); die Verbindung lässt nur denselben Benutzer zu, und im Protokoll steht der Anker als Akteur seiner eigenen Geburt. Das Feature `client` von `fs-person-service` gibt anderen Modulen `PersonServiceOwner` (ein `LocalOwner`: holt oder startet den Dienst, liest die Form `me`, legt sie nie an).

Mit Hilfe von Claude (KI, Anthropic) gebaut. Die Commits nennen das (`Co-Authored-By`). Jede Änderung wird durch Tests und eine
zweite Prüfung geprüft, bevor sie auf `main` landet. Lizenz MIT OR Apache-2.0.
