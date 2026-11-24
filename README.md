# FreeSynergy person modules

FreeSynergy person module: the one person of an installation (the anchor, an AS2 `Person` with its `urn:uuid` id), served as a
module service with its form source. Profile data (names, addresses, links to accounts) comes later, as profiles of this anchor.

**Status: `fs-person` exists.** The anchor (`Anchor`), the port `MeRepo`, its adapter `StoreMe` over fs-objects and `StoreOwner`
(a `LocalOwner` of fs-interfaces) are in. The service (`fs-person-service`) is the next step.

## Crates

- `fs-person`: the module (the anchor, `MeRepo`, `StoreMe`, `StoreOwner`), without fs-rpc.
- `fs-person-service`: the module as a service over fs-rpc, one process per person. Empty for now.

## About the AI help

This repository is written with the help of Claude (Anthropic), an AI. The commits say so (`Co-Authored-By`). Every change is checked
by tests and by a second review before it lands on `main`.

Licence: MIT OR Apache-2.0.

## Auf Deutsch

FreeSynergy-Personenmodul: die eine Person einer Installation (der Anker, ein AS2-`Person` mit seiner `urn:uuid`-Kennung), als
Modul-Dienst mit seiner Formquelle. Profildaten (Namen, Adressen, Verweise auf Konten) kommen später als Profile dieses Ankers.

**Stand: `fs-person` steht.** Anker (`Anchor`), Port `MeRepo`, Adapter `StoreMe` über fs-objects und `StoreOwner`
(ein `LocalOwner` aus fs-interfaces) sind drin. Der Dienst (`fs-person-service`) ist der nächste Schritt.

Mit Hilfe von Claude (KI, Anthropic) gebaut. Die Commits nennen das (`Co-Authored-By`). Jede Änderung wird durch Tests und eine
zweite Prüfung geprüft, bevor sie auf `main` landet. Lizenz MIT OR Apache-2.0.
