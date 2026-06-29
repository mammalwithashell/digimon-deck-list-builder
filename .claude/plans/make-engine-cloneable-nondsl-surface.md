# make-engine-cloneable — non-DSL selection surface (attack plan)

Produced 2026-06-28. The DSL card-effect selection surface is fully on the resumable VM
(see `make-engine-cloneable-remaining-installers.md`). This plan sequences the REMAINING
closure-based selections — the engine-core / non-DSL surface that still clones to the
`PendingSelection` panic-stub. Completing this unblocks task 6.1 (delete the legacy
executor) and makes `Game: Clone` faithful everywhere.

## The proven pattern (from the digivolve cost-choice flip, commit 9d1cf63b8)

Every non-DSL flip is the same five moves:

1. **Frame** — add `ResumeFrame::X(XState)` (or reuse an existing kind) holding the
   closure's captured params as **plain Clone/Debug data** (Copy scalars + Clone vecs;
   no closures). Define `XState` in the owning module (`pub(crate)`), referenced from
   `resume.rs` (the `LinkPickStep`/`DigivolveCostChoice` precedent: a pub enum variant
   may hold a `pub(crate)` type in this crate).
2. **Resolve method** — `Game::run_X_step(state, action_id[, is_pass])` (or a free fn in
   the owning module) that mirrors the closure callback **1:1**.
3. **Park** — at the install site, after `self.pending_selection = Some(..)`, set
   `self.pending_selection_resume = Some(ResumeStack { frames: vec![ResumeFrame::X(..)] })`.
   Clone any captured data BEFORE the closure moves it (coexistence: both paths install).
4. **wrap arm** — add the variant to `wrap_pending_selection_with_tail`'s match
   (`dsl_cards/step/mod.rs`). `unreachable!("…never nested in a DSL clause")` for
   top-level player/engine actions; a real `s.outer_conts.push(cont)` only if the
   selection CAN be installed mid-DSL-clause (rare — verify per site).
5. **run_resume arm** — `ResumeFrame::X(state) => game.run_X_step(state, action_id, …)`.

**Parity gate, for free:** because the park sets `pending_selection_resume`, the
subsystem's EXISTING tests auto-route through the resume path (`resolve_generic_selection`
prefers it) — so the existing suite IS the parity gate. Add one `*_clones_faithfully`
test per flip + the standing full gate (`cards_behavioral` 5830 / `dsl` 782 /
`archetypes` 211 / `option_flow` 133 / lib `resume`, `RUST_MIN_STACK=268435456`,
`--test-threads=8`, isolated `CARGO_TARGET_DIR`).

## Inventory (every remaining closure-based install site)

| # | Site | Kind | Shape | Value* | Diff | Chain |
|---|------|------|-------|--------|------|-------|
| ✅ | `digivolve.rs:412` cost-choice | EffectChoice | index→re-enter digivolve | high | — | head of digivolve chain (DONE) |
| A1 | `digivolve.rs:493` reducer accept/decline | EffectChoice (opt) | accept→`reducer_accept`; decline→re-enter | high | low | chains → A2 |
| A2 | `digivolve.rs:567` reducer suspend-cost | OwnField | bind→pay suspend→`consume_reducer`→re-enter | high | low | tail of A1 |
| A3 | `cost.rs:133` play-from-hand cost-reduction | EffectChoice | index→`continue_play_from_hand_cost_reduction_chain` | high | low | self-chaining |
| A4 | `cost.rs:346` play digivolve cost-reducer | EffectChoice (opt) | accept/decline→re-enter | high | low | parallel to A1 |
| A5 | `options.rs:520` option-play mode | EffectChoice (opt) | index→option mode | med | low | — |
| A6 | `options.rs:776` option-play | EffectChoice | index→play option | med | low | — |
| A7 | `effect_queue.rs:202` refire-effect | EffectChoice | index→refire chosen effect | med | low | — |
| B1 | `game_phases.rs:730` Overclock | OwnField (opt) | bind token→delete→attack-without-suspend | med | med | — |
| B2 | `game/lifecycle.rs:156` BO3 play-order | PlayOrder | First/Second→`last_play_order_choice` | low | low | — (rare in search) |
| B3 | `misc.rs:602` DigiXros material | Material (opt) | accumulate material→finish DigiXros | med | med | multi-pick (like SourceMulti) |
| B4 | `digivolve.rs:955/1015` DNA digivolve | Material ×2 | two-stage material pick → DNA digivolve | med | med | 2-stage chain |
| B5 | `effect_queue.rs:3412` security-removal replacement | Replacement (opt) | accept→run replacement | med | med | — |
| B6 | `replacement.rs:1012` passive "would" replacement | Replacement (opt) | accept→run replacement | med | med | — |
| B7 | `app_fuse.rs:118/181` effect App-Fusion | Target ×2 | pick host/material → app-fuse | low | med | — |
| C1 | `action/combat.rs:143` `select_redirect_attack_target` (primitive) | Target | redirect (reuses `AttackTarget` kind) | high | med | non-DSL callers (keyword/raw_rust) |
| C2 | `action/combat.rs:344` `may_attack_now_*` / `force_opponent_attack` (primitive) | Target | begin-attack (reuses `BeginAttack` kind) | high | med | non-DSL callers |
| C3 | `combat/mod.rs:1268` Alliance declaration | OwnField (opt) | suspend ally→add DP+SA | **highest** | high | attack interrupt window |
| C4 | `combat/mod.rs:1563/1724/1750` Counter | Hand/Material | counter-effect sub-selection | **highest** | high | attack interrupt window |
| C5 | `combat/mod.rs:1984` Block redirect | OwnField | blocker→redirect attack | **highest** | high | attack interrupt window |
| C6 | `combat/mod.rs:2165/2386` (opp-field combat picks) | OppField | effect-during-attack target | high | high | attack interrupt window |
| C7 | `effect_queue.rs:3503` TriggerOrder | TriggerOrder | order simultaneous triggers | high | med | every multi-trigger moment |

