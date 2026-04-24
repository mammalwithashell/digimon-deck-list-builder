# DCGO ↔ Rust Keyword Parity — Design Spec

**Date:** 2026-04-24
**Scope doc (current state):** [docs/DCGO_KEYWORD_PARITY.md](../../DCGO_KEYWORD_PARITY.md)
**Related trackers:** [docs/RUST_ENGINE_GAPS.md](../../RUST_ENGINE_GAPS.md), [docs/RUST_PYTHON_PARITY.md](../../RUST_PYTHON_PARITY.md)
**Rules source:** [docs/RULES_CONTEXT.md](../../RULES_CONTEXT.md) §16
**DCGO source of truth:** `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/*.cs`

## 1. Goal

Bring the Rust engine's printed-keyword surface to full behavioral parity with DCGO under the printed rules (RULES_CONTEXT §16), in a phased, test-driven sequence that:

1. Fixes the shipped-wrong Progress semantics before further cards are scripted against the incorrect site.
2. Closes easy declarative and combat gaps (Jamming scope, Security A. ±N auto-install, enum cleanup) that don't need new framework.
3. Backfills the missing-from-enum keywords required by the alpha archetypes (Royal Knights, Jesmon GX, Rocks, Medusamon, Dark Masters).
4. Delivers the shared infrastructure — **source-attribution threading** and **nested-selection-in-replacement** — that the remaining keyword wire-ups (Save auto-install, Fortitude, Decoy, Fragment, ArmorPurge, Partition) all depend on.
5. Removes dead enum variants that never had DCGO counterparts.

Every keyword change must preserve the no-approximations invariant: no stubs, no auto-selections, every choice surfaces through `pending_selection` for the RL action space.

## 2. Non-goals

- **Python engine parity.** This spec is Rust-only. Where the Rust fix deliberately diverges from the current Python behavior, we log a row in `RUST_PYTHON_PARITY.md`; we do not back-port. Python is in sunset.
- **New card scripting.** Card effects themselves are out of scope; this spec delivers the keyword substrate that card scripts and native-keyword parsing consume.
- **Delay / Digi-Burst / Link+X / DNA Blast Digivolve.** These are mechanics, not keywords consumed via `Keyword` enum; their treatment belongs in separate specs (Delay already landed via the DSL Phase 2a lowering).

## 3. Current state snapshot

From DCGO_KEYWORD_PARITY.md summary table:

| Bucket | Count | Keywords |
|---|---|---|
| ✅ Correct | 11 | Rush, Blocker, Piercing, Reboot, Blitz, Raid, Alliance, Vortex, Overclock, Collision, Barrier, Evade, Decode |
| 🟡 Divergent | 3 | Jamming (scope), Progress (wrong site — shipped incorrect), Blast (dead variant) |
| 🟣 Deferred (nested-select infra) | 3 | Fragment(N), ArmorPurge, Partition |
| 🔴 Parsed-but-unwired | 8 | SecurityAttackPlus(N), SecurityAttackMinus(N), DeDigivolve(N), DrawX(N), Save, Fortitude, Decoy, plus dead variants (Armor, Material) |
| ❌ Missing from enum | 7 | MaterialSave(N), MindLink, Iceclad, Execute, Retaliation, Scapegoat, Training |
| Enum mis-mapping | 1 | `GrantBarrier` sits where `GrantFortitude` belongs |

## 4. Approach — three substrate threads

The work decomposes into three orthogonal substrate threads. All later phases assemble finished keywords from these primitives.

### 4.1 Source-attribution thread

**Problem.** DCGO's `CanNotAffectedClass` and its cousins filter on `IsOpponentEffect(cardEffect, cardSource)` — they gate on the *controller whose effect is currently resolving* against the *target permanent's controller*. Keywords that need this: Progress, Scapegoat (cause ≠ OwnEffect), Retaliation (cause = Battle), Mephistomon-style OnDeletion riders, Mercurymon-style zone-return immunity.

**What's already there.**
- `Game.effect_source_player: Option<PlayerId>` at [game.rs:211](../../../digimon-engine/src/game.rs) — maintained by `effect_queue.rs` during triggered-effect resolution.
- `Modifier.source_player: PlayerId` at [modifiers.rs:21](../../../digimon-engine/src/modifiers.rs) — every modifier carries who applied it.
- `ReplacementCause` enum (Battle / Effect / SecurityCheck / Other) threaded through `delete_permanent_with_cause` at [combat.rs:2231](../../../digimon-engine/src/combat.rs).

