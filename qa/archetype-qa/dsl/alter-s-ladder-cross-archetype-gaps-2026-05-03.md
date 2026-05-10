# Alter-S Ladder Cross-Archetype DSL / Engine Gap Source

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.


Date: 2026-05-03

Assessment source: `qa/archetype-qa/dsl/alter-s-ladder-2026-05-03.md`, refreshed from `data/deck_library.json` archetype `Alter-S Ladder` after the DigiLab and Egman import on 2026-05-03.

This document distills the Alter-S Ladder readiness assessment into reusable DSL/engine work items that can feed a later cross-archetype implementation spec. It is not a card implementation plan. Alter-S Ladder still needs production YAML and card-level behavioral tests for most of the 17-card pool; the goal here is to separate that authoring backlog from remaining shared capability gaps.

## Current Verdict

`blocked`

The refreshed `Alter-S Ladder` pool has one exact DigiLab decklist. Only 3 of 17 unique card IDs are currently reported as implemented by `digimon_engine.load_implemented_card_ids()`: `BT16-082`, `EX10-010`, and `EX9-013`. Those three are partial, with ignored tests still covering no-op raw Rust bodies, omitted printed clauses, and missing effect-initiated attack/immunity support.

The reusable blockers cluster around source-stack play, effect-generated attacks, target-change observers, face-down sources, multi-zone reveal/selection workflows, and conditional immunity. These are broader than Alter-S and should be compiled as shared engine/DSL primitives before the archetype is called playable.

## Card Authoring Backlog, Not Cross-Archetype Gaps

These cards need production YAML and card-specific tests, but should not become new shared gap specs unless implementation proves a missing primitive.

| Area | Alter-S Ladder cards | Required local work |
|---|---|---|
| Existing partial cards | `BT16-082`, `EX10-010`, `EX9-013` | Replace no-op/partial bodies, unignore card tests, and use existing predicates where now supported. |
| Egg and lower-level setup | `EX10-002`, `BT16-082`, `P-101`, `EX9-068`, `P-128` | Author draw/discard, search, played-Digimon observer, memory setter, and inherited/tamer effects. |
| Lv.5/Lv.6 shell | `EX10-008`, `EX9-011`, `EX10-010`, `EX9-013`, `EX9-020`, `EX5-048` | Author Collision, forced attack, face-down source, De-Digivolve, bottom-deck, leave-field replacement, and ACE tests. |
| Lv.7 payoff and tech | `EX4-060`, `EX9-021`, `BT5-087`, `BT5-112`, `BT17-077` | Author source-stack play, security placement, mass deletion/bottom-deck, trash play, security play, and mass cleanup tests. |
| Option package | `BT15-096`, `BT5-087` source-cost clauses | Author reveal/add/trash ordering, Delay play reduction, source-return costs, and up-to-N trash play choices. |

## Remaining Reusable Gap Candidates

### G-ASL-01: Source-stack pair play and self-to-security leave-field replacement

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `EX4-060`, `EX9-021`, `EX9-020`
- **Cross-archetype value:** Partition-like, Decode-like, and Lv.7 replacement effects often need to inspect the triggering stack, select one or more sources, play those cards, then move the original card to security or another destination.
- **Printed behavior examples:** `EX4-060` plays a `BlitzGreymon` and a `CresGarurumon` from its digivolution cards when it would leave by an opponent effect, then places itself at the bottom of security. `EX9-021` can play two named/trait sources at end of attack and place itself on top of security.
- **Missing capability:** Replacement/effect steps that can select multiple cards from the triggering source stack by name or trait, play them without paying costs, preserve legal player choices through pending selections, and place the original card at top or bottom security after the selected sources move.
- **Why it matters:** Auto-picking sources hides a player-visible choice; playing from trash or hand instead of the exact source stack changes both board state and hidden information.
- **Spec should cover:** replacement cause filters, source-stack candidate predicates, multi-pick order, simultaneous or sequential play semantics, original-card routing, top/bottom security placement, and no-op behavior when required sources are unavailable.
- **First test:** Set up `EX4-060` with one `BlitzGreymon` and one `CresGarurumon` source, make it leave by an opponent effect, assert the pending selection exposes the legal source choices, then assert both sources are played and `EX4-060` is placed at bottom security.
- **Likely files:** `code/digimon-engine/src/replacements/`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/selection.rs`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/step/`.

