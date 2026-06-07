## Why

The Rust engine's card-scripting facade (`effect_context/mod.rs`) has grown to 6,933 lines / 307 methods, and sibling core files (`game_actions.rs` 7,199, `game.rs` 5,087) are comparably large. The code already has an implicit, sound 3-tier structure (core state → operations → scripting facade) plus clean read-only output ports — but the tiers are smeared across a handful of god-files with no documented placement rule, so each new mechanic re-accretes onto the same files. Three targeted audits (below) show the defects are **localized**, not systemic, which makes this a high-value, low-risk cleanup if done now before the file grows further.

This change makes the existing layering **legible** and **enforceable**. It is **behavior-preserving** — pure code movement plus two small, test-guarded logic relocations. No card behavior, action space, tensor layout, or parity contract changes.

## What Changes

- **Phase A — mechanic decomposition (pure code movement):** split the Tier-3 facade (`effect_context/mod.rs`) by mechanic into ~14 `impl EffectContext` files under `effect_context/action/` and `effect_context/query/` (e.g. `action/play.rs`, `action/trash.rs`, `action/digivolve.rs`, `action/sources.rs`, `action/security.rs`, `action/combat.rs`, `query/event_ctx.rs`, …). `EffectContext` stays one type; only the `impl` block is split across files (the codebase already does this 14× with `impl Game`). Apply the same mechanic split to `game_actions.rs` (Tier 2), yielding a **parallel taxonomy** so an operation lives at a predictable `<tier>/<mechanic>.rs` address.
- **Phase A — name the output ports:** group the already-read-only `tensor*.rs` / `observation.rs` / `tensor_profiles/` under an `observation/` module documented as an output port (reads `&Game`, never mutates).
- **Phase B1 — extract a shared `trash_source` primitive (Tier 2):** the facade hand-rolls `card_sources.pop()` + `trash.push()` + `fire_digivolution_card_trashed(...)` in **9** separate methods while Tier-2 `trash_source_ref` / `remove_source_ref` sit unused; collapse the 9 copies onto one primitive.
- **Phase B3 — fix the one layering inversion:** `game_actions::de_digivolve_from_effect` constructs an `EffectContext` to call `ctx.de_digivolve(...)`, whose pop-loop + `WhenWouldBeDeDigivolved` replacement logic lives in the facade (Tier 3). Move that logic **down** to a Tier-2 `de_digivolve`, leaving a thin guard+delegate in the facade (matching `return_to_hand`'s shape). This is the **only** inversion in 57 `EffectContext` construction sites.
- **Document a placement rule + optional lint:** "rules machinery (replacement windows, observer firing, stack mutation) lives in Tier 2; the facade = guards + identity + sugar + effect-entry only; effect-only operations may hold logic IFF no Tier-2 counterpart exists, and must say so in a doc comment." Optionally enforce via a lint (no `try_replace` / `fire_*` / `battle_area[..]` mutation in `effect_context` outside effect-entry points).
- **NON-GOAL (explicitly ruled out): the modifier surface.** The pull-vs-push audit showed `add_*_modifier` / `grant_*` are a healthy, thin installation substrate (~29 internal callers fanning out to 330+ declarative DSL-clause uses), already matching DCGO's declarative-authoring + pull-application model and improving on it with caching. No modifier-surface rework. (Captured here so it is not re-litigated.)

## Capabilities

### New Capabilities
- `engine-effect-context-layering`: the architectural contract for the engine's effect-execution tiers — where rules logic lives (Tier 2 core vs Tier 3 facade), the by-mechanic module organization, the no-inversion invariant, the shared `trash_source` primitive, and the read-only output-port boundary. These are enforceable structural invariants (some lint-checkable), not card-behavior requirements.

### Modified Capabilities
<!-- None. This change is behavior-preserving: no spec-level behavior changes.
     de_digivolve / deletion / parity behavior is intentionally unchanged; the
     existing dedigivolve-resolution-parity and permanent-deletion-semantics
     specs remain valid and act as the regression net for Phase B3. -->

## Impact

- **Code (moved, not rewritten):** `code/digimon-engine/src/effect_context/` (mod.rs → `action/`, `query/`, `core.rs`), `code/digimon-engine/src/game_actions.rs` (→ `game_actions/` by mechanic), `code/digimon-engine/src/{tensor*.rs,observation.rs,tensor_profiles/}` (→ `observation/`).
- **Code (logic relocated, behavior-preserving):** `game_actions.rs` + `effect_context/mod.rs` for B1 (`trash_source`) and B3 (`de_digivolve`).
- **No change to:** public engine API used by `digimon-engine-py` (PyO3), the 2192-action space, tensor profiles/layout, DSL surface, or any card YAML. `lib.rs` `pub mod` lines update to re-export the new module paths.
- **Safety net:** existing behavioral, parity (`RUST_PYTHON_PARITY.md`, DCGO replay), and tensor test suites must remain green at every step; B1/B3 (timing relocation) get extra scrutiny as the only behavioral-risk surface.
- **Constraint:** parity-tracked RL engine — every step is behavior-preserving and gated on the full test suite (rules 17–19, 25).
