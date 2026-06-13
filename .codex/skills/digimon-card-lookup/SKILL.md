---
name: digimon-card-lookup
description: Use when Codex needs to reason about, implement, fix, debug, QA, compare, or explain specific Digimon TCG cards, all printings of a card name, or an archetype card pool. Resolves card IDs, card names, and archetypes to local card-image paths plus printed-text fields so Codex can inspect the actual card face instead of relying only on API-ingested card JSON or existing YAML.
---

# Digimon Card Lookup

Use this skill whenever printed card text matters. `data/cards.json` is useful but API-ingested and sometimes wrong; the local card image is the highest-fidelity source for printed text. Use rules docs, DCGO, and rulings for behavior after you know what the card actually says.

## Workflow

1. Resolve the query from the repo root:

```bash
python .codex/skills/digimon-card-lookup/scripts/resolve_cards.py <query> [<query> ...]
```

Use exact card IDs when available. For names, resolve all printings. For archetypes, resolve the card pool through `data/deck_library.json` and aliases.

2. Inspect the returned card images.

The resolver prints an `IMAGES TO READ` list of absolute `.webp` paths. Use `view_image` for one or a few cards, or batch visual inspection only for the cards currently under implementation or audit. Do not load a full 40-60 card archetype image pool into context at once.

3. Cross-check text and behavior.

- Printed text priority: card image, then `data/card_overrides.json`, then `data/cards.json`.
- Behavior priority: official rules PDF and DCGO C# source, then Fandom ruling notes, then local decomposed rules docs as an index.
- If the image, overrides, JSON, YAML, or tests disagree, call out the discrepancy before implementing or claiming readiness.

## Query Types

| Input | Example | Result |
|---|---|---|
| Card ID | `BT20-065` | That printing; add `--editions` for alt-art files |
| Card name | `Wormmon` | Every matching printing, since each printing can have different text |
| Archetype | `"Xros Heart"` | Unique card pool from `deck_library.json`, with aliases applied |

Use `--type id|name|archetype` when auto-detection guesses wrong.

## Useful Flags

- `--editions`: include alt-art editions such as `_P0`, `_P1`, `_P2`.
- `--paths-only`: print only resolved image paths.
- `--json`: emit a structured manifest.
- `--no-download`: skip CDN fallback and use only local files.

The image directory defaults to `C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card`; override with `DIGIMON_CARD_IMAGE_DIR`.
