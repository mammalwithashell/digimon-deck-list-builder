# Store Night Recommender — Implementation Plan

## Overview

Build a CLI tool (`tools/store_night.py`) that helps you decide **which deck to bring** and **how to tune it** for a specific store, plus enhance the data pipeline with player tracking and sample-size-aware sleeper detection.

---

## 1. Personal Deck Library (`my_decks.json`)

### Format
A single JSON file, archetype-keyed, separate and distinct from scraped `deck_library.json`:

```json
{
  "Rocks": {
    "decklists": [
      {
        "name": "anti-Medusamon tech",
        "deck": ["BT24-001", "BT24-001", "EX10-036", "..."],
        "notes": "Heavy removal build for red-heavy metas"
      },
      {
        "name": "standard",
        "deck": ["BT24-001", "BT24-001", "..."],
        "notes": "Default list"
      }
    ]
  },
  "Millenniummon": {
    "decklists": [
      {
        "name": "main",
        "deck": ["P-220", "..."]
      }
    ]
  }
}
```

### Resolution order
When the store night tool needs a deck for an archetype you provide:
1. **Your personal library** (`--library my_decks.json`) — first decklist for the archetype
2. Falls back to `deck_library.json` only if the archetype is missing from your library

### General tech cards
A top-level `"general_pool"` key in `my_decks.json` defines cards any archetype's architect can consider — generic removal, draw power, defensive options, etc:

```json
{
  "general_pool": ["BT24-099", "EX10-068", "ST17-016", "..."],
  "Rocks": { ... },
  "Millenniummon": { ... }
}
```

