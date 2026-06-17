---
name: digimon-card-lookup
description: Use when reasoning about, implementing, fixing, debugging, or QA'ing a specific Digimon card's effect — or a whole archetype's cards. Resolves a card ID, a card NAME (all printings — e.g. "Wormmon" is 8 distinct cards), or an archetype name to local card-image paths + printed text, so you VIEW the actual card face (the authoritative source for printed text) instead of relying only on cards.json or the YAML DSL. Triggers on "what does <card> do", "fix/implement/check <card>", "look up <card>", comparing printings, or implementing/auditing an archetype.
---

# Digimon Card Visual Lookup

When you are reasoning about what a Digimon card *says* or *does*, do not rely only on
`data/cards.json` (API-ingested, **not always accurate**) or the YAML DSL spec. **Look at
the actual printed card.** A complete local image mirror exists on this machine, and the
`Read` tool renders `.webp` natively — the card face is the **highest-fidelity source for
printed text** (effect / inherited / security / DP / level / color / traits).

This applies at three scopes: a **single card**, **all printings of a name**, and a
**whole archetype**.

## Workflow

### 1. Resolve the query → image paths + text

Run the resolver from the repo root (it self-detects the query type):

```bash
python .claude/skills/digimon-card-lookup/resolve_cards.py <query> [<query> ...]
```

| You have… | Example | What you get |
|---|---|---|
| a card **ID** | `BT20-065` | that printing (add `--editions` for alt-art `_P0/_P1/_P2`) |
| a card **NAME** | `Wormmon` | **every** printing — e.g. all 8 distinct Wormmon cards, each with different text |
| an **archetype** | `"Xros Heart"` | the unique card pool from `deck_library.json` (aliases applied) |

Force the type with `--type id|name|archetype` if auto-detection guesses wrong.
The output ends with an **`IMAGES TO READ`** list of absolute `.webp` paths.

A card missing from the local mirror is auto-downloaded from the digimoncard.io CDN into
`.cache/` (so it can still be `Read`); any card with no image anywhere is flagged.

### 2. View the cards — batch your `Read`s

- **One card / a few printings:** `Read` every listed `.webp` in a **single message**
  (parallel tool calls). Then you can see the art and read the printed text directly.
- **A whole archetype (40–60 cards):** the manifest is cheap, but **do NOT `Read` all 60
  images at once** — that floods context. Skim the manifest first, then `Read` images in
  **small batches** for the cards you are actively working on. Surface the manifest +
  any missing/downloaded cards up front.

### 3. Trust order

- **Digivolution requirements** (cost circles, level, colour, DNA/alt-digivolve "black
  text") — **use the authoritative bundle, NOT the image or cards.json.** The card image
  is *unreliable* for the small digivolve circles (colours are easy to misread — black vs
  blue, the level digit), and `data/cards.json` (digimoncard.io API ingest) **drops the
  second colour** of multi-colour digivolve lines. The authoritative source is the
  official Bandai DB, captured per card in **`data/card_bundles/<ID>.md`** (and the
  machine-readable **`data/card_official.json`**). `resolve_cards.py` prints the bundle
  path when one exists — `Read` it for digivolve costs. To (re)generate a bundle for a
  card not yet covered: `python code/tools/build_card_bundles.py --ids <ID>`.
- **Printed text** ("what does it say"): the **card image** is authoritative → the bundle's
  official text → `data/card_overrides.json` → `data/cards.json`.
- **Behavior** ("how does it resolve"): keep following the project's source-priority
  chain — DCGO C# → `general_rule.pdf` → fandom wiki. The image tells you the text;
  DCGO/rules tell you how that text actually plays out.

### Authoritative digivolution data — tooling

The flaky-API problem (cards.json drops off-colour digivolve circles; vision misreads
them) is solved by sourcing from the **official Bandai card DB** (`world.digimoncard.com`):

- `code/tools/scrape_official_evo_costs.py` — scrape authoritative `evo_costs` per card.
- `code/tools/build_card_bundles.py` — build the full `data/card_bundles/<ID>.md` resource
  (standard cost circles + Special Digivolution Condition + official text + image path) and
  the `data/card_official.json` index.

When implementing a card or writing a real-card test/scenario, prefer the bundle's
`evo_costs` over cards.json — they are the publisher's ground truth.

## Useful flags

- `--editions` — include alt-art editions (`_P0/_P1/_P2`) for an ID query.
- `--paths-only` — print just the image paths (handy when you only need the files to `Read`).
- `--json` — structured manifest.
- `--no-download` — skip the CDN fallback (local-only).

## Notes

- Image directory defaults to `C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card`;
  override with the `DIGIMON_CARD_IMAGE_DIR` env var.
- The `UserPromptSubmit` hook `digimon_card_image_hint.py` already reminds you (with paths)
  when a prompt mentions card IDs — but you still must run this skill / `Read` the images.
- For a bare card **name** or **archetype**, always go through `resolve_cards.py`: one name
  can be many cards, and you want to view the *right* printing.
