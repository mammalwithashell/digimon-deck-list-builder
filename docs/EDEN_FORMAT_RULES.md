# EDEN Format Ruleset

Implementation source: Digimon Card Game EDEN Format Rules & Guidance, version
1.1.1, last updated 18/01/2025.

EDEN uses normal Digimon Card Game gameplay and deck-size rules. Deck legality
differs from standard play:

- Main deck remains exactly 50 cards.
- Digi-Egg deck remains 0-5 cards.
- Common and uncommon cards are legal by default.
- Digi-Eggs of any rarity are legal unless EDEN bans or limits them.
- Rare-or-higher cards are illegal unless they count as one of the EDEN
  Anomaly Protocol cards.
- A deck may contain at most 4 total EDEN Anomaly Protocol cards.

EDEN Anomaly Protocol cards are:

- Rare or promo Tamers.
- Rare, super-rare, or promo Memory Boost options.
- Promo Training options.
- Promo Scramble options.

The implementation also applies EDEN's custom banned, limited, and banned-pair
list. `EX02-007` from the source document is normalized to the card database ID
`EX2-007`.

The ruleset is available to validation callers as `game_mode: "eden"`.

## Configuration source (single source of truth)

As of the `add-deck-format-registry` change, EDEN's banlist, the EDEN Anomaly
Protocol definition, and every format descriptor live in **`data/deck_formats.json`**
— baked into the Rust engine via `include_str!` and read at runtime by the
hosted API, so there is no longer a parallel hardcoded list in Rust or Python.
To adjust EDEN:

- **Banlist / limits / choice groups:** edit `restrictions.eden` in that file
  (`banned`, `limited`, `limited_to`, `choice_groups`).
- **Anomaly Protocol:** edit `anomaly_protocol` — add a `categories` rule
  (match by `card_kind` + optional `name_contains` + legal `rarities`) or append
  explicit card IDs to `extra_card_ids`. `max_total` caps the total anomaly count.

Changes take effect for the hosted API on restart; for the desktop app they ship
with the next build (the file is compile-time baked). The engine `format` module
parses it into the `FormatDescriptor` registry that drives both validation and
the `card_legality(card_id, game_mode)` query.

## EDEN Singleton

`game_mode: "eden_singleton"` is EDEN played highlander: the EDEN Anomaly rarity
policy and the EDEN banlist apply unchanged, plus every card is limited to a
single copy. The anomaly total cap (≤ `max_total`) still applies independently
of the one-copy rule.