### Architect candidate pool (3 layers)
When `--optimize` runs, the candidate pool is the **union** of:
1. **Your personal decklists** for that archetype (cards you've considered)
2. **Scraped decklists** for that archetype from `deck_library.json` (cards other players have tried)
3. **General tech cards** from `general_pool` (flexible cards worth considering for any archetype)

All filtered to implemented (frozen-script) cards only. This gives the architect a broad but grounded search space.

The scraped `deck_library.json` is also used for **opponent decks** (what the store's meta plays against you).

---

## 2. Enhance `digilab_client.py` — temporal filtering + per-store split + player data

### 2a. Add `since_date` parameter to `get_scoped_meta()`
- Add optional `since_date: Optional[str]` (ISO format) parameter
- Add `WHERE t.event_date >= %s` clause when provided
- Lets you weight recent meta (e.g., last 3 months) vs all-time

### 2b. Add `get_player_history()` function
- New query joining `results` with any player identity columns in the DigiLab DB
- Returns per-player: player name, store, archetype history, dates, placements
- Falls back gracefully if DigiLab's `results` table lacks player columns (we'll probe the schema)

### 2c. Add median/mean play count to `get_scoped_meta()` return
- Compute `median_times_played` and `mean_times_played` across all archetypes in the scoped query
- Return as separate fields alongside the per-archetype dict so callers can threshold sleepers

---

## 3. Enhance `meta_loader.py` — store player data on ingest

### 3a. Add `player_name` to `IngestedDeck` dataclass
- New optional field: `player_name: Optional[str] = None`

### 3b. Capture player names from DigiLab ingest
- In `scrape_digilab()`: probe for player-related columns (e.g., `player_name`, `user_name`) in the results table, select them if available
- In `_parse_egman_row()`: return player name from column 2 (currently parsed but discarded)

### 3c. Store player data in `deck_library.json`
- Add optional `player_name` field to each decklist entry
- Add per-archetype `players` summary: `{player_name: {times_played, stores, last_seen}}`

---

## 4. Sample-size-aware sleeper detection

### 4a. In `digilab_client.py` `get_scoped_meta()` return data
- Already returns `times_played` per archetype
- Median and mean added in §2c above

### 4b. In the store night tool
- **Sleeper qualification**: only flag as sleeper if `times_played >= max(3, median_times_played / 2)` AND `conversion_rate > 50%`
- **Confidence display**: show sample size alongside conversion rate so user can judge
- Archetypes with 1-2 plays are shown but explicitly marked as insufficient data

### 4c. Update `local_meta_dfw.json` output format
- Add `median_times_played` and `mean_times_played` to the scope metadata

---

## 5. New CLI tool: `tools/store_night.py`

### Core workflow:
```
python tools/store_night.py \
    --store "The Card Haven" \
    --archetypes "Rocks,Millenniummon,Dark Masters" \
    --library my_decks.json \
    --since 2025-12-01 \
    --games 100 \
    --optimize
```

### Input:
- `--store NAME` — store name (looked up in DigiLab)
- `--archetypes NAME,NAME,...` — your candidate archetypes
- `--library PATH` — path to your personal deck library JSON (default: `my_decks.json`)
- `--since DATE` — only consider tournaments after this date (default: 3 months ago)
- `--games N` — games per matchup for evaluation (default: 50)
- `--pilot PATH` — pilot policy (default: "greedy")
- `--optimize` — also run deck optimization for the top pick
- `--optimize-episodes N` — architect training episodes if optimizing (default: 100)
- `--workers N` — parallel sim workers (default: 1)
- `--min-plays N` — minimum local plays for an archetype to count as a real meta threat (default: 3)

### Steps:
1. **Load store meta**: Call `get_scoped_meta(store_ids=[X], since_date=Y)` to get the current local meta with median/mean play counts
2. **Resolve your decks**: For each archetype you provide, load from your personal library (`--library`). Fall back to `deck_library.json` only if missing from your library.
3. **Build opponent list**: From local meta, pick representative decklists from `deck_library.json` (scraped data) weighted by local meta share. Filter archetypes below `--min-plays`. Exclude your candidate archetypes from the opponent pool.
4. **Evaluate each of your archetypes**: Use `DeckSimulator.evaluate_deck()` against the store's opponent list. Also compute per-opponent matchup win rates for the breakdown.
5. **Compute ETWR per archetype**: Expected Tournament Win Rate weighted by local meta shares
6. **Print recommendation**: Ranked table with ETWR + per-matchup breakdown
7. **Sleeper report**: Flag archetypes with high local conversion that have sufficient sample size (above `max(3, median/2)` plays)
8. **(Optional) Optimize**: If `--optimize`, run `MetaOptimizer` on the top-ranked archetype using a 3-layer candidate pool: your personal lists + scraped archetype lists + general tech cards. Suggest card swaps tuned for the store meta.

### Output format:
```
=== Store Night: The Card Haven ===
  Meta based on 45 results since 2025-12-01
  Median archetype plays: 4  |  Mean: 3.5

  YOUR ARCHETYPES (ranked by ETWR):
  #  Archetype         ETWR    vs Top3 Matchups
  1. Rocks             .621    Medusamon(.72) Zephagamon(.58) Jupitermon(.55)
  2. Millenniummon     .589    Medusamon(.65) Zephagamon(.52) Jupitermon(.61)
  3. Dark Masters      .513    Medusamon(.48) Zephagamon(.55) Jupitermon(.49)

  RECOMMENDATION: Bring Rocks

  LOCAL META THREATS (plays >= 3):
  Archetype         Share   WR     Conv   Plays
  Medusamon         10.2%   47.7%  46.2%  13
  Zephagamon        7.0%    53.3%  22.2%  9
  Rocks             6.3%    61.5%  50.0%  8
  ...

  SLEEPERS (conv > 50%, plays >= 3):
  Dark Masters      3.9%    94.1%  100%   5  ⚠ sufficient sample
  Millenniummon     2.3%    81.8%  100%   3  ⚠ at threshold

  INSUFFICIENT DATA (plays < 3):
  Chaos Control     3.1%    83.3%  100%   2  ? too few plays to trust

  OPTIMIZATION (Rocks from "anti-Medusamon tech"):
  Pool: 73 cards (your lists: 47, scraped: 18 new, general: 8)
  Swap: -1 BT24-055 → +1 EX10-042  (WR: .621 → .638)
  Swap: -1 BT22-019 → +1 BT24-017  (WR: .638 → .645)
```

---

## 6. File changes summary

| File | Change |
|------|--------|
| `digimon_gym/digilab_client.py` | Add `since_date` param, `get_player_history()`, median/mean play stats |
| `tools/meta_loader.py` | Add `player_name` to `IngestedDeck`, capture from DigiLab + Egman, store in library |
| `tools/store_night.py` | **New file** — the store night recommender CLI |
| `my_decks.json` | **New file** — example personal deck library (gitignored or committed, user choice) |
| `tests/test_store_night.py` | **New file** — tests for the recommender |
| `tests/test_digilab_client.py` | Tests for new temporal/player queries |

---

## 7. Design decisions

- **No DB imports** — `store_night.py` is engine-only (safe for desktop sidecar)
- **Two separate libraries** — your personal `my_decks.json` for your decks; scraped `deck_library.json` for opponent decks and meta data. Never mixed.
- **3-layer architect pool** — your personal lists + scraped archetype lists from `deck_library.json` + a `general_pool` of flexible tech cards. All three merged, deduped, filtered to implemented cards. Gives the architect broad reach while staying grounded in real card choices.
- **Player data is best-effort** — if DigiLab DB lacks player columns, we log a warning and skip; Egman player names are captured when available
- **Sleeper threshold** — `times_played >= max(3, median_plays / 2)` AND `conversion_rate > 50%`. Below threshold shown as "insufficient data" rather than hidden.
- **Greedy-first evaluation** — defaults to greedy pilot for speed; ONNX pilot optional for accuracy
