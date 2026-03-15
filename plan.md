# Store Night Recommender — Implementation Plan

## Overview

Build a CLI tool (`tools/store_night.py`) that helps you decide **which deck to bring** and **how to tune it** for a specific store, plus enhance the data pipeline with player tracking and sample-size-aware sleeper detection.

---

## 1. Enhance `digilab_client.py` — temporal filtering + per-store split + player data

### 1a. Add `since_date` parameter to `get_scoped_meta()`
- Add optional `since_date: Optional[str]` (ISO format) parameter
- Add `WHERE t.event_date >= %s` clause when provided
- Lets you weight recent meta (e.g., last 3 months) vs all-time

### 1b. Add `get_player_history()` function
- New query joining `results` with any player identity columns in the DigiLab DB
- Returns per-player: player name, store, archetype history, dates, placements
- Falls back gracefully if DigiLab's `results` table lacks player columns (we'll probe the schema)

### 1c. Add median play count to `get_scoped_meta()` return type
- Compute `median_times_played` across all archetypes in the scoped query
- Add to return value so callers can threshold sleepers against it

---

## 2. Enhance `meta_loader.py` — store player data on ingest

### 2a. Add `player_name` to `IngestedDeck` dataclass
- New optional field: `player_name: Optional[str] = None`

### 2b. Capture player names from DigiLab ingest
- In `scrape_digilab()`: probe for player-related columns (e.g., `player_name`, `user_name`) in the results table, select them if available
- In `_parse_egman_row()`: return player name from column 2 (currently parsed but discarded)

### 2c. Store player data in `deck_library.json`
- Add optional `player_name` field to each decklist entry
- Add per-archetype `players` summary: `{player_name: {times_played, stores, last_seen}}`

---

## 3. Sample-size-aware sleeper detection

### 3a. In `digilab_client.py` `get_scoped_meta()` return data
- Already returns `times_played` per archetype
- Add `median_times_played` and `mean_times_played` to the scoped meta output

### 3b. In the new store night tool (see §4)
- **Sleeper qualification**: only flag as sleeper if `times_played >= median_times_played / 2` (or configurable floor, default 3)
- **Confidence display**: show sample size alongside conversion rate so user can judge

### 3c. Update `local_meta_dfw.json` output format
- Add `median_times_played` and `mean_times_played` to the scope metadata

---

## 4. New CLI tool: `tools/store_night.py`

### Core workflow:
```
python tools/store_night.py \
    --store "The Card Haven" \
    --archetypes "Rocks,Millenniummon,Dark Masters" \
    --since 2025-12-01 \
    --games 100 \
    --optimize
```

### Input:
- `--store NAME` — store name (looked up in DigiLab)
- `--archetypes NAME,NAME,...` — your candidate archetypes (best decklist resolved from `deck_library.json` per archetype, using source preference: digilab > digimonmeta > egman)
- `--since DATE` — only consider tournaments after this date (default: 3 months ago)
- `--games N` — games per matchup for evaluation (default: 50)
- `--pilot PATH` — pilot policy (default: "greedy")
- `--optimize` — also run deck optimization for the top pick
- `--optimize-episodes N` — architect training episodes if optimizing (default: 100)
- `--workers N` — parallel sim workers (default: 1)
- `--min-plays N` — minimum local plays for an archetype to count as a real meta threat (default: 3)

### Steps:
1. **Load store meta**: Call `get_scoped_meta(store_ids=[X], since_date=Y)` to get the current local meta
2. **Resolve your decks**: For each archetype you provide, pull the best decklist from `deck_library.json` (same `resolve_base_deck()` logic used by architect training — prefers digilab > digimonmeta > egman source)
3. **Build opponent list**: From local meta, pick representative decklists from `deck_library.json` weighted by local meta share. Filter archetypes below `--min-plays`. Exclude your own candidate archetypes from the opponent pool to avoid mirror-match bias.
4. **Evaluate each of your archetypes**: Use `DeckSimulator.evaluate_deck()` against the store's opponent list. Also compute per-opponent matchup win rates for the breakdown.
5. **Compute ETWR per archetype**: Expected Tournament Win Rate weighted by local meta shares
6. **Print recommendation**: Ranked table of your archetypes with ETWR, plus per-matchup breakdown showing which opponents are good/bad
7. **Sleeper report**: Flag archetypes with high local conversion that have sufficient sample size (above median/2 plays)
8. **(Optional) Optimize**: If `--optimize`, run `MetaOptimizer` on the top-ranked archetype's deck to suggest card swaps for the store meta

### Output format:
```
=== Store Night: The Card Haven ===
  Meta based on 45 results since 2025-12-01
  Median archetype plays: 4

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
  ...

  SLEEPERS (conv > 50%, plays >= median/2):
  Dark Masters      3.9%    94.1%  100%   5  ⚠ High conversion + sufficient sample

  OPTIMIZATION (Rocks):
  Swap: -1 BT24-055 → +1 EX10-042  (WR: .621 → .638)
  Swap: -1 BT22-019 → +1 BT24-017  (WR: .638 → .645)
```

---

## 5. File changes summary

| File | Change |
|------|--------|
| `digimon_gym/digilab_client.py` | Add `since_date` param, `get_player_history()`, median/mean play stats |
| `tools/meta_loader.py` | Add `player_name` to `IngestedDeck`, capture from DigiLab + Egman, store in library |
| `tools/store_night.py` | **New file** — the store night recommender CLI |
| `local_meta_dfw.json` | Schema update: add median/mean stats (regenerated) |
| `tests/test_store_night.py` | **New file** — tests for the recommender |
| `tests/test_digilab_client.py` | Tests for new temporal/player queries |

---

## 6. Design decisions

- **No DB imports** — `store_night.py` is engine-only (safe for desktop sidecar)
- **Archetype-based input** — you provide archetype names, tool resolves best decklist from `deck_library.json` using source preference (digilab > digimonmeta > egman). Same `resolve_base_deck()` logic as architect training.
- **Player data is best-effort** — if DigiLab DB lacks player columns, we log a warning and skip; Egman player names are captured when available
- **Sleeper threshold** — configurable but defaults to `times_played >= max(3, median_plays / 2)` AND `conversion_rate > 50%`
- **Greedy-first evaluation** — defaults to greedy pilot for speed; ONNX pilot optional for accuracy