**What's missing.**
- `ReplacementCause` encodes *kind* but not *source controller*. Add a `source: Option<PlayerId>` field so "by opponent effects" filters work without re-reading global state.
- Gate helpers on `Game` that every opponent-mutation entry point can call uniformly: `progress_excludes(target, source)`, `opponent_sourced_mutation(target)`.
- Audit-complete coverage of mutation entry points. Today source is inferred for `delete_permanent_with_effects` only — not for return-to-hand, return-to-deck, de-digivolve, suspend-by-effect, move-to-stack, or negative-DP `modifiers.add` calls.

**Deliverable.**
- Extend `ReplacementCause` with a `source: Option<PlayerId>` field. No new type; existing `ReplacementCause::Battle / Effect / SecurityCheck / Other` variants keep their names and gain the source field.
- Add `Game::progress_excludes(target, source_controller) -> bool` and `Game::opponent_sourced_mutation(target) -> bool`.
- Thread source through: `return_to_hand_with_cause`, `return_to_deck_with_cause`, `de_digivolve_with_cause`, `suspend_by_effect`, and every `modifiers.add` path for negative DP (via a wrapper `modifiers.add_from_effect(source, ...)` or by reading `Game.effect_source_player` inside the add path).
- Expose `ctx.deletion_cause() -> ReplacementCause`, `ctx.was_deleted_by_effect() -> bool`, `ctx.was_deleted_by_opponent() -> bool` on the `OnDeletion` / `WhenWouldBeDeleted` `EffectContext` for consumers like Retaliation, Scapegoat, Mephistomon.

### 4.2 Nested-selection-in-replacement thread

**Problem.** [combat.rs:2213-2229](../../../digimon-engine/src/combat.rs) documents the Task 3 limitation: if a `WhenWouldBeDeleted` / `WhenWouldLeaveBattleArea` replacement installs a `PendingSelection::Replacement`, the entry point early-returns and the caller must re-drive. Single-stage self-targeted replacements (Barrier, Evade, Decode) work because they don't need a target selection. But:

- **Save** selects which own permanent to place self under.
- **Decoy** selects which ally permanent to redirect deletion to (color-filtered).
- **Fortitude** selects which Digimon to play self under from trash when an ally deleted (and self must have sources in trash).
- **Fragment(N)** selects N digivolution sources to trash from self's own stack.
- **ArmorPurge** selects which digivolution source to trash (N=1 specialization of Fragment).
- **Partition** selects one card from each of two digivolution groups.

All six share the pattern: a `WhenWouldBeDeleted` (or `WhenWouldLeaveBattleArea`) replacement fires, parks a selection, receives a choice, performs a mutation, then cancels the original deletion. Without clean re-drive, the auto-install path can't be authored.

