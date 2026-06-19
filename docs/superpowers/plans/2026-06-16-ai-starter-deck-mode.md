# AI Starter Deck Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "AI Starter Deck" game mode where the player picks one of the 6 official starter decks (ST-1…ST-6) and plays a trained AI piloting a randomly chosen starter deck; ship the 6 starters with the app; allow the as-printed ST-2 deck (ST2-13 ×4) via a restriction-free `starter` format; and lay the released-model substrate with a greedy-CPU fallback.

**Architecture:** A single Python generator turns the 6 starter decklists in `data/deck_library.json` into (a) canonical `data/starter_decks.json` and (b) a typed `starterDecks.generated.ts` frontend module — one source of truth, bundled into both the Rust binary (`include_str!`) and the frontend bundle. A new restriction-free `starter` format in `data/deck_formats.json` makes the ST-2 deck legal. The frontend adds a 4th opponent tile (`ai_starter`) → a dedicated starter-deck picker → `createAiStarterGame`, which picks a random AI starter deck (seed-derived) and resolves the released model (desktop), falling back to the greedy CPU. The model substrate is a new top-level `starter_ai_model_id` pointer in the hosted manifest + a desktop resolver command. Built-in starter decks also surface (read-only) in the desktop deck library.

**Tech Stack:** Rust (digimon-engine, src-tauri/PyO3-free Tauri shell), React 18 + TypeScript + Vite + Zustand, Python (generator script + FastAPI hosted manifest), `cargo test` / `vitest` / `pytest`.

---

## File Structure

**New files**
- `code/tools/gen_starter_decks.py` — generator: `deck_library.json` + `cards.json` → `data/starter_decks.json` + `starterDecks.generated.ts`.
- `code/frontend/src/features/play/starterDecks.generated.ts` — generated `STARTER_DECKS: DeckResponse[]` (the picker + game-creation source, both builds).
- `code/digimon-engine/tests/starter_format.rs` — engine legality test (ST-2 passes under `starter`, fails under `standard`).
- `code/frontend/src/pages/StarterDeckSelectPage.tsx` — the AI-starter-mode deck picker page.
- `code/frontend/src/pages/StarterDeckSelectPage.test.tsx` — picker tests.
- `code/frontend/src/features/play/aiStarter.test.ts` — `createAiStarterGame` / seed-index tests.

**Modified files**
- `data/starter_decks.json` — regenerated to v2 (all 6 decks, egg/main split, `game_mode: "starter"`).
- `data/deck_formats.json` — add the `starter` format descriptor.
- `code/src-tauri/src/models.rs` — parse `starter_ai_model_id`; add `models_resolve_starter` command.
- `code/src-tauri/src/deck_storage.rs` — `is_builtin` field; `builtin_starter_decks()`; merge into list/get; guard mutations; `rust_list_starter_decks` command.
- `code/src-tauri/src/main.rs` — register `models_resolve_starter` + `rust_list_starter_decks`.
- `code/server/db/schemas.py` — add `starter_ai_model_id` to `ManifestResponse`.
- `code/server/db/routers/admin_models.py` — populate `starter_ai_model_id` from env.
- `code/frontend/src/features/play/formatCatalog.ts` — `'starter'` PlayFormatId + presentation; `'ai_starter'` OpponentMode.
- `code/frontend/src/types/deck.ts` — `is_builtin?: boolean` on `DeckSummary` + `DeckResponse`.
- `code/frontend/src/api/desktopModelsApi.ts` — `resolveStarterModel`.
- `code/frontend/src/features/play/playApi.ts` — `listStarterDecks`, `createAiStarterGame`, seed-index util.
- `code/frontend/src/pages/ModeSelectPage.tsx` — 4th tile, hide format grid for `ai_starter`, filter `starter`, routing.
- `code/frontend/src/App.tsx` — `/play/ai-starter` route.
- `code/frontend/src/pages/DeckLibraryPage.tsx` — built-in lock affordance (disable delete for `is_builtin`).