*Value = how often a search rollout pauses here (combat/triggers/digivolve = constant; play-order/app-fuse = rare).

## Sequenced waves

### Wave A — simple `EffectChoice`/index frames (the cost-choice pattern, lowest risk)
A1–A7. Each is "decode an index → re-enter a Rust fn" (or accept/decline → two Rust paths).
Identical shape to the DONE cost-choice flip; mostly `unreachable!` wrap arms (player
actions). **Do A1+A2 together** (the reducer accept→suspend-cost chain — a clone mid-chain
is only faithful once BOTH are flipped). A3/A4 are the cost-reduction cousins. A5–A7 are
independent. Expect ~1 frame variant each (or reuse a generic `EngineEffectChoice` carrying
a small enum tag if they collapse). **Goal: clear all 7 in 1–2 sessions; each full-gated.**

### Wave B — single-pick engine-action frames (bind a permanent/card/material → Rust action)
B1–B7. A frame carrying the post-pick action as data (like `FieldPermanentPostAction`).
- B1 Overclock, B2 play-order: self-contained single-pick → trivial frames.
- B3 DigiXros material, B4 DNA digivolve: **multi-pick / multi-stage** — model like the
  existing `SourceMultiStep`/`MultiPickStep` (accumulate then terminate) or a 2-stage
  frame-installs-frame (DNA stage1→stage2).
- B5/B6 replacement-accept: a `Replacement` "you may" frame (accept→run replacement,
  decline→continue). Shared shape; do them together.
- B7 app-fuse: low value, defer to end of B.

### Wave C — combat interrupt sub-machine + `Target` primitives + TriggerOrder (the hard half)
Highest value (combat + triggers fire on nearly every turn) and highest complexity.
- **C1/C2 first** — the `Target` primitives already have resume kinds (`AttackTarget`/
  `BeginAttack` from the DSL flips); lift the park from the DSL step handler INTO the
  primitive (`effect_context/action/combat.rs`) so ALL callers (keyword/raw_rust) get it.
  This is mostly relocating proven machinery.
- **C7 TriggerOrder** — a frame carrying the orderable trigger list; decode pick → order →
  re-drain. Self-contained-ish; high value; do early in C.
- **C3–C6 the attack interrupt window** — the genuine sub-machine. Counter/Block/Alliance
  install selections DURING `begin_attack_open`'s interrupt resolution, each with state
  that spans multiple picks. Model the interrupt window as resumable state (a
  `CombatInterruptState` frame, or a small stack of them). **This is the make-engine-
  cloneable analog of `link_cards` — give it a dedicated session + a design pass first.**
  Reuse the C1/C2 `AttackTarget`/`BeginAttack` groundwork.

## Cross-cutting concerns

- **wrap-arm reachability** — for each site, verify whether it can be installed mid-DSL-
  clause (grep callers up to a player action vs an effect clause). Engine-core actions
  (digivolve, combat open, play-order, overclock) are NOT nested → `unreachable!`. Anything
  an effect can trigger mid-clause (some replacements, app-fuse) needs a real `outer_conts`
  arm. **Always trace the caller before choosing the arm.**
- **chains** — A1→A2, B4 stage1→stage2, and the digivolve cost-choice→reducer→suspend-cost
  chain mean a clone is faithful at a chain link ONLY once every later link is also flipped.
  Sequence chained sites together and add a clone test that forks MID-chain.
- **atomicity** — like the DSL sites, verify each post-action's atomicity (does it
  `drain_effect_queue` → can park a nested selection?). Non-atomic ones need the empty-tail
  + `outer_conts` thread (but most non-DSL frames have no DSL tail; the nested park is
  another non-DSL selection, faithful once IT is flipped).
- **gate every flip** — full `cards_behavioral` + `dsl` + `archetypes` + `option_flow` +
  lib `resume`. The subsystem's own tests are the parity gate (they auto-route post-park).
- **6.1 cutover** — only after C completes: rewrite the ~8 callback-composition wrapper
  sites onto `after_selection_resume_hooks` unconditionally, then delete
  `resolve_generic_selection`'s `else { sel.callback }` branch + the non-Optional
  `PendingSelection.callback` field. `partition` (test-only) may stay closure-based.

## Rough effort

Wave A ≈ 1–2 sessions (7 small flips). Wave B ≈ 2–3 sessions (7 flips, 2 multi-pick).
Wave C ≈ 3–4 sessions (C3–C6 the attack window is the big one). Plus the 6.1 cutover +
7.2/7.3/7.4 final gates. Each flip is independently shippable and full-gated — no big-bang.
