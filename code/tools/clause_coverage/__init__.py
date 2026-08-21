"""Clause-level card-validation tooling — the denominator + coverage pipeline.

First two pieces of a card-validation workflow that uses DCGO recordings as a
parity oracle. A card is not a pass/fail unit: it has several independently
testable **clauses** (on-play, when-attacking, on-deletion, inherited,
security, each printed keyword, ...), and each fails independently. The
tracked unit is (card, clause).

Modules:

- `text_split`   — pure text -> clause-span splitter (timing markers / angle-
  bracket keywords -> discrete clauses). No I/O.
- `models`       — `Clause` dataclass + JSON (de)serialization helpers.
- `card_sources` — source-priority resolution per card: `data/card_bundles/
  <ID>.md` (via `data/card_official.json`'s structured `text_sections`) ->
  `data/cards.json` + `data/card_overrides.json` -> image-required.
- `extract`      — CLI: decklist / card-ID list -> per-card clause list (the
  denominator). `python -m tools.clause_coverage.extract --help`
- `coverage`     — CLI: clause list + a directory of DCGO `.jsonl` recordings
  -> what was and was not exercised. `python -m tools.clause_coverage.coverage --help`

See `README.md` in this directory for the source-priority rules and the
known gaps (most importantly: clause-level firing is NOT measurable from
today's DCGO recordings — see the README and `coverage.py`'s docstring).
"""