### G-ASL-02: Effect-initiated immediate and forced attacks

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** reusable primitive resolved for EX9-013 and cost-upgraded attack openings as of 2026-05-08; still relevant to EX10-008 / EX5-048 card authoring.
- **Formerly blocked:** `EX9-013`, `EX10-008`, `EX5-048`
- **Cross-archetype value:** DNA Omnimon, BG Imperial, Vortex-style cards, and forced-attack enablers all need effect-generated attacks that reuse normal combat legality while preserving optionality or mandatory timing.
- **Printed behavior examples:** `EX9-013` lets one of your Digimon attack after its end-turn DNA process. `EX10-008` and `EX5-048` can cause an opponent's Digimon to attack at start of main phase.
- **Implemented capability:** `may_attack_now` and `force_attack` cover effect-resolution attacks that can be optional or mandatory, can target "this Digimon", one selected own Digimon, or an affected opponent Digimon, and use the normal attack target/action mask machinery. The optional `cost_upgrade: { dp, security_attack }` payload now applies attack-only DP/security riders through `AttackOpen` and tears them down at `EndOfAttack`.
- **Why it matters:** Omitting the attack loses printed payoff; auto-attacking hides legal choices and can choose the wrong target.
- **Spec should cover:** PASS/decline for optional attacks, mandatory prompt behavior, attacker scoping, target legality reuse, memory/turn-state legality, suspended/can-attack checks, and interactions with "can't attack" modifiers.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_013_eot_clause_contains_post_dna_may_attack_now ex9_013_eot_after_dna_one_digimon_may_attack`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- attack_open_cost_upgrade_applies_for_attack_only`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- may_attack_now_cost_upgrade_yaml_lowers_to_compiled_step`.
- **Likely files:** `code/digimon-engine/src/action/`, `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/tests/dsl/`.

### G-ASL-03: Attack-target-change observer and Collision/Raid event fan-out

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `EX10-002`, `EX10-008`
- **Cross-archetype value:** Collision, Raid, Block, and other target-substitution effects need stable event context so inherited and face-up observers can react to "when an attack target is switched".
- **Printed behavior examples:** `EX10-002` draws when an attack target is switched. `EX10-008` has inherited removal tied to the attack target being switched to an opponent's Digimon.
- **Missing capability:** A normalized `attack_target_changed` event emitted for all target-change paths, including granted Collision/Raid-style paths, with old target, new target, attacker, controller, and reason context available to DSL predicates.
- **Why it matters:** If each target-switch mechanic handles observers separately, inherited effects will miss legal triggers or double-trigger depending on the path.
- **Spec should cover:** event payload, once-per-switch fan-out, inherited observer timing, granted keyword provenance, target predicates, and interaction with blocker/redirect effects.
- **First test:** Give an attacker an inherited `EX10-008` effect, switch its attack target to an opponent Digimon through a Collision-like path, and assert the inherited security trash effect sees exactly one target-change event.
- **Likely files:** `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/dsl_cards/lower_triggered.rs`, `code/digimon-dsl/src/predicate.rs`.

### G-ASL-04: Face-down source representation, predicates, and formulas

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `EX9-011`, `EX9-068`
- **Cross-archetype value:** Face-down source tuck, hidden-source counts, and source-state-based formulas appear across multiple decks and should not require card-specific raw Rust.
- **Printed behavior examples:** `EX9-011` places a card from trash as a face-down bottom digivolution card and scales deletion DP by the number of face-down digivolution cards. `EX9-068` can place a card from hand as a face-down source.
- **Missing capability:** First-class face-down source state, including movement into sources face down, preservation through source-stack movement, predicates/count formulas over face-down sources, and public/private observation behavior.
- **Why it matters:** Treating a face-down source as a normal visible source leaks hidden information and makes DP/count formulas impossible to script faithfully.
- **Spec should cover:** face-down card identity ownership, visibility rules, source count formulas, bottom-source placement, movement out of a face-down state, and tensor/privacy notes if observation metadata needs adjustment.
- **First test:** Resolve an `EX9-011`-shaped effect that tucks one trash card face down, then verify a later DP-budget deletion gains exactly `+2000` per face-down source and the source remains marked hidden.
- **Likely files:** `code/digimon-engine/src/zones.rs`, `code/digimon-engine/src/game.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/predicate.rs`, `code/digimon-dsl/src/formula.rs`.

### G-ASL-05: Reveal multi-pick with mixed destinations and ordered remainder

- **Type:** `dsl-gap`
- **Blocks:** `BT16-082`, `BT15-096`
- **Cross-archetype value:** Searchers, Trainings, Memory Boosts, and many options reveal cards, choose multiple categories for different destinations, then place the rest on top or bottom in a defined order.
- **Printed behavior examples:** `BT16-082` reveals 3, adds a Digimon/Tamer, then bottoms the rest. `BT15-096` reveals 5, adds one Machine/Cyborg, trashes one Machine/Cyborg, then returns the rest to the top of the deck.
- **Missing capability:** A reusable reveal workflow with multiple pick clauses, mixed destination routing, optional/mandatory category constraints, and ordered remainder placement.
- **Why it matters:** Separate ad hoc reveal prompts can choose in the wrong order, lose revealed-state tracking, or place the remainder incorrectly.
- **Spec should cover:** reveal zone lifetime, pick ordering, category-specific destination clauses, PASS behavior where printed, top/bottom ordering, and hidden-information restoration after resolution.
- **First test:** Resolve a `BT15-096`-shaped reveal where exactly one eligible card can be added and one can be trashed, then assert both choices resolve and all unchosen revealed cards return to top deck in the selected order.
- **Likely files:** `code/digimon-engine/src/selection.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/step/`.

### G-ASL-06: Conditional/source-scoped effect immunity

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `EX10-010`, `EX9-021`
- **Cross-archetype value:** ACE, DNA, and boss Digimon often gain immunity from specific opponent effect sources, sometimes only if they have named or trait sources.
- **Printed behavior examples:** `EX10-010` gains immunity from opponent Digimon effects while its source condition is met. `EX9-021` gains opponent-effect immunity for the turn after DNA digivolving.
- **Missing capability:** Scriptable `CannotBeAffected`/effect-immunity modifiers with controller, source kind, source-card, duration, and condition filters, enforced before opponent effects mutate protected permanents.
- **Why it matters:** A decorative immunity status that does not gate deletion, bottom-decking, DP reduction, source trashing, or bounce effects is not rules-complete.
- **Spec should cover:** immunity source taxonomy, continuous vs duration-bound checks, source-condition reevaluation, replacement interaction order, and behavioral tests for every affected movement/removal class introduced in the same spec.
- **First test:** Give `EX10-010` its source-condition immunity, resolve an opponent Digimon deletion effect and an opponent option deletion effect, and assert only the Digimon-source effect is blocked.
- **Likely files:** `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/replacements/`, `code/digimon-dsl/src/modifier.rs`, `code/digimon-dsl/src/predicate.rs`.

### G-ASL-07: Mass source cleanup and bottom-deck returns from trash

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `BT17-077`
- **Cross-archetype value:** Paladin-style cleanup, trash recycling, and mass source removal effects need reusable movement primitives over both players' trash and opponent source stacks.
- **Printed behavior examples:** `BT17-077` trashes all opponent Digimon's digivolution cards, returns all cards from both trashes to the bottom of the deck, and conditionally gains memory when a white Lv.7 card is returned.
- **Missing capability:** Mass movement over opponent source stacks and both trash zones, preserving owner routing to deck bottoms, with conditional counters/formulas based on moved cards.
- **Why it matters:** Rebuilding this as one-card raw Rust would duplicate a broadly useful cleanup primitive and risk moving cards to the wrong owner's deck.
- **Spec should cover:** all-opponent-source traversal, both-trash iteration, bottom-deck owner routing, movement counters, moved-card predicates, and memory rider formulas.
- **First test:** Resolve a `BT17-077`-shaped effect with opponent sources and both trashes populated, assert sources are trashed first, all trash cards return to each owner's deck bottom, and memory is gained if a white Lv.7 was among returned cards.
- **Likely files:** `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/zones.rs`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/step/`.

