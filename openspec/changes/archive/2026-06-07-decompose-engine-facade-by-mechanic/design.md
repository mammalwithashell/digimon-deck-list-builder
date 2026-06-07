## Context

The Rust engine (`code/digimon-engine/`) is the target source of truth for a parity-tracked RL training simulator. Its effect-execution code has organically grown into a few god-files:

| file | LOC | surface |
|------|-----|---------|
| `game_actions.rs` | 7,199 | 46 pub fns (Tier-2 operations) |
| `effect_context/mod.rs` | 6,933 | 307 pub fns (Tier-3 scripting facade) |
| `game.rs` | 5,087 | 90 pub fns (Tier-1 core state) |
| `effect_context/selections.rs` | 3,373 | selection callbacks (Tier-3) |

An exploration established that the engine already has a sound implicit structure that was never named or enforced:

```
  Tier 3  effect_context   "curated card-scripting API" — what a CARD EFFECT may do
            │ delegates to
  Tier 2  game_actions     use-case operations: play / digivolve / move / breeding
            │ built on
  Tier 1  game + player + permanent + combat   state machine, zones, memory, lifecycle

  Output ports (read-only, off to the side):  tensor*/observation · recorder · serialization
```

Three audits scoped the actual defects:

1. **Facade-is-legitimate audit.** The Tier-2/Tier-3 name overlap is *delegation*, not duplication. The facade does four real jobs the core must not: inject effect identity (`PlaySource::ByEffect`, provenance), enforce effect-only guards (`can_affect_permanent` for `CannotBeAffected`), type ergonomics (`PermanentHandle` vs raw indices), and sugar (`bounce_self`). Conclusion: **keep both tiers; do not merge.**
2. **Inversion audit.** Of 57 `EffectContext` construction sites, 56 are legitimate effect-resolution entry points (they call `process()` / `pay_cost_fn()` / `run_steps()` — engine handing control to card-authored code) or tests. Exactly **one** is an inversion: `game_actions::de_digivolve_from_effect` builds a context to call `ctx.de_digivolve(...)`, whose rules logic (pop-loop + `WhenWouldBeDeDigivolved`) lives in the facade.
3. **Fat-facade + pull-vs-push audit.** ~190 facade *action* methods (~5,200 LOC) carry the bloat; the *modifier* surface (`add_*_modifier` / `grant_*`) is healthy — a thin install primitive (~29 callers) under a large declarative DSL vocabulary (`Aura` 58, `GrantKeyword` 102, … 330+ uses), already matching DCGO's declarative-authoring + pull-application model and improving on it with a cached `ModifierRegistry`. The densest action smell: a 3-step `pop()`+`trash.push()`+`fire_digivolution_card_trashed(...)` machine hand-rolled in **9** facade methods while Tier-2 `trash_source_ref`/`remove_source_ref` go unused.

**DCGO cross-check.** The battle-tested C# reference (3,918 card files) scales with a flat *component catalog* + *capability interfaces* and **no god-facade**, with mechanic helpers in one-file-per-mechanic (`DNADigivolveEffects.cs`, `DigiXrosEffects.cs` are separate). This validates by-mechanic decomposition and a one-file-per-modifier-type catalog as the proven shape, and confirms the facade gigantism is the anomaly.

Constraints: parity-tracked engine (rules 17–19, 25); PyO3 boundary convention (rule 20); behavior must be preserved and gated on behavioral + parity (`RUST_PYTHON_PARITY.md`, DCGO replay) + tensor suites.

## Goals / Non-Goals

**Goals:**
- Make the existing 3-tier layering legible: one file per mechanic, a parallel `<tier>/<mechanic>.rs` address scheme across Tier 2 and Tier 3.
- Eliminate the single layering inversion (`de_digivolve`) and the 9× hand-rolled trash-source duplication via one Tier-2 primitive.
- Document an enforceable placement rule so new mechanics land in the right tier instead of re-accreting onto god-files.
- Name the read-only output ports (`observation/`) so the one genuine hexagonal seam is explicit.
- Preserve behavior exactly — no card, action-space, tensor, or parity change.

**Non-Goals:**
- No domain/application/infrastructure layering imposed on the engine. The engine is ~85–90% domain; that ceremony belongs to `code/server/` (where it already exists via the crate split) and would produce an anemic split here.
- No rework of the modifier surface (ruled out by the pull-vs-push audit).
- No merging of Tier 2 and Tier 3 (the facade is a legitimate anti-corruption layer).
- No crate extraction (e.g. promoting `observation/` to its own crate) — deferred; the read-only invariant has held organically.
- No adoption of DCGO's stringly-typed `Hashtable` context or its recompute-every-read stat model.

