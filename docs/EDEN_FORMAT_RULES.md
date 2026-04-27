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