### G-ASL-08: Union-zone and up-to-N play selectors with cost/color filters

- **Type:** `dsl-gap`
- **Blocks:** `BT5-087`, `BT15-096`, `BT5-112`
- **Cross-archetype value:** Security and option effects frequently play cards from hand/trash/security/reveal zones with cost, color, level, and count filters.
- **Printed behavior examples:** `BT5-087` may play up to two black/purple Digimon cards with play cost 8 or less from trash. `BT15-096` Delay plays a Lv.5+ Machine/Cyborg with a cost reduction. `BT5-112` plays itself from security and deletes an opponent Digimon.
- **Missing capability:** A DSL selector/action workflow for up-to-N candidates across one or more zones, where each candidate preserves its origin zone and applies zone-specific movement/play semantics.
- **Why it matters:** Separate prompts for each zone or each copy can change the printed choice set and make PASS/partial-pick behavior incorrect.
- **Spec should cover:** candidate identity with zone, up-to-N partial completion, PASS/done actions, cost/color/level predicates, play-with-cost-reduction hooks, and security-origin movement.
- **First test:** Resolve a `BT5-087`-shaped effect with three valid trash candidates and assert the player can choose zero, one, or two legal cards, then each selected card is played from trash without paying cost.
- **Likely files:** `code/digimon-engine/src/selection.rs`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs`.

### G-ASL-09: Property-extrema and property-bounded mass selection

- **Type:** `dsl-gap`
- **Blocks:** `EX9-021`, `EX9-020`, `EX10-010`
- **Cross-archetype value:** Many cards choose all opponent Digimon tied for highest/lowest level, delete by play-cost ceilings, or route cards by level/color/property extrema.
- **Printed behavior examples:** `EX9-021` deletes all opponent Digimon with the highest level. `EX9-020` places an opponent Lv.5 or lower Digimon at the bottom of the deck. `EX10-010` deletes an opponent Digimon/Tamer with play cost 7 or less.
- **Missing capability:** Concise DSL predicates/formulas for property extrema and property-bounded mass application, including ties and mixed permanent types.
- **Why it matters:** One-off predicates for every card will fragment target-mask logic and make legality harder to verify.
- **Spec should cover:** highest/lowest level among a controller's permanents, tie handling, mixed Digimon/Tamer predicates, play-cost comparison, and mass apply vs chosen target semantics.
- **First test:** Build a fixture with two opponent Digimon tied for highest level and one lower-level Digimon, resolve a highest-level deletion effect, and assert both tied Digimon are deleted.
- **Likely files:** `code/digimon-dsl/src/predicate.rs`, `code/digimon-dsl/src/formula.rs`, `code/digimon-engine/src/dsl_cards/step/`, `code/digimon-engine/tests/dsl/`.

## Cross-Archetype Spec Compile Notes

When compiling the next shared DSL/engine spec, do not include "missing Alter-S Ladder YAML" as a reusable primitive. Use the gap candidates above as spec inputs, then schedule card YAML authoring as a separate archetype unlock pass after the relevant primitives have behavioral coverage.

Recommended ordering:

1. `G-ASL-02` effect-initiated immediate and forced attacks is no longer blocking `EX9-013`; use the covered primitive when authoring EX10-008 / EX5-048 forced-attack cards.
2. `G-ASL-01` source-stack play and self-to-security replacement, because it unlocks the Alter-S Lv.7 payoff identity and overlaps Decode/Partition-style replacement work.
3. `G-ASL-04` face-down source representation, because it affects hidden state and may require observation/privacy review before card authoring scales.
4. `G-ASL-03` attack-target-change observer fan-out, because it should be centralized before adding Collision/Raid-heavy cards.
5. `G-ASL-05` reveal multi-pick workflows, because it removes repeated searcher/option raw-Rust pressure.
6. `G-ASL-06` conditional effect immunity, because it must be enforced at every mutation site touched by opponent effects.
7. `G-ASL-08` union-zone/up-to-N selectors and `G-ASL-09` property-extrema predicates, because they are mostly DSL vocabulary and lowering work once selection plumbing is stable.
8. `G-ASL-07` mass source/trash cleanup, because it is high impact but mainly concentrated in Lv.7 tech and can follow the core Alter-S shell.

Acceptance criteria for any resulting spec:

- No `ACTION_SPACE_SIZE` or active tensor contract change unless the spec explicitly updates `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, Rust constants, PyO3 exports, RL wrappers, frontend constants, and model metadata together.
- Every player-visible decision must be surfaced through action masks or `PendingSelection`; no auto-picks for source, reveal, trash, attack, or play choices.
- Every reusable primitive must have at least one non-card fixture test and one card-shaped regression using an Alter-S card or adjacent cross-archetype card when possible.
- Tracker updates must distinguish `docs/RUST_ENGINE_GAPS.md` engine primitives from `qa/dsl-vocab-gaps.md` vocabulary/lowering work.
- Card YAML authoring should not use no-op raw Rust placeholders to claim readiness.