## Decisions

### D1 — Split `impl` blocks across files, keep `EffectContext` one type
Rust allows a single type's `impl` to span many files. The codebase already does this 14× for `impl Game`. Each mechanic file holds `impl EffectContext { … }` for that mechanic's methods. **Alternative considered:** splitting into sub-structs / sub-traits (e.g. `ctx.play().foo()`). Rejected — it changes the call surface used by every card script and the PyO3 boundary, breaking the behavior-preserving constraint and forcing churn on hundreds of call sites for no semantic gain.

### D2 — Mechanic taxonomy (sized from the real inventory)
Tier-3 `effect_context/`:
- `query/{state,event_ctx,deletion_ctx,source_ctx}.rs` — the ~35 read accessors.
- `action/{trash,sources,play,digivolve,security,zones,combat,modifiers,digixros,scheduling,lifecycle}.rs` — the ~190 mutators. Largest files ≈ `trash` (868), `sources` (781), `play` (653), `digivolve` (583), `security` (510); none over ~870.
- `core.rs` — constructors, `as_read`, `can_affect_permanent`, `refire_*`.
- `selections.rs` — existing sibling; its own split deferred within this change (large, mechanical).

Tier-2 `game_actions/` mirrors the same mechanic names (`play`, `digivolve`, `trash`, `sources`, `zones`, `security`, `combat`). **Rationale:** a learnable address scheme — "operation X on mechanic M → `<tier>/M.rs`" — which is the missing rule that stops re-accretion, and mirrors DCGO's `CardEffectCommons/<mechanic>.cs`. **Alternative:** split by verb only (no tier parallelism). Rejected — loses the cross-tier navigability that makes the rule teachable.

### D3 — Placement rule
"Rules machinery (replacement windows via `try_replace`, observer firing via `fire_*`, direct stack/`battle_area` mutation) lives in **Tier 2**. The **Tier-3 facade** = guards + effect identity + sugar + effect-entry only. An effect-only operation MAY hold logic in Tier 3 **iff** no Tier-2 counterpart exists, and MUST say so in a doc comment." **Optional enforcement:** a lint/test asserting no `try_replace` / `self.game.fire_*` / `battle_area[..]`-write appears in `effect_context/` outside effect-entry points. **Alternative:** convention only, no lint. Kept as optional because the lint has upkeep cost and several legitimately-Tier-3-only ops (digixros materials, `attach_tamer`, `play_token`) need a documented exception mechanism either way.

### D4 — B1: one `trash_source` primitive
Add/standardize a Tier-2 `game_actions` primitive that performs pop + trash-move + `fire_digivolution_card_trashed` with the correct `EventCause`, reusing `trash_source_ref`/`remove_source_ref` semantics. Rewrite the 9 facade methods to delegate. **Risk-managed:** done one call site at a time, each gated on the existing per-mechanic behavioral tests, because observer-firing order is exactly where divergences hide.

### D5 — B3: relocate `de_digivolve` logic down a tier
Move the pop-loop + `WhenWouldBeDeDigivolved` replacement handling from `effect_context::de_digivolve` into a Tier-2 `game_actions::de_digivolve`. Leave a thin facade method: `can_affect_permanent` guard → delegate (matching `return_to_hand`). `de_digivolve_from_effect` then calls the Tier-2 fn directly instead of constructing an `EffectContext` to reach up. **Net effect:** the inversion disappears; behavior identical. Guarded by `dedigivolve-resolution-parity` + `permanent-deletion-semantics` specs/tests.

### D6 — Name the output ports
Move `tensor*.rs`, `observation.rs`, `tensor_profiles/` under `observation/` and document the invariant: reads `&Game`, never `&mut`. Audit confirmed all entry points already take `&Game` and only two callers exist. **Alternative:** extract to a `digimon-observation` crate (compiler-enforced). Deferred (D-nongoal) — the invariant has held without enforcement; revisit only if mutation is ever tempted.

### D7 — Sequencing: Phase A (mechanical) fully before Phase B (behavioral)
Land all pure code-movement first so the diff is reviewable as "no logic changed," then do B1/B3 as small isolated logic relocations on top. **Rationale:** keeps the high-risk behavioral changes in tiny, individually-revertable commits against an already-reorganized, green tree.

## Risks / Trade-offs

