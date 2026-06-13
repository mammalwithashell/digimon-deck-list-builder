# Digimon Card Visual Lookup — Design

**Date:** 2026-06-02
**Status:** Approved (brainstorming) — ready for implementation plan
**Author:** Claude (with james)

## Problem

When reasoning about, implementing, fixing, or QA'ing a Digimon card's effect, the
assistant currently leans on `data/cards.json` (API-ingested, *not always accurate*)
and the YAML DSL specs. The actual printed card face — the highest-fidelity source
for what a card *says* — is never consulted, even though a complete local image
mirror exists on this machine. The goal: make visual card lookup a reliable,
low-friction part of how cards are reasoned about, at three scopes:

1. **A single card** (by ID or name).
2. **Multiple printings / editions** — e.g. "Wormmon" is **8 distinct cards** across
   sets, each with different effect text; a printing can also have alt-art editions
   (`_P0/_P1/_P2`).
3. **A whole archetype** — every card in an archetype's pool at once.

## Grounded facts (verified 2026-06-02)

- **Local image store:** `C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card\<ID>.webp`
  — 6,275 files, all sets (AD1, BT1–25, EX1–11, ST1–24, LM, P, RB1…). Alt arts use
  `_P0/_P1/_P2` suffixes. The mirror is *slightly incomplete* (~3/16 missing in an
  ST-2 sample), so a fallback is required.
- **The `Read` tool renders `.webp` natively** at full readable quality — art **and**
  printed text (effect / inherited / security / DP / level / color / traits) are legible.
  No conversion step needed. The `Read` tool also supports **multiple parallel reads in
  one message** → batch viewing.
- **CDN fallback:** `https://images.digimoncard.io/images/cards/<ID>.webp` serves cards
  missing locally (HTTP 200, valid WebP confirmed for ST2-04).
- **`data/cards.json`** is a dict keyed by card ID. Relevant fields:
  `card_name_eng`, `card_id`, `effect_description_eng`,
  `inherited_effect_description_eng`, `security_effect_description_eng`.
- **`data/card_overrides.json`** holds trusted hand-maintained text corrections.
- **Archetypes:** `data/deck_library.json` → `archetypes[name].decklists[].decklist`
  is a **JSON-encoded string** of card IDs (with copies). `data/archetype_aliases.json`
  maps canonical archetype names → aliases.
- **`Wormmon` → 8 printings:** BT3-047, BT12-047, BT16-040, BT20-065, BT23-040,
  EX3-055, P-118, ST9-08 — each with distinct effect text.

## Components

### 1. Resolver helper — `.claude/skills/digimon-card-lookup/resolve_cards.py`

Pure resolution (no viewing). Takes one or more queries; for each emits a manifest:
resolved card ID(s), local `.webp` path (or downloaded-cache path), and the JSON text
fields. Query auto-detection:

| Query type | Example | Resolves to |
|---|---|---|
| Card ID | `BT20-065` | that printing; `--editions` adds `_P0/_P1/_P2` alt arts |
| Card name | `Wormmon` | **all** matching printings (all 8 IDs) |
| Archetype | `"Xros Heart"` | unique card pool from `deck_library.json` (aliases applied) |

**Image dir:** read from `DIGIMON_CARD_IMAGE_DIR`, default
`C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card`.

**CDN fallback:** any ID missing locally is downloaded from
`https://images.digimoncard.io/images/cards/<ID>.webp` into
`.claude/skills/digimon-card-lookup/.cache/` (gitignored), so it can always be `Read`.

**Output:** human-readable manifest plus, per card, the printed-text fields, and a
clear list of which images are local / downloaded / genuinely unavailable.

### 2. The skill — `.claude/skills/digimon-card-lookup/SKILL.md`

Description tuned to trigger whenever reasoning about / implementing / fixing / QA'ing
a specific card or archetype's effects. Directs the workflow:

1. Resolve the query via `resolve_cards.py` (name → all printings; archetype → full pool).
2. **Batch-`Read`** the relevant `.webp`(s):
   - 1 card / a few printings → `Read` all in **one parallel message**.
   - Whole archetype (40–60 cards) → resolve the full manifest cheaply, then `Read`
     images in **batches** while working through cards (never 60 vision-loads at once —
     that's a context bomb). Surface the manifest + any missing/downloaded cards up front.
3. Treat the **image as the authoritative source for *printed text*** — it outranks the
   API-ingested `cards.json`. Cross-check `card_overrides.json` for machine-readable text.
4. For *behavior*, continue down the existing source-priority chain
   (DCGO C# → `general_rule.pdf` → fandom).

### 3. The reminder hook — `.claude/hooks/digimon_card_image_hint.py` (`UserPromptSubmit`)

- Regex-detects card IDs (`\b(BT|EX|ST|RB|AD|LM|P)\d{0,2}-\d{2,3}\b`); lists each with
  its resolved local path.
- If a bare card **name** or **archetype** term appears, nudges to run `resolve_cards.py`
  (one name → many cards).
- Cheap (regex only), silent when nothing matches.
- Registered in the **base repo's** `.claude/settings.local.json` (the image path is
  machine-specific; this file is gitignored and never merged).

## Source-priority reframe

The local card image is the highest-fidelity source for **printed text** (it is the
literal card face). It slots into CLAUDE.md's existing source-priority chain *above*
API-ingested `cards.json` for "what does the card say" — while DCGO/`general_rule.pdf`
remain authoritative for *behavior*. (A CLAUDE.md note documents this.)

## Install / workflow

- Files written **directly into the base repo** (`...\digimon-deck-list-builder-1\.claude\`),
  not this worktree — worktree `.claude/` is gitignored/ephemeral.
- Skill + hook script + resolver: **committed on `main`** in the base repo (`.claude/skills/`
  is git-tracked).
- Hook registration in `.claude/settings.local.json`: a **direct local edit**, left
  uncommitted (gitignored by design).
- **No PR, no merge.**

## Out of scope (YAGNI)

- Fuzzy name matching in the hook (resolver handles name→ID).
- Viewing remote URLs without download (the vision pipeline needs a local file).
- Auto-refresh / re-mirroring the local image set.