**Deliverable.**
- Promote the Task 3 limitation to a supported flow: `delete_permanent_with_cause` stores the parked deletion intent on `Game`; when the selection callback fires and returns `ReplacementOutcome::Cancel`, the intent is discarded; when it returns `None` or `Handled`, the intent is re-driven past the already-fired replacement.
- Re-drive must skip replacements that already ran once for this deletion (so Barrier-then-Evade doesn't double-fire Barrier).
- Guarantee idempotency for pure-cancel replacements; document the "process mutates state before outcome" pitfall for authors.

### 4.3 Native-keyword auto-install thread

**Problem.** `cards/keyword_effects.rs::keyword_to_auto_effect` emits declarative `Effect` values for keywords that work "just by being printed" (Barrier, Evade, Decode today). Everything else — SecurityAttackPlus(N), DeDigivolve(N) printed form, DrawX(N), and the selection-bearing Save / Decoy / Fortitude — still requires a hand-rolled `CardEffect`. This forces card authors to replicate the keyword semantics per card and defeats the point of native parsing.

**Deliverable.**
- Extend `keyword_to_auto_effect` to emit the full matrix. After this spec lands, a card with only printed keywords should need zero hand-rolled `CardEffect` code.
- Factor shared primitives as `Effect` builder helpers (`Effect::security_attack_change(N)`, `Effect::de_digivolve_active_skill(N)`, `Effect::draw_option(N)`).
- For selection-bearing keywords (Save, Decoy, Fortitude, Fragment, ArmorPurge, Partition, MaterialSave): one auto-install per keyword, each depending on the nested-selection thread (§4.2).

## 5. Phased plan

Phases are gated on the substrate threads they require. Each phase ends with `cargo test` green + behavioral tests covering the new keyword(s). No phase proceeds until the previous phase is merged.

### Phase A — "Shipped-wrong" revert + easy wins (no new infra)

Unblocks alpha testing and prevents more cards from being scripted against the incorrect Progress site.

- **A1. Progress — partial fix.** Revert the `SecuritySkillDrain` gate in [combat.rs:1762-1774](../../../digimon-engine/src/combat.rs). Add `Game::progress_excludes(target, source) -> bool` gated on `has_keyword(target, Progress) + target.is_attacking + source != target.controller`. Apply at selection filters (`select_opponent_permanent`, `select_any_permanent`, multi-select predicates in `effect_context/selections.rs`). Mutation-site coverage lands in Phase B.
- **A2. Jamming — widen.** Add `has_keyword(attacker, Jamming)` check in `resolve_pending_battle` before the DP-loss `delete_permanent_with_effects(attacker)` branch — mirrors the existing security-battle branch at [combat.rs:1816](../../../digimon-engine/src/combat.rs).
- **A3. SecurityAttackPlus / Minus auto-install.** Extend `keyword_to_auto_effect` to emit a declarative `SecurityAttackChange(N)` modifier effect for `Keyword::SecurityAttackPlus(N)` / `Keyword::SecurityAttackMinus(N)`.
- **A4. Enum cleanup.** Drop dead variants after grep-confirming zero consumers:
  - `Keyword::Blast` — Blast Digivolve runs through `Effect::blast_digivolve` flag, not this variant.
  - `Keyword::Armor` — no DCGO counterpart.
  - `Keyword::Material` — name collision with DCGO's `MaterialSave`.
  - `ModifierType::GrantBarrier` — mis-mapped slot for Fortitude. Rename to `GrantFortitude`, or drop entirely until a granted form is needed.
- **A5. Save / MaterialSave split.** Introduce `Keyword::MaterialSave(u8)`. Stop aliasing `"MaterialSave"` → `Save` in `parse_printed_keywords`. Auto-installs for both come in Phase C.
- **A6. Update `DCGO_KEYWORD_PARITY.md`** with Phase A landings and correct the Iceclad description (RULES_CONTEXT 16-34 says Iceclad is digi-card-count-instead-of-DP, not "immunity to suspension").

**Exit criteria.** All ✅ and 🟡-marked rows in the parity doc now accurate; Progress selection-filter gating verified against Digital Gate Open + Mega Death behavioral tests.

### Phase B — Source-attribution substrate (§4.1)

- **B1. Extend `MutationCause`.** Add `source: Option<PlayerId>`. Update the `delete_permanent_with_effects` inference at [combat.rs:2199-2207](../../../digimon-engine/src/combat.rs) to populate source from `effect_source_player`.
- **B2. Thread cause through zone operations.** Add `_with_cause` variants for return-to-hand, return-to-deck, de-digivolve-N, trash-by-effect, suspend-by-effect. Existing callers default to a best-inferred cause; new callers pass explicitly.
- **B3. Guard negative-DP `modifiers.add`.** Route negative-DP adds through a helper that checks `progress_excludes`. Skip the add (no-op, log at debug) rather than partially apply. Document that Progress is a hard gate, not a "may prevent" prompt.
- **B4. Progress mutation-site coverage.** Apply `progress_excludes` at every opponent-sourced mutation entry point: delete, return, bounce, de-digivolve, suspend, move-to-stack, negative DP.
- **B5. `OnDeletion` cause discriminator.** Expose `ctx.deletion_cause()` / `ctx.was_deleted_by_effect()` / `ctx.was_deleted_by_opponent()` on the deletion-observer `EffectContext`. Unblocks Retaliation, Scapegoat, Mephistomon.

**Exit criteria.** Every opponent-mutation entry point audited and gated. Behavioral tests cover: Progress + opponent negative-DP (skipped), Progress + opponent delete-by-cost (skipped), Progress + own-sourced delete (applies normally). `ctx.was_deleted_by_effect()` verified on a test card's OnDeletion branch.

### Phase C — Nested-selection-in-replacement substrate (§4.2)

- **C1. Lift Task 3 limitation.** Implement parked-deletion re-drive. Store a `DeletionIntent { handle, cause, replacements_fired: SmallSet<EffectId> }` on `Game`; replace the current "early-return, caller re-drives" contract with an engine-managed continuation.
- **C2. Author-facing API.** Document the `ctx.select_<whatever>().then(|h| { ctx.<do_thing>; ctx.cancel_leave(); })` builder pattern for selection-bearing replacements. Verify it composes cleanly inside `WhenWouldBeDeleted` closures.
- **C3. Regression coverage.** Existing Barrier / Evade / Decode behavioral tests must still pass unchanged. Barrier-then-Evade ordering must not double-fire Barrier.

**Exit criteria.** Selection-bearing replacements can be authored without ad-hoc park-and-resume plumbing in each card. A worked example (Save or Decoy) passes end-to-end against a hand-written test card.

### Phase D — Alpha-tier keyword wire-ups (depends on B + C)

Auto-installs for keywords blocked on the substrate threads. Ordering within the phase follows the alpha blast-radius ranking in DCGO_KEYWORD_PARITY.md §"Gap ranking".

- **D1. Fragment(N).** `WhenWouldBeDeleted` replacement selecting N digivolution sources from self to trash. Unblocks the Rocks archetype.
- **D2. ArmorPurge.** N=1 specialization of Fragment; share the underlying `trash_own_sources(N)` primitive.
- **D3. Save.** `WhenWouldBeDeleted` replacement selecting one of controller's own Tamers to place self under as bottom source.
- **D4. Decoy.** `WhenWouldBeDeleted` replacement, triggered on *ally* deletion (not self), redirecting deletion to self by color filter. Color parameter lives on the `Keyword::Decoy` variant.
- **D5. Fortitude.** `OnAllyDeletion` observer playing self from trash free + unsuspended when source count gate passes.
- **D6. Partition.** `WhenWouldLeaveBattleArea` (cause ≠ Battle) two-group selection; plays one card from each group from own stack without cost.
- **D7. MaterialSave(N).** Active-skill emission: main-phase `Effect` moving up-to-N of own stack sources under another permanent (selection-bearing).

### Phase E — Missing-from-enum backfill

Enum variants + auto-installs for alpha-relevant missing keywords. Ordering follows §"Missing-keyword backfill priorities" in the parity doc.

- **E1. Retaliation.** `Keyword::Retaliation` + auto-installed `OnDeletion` effect that checks `ctx.was_deleted_by_effect() == false` and deletes the battled opponent Digimon. Hard blocker for Dark Masters (BT15-077, BT15-079).
- **E2. Scapegoat.** `Keyword::Scapegoat` + `WhenWouldBeDeleted` replacement (cause ≠ OwnEffect per RULES_CONTEXT 16-31) selecting another own permanent to delete instead.
- **E3. DeDigivolve(N) printed-form auto-install.** Existing `Keyword::DeDigivolve(N)` variant gets an active-skill auto-emit via `keyword_to_auto_effect`. Consumes the existing `ctx.de_digivolve_n` helper.
- **E4. DrawX(N) printed-form auto-install.** `[Main]` active-skill draw for Option cards.

### Phase F — Remaining keyword backfill (lower archetype blast radius)

Lower priority than A-E but in scope. Each needs a new enum variant plus the specific primitives called out below.

- **F1. Execute.** Trigger-type at end of your turn (RULES_CONTEXT 16-37). Optional attack; attack may target unsuspended Digimon; self deletes at end-of-attack.
  - `Keyword::Execute` + `keyword_to_auto_effect` emits an `EndOfYourTurn` triggered effect that grants `MayAttack` + `CanAttackUnsuspended` for the window and installs an `OnEndOfAttack` observer that calls `ctx.delete_permanent(self)` with `cause = OwnEffect`.
  - Depends on: already-existing `MayAttack` modifier (Phase 5), `CanAttackUnsuspended` modifier, `OnEndOfAttack` observer timing. `EndOfAttack` already exists in `EffectTiming` per [enums.rs:134](../../../digimon-engine/src/enums.rs). No new timings required.
- **F2. Iceclad.** Passive (RULES_CONTEXT 16-34). Compare digivolution-card count instead of DP in battle (except vs Security Digimon). Higher count wins; same count = both combatants delete.
  - `Keyword::Iceclad` + new battle-resolution branch in `resolve_pending_battle`. When either combatant has Iceclad, swap the DP-compare for a `card_sources.len()` compare; tie path routes both to `delete_permanent_with_effects` with `cause = Battle`. Security battle branch is unaffected (RULES 16-34 exception).
  - No auto-install from `keyword_to_auto_effect`; consumption is a hard-coded combat-resolution branch gated on `has_keyword(combatant, Iceclad)`.
  - Corrects the incorrect "immunity to suspension" description in DCGO_KEYWORD_PARITY.md (flagged in A6; implementation here).
- **F3. MindLink.** Active-skill (RULES_CONTEXT 16-27). Place a Tamer with this effect in the digi cards of a Digimon that has no Tamer cards in its digi cards. Mandatory processing; optional timing under `[Main]`.
  - `Keyword::MindLink` + new primitive `ctx.attach_tamer_to_digimon(tamer_handle, digimon_handle)` — adds the Tamer as a digivolution source with `stack_kind: Tamer` marker. Selection filter excludes Digimon whose sources contain any Tamer card.
  - New `Permanent` helper `has_tamer_source() -> bool` backed by iterating `card_sources` and checking `card_type == CardType::Tamer`.
  - `keyword_to_auto_effect` emits a `[Main]` active-skill effect on Tamers with `<Mind Link>`.
- **F4. Training.** Active-skill (RULES_CONTEXT 16-40). Optional "by suspending this Digimon during main phase" (also active in breeding area). Places top deck card at bottom of self's digi cards face-down.
  - `Keyword::Training` + new primitive `ctx.place_deck_top_under_self_face_down(perm)` — pops deck[0] and appends to `perm.card_sources` at index 0 (bottom) with a face-down marker. Face-down handling matches the approximation noted in [engine-gaps](../../RUST_ENGINE_GAPS.md) "Face-Down Card Tracking" (Resolved 2026-03-14): DCGO's `IsFlipped` is Security-only, so we use a `source_face_down: bool` flag on the new `CardSource` entry.
  - `keyword_to_auto_effect` emits a `[Main]` active-skill with cost `Suspend(self)`. Must also be emittable from breeding area — extend the main-phase active-skill emission site to permit breeding-area consumption of this specific keyword.
  - Open question: does DCGO's Training allow activation of a suspended Digimon's `<Training>` (presumably no, since the cost is "suspend this Digimon")? Verify against DCGO source before landing.

**Exit criteria for F.** Every DCGO KeyWordEffects file has a matching Rust enum variant + consumer; the DCGO_KEYWORD_PARITY.md summary table contains no 🔴 or ❌ rows.

## 6. API surface changes

### Enum changes
- `Keyword` — drop `Blast`, `Armor`, `Material`; add `MaterialSave(u8)`, `Retaliation`, `Scapegoat`, `Execute`, `Iceclad`, `MindLink`, `Training`. Final enum matches the 28 DCGO KeyWordEffects files 1:1 (modulo the printed/granted split on Barrier → Fortitude).
- `ModifierType` — rename `GrantBarrier` → `GrantFortitude` (if any consumer exists) or drop.
- `ReplacementCause` in `replacement.rs` — add `source: Option<PlayerId>` field. Variants stay the same.

### `Game` helpers
- `progress_excludes(target: PermanentHandle, source: Option<PlayerId>) -> bool`
- `opponent_sourced_mutation(target: PermanentHandle) -> bool`
- `deletion_intent: Option<DeletionIntent>` (internal, for nested-select re-drive)

### `EffectContext` extensions
- `ctx.deletion_cause() -> ReplacementCause`
- `ctx.was_deleted_by_effect() -> bool`
- `ctx.was_deleted_by_opponent() -> bool`
- `ctx.cancel_leave()` (already exists; confirm semantics for nested-select re-drive)

### `Effect` builder additions
- `Effect::security_attack_change(n: i8)` — already partially exists; confirm for auto-install emission.
- `Effect::de_digivolve_active_skill(n: u8)` — thin wrapper over existing `ctx.de_digivolve_n`.
- `Effect::draw_option_active_skill(n: u8)`.
- `Effect::end_of_turn_self_delete_attack()` — Execute auto-install body.
- `Effect::attach_tamer_to_digimon_active_skill()` — MindLink auto-install body.
- `Effect::suspend_and_bottom_deck_face_down_active_skill()` — Training auto-install body.

### `EffectContext` / primitive additions
- `ctx.attach_tamer_to_digimon(tamer, target)` — new zone operation for MindLink.
- `ctx.place_deck_top_under_self_face_down(perm)` — new zone operation for Training.
- `Permanent::has_tamer_source() -> bool` — filter helper for MindLink target validation.

### `keyword_to_auto_effect` extensions
Emits declarative effects for: `SecurityAttackPlus(N)`, `SecurityAttackMinus(N)`, `DeDigivolve(N)`, `DrawX(N)`, `Save`, `MaterialSave(N)`, `Decoy`, `Fortitude`, `Fragment(N)`, `ArmorPurge`, `Partition`, `Retaliation`, `Scapegoat`, `Execute`, `MindLink`, `Training`. `Iceclad` is hard-coded in combat resolution, not emitted as an effect.

## 7. Testing strategy

All phases TDD. Each keyword gets at least one `DebugRunner` behavioral test under `digimon-engine/tests/` covering:

1. The positive case (keyword fires when its condition holds).
2. The gated case (keyword does not fire when gate fails — e.g. Progress ignores own-sourced mutations).
3. A multi-instance / stacking edge case where RULES_CONTEXT specifies one (e.g. Barrier multiple instances per 16-24-4).

**Specific test cards (hand-written, not real printed cards):**
- `TEST-PROGRESS-A` — passive `<Progress>`, attacks; exercised against opponent negative-DP effect, opponent delete-by-cost, own-sourced delete.
- `TEST-JAMMING-A` — passive `<Jamming>`, low DP, attacks higher-DP defender; verifies no attacker delete on DP loss.
- `TEST-SEC-ATTACK-PLUS` — printed `<Security A. +1>`, no hand-rolled script; expects 2 security attacks.
- `TEST-MATERIAL-SAVE` — printed `<Material Save 2>` Digimon with ≥2 stack sources; active skill moves 2 sources under a Tamer.
- `TEST-RETALIATION` — `<Retaliation>` Digimon; verifies only fires on battle deletion.
- `TEST-SCAPEGOAT` — `<Scapegoat>` Digimon; verifies no fire on own-sourced deletion.
- `TEST-EXECUTE` — `<Execute>` Digimon at end-of-turn; verifies attack-unsuspended permitted and self-delete fires at end of attack.
- `TEST-ICECLAD` — two `<Iceclad>` combatants with different stack counts; verifies higher-count survives; equal-count path deletes both.
- `TEST-MINDLINK` — `<Mind Link>` Tamer attaching to a Digimon with no Tamer sources; verifies target filter excludes Digimon already carrying a Tamer source.
- `TEST-TRAINING` — `<Training>` Digimon activating from main phase + from breeding area; verifies suspend cost, deck-top-to-bottom-face-down placement, and rejection when already suspended.

**Parity regression.** `cargo test --manifest-path digimon-engine/Cargo.toml` must remain green across all phases. `tests/test_rust_backend_parity.py` with `DIGIMON_BACKEND=rust` must remain green (or receive documented diff rows in `RUST_PYTHON_PARITY.md` for Progress semantics).

## 8. Documentation updates

Each phase ends with a matching doc update — same commit or immediate follow-up:

- **`DCGO_KEYWORD_PARITY.md`** — flip rows to ✅ as phases land. This is the canonical keyword parity tracker.
- **`RUST_ENGINE_GAPS.md`** — mark `WhenWouldBeDeleted framework extensions` and `OnDeletion cause discriminator` and `Play / digivolve origin context flag` as resolved once Phase B + C land.
- **`RUST_PYTHON_PARITY.md`** — add row for Progress semantics divergence (Python skips SecuritySkill, Rust after Phase A does not). This row stays until Python retirement.
- **`RUST_ENGINE_API.md`** — document the selection-bearing replacement builder pattern from Phase C.

## 9. Risks and trade-offs

| Risk | Mitigation |
|---|---|
| **Coverage gap on Progress mutation sites.** Auditing every opponent-mutation entry point is grep-heavy and easy to miss. | Phase B1-B4 is audit-structured: enumerate mutation entry points in `effect_context/`, `combat.rs`, `modifiers.rs` in a worksheet before gating; add `debug_assert!` at each gated site during development. |
| **Nested-select re-drive ordering bugs.** Barrier-then-Evade could fire Barrier twice if `replacements_fired` tracking misses an effect ID. | Phase C3 regression tests cover stacked-keyword ordering explicitly. Idempotency invariant: pure-cancel replacements are safe to re-fire; stateful processes are not and must set their outcome atomically. |
| **Enum variant removal breaks external callers.** Dropping `Keyword::Blast` / `Armor` / `Material` is a breaking change for anything pattern-matching exhaustively. | Grep for each variant across `digimon-engine/` + `digimon-engine-py/` + `src-tauri/` + Python bindings before removal; all matches must be dead code. The Rust compiler catches the rest. |
| **Python-side cards authored against today's Rust Progress semantics** (the wrong SecuritySkill gate). | Phase A reverts the gate before any new cards script against it. Check Notion board and `validated_cards_rust.json` for recent cards touching Progress; none expected. |
| **`ImmunityToOpponentEffects` modifier duplicates `progress_excludes`.** The modifier was added as the "granted form of Progress" but now both code paths must check it. | Unify: `has_keyword(p, Progress)` returns true if the permanent has printed Progress *or* an `ImmunityToOpponentEffects` modifier. Single gate helper reads through `has_keyword`. |
| **Iceclad description mismatch between parity doc and RULES_CONTEXT.** Parity doc says "immunity to suspension"; RULES_CONTEXT 16-34 says digi-card-count-vs-DP. | Phase A6 fixes the parity doc; Iceclad itself is Phase F (post-alpha). Verify against DCGO source when the implementation phase arrives. |

## 10. Open questions

- **Decoy color parameter.** `Keyword::Decoy` in the current enum is unparameterized, but DCGO Decoy is color-filtered (16-17). Before Phase D4 lands, decide: `Decoy(Color)` variant, or keep unparameterized and carry color on the granted `Effect`.
- **MaterialSave on non-DigiXros cards.** RULES_CONTEXT 16-20 says "X cards from digi cards (specified in DigiXros requirements)". For non-DigiXros cards that print `<Material Save N>`, is the source set simply "any N stack sources"? Resolve by cross-checking DCGO's `MaterialSave.cs` before Phase D7.
- **Fortitude source-count gate.** RULES_CONTEXT 16-26 says "Digimon with digi cards and this effect is deleted". Parity doc says "if sources available". Confirm the exact gate (≥1 source? any source type?) against DCGO before Phase D5.
- **Iceclad scope.** RULES_CONTEXT 16-34 says "Compare number of digivolution cards instead of DP in battle". Unclear whether Iceclad applies when only *one* combatant has it (attacker's Iceclad imposes the comparison mode) or requires both sides. Verify against DCGO `Iceclad.cs` before Phase F2.
- **Training activation on suspended self.** Cost is "suspend this Digimon", so presumably unsuspended-required, but verify against DCGO `Training.cs` before Phase F4.
- **MindLink [Main] timing.** RULES_CONTEXT 16-27 says "Mandatory (but if [Main], player chooses timing)". Confirm whether the keyword always prints with a `[Main]` timing tag, or whether it can print without one (which would make activation automatic). Verify against DCGO + printed card samples before Phase F3.

## 11. Deliverable sequencing

```
Phase A  (no new infra)                          → ~1 week
Phase B  (source-attribution)                    → ~1 week     [depends on A]
Phase C  (nested-select re-drive)                → ~1-2 weeks  [independent of B; parallel]
Phase D  (alpha keyword wire-ups)                → ~2 weeks    [depends on B + C]
Phase E  (missing-enum backfill: alpha-critical) → ~1 week     [depends on B]
Phase F  (remaining backfill: Execute, Iceclad,  → ~2 weeks    [F1/F3/F4 depend on B;
           MindLink, Training)                                   F2 independent after A]
```

Total critical path: A → B → D ≈ 4 weeks. Phase C parallel to B. Phase E parallel to D. Phase F sequencing:
- F2 (Iceclad) has no cross-dependency on B/C/D and can start right after A lands.
- F1 (Execute) depends on B5 (`ctx.was_deleted_by_effect` for the self-delete path's OnDeletion observers to see the correct cause).
- F3 / F4 depend on their new primitives only; can run parallel to D once B ships.

With parallelism: the full spec can complete in ~5 weeks of critical-path work plus parallel tracks.

## 12. Out-of-scope follow-ups

Tracked but not owned by this spec:
- Python engine retirement (gates final removal of `RUST_PYTHON_PARITY.md` rows).
- Rust `batch-fix-cards` skill (blocks migrating existing scripted cards to consume the new auto-installs).
- Observer timings refactor (sibling gap; loosely related to the `OnDeletion` cause discriminator).