- **Silent parity divergence from a "refactor" that changes resolution order or a tensor offset** → Phase A is pure movement (no body edits); run the full behavioral + parity + tensor suite after each module move; B1/B3 land one call site at a time, each gated on targeted tests before the next.
- **`fire_digivolution_card_trashed` ordering differences when collapsing 9 copies onto one primitive** → keep the primitive's event sequence byte-identical to the most common existing copy; diff observer-trigger order in tests for each migrated method; migrate incrementally.
- **`de_digivolve` relocation alters de-digivolve/deletion timing** → the existing `dedigivolve-resolution-parity` and `permanent-deletion-semantics` specs are the regression net; treat them as must-pass gates; behavior target is byte-identical.
- **Large mechanical diff is hard to review** → consistent one-mechanic-per-commit structure; `lib.rs` re-export updates isolated; no public API rename so reviewers can diff by module.
- **Merge churn against in-flight card work** (many active changes touch `cards/` and `dsl_cards/`, not these core files) → these files are rarely touched by card-authoring changes; land Phase A as a focused, fast sequence to minimize the window.
- **Optional lint upkeep / false positives on legitimately-Tier-3-only ops** → ship the lint only after the documented-exception mechanism (D3 doc comments) exists; start in warn mode.

## Migration Plan

1. Phase A: per-mechanic module moves (Tier 3 `query/` then `action/`, then Tier 2 `game_actions/`, then `observation/`), each commit updating `lib.rs` re-exports and running the full suite.
2. Phase B1: introduce `trash_source` primitive; migrate the 9 facade methods one at a time, suite-gated.
3. Phase B3: relocate `de_digivolve` down a tier; repoint `de_digivolve_from_effect`; parity-gated.
4. Document the placement rule in `docs/RUST_ENGINE_API.md`; optionally add the lint (warn → deny).
5. Rollback: each phase is independently revertable; Phase A reverts are trivial (movement only); B1/B3 are small isolated commits.

## Resolved Questions

The three open questions below were researched against the codebase; each resolution *tightens* scope rather than expanding it.

### RQ1 — Ship the lint now, in warm-up mode (not deny, not deferred)
**Resolution: document the rule now; ship the lint in this change in warn / `continue-on-error` mode with an explicit allowlist for documented Tier-3 exceptions; promote to required in a follow-up after B1/B3 land.**
- The repo has the exact precedent: `code/tools/dsl-lint/` proves the custom-lint-crate pattern, and `.github/workflows/action-space-codegen-drift.yml` ships `continue-on-error: true` during rollout then "promote[s] to required by removing `continue-on-error`."
- This would be the repo's *first* architectural lint (import rules 11/12/22 are convention-only, unenforced today).
- The lint cannot be a naive grep: it must distinguish effect-entry constructions (`process(&mut ctx)`) from inversions (`ctx.de_digivolve(...)`) and exempt the legitimately-Tier-3-only ops (digixros materials, `attach_tamer`, `play_token`) — the same classification the inversion audit did by hand. Hence the allowlist.
- **Hard ordering constraint:** a *deny*-lint shipped before Phase B would turn CI red on the still-present `de_digivolve` inversion. Warn-first is therefore required by sequencing, not merely preferred.

### RQ2 — Defer the `selections.rs` split
**Resolution: defer entirely; record as a named follow-up ("split `selections.rs` by selection-target if it keeps growing").**
- `effect_context/selections.rs` (3,373 LOC) is *not* a multi-domain god-file: it is one cohesive concern — 35 `select_*` selection primitives (`select_opponent_permanent`, `select_hand`, `select_trash`, `select_material`, `select_count_capped_multi`, …).
- Its natural split axis is *selection-target*, which is **orthogonal** to Phase A's by-mechanic taxonomy. Bolting it into this change would add 3,373 LOC of movement for no cohesion win and muddy the "organized by mechanic" review narrative.

### RQ3 — Narrow the optional `game.rs` split
**Resolution: keep `game.rs` work as *optional* and narrow it — extract only the `until_condition` machinery and the read-only query/aura-bonus helpers; leave the state-machine core intact. A full 90-fn split is deferred to a dedicated follow-up.**
- Splitting is mechanically free (14 files already carry `impl Game`; Rust impl-splitting shares full field visibility, so the 25-field coupling surface is not a barrier).
- But `game.rs` (90 fns) is cohesive Tier-1 core and not the acute pain (the facade's 307 fns are). Only the `until_condition` machinery and read-only query helpers (`can_digivolve`, `has_keyword`, `*_aura_bonus`, `effects_for_card`) are clearly separable; the lifecycle core (`new`/`start_game`/`turn`/`memory`/`declare_winner`) gains little from moving and a full split risks an unreviewable diff for the lowest-pain tier.

## Deferred to follow-up changes

- Split `effect_context/selections.rs` by selection-target (only if it keeps growing).
- Full `game.rs` mechanic split (beyond the narrow `until_condition` + query-helper extraction).
- Promote the placement-rule lint from warn → deny/required, once B1/B3 have landed and the Tier-3 exception allowlist is stable.
