# Card Pool Resolution

Use this reference when the user gives an archetype, deck name, decklist, or loose card group.

## Resolution Order

1. **Exact card list**: use the given card IDs exactly.
2. **Decklist**: parse the provided list; preserve counts and separate Digi-Eggs from main deck.
3. **Named archetype**:
   - Check `data/archetype_aliases.json` for canonical names.
   - Search `data/deck_library.json` for matching archetype names and decklists.
   - Use counts/frequency to identify core cards.
   - Include common support Options/Tamers only when they appear in the resolved decklists or archetype QA notes.
4. **Existing QA source**:
   - Search `qa/archetype-qa/` and `qa/archetype-qa/dsl/` for prior readiness or gap documents.
   - Reuse previous card grouping if it matches the target; update stale status with current code evidence.
5. **Card metadata**:
   - Prefer per-card metadata from `code/digimon-engine/cards/<set>/<CARD-ID>.json` for `effect_description_eng`, `inherited_effect_description_eng`, `security_effect_description_eng`, type, colors, traits, levels, DP, play cost, `evo_costs`, and `xros_req`.
   - Use `data/cards.json` only when the per-card JSON is missing or to cross-check an apparent metadata mismatch.

## Core vs Tech

Classify cards as:

- `core`: repeated across decklists, named by archetype engine, or required for the main gameplay loop.
- `support`: common support cards that enable the core loop.
- `tech`: low-frequency or meta-dependent cards.
- `out-of-scope`: cards present in one list but unrelated to the requested implementation slice.

Implement core cards first. Do not block an archetype on a tech card unless the user explicitly requested full deck coverage.

## Existing Implementation Check

For each card, inspect:

- Per-card metadata: `code/digimon-engine/cards/<set>/<CARD-ID>.json`.
- Production YAML: `code/digimon-engine/cards/<set>/<CARD-ID>.yaml`.
- Example YAML: `code/digimon-engine/cards/_examples/`.
- Behavioral tests: `code/digimon-engine/tests/cards_behavioral/<set>/`.
- DSL tests: `code/digimon-engine/tests/dsl/`.
- Raw Rust registry: `code/digimon-engine/src/cards/raw_rust/`.
- Gap trackers: `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`.

Statuses:

- `implemented`: full printed behavior is covered by active tests.
- `partial`: YAML/test exists but a printed clause is omitted, ignored, raw-Rusted, or blocked.
- `not-authored`: no production YAML.
- `blocked`: known reusable DSL or engine gap prevents faithful implementation.

## Implementation Queue Shape

Produce a queue like:

| Priority | Card | Scope | Status | First task | Dependency |
|---|---|---|---|---|---|
| 1 | `CARD-ID` Name | core | not-authored | On Play reveal/search | existing DSL |
| 2 | `CARD-ID` Name | core | blocked | source-selection cost | `select_own_sources` primitive |

Prefer a queue that lands supported DSL cards first, then closes high-reuse primitives before large card batches.
