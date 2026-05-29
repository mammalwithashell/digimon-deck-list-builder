# Scenario fixtures

Declarative `(staged board → expected outcome)` fixtures for gameplay-rules
conformance. One JSON file per scenario. The **same file** is consumed by
two layers:

- **Rust headless runner** (`code/digimon-engine/tests/scenario_corpus.rs`)
  — stages a `DebugRunner`, applies the action script, evaluates the
  `engine` assertions. Fast, runs in `cargo test`.
- **Playwright UI fixture** (`code/frontend/e2e/`) — stages the same board
  over HTTP via the `/debug` router, then evaluates `engine` assertions
  server-side (`POST /debug/games/{id}/evaluate`) and `ui` assertions
  against the rendered DOM.

This split is deliberate: engine-correctness ("is the rule right?") has one
source of truth and is evaluated once, server-side; UI-wiring ("can a human
reach it?") is DOM-level and lives only in the Playwright layer.

The corpus is seeded from a community rules quiz (mechanics paraphrased, not
reproduced). Each fixture is tagged by implementation readiness so the
corpus can grow ahead of per-card implementation.

## File shape

```jsonc
{
  "schema_version": 1,
  "id": "q16-paildramon-partition-self-deletion",   // unique, kebab-case
  "title": "Partition does not trigger on self-deletion",
  "readiness": "expected_pass",   // or "blocked_on_card_impl"
  "blocked_reason": "",           // required when readiness=blocked_on_card_impl

  // Full deck lists (incl. 4 Digi-Eggs each). Needed so the registry,
  // observation tensor, and a real draw pile exist. Staged zones below
  // override whatever is dealt.
  "decks": {
    "1": ["BT12-002", "BT12-002", "...50 main + 4 egg ids..."],
    "2": ["..."]
  },

  "seed": 12345,                  // optional; pins the shuffle for reproducibility

  // Scalar starting state.
  "state": {
    "memory": 0,                  // positive favors player 1
    "phase": "Main",              // Mulligan|Unsuspend|Draw|Breeding|Main|EndTurn
    "turn": 4,
    "first_player": 1             // Python 1/2 convention
  },

  // Per-player zone staging. Any zone present REPLACES what was dealt
  // (the loader clears it first). Omitted zones keep the dealt contents.
  "zones": {
    "1": {
      "hand": ["P-165"],
      "deck_top": ["..."],        // ordered, index 0 = next draw
      "security": ["..."],        // ordered top -> bottom
      "trash": ["..."],
      "field": [
        { "stack": ["BT12-022", "BT12-050", "AD1-011"], "suspended": false, "turn_played": 0 }
        // stack is bottom-to-top: AD1-011 (Paildramon) is the top card,
        // ExVeemon + Stingmon are its digivolution sources.
      ],
      "breeding": ["BT12-002"]    // bottom-to-top stack, optional
    },
    "2": { "field": [ { "stack": ["EX6-..."], "suspended": false, "turn_played": 0 } ] }
  },

  // Optional: instead of (or in addition to) direct zone injection, reach
  // the state by replaying human action ids through the live /games path.
  // Each is validated against the live mask before stepping (illegal -> error).
  "action_script": [],

  "assertions": {
    // Engine-correctness — evaluated server-side / in the Rust runner.
    "engine": [
      { "kind": "memory_equals", "value": 0 },
      { "kind": "stack_top", "player": 1, "field_index": 0, "card_id": "AD1-011" },
      { "kind": "effective_dp", "player": 2, "field_index": 0, "value": 9000 },
      { "kind": "zone_count", "player": 1, "zone": "trash", "value": 3 },
      { "kind": "zone_contains", "player": 1, "zone": "hand", "card_id": "P-165" },
      { "kind": "effect_triggered", "event_type": "Partition", "expected": false },
      { "kind": "action_legal", "action_id": 79, "expected": true },
      { "kind": "legal_selection_options", "count": 2 }
    ],
    // UI-wiring — Playwright-only, evaluated against the DOM.
    "ui": [
      { "kind": "dna_option_present" },
      { "kind": "field_target_highlighted", "player": 1, "field_index": 0 }
    ]
  }
}
```

## Engine assertion vocabulary

| `kind` | Fields | Passes when |
|---|---|---|
| `memory_equals` | `value` | memory gauge equals `value` |
| `stack_top` | `player`, `field_index`, `card_id` | top card of that permanent is `card_id` |
| `effective_dp` | `player`, `field_index`, `value` | the permanent's effective DP equals `value` |
| `zone_count` | `player`, `zone`, `value` | the zone holds exactly `value` cards |
| `zone_contains` | `player`, `zone`, `card_id` | the zone contains `card_id` |
| `effect_triggered` | `event_type`, `expected` | an event of that type was (not) emitted during resolution, matching `expected` |
| `action_legal` | `action_id`, `expected` | the action mask bit matches `expected` |
| `legal_selection_options` | `count` | the outstanding selection offers exactly `count` options |

`zone` ∈ `{hand, deck, security, trash}` for count/contains.

## UI assertion vocabulary (Playwright-only)

| `kind` | Fields | Passes when |
|---|---|---|
| `dna_option_present` | — | the DNA-digivolve affordance is rendered/clickable |
| `field_target_highlighted` | `player`, `field_index` | that permanent is highlighted as a legal selection target |
| `selection_panel_options` | `count` | the selection panel renders `count` choices |

## Readiness tags

- `expected_pass` — should pass against the current engine; a failure is a regression.
- `blocked_on_card_impl` — depends on card behavior not yet implemented. The
  harness reports these as **pending** (not failures). When a blocked fixture
  starts passing, flip its tag — that surfaces newly-covered behavior.

## Authoring notes

- `stack` arrays are always **bottom-to-top**: the last id is the top card,
  earlier ids are its digivolution sources.
- A staged board must be rule-consistent enough for the turn machine; the
  loader calls `validate()` and fails loud on an illegal board (e.g. a
  non-Mulligan phase with a mulligan still owed).
- Keep `decks` large enough that staged deck overrides don't deck-out the
  player mid-scenario.
