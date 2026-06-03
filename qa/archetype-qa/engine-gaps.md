# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-06-02
Last sweep: 2026-05-17 (Phase 2 rollup — Tracks A–J, PR #480)

### §DSL-loaded fixtures have empty `evo_costs` (G-DSL-FIXTURE-EVO-COSTS) — RESOLVED 2026-06-02

- **First seen / RESOLVED** 2026-06-02. `DebugRunner`/`dsl_card`-loaded cards are materialized by `code/digimon-engine/src/debug_runner.rs::card_data_from_compiled`, which hardcoded `evo_costs: Vec::new()`. YAML `alt_paths` lower into a separate alt-path registration (`src/dsl_cards/lower_alt_path_registration.rs`), NOT into `CardData.evo_costs`. So `Game::effect_initiated_digivolve` with `ignore_requirements: false` (`src/game_actions.rs::effect_initiated_digivolve_from_source_inner`) — which matches the digivolve base against the evolving card's `evo_costs` table — found no matching row for any DSL-loaded card, and a cost-reduced Delay/Option "digivolve into a hand card with cost reduced by N" effect **silently no-opped** (the body stayed in hand). In production this works because `data/cards.json` carries `evo_costs` (e.g. EX11-022: `[{card_color: 2, level: 4, memory_cost: 4}]`); the gap was DebugRunner-only.
- **Fix:** `card_data_from_compiled` now backfills `CardData.evo_costs` from the compiled `alt_paths` — each `kind: digivolve` path with direction `From` (the default), a `from: { level_eq, color_is }` filter, and a literal `cost` becomes one `EvoCost { card_color, level, memory_cost }` row (color mapped via the new `compiled_color_to_evo_code` to the `cards.json` int convention). Trait-only / formula-cost / direction-`Into` alt-paths are not static evo-cost rows and are still resolved through the alt-path registration machinery — so EX11-022's Lv.3 Puppet alt-path is (correctly) excluded, exactly matching `cards.json`. No DSL widening or engine primitive was needed; this was a test-harness materialization gap.
- **Pinned by:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral ex11_022` — `ex11_022_dsl_fixture_backfills_printed_evo_cost_table` (the backfilled row equals the production `cards.json` table), `ex11_022_effect_initiated_cost_reduced_digivolve_onto_matching_base_succeeds` (an effect-initiated, cost-reduced digivolve now resolves and pays the reduced memory), and `ex11_022_effect_initiated_digivolve_onto_nonmatching_base_is_rejected` (precision: a non-matching base is still rejected). Both positive tests were verified to FAIL with the backfill reverted.

### §Medusamon EX11-012 token-shield deletes own tokens only (G-EX11-012-TOKEN-SHIELD-OWN-ONLY) — RESOLVED 2026-05-30

- **First seen / RESOLVED** 2026-05-30, Medusamon faithfulness pass. EX11-012 Medusamon's `[All Turns]` shield — "When this Digimon would leave the battle area, by deleting 1 Token, it doesn't leave." — was authored with `select_own_permanent: { filter: { kind: token } }` (controller's OWN tokens only). But Medusamon's `[When Digivolving]`/`[End of Attack]` clause gives the Petrification Token to the **opponent** (`play_token: { controller: opponent }`), and the deck has no own-token source — so the shield could never fire in real play. Card text says "1 Token" with no owner qualifier, and DCGO `EX11/Red/EX11_012.cs` (would-leave region) uses `CanSelectPermanentCondition(p) => p.IsToken` via `HasMatchConditionPermanent(...)` with `selectPlayer: card.Owner` — the controller may delete ANY token (in practice the opponent's gifted Petrification Token, whose own `[On Deletion]` then trashes the opponent's top security).
- **Fix:** localized YAML change in `code/digimon-engine/cards/ex11/EX11-012.yaml` — the would-leave cost selector is now `select_any_permanent: { filter: { kind: token } }` (scans BOTH battle areas; selecting player = controller). The any-owner permanent selector with a kind filter already existed in the DSL (`select_any_permanent`, runtime `install_select_any_permanent` in `src/dsl_cards/step/selections.rs`), so **no DSL widening was needed** — this was not a substrate gap.
- **Pinned by:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes medusamon -- --include-ignored` (`medusamon::ex11_012_survives_by_deleting_opponents_petrification_token`: Medusamon survives + opponent's Petrification Token consumed + opponent security −1) and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral ex11_012` (7/7, no regression).

## Open gaps — judge-quiz faithfulness suite discovery wave (2026-05-29)

Surfaced by `openspec/changes/add-judge-quiz-faithfulness-suite` (TCG-Judges' rules quiz reproduced as behavioral tests). Discover-then-pin: tests assert the judge-correct outcome; a failure is logged here, not weakened.

### §No general state-based ≤0-DP rules-check (G-NO-GENERAL-ZERO-DP-RULES-CHECK) — RESOLVED 2026-05-29

- **RESOLVED** by change `fix-judge-quiz-engine-gaps` (Gap 1). `run_rule_check_after_arts` was generalized to `Game::run_state_based_rules_check` (deletes every battle-area Digimon at `effective_dp ≤ 0` via the batched-deletion flow, skipping transiently-empty/zombie slots) and invoked at the outermost `drain_effect_queue` boundary: BETWEEN each top-level queued effect (so a Digimon driven ≤0 by one effect is deleted before the next queued trigger — Q24) AND a final fixpoint sweep, guarded by `Game::effect_drain_depth` so it never fires mid-effect (Q6/Q13/Q14). The unfaithful inline mid-effect ≤0-DP deletion in `EffectContext::add_modifier` (the "latent 17-1-2-2 timing bug") was removed. Pinned by the un-ignored `zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` + synthetic `q6_analog_no_mid_effect_deletion_within_single_effect` / `q24_analog_rules_check_deletes_between_queued_effects` regression tests; full suite regression-clean. Full resolution note in [qa/resolved-gaps.md](../resolved-gaps.md). Cluster-B per-card scenarios (Q6/Q8/Q13/Q14/Q24) flip to PASS as their cards are authored.

### §`<Blast Digivolve>` skips effect-immunity (G-BLAST-DIGIVOLVE-IMMUNITY) — RESOLVED 2026-05-29

- **First seen / RESOLVED** 2026-05-29, judge-quiz Q18. Neither `execute_blast_digivolve` (combat.rs) nor the Blast DNA field-target generator (`dna_digivolve.rs::valid_blast_dna_field_targets_for_hand_card`) consulted the effect-immunity machinery; a Digimon immune to ALL Digimon effects including its own (Quantumon LM-020 — `CannotBeAffected` with `EffectControllerFilter::Any`) could still `<Blast Digivolve>`, but `<Blast Digivolve>` is itself a Digimon effect. **Fixed** (change `fix-judge-quiz-cluster-wiring-gaps`): gate Blast counter-candidate collection (`combat.rs::try_enter_counter`) AND the Blast DNA field-target generator on `permanent_is_unaffected_by_effect(base, base.player, Digimon)`, plus a defensive abort in `execute_blast_digivolve`. Pinned by `combat::counter_interrupt::blast_target_immune_to_own_effects_is_not_a_counter_candidate`. (Q18 stays BLOCKED-CARD on LM-020 for the end-to-end pin; the substrate is closed.)

### §DigiXros material-consumption fires no leave trigger (G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER) — OPEN

- **First seen:** 2026-05-29, judge-quiz Q25 (probe). When a battle-area Digimon is consumed as a DigiXros material, `Game::take_digixros_material_origin` (game_actions.rs, `DigiXrosMaterialOrigin::BattleArea` arm) silently `battle_area.remove(idx)`s it and tucks its top card under the new Digimon — firing NO `OnLeaveField` / `WhenWouldLeaveBattleArea` trigger, unlike the standard deletion path (`combat.rs` stages 1 + 6). The judge rule (Q25): a DigiXros departure DOES count as "leaving the battle area" (and is NOT "leaving by battle").
- **DCGO-verified design (2026-05-29).** The observer that must fire is **BT17-095 Miraculous Mega Knight** (already implemented), NOT EX3-014. Its relevant clause is a `[Delay]` keyed off DCGO `EffectTiming.WhenRemoveField` with `CanUseCondition = CanDeclareOptionDelayEffect && CanTriggerWhenPermanentRemoveField(IsOwnerPermanentCondition) && !IsByBattle` — "When one of your level-6 [Greymon]/[Garurumon] **would leave the battle area outside of a battle**". The Rust BT17-095 YAML models this as `kind: replacement` / `trigger: when_would_leave_battle_area` with `active_when: { ..., none_of: [replacement_cause: battle] }`. So the engine must fire the **`WhenWouldLeaveBattleArea` REPLACEMENT WINDOW** (the deletion-batch stage-1 path, `combat.rs::run_replacement_stage(WhenWouldLeaveBattleArea, ...)`) — NOT a post-removal `OnLeaveField` observer — for each `BattleArea` DigiXros material BEFORE it is consumed, with a `ReplacementCause` that is **not `Battle`** (candidate: new `ReplacementCause::DigiXros`, or `Cost`).
- **Why this is DigiXros-transaction surgery (not simple wiring):** `commit_digixros_material_sources` → `take_digixros_material_origin` must run the replacement window per battle-area material and then handle the OUTCOME — and BT17-095 does NOT proceed: on success it **redirects/substitutes** the leaving WarGreymon into a DNA-digivolve into Omnimon (the material is consumed into Omnimon instead of into Dorbickmon's DigiXros). So the DigiXros commit must tolerate a material being cancelled/redirected/substituted away mid-transaction. Build + validate this as the substrate step of EX3-014 authoring with BT17-095 as the integration oracle (DCGO `BT17_095.cs`, `EX3_014.cs`).
- **Blocks (judge-quiz):** Q25 (BLOCKED-CARD on EX3-014 regardless).

### §No digivolve-target restriction modifier (G-DIGIVOLVE-TARGET-RESTRICTION) — ENGINE SUBSTRATE DONE 2026-05-29; DSL-install + card at authoring

- **Engine substrate IMPLEMENTED** 2026-05-29 (change `fix-judge-quiz-cluster-wiring-gaps`). `ModifierType::CanOnlyDigivolveInto` added (carries the allowed name in `ModifierPayload::Name { value }`); consulted by `Game::digivolve_target_blocked_by_restriction`, wired into the single central digivolve-route function `normal_digivolve_route_for_card` (feeds the digivolve action mask, the Blast counter path, and hand-digivolve execution) AND the arts-digivolve path (`game_actions.rs`). A base carrying the modifier offers NO digivolve route into a non-matching card; no-op when absent (zero blast radius for existing cards). Registered in `modifier_map.rs` (lookup + exhaustiveness + all_variants), `validator::KNOWN_MODIFIER_KEYS`, and the `payload_matches_modifier` guard. Pinned by `dna_digivolve::tests_q3_digivolve_target_restriction::{can_only_digivolve_into_blocks_nonmatching_name, no_restriction_is_a_noop}`. Full suite regression-clean.
- **Remaining (deferred to EX10-020 authoring):** a DSL step/keyword to INSTALL `CanOnlyDigivolveInto` with a card-specific allowed name as a declarative aura sourced from the battle area (so it's breeding-inactive for free — see breeding-area note below). The allowed name is card-specific ("Apocalymon"), so this thin lowering lands with EX10-020 (cluster G — NOT first wave). The `ChangeBaseCardName` Name-payload-aura lowering is the template.
- **First seen:** 2026-05-29, judge-quiz Q3 (probe). The `ModifierType` enum has `CannotDigivolve` (the source can't digivolve at all) but NO "can only digivolve INTO [X]" / "cannot digivolve into [X]" digivolve-TARGET restriction. EX10-020's `[All Turns] This Digimon can only digivolve into [Apocalymon]` (a self-restriction on its own digivolution targets) has no primitive.
- **Breeding-area note (NOT a gap):** the Q3 judge ruling turns on this `[All Turns]` being INACTIVE while EX10-020 is in the breeding area. The probe confirmed continuous/aura effects already enumerate sources from `battle_area` only (`aura.rs::snapshot_player_battle_area` / `snapshot_all_battle_areas`), never `breeding_area` — so a restriction modelled as a declarative aura sourced from EX10-020 is automatically inactive in breeding. No breeding-isolation gap.
- **DCGO-verified design (2026-05-29).** EX10-020 (`EX10_020.cs`) models the `[All Turns]` via `CardEffectFactory.CanNotDigivolveStaticSelfEffect(cardCondition: cs => !cs.EqualsCardName("Apocalymon"), condition: () => IsExistOnBattleArea(card), ...)` — a continuous SELF restriction: "this Digimon cannot digivolve INTO a card whose name ≠ Apocalymon", active only while EX10-020 is on the battle area. Engine design: add `ModifierType::CanOnlyDigivolveInto` (or `CannotDigivolveInto`) carrying a name predicate (reuse `ModifierPayload::Name { value }` for the allowed name); install it as a declarative aura sourced from EX10-020 (aura sources are scanned from `battle_area` only — `aura.rs` — so the restriction is automatically INACTIVE while EX10-020 is in breeding, matching the `IsExistOnBattleArea` gate AND the Q3 judge ruling); consult it at digivolve-target legality. **Consult-site note:** `Game::can_digivolve(card, perm)` takes a `&Permanent` (no handle), so the modifier consult must be added either by threading the base handle into `can_digivolve` (7 callers) or via a sibling `digivolve_target_allowed(base_handle, card)` called at the battle-area digivolve sites (action mask `mask.rs:282` region + the hand/arts digivolve execution in `game_actions.rs`); breeding digivolve sites need no consult (the base isn't an aura source there). Add DSL vocab to install "can only digivolve into [name]". Low-risk additive (no existing card installs this modifier). Close as the substrate step of EX10-020 authoring (cluster G — NOT first wave).
- **Blocks (judge-quiz):** Q3 (BLOCKED-CARD on EX10-020 / BT12-057 regardless).

### §On-trash inherited effect fires synchronously, blocking remain-in-trash gating (G-ON-TRASH-OBSERVER-SYNCHRONOUS) — **WITHDRAWN 2026-05-30 (mischaracterized; moved to `resolved-gaps.md`)**

- **WITHDRAWN — not a gap.** This entry asserted a "+3 over-count" (on-trash observers fire synchronously, can't defer to re-check remain-in-trash) blocking Q23. Running the real 3-source-trash-then-return-2 scenario **to completion** (2026-05-30) disproved it: the engine already produces the judge-correct **+1**. When ≥2 sources are trashed mid-effect, their mandatory `OnDigivolutionCardTrashed` observers form a multi-trigger bundle → the drainer installs a `TriggerOrder` selection that PARKS them past the trashing effect (the return-2 runs first); on resolution each observer's clause condition is RE-EVALUATED and the cards returned in the meantime fail (no longer in trash) → dropped. The earlier evidence was the SINGLE-source probe (which does fire synchronously) plus abstract reasoning about deferral — the multi-source scenario was never run end-to-end (the probe stopped at the first `pending_selection`).
- **Q23 → PASS** via `d_activation_site::q23_inherited_trash_memory_gated_on_remaining_in_trash` (synthetic Medusamon driver over real EX8-051/EX8-005). No engine change was made; the deferral "fix seam" / split-out follow-up change is retired.
- **Residual narrow open question (no known card; not a blocker):** a SINGLE source trashed then returned WITHIN one effect would still fire synchronously (a lone mandatory trigger doesn't park). If a real card ever needs that gated, re-open then. The single-source synchronous behavior is characterized by `d_activation_site::cluster_d_on_trash_observer_fires_synchronously_not_deferred`.
- Full resolution note in [`resolved-gaps.md`](../resolved-gaps.md).

<!-- §Return-trash-to-deck-bottom ignores Digi-Egg routing (G-RETURN-TRASH-DIGI-EGG-ROUTING)
     RESOLVED 2026-05-29 by change `fix-judge-quiz-engine-gaps` (Gap 2). Full
     resolution entry moved to qa/resolved-gaps.md. -->

## Closures (2026-05-29)

- **ST5 Machine Black attack-history/blocker context** — CLOSED. The engine now
  tracks per-player Digimon attack counts for the current turn and resets them
  at turn start, enabling the DSL `digimon_attacked_this_turn` predicate used by
  ST5-04 ToyAgumon and ST5-06 Greymon. ST5-14 Tai Kamiya required no new
  player-visible action contract: existing Blocker target-change context is
  faithful once the blocker declaration path suspends the blocker before
  target-change observers resolve. Detailed closure and verification commands
  are archived in [qa/resolved-gaps.md](../resolved-gaps.md).

## Closures (2026-05-24)

- **Mid-attack `<Security A. +N>` not recomputed** — CLOSED. The
  player-attack security-check loop (`resolve_player_security_loop` +
  `drive_security_resolution`'s `DisposeFinalize` arm in
  [`code/digimon-engine/src/combat.rs`](../../code/digimon-engine/src/combat.rs))
  used to snapshot the attacker's effective `<Security A.>` once at
  attack declaration and decrement that count. DCGO re-reads
  `Permanent.Strike` every iteration. Exposed by [BT21-001] Gigimon's
  `on_opponent_security_removed` inherited that may digivolve an
  attacker into [BT21-029] Medusamon mid-attack — Medusamon's
  `<Security A. +1>` was ignored by subsequent checks. Closed by change
  [`fix-security-check-recompute-mid-attack`](../../openspec/changes/fix-security-check-recompute-mid-attack/).
  Regression test:
  [`code/digimon-engine/tests/mid_attack_security_attack_recompute.rs`](../../code/digimon-engine/tests/mid_attack_security_attack_recompute.rs).

## Sweep notes (2026-05-24 — Xros Heart DigiXros closure)

The `close-xros-heart-digixros-gaps` change closes the reusable engine
substrate that blocked the first Xros Heart acceptance pool. `DigiXrosTransaction`
now covers recipe material selection, cost deltas before payment, post-payment
source attachment, transaction-local origin allowances, pre-attached materials,
and `digixros_count`; `<Material Save>` now uses deletion snapshots and filters
eligible sources through the carrier's DigiXros recipe. Production YAML and
behavioral tests landed for BT10-009, BT10-013, BT10-087, and BT12-112.

The follow-up `author-xros-heart-reusable-primitives` change closes the next
reusable layer: under-Tamer card flow, generalized source movement and
leave-battle rescue, scoped DigiXros wildcard substitution, and effect-created
attack prompts. Production YAML now covers BT21-083, BT11-095, P-224,
BT19-090, BT21-092, BT10-111, BT21-027, and BT19-061 without `raw_rust`.

The `complete-xros-heart-authoring-substrate` follow-up also closes the
reveal-pool free-play sub-shape: `EffectContext::play_from_reveal_free` and
DSL `choose_from_reveal destination: play_free` now route selected revealed
cards through free play with reveal-origin rollback. `BT19-008` is the Xros
Heart production proof for this primitive.

The same follow-up now closes the stack-derived metric slice: DSL
`source_color_count` lowers as both a source-relative formula and
base/per/delta selector, and `source_stack_count` counts predicate-matched
source cards for count bounds and effect math. These compose with existing
`source_dp`, no-source filters, and `lowest_material_count`. BT19-014, AD1-006,
AD1-013, BT19-026, BT21-030, and BT20-037 are production YAML proofs for
source-color DP math, current-DP comparison, fewest-source ties, De-Digivolve
payoff selection, no-source targeting, and per-level-6-source suspend/memory
counting.

The temporary lockout slice now has production proof on BT19-038 and BT20-037:
permanent-scoped `CannotActivateWhenDigivolvingEffects`,
`CannotActivateOnPlayEffects`, and `CannotUnsuspend` modifiers suppress only
their named timing/phase behavior and expire at the printed
end-of-opponent-turn duration. BT19-051 and BT19-035 round out the same fixture
batch with return-protection/DP and played-Xros-Heart observer coverage.

Remaining Xros Heart work in this tracker should be card-specific authoring or
non-Xros-specific residual primitives discovered by later cards, not a generic
"no DigiXros transaction / under-Tamer flow" engine gap.

The follow-up Xros Heart card-authoring pass added production YAML and focused
behavioral tests for BT10-003, BT10-029, BT19-033, and BT19-047. The
same-effect DP modifier visibility primitive that blocked BT19-012 is now
closed: permanent DP predicates clear printed DP checks from the delegated
card-field pass and evaluate field targets through `effective_dp`. BT19-012
and BT21-011 should proceed as card-authoring follow-up unless their focused
production tests prove a narrower residual.

## Sweep notes (2026-05-17 — Phase 2 rollup)

10 Phase 2 tracks landed in PR #480 (`claude/musing-ishizaka-c4b355` against
`main`). The substrate-side closures in this shadow tracker:

- **Track B** — `Effect::activation_cost(...)` builder hook +
  `ctx.suspend_self_as_cost` / `ctx.return_self_to_deck_bottom_as_cost`
  helpers landed. Cost failure consumes OPT slot per Working Rule §17.
- **Track C** — `G-OPT-TRIGGERED` and `G-OPT-RESET-VIA-ATTACK-CYCLE`
  diagnosed as already-closed (phantom + test-setup misdiagnosis). 23
  stale `#[ignore]` annotations removed. The G-OPT-RESET entry below is
  already marked CLOSED at §348.
- **Track D** — `enqueue_from_permanent` digivolution-stack walk
  completed (already RESOLVED 2026-05-15 per
  `docs/RUST_ENGINE_GAPS.md`); Track D added the dedicated regression
  test and un-ignored 18 dependent behavioral tests. G-WHEN-DIGIVOLVING-DISPATCH
  absorbed.
- **Track G** — EX11-054 (entering-permanent observer) migrated to
  Track B's `activation_cost`. The G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER
  entry below at §208 retains an updated footer for that card; the
  underlying entering-permanent predicate gap remains open for other
  observer cards.
- **Track H** — BeforePayCost substrate extensions, plus
  `G-PLAY-FROM-HAND-FREE-BIND-AS` (already marked CLOSED at §325).
- **Track I** — PUPPETS-G009 Standard Delay [Main] activation closure
  (substrate now exposes the `[Main]` activation action through normal
  action mask). End-of-attack mandatory self-delete chain closed for
  EX4-074 ShineGreymon: Ruin Mode (no engine changes — existing
  primitives suffice).

See [qa/resolved-gaps.md](../resolved-gaps.md) for full per-track closure
details. The Phase 2 rollup also closed many DSL-only gaps tracked in
[qa/dsl-vocab-gaps.md](../dsl-vocab-gaps.md).

## Sweep notes (2026-05-15)

Post-rebaseline audit cleanup driven by
[`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../../docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md):
the canonical engine-side tracker
[`docs/RUST_ENGINE_GAPS.md`](../../docs/RUST_ENGINE_GAPS.md) was shrunk
from ~50 open entries to ~22, with ~54 entries (the 8 audit-flagged
CLOSED + ~46 NARROW closed-core halves) relocated to
[`qa/resolved-gaps.md`](../resolved-gaps.md). The narrowed residual
sub-shapes (e.g. `play_from_revealed_free`, `play_from_security_at`,
top-N security trash + face-up flip, bilateral `UntilLeaveField`
delivery for BT14-009, `pop_top_digivolution_source` for BT24-093)
live as their own entries.

This shadow tracker remains consistent with the canonical engine-side
gap document — the per-entry status updates here already cited the
2026-05-08 and 2026-05-10 closures that the audit confirmed as
closed-at-substrate. No engine code, tests, or card YAML were
modified by the sweep.

## Sweep notes (2026-05-14)

Cross-referenced every entry against PRs #459–#473 and the per-archetype
DSL gap input documents in `qa/archetype-qa/dsl/`. New closures since the
2026-05-10 sweep:

- **Track H aura system (PR #467):** typed `AuraScope` / `AuraGrant`
  builder API, security-zone aura tick dispatch, and queue-based granted-
  triggered-effect dispatch with parked-selection support. Closes the
  "Granted triggered ability", "Named-target declarative aura", and
  "Declarative aura sourced from security zone" entries in
  `docs/RUST_ENGINE_GAPS.md` at the substrate level — entries remain
  🟡 PARTIAL pending the body-registry cleanup optimization and the
  query-time aura model follow-up.
- **Alter-S Ladder DSL (PR #468):** EX9-021 Omnimon Alter-S and DNA
  Omnimon ladder cards landed on existing zone-movement / replacement /
  source-selection substrate. No new engine gap surfaced.
- **Formula thresholds (PR #470):** validates the Track J
  formula/result substrate slice on real card-shaped fixtures
  (BT15-096, BT21-102). No new substrate.
- **Puppet DSL observers (PR #472):** PUPPETS-G011 closed; observer
  fan-out and `OnAnyDeletion` event-target predicates are exercised by
  card-shaped fixtures (BT22-002, BT22-088, EX9-033, EX11-023, ST19-14).
  No new engine timing required.

No new engine gaps surfaced from the per-archetype DSL gap input
documents in `qa/archetype-qa/dsl/`. The shadow tracker in this file
remains consistent with the canonical engine-side gap document
[docs/RUST_ENGINE_GAPS.md](../../docs/RUST_ENGINE_GAPS.md).

## Sweep notes (2026-05-10)

Cross-referenced every entry against PRs #449–#458. Below is the closure
index — what landed in each PR and which entries it narrows or closes.
Entries already noted "RESOLVED" / "PARTIALLY RESOLVED" with PR-cited
test commands stay as-is. New closures since the previous sweep:

- **Track B replacement framework (PR #449):** replacement-effect framework
  scaffold landed; consumed by Track C/D consult sites (e.g.
  `WhenWouldLeaveBattleArea`, `WhenWouldBeReturnedToHand`,
  `WhenWouldPlaceInSecurity`).
- **Track D combat centralization (PR #450):** `Game::begin_attack_open` is
  the central entry for natural / Vortex / Overclock / effect-created
  attacks. Closes "fixed attack target" and "non-switchable attack
  target" gap shapes; `CannotSwitchAttackTarget` /
  `CannotBeRedirectedAsAttackTarget` consult sites are wired (PR #452).
- **Track A event payload (PR #451):** `ProvenanceToken` system + typed
  event-payload contract; consumed by Track E zone helpers' source
  attribution.
- **Track C foundation (PR #452):** modifier taxonomy publication +
  10 fully-wired consult sites: `MayAttackPlayerOnly`,
  `CannotSwitchAttackTarget`, `CannotBeRedirectedAsAttackTarget`,
  `CannotMove`, `DisableEffect`, `CannotAddMemory`, `CannotAddSecurity`,
  `ImmuneFromStackTrashing`, `CanAttackTargetDefendingPermanent`,
  `ImmuneFromDPMinus`. New `Expiry` variants
  (`EndOfYourTurn`, `OnceUsed`, `UntilCondition`) typed.
- **Track E zone movement (PR #453):** 8 zone-movement helpers + the
  owner-routing fix (`CardSource.owner` consulted in `return_to_hand`
  and `return_to_deck_inner`). The dormant fix now has live coverage
  via `tests/owner_routing_live.rs` (added by this sweep). Closes:
  "Forced opponent hand reduction primitive", "Effect-played permanent
  cleanup provenance" (superseded by Track A `ProvenanceToken`),
  "Zone-manipulation: security stack operations" (significantly
  expanded), "Zone-manipulation: return-to-hand / return-to-deck /
  bounce self".
- **Track E DSL verbs (PR #454):** the 10 deferred zone-movement DSL
  verbs are now expressible end to end. Demote `raw_rust` carve-out
  notes pointing at these verbs; see the DSL-verb table in
  `qa/dsl-vocab-gaps.md` for the per-verb closure.
- **Track C deferred modifiers (PR #455):** `ModifierEntry` /
  `PlayerModifierEntry` carry typed `ModifierPayload`;
  `Permanent::synth_identity` centralizes identity overlays. Wires
  `ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`,
  `ChangeCardNamesForDigiXros`, `TreatAsDigimon`,
  `ChangePermanentLevel`, `ChangeCardDP`, `ChangeOriginDP`,
  `ChangeSAttack`, `ChangeEndTurnMinMemory`, `ChangeLinkCost`,
  `ChangeLinkMax`, `CannotPlayFromTrash`, bilateral
  `CannotReducePlayCost`, `OpponentCannotReduceDigivolveCost`. The
  Track C entry above is updated.
- **Track G keyword library close (PR #457):** Evade printed-semantics
  (suspend-and-cancel, not deck redirect); Decoy color-filter via
  `Keyword::Decoy(u8)` bitmask payload; Progress card-shape backfill;
  Digi-Burst documented as not auto-installed. Decoy trait-filter
  remains open.
- **UntilCondition continuous controller (PR #458):** runtime
  evaluation/eviction for `Expiry::UntilCondition`. The Zephagamon
  status-condition entries that referenced "needs UntilCondition
  controller" are now substrate-complete; remaining work is per-card
  predicate authoring.

For the canonical engine-side gap status, see
[docs/RUST_ENGINE_GAPS.md](../../docs/RUST_ENGINE_GAPS.md). The
per-archetype `qa/archetype-qa/dsl/*.md` rollups also received sweep
markers in this batch.

## Open / Partial Gaps

Resolved engine gaps have been moved to [qa/resolved-gaps.md](../resolved-gaps.md). This file tracks only open gaps and partial slices with remaining follow-up work.

### Same-effect DP modifier visibility in subsequent `dp_lte` selections
> Moved to [qa/resolved-gaps.md](../resolved-gaps.md#same-effect-dp-modifier-visibility-in-subsequent-dp_lte-selections--2026-05-24).
> BT19-012 remains unauthored, but the reusable primitive is no longer an
> active engine blocker.

### Track C modifier payload/identity consults — PARTIALLY RESOLVED 2026-05-09
- **Discovered in:** Puppets / Royal Knights / Olympos / DigiXros readiness passes.
- **Card(s):** Cards that print "this Digimon is also [Trait]", "treat this Tamer as a Digimon", DigiXros name aliases, Security Attack changes, end-turn memory floors, and Link cost/max adjustments.
- **Status update:** `ModifierEntry` and `PlayerModifierEntry` now carry typed `ModifierPayload`; `Permanent::synth_identity` centralizes field identity overlays. Consults are wired for trait/name/color overlays, DigiXros aliases on permanents, TreatAsDigimon, permanent level overrides, printed/origin DP overlays, Security Attack adjustments, end-turn memory floors, Link cost/max, `CannotPlayFromTrash`, bilateral `CannotReducePlayCost`, and `OpponentCannotReduceDigivolveCost`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat track_c_deferred_modifiers -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers`.
- **Remaining work:** structured DSL payload parsing for string/list/profile payloads; `ChangeCardLevelForAssembly` consult once cast-time assembly selection exists; broader card-shaped fixtures for each printed family.

### OnPlaceSecurity / Added-to-Security Observer Payload — PARTIALLY RESOLVED 2026-05-08
- **Discovered in:** TS Olympos / Dark Masters timing backlog.
- **Card(s):** BT14-033 Patamon, BT8-090 Kari Kamiya, and any "when a card is added to security" observer.
- **Status update:** Effect-driven `place_on_security` now fires `OnPlaceSecurity` after the card reaches security. Payload carries `event_card`, `affected_player`, `source_player`, `EventCause::SecurityPlacement`, and a moved-card set into `Zone::Security`; fan-out scans the affected player's battle area and breeding slot once each. DSL `when: on_place_security` and printed alias `when: on_added_to_security` lower to the same timing and support event predicates such as `event_card_trait_has`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_place_security_fires_once_with_security_placement_payload`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_place_security_event_card_trait_predicate_matches_placed_card`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_added_to_security_alias_uses_place_security_payload`.
- **Remaining work:** Card-shaped production tests for Patamon/Kari-style observers plus recovery/setup multi-card addition proof. The separate `OnDiscardSecurity` self-trigger path is tracked below as resolved for effect-driven security-to-trash movement.

### OnDiscardSecurity Self-Trigger — RESOLVED 2026-05-08
- **Discovered in:** TS Olympos security-discard backlog.
- **Card(s):** BT13-106 Odin's Breath and sibling "when an effect trashes this card from security" cards.
- **Status update:** `EffectTiming::OnDiscardSecurity`, `Effect::on_discard_security`, DSL `when: on_discard_security`, and `TriggerSource::SecurityDiscarded` are wired. Effect-driven `trash_top_security` fires the trashed security card's own timing with event cause/source payload; normal attack security checks do not.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- discard_security`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_discard_security_event_cause_predicate_matches_effect_trash`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_106`.
- **Remaining work:** Card-local authoring for full BT13-106 Main-effect activation and other printed bodies; the reusable dispatch primitive is closed.

### Activate Another Card's When Digivolving Effect — PARTIALLY RESOLVED 2026-05-08
- **Discovered in:** Jesmon (2026-03-17); Puppets/Nyabootmon assessment (2026-04-28)
- **Card(s):** BT10-112 Jesmon GX, BT10-110 Seiken Meppa, BT22-042 Nyabootmon
- **Effect text:** BT10-112 / BT10-110: "Activate 1 of that card's [When Digivolving] effects as an effect of this Digimon." BT22-042: "[All Turns] [Once Per Turn] When any of your other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects."
- **Status update:** The reusable Rust/DSL refire primitive now exists as `EffectContext::refire_effect_from_permanent(source, "when_digivolving", optional)` for Puppet self-refire and `EffectContext::refire_target_effect(target, TimingFilter::Either, selecting_player, bypass_once_per_turn)` for Homeros-style cross-card permanent refire. YAML `refire_effect` supports `timing: when_digivolving` and `timing: on_play_or_when_digivolving`. It enumerates refireable effects, preserves grantor source identity, keeps carrier semantics on the target permanent, respects once-per-turn slots, and exposes visible choices when needed.
- **Remaining missing for Puppets:** Closed for the Puppet self-refire shape as of 2026-05-08. `OnAnyDeletion` carries a pre-removal deleted-object snapshot to Rust observers, including inherited-stack observers, and DSL event-target predicates read snapshot owner/kind/trait data after removal. `BT22-002` proves the inherited Token/other-Puppet draw fixture, including Token kind matching, carrier exclusion, and once-per-turn suppression. EX11-060 proves Overclock-specific deletion payloads and DSL `event_cause: overclock`. `BT22-040` proves the "your other Digimon" deletion refire fixture with visible optional refire, source exclusion, opponent suppression, and once-per-turn lockout. `BT22-042` now proves the same refire contract against a non-trivial `[When Digivolving]` body whose optional play branch resumes into the mandatory DP tail.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_040 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_042 --nocapture`.
- **Remaining non-Puppet work:** BT10-112 / BT10-110 still need the foreign-card variant that activates another card's `[When Digivolving]` effect as the source Digimon's effect.

### Event-Gated Delay Activation Windows [G-DELAY-EVENT-GATED]
- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust engine delayed-option state, action mask, and DSL lowering.
- **Card(s):** BT22-098 Unique Emblem: Fable Waltz, P-229 Unique Emblem: Narrative Ronde.
- **Effect text:** BT22-098: "[Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay> ... 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3." P-229: "[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay> ... 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3."
- **Status:** **RESOLVED 2026-05-21.** The `on_suspend` slice closed 2026-05-02 (BT22-098); the `on_ally_played` slice closed 2026-05-21 (P-229). Delayed Option permanents store `DelayTrigger::OnEvent(_)` plus placement turn and park indefinitely; Delay activation is gated until after the placement turn before trashing itself through the replacement-aware cost path. DSL `kind: delay` lowers `trigger: on_suspend` / `on_unsuspend` / `on_ally_played` to `DelayTrigger::OnEvent(_)` with body-level `active_when` event predicates.
- **Closed via:** DSL — `lower_delay.rs` maps `CompiledTiming::OnAllyPlayed` → `DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed)`. Engine — `effect_queue.rs` `enqueue_triggered` fans `TriggerSource::EnteredField` dispatches (the `OnEnterFieldAnyone` / `OnAllyPlayed` play broadcasts) out to `enqueue_event_gated_delayed_options`; previously only `EventObserved` / `AttackTargetChanged` reached it.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- event_gated_delay_only_fires_after_placement_turn_and_matching_event`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_event_trigger_lowers_to_on_event_delay`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_098`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_229` (13 tests, 0 ignored).
- **Updated 2026-05-08:** Self-scoped suspend observers can use `event_permanent_is_source: true` to compare the suspended event permanent with the observer source permanent. BT23-077 Sistermon Ciel uses this to avoid over-firing when another own permanent suspends. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_077`.

### Deletion Observer Optionality Not Exposed to Agent
- **Discovered in:** Chaos Control (2026-04-10)
- **Card(s):** EX1-066 — Analog Youth, ST6-14 — Matt Ishida
- **Effect text:** "you may suspend this Tamer" / "you may suspend this Tamer to gain 1 memory"
- **What's missing:** `_fire_deletion_observers` (game/__init__.py:1128) auto-fires effects when conditions pass, ignoring `is_optional`. The DCGO `ActivateClass` offers the player a decline choice (`canNoSelect: true`) before the coroutine runs. In the Python engine, "you may" effects fire automatically with no agent choice to decline.
- **Suggested change:** When `effect.is_optional` is True, create a branch selection (accept/decline) before calling `on_process_callback`. This would expose the choice to the RL action space.
- **Workaround:** Scripts use condition gates (e.g., `perm.is_suspended`) that prevent re-activation, effectively limiting to once per event. The auto-fire behavior is functionally correct but removes the agent's ability to strategically decline (e.g., keeping tamer unsuspended for a later, more valuable deletion).

### `[All Turns]` (Both-Player) Filter on Triggered Clauses  [G-ALL-TURNS-FILTER]
- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (2026-04-27).
- **Scope:** DSL.
- **Card(s):** BT24-018, BT21-029, BT24-016, BT21-025 — every card with `[All Turns]` triggered clauses.
- **What's missing:** `active_when: { all_turns: true }` parses but the predicate evaluator may not actually allow firing on the opponent's turn (uncertain — needs verification). Tests for opp-turn triggers are #[ignore]'d pending verification.
- **Workaround:** Use `active_when: { all_turns: true }` and confirm via behavioral test on opp's turn.

### ~~`trash_security_card` Verb (Non-Top Security) Missing~~  [G-TRASH-SELECTED-SECURITY] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). `EffectContext::trash_security_card(player, handle)` trashes a chosen security card by stable handle; the `trash_selected_security` DSL verb consumes a `select_security` binding. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (2026-04-27).
- **Scope:** DSL + engine (hybrid).
- **Card(s):** BT24-018 — "[When Digivolving] You may trash any 1 of your opponent's security cards."
- **What's missing:** `select_security` can bind a target index but no DSL verb consumes that binding to actually trash the chosen card. Only `trash_top_security` exists. The engine likely has the primitive (security indexing already supported elsewhere); just no DSL bridge.
- **Workaround:** `raw_rust:` escape hatch.

### ~~Trash → Deck-Bottom Move (Without Reveal Phase)~~  [G-ZONE-TRASH-TO-DECK] — RESOLVED 2026-05-21
- **Status:** RESOLVED. Confirmed during the Medusamon re-attempt run (BT24-017 Batch 2, 2026-05-21). The first-class DSL verb `return_trash_list_to_deck_bottom` exists (`code/digimon-dsl/src/step.rs`, lowered in `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`) and consumes a bound card-list, calling the real engine API `EffectContext::return_trash_cards_to_deck_bottom` (`code/digimon-engine/src/effect_context/mod.rs`), which removes the selected cards from trash and inserts them at deck index 0 (bottom). BT24-017's `[When Digivolving]` "return 2 cards from their trash to the bottom of the deck" sub-clause is now fully implemented in pure DSL with a real `CountCappedMultiSelect` player choice. The stale entry was carried as open in both this file and `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype, BT24-017 Medusamon (Batch 3, 2026-04-27).
- **Scope:** DSL + engine (hybrid).
- **Card(s):** BT24-017 (return 2 trash to bottom of deck), BT21-029-related, EX11-012 (return 1 trash to bottom).

### `on_digivolve` Trigger Context Missing Newly-Digivolved Permanent Reference  [G-ON-DIGIVOLVE-TRAIT-FILTER]
- **Discovered in:** Medusamon archetype, BT24-082 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT24-082 Owen Dreadnought — "[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn."
- **Effect text:** "When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon … that Digimon gets +3000 DP"
- **What's missing:** `on_digivolve` fires via `TriggerSource::PlayerBattleArea(pid)` in `game_actions.rs`, which sets every permanent's effect as an observer. When constructing the `TriggerContext`, `target_permanent` is set to the observer permanent (the tamer itself), NOT the permanent that just digivolved. Therefore: (a) a trait filter on the newly-digivolved card ("digivolve INTO a Reptile/Dragonkin") cannot be expressed in the condition predicate, and (b) the DP-modifier target ("that Digimon") cannot be bound to the newly-digivolved card.
- **Suggested change:** Add a `digivolve_target: Option<PermanentHandle>` field to `TriggerSource::PlayerBattleArea` (or a sibling `DigivolveTarget` variant). Populate it in `fire_on_digivolve` with the permanent that just completed digivolution. Thread it through to `TriggerContext::target_permanent` for each observer's effect dispatch, or add a distinct `digivolve_target` field to `TriggerContext` so observer effects can reference both "the observer" and "the card that digivolved".
- **Workaround:** `any_permanent` condition over own battle area with `trait_has: Reptile/Dragonkin` (over-fires if a matching ally is on board but a non-matching Digimon digivolved). `select_own_permanent` prompt for DP modifier target (player picks instead of auto-targeting). Two tests `#[ignore]`'d.
- **Updated 2026-04-29:** Normal battle-area `Game::digivolve_from_hand` now dispatches `OnDigivolve` via `TriggerSource::Digivolved { player, permanent, card }`, and `TriggerContext.event_permanent` / `event_card` identify the just-digivolved permanent and new top card. `event_card_trait_has` is proven against the new top card by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card`, and `target: event_target` binding is proven to affect the just-digivolved permanent by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`. Keep breeding-area digivolve as an open follow-up unless separately tested.
- **Updated 2026-05-08:** Effect-initiated digivolve now uses the same `Digivolved` payload and additionally sets `TriggerContext.effect_initiated = true`, enabling DSL `event_is_effect_initiated` gates. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_028`.
- **Updated 2026-05-08:** DNA digivolve now carries `TriggerContext.dna_origin = true` through scoped `WhenDigivolving` / `OnDnaDigivolve` drains and the global `OnDigivolve` payload; standard digivolve sets it false. Effect-initiated DNA also sets `TriggerContext.effect_initiated = true` on the global payload. `EffectReadContext` / `EffectContext` expose `event_dna_origin()`, and DSL `dna_origin: true` gates on the same payload. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3_dna_digivolve_triggers` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078_when_digivolving`.

### `OnEnterFieldAnyone` Observer Context Missing Entering-Permanent Reference  [G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER]
- **Discovered in:** Medusamon archetype, EX11-054 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-054 Owen Dreadnought — "[All Turns] When your Digimon are played or digivolve, if any of them have the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>. After, 1 of your Digimon with <Progress> gets +3000 DP."
- **Effect text:** "When your Digimon are played … if any of them have the [Reptile] or [Dragonkin] trait"
- **What's missing:** `OnEnterFieldAnyone` fires via `TriggerSource::PlayerBattleArea(pid)` in `game_actions.rs`. `trigger_context_for_source` for this variant iterates every permanent in `pid`'s battle area and sets `target_permanent = source_permanent` (the OBSERVER). The entering permanent's handle is never threaded into `TriggerContext`. An observer like Owen Dreadnought therefore cannot inspect the traits of the card that just entered — `event_target_trait_has` evaluates Owen's own traits, not the entrant's.
- **Related gap:** G-ON-DIGIVOLVE-TRAIT-FILTER (same limitation for `on_digivolve`). Both share the same root cause: the trigger source variant doesn't carry the triggering permanent's handle.
- **Suggested change:** Add `entering_permanent: Option<PermanentHandle>` to `TriggerContext` (alongside existing `target_permanent`). Populate it in `game_actions.rs::broadcast_on_enter_field_anyone` (and the digivolve broadcast) with the handle of the card that just entered/digivolved. Add a matching `entering_permanent_trait_has` DSL BoolPredicate leaf in `predicate.rs` that reads `ctx.trigger_context.entering_permanent`.
- **Workaround:** `kind: raw_rust` no-op placeholder (`ex11_054_all_turns_noop`). See `qa/dsl-vocab-gaps.md` entry `G-ENTERING-PERMANENT-TRAIT`.
- **Updated 2026-05-17 (Phase 2 Track G):** EX11-054 specifically migrated off the `ex11_054_all_turns_noop` workaround. The [All Turns] clause now uses Track B's `activation_cost: { suspend_self: true }` to gate the body via the single-trigger drainer model, and the previously-failing `ex11_054_all_turns_suspends_and_draws_when_reptile_ally_played` test passes. The underlying entering-permanent trait-filter gap remains open for other observer cards that need an entering-permanent predicate beyond what `event_card_trait_has` covers; this card was unblocked through a different shape.
- **Updated 2026-04-29:** Normal hand-played battle-area permanents now dispatch `OnEnterFieldAnyone` via `TriggerSource::EnteredField { player, permanent, card }`, and `TriggerContext.event_permanent` / `event_card` identify the entering permanent and card. `event_card_trait_has` is proven against the entering card by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card`. Keep token play, option placement, play-from-trash context, and breeding-area observer fan-out as open follow-ups unless separately tested.
- **Updated 2026-05-08:** Effect-created battle-area permanents now use `EnteredField` with `TriggerContext.effect_initiated = true`, while normal player-action play sets it false. BT16-028 proves effect-play vs normal-play gating with `event_is_effect_initiated`.
- **Updated 2026-05-08:** Provenance-token helpers are available for effect-created play/digivolve flows. `play_from_hand_free_with_provenance`, `effect_initiated_digivolve_with_provenance`, and `effect_initiated_dna_digivolve_with_provenance` return a token keyed to the physical card instance, and `resolve_provenance_token` follows it across battle-area index shifts and zone moves. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- provenance_tokens`.
- **Updated 2026-05-08:** Printed timing vocabulary now includes `Effect::on_any_digimon_played(card)` and DSL `when: on_any_digimon_played`. Both lower to the existing `OnEnterFieldAnyone` dispatcher and use the same `EnteredField` event payload, avoiding overlapping fan-out. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- new_effect_timings_are_constructible` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_any_digimon_played_alias_uses_enter_field_payload`.

### `GameEvent::Digivolve` Not Emitted  [G-GAME-EVENT-DIGIVOLVE]
- **Discovered in:** Medusamon archetype, EX11-054 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-054 Owen Dreadnought (digivolve half of [All Turns] trigger); any card that would use the event log to detect digivolves.
- **Effect text:** "When your Digimon … digivolve, if any of them have the [Reptile] or [Dragonkin] trait …"
- **What's missing:** `GameEvent::Digivolve` is defined in `events.rs` as "for future wiring — not emitted yet." Even if an observer could use raw_rust to read `ctx.game.events`, the digivolve-detection path is unavailable. Blocks raw_rust workarounds for G-ON-DIGIVOLVE-TRAIT-FILTER that try to infer "which permanent just digivolved" via the event log.
- **Suggested change:** Emit `GameEvent::Digivolve { player, permanent: PermanentHandle }` inside the digivolve execution path (wherever `fire_on_digivolve` is called). This unblocks event-log-based raw_rust workarounds until the full TriggerContext fix lands.
- **Workaround:** None — raw_rust event-log detection blocked until emission is wired.
- **Updated 2026-04-29:** `Game::digivolve_from_hand` now emits `GameEvent::Digivolve { player, top_card_id, field_index, from_stack_top }` after stack mutation. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index`. Effect-initiated digivolve, DNA digivolve, and breeding-area digivolve event-log coverage remain open.

### Outer-Tail Steps Lost When Inner `select_hand` Has No Candidates  [G-SELECT-EMPTY-OUTER-TAIL]
- **Discovered in:** Medusamon Batch 8, BT21-024 Cyberdramon side-fix (2026-04-27)
- **Card(s):** BT21-024 Cyberdramon — opponent places hand card as bottom security, then top security trashed.
- **Effect text:** "they place 1 card from their hand as the bottom security card. Then, trash their top security card."
- **What's missing:** When `select_hand` is called inside an `as_selecting_player` body and there are no valid candidates (empty hand), `install_select_hand` returns early without installing a `PendingSelection`. `try_install` still returns `true` (the variant was matched), so `run_steps` returns `RunOutcome::Parked`. `as_selecting_player` propagates `Parked`, and `park_outer_tail` parks subsequent sibling steps in `dsl_outer_tail`. Since no selection was ever installed, the selection callback never fires, and `drain_dsl_outer_tail` is never called — outer-tail steps are permanently lost.
- **Affected pattern:** Any YAML where `as_selecting_player { body: [select_hand, ...] }` is followed by sibling steps, and the opponent may have an empty hand. The sibling steps after `as_selecting_player` are silently skipped in the empty-hand scenario.
- **Suggested change:** When `install_select_hand` detects `valid_action_ids.is_empty()` and `optional=true`, it should run the callback synchronously with a sentinel `NO_SELECTION` index (or call `drain_dsl_outer_tail` directly) rather than just returning. For `optional=false` with an empty hand, the current silent-skip behavior may be acceptable — but `drain_dsl_outer_tail` should still fire.
- **Workaround:** Move subsequent steps that must fire unconditionally INSIDE the `as_selecting_player` body (at the cost of tying them to the selection resolution). Steps after the body that require unconditional execution in the empty-hand case cannot be expressed in the current DSL. The BT21-024 empty-hand test is `#[ignore]`'d with this gap tag.
- **Updated 2026-04-29:** Empty inner selection handling now preserves the outer tail for `select_material` and the new `select_own_sources` path. Covered by `empty_select_material_runs_outer_tail_synchronously` and `empty_select_own_sources_runs_outer_tail_synchronously`. Other legacy selection installers should use the same "no candidates means no park" pattern when they grow empty-candidate tests.

### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->

### Self-Digivolution-Stack Name Check (triggered clause condition)  [G-SELF-DIGIVOLUTION-CONTAINS-NAME]
- **Discovered in:** Medusamon Batch 11, BT20-102 Omnimon (X Antibody) DSL implementation (2026-04-27)
- **Card(s):** BT20-102 Omnimon (X Antibody) — "[On Play][When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, ..."
- **Effect text:** "If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards" — a condition on the triggering permanent's OWN card stack.
- **What's missing:** `lower_triggered.rs` passes `PredicateSubject::None` to the condition closure when evaluating a triggered clause's `condition:` block. This means no subject-requiring predicate (e.g., a hypothetical `self_digivolution_contains_name`) can evaluate the source permanent's own stack. The condition closure must receive a `PredicateSubject::Permanent(source_h)` where `source_h` is the permanent that fired the trigger. The engine method `Permanent::contains_card_name(name, data)` already exists in `permanent.rs` and scans the full card_sources stack — the gap is the predicate threading, not the engine primitive.
- **Suggested change:** In `lower_triggered.rs`, when building the condition closure, capture the `source_permanent` handle (available from `EffectContext` at fire time) and pass it as `PredicateSubject::Permanent(source_h)` instead of `PredicateSubject::None`. Add a `self_digivolution_contains_name: Option<String>` field to `BoolPredicateSpec` in `digimon-dsl` that evaluates `perm.contains_card_name(name, &game.card_data)` when the subject is a permanent. This is a hybrid gap: engine has the method, DSL+lowering need the predicate leaf + subject threading.
- **Workaround:** Entire boardwipe clause (clause d) routed through `raw_rust: { fn: bt20_102_boardwipe_and_return }` which checks `perm.contains_card_name("Omnimon", ...)` and `perm.contains_card_name("X Antibody", ...)` directly. Over-approximation: top card name "Omnimon (X Antibody)" always contains "X Antibody", so condition is always true for standalone BT20-102 rather than only when a genuine "Omnimon" or "X Antibody" base is in the digivolution stack.

### `for_each` + `delete_permanent` Stale Index After First Deletion  [G-FOR-EACH-DELETE-INDEX-SHIFT]
- **Discovered in:** Medusamon Batch 12, BT8-097 Crimson Blaze DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT8-097 Crimson Blaze — "[Main] Delete all of your opponent's Digimon with 6000 DP or less." Also any card whose `for_each` body includes a `delete_permanent` step and has multiple valid targets occupying ascending battle_area indices.
- **Effect text:** "Delete all of your opponent's Digimon with 6000 DP or less." — automated sweep, no player choice.
- **What's missing:** `permanent_scan::scan` in `src/dsl_cards/step/permanent_scan.rs` produces a snapshot of `PermanentHandle` values (each encoding `{player: u8, index: u8}`) before the `for_each` loop begins. `Player::delete_permanent` uses `Vec::remove(index)` which compacts the `battle_area` Vec in place. After the first deletion of a permanent at index `i`, all permanents at indices `> i` shift down by 1. The stale handle for the second target (originally at index `i+1`) now points to the permanent that was at `i+2` (or is out-of-bounds if the first deletion was the last element). The `field_index >= battle_area.len()` guard in `Player::delete_permanent` silently returns without deleting in the out-of-bounds case. Result: when all N targets need to be deleted and they are at contiguous ascending indices, only the first target is deleted.
- **Affected pattern:** `for_each { over: { all_of: [...] }, body: [delete_permanent] }` with 2+ matching targets sharing the same `player`. The bug is latent in BT9-112's Clause B test (`bt9_112_clause_b_deletes_all_lv4_or_lower_spares_lv5`) — that test passes only because the de-digivolve pass shifts survivor indices, masking which permanent was actually deleted.
- **Suggested change:** Either (a) reverse the scan order so highest indices are deleted first (no index shift affects lower indices), or (b) use a stable permanent identifier (e.g., `card_index: u16` on `CardSource`, which is already unique per card) instead of position-based handles, or (c) after each deletion, re-scan to collect the remaining targets. Option (a) is the lowest-effort fix: reverse `scan`'s output before the `for_each` iteration loop when the body contains a deletion verb, or unconditionally reverse (deletion order does not affect observable game state for mass-delete sweeps).
- **Workaround:** Test `bt8_097_main_deletes_multiple_opp_digimon_with_no_player_choice` is `#[ignore]`'d with this gap tag. The single-target delete test (`bt8_097_main_deletes_opp_digimon_with_dp_lte_6000`) still passes because only one target is in the scan snapshot.

### Breeding-Area Trigger Dispatch Partially Resolved  [G-BREEDING-TRIGGER-DISPATCH]
- **Discovered in:** Royal Knights archetype assessment (2026-04-28)
- **Scope:** Rust engine.
- **Card(s):** BT13-007 King Drasil_7D6 — `[Breeding] [Start of Your Main Phase] Reveal the top card of your Digi-Egg deck, then place that card and all of your [Royal Knight] trait Digimon as this Digimon's bottom digivolution cards.` BT20-083 Omekamon inherited also needs a breeding-area carrier for its opponent-security-removed trigger.
- **Effect text:** any clause whose source permanent is in the breeding area and whose timing fires while it remains there, especially `[Breeding] [Start of Your Main Phase]`, inherited breeding effects, and future effects that explicitly act from breeding.
- **What's missing:** Broader event-trigger fan-out from breeding remains incomplete for timings beyond the phase slice below and the security-removal slice below. Those paths must be wired one timing at a time so a breeding observer is not also reachable through an overlapping battle-area scan.
- **Resolved slice:** `Game::enter_main_phase` now dispatches `StartOfYourMainPhase` through both `TriggerSource::PlayerBattleArea(tp)` and `TriggerSource::PlayerBreedingArea(tp)`. The breeding source uses the stable `BREEDING_TARGET` sentinel handle, `enqueue_from_breeding_permanent`, and existing breeding-source liveness / activation-count paths, so top-card and inherited breeding observers can fire once without pretending the breeding slot is a battle-area index.
- **Card-shaped proof:** BT13-007's DSL now uses `target: source` for its breeding stack, `reveal_top_deck: { zone: digi_egg_deck }` removes from the Digi-Egg deck, and `place_as_bottom_source: { source: { permanent: rk } }` moves the Royal Knight permanent's stack under King Drasil instead of duplicating it.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- start_of_your_main_phase_fans_out_to_battle_and_breeding_once_each --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_007 --nocapture`
- **Resolved slice 2026-05-08:** `TriggerSource::SecurityRemoved` now scans the observer player's breeding slot through `enqueue_from_breeding_permanent`, preserving the removed-security payload and the `BREEDING_TARGET` source permanent for top-card/inherited breeding observers. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_opponent_security_removed_fans_out_to_breeding_inherited_once_with_payload` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_083_inherited_breeding_security_removed_fans_out_once_with_payload`.
- **First test:** place `BT13-007` in player 0 breeding, put one Royal Knight in player 0 battle area, enter main phase, and assert the top digitama plus that Royal Knight are placed under King Drasil while the Royal Knight leaves battle.
- **Workaround:** None for remaining event fan-out slices. Moving King Drasil to battle just to reuse `PlayerBattleArea` would change legal zones and action masks.

### Breeding-Area Pending Selection / Permanent Handles  [G-BREEDING-PERMANENT-SELECTION]
- **Discovered in:** Royal Knights archetype assessment (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** BT20-083 Omekamon — `[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area.` Also BT13-093 Omekamon, BT13-110 Royal Knights of the Purge, BT13-112 Omnimon, EX11-053 Omekamon, and BT23-072 King Drasil_7D6, all of which target or play cards from a breeding-area King Drasil stack.
- **Effect text:** effects that select "your [King Drasil_7D6] in the breeding area" or select cards from that Digimon's digivolution cards.
- **What's missing:** DSL selection lowering for `select_own_permanent` and `select_opponent_permanent` prefilters by iterating `player.battle_area`; the `zone: [breeding]` predicate cannot produce candidates. Runtime `PendingSelection` kinds and action encodings cover battle-area permanents, hands, trash, reveal, security, sources, and count-capped selections, but not a breeding-area permanent. `PermanentHandle { player, index }` currently encodes battle-area vector indices, so the breeding slot needs either a distinct handle form or a dedicated selection kind.
- **Suggested change:** Introduce a stable way to address breeding permanents in selections, such as `PermanentHandle::Breeding { player }` or a new `PermanentRef` enum with `BattleArea(PermanentHandle)` and `Breeding(PlayerId)`. Add an action-mask/decoder path for selecting the breeding slot, then update `select_own_permanent` / `select_any_permanent` prefilters to include it when the compiled predicate allows `CompiledZone::Breeding`.
- **First test:** trigger `BT20-083` On Deletion with a `BT13-007` in breeding and assert a pending selection offers the breeding King Drasil rather than silently doing nothing.
- **Workaround:** None faithful. Auto-selecting the only breeding permanent hides a gameplay choice and fails when future cards offer multiple legal destinations across battle/breeding zones.
- **Updated 2026-04-29:** Resolved for pending selection and DSL binding without fake battle-area handles. `EffectContext::select_own_breeding_permanent` installs `SelectionKind::BreedingPermanent`, masks only the phase-scoped breeding selection action (`encode_breeding_select(player)`), and DSL `select_own_breeding_permanent` binds a `BreedingPermanentRef`. Covered by `breeding_permanent_selection_targets_breeding_without_fake_battle_handle`, `breeding_selection_mask_exposes_only_breeding_select_action`, and `dsl_select_breeding_permanent_binds_target`.
- **Updated 2026-05-20 (Task S1.3):** selecting digivolution *sources* from a breeding-area carrier (King Drasil's resident stack) is now resolved. `select_material` / `select_materials` (`CountCappedZone::Material`) against a `BREEDING_TARGET`-sentinel carrier install a real `pending_selection` whose action IDs use the appended `BREEDING_SOURCE_SELECT` sub-range (`2168..2192`, keyed by carrier owner; `ACTION_SPACE_SIZE` raised 2168→2192). `material_zone_geometry` is the single branch point. This unblocks the source-pick side of BT13-112, BT13-110, EX11-053, BT13-019, BT23-072. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_carrier`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials::select_materials_breeding_carrier`.
- **Updated 2026-05-22 (`close-royal-knights-substrate-gaps`):** optional breeding-permanent selections are now resolved as well. `select_own_breeding_permanent optional: true` exposes PASS, declines without running the remaining tail, and mandatory/no-candidate behavior remains separate. Card-shaped coverage includes BT20-083's optional On Deletion tuck and BT13-110's optional hand-to-breeding-source placement.
- **Remaining limits:** Group 4 covers effect-initiated movement to/from the real breeding slot and bottom-source placement under the `BREEDING_TARGET` selected breeding permanent; Task S1.3 covers selecting sources *within* a breeding-area carrier, and the 2026-05-22 slice covers optional decline. The 2026-05-08 `PlayerBreedingArea` slices cover `StartOfYourMainPhase` and security-removal fan-out while the source remains in breeding; other event fan-outs from breeding remain under `G-BREEDING-TRIGGER-DISPATCH`.

### Option-Placed Observer Timing Missing  [G-OPTION-PLACED-TIMING]
- **Discovered in:** Royal Knights archetype assessment (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** BT13-007 King Drasil_7D6 inherited — `[Breeding] [Your Turn] [Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory.` Royal Knights of the Purge (BT13-110) and The Last Guardian (BT20-100) are common Royal Knights options that need to surface this trigger when placed.
- **Effect text:** "When an Option card with the [Royal Knight] trait is placed in the battle area..."
- **What's missing:** The DSL has `CompiledTiming::OnOptionPlaced`, but `compiled_timing_to_engine` returns `None` for it, and the engine has no `EffectTiming::OnOptionPlaced` variant or dispatch site after Option cards are placed as battle-area permanents. Without a trigger context carrying the placed Option card, predicates such as `event_card_trait_has: "Royal Knight"` cannot be evaluated.
- **Suggested change:** Add `EffectTiming::OnOptionPlaced` and fire it after `dispose_option` / option placement helpers create the delayed/training/field Option permanent. Dispatch should scan relevant observers, including breeding-area sources once `G-BREEDING-TRIGGER-DISPATCH` is fixed, and should set trigger context fields for the placed card, owner, and permanent if one exists.
- **First test:** place `BT13-110` Royal Knights of the Purge into battle while `BT13-007` is in breeding with its inherited effect active, then assert the King Drasil controller gains 1 memory exactly once per turn.
- **Workaround:** None — BLOCKED for the inherited memory trigger. Piggybacking on `OnEnterFieldAnyone` would over-fire for Digimon/Tamers and lacks the Option-specific trait context.
- **Updated 2026-04-29:** Delay-style Option placement through `Game::play_option_from_hand` now dispatches `OnOptionPlaced` via `TriggerSource::OptionPlaced { player, permanent, card }`, and the placed Option is exposed through `TriggerContext.event_permanent`, `event_card`, and `source_player`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option`.
- **Updated 2026-05-02:** Group 5 Task 4 extends `TriggerSource::OptionPlaced` with optional standalone permanent and linked-host context, dispatches `OnOptionPlaced` from Delay, Training, Link, and inherited/security self-placement paths, includes top-card and inherited breeding-area observers in the `OnOptionPlaced` fan-out, resumes `OnLink` after placed-option selections settle, and makes breeding-source `max_per_turn` accounting work for this queued observer path. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- on_option_placed_fires_for_training_link_and_security_placement_with_event_card link_on_option_placed_selection_resumes_on_link_after_choice_resolves on_option_placed_scans_inherited_sources_under_breeding_top_card once_per_turn_breeding_on_option_placed_observer_fires_once_not_zero`. Keep transient Standard options open; they still are not battle-area placements.
- **Group 5 contract note:** Group 5 did not change ACTION_SPACE_SIZE or TENSOR_SIZE. New Link/Delay choices reuse existing pending-selection masks. Task 8 verified the handoff regression set with `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`.

### `OnAllyAttack` / `OnOpponentAttack` Declared-Attack Observer Timing
- **Discovered in:** Dark Masters / Rocks archetype assessments (2026-04-29 follow-up)
- **Scope:** Rust engine runtime context.
- **Card(s):** BT15-008 Muchomon (`OnAllyAttack`-style "when one of your Digimon attacks a player"); EX10-003 Tumblemon and EX8-050 Gogmamon (`OnOpponentAttack`-style defender-side inherited observers).
- **Effect text:** "When one of your red Digimon attacks a player..." / "When one of your opponent's Digimon attacks..."
- **Updated 2026-04-29:** Battle-area declared-attack observers now dispatch from the real combat state machine. `OnAllyAttack` scans the attacker's controller battle area and excludes the attacking permanent; `OnOpponentAttack` scans the defending player's battle area before Alliance/Counter/Block windows. `EffectReadContext` / `EffectContext` expose `attack_attacker()` and `attack_target()` over the live pending attack, with `attack_target()` reporting the effective target after substitution, including accepted optional target substitutions. `PendingAttack::declaration_committed` keeps optional pre-declaration replacement resumes legal while accepted pre-declaration cancel/substitute outcomes mutate the pending attack before declaration commits; `resolve_generic_selection` resumes parked attacks after replacement accept/decline resolution so normal `decode_action` callers cannot strand a pending attack. Post-declaration resumes require the original handle to still be a live attacking permanent. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- declared_attack_fires_ally_and_opponent_observers_with_attack_context on_ally_attack_does_not_fire_on_the_attacker_itself attack_target_context_reports_effective_declared_target_after_substitution accepted_predeclare_cancel_replacement_cancels_before_observers declined_predeclare_replacement_resumes_attack_declaration accepted_predeclare_target_substitution_updates_attack_context attack_resume_after_trigger_order_does_not_alias_removed_attacker on_ally_attack_still_fires_if_attacker_stack_changes_during_on_attack on_ally_attack_does_not_fire_if_attacker_left_during_on_attack on_opponent_attack_does_not_fire_if_ally_observer_removes_attacker`, plus `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- on_ally_attack` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- on_opponent_attack`.
- **Updated 2026-05-08:** EX10-003 is no longer blocked on this primitive: production YAML uses `on_opponent_attack`, filtered own-source cost payment, and `cancel_attack`, covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_003`.
- **Remaining limits:** First-class DSL predicates such as attack-target kind / attacker trait are still follow-ups. Breeding-area observer fan-out is not proven by this slice.

### Modifier preventing attack-target redirection [G-MOD-CANNOT-CHANGE-ATTACK-TARGET]
- **Discovered in:** DNA Omnimon archetype, AD1-012 CresGarurumon DSL implementation (2026-05-03)
- **Scope:** Rust engine.
- **Card(s):** AD1-012 CresGarurumon — `[Inherited][Your Turn] This Digimon's attack target can't change.`
- **Effect text:** any clause that prevents the carrier permanent's attack from being retargeted (Blocker auto-redirect, attacker-side Raid switch, opponent-effect SwitchDefender).
- **Status (2026-05-08):** Closed for current combat retarget sources and the AD1-012 / ST18-14 card-shaped fixtures. `ModifierType::CanNotSwitchAttackTarget` and `ModifierType::CannotBeRedirectedAsAttackTarget` now exist, lower through the DSL modifier map, and are enforced by `EffectContext::redirect_attack`, the prompted `redirect_attack_target` selection, Blocker candidate selection/resolution, and the post-Block Raid retarget rider via `Game::validate_attack_redirect_target`. Rejected redirects do not fire `OnAttackTargetChange`; successful retarget payloads are available to DSL predicates for reason, attacker trait, new-target player/owner/trait, and old-target-was-self checks. Inherited self auras with modifiers now materialize onto the source permanent, so AD1-012's `[Your Turn] This Digimon's attack target can't change` blocks a scripted redirect attempt. ST18-14 proves the "another opponent Digimon or player" prompt shape. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- redirect_and_cancel`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- blocker_window_respects raid_retarget_respects`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- attack_target_change_ redirect_attack_target_prompt_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ad1_012_inherited_blocks_attack_target_change_during_your_turn st18_14`.
- **Remaining:** BT24-062 should get its own card-shaped fixture when that card is wired. Any future target-switch source must route through the same redirect validator before mutating `effective_target`.
- **Workaround:** None needed for current script-facing redirects and current Blocker/Raid retargets.

### `play_from_hand_free` Missing `bind_as` PermanentHandle Output  [G-PLAY-FROM-HAND-FREE-BIND-AS]
- **Status: RESOLVED 2026-05-17** (Phase 2 Track H). See `qa/resolved-gaps.md` § "Phase 2 Track H closure — 2026-05-17" for the full closure details.
- **Surface landed:** `PlayFromHandFreeArgs` (new struct distinct from `PlayFromHandArgs`) carries `bind_as: Option<String>`; `CompiledStep::PlayFromHandFree` carries the same. Execute path in `play_digivolve.rs` inserts the just-played permanent handle into the bindings under the configured name. BT16-085 YAML clause 0 now expresses the full free-play + scheduled delayed-return.
- **Discovered in:** BT16-085 Davis Motomiya & Ken Ichijoji implementation (2026-05-04)
- **Card(s) unblocked:** BT16-085 clause 0 (free-play + delayed return at next opponent's EOT).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16::bt16_085::bt16_085_start_of_main_played_digimon_returns_at_opponent_eot`.

### `event_card_color_has` Predicate Missing (Color-Gate on Digivolve/Enter Observer)  [G-EVENT-CARD-COLOR-IS]
- **Discovered in:** BT16-085 Davis Motomiya & Ken Ichijoji implementation (2026-05-04)
- **Card(s):** BT16-085 — "[Your Turn] When one of your Digimon digivolves into a **blue or green** Digimon, by suspending this Tamer, gain 1 memory." Also related: any card whose observer is conditioned on the entering/digivolving card's color.
- **Effect text:** "digivolves into a blue or green Digimon" — a color-containment check on the new top card of the digivolving permanent.
- **What's missing:** `PredicateSpec` (DSL) and `CompiledPredicate` (engine) have `event_card_trait_has` and `event_card_name_contains` predicates that inspect the entering/digivolving card, but no equivalent predicate for checking color membership. Related: PUPPETS-G023 (`event_card_color_only`, `event_card_color_count`) tracks exact multi-color checks; a single-color containment check (`event_card_color_has: blue`) belongs to the same family and is equally absent. Without it, BT16-085 Clause 1's "blue or green" gate cannot be expressed and the observer over-fires on any own Digimon digivolve.
- **Suggested change:** Add `event_card_color_has: Option<CompiledColor>` to `CompiledPredicate` and the matching leaf to `BoolPredicateSpec` / `PredicateSpec` in `digimon-dsl`. In `eval_predicate` (`predicate.rs`), implement the check by calling `event_target_card(rctx)`, resolving its `digimon_colors` from card_data, and testing for color membership.
- **Workaround:** Color gate omitted from YAML — observer over-fires on any own Digimon digivolve. Test `bt16_085_digivolve_observer_does_not_fire_on_non_blue_non_green_digivolve` is `#[ignore = "BLOCKED: G-EVENT-CARD-COLOR-IS"]`.

### ~~Opponent Digivolution-Card Source Selection Missing~~  [G-SELECT-OPPONENT-SOURCES] — RESOLVED
- **Status:** Stale active entry. `select_opponent_sources` and
  `EffectContext::select_opponent_sources` landed during the BG Imperial
  substrate closeout and are archived in `qa/resolved-gaps.md`.
- **ST2 reconciliation (2026-05-29):** ST2-03, ST2-06, and ST2-09 do **not**
  use `select_opponent_sources` because their printed text deterministically
  trashes the bottom source(s) after the Digimon target is chosen. They are
  implemented with `trash_bottom_sources`, covered by
  `tests/dsl/st2_substrate.rs` and
  `tests/cards_behavioral/st2/st2_cards.rs`.
- **Remaining work:** None for this reusable gap. BT16-085 card migration may
  still need a card-local follow-up, but it is not blocked by a missing
  opponent-source selector.

### OPT Reset via Attack Cycle  [G-OPT-RESET-VIA-ATTACK-CYCLE]  — CLOSED 2026-05-17 (Phase 2 Track C)
- **Closure:** Substrate already correct; the suspected "key persistence across turn boundaries" was a misdiagnosis. The slot key is `(carrier_permanent's `effect_activations` HashMap) × (source_card_handle, effect_slot)` and the reset clears the entire HashMap via `Permanent::new_turn()` at `begin_turn`, so any divergence between carrier identity and trigger source is irrelevant — both keys live in the same per-carrier map.
- **Failing test root cause:** `bt16_040_opt_resets_after_turn_cycle` (and the parallel BT17-015 / BT17-018 reset tests) failed because their test setup had no decks and no security for either player. After the first `end_turn()`, `begin_turn()` for the opponent tripped a deck-out and ended the game before rotation could reach the controller again, so `Permanent::new_turn` never ran for the carrier and the OPT slot stayed populated.
- **Fix landed:** Test-setup adjustments (decks + security for both players, low-DP defenders where needed). No engine-side changes. Migrated to `qa/resolved-gaps.md`.

### Activated-Digivolve Alt-Path Has No Engine Execution Route  [G-ACTIVATED-DIGIVOLVE-EXECUTION] — BT24-016 UNBLOCKED 2026-05-22 (residual for 3 cards)
- **Status (2026-05-22, `unblock-medusamon-tier3-cards`, design.md D1-REVISED):** **BT24-016 Lamiamon is unblocked** — clause 1 was re-modelled from a `kind: activated_digivolve` alt-path to a `when: main_from_hand` triggered clause (select Elizamon → select Dimetromon from trash → `place_as_bottom_source` → `effect_initiated_digivolve` cost 3, `ignore_requirements`), using only existing engine machinery and **zero engine code**. The card is `IMPLEMENTED`, 24/24 tests pass. **Residual:** the `CompiledAltPathKind::ActivatedDigivolve` alt-path *kind* still has no engine execution route — only the 3 out-of-scope cards below (BT22-013/026, BT16-027) need one; this entry stays open for them. The task-1.1 investigation also found `extra_cost` is unimplemented engine-wide (3 sites, all exclusions), so a true `activated_digivolve` route would need a from-scratch parking `extra_cost` runner.
- **Discovered in:** Medusamon archetype re-attempt run, BT24-016 Lamiamon DSL implementation (2026-05-21).
- **Scope:** Rust engine.
- **Card(s):** ~~BT24-016 Lamiamon~~ (UNBLOCKED — see Status above). Residual: BT22-013, BT22-026, BT16-027 — other `activated_digivolve` alt-path cards, currently covered structurally only.
- **Effect text:** "[Hand] [Main] ... it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements." — an activated, Main-timed digivolve initiated from a card in hand.
- **What's missing:** The `CompiledAltPathKind::ActivatedDigivolve` alt-path kind has no engine execution route. `dna_digivolve.rs` matches only `Digivolve`, `DnaDigivolve`, `BlastDnaDigivolve` — never `ActivatedDigivolve`. `game.rs` has zero `ActivatedDigivolve` references, and the action layer (`action/space.rs`, `action/mask.rs`) offers no action ID for an activated-digivolve alt-path. The DSL surface is complete — `condition:`, `from:`, `extra_cost`, `cost`, `ignore_requirements` all compile (G-ALT-PATH-CONDITION resolved the `condition:` field) — but the `[Hand][Main]` activated-digivolve action is never offered to the action space, so the clause cannot be played or behaviorally tested.
- **Suggested change:** Add an execution route for `CompiledAltPathKind::ActivatedDigivolve`: a Main-phase action masked in when a hand card declares an `activated_digivolve` alt-path whose `condition:` passes and whose `from:` source + `extra_cost` are satisfiable, then runs the digivolve at the declared `cost` with `ignore_requirements`.
- **Workaround:** None faithful. Clause 1 of BT24-016 ships structurally (alt-path compiles, `condition:` populated) but is un-executable; its tests cover it structurally only.

### ~~Declined Optional `[Security]` Effect Infinite-Loops on Resume~~  [G-SECURITY-SKILL-RESUME-REFIRE] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21. The security-resolution drain arms now record a `phase_enqueue_done: bool` on `SecurityResolutionState` — the drain phase enqueues its `EffectTiming` exactly once, and a resume after a parked (or declined) selection flushes/advances the phase instead of re-enqueueing. This covers all three drain phases (SecuritySkillDrain and siblings). The decline path no longer infinite-loops; `p_189_security_clause_can_be_declined` is an active (non-ignored) regression test. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, P-189 Dimetromon DSL implementation (2026-05-21).
- **Scope:** Rust engine.
- **Card(s):** P-189 Dimetromon (and any card with a declinable `[Security]` "you may" triggered effect — P-206, ST19-08, etc.).
- **Effect text:** "[Security] You may play 1 card ... " — any optional `on_security` triggered effect.
- **What was missing:** In `combat.rs::drive_security_resolution`, a drain arm enqueued its `EffectTiming`, drained, and returned early when a selection parked — **without advancing the phase or recording that the drain already fired**. On resume the phase was unchanged and, because the revealed card was still in `Game::pending_security`, the same `[Security]` effect re-installed its selection. When the player **declined** an optional security effect whose candidate persists, this re-parked indefinitely — an infinite loop (verified: 11+ consecutive PASSes never cleared `pending_selection`). The play (accept) path was unaffected because resolving the play consumes the candidate.
- **Resolution:** `SecurityResolutionState` gained a `phase_enqueue_done: bool` flag; each drain phase records that it has enqueued its `EffectTiming` for the current `revealed_card`, so the resume path advances past it instead of re-enqueueing. (Supersedes an earlier `security_skill_drained` single-phase variant — the `phase_enqueue_done` flag covers every drain phase.)

### ~~Plug-In Option Cannot Be Both a Standard `[Main]` Option and a Link Option~~  [G-LINK-OPTION-DUAL-PLAY-MODE] — RESOLVED 2026-05-22
- **Status:** RESOLVED 2026-05-22 (`unblock-medusamon-tier3-cards`). `classify_option_modes` returns the **set** of available play modes; `play_option_core` installs an `EffectChoice` mode-select for a dual-mode Plug-In and forks cost (Standard use cost vs flat Link cost) + disposal (Standard trash vs Link attach) on the chosen mode — reusing the existing `EffectChoice` / `PLAY_HAND` action range, so `ACTION_SPACE_SIZE` is unchanged. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, ST22-08 Offensive Plug-In V DSL implementation (2026-05-21).
- **Scope:** Rust engine.
- **Card(s):** ST22-08 Offensive Plug-In V, and any Plug-In Option with both a `[Main]`/`[Security]` effect and standalone Link Requirements.
- **Effect text:** ST22-08 has a `[Main]` effect (use as an Option, pay use-cost 4) AND inherited "Link Requirements [Link] Lv.3 or higher: Cost 2" (plug it in via the Link mechanic, pay link-cost 2) — two mutually-exclusive play modes.
- **What was missing:** `classify_option_subtype` (`game_actions.rs`) was first-match-wins: any effect carrying `link_cost.is_some()` reclassified the **entire card** as `OptionSubtype::Link`. The spike (design.md D2) ruled out a new action ID — the mode choice is surfaced as a `pending_selection` instead.
- **Resolution:** `classify_option_subtype` → `classify_option_modes` (returns `Vec<OptionPlayMode>`); `play_option_core` gained a `chosen_mode` parameter — for a dual-mode card it parks an `EffectChoice` mode-select (`install_option_mode_select`) and the callback re-enters with the chosen mode. `OptionSubtype` moved to `selection.rs` and is stored on `PendingOption.subtype` so `dispose_option` routes on the resolved mode. ST22-08.yaml gained a `kind: link_requirement` clause; `st22_08.rs` has 34 behavioral tests.

### ~~Move a Selected Trash Card to Deck TOP~~  [G-ZONE-SELECTED-TRASH-TO-DECK-TOP] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). `EffectContext::return_trash_cards_to_deck_top` + the `destination: top | bottom` DSL param move a selected trash card to the deck top. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, LM-027 Red Scramble DSL implementation (2026-05-21).
- **Scope:** Rust engine + DSL (hybrid). Full entry + suggested DSL surface in `qa/dsl-vocab-gaps.md` under the same gap ID.
- **Card(s):** LM-027 Red Scramble `[Start of Your Turn] <Delay>` body; also LM-029 / LM-030 / LM-031.
- **Effect text:** "Return 1 red Digimon card from your trash to the top of the deck."
- **What's missing:** All `EffectContext` trash→deck methods (`return_trash_cards_to_deck_bottom`, `return_all_trash_to_deck_bottom`) hard-code `deck.insert(0, card)` (deck bottom). No engine method moves a chosen trash card to the deck **top**. Distinct from the now-RESOLVED deck-bottom gap `G-ZONE-TRASH-TO-DECK`.
- **Suggested change:** Add `EffectContext::return_trash_cards_to_deck_top` (mirror the bottom variant but `deck.push`), exposed via a `destination: top|bottom` DSL parameter — see `qa/dsl-vocab-gaps.md`.
- **Workaround:** LM-027 clause B retains a `raw_rust` no-op; 4 tests `#[ignore]`'d with this gap tag.

### Outer-Optional-Prompt Condition Evaluated Without Trigger Context  [G-OUTER-OPTIONAL-COND-NO-TRIGGER-CONTEXT] — RESOLVED 2026-05-24
- **Discovered in:** Medusamon archetype re-attempt run, BT20-016 Paildramon DSL implementation (2026-05-21). Latent — not hit by BT20-016 itself. Reproduced externally via [scripts/mcp_paildramon_dna_confirm_choice.py](../../scripts/mcp_paildramon_dna_confirm_choice.py) driving BT16-085 Davis & Ken's `on_digivolve` clause through the engine MCP.
- **Scope:** Rust engine.
- **Card(s):** Any `optional` triggered clause whose body's first step is mandatory (so an outer accept/decline prompt is required) AND whose `condition` reads event-context predicates. BT21-026's deletion arm and BT16-085 Davis & Ken's `on_digivolve` clause are known affected cards. BT17-081 and EX4-061 / EX9-066 sister Tamers were impacted but masked by `if !accepted { return; }` lenient test patterns.
- **What's missing:** `queued_effect_wants_outer_optional_prompt` (`effect_queue.rs`) built an `EffectReadContext` and evaluated `effect.condition` WITHOUT installing the queued effect's `trigger_context` — unlike `evaluate_effect_condition` and the pre-cost-prompt branch, which both install it via `TriggerContextGuard`. For an optional triggered clause needing an outer prompt whose `condition` reads event-context predicates (`event_target_owner`, `event_target_kind`, `event_target_name_contains`, `event_card_color_has`, deleted-object snapshots), the predicate defaulted false → the outer prompt was wrongly suppressed → `run_queued_effect` then installed the correct context and silently ran the body. Player never saw the choice.
- **Fix (landed 2026-05-24):** Changed `queued_effect_wants_outer_optional_prompt`'s signature from `&self` to `&mut self` and installed `TriggerContextGuard::install(self, qe.trigger_context.clone())` before the condition + `outer_optional_guard` closure evaluations, mirroring the pre-cost-prompt branch. RAII Drop restores the previous context. Implemented under proposal `fix-outer-optional-prompt-trigger-ctx`.
- **Verification (2026-05-24):** New tests `bt16_085_optional_outer_prompt_installs_on_normal_digivolve` / `bt16_085_optional_outer_prompt_installs_on_dna_digivolve` / `bt16_085_optional_outer_prompt_decline_skips_body` / `bt16_085_optional_outer_prompt_accept_runs_body_with_trigger_ctx` (and sibling `bt17_081_optional_outer_prompt_installs_on_own_digivolve`) PASS post-fix and FAIL on `main`. 10 pre-existing tests with the lenient pattern were updated to drop the `if !accepted { return; }` early-return and lower starting memory below the +10 cap so the +1 gain is observable. Full `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` shows only 3 pre-existing failures (unchanged from `main`).

### Optional `select_hand` / `select_trash` Tail Does Not Run on PASS  [G-DSL-OPTIONAL-SELECT-PASS-TAIL]
- **Discovered in:** BT21-102 Tai Kamiya main clause (2026-05-11)
- **Scope:** Rust engine / DSL step executor.
- **Card(s):** BT21-102 Tai Kamiya, plus any card whose process is an optional `select_hand` / `select_trash` / etc. followed by trailing unconditional steps.
- **Effect text:** "...You may play 1 [Tai Kamiya] from your hand without paying the cost. Then, return this Tamer to the bottom of the deck." — the trailing "Then, return this Tamer to the bottom of the deck" must fire unconditionally, including when the optional play step is declined.
- **What's missing:** In `code/digimon-engine/src/dsl_cards/step/selections.rs`, the `install_select_hand` function (and the sibling `select_*` installers) thread the process `tail` into the selection callback only — `on_decline` is set to `None` for all DSL-driven prompts. Two divergent paths result:
  - **Path A (no eligible cards):** `select_hand` detects an empty `valid_action_ids`, does NOT install a prompt, returns `InstallResult::Continue`. The outer step executor advances and runs the remaining steps — `play_from_hand_free` no-ops (binding absent), then `return_to_deck` fires.
  - **Path B (eligible cards, player PASSes):** `select_hand` installs the prompt with `on_decline: None`. When PASS is submitted, `resolve_generic_selection` calls `on_decline` (None) → nothing runs → tail is never invoked → `return_to_deck` does NOT fire.
  Path A and Path B are semantically equivalent (player makes no selection) but produce different observable game state. For BT21-102 the printed "Then, return this Tamer..." is unconditional and both paths should fire it.
- **Engine location:** `code/digimon-engine/src/dsl_cards/step/selections.rs`, `install_select_hand` (and sibling `select_*` installers); `on_decline: None` is the divergence site.
- **Suggested change:** When `optional: true`, pass the `tail` as the `on_decline` callback in `install_select_hand` (and the sibling installers) — e.g. `on_decline: Some(Box::new(|game| { run_tail(tail, game); }))` — so PASS triggers the same continuation as the no-eligible-cards path.
- **Cards affected:** any card whose YAML has trailing unconditional steps after an optional `select_hand` / `select_trash` / etc. step.
- **Workaround:** None faithful. The BT21-102 test `bt21_102_main_opt_decline_hand_card_tai_stays_on_field` documents the divergence as observed behavior (Tai stays on field after PASS) rather than the printed-text outcome.

## Sweep notes (2026-05-23 — generalist training smoke surfacing)

Three single-outstanding-invariant violations surfaced from a generalist
pretraining smoke run over the 4 eligible Rust-DSL archetypes
(Medusamon, Puppets, DNA Omnimon, BG Imperial — 188 decks). All three
are debug-assertion panics that fire in real card chains the existing
behavioral tests don't cover. They share the same architectural shape:
a `Game::*` slot designed as single-outstanding (`debug_assert!` on
overwrite) is overwritten by a second resolution that fires before the
first drains. The [`game.rs:553-577`](../../code/digimon-engine/src/game.rs)
docstring on `dsl_outer_tail` already predicts this: *"a future change
that allows nested parks ... will need to either (a) make this a
`Vec<(_, _)>` stack, or (b) refuse the second park with a clear
validation error."* Phase 8 deferred-deletion and Phase 8 Option-play
slots have the same shape but no prediction in their docstrings.

### Nested DSL Outer-Tail Park  [G-DSL-OUTER-TAIL-NESTED-PARK] — RESOLVED 2026-05-23
- **Discovered in:** Generalist training smoke run, 2026-05-23 (mixed-archetype game across Medusamon / Puppets / DNA Omnimon / BG Imperial). Reliably reproducible via BT24-016 Lamiamon's clause-2 path.
- **Scope:** Rust engine.
- **Panic site:** [`code/digimon-engine/src/dsl_cards/step/mod.rs:119`](../../code/digimon-engine/src/dsl_cards/step/mod.rs) in `park_outer_tail`.
- **Invariant:** `ctx.game.dsl_outer_tail.is_none()` before writing — see [`game.rs:561-571`](../../code/digimon-engine/src/game.rs).
- **Card(s) surfaced:** BT24-016 Lamiamon (Medusamon shell) is the dominant trigger; clause 2 (`[When Digivolving][When Attacking][Once Per Turn]`) has body `[as_selecting_player { body: [select_hand, place_on_security] }, trash_top_security]`. Other archetype cards with the same "selection step with sibling continuation, inner body that calls a fire-and-inline-drain helper" shape are latent triggers too.
- **Root cause (identified 2026-05-23):** This is NOT a card-script bug — it's an engine architectural issue. `park_outer_tail`'s single-slot invariant is violated whenever an observer-fire helper does an INLINE `drain_effect_queue()` while a previous step's outer tail is still parked. Concretely for Lamiamon:
  1. Lamiamon clause 2 fires. Body step 0 `AsSelectingPlayer` returns Parked. `park_outer_tail([TrashTopSecurity])` stashes the outer tail; `dsl_outer_tail = Some(...)`.
  2. Player resolves the inner `select_hand`. Install-callback runs the inner tail `[place_on_security]`.
  3. `EffectContext::place_on_security` → `Game::place_on_security_observed` → eventually [`Game::fire_on_place_security` at `game_actions.rs:5743`](../../code/digimon-engine/src/game_actions.rs#L5743), which does `enqueue_triggered(OnPlaceSecurity, ...); self.drain_effect_queue();` **inline**, while we are still mid-callback and `dsl_outer_tail` is still set.
  4. That inline `drain_effect_queue` processes whatever is already queued — frequently a second Lamiamon clause-2 firing (e.g. a parallel `when_digivolving` from the same attack chain, or another Lamiamon's `when_attacking` queued from the same attack event). The second clause 2 body's step 0 `AsSelectingPlayer` calls `park_outer_tail([TrashTopSecurity])` → assertion trips because the first park is still parked.
- **Broader scope:** [`game_actions.rs`](../../code/digimon-engine/src/game_actions.rs) has 30+ inline `self.drain_effect_queue()` call sites, most inside `fire_on_*` observer helpers (`fire_on_play`, `fire_on_leave_field`, `fire_on_place_security`, `fire_on_link_after_option_placed`, etc.) and inside `play_option_core` / `dispose_option`. Every one of them is a potential nested-park trigger when called from inside an outer-tail-parked callback. Lamiamon happens to be the most-frequently-hit because of its specific body shape + frequency in eligible decks (the same card appears 4× in many Medusamon decks).
- **DCGO reference (2026-05-23):** [`DCGO/Assets/Scripts/Script/CardController.cs:5506`](../../DCGO/Assets/Scripts/Script/CardController.cs#L5506) `IAddSecurity.AddSecurity()` just **enqueues** OnAddSecurity triggers via `autoProcessing.StackSkillInfos` and does **not** drain them. The drain happens later at an explicit checkpoint (`TriggeredSkillProcess`). DCGO's architectural answer is "defer trigger drains to safe checkpoints" rather than "stack the parked-tail slot" — the C# coroutine system makes the call-stack implicit and the trigger queue is processed at well-defined moments, so the collision can't happen.
- **Suggested fix (immediate, narrow — Option A):** Convert [`Game::dsl_outer_tail`](../../code/digimon-engine/src/game.rs#L573) from `Option<(Vec<CompiledStep>, Bindings, StepRuntime)>` to `Vec<...>` — a stack of parked tails. `park_outer_tail` pushes; `drain_dsl_outer_tail` pops the most recent. Stack depth tracks nesting; add a sanity cap (e.g. 8) to surface runaway recursion. The docstring at [`game.rs:561-571`](../../code/digimon-engine/src/game.rs) prescribes exactly this fix. Same shape applies to sibling slots `pending_option` and `pending_deletion_resume` (the other two single-outstanding-invariant bugs in this family).
- **Suggested fix (architectural, wider — Option B):** Match DCGO's deferred-drain pattern: remove inline `self.drain_effect_queue()` from `fire_on_*` observer helpers and let drains happen at higher-level checkpoints (after a step's process body completes, after a selection resolves). Each removed inline drain needs an audit to ensure no downstream code depends on observers having already fired. Wider surgery, but eliminates the entire class of nested-park collisions instead of just paving over them with a stack.
- **Recommended order:** Option A now (small, contained, closes the panic). Option B later as broader architectural cleanup when there's appetite — they're not mutually exclusive; stacking the slot makes B safer to refactor in pieces.
- **Fix (landed 2026-05-23):** Option B chosen — deferred-drain mechanism mirroring DCGO's pattern. Added [`Game::draining_deferred: u32`](../../code/digimon-engine/src/game.rs) counter, plus `enter_deferred_drain()` / `exit_deferred_drain_and_flush()` / `maybe_drain_effect_queue()` helpers. `resolve_generic_selection` wraps its callback in enter/exit; `drain_dsl_outer_tail` wraps its outer-tail run the same way; `fire_on_*` observer helpers (`fire_on_link_after_option_placed`, `fire_on_play`, `fire_on_leave_field`, `fire_on_place_security`, `combat::fire_on_attack`) call `maybe_drain` so triggers enqueued mid-callback defer to the scope's exit. Two helpers — `fire_digivolution_card_trashed` and `place_permanent_on_security`'s OnDigivolutionCardTrashed / OnLinkedCardTrashed fires — INTENTIONALLY retain inline drain because behavioral test `ex10::ex10_036::ex10_036_clause_a_after_source_trash_prompts_opp_field_delete` depends on synchronous between-source observer firing for chained trash-pickup clauses.
- **Verification (2026-05-23):** Replayed all 84 BT24-016 crash recordings against the fixed engine — 84/84 no longer crash. Engine test suite shows 3292 passing, 8 pre-existing failures (same as `main` baseline), 0 new regressions.
- **Workaround:** Training crash-resilience wrapper catches the panic, writes a crash recording, and synthesizes a terminal step so training continues. Each hit costs one game's worth of training samples (≈0.5%/game frequency in current run).
- **Identifier:** the panic message includes the source card via the 2026-05-23 instrumentation patch (`card={card_id} player={pid} parking_step={discriminant} previously_parked_first_step={discriminant} ...`).

### Reentrant Option Play While Another Is Mid-Resolution  [G-OPTION-PLAY-REENTRANT] — RESOLVED 2026-05-23
- **Discovered in:** Generalist training smoke run, 2026-05-23.
- **Scope:** Rust engine.
- **Panic site:** [`code/digimon-engine/src/game_actions.rs:1148`](../../code/digimon-engine/src/game_actions.rs) in `play_option_core`.
- **Invariant:** `self.pending_option.is_none()` at play start — single in-flight Option.
- **Card(s) surfaced:** P-103 Offense Training (Medusamon shell; appears in 91/188 eligible decks). The panic instrumentation reported both the in-flight and incoming card, in the observed case both `P-103` with `in_flight_resolution_phase=MainEffectDrain` and `in_counter_window=false`.
- **Root cause (identified 2026-05-23):** Not a `play_option_core` overlap per se — the real bug was upstream in the end-turn state machine. `Game::end_turn` returned early at the old `game_phases.rs:214` `if self.pending_selection.is_some() { return; }` when an `EndOfYourTurn`-triggered effect parked a player selection, but the end-turn machinery never resumed after the selection unwound. The turn was left in an inconsistent state: `pending_option` from P-103's `<Delay>` activation chain stayed occupied, and the agent's next Option-play action tripped the assertion. P-103 was the trigger card because its `<Delay>` body runs at end-of-turn and clause 1's `select_own_permanent` installs exactly the selection the unresumed-end-turn bug needed.
- **Fix:** PR #520 (commit `008386f1`, 2026-05-23) added [`Game::pending_end_turn_resume: Option<EndTurnResume>`](../../code/digimon-engine/src/game.rs) and `Game::resume_pending_end_turn()`, wired into `effect_queue::resolve_generic_selection` after the parked selection resolves. End-turn now parks → selection resolves → resume → end-turn completes → turn rotates. `pending_option` no longer leaks across the resume boundary.
- **Regression test:** [`code/digimon-engine/tests/phase_flow/pending_selection_turn_end.rs::end_turn_selection_resolution_resumes_turn_rotation`](../../code/digimon-engine/tests/phase_flow/pending_selection_turn_end.rs).
- **Empirical confirmation:** post-`008386f1` generalist training run observed 16 panics across 12 parallel envs in the first ~10 minutes; zero were `reentrant Option play`. As of 2026-05-23 the entire family is resolved: `G-DSL-OUTER-TAIL-NESTED-PARK` (deferred-drain landed via PR #520/#521), `G-OPTION-PLAY-REENTRANT` (`pending_end_turn_resume` fix in PR #520), and `G-DELETION-RESUME-NESTED` (DCGO-modeled batched deletion flow, see entry below).

### ~~Nested Deferred Deletion (OnDeletion-Parked Selection)~~  [G-DELETION-RESUME-NESTED] — RESOLVED 2026-05-23
- **Discovered in:** Generalist training smoke run, 2026-05-23 (turn 17, mixed-archetype game; recording at `models/generalist_smoke/pilot_ppo_20260523_014433/recordings/train_env_000_game_000034_draw_crash.json`).
- **Scope:** Rust engine.
- **Panic site (historical):** `code/digimon-engine/src/replacement.rs:1382` (now deleted) in the deferred-decline branch.
- **Invariant (historical):** `game.pending_deletion_resume.is_none()` when parking a new deferred deletion — single in-flight OnDeletion-parked deletion.
- **Resolution (2026-05-23, `align-deletion-with-dcgo-model` change):** Option B chosen over the suggested stack-the-slot stop-gap — the deletion architecture was migrated to a DCGO-modeled batched flow. Key changes:
  - New `Game::delete_permanents_batch(handles, cause) -> DeletionBatchOutcome` as the unified deletion entrypoint; single-target callers (`delete_permanent_with_effects`, `delete_permanent_with_cause`) shim through as one-element batches.
  - Trash-before-OnDeletion drain (DCGO `DestroyPermanentsClass` parity): `enter_deferred_drain` → enqueue OnDeletion per survivor → trash all → `exit_deferred_drain_and_flush` drains handlers post-trash.
  - `DeletedObjectSnapshot` extended with pre-removal fields (`dp_just_before`, `level_just_before`, `cost_just_before`, `names_just_before`, `traits_just_before`, `source_count_just_before`, `digisources_just_before`) and threaded into the OnDeletion trigger context.
  - `EffectContext::deleted_self_*` accessors expose snapshot state to handlers.
  - `Keyword::Save` / `Keyword::Fortitude` / `Keyword::Partition` rewritten to read from snapshot+trash inline (no `pending_post_deletion_replays` push).
  - DSL-side fix at `predicate_subject_for_source`: when `source_permanent`'s slot is gone AND trigger context has `deleted_object`, fall back to `PredicateSubject::None` so subject-agnostic predicates (count_gte on hand, etc.) still evaluate correctly post-trash. This single fix closed 5 card_behavioral OnDeletion handler regressions without per-card edits.
  - `pending_post_deletion_replays` slot retired entirely.
  - `pending_deletion_resume` Vec slot retired (active-batch state machine in `Game::resume_pending_deletion` handles all parking).
  - Dead functions deleted: `commit_permanent_deletion`, `finalize_permanent_deletion`, `finalize_permanent_deletion_with_event_card` — ~270 lines of legacy code.
- **Regression test:** [`code/digimon-engine/tests/deletion_batching/aoe_save_park.rs`](../../code/digimon-engine/tests/deletion_batching/aoe_save_park.rs::aoe_delete_two_save_permanents_both_park_sequentially) — explicit N=2 AoE-Saves regression. Plus `aoe_delete_three_save_permanents_all_park_in_sequence` (N=3) and `aoe_delete_two_save_permanents_both_declined`.
- **Test results (post-fix):** lib 153/153 ✓, combat 206/206 ✓, keyword_phase_d 41/41 ✓, deletion_batching 7/7 ✓, cards_behavioral 3292/3300 (8 baseline pre-existing failures, 0 new regressions).
- **Change reference:** `openspec/changes/archive/2026-05-23-align-deletion-with-dcgo-model/` (proposal, design, specs, tasks).

### ~~Empty Permanent During Batched Deletion — digivolve-from-material zombie~~  [G-PERMANENT-EMPTY-DIGIVOLVE-FROM-MATERIAL] — RESOLVED 2026-05-23 by PR #533

> **Family-split note (2026-05-24):** the original `G-PERMANENT-EMPTY-DURING-BATCH-DELETION` family was mis-named (the cause was a digivolve-from-material zombie, not batch deletion). PR #533 closed ONE code path; sibling material-extraction paths (`play_from_materials`, `place_as_bottom_source`, replacement-redirect-to-Trash, place-into-security from material, `trash_source_ref`, `trash_card_source`) plus two effect-queue read-side panic sites remained open and were closed by the `fix-zombie-permanent-siblings` change. The narrative below is preserved verbatim from PR #533 for the digivolve-specific variant. The broader-class entry follows.

- **Discovered in:** Generalist v4 training run, 2026-05-23, ~15 minutes after launch (recordings at `C:/Users/james/digimon-training-runs/models/generalist_v4/pilot_ppo_20260523_145133/recordings/train_env_003_game_000017_draw_crash.json` and `train_env_004_game_000024_draw_crash.json`).
- **Scope:** Rust engine. Latent pre-PR #525; surfaced after `G-DELETION-RESUME-NESTED` was silenced (the deletion panic was firing ~1 per 12k steps in v3, drowning out this rarer empty-permanent case).
- **Panic site:** [`code/digimon-engine/src/permanent.rs:134`](../../code/digimon-engine/src/permanent.rs) in `Permanent::top_card()` (and the `top_card_mut` sibling at line 141).
- **Invariant:** `self.card_sources` is non-empty when `top_card()` is called. A Permanent should always have at least one card on its digivolution stack while it sits in the battle area.
- **Symptom rate:** ~0.17 panics/min in v4 (2 panics in first 12 min across 12 parallel envs). Similar order-of-magnitude to v3's `G-DELETION-RESUME-NESTED` rate (0.27/min) — the underlying frequency of the trigger pattern is probably comparable; the bug was just masked by the noisier deletion panic before PR #525.
- **Initial hypothesis (DISPROVEN 2026-05-23):** that PR #525's `delete_permanents_batch` had an unsafe window between trash and slot-removal where downstream code still did a live `top_card()` call.
- **Actual root cause (identified 2026-05-23 by replaying `train_env_003_game_000017_draw_crash.json --step 96` with `RUST_BACKTRACE=full`):** the panic is NOT a deletion-batching bug. The backtrace points at the DSL `effect_initiated_digivolve` path:

  ```
  permanent.rs:134                Permanent::top_card                (panic site)
  effect_queue.rs:1220            Game::top_card_handle
  effect_queue.rs:1003            trigger_context_for_source (Digivolved arm)
  effect_queue.rs:317             Game::enqueue_triggered (OnDigivolve fan-out)
  game_actions.rs:6438            effect_initiated_digivolve_from_source_inner
  game_actions.rs:6266            …_ignore_requirements
  dsl_cards/step/play_digivolve.rs:480  CompiledStep::EffectInitiatedDigivolve
  ```

  Mechanism: `effect_initiated_digivolve_from_source_inner` calls `take_card_source_ref(Material(src, i))` which uses `card_sources.remove(i)` ([`game_actions.rs:4249`](../../code/digimon-engine/src/game_actions.rs#L4249)). When the source permanent has only one card and `i == 0`, the take leaves `src.card_sources` empty but the slot remains in `battle_area`. The subsequent `enqueue_triggered(OnDigivolve, …)` then iterates EVERY permanent in BOTH players' battle areas ([`effect_queue.rs:307-321`](../../code/digimon-engine/src/effect_queue.rs#L307-L321)) and calls `top_card_handle(observer)` per observer, panicking on the now-empty carrier with `Permanent must have at least one card`.

  PR #525's deletion-batching refactor merely silenced the noisier `G-DELETION-RESUME-NESTED` panic; this latent zombie-permanent bug is now the dominant residual. It will fire on any code path that empties a permanent's `card_sources` without also removing the slot from `battle_area` — digivolve-from-material is the surfaced case; play-from-material via `play_from_materials_suppress_on_play` is a likely sibling.

- **Affected card surface:** 41 YAML cards use the DSL `effect_initiated_digivolve` step (BT24-016, P-103, LM-027/029/030/031/032/054/055, BT21-001/013/093, BT22-013/026/036/098, EX9-012/019/032, EX10-032/069, BT16-028/040, BT17-015/027/097, BT20-083/084, AD1-001/010, …). Any of them hits this path when their `source` binding resolves to a single-card Material ref.
- **Regression test (failing on `main` as of 2026-05-23):** [`code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs::effect_digivolve_from_material_emptying_source_does_not_leave_zombie_permanent`](../../code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs). Asserts the carrier is removed from `battle_area` after a `Material(src, 0)` digivolve consumes its only card. Panics with the exact `Permanent must have at least one card` on current `main`.
- **Fix (landed 2026-05-23, two layers — both implemented):**
  1. **Root cause:** [`Game::soft_remove_if_emptied`](../../code/digimon-engine/src/game.rs) helper, called from `effect_initiated_digivolve_from_source_inner` ([`game_actions.rs:6418-6440`](../../code/digimon-engine/src/game_actions.rs#L6418)) after step 4's `digivolve` mutation. If `source_ref` was `Material(src, _)` and `src`'s `card_sources` is now empty, the helper removes the slot from `battle_area` BEFORE firing WhenDigivolving / OnDigivolve. Linked cards on the removed carrier flow to trash + fire `OnLinkedCardTrashed` per the same pattern as `trash_single_for_batch`. `target.index` is shifted via [`Game::shift_handle_after_soft_remove`](../../code/digimon-engine/src/game.rs) when the removal shifts later same-player indices.
  2. **Defensive guardrail (general):**
     - [`Game::top_card_handle`](../../code/digimon-engine/src/effect_queue.rs) and [`EffectContext::permanent_top_card_handle`](../../code/digimon-engine/src/effect_context/mod.rs) — both now `.and_then(card_sources.last())` instead of `.map(top_card())`. Returns `None` for zombie permanents instead of panicking; all callers already wrap in `Option::and_then`/`let Some(…) else`.
     - [`Game::enqueue_from_permanent`](../../code/digimon-engine/src/effect_queue.rs) — early-return guard if the target slot is missing or has empty `card_sources`. No effects to enqueue for a zombie.
     - [`Game::queued_effect_source_is_live`](../../code/digimon-engine/src/effect_queue.rs) — replaced 3 direct `perm.top_card().card_index` calls with `perm.card_sources.last().map(|c| c.card_index)` patterns (breeding-area branch, battle-area branch, Training scan). Zombie permanents now correctly fail the liveness check instead of panicking.
- **Regression tests (in [`code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`](../../code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs)):**
  - `effect_digivolve_from_material_emptying_source_does_not_leave_zombie_permanent` — the original reproduction
  - `effect_digivolve_from_material_emptying_lower_indexed_source_shifts_target_index` — target-index shift case
  - `effect_digivolve_from_material_emptying_source_with_linked_cards_trashes_them` — linked-card cleanup
  - `effect_digivolve_from_material_taking_bottom_of_multi_card_stack_keeps_carrier_alive` — boundary (no cleanup needed)
  - `ondigivolve_fanout_tolerates_pre_existing_empty_permanent_in_battle_area` — Layer 2 synthetic guard
- **DCGO reference (informed the fix):**
  - [`DCGO/Assets/Scripts/Script/Permanent.cs:1352-1367`](../../DCGO/Assets/Scripts/Script/Permanent.cs#L1352) — DCGO's `TopCard` returns `null` for empty stacks rather than asserting; all callers null-check. Inspired Layer 2's `Option`-returning `top_card_handle`.
  - [`DCGO/Assets/Scripts/Script/CardController.cs:1509`](../../DCGO/Assets/Scripts/Script/CardController.cs#L1509) — Jogress uses caller-side `CardObjectController.RemoveField(permanent)` to explicitly remove a permanent whose body was absorbed. Inspired Layer 1's `soft_remove_if_emptied` (the DCGO `RemoveField` analog — distinct from `DestroyPermanentsClass.Destroy()`).
  - [`DCGO/Assets/Scripts/Script/CardObjectController.cs:370-447`](../../DCGO/Assets/Scripts/Script/CardObjectController.cs#L370) — `RemoveFromAllArea` also leaves zombie permanents (same `Vec.Remove` pattern as Rust's `take_card_source_ref`). Confirms the zombie risk is shared across both engines; DCGO survives because consumers null-check `TopCard` everywhere and there's no global `OnDigivolve` fan-out (`EffectTiming` enum at `ICardEffect.cs:969-1032` has no `OnDigivolve` value).
- **Empirical confirmation:** all 4 known crash recordings replay cleanly post-fix (v4 003_017 and v4 004_024 from the `G-PERMANENT-EMPTY` family, plus 2 from the user's currently-running training that were stale-binding panics from PR #525 not yet being installed). Engine test suite: 3292 cards_behavioral pass + 153 lib + 614 across 9 other test binaries = 4059 tests, 0 new regressions, 8 pre-existing baseline failures unchanged.
- **Previous-hypothesis (deletion-batching) suggested investigation (RETAINED for context):**
  - Replay `train_env_003_game_000017_draw_crash.json` against current main with `RUST_BACKTRACE=1`; the backtrace points at the live `top_card()` caller — and now we know it's the digivolve fan-out, not the deletion drain.
  - Audit every `top_card()` / `top_card_mut()` call site for "could this Permanent have been emptied by an in-flight batch deletion?" — most callers are fine post-PR-#525; the zombie-permanent class above is a separate concern from deletion-batching.
  - DCGO reference: `DCGO/Assets/Scripts/Script/CardController.cs` `DestroyPermanentsClass.Destroy()` and the snapshot threading PR #525 already mirrors.
- **Workaround:** Training crash-resilience wrapper catches the panic, writes a crash recording, synthesizes a terminal step → training continues. Each hit costs one game's worth of training samples (~0.5% of games at current rate).
- **Identifier:** the panic message `Permanent must have at least one card` is verbatim from `.expect(...)` on `Vec::last()` — no card identity surfaces. Adding card identity to the panic message would speed up triage.

### ~~Empty Permanent During Material Extraction (sibling class)~~  [G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION] — RESOLVED 2026-05-24 by `fix-zombie-permanent-siblings`
- **Discovered in:** Generalist training run `pilot_ppo_20260523_215003`, started 2026-05-23 ~21:50, ~2h after PR #533 landed. 10 fresh `_draw_crash.json` panics across the first ~50 minutes (~0.25 panics/min — same order as the pre-PR-#533 rate). Recordings: `models/generalist_1m/pilot_ppo_20260523_215003/recordings/*_draw_crash.json` (10 files: game indices 42, 377, 551, 637, 660, 2556, 2619, 3174, 3490 from `train_env_000`; plus 1 from `eval_env_000`). Spread across 5 archetypes (BG Imperial, Rocks, Medusamon, DNA Omnimon, Puppets), confirming class-level rather than card-specific.
- **Scope:** Rust engine. Latent pre-PR #533; surfaced after the digivolve-from-material variant was silenced (the dominant sibling) — same family-narrowing pattern that surfaced this whole class when PR #525 silenced `G-DELETION-RESUME-NESTED`.
- **Panic site:** `code/digimon-engine/src/permanent.rs:134` in `Permanent::top_card()`. Identical message to the digivolve variant (`Permanent must have at least one card`); the panic message carries no card identity so disambiguation requires `RUST_BACKTRACE=full` or process-of-elimination via recording replay.
- **Root cause (audited 2026-05-24, change `fix-zombie-permanent-siblings`):** `Game::soft_remove_if_emptied` (introduced by PR #533) was called from EXACTLY ONE production site — `effect_initiated_digivolve_from_source_inner` at `game_actions.rs:6481`. Six other production code paths mutate `card_sources` in ways that can empty the carrier without cleanup, all matching the same bug pattern PR #533 fixed:
  1. `EffectContext::play_from_materials_suppress_on_play` (effect_context/mod.rs:3329) — explicit sibling flagged in the original gaps.md prose. 8 YAML cards trigger this: BT22-015 Omnimon `<Decode>`, BT13-110, BT13-112, BT20-083, BT23-072, EX4-060, EX9-021.
  2. `Game::place_as_bottom_source_observed` (game_actions.rs:4426) — `<Save>` / Stash / BottomReturn. Common across archetypes.
  3. Replacement-redirect-to-Trash branch (game_actions.rs:6141) — `WhenWouldPlaceInSecurity` replacement outcome.
  4. Place-into-security from material (game_actions.rs:6192) — `EffectContext::place_on_security` with `CardSourceRef::Material`.
  5. `Game::trash_source_ref` (game.rs:1058) — agent-selected "trash 1 of your digivolution sources" actions. **Rocks archetype hits this heavily**, accounting for the bulk of post-PR-#533 panics.
  6. `EffectContext::trash_card_source` (effect_context/mod.rs:4028) — targeted by-handle source-trash.
- **Layer 2 (read-side) gaps closed in the same change:** PR #533 hardened `enqueue_from_permanent`, `enqueue_from_breeding_permanent`, `queued_effect_source_is_live`, and `top_card_handle` to tolerate zombies. Two more iter callers were still unguarded:
  - `find_event_gated_delay_permanent` (effect_queue.rs:2361) — iterates ALL battle_area perms calling raw `top_card()` on each; any zombie panics the scan. Likely dominant production panic site for non-Rocks decks.
  - `event_gated_delay_source` (effect_queue.rs:2327) — raw `top_card()` on `qe.source_permanent`.
- **Fix landed 2026-05-24:** soft-remove cleanup added at each of the 6 mutation sibling sites; Layer 2 zombie-skip guards added at the 2 effect-queue read sites. Per-sibling regression tests added (5 new files in `code/digimon-engine/tests/effect_context/`).
- **Regression tests (in [`code/digimon-engine/tests/effect_context/`](../../code/digimon-engine/tests/effect_context/)):**
  - `play_from_materials.rs` — `play_from_materials_emptying_source_does_not_leave_zombie_permanent`, `play_from_materials_emptying_lower_indexed_carrier_shifts_neighbor_index`, `play_from_materials_failed_rollback_keeps_single_source_carrier`
  - `place_as_bottom_source_zombie.rs` — `place_as_bottom_source_from_material_emptying_carrier_removes_slot`, `place_as_bottom_source_lower_indexed_carrier_shifts_target_index`
  - `place_on_security_zombie.rs` — `place_on_security_from_material_emptying_carrier_removes_slot`
  - `trash_source_ref_zombie.rs` — `trash_source_ref_emptying_carrier_removes_slot`
  - `trash_card_source.rs` (appended) — `trash_card_source_emptying_carrier_removes_slot`
- **Remaining sibling out of scope (filed for follow-up):** `EffectContext::trash_top_source` (effect_context/mod.rs:4186) — same fix shape; discovered during the Task 1.1 audit but not bundled into this change to keep scope focused.
- **Change reference:** `openspec/changes/fix-zombie-permanent-siblings/` (proposal, design, specs, tasks).

### Family-wide note: Single-Outstanding-Invariant Pattern

The three bugs above plus their predicted siblings (`pending_post_deletion_replays` at [`game.rs:519-551`](../../code/digimon-engine/src/game.rs) is already a `Vec` and works correctly under nesting) all reflect a Phase 8 / Phase 2d design choice: when adding a parked-state slot, default to `Option<T>` with a `debug_assert!` guard, and audit later if nesting surfaces. The audit time is now. Recommend a tracking task to:

1. Audit every `pub(crate) ... : Option<T>` field on `Game` that represents in-flight resolution state.
2. For each, decide stack-vs-refuse based on whether the action surface should expose nesting to the RL agent.
3. Where stack semantics are chosen, write a behavioral test that exercises nesting depth ≥ 2 before promoting the field.

### Family-wide note: Empty-Permanent class (updated 2026-05-24)

PR #533's Layer 1 + Layer 2 pattern is now applied uniformly across the material-extraction surface (`fix-zombie-permanent-siblings`, 2026-05-24). Every production code path that mutates a `Permanent`'s `card_sources` Vec in a way that can empty it now invokes `Game::soft_remove_if_emptied` to drop the carrier slot; the two effect-queue read-side iter callers that previously panicked on zombies (`find_event_gated_delay_permanent`, `event_gated_delay_source`) now skip empty carriers in line with the existing `enqueue_from_permanent` / `queued_effect_source_is_live` pattern.

The architectural follow-up — refactoring `Permanent::top_card()` to return `Option<&CardSource>` (DCGO's `Permanent.cs:1352-1367` shape, where every caller null-checks) — remains the long-term direction. Per-site `soft_remove_if_emptied` is a continued whack-a-mole if new material-extraction shapes land; the Option-returning refactor systemically prevents the failure mode. ~40 raw `top_card()` callers across `combat.rs`, `dsl_cards/predicate.rs`, `dna_digivolve.rs`, and `dsl_cards/formula_eval.rs` would be in-scope for that refactor. Tracking this here so the audit picks it up after the panic-family pressure subsides.

One known sibling deferred from `fix-zombie-permanent-siblings`: `EffectContext::trash_top_source` (effect_context/mod.rs:4186) follows the same fix shape but was discovered after scope was set; file a follow-up change to close it.

### §DSL Trash-Selected-Sources Stale Handle (G-DSL-TRASH-SOURCES-STALE-HANDLE)

- **First seen:** 2026-05-24, training run `generalist_1m_v2`, game 9728, recorder action 87 (TS Olympos Magneticdramon-Mineral/Rock vs Yellow Tamer-heavy, turn 10).
- **Symptom:** `EffectContext::trash_card_source` panics with `"card not in this permanent's stack"`. Family pattern (regex): `trash_card_source: card not in this permanent's stack`. Panic site: [`code/digimon-engine/src/effect_context/mod.rs::trash_card_source`](../../code/digimon-engine/src/effect_context/mod.rs).
- **Sibling class:** read-side family is `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` (closed 2026-05-24). Same handle-staleness mechanism, distinct manifestation — write-after-shift on `card_sources.position` (this entry) vs read-after-empty on `top_card` (sibling). Discovered as a follow-up while production training surfaced a stale candidate the picker was advertising past observer-cascaded invalidation.
- **Root cause:** [`install_source_multi_selection`](../../code/digimon-engine/src/effect_context/selections.rs) snapshots candidate `SourceSelectionRef`s at install time into `action_to_source` (line 2564) and never re-validates them. When two `[WD]` triggers fire in sequence on the same play action (player picks ordering at TriggerOrder, then resolves them serially), the first trigger's `trash_selected_sources` can drain the second trigger's snapshotted candidates BEFORE the agent submits. The agent's submit drives `trash_card_source` with a stale `CardHandle` that no longer matches any card in `perm.card_sources`, and the `.expect("card not in this permanent's stack")` panics. Production reproducer: EX10-033 Pyramidimon [WD] Clause B (mandatory `select_own_sources(min=0, max=3) → trash_selected_sources`) ran first and trashed both EX8-050 sources from slot 0; EX10-032 Proganomon [WD] Clause 2 (mandatory `select_own_sources(min=1, max=1)`) then installed its picker — but slot 0 had a captured candidate that was already drained.
- **Fix landed (`fix-trash-card-source-stale-handle`, 2026-05-24):** DCGO-parity reshape of the trash primitive. `EffectContext::trash_card_source` now returns `bool` (true iff actually trashed) and soft-fails — no panic — on three rules-natural conditions (carrier missing, empty stack, card not in stack). Mirrors DCGO `ITrashDigivolutionCards.TrashDigivolutionCards()` ([`DCGO/Assets/Scripts/Script/CardController.cs:5181`](../../DCGO/Assets/Scripts/Script/CardController.cs)) which guards entry and filters target cards against the live `_permanent.DigivolutionCards`. `install_source_multi_selection`'s pick callback (effect_context/selections.rs:2586) now re-validates the picked `source_ref` against the live `card_sources` and re-installs with refreshed candidates if the pick has vanished — mirrors DCGO `SelectCardEffect.SetUp(... customRootCardList: selectedPermanent.DigivolutionCards ...)` ([`DCGO/Assets/Scripts/Script/CardEffectCommons/TrashDigivolutionCards.cs:125`](../../DCGO/Assets/Scripts/Script/CardEffectCommons/TrashDigivolutionCards.cs)) which reads the live list at display time. DSL `CompiledStep::TrashSelectedSources` and `CompiledStep::TrashUnionBound` discard the new bool with `let _ = ...` — surviving picks trash, stale ones no-op silently.
- **Regression tests:**
  - `code/digimon-engine/tests/effect_context/trash_card_source.rs` — three new unit tests: `trash_card_source_returns_false_on_stale_card_handle`, `trash_card_source_returns_false_on_missing_carrier`, `trash_card_source_returns_false_on_empty_stack`. Plus bool-return assertions on the three existing happy-path tests.
  - `code/digimon-engine/tests/selection/source_multi.rs` — two new picker tests: `source_multi_picker_re_installs_on_stale_pick_min_one_no_op` (verifies clean termination when re-install lands with no candidates and picked < min) and `..._min_zero_finalizes_empty` (verifies final-callback fires with empty Vec when picked >= min).
  - `code/digimon-engine/tests/recording_replay_regressions.rs` — `replay_g_dsl_trash_sources_stale_handle_does_not_panic`. Replays the captured production crash (fixture: `code/digimon-engine/tests/recordings/g_dsl_trash_sources_stale_handle.json`, 17 KB, 87 actions) to end without panic. Pre-fix, the panic fires mid-seek; post-fix, clean Ok.
- **Out-of-scope follow-ups (filed in `fix-trash-card-source-stale-handle`/design.md):**
  - Tier 3: full DCGO-shape two-step picker (`SelectPermanent` outer → `SelectCard` scoped to live `selectedPermanent.card_sources` inner, with per-permanent trash-interleave). Closes the staleness problem at the picker level rather than at the trash primitive. Bigger refactor.
  - Recorder/replay JSON schema mismatch: `code/digimon-engine/src/runners/replay.rs:134` expects `initial_state` at the top of the JSON; the recorder nests it under `recording`. Worked around in tests by extracting the inner object.
  - `tracing::debug!` on soft-fail paths for post-hoc analysis (design Q3) — gated on adding `tracing` crate dep.

Crash recordings from the 2026-05-23 training smoke are preserved under
`models/generalist_smoke/pilot_ppo_*/recordings/*draw_crash.json` and contain
the exact action sequences (initial state, deck contents, action ids,
selection prompts) that reach each panic site — useful starting points
for the failing tests.

A machine-readable index of these families lives next to this file at
[`panic-families.json`](panic-families.json) — used by `digimon-training-mcp`
to group panics in training console logs by family. This markdown is the
prose source-of-truth; the JSON is the index that points back to it. When
adding a new family entry above, add a matching record to the JSON (the same
`family_id`, a distinctive substring pattern, a panic-site reference, and a
status).


### §`kind: digimon` field-select filter excludes Tokens (G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT) — OPEN

- **First seen:** 2026-05-29, judge-quiz Q12 pin attempt (`batch-implement-cards-rust-dsl` first wave). The DSL `kind: digimon` target filter lowers to `kind_matches_field` (`code/digimon-engine/src/dsl_cards/predicate.rs` ~2826), which matches `CardKind::Digimon | CardKind::Dual` but NOT `CardKind::Token`. A Petrification token on the field is therefore filtered OUT of a "select 1 of your Digimon" candidate set.
- **Judge rule (Q12):** a token placed as a digivolution card counts; a Digimon token IS a Digimon on the field. DCGO `BT24_059.cs` uses `IsPermanentExistsOnOwnerBattleAreaDigimon` -> `Permanent.IsDigimon`, and a token's card entity includes `CardKind.Digimon`. So BT24-059's inherited `[When Attacking]` (place 1 of your other Digimon as a source -> unsuspend) should accept the Petrification token as the placed Digimon -> the Digimon unsuspends.
- **Symptom / proof:** the Q12 scenario test (`f_token_and_memory::q12_token_placeable_as_digivolution_card_unsuspends`) uses the REAL `TOKEN_PETRIFICATION` permanent (`CardKind::Token`) and finds the placement selection never installs (token excluded) -> left `#[ignore]` citing this gap (refused to false-pass). NOTE: the per-card test `bt24::bt24_059::inherited_q12_token_source_counts_and_unsuspends` passes only because it uses a `CardKind::Digimon` STAND-IN, which dodges the token-as-Digimon question — it should be re-pointed at a real token once this gap closes.
- **Fix shape (engine; broad — validate carefully):** `kind_matches_field` (and any sibling kind-matchers used for field target selection) should treat `CardKind::Token` as satisfying `digimon` (tokens are Digimon on the field). This is a BROAD behavioral change — every "select a Digimon" effect would then include tokens (which is correct per rules, but affects many cards/tests). Land it as its own change with a full regression pass; do not fix inline.
- **Blocks (judge-quiz):** Q12 (cards BT24-040 + BT24-059 + Petrification token ALL implemented — this is now a PRIMITIVE block, not a card block).


### §Suspend event carries no effect-initiated bit (G-SUSPEND-EFFECT-INITIATED) — OPEN

**Surfaced 2026-05-30** (judge-quiz cluster B, EX6-004 Kokomon — BLOCKED). EX6-004's
inherited clause is "[Your Turn][OPT] When an EFFECT suspends one of your Digimon,
1 of your Digimon gets +2000 DP for the turn." The "by an effect" qualifier is
un-gatable: `TriggerSource::EventObserved` (the source for `OnSuspend`, built by
`Game::suspend`) carries no `effect_initiated`/`by_effect` field — its
`TriggerContext` is constructed with `..TriggerContext::default()` (effectively
`false`). The DSL predicate `event_is_effect_initiated` compiles but always
evaluates `false` on suspend events, so the clause would over-trigger on
attack-declaration / cost suspends. Fix: add `effect_initiated: bool` to
`TriggerSource::EventObserved`, set it `true` on the `EffectContext::suspend`
path vs `false` on the raw `Game::suspend`/attack/cost path, and populate the
`TriggerContext` in `effect_queue.rs`. Additive, contained engine-event change
(no behavior change for existing cards — they don't gate on it). Also tracked in
`qa/archetype-qa/dsl/zephagamon-2026-05-03-dsl-engine-gaps.md` ("Extend
suspend/unsuspend event context with by_effect"). EX6-004 stays BLOCKED (no card
authored — no stub) until this lands.


### §Mass DP debuff applied as a one-time snapshot, not continuous (G-CONTINUOUS-MASS-DP-DEBUFF) — OPEN

**Surfaced 2026-05-30** (judge-quiz Q14 pin). EX4-074 ShineGreymon: Ruin Mode's
"[When Digivolving][On Deletion] Until the end of your opponent's next turn, all
of your opponent's Digimon get -5000DP" is authored as `add_modifier target:
{ of: opponent, kind: digimon }` — a one-time scan applying `ChangeDp -5000` to
the opponent's CURRENT battle-area Digimon. Per Digimon rules a continuous "all X
get -Y until Z" effect also applies to Digimon that ENTER during the window.
Verified empirically (Q14): a ShoeShoemon (P-165) played AFTER Ruin Mode resolved
stays at 4000 DP (not -1000). Faithful fix: install a CONTINUOUS player-scoped /
until-condition modifier (re-evaluated as Digimon enter), not a snapshot, so
later-entering opponent Digimon also receive the debuff. Blocks judge-quiz Q14
(`q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode`, body written + #[ignore]'d).
Likely a shared substrate need for every "all [opp] Digimon get ±DP until [turn]"
mass-buff/debuff card, not just EX4-074.


### §Burst-Digivolve `on_burst_turn_end` teardown never executed (G-BURST-ON-TURN-END-NOT-EXECUTED) — OPEN

**Surfaced 2026-05-30** (judge-quiz Q8 attempt). The DSL `on_burst_turn_end:` alt-path step
list (Burst Mode's "At the end of the burst digivolution turn, trash this Digimon's top
card") is fully parsed + compiled (`alt_path.rs`, `compiled.rs`, `compile.rs`) and
structurally tested, but it is **NEVER executed or scheduled** anywhere in the engine. The
only references to `CompiledAltPath::on_burst_turn_end` are three path-detection
`!is_empty()` checks in `dna_digivolve.rs` and a raw-rust-name collector in
`digimon-dsl/pack.rs` — none run the steps. Furthermore `CompiledAltPathKind::BurstDigivolve`
is lumped with `BlastDnaDigivolve` in `dsl_cards/mod.rs:97-108` and lowered only to a
"Blast digivolve marker" (the combat counter-window blast), so a Burst-Mode digivolution
has no "trash top at end of burst turn" wiring at all.

Impact:
- **Judge-quiz Q8 BLOCKED** (`q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot`): the
  scenario (Comet Hammer de-digivolves the Burst stack to Agumon → at EoT the Burst trashes
  the top Agumon → the revealed DP-less Koromon can't remain) cannot occur because the EoT
  trash never fires. (A second blocker, the "DP-less Lv2 can't remain in the battle area"
  rule, is moot until this lands; and DebugRunner has no burst-digivolve action driver.)
- **BT13-020 (ShineGreymon: Burst Mode) + the BT13-060 example** ship a burst alt-path whose
  `on_burst_turn_end: trash_top_source` is inert — their EoT self-trash does not happen.
  Their behavioral tests are structural-only for that clause, which masked this. Both are
  effectively PARTIAL on the Burst-Mode EoT teardown until this gap closes.

Fix shape: (a) distinguish `BurstDigivolve` from `BlastDnaDigivolve` in lowering; (b) when a
burst-digivolve action resolves, schedule the path's `on_burst_turn_end` steps to run at the
end of that turn (a delayed/scheduled effect keyed to the burst-digivolution turn); (c) then
the "DP-less / sub-Lv3 top card can't remain in the battle area" rules-check for the revealed
Koromon. Likely needs a DebugRunner burst-digivolve driver to pin behaviorally.
