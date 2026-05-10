# Track E DSL Verbs Prompt

This file preserves the active Codex prompt for the Track E zone-movement DSL work.

## Goal

Land the ten deferred YAML `kind:` verbs for the Rust engine zone-movement layer in
`code/digimon-engine/` and the YAML DSL crate in `code/digimon-dsl/`.

## Verbs

| YAML `kind` | Engine helper | Driver cards |
|---|---|---|
| `bounce_self` | `EffectContext::bounce_self()` | BT24-012 and any Tamer/Option self-bounce |
| `place_self_at_security` | `EffectContext::place_self_at_security(position, face_up)` | EX9-021, EX4-060 |
| `place_self_option_at_security` | `EffectContext::place_self_option_at_security(position, face_up)` | ST20-15 |
| `place_permanent_on_security_observed` | `Game::place_permanent_on_security_observed` | another permanent to security with source preservation |
| `security_place_stacked_card` | `EffectContext::security_place_stacked_card(carrier, source_handle, position, face_up)` | Puppets G027 |
| `security_place_top_stacked_card` | `EffectContext::security_place_top_stacked_card(carrier, position, face_up)` | Puppets G027 top-source flavor |
| `return_all_trash_to_deck_bottom` | `EffectContext::return_all_trash_to_deck_bottom(player)` | BT17-077 |
| `trash_top_n_digivolution_cards_of_each` | `EffectContext::trash_top_n_digivolution_cards_of_each(target_player, n)` | BT12-028 |
| `trash_opponent_hand_to_count` | `EffectContext::trash_opponent_hand_to_count(opponent, target_count)` | BT19-075 |
| `search_own_security_stack` | `EffectContext::search_own_security_stack(filter, ...)` | TS Olympos |

## Required Work

- Add `Step` variants in `code/digimon-dsl/src/step.rs`.
- Add parser/serde support, validator checks, compile-down, and `CompiledStep` variants.
- Add engine-side lowering arms in `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
  or the existing relevant step modules.
- Reuse existing selector, formula, predicate, and filter primitives.
- Do not reimplement the engine helpers.
- Ensure missing substrate fails loudly; never silently no-op.
- Add parse and compile tests for every verb, including negative cases.
- Add engine-side DSL behavioral tests that load YAML, compile it, run it through
  `DebugRunner`, and assert engine state.
- Add card-shaped YAML fixtures and behavioral-test skeletons for the driver cards.
- Update `qa/dsl-vocab-gaps.md`, related archetype QA rollups, `docs/RUST_ENGINE_GAPS.md`,
  and `docs/RUST_ENGINE_API.md`.

## Constraints

- No approximations and no hidden auto-selection.
- No `ACTION_SPACE_SIZE`, tensor-profile, PyO3, frontend constant, or RL wrapper changes.
- Do not modify the engine helpers themselves.
- Printed text and rules sources take priority over DCGO; DCGO is implementation-shape reference only.
- Follow TDD: failing parse or behavioral tests before implementation.
- Do not author Python-side card scripts or import from `code/engine_py_legacy/`.
- Match the existing `place_permanent_on_security_and_handle_replacement` verb as the canonical template.

## Verification

Run:

```bash
cargo test --manifest-path code/digimon-dsl/Cargo.toml
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker Discipline

Mark entries closed, partially closed, or narrowed with test command evidence in:

- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/archetype-qa/dsl/*.md`
- `docs/RUST_ENGINE_GAPS.md`
- `docs/RUST_ENGINE_API.md`
