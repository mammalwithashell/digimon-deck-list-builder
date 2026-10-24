# Deck pools for the lowering-coverage report

- `digilab-11087-top10.json` — the DigiLab tournament 11087 (EX13 window) top-10 placings, snapshot 2026-10-21; a JSON list of `{name, cards}` (one card id per copy), the shape `PYTHONPATH=code python -m tools.lowering_coverage --report <out>.md --decks <file>` expects. Two Jupitermon lists (3 and 8) are distinct placings of the same archetype. This is the pool the construct-test campaign (spec §5d) measures against.