**Build/verify commands** (run from repo root unless noted)
- Engine tests: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test starter_format`
- Tauri tests: `cargo test --manifest-path code/src-tauri/Cargo.toml`
- Frontend tests: `cd code/frontend && npm test -- <file>`
- Python tests: `python -m pytest code/tests/api -k manifest -v`

> **Per-worktree Rust target** (memory `reference_cargo_target_per_worktree`): if a `cargo` command surfaces a compile error in a file you did NOT edit, prefix the command with `CARGO_TARGET_DIR='D:\cargo-target-wt\brave-hopper-7ca8dc'` to build in an isolated target dir (shared-target contamination), then re-run.

---

## Phase 0 — Data + generator

### Task 0.1: Starter-deck generator + regenerated data

**Files:**
- Create: `code/tools/gen_starter_decks.py`
- Modify (generated): `data/starter_decks.json`
- Create (generated): `code/frontend/src/features/play/starterDecks.generated.ts`

- [ ] **Step 1: Write the generator**

Create `code/tools/gen_starter_decks.py`:

```python
#!/usr/bin/env python3
"""Generate the bundled 6 starter decks from the deck library.

Reads the 6 `starter_st*` decklists in `data/deck_library.json`, splits each
into egg (card_kind == 3) and main using `data/cards.json`, and emits:
  * data/starter_decks.json                                  (canonical; Rust include_str!)
  * code/frontend/src/features/play/starterDecks.generated.ts (frontend picker source)

Run from anywhere:  python code/tools/gen_starter_decks.py
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]  # tools -> code -> repo root
DATA = ROOT / "data"

# (deck_id in deck_library.json) -> (set id, display name)
DECKS = [
    ("starter_st1_gaia_red", "ST1", "Starter Deck Gaia Red"),
    ("starter_st2_cocytus_blue", "ST2", "Starter Deck Cocytus Blue"),
    ("starter_st3_heavens_yellow", "ST3", "Starter Deck Heaven's Yellow"),
    ("starter_st4_giga_green", "ST4", "Starter Deck Giga Green"),
    ("starter_st5_machine_black", "ST5", "Starter Deck Machine Black"),
    ("starter_st6_venomous_violet", "ST6", "Starter Deck Venomous Violet"),
]
EGG_KIND = 3  # cards.json card_kind: 0 digimon, 1 tamer, 2 option, 3 digi-egg


def _walk(obj):
    if isinstance(obj, dict):
        if "deck_id" in obj and "decklist" in obj:
            yield obj
        for v in obj.values():
            yield from _walk(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from _walk(v)


def main() -> None:
    lib = json.loads((DATA / "deck_library.json").read_text(encoding="utf-8"))
    cards = json.loads((DATA / "cards.json").read_text(encoding="utf-8"))

    found = {}
    wanted = {d[0] for d in DECKS}
    for entry in _walk(lib):
        did = entry["deck_id"]
        if did in wanted and did not in found:
            dl = entry["decklist"]
            found[did] = json.loads(dl) if isinstance(dl, str) else list(dl)

    decks = []
    for did, set_id, name in DECKS:
        if did not in found:
            raise SystemExit(f"starter deck {did!r} not found in deck_library.json")
        dl = found[did]
        egg = [c for c in dl if cards.get(c, {}).get("card_kind") == EGG_KIND]
        main = [c for c in dl if cards.get(c, {}).get("card_kind") != EGG_KIND]
        if len(main) != 50:
            raise SystemExit(f"{did}: expected 50 main cards, got {len(main)}")
        if len(egg) > 5:
            raise SystemExit(f"{did}: egg deck {len(egg)} exceeds 5")
        decks.append(
            {
                "id": did,
                "name": name,
                "set": set_id,
                "game_mode": "starter",
                "egg_deck": egg,
                "main_deck": main,
            }
        )

    out = {
        "version": 2,
        "_generated_by": "code/tools/gen_starter_decks.py",
        "starter_decks": decks,
    }
    (DATA / "starter_decks.json").write_text(
        json.dumps(out, indent=2) + "\n", encoding="utf-8"
    )

    def ts_deck(d):
        return (
            "  {\n"
            f"    id: {json.dumps(d['id'])},\n"
            "    owner_id: 'builtin',\n"
            "    folder_id: null,\n"
            f"    name: {json.dumps(d['name'])},\n"
            f"    description: {json.dumps('Official ' + d['set'] + ' starter deck.')},\n"
            "    game_mode: 'starter',\n"
            f"    main_deck: {json.dumps(d['main_deck'])},\n"
            f"    egg_deck: {json.dumps(d['egg_deck'])},\n"
            "    main_deck_alt_arts: [],\n"
            "    egg_deck_alt_arts: [],\n"
            "    commander_id: null,\n"
            "    is_valid: true,\n"
            "    validation_errors: [],\n"
            "    is_public: false,\n"
            "    is_pinned: false,\n"
            "    tags: ['starter'],\n"
            "    meta_tier: null,\n"
            f"    meta_archetype: {json.dumps(d['set'])},\n"
            "    is_builtin: true,\n"
            "    created_at: '2024-01-01T00:00:00Z',\n"
            "    updated_at: '2024-01-01T00:00:00Z',\n"
            "  }"
        )

    ts = (
        "// @generated by code/tools/gen_starter_decks.py — DO NOT EDIT.\n"
        "import type { DeckResponse } from '@/types/deck';\n\n"
        "export const STARTER_DECKS: DeckResponse[] = [\n"
        + ",\n".join(ts_deck(d) for d in decks)
        + "\n];\n"
    )
    (ROOT / "code/frontend/src/features/play/starterDecks.generated.ts").write_text(
        ts, encoding="utf-8"
    )
    print(f"wrote {len(decks)} starter decks")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the generator**

Run: `python code/tools/gen_starter_decks.py`
Expected: prints `wrote 6 starter decks`; `data/starter_decks.json` now has `"version": 2` and a 6-element `starter_decks` array; `code/frontend/src/features/play/starterDecks.generated.ts` exists.

- [ ] **Step 3: Sanity-check the output**

Run: `python -c "import json; d=json.load(open('data/starter_decks.json',encoding='utf-8')); print(d['version'], len(d['starter_decks']), [ (x['id'], len(x['main_deck']), len(x['egg_deck'])) for x in d['starter_decks'] ])"`
Expected: `2 6 [('starter_st1_gaia_red', 50, 4), ('starter_st2_cocytus_blue', 50, 4), ... ]` (every deck 50 main / 4 egg).

- [ ] **Step 4: Commit**

```bash
git add code/tools/gen_starter_decks.py data/starter_decks.json code/frontend/src/features/play/starterDecks.generated.ts
git commit -m "feat(starter): generator + bundled 6 starter decks (data + TS)"
```

### Task 0.2: Add the restriction-free `starter` format

**Files:**
- Modify: `data/deck_formats.json` (the `formats` array, after `eden_singleton`)

- [ ] **Step 1: Add the format descriptor**

In `data/deck_formats.json`, inside the `"formats"` array, add a new object after the `eden_singleton` entry (insert a comma after the `eden_singleton` closing `}`):

```json
    {
      "id": "starter",
      "name": "Starter",
      "description": "The six official starter decks (ST-1..ST-6), played with no banlist so the as-printed ST-2 deck (ST2-13 x4) is legal.",
      "deck_size": 50,
      "egg_max": 5,
      "rarity_policy": "all",
      "banlist": null,
      "singleton": false,
      "default_max_copies": 4,
      "playable": false
    }
```

- [ ] **Step 2: Verify the JSON parses and the engine registry loads it**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml format:: -- --nocapture` (the `format` module's existing registry tests build the registry, which will panic if the new entry is malformed or duplicates an id).
Expected: PASS (registry builds; no `duplicate format id` / `malformed` panic).

- [ ] **Step 3: Commit**

```bash
git add data/deck_formats.json
git commit -m "feat(starter): add restriction-free 'starter' deck format"
```

---

## Phase 1 — Engine: starter-format legality test

### Task 1.1: Prove ST-2 is legal under `starter`, illegal under `standard`

**Files:**
- Create: `code/digimon-engine/tests/starter_format.rs`
- Possibly modify: `code/digimon-engine/Cargo.toml` (ensure `serde_json` dev-dependency)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/starter_format.rs`:

```rust
//! The `starter` format must accept the as-printed ST-2 starter deck
//! (ST2-13 x4), which the `standard` banlist Limits to 1 copy.

use digimon_engine::deck_tools::validate_deck_for_game_mode;

const STARTER_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/starter_decks.json"));

fn st2_cards() -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(STARTER_JSON).expect("parse starter_decks.json");
    let decks = v["starter_decks"].as_array().expect("starter_decks array");
    let st2 = decks
        .iter()
        .find(|d| d["id"] == "starter_st2_cocytus_blue")
        .expect("ST-2 deck present");
    let mut cards: Vec<String> = Vec::new();
    for key in ["main_deck", "egg_deck"] {
        for c in st2[key].as_array().expect("deck array") {
            cards.push(c.as_str().expect("card id string").to_string());
        }
    }
    cards
}

#[test]
fn st2_starter_deck_is_legal_in_starter_format() {
    let cards = st2_cards();
    let res = validate_deck_for_game_mode(&cards, "starter").expect("starter format exists");
    assert!(
        res.is_valid,
        "ST-2 starter deck should be legal in the starter format; errors: {:?}",
        res.errors
    );
}

#[test]
fn st2_starter_deck_is_illegal_in_standard_due_to_limited_card() {
    let cards = st2_cards();
    let res = validate_deck_for_game_mode(&cards, "standard").expect("standard format exists");
    assert!(
        !res.is_valid,
        "ST-2 deck runs ST2-13 x4, over the standard Limited cap of 1"
    );
    assert!(
        res.errors.iter().any(|e| e.contains("ST2-13")),
        "expected a limit error mentioning ST2-13; got {:?}",
        res.errors
    );
}
```

- [ ] **Step 2: Run the test to verify it passes** (the format + data already landed in Phase 0, so this should pass immediately; if `serde_json` is not a dev-dependency the test will fail to compile — see Step 3)

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test starter_format`
Expected: 2 passed. If it fails to **compile** with `unresolved import serde_json` or `use of undeclared crate serde_json`, do Step 3 then re-run.

- [ ] **Step 3 (only if Step 2 failed to compile): add the dev-dependency**

In `code/digimon-engine/Cargo.toml`, under `[dev-dependencies]`, ensure:
```toml
serde_json = "1"
```
Re-run Step 2. Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/tests/starter_format.rs code/digimon-engine/Cargo.toml
git commit -m "test(engine): ST-2 starter deck legal under starter, banned under standard"
```

---

## Phase 2 — Frontend types + format catalog

### Task 2.1: Add `starter` PlayFormatId, `ai_starter` OpponentMode, and `is_builtin` deck flag

**Files:**
- Modify: `code/frontend/src/features/play/formatCatalog.ts:8-18,36-46`
- Modify: `code/frontend/src/types/deck.ts:31-75`

- [ ] **Step 1: Extend the format + opponent types**

In `code/frontend/src/features/play/formatCatalog.ts`, change the `PlayFormatId` union (lines 8-17) to add `'starter'` and the `OpponentMode` (line 18) to add `'ai_starter'`:

```typescript
export type PlayFormatId =
  | 'standard'
  | 'no_restriction'
  | 'pauper'
  | 'eden'
  | 'eden_singleton'
  | 'starter'
  | 'titan'
  | 'edh_commander'
  | 'draft'
  | 'tutorial';
export type OpponentMode = 'quick' | 'room' | 'bot' | 'ai_starter';
```

Then in the `PRESENTATION` map (lines 36-46) add a `starter` entry (keeps the `Record<PlayFormatId, …>` exhaustive):

```typescript
  eden_singleton: { tagline: 'EDEN, highlander', populationPct: 12 },
  starter: { tagline: 'Six official starter decks', populationPct: 0 },
```

- [ ] **Step 2: Add `is_builtin` to the deck types**

In `code/frontend/src/types/deck.ts`, add `is_builtin?: boolean;` to both `DeckSummary` (after line 38 `is_pinned: boolean;`) and `DeckResponse` (after the `is_pinned: boolean;` line ~66):

```typescript
  is_pinned: boolean;
  is_builtin?: boolean;
```

(Apply to BOTH interfaces.)

- [ ] **Step 3: Typecheck**

Run: `cd code/frontend && npx tsc --noEmit`
Expected: no errors (the generated `starterDecks.generated.ts` already uses `is_builtin: true`, which now typechecks).

- [ ] **Step 4: Commit**

```bash
git add code/frontend/src/features/play/formatCatalog.ts code/frontend/src/types/deck.ts
git commit -m "feat(starter): add starter format id, ai_starter mode, is_builtin deck flag"
```

---

## Phase 3 — Frontend: AI-starter game creation logic

### Task 3.1: `listStarterDecks`, seed-index, and `createAiStarterGame` (greedy fallback first)

**Files:**
- Modify: `code/frontend/src/features/play/playApi.ts` (imports + new exports at end)
- Test: `code/frontend/src/features/play/aiStarter.test.ts`

- [ ] **Step 1: Write the failing test**

Create `code/frontend/src/features/play/aiStarter.test.ts`:

```typescript
import { describe, expect, it, vi, beforeEach } from 'vitest';

// Mock the gameApi + http client so we observe what createAiStarterGame sends.
const createGame = vi.fn(async () => ({ game_id: 'g1', seed: '7' }));
vi.mock('@/api/gameApi', () => ({
  createGame: (...args: unknown[]) => createGame(...args),
  normalizeSeedInput: (s: string | null) => s,
}));
vi.mock('@/api/client', () => ({ default: { post: vi.fn() } }));
// Force the Tauri-desktop path.
vi.stubGlobal('isTauri', true);

import { starterIndexFromSeed, createAiStarterGame } from './playApi';
import { STARTER_DECKS } from './starterDecks.generated';

beforeEach(() => createGame.mockClear());

describe('starterIndexFromSeed', () => {
  it('is deterministic for a given seed', () => {
    expect(starterIndexFromSeed('42', 6)).toBe(starterIndexFromSeed('42', 6));
  });
  it('stays within range', () => {
    for (const s of ['', '1', 'abc', '999999']) {
      const i = starterIndexFromSeed(s || null, 6);
      expect(i).toBeGreaterThanOrEqual(0);
      expect(i).toBeLessThan(6);
    }
  });
});

describe('createAiStarterGame', () => {
  it('sends player + a starter AI deck and falls back to greedy when no model', async () => {
    const res = await createAiStarterGame({
      deck: STARTER_DECKS[0],
      starterDecks: STARTER_DECKS,
      seed: '42',
    });
    expect(res.game_id).toBe('g1');
    expect(createGame).toHaveBeenCalledTimes(1);
    const arg = createGame.mock.calls[0][0] as {
      deck1: string[];
      deck2: string[];
      player_kinds: string[];
      player_model_ids: (string | null)[];
    };
    // Player 1 is the chosen deck; player 2 is a (seed-derived) starter deck.
    expect(arg.deck1.length).toBe(54);
    expect(arg.deck2.length).toBe(54);
    // No model published in tests -> greedy CPU.
    expect(arg.player_kinds).toEqual(['human', 'greedy']);
    expect(arg.player_model_ids).toEqual([null, null]);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd code/frontend && npm test -- src/features/play/aiStarter.test.ts`
Expected: FAIL — `starterIndexFromSeed`/`createAiStarterGame` are not exported from `playApi`.

- [ ] **Step 3: Implement in `playApi.ts`**

At the top of `code/frontend/src/features/play/playApi.ts`, add imports (after the existing imports):

```typescript
import type { PlayerKind } from '@/api/gameApi';
import { resolveStarterModel } from '@/api/desktopModelsApi';
import { STARTER_DECKS } from './starterDecks.generated';

const MANIFEST_BASE = (import.meta.env.VITE_MODELS_MANIFEST_URL as string | undefined) ?? '';
```

> `PlayerKind` is already exported from `@/api/gameApi` (`gameApi.ts:112` — `export type PlayerKind = 'human' | 'greedy' | 'trained';`), so the import resolves as-is; no change to `gameApi.ts`.

At the END of `playApi.ts`, append:

```typescript
/** The 6 bundled starter decks (same data in desktop + browser builds). */
export async function listStarterDecks(): Promise<DeckResponse[]> {
  return STARTER_DECKS;
}

/** Deterministic-from-seed index in [0, count). FNV-1a so a given shuffle
 *  seed reproduces the same AI deck; random when no seed is set. */
export function starterIndexFromSeed(seed: string | null, count: number): number {
  if (!seed) return Math.floor(Math.random() * count);
  let h = 2166136261;
  for (let i = 0; i < seed.length; i += 1) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h) % count;
}

/** Start a game vs the AI: player pilots `deck`, the AI pilots a random
 *  starter deck (seed-derived). Uses the released starter model on desktop
 *  when one is published; otherwise the greedy CPU. */
export async function createAiStarterGame(params: {
  deck: DeckResponse;
  starterDecks: DeckResponse[];
  seed?: string | null;
}): Promise<{ game_id: string; seed?: string; aiDeckName: string }> {
  const seed = params.seed ?? null;
  const aiDeck = params.starterDecks[starterIndexFromSeed(seed, params.starterDecks.length)];
  const deck1 = [...params.deck.egg_deck, ...params.deck.main_deck];
  const deck2 = [...aiDeck.egg_deck, ...aiDeck.main_deck];

  // Resolve the released model (desktop only); any failure -> greedy CPU.
  let modelId: string | null = null;
  if (IS_DESKTOP && MANIFEST_BASE) {
    try {
      modelId = await resolveStarterModel(MANIFEST_BASE);
    } catch {
      modelId = null;
    }
  }

  if (!hasTauriBridge()) {
    const { data } = await client.post<{ game_id: string; seed?: string }>('/games', {
      deck1,
      deck2,
      player1_type: 'human',
      player2_type: 'agent',
      player2_policy: 'greedy',
      seed,
    });
    return { game_id: data.game_id, seed: data.seed, aiDeckName: aiDeck.name };
  }

  const kinds: PlayerKind[] = ['human', modelId ? 'trained' : 'greedy'];
  const response = await gameApi.createGame({
    deck1,
    deck2,
    player_kinds: kinds,
    player_model_ids: [null, modelId],
    seed,
  });
  return { game_id: response.game_id, seed: response.seed, aiDeckName: aiDeck.name };
}
```

> `resolveStarterModel` is added in Task 3.2. Implement that first if your runner builds eagerly; the test mocks the desktop path off (`IS_DESKTOP` is false under `vitest` since `VITE_BUILD_TARGET` is unset), so `resolveStarterModel` is never called in the test.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd code/frontend && npm test -- src/features/play/aiStarter.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add code/frontend/src/features/play/playApi.ts code/frontend/src/features/play/aiStarter.test.ts
git commit -m "feat(starter): createAiStarterGame (random AI deck, greedy fallback) + seed index"
```

### Task 3.2: Desktop model-resolver wrapper

**Files:**
- Modify: `code/frontend/src/api/desktopModelsApi.ts` (append after `loadCached`)

- [ ] **Step 1: Add the wrapper**

Append to `code/frontend/src/api/desktopModelsApi.ts`:

```typescript
/// Resolve + load the released "starter AI" model flagged in the hosted
/// manifest (`starter_ai_model_id`). Returns the loaded model id, or `null`
/// when no model is published / it's incompatible / the fetch fails — callers
/// fall back to the greedy CPU.
export async function resolveStarterModel(baseUrl: string): Promise<string | null> {
  return invoke<string | null>('models_resolve_starter', { baseUrl });
}
```

- [ ] **Step 2: Typecheck**

Run: `cd code/frontend && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add code/frontend/src/api/desktopModelsApi.ts
git commit -m "feat(starter): desktop resolveStarterModel invoke wrapper"
```

---

## Phase 4 — Frontend: starter-deck picker page

### Task 4.1: `StarterDeckSelectPage` + route

**Files:**
- Create: `code/frontend/src/pages/StarterDeckSelectPage.tsx`
- Create: `code/frontend/src/pages/StarterDeckSelectPage.test.tsx`
- Modify: `code/frontend/src/App.tsx:13-16,79-84`

- [ ] **Step 1: Write the failing test**

Create `code/frontend/src/pages/StarterDeckSelectPage.test.tsx`:

```typescript
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

const createAiStarterGame = vi.fn(async () => ({ game_id: 'g99', seed: null, aiDeckName: 'X' }));
vi.mock('@/features/play/playApi', async () => {
  const { STARTER_DECKS } = await vi.importActual<typeof import('@/features/play/starterDecks.generated')>(
    '@/features/play/starterDecks.generated',
  );
  return {
    listStarterDecks: async () => STARTER_DECKS,
    createAiStarterGame: (...a: unknown[]) => createAiStarterGame(...a),
  };
});
vi.mock('@/api/gameApi', () => ({ normalizeSeedInput: (s: string | null) => s }));

import { StarterDeckSelectPage } from './StarterDeckSelectPage';

beforeEach(() => {
  navigate.mockClear();
  createAiStarterGame.mockClear();
});

describe('StarterDeckSelectPage', () => {
  it('lists the 6 starter decks', async () => {
    render(
      <MemoryRouter>
        <StarterDeckSelectPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('Starter Deck Gaia Red')).toBeInTheDocument());
    expect(screen.getByText('Starter Deck Cocytus Blue')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /Starter Deck/ })).toHaveLength(6);
  });

  it('launches a game with the selected deck', async () => {
    render(
      <MemoryRouter>
        <StarterDeckSelectPage />
      </MemoryRouter>,
    );
    await waitFor(() => screen.getByText('Starter Deck Cocytus Blue'));
    fireEvent.click(screen.getByRole('button', { name: /Cocytus Blue/ }));
    fireEvent.click(screen.getByRole('button', { name: /FACE THE AI/i }));
    await waitFor(() => expect(createAiStarterGame).toHaveBeenCalledTimes(1));
    const arg = createAiStarterGame.mock.calls[0][0] as { deck: { set?: string } };
    expect((createAiStarterGame.mock.calls[0][0] as { deck: { name: string } }).deck.name).toBe(
      'Starter Deck Cocytus Blue',
    );
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/game/g99'));
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd code/frontend && npm test -- src/pages/StarterDeckSelectPage.test.tsx`
Expected: FAIL — module `./StarterDeckSelectPage` does not exist.

- [ ] **Step 3: Implement the page**

Create `code/frontend/src/pages/StarterDeckSelectPage.tsx`:

```typescript
import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { normalizeSeedInput } from '@/api/gameApi';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { createAiStarterGame, listStarterDecks } from '@/features/play/playApi';
import type { DeckResponse } from '@/types/deck';
import { getCardImageUrl } from '@/utils/cardImages';
import './DeckSelectPage.css';

export function StarterDeckSelectPage() {
  const navigate = useNavigate();
  const [decks, setDecks] = useState<DeckResponse[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [seedInput, setSeedInput] = useState('');
  const [seedError, setSeedError] = useState('');
  const [launching, setLaunching] = useState(false);

  useEffect(() => {
    listStarterDecks()
      .then((items) => {
        setDecks(items);
        setSelectedId(items[0]?.id ?? null);
      })
      .catch(() => setDecks([]));
  }, []);

  const selected = useMemo(
    () => decks.find((deck) => deck.id === selectedId) ?? decks[0] ?? null,
    [decks, selectedId],
  );

  const handleConfirm = async () => {
    if (!selected || launching) return;
    let normalizedSeed: string | null = null;
    try {
      normalizedSeed = normalizeSeedInput(seedInput);
      setSeedError('');
    } catch (err) {
      setSeedError((err as Error).message);
      return;
    }
    setLaunching(true);
    try {
      const response = await createAiStarterGame({
        deck: selected,
        starterDecks: decks,
        seed: normalizedSeed,
      });
      navigate(`/game/${response.game_id}`);
    } finally {
      setLaunching(false);
    }
  };

  return (
    <InBetweenShell
      title="CHOOSE STARTER"
      stepLabel="02"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'AI STARTER DECK' }]}
      rightSlot={<span>STARTER - vs AI</span>}
    >
      <main className="deck-select-main">
        <section className="deck-select-banner">
          <div>
            <span className="label">MODE //</span>
            <h1>AI STARTER DECK</h1>
            <p>Pick a starter deck. The AI plays a random one of the six.</p>
          </div>
          <Link to="/play">CHANGE</Link>
        </section>

        <section className="deck-select-grid">
          {decks.map((deck) => (
            <button
              key={deck.id}
              type="button"
              aria-label={deck.name}
              className={`deck-select-card ${deck.id === selected?.id ? 'selected' : ''}`}
              onClick={() => setSelectedId(deck.id)}
            >
              <span className="glyph">
                <img
                  src={getCardImageUrl(deck.egg_deck[0] ?? deck.main_deck[0])}
                  alt=""
                  loading="lazy"
                  draggable={false}
                  onError={(event) => {
                    event.currentTarget.style.display = 'none';
                  }}
                />
              </span>
              <span className="name">{deck.name}</span>
              <span className="meta">
                {deck.main_deck.length}/{deck.egg_deck.length} - {deck.meta_archetype ?? 'Starter'}
              </span>
              <span className="legal">READY</span>
            </button>
          ))}
        </section>

        <div className="deck-confirm-bar">
          <div className="deck-confirm-info">
            <span>{selected ? selected.name : 'NO DECK SELECTED'}</span>
            <label className="deck-seed-control">
              <span>SHUFFLE SEED</span>
              <input
                value={seedInput}
                onChange={(event) => {
                  setSeedInput(event.target.value);
                  setSeedError('');
                }}
                placeholder="Random"
                inputMode="numeric"
              />
              {seedError && <em>{seedError}</em>}
            </label>
          </div>
          <button type="button" disabled={!selected || launching} onClick={handleConfirm}>
            {launching ? 'LAUNCHING...' : 'FACE THE AI'}
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd code/frontend && npm test -- src/pages/StarterDeckSelectPage.test.tsx`
Expected: PASS (2 tests).

- [ ] **Step 5: Register the route**

In `code/frontend/src/App.tsx`, add the import next to the other page imports (after line 13 `DeckSelectPage`):

```typescript
import { StarterDeckSelectPage } from '@/pages/StarterDeckSelectPage';
```

And add the route inside the `AuthGuard` block, right after the `/play/deck` route (line 80):

```typescript
              <Route path="/play/ai-starter" element={<StarterDeckSelectPage />} />
```

- [ ] **Step 6: Typecheck + commit**

Run: `cd code/frontend && npx tsc --noEmit`
Expected: no errors.

```bash
git add code/frontend/src/pages/StarterDeckSelectPage.tsx code/frontend/src/pages/StarterDeckSelectPage.test.tsx code/frontend/src/App.tsx
git commit -m "feat(starter): StarterDeckSelectPage + /play/ai-starter route"
```

---

## Phase 5 — Frontend: the mode tile

### Task 5.1: Add the `ai_starter` tile, hide the format grid, route to the picker

**Files:**
- Modify: `code/frontend/src/pages/ModeSelectPage.tsx:9-13,21,65-99`
- Test: `code/frontend/src/pages/ModeSelectPage.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `code/frontend/src/pages/ModeSelectPage.test.tsx`:

```typescript
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});
vi.mock('@/features/play/playApi', () => ({ listFormats: async () => [] }));

import { ModeSelectPage } from './ModeSelectPage';

beforeEach(() => navigate.mockClear());

describe('ModeSelectPage', () => {
  it('offers an AI Starter Deck tile and routes it to the starter picker', async () => {
    render(
      <MemoryRouter>
        <ModeSelectPage />
      </MemoryRouter>,
    );
    const tile = await screen.findByRole('button', { name: /AI STARTER DECK/i });
    fireEvent.click(tile);
    // Format grid hidden in AI-starter mode.
    expect(screen.queryByRole('region', { name: 'Formats' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /ENTER/i }));
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/play/ai-starter'));
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd code/frontend && npm test -- src/pages/ModeSelectPage.test.tsx`
Expected: FAIL — no "AI STARTER DECK" tile.

- [ ] **Step 3: Add the tile + grid-hide + routing**

In `code/frontend/src/pages/ModeSelectPage.tsx`:

(a) Add the tile to the `OPPONENTS` array (after the `bot` entry, line 12):
```typescript
  { id: 'bot', name: 'BOT MATCH', sub: 'CPU practice', meta: 'LOCAL ENGINE' },
  { id: 'ai_starter', name: 'AI STARTER DECK', sub: 'Pick a starter, face the AI', meta: 'AI OPPONENT' },
```

(b) Filter the internal `starter` format out of the grid (line 21):
```typescript
  const visibleFormats = (formats.length > 0 ? formats : [fallback]).filter(
    (format) => format.id !== 'starter',
  );
```

(c) Hide the format grid in AI-starter mode — wrap the `<section className="mode-grid" …>` block (lines 65-86) in a conditional:
```tsx
        {opponentMode !== 'ai_starter' && (
          <section className="mode-grid" aria-label="Formats">
            {visibleFormats.map((format, index) => (
              {/* …unchanged contents… */}
            ))}
          </section>
        )}
```

(d) Update the action-bar label + routing (lines 88-99):
```tsx
        <div className="mode-action-bar">
          <span>
            {opponentMode === 'ai_starter' ? 'STARTER DECKS' : selected.name} /{' '}
            {opponentMode.toUpperCase()}
          </span>
          <button
            type="button"
            onClick={() =>
              navigate(
                opponentMode === 'room'
                  ? '/play/room'
                  : opponentMode === 'ai_starter'
                    ? '/play/ai-starter'
                    : '/play/deck',
              )
            }
            disabled={opponentMode !== 'ai_starter' && !selected.enabled}
          >
            ENTER FORMAT
          </button>
        </div>
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd code/frontend && npm test -- src/pages/ModeSelectPage.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/frontend/src/pages/ModeSelectPage.tsx code/frontend/src/pages/ModeSelectPage.test.tsx
git commit -m "feat(starter): AI Starter Deck mode tile + routing + format-grid hide"
```

---

## Phase 6 — Model substrate backend

### Task 6.1: Rust manifest `starter_ai_model_id` + `models_resolve_starter`

**Files:**
- Modify: `code/src-tauri/src/models.rs:57-61,201-209` + new command + tests
- Modify: `code/src-tauri/src/main.rs:104-108`

- [ ] **Step 1: Write the failing parse test**

In `code/src-tauri/src/models.rs`, inside the existing `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn manifest_response_parses_starter_pointer_and_defaults_none() {
        let with = serde_json::json!({
            "generated_at": "2026-06-16T00:00:00Z",
            "models": [],
            "starter_ai_model_id": "abc-123"
        });
        let parsed: ManifestResponse = serde_json::from_value(with).unwrap();
        assert_eq!(parsed.starter_ai_model_id.as_deref(), Some("abc-123"));

        // Missing field defaults to None (older servers).
        let without = serde_json::json!({ "generated_at": "x", "models": [] });
        let parsed: ManifestResponse = serde_json::from_value(without).unwrap();
        assert_eq!(parsed.starter_ai_model_id, None);
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml manifest_response_parses_starter_pointer`
Expected: FAIL to compile — `ManifestResponse` has no field `starter_ai_model_id`.

- [ ] **Step 3: Add the field + resolver**

In `code/src-tauri/src/models.rs`, extend `ManifestResponse` (lines 57-61):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestResponse {
    generated_at: Option<String>,
    models: Vec<ManifestModel>,
    #[serde(default)]
    starter_ai_model_id: Option<String>,
}
```

Replace `fetch_manifest` (lines 201-209) with a full + thin pair:

```rust
    pub async fn fetch_manifest_full(
        &self,
        base_url: &str,
    ) -> Result<(Vec<ManifestModel>, Option<String>), ModelManagerError> {
        let url = format!("{}/models/manifest.json", base_url.trim_end_matches('/'));
        let resp = self.http.get(&url).send().await?.error_for_status()?;
        let parsed: ManifestResponse = resp.json().await?;
        Ok((parsed.models, parsed.starter_ai_model_id))
    }

    pub async fn fetch_manifest(
        &self,
        base_url: &str,
    ) -> Result<Vec<ManifestModel>, ModelManagerError> {
        Ok(self.fetch_manifest_full(base_url).await?.0)
    }
```

Add a new Tauri command in the `// ─── Tauri commands ───` section (after `models_load_cached`, ~line 425):

```rust
/// Resolve the released "starter AI" model the hosted manifest points at via
/// `starter_ai_model_id`: download it (if not cached) and load it into the
/// inference cache, returning its id. Returns `Ok(None)` — so the caller falls
/// back to the greedy CPU — when there's no pointer, the model is missing /
/// incompatible, or the manifest can't be fetched (offline).
#[tauri::command]
pub async fn models_resolve_starter(
    manager: tauri::State<'_, Arc<ModelsManager>>,
    engine: tauri::State<'_, EngineHandle>,
    base_url: String,
) -> Result<Option<String>, String> {
    let (models, starter_id) = match manager.fetch_manifest_full(&base_url).await {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let Some(starter_id) = starter_id else {
        return Ok(None);
    };
    let Some(model) = models.into_iter().find(|m| m.id == starter_id) else {
        return Ok(None);
    };
    // Shape gate: skip incompatible models rather than erroring the game start.
    if model.tensor_size != TENSOR_SIZE || model.action_space_size != ACTION_SPACE_SIZE {
        return Ok(None);
    }
    // Download if not already cached.
    if manager.local_meta(&model.id).map_err(|e| e.to_string())?.is_none() {
        if manager.download(&model).await.is_err() {
            return Ok(None);
        }
    }
    // Load into the inference cache on the engine worker (owns the session).
    let manager_arc: Arc<ModelsManager> = Arc::clone(&manager);
    let id = model.id.clone();
    let load: Result<(), String> = engine
        .run(move |world| -> Result<(), String> {
            manager_arc
                .load_cached(&id, &world.inference)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await?;
    match load {
        Ok(()) => Ok(Some(model.id)),
        Err(_) => Ok(None),
    }
}
```

- [ ] **Step 4: Run the parse test to verify it passes**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml manifest_response_parses_starter_pointer`
Expected: PASS. Also run the full models test module: `cargo test --manifest-path code/src-tauri/Cargo.toml models::` → all PASS (existing `manifest_response_parses_backend_schema` still passes — the new field is `#[serde(default)]`).

- [ ] **Step 5: Register the command**

In `code/src-tauri/src/main.rs`, add to the `tauri::generate_handler!` list (after `models::models_load_cached,` line 108):

```rust
            models::models_resolve_starter,
```

- [ ] **Step 6: Build + commit**

Run: `cargo build --manifest-path code/src-tauri/Cargo.toml`
Expected: builds clean.

```bash
git add code/src-tauri/src/models.rs code/src-tauri/src/main.rs
git commit -m "feat(starter): manifest starter_ai_model_id + models_resolve_starter command"
```

### Task 6.2: Hosted manifest exposes `starter_ai_model_id`

**Files:**
- Modify: `code/server/db/schemas.py:1062-1064`
- Modify: `code/server/db/routers/admin_models.py:387-390` (+ `import os`)
- Test: `code/tests/api/test_manifest_starter_pointer.py`

- [ ] **Step 1: Write the failing test**

Create `code/tests/api/test_manifest_starter_pointer.py`:

```python
"""The public manifest exposes an optional starter_ai_model_id pointer,
sourced from the STARTER_AI_MODEL_ID env var, so the desktop app knows which
model the AI-Starter mode should play. Defaults to None when unset."""
from server.db.schemas import ManifestResponse


def test_manifest_response_has_optional_starter_pointer():
    m = ManifestResponse(generated_at="2026-06-16T00:00:00Z", models=[])
    assert m.starter_ai_model_id is None

    m2 = ManifestResponse(
        generated_at="2026-06-16T00:00:00Z", models=[], starter_ai_model_id="abc-123"
    )
    assert m2.starter_ai_model_id == "abc-123"
```

- [ ] **Step 2: Run it to verify it fails**

Run: `python -m pytest code/tests/api/test_manifest_starter_pointer.py -v`
Expected: FAIL — `ManifestResponse` rejects/has no `starter_ai_model_id`.

- [ ] **Step 3: Add the schema field**

In `code/server/db/schemas.py`, extend `ManifestResponse` (lines 1062-1064):

```python
class ManifestResponse(BaseModel):
    generated_at: datetime
    models: List[ManifestModel]
    starter_ai_model_id: Optional[str] = None
```

- [ ] **Step 4: Populate it from env in the endpoint**

In `code/server/db/routers/admin_models.py`, ensure `import os` is present near the top, then update the `get_manifest` return (lines 387-390):

```python
    return ManifestResponse(
        generated_at=_utcnow(),
        models=manifest_models,
        starter_ai_model_id=os.environ.get("STARTER_AI_MODEL_ID") or None,
    )
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `python -m pytest code/tests/api/test_manifest_starter_pointer.py -v`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/server/db/schemas.py code/server/db/routers/admin_models.py code/tests/api/test_manifest_starter_pointer.py
git commit -m "feat(starter): hosted manifest exposes starter_ai_model_id (env-driven)"
```

> **Publish lever (no code, ops only):** when the first starter model is published, set `STARTER_AI_MODEL_ID=<model id>` in the droplet environment (see `docs/runbooks/api-deploy.md`) and redeploy/restart the API. The desktop app then auto-downloads + uses it with no client change.

---

## Phase 7 — Desktop: built-in starter decks in the library

### Task 7.1: `is_builtin` decks merged into the desktop library (read-only)

**Files:**
- Modify: `code/src-tauri/src/deck_storage.rs:14-48,166-239` + new fn/command + tests
- Modify: `code/src-tauri/src/main.rs:116-120`

- [ ] **Step 1: Write the failing test**

In `code/src-tauri/src/deck_storage.rs`, inside `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn builtin_starter_decks_are_six_and_starter_mode() {
        let decks = builtin_starter_decks();
        assert_eq!(decks.len(), 6, "ship all 6 starter decks");
        for d in &decks {
            assert!(d.is_builtin, "{} must be flagged built-in", d.id);
            assert_eq!(d.game_mode, "starter");
            assert_eq!(d.main_deck.len(), 50, "{} main", d.id);
            assert!(d.egg_deck.len() <= 5, "{} egg <= 5", d.id);
            assert!(d.id.starts_with("starter_st"), "{} id prefix", d.id);
        }
        assert!(decks.iter().any(|d| d.id == "starter_st2_cocytus_blue"));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml builtin_starter_decks_are_six`
Expected: FAIL to compile — `builtin_starter_decks` undefined; `Deck` has no `is_builtin`.

- [ ] **Step 3: Add `is_builtin`, the loader, and merge/guards**

(a) Add to the `Deck` struct (after `is_pinned`, line 39) and `DeckSummary` (after its `is_pinned`, line 58):
```rust
    #[serde(default)]
    pub is_builtin: bool,
```

(b) Map it through `deck_summary` (in the `DeckSummary { … }` literal, ~line 168):
```rust
        is_pinned: deck.is_pinned,
        is_builtin: deck.is_builtin,
```

(c) Add the loader near the top of the file (after the `DEFAULT_FOLDER_NAMES` const, ~line 99):
```rust
/// The 6 official starter decks bundled into the binary (generated from
/// `data/deck_library.json` by `code/tools/gen_starter_decks.py`). These are
/// read-only: they always appear in the library and can't be edited/deleted.
const STARTER_DECKS_JSON: &str = include_str!("../../../data/starter_decks.json");

pub fn builtin_starter_decks() -> Vec<Deck> {
    #[derive(serde::Deserialize)]
    struct File {
        starter_decks: Vec<Raw>,
    }
    #[derive(serde::Deserialize)]
    struct Raw {
        id: String,
        name: String,
        egg_deck: Vec<String>,
        main_deck: Vec<String>,
    }
    let file: File =
        serde_json::from_str(STARTER_DECKS_JSON).expect("starter_decks.json is malformed");
    file.starter_decks
        .into_iter()
        .map(|r| Deck {
            id: r.id,
            owner_id: "builtin".into(),
            folder_id: None,
            name: r.name,
            description: "Official starter deck.".into(),
            game_mode: "starter".into(),
            main_deck: r.main_deck,
            egg_deck: r.egg_deck,
            main_deck_alt_arts: vec![],
            egg_deck_alt_arts: vec![],
            commander_id: None,
            is_valid: true,
            validation_errors: vec![],
            is_public: false,
            is_pinned: false,
            tags: vec!["starter".into()],
            meta_tier: None,
            meta_archetype: None,
            is_builtin: true,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        })
        .collect()
}

fn is_builtin_id(id: &str) -> bool {
    builtin_starter_decks().iter().any(|d| d.id == id)
}
```

(d) Merge built-ins into `decks_list` — before the final `out.sort_by(…)` (line 231), append them:
```rust
    for deck in builtin_starter_decks() {
        out.push(deck_summary(deck));
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
```

(e) Make `decks_get` resolve built-ins first (top of the fn, line 237):
```rust
pub fn decks_get(app: AppHandle, deck_id: String) -> Result<Deck, String> {
    if let Some(deck) = builtin_starter_decks().into_iter().find(|d| d.id == deck_id) {
        return Ok(deck);
    }
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    read_deck_file(&path).ok_or_else(|| format!("deck not found: {deck_id}"))
}
```

(f) Guard mutations. In `decks_put` (after `let mut deck = deck;`, ~line 245):
```rust
    if deck.is_builtin || is_builtin_id(&deck.id) {
        return Err("Starter decks are built-in and can't be modified".into());
    }
```
In `decks_delete` (top, ~line 267):
```rust
    if is_builtin_id(&deck_id) {
        return Err("Starter decks are built-in and can't be deleted".into());
    }
```
In `decks_update_library` (top, ~line 387):
```rust
    if is_builtin_id(&deck_id) {
        return Err("Starter decks are built-in and can't be modified".into());
    }
```

(g) Update the test fixture `sample_deck` (line 411) to set the new field so existing tests compile — add to the `Deck { … }` literal:
```rust
            is_pinned: false,
            is_builtin: false,
```

(h) Add the listing command (after `decks_update_library`, ~line 398):
```rust
/// The 6 bundled read-only starter decks, exposed as a Tauri command for a
/// future "starter only" library view / external tooling. (The AI-Starter
/// picker itself reads the bundled `starterDecks.generated.ts`, so it needs no
/// round-trip; this command is the desktop runtime equivalent of that data.)
#[tauri::command]
pub fn rust_list_starter_decks() -> Vec<Deck> {
    builtin_starter_decks()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml deck_storage::`
Expected: all PASS (new `builtin_starter_decks_are_six_and_starter_mode` + existing fixture tests compile with the new field).

- [ ] **Step 5: Register the command**

In `code/src-tauri/src/main.rs`, add to the handler list (after `deck_storage::decks_update_library,` line 120):

```rust
            deck_storage::rust_list_starter_decks,
```

- [ ] **Step 6: Build + commit**

Run: `cargo build --manifest-path code/src-tauri/Cargo.toml`
Expected: builds clean.

```bash
git add code/src-tauri/src/deck_storage.rs code/src-tauri/src/main.rs
git commit -m "feat(starter): bundle 6 read-only starter decks into desktop library"
```

### Task 7.2: Library UI marks built-ins read-only

**Files:**
- Modify: `code/frontend/src/pages/DeckLibraryPage.tsx` (the selected-deck action bar, ~lines 263-276,505-516)

- [ ] **Step 1: Disable destructive actions for built-ins**

In `code/frontend/src/pages/DeckLibraryPage.tsx`, the component tracks `selectedSummary` (a `DeckSummary`). Compute a guard near the other derived values (e.g. beside `pinned`/`others`, ~line 207):

```typescript
  const selectedIsBuiltin = Boolean(selectedSummary?.is_builtin);
```

Then guard the duplicate/delete handlers (`handleDuplicate` ~line 265 calls `library.duplicateDeck`; `handleDelete` ~line 274 calls `library.deleteDeck`) — return early for built-ins:

```typescript
  const handleDelete = async () => {
    if (!selectedSummary || selectedSummary.is_builtin) return;
    await library.deleteDeck(selectedSummary.id);
    // …existing body…
  };
```

And disable the Delete control + add a badge in the action bar (the `Pin/Unpin` control is ~line 516 — add alongside it):

```tsx
        {selectedIsBuiltin && <span className="library-builtin-badge">BUILT-IN</span>}
        <button
          type="button"
          onClick={handleDelete}
          disabled={!selectedSummary || selectedIsBuiltin}
          title={selectedIsBuiltin ? 'Starter decks are built-in' : 'Delete deck'}
        >
          Delete
        </button>
```

> Match the existing button markup in this file — the snippet shows the `disabled`/`title` additions to apply to whatever the current Delete control looks like. The real enforcement is server-side (Task 7.1 guards); this just prevents the dead click.

- [ ] **Step 2: Typecheck**

Run: `cd code/frontend && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add code/frontend/src/pages/DeckLibraryPage.tsx
git commit -m "feat(starter): mark built-in starter decks read-only in the library UI"
```

---

## Phase 8 — Verification

### Task 8.1: Full automated suites

- [ ] **Step 1: Engine + Tauri Rust tests**

Run:
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test starter_format
cargo test --manifest-path code/src-tauri/Cargo.toml
```
Expected: all PASS. (If a phantom error appears in an untouched file, re-run with `CARGO_TARGET_DIR='D:\cargo-target-wt\brave-hopper-7ca8dc'` prefixed — see the note at the top.)

- [ ] **Step 2: Frontend tests + typecheck**

Run:
```bash
cd code/frontend && npx tsc --noEmit && npm test -- src/features/play src/pages/StarterDeckSelectPage.test.tsx src/pages/ModeSelectPage.test.tsx
```
Expected: typecheck clean; all new tests PASS.

- [ ] **Step 3: Python manifest test**

Run: `python -m pytest code/tests/api/test_manifest_starter_pointer.py -v`
Expected: PASS.

### Task 8.2: Manual desktop smoke (no model published → greedy CPU)

- [ ] **Step 1: Launch the desktop app** using the `/run-desktop` recipe (the repo's `beforeDevCommand` is broken in this env — use the skill).

- [ ] **Step 2: Walk the flow**
  - Home → PLAY → confirm a 4th tile **AI STARTER DECK** appears after **BOT MATCH**.
  - Click it → the format grid disappears → **ENTER FORMAT** → lands on the starter picker showing all **6** starter decks.
  - Pick **Starter Deck Cocytus Blue (ST-2)** → **FACE THE AI** → a game starts vs the greedy CPU (no model published yet); confirm the board loads and the AI takes turns.
  - Open the Deck Library → confirm the 6 starter decks appear with a **BUILT-IN** badge and the Delete button is disabled for them.

- [ ] **Step 3: Record the result** in the PR description (screenshots optional). No code change in this task.

---

## Notes for the implementer

- **One source of truth for the decks.** Never hand-edit `data/starter_decks.json` or `starterDecks.generated.ts` — re-run `python code/tools/gen_starter_decks.py` and commit both. The engine test, the Rust built-ins, and the frontend picker all read from this generated pair.
- **Greedy is the shipping default.** Until `STARTER_AI_MODEL_ID` is set on the hosted API, every AI-Starter game is vs the greedy CPU. That is the intended alpha behavior; the trained model lights up server-side with zero client change.
- **ST-2 exception is contained.** Only the `starter` format drops the banlist. `standard` (and every other format) still Limits ST2-13 to 1 — the Phase 1 test pins both halves.
- **Browser build.** The tile/picker render in the browser build too (greedy CPU via the `/games` POST path). Trained-model inference is desktop-only by design.
