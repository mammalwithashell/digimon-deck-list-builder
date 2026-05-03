# DNA Omnimon Rust DSL/Engine Gap Source Document

**Date:** 2026-05-03
**Status:** Source document for compiling a cross-archetype Rust DSL/engine gap spec
**Assessment target:** `DNA Omnimon` from `data/deck_library.json`

## Purpose

This document distills the remaining Rust engine and YAML DSL gaps surfaced by the DNA Omnimon archetype into reusable capability groups. It is not a card implementation plan. It is intended to feed a future spec for tackling the remaining cross-archetype DSL and engine gaps without treating every blocked card as a one-off.

The controlling rule is the repository no-approximations policy: every gameplay choice must be represented through engine actions or `PendingSelection`. Do not close these gaps with hidden auto-selections, no-op placeholders, UI-only decisions, or broad raw-Rust bypasses.

## Sources

- Printed card text: `data/cards.json`
- Archetype target and card frequency: `data/deck_library.json` entry `DNA Omnimon`
- Rules context: `docs/RULES_CONTEXT.md`, especially DNA Digivolution section 8-2
- Existing gap trackers: `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `docs/RUST_PYTHON_PARITY.md`
- Current authored Rust YAML: `code/digimon-engine/cards/`
- Current engine tests: `code/digimon-engine/tests/`

This document supersedes the legacy Python-lane faithfulness claim in `qa/archetype-qa/DNA_Omnimon.md` only for Rust YAML/engine readiness. That older file remains useful as historical Python QA context.

## Current Verdict

DNA Omnimon is **blocked** for faithful Rust DSL implementation.

The base engine now supports more DNA infrastructure than the older 2026-04-17 audit reflected:

- Normal hand-card DNA digivolve action masks and two-stage material selection.
- DNA digivolve execution through shared stack construction.
- Effect-initiated DNA digivolve for two battle-area materials plus one hand evolution card.
- `on_dna_digivolve` DSL timing.
- Top-level `alt_paths: kind: dna_digivolve` authoring into runtime `CardData.dna_costs`.
- Inherited end-of-turn DNA registration for the v1 literal-cost, two-material action path.

The archetype remains blocked because its most important cards need specialized cross-zone DNA, Counter-window DNA, leave-field replacement, Decode/material play, immediate attack, and mass-selection semantics.

## Core Archetype Cards And Pressure Points

The local `DNA Omnimon` decklists most heavily use:

| Card | Role | Required behavior that still matters for Rust readiness |
|---|---|---|
| `BT22-017` Gabumon | Search / inherited DNA registration | Reveal filtering plus inherited EOT DNA path. V1 inherited DNA is now supported, but full card authoring/coverage still needs production YAML. |
| `BT17-095` Miraculous Mega Knight | Option/Delay engine card | Play Agumon/Gabumon from hand or trash, persist as Delay, observe non-battle leave, then DNA using a leaving field material plus hand material into an Omnimon in hand. |
| `BT22-008` Agumon | Search / inherited DNA registration | Reveal/trash-to-hand plus inherited EOT DNA path. V1 inherited DNA is now supported, but full card authoring/coverage still needs production YAML. |
| `BT22-084` Nokia Shiramine | Tamer support | Memory setter, free play, DP aura, security play. Example YAML exists but production card readiness needs coverage and Tamer security-route verification. |
| `BT22-013` WarGreymon | Lv.6 material / payoff | Hand Main warp digivolve ignoring requirements, modal When Digivolving, inherited Omnimon security trash. |
| `BT22-026` MetalGarurumon | Lv.6 material / payoff | Hand Main warp digivolve ignoring requirements, modal When Digivolving, inherited Omnimon unsuspend. |
| `BT17-078` Omnimon | Ace top end | Blast DNA from Counter using field plus hand material, DNA-origin branch, same-level mass bottom-deck, then delete. |
| `BT22-015` Omnimon | Top end | Printed Decode, per-two-same-level stack count bottom-decking, then immediate attack. |
| `EX9-021` Omnimon Alter-S | Alternate top end | DNA-origin immunity, highest-level mass delete, End of Attack play from sources, place self as top security. |
| `AD1-009` BlitzGreymon | Alternate Lv.6 material | EOT DNA into Alter-S, then follow-up attack; temporary opponent-effect immunity for itself and a Garurumon-named ally. |
| `AD1-012` CresGarurumon | Alternate Lv.6 material | Reactive DNA on opponent attack, attack-target redirect, Evade, bounce lowest-level target. |

## Reusable Gap Backlog

### 1. Counter-window Blast DNA with mixed field plus hand materials

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under Counter hand-play / Blast DNA residuals and selection DNA-pair residuals
- **Blocks:** `BT17-078`, `BT17-095`, `AD1-009`, `AD1-012`
- **Cross-archetype value:** Any future Blast DNA, effect DNA using a material from hand, defender-side reactive DNA, or named-material fusion route.
- **Missing capability:** Counter timing can perform ordinary Blast Digivolve, and effect-initiated DNA can consume two battle-area materials, but DNA Omnimon needs pair selection where one material may be a field Digimon and another may be a card in hand, with the evolution card also in hand.
- **First regression:** Set up opponent attack into a Digimon target while `BT17-078` is in defender hand, `WarGreymon` is on field, and `MetalGarurumon` is in hand. The Counter mask must expose the Blast DNA action, then a pending selection must choose the hand material, perform DNA, fire When Digivolving, and resume the attack state correctly.
- **Implementation hint:** `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, plus DSL lowering for a mixed-material DNA verb.

### 2. Leave-field replacement framework with cause discrimination

- **Type:** engine-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under `WhenWouldBeDeleted` / leave-field replacement-effect framework
- **Blocks:** `BT17-095`, `BT22-015`, `EX4-060`, `AD1-025`, `EX5-015`, `AD1-012`, `AD1-014`
- **Cross-archetype value:** Decode, Partition, Armor Purge, Evade, Fragment, Scapegoat, non-battle leave observers, own-effect vs opponent-effect prevention.
- **Missing capability:** Replacement checks need to see why a card would leave, who caused it, whether it is battle or non-battle, and whether the prevention is optional or cost-gated. DNA Omnimon particularly needs "would leave the battle area outside of a battle" and "other than by one of your effects".
- **First regression:** `BT17-095` in battle area should offer its Delay only when a level 6 Greymon/Garurumon would leave outside battle, not when deleted in battle. Declining the Delay must allow the leave event to continue unchanged.
- **Implementation hint:** Centralize leave-field attempts in engine movement/deletion helpers before destination mutation; thread cause/controller/source-player into replacement evaluation.

### 3. Decode and play-from-material source selection

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under Decode keyword
- **Blocks:** `BT22-015`
- **Cross-archetype value:** Decode-style mechanics, Partition variants, source extraction, play material without paying cost, source-trigger interactions.
- **Missing capability:** Current keyword handling does not model printed Decode as "select a matching source and play it for free". It needs `SelectSource` over the triggering stack and a movement helper that pops that source card, creates a fresh permanent, and fires On Play.
- **First regression:** `BT22-015` leaving outside battle should offer a source-selection prompt for matching Lv.3 material. Selecting a legal source plays that material without paying cost and does not redirect the original Omnimon to hand/deck.
- **Implementation hint:** `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/cards/keyword_effects.rs`, plus DSL/keyword metadata for Decode filters.

### 4. Immediate follow-up attack and attack without suspending

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under force-follow-up-attack; `qa/dsl-vocab-gaps.md` under `G-MAY-ATTACK-NOW`
- **Blocks:** `BT22-015`, `BT20-102`, `AD1-009`, `EX9-013`
- **Cross-archetype value:** Any "then it may attack", "then that Digimon attacks", or "attack without suspending" effect that occurs inside an effect resolution rather than the normal attack window.
- **Missing capability:** Existing attack masks cover ordinary attack windows and some end-of-turn granted attack shapes, but there is no effect step that starts an immediate attack on a bound permanent while optionally skipping suspension.
- **First regression:** `BT22-015` When Digivolving resolves its bottom-deck effect and then offers an attack for that same Digimon even if memory has passed. If selected, it starts combat through the normal attack state machine.
- **Implementation hint:** Add an engine primitive such as `ctx.attack_now(target, without_suspending, optional)` and a DSL step like `force_attack_now` or `may_attack_now`.

### 5. Option, Delay, and security disposition completeness

- **Type:** engine-gap, dsl-gap, test-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under Option card play flow; `qa/dsl-vocab-gaps.md` Delay-related entries
- **Blocks:** `BT17-095`, `LM-034`, `BT22-099`, `ST20-15`, `BT23-096`, `ST2-13`, `EX1-068`
- **Cross-archetype value:** Memory Boosts, Scrambles, Training cards, battlefield Options, security effects that add to hand or place in battle area.
- **Missing capability:** Group 5 closed major Delay and scheduled end-of-turn pieces, but DNA Omnimon still needs coverage for Ace Option persistence, security "add this card to hand", security "place this card in battle area", Tamer security play routing, and multi-color Option color semantics.
- **First regression:** `BT17-095` Main effect plays a legal Agumon/Gabumon from hand or trash, then places itself in battle area as a Delay Option. Its Security effect plays a legal Tai/Matt card from hand or trash and adds the Option to hand.
- **Implementation hint:** Verify existing Group 5 option flow before adding new primitives; keep disposition explicit rather than encoding it as card-local cleanup.

### 6. Bind selected property, then mass-apply to matching permanents

- **Type:** dsl-gap
- **Tracker:** `qa/dsl-vocab-gaps.md` under `BT17-078 -- bottom-deck all opponent Digimon sharing chosen level`
- **Blocks:** `BT17-078`
- **Cross-archetype value:** Any "choose 1, affect all with same level/play cost/name/trait/color" card text.
- **Missing capability:** The DSL can select targets and iterate permanents, but needs a clean way to bind a selected permanent's property and use that bound value as a later predicate.
- **First regression:** `BT17-078` selects one opponent Digimon, returns all opponent Digimon with that selected level to the bottom of deck, then deletes one remaining opponent Digimon if any are legal.
- **Implementation hint:** Add a property binding form such as `bind_property: { from: chosen, property: level, as: chosen_level }` and allow predicate comparisons to binding values.

### 7. Stack-derived formulas and same-level pair counts

- **Type:** dsl-gap, engine-gap if formula inputs are unavailable
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` dynamic formula sections; `qa/dsl-vocab-gaps.md` formula/residual entries
- **Blocks:** `BT22-015`
- **Cross-archetype value:** Stack-depth formulas, source-count formulas, per-N source scaling, source-level grouping.
- **Missing capability:** `BT22-015` needs "for every 2 same-level cards in this Digimon's stack, return 1 opponent Digimon to bottom deck". This requires grouping the source stack by level, summing floor(count / 2), and using the result as a capped target count.
- **First regression:** A `BT22-015` stack with two Lv.3, two Lv.4, and one Lv.6 source should allow exactly two opponent Digimon to be selected for bottom-decking.
- **Implementation hint:** Add formula terms for source-stack grouping and bind the computed count into a count-capped selection.

### 8. Source-scoped immunity and `CannotBeAffected` enforcement

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` condition-gated modifiers; `qa/dsl-vocab-gaps.md` EX10-010 immunity notes
- **Blocks:** `EX9-021`, `AD1-009`, `EX10-010`, `ST20-11`
- **Cross-archetype value:** Any "opponent's effects don't affect this Digimon", source-scoped immunity, DP-gated immunity, or until-opponent-turn-end protection.
- **Missing capability:** The DSL can express some modifier-like shapes, but enforcement and source scoping must be checked before applying opponent effects. DNA Omnimon also needs immunity scoped to selected allies and to DNA-origin conditions.
- **First regression:** `EX9-021` DNA digivolves, gains immunity to opponent effects for the turn, and an opponent deletion/bounce effect cannot affect it while same-side effects still can.
- **Implementation hint:** Wire modifier installation and checks into effect application paths, not only combat masks.

### 9. Hand-resident and global observer fan-out for play/digivolve events

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under global OnAnyDigimonPlayed / OnAnyDeletion observer timings and observer timings tied to specific events
- **Blocks:** `EX9-019`, `EX9-012`, `AD1-001`, `AD1-010`, `BT17-081`, `EX4-061`, `EX4-039`, `EX4-038`
- **Cross-archetype value:** Any hand-resident "when your Digimon/Tamer is played or digivolves" effect, Tamer observer, or ally digivolve observer.
- **Missing capability:** Trigger fan-out needs consistent event payloads for hand, field, inherited, Tamer, and breeding sources. The event must identify the entering or digivolving permanent/card so filters do not inspect the observer itself.
- **First regression:** `AD1-001` in hand observes an ally Garurumon/Tai being played or digivolving, then offers the printed free digivolve from hand into a Greymon-named card.
- **Implementation hint:** Extend event dispatch and `TriggerContext` payloads before adding card-specific YAML.

### 10. Reveal multi-pick with per-category filters and ordered rest handling

- **Type:** dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` ordered permutation and reveal sections; `qa/dsl-vocab-gaps.md`
- **Blocks:** `BT22-017`, `EX4-039`, `EX4-038`, `BT12-059`, `BT22-099`, `LM-034`, `BT22-094`
- **Cross-archetype value:** Search cards that add one card per category, return rest in any order, or allow multiple picks from one revealed set.
- **Missing capability:** The DSL needs reusable multi-pick from revealed cards where each pick can have a separate predicate and the remaining cards can be bottom-decked or top/bottom ordered according to printed text.
- **First regression:** `BT22-017` reveals, adds one qualifying Omnimon-text card and one qualifying CS trait card when both are present, then returns the rest in the required order/disposition.
- **Implementation hint:** Reuse existing ordered permutation and reveal selection infrastructure; add per-slot category constraints and stable reveal references.

### 11. Production YAML authoring and raw-Rust retirement for DNA Omnimon core

- **Type:** data-gap, test-gap, dsl-gap where raw-Rust remains
- **Tracker:** `qa/dsl-vocab-gaps.md`; `docs/RUST_ENGINE_GAPS.md`; card YAML comments
- **Blocks:** `BT17-078`, `BT22-015`, `EX9-021`, `BT17-095`, `BT22-013`, `BT22-026`, `BT22-017`, `BT22-008`
- **Cross-archetype value:** Converts reusable primitives into real cards and catches remaining DSL omissions.
- **Missing capability:** Several useful slices exist only as `_examples`, while major top-end DNA Omnimon cards are not production YAML. `BT20-102` still uses raw Rust for board wipe/return despite several predicate gaps being closed since that YAML was written.
- **First regression:** Move one core DNA Omnimon card from assessment to production YAML only after its reusable primitive exists, with a behavioral test under `code/digimon-engine/tests/cards_behavioral/`.
- **Implementation hint:** Author cards in small readiness slices. Do not mark full card readiness when omitted text still contains a player choice, timing hook, or replacement effect.

## Spec Compilation Notes

The future cross-archetype spec should group work by capability rather than by DNA Omnimon card:

1. **Counter and DNA selection surfaces:** mixed material selection, Counter-window Blast DNA, effect DNA variants.
2. **Leave-field and replacement semantics:** Decode, Partition, Evade, non-battle leave, source-player/cause filters.
3. **In-effect attack starts:** optional attack, mandatory attack, attack without suspending.
4. **Source and stack operations:** SelectSource, play material free, stack-derived formulas, same-level grouping.
5. **Event context fan-out:** hand-resident observers, ally digivolve/play observers, event payloads.
6. **Option and security dispositions:** Delay persistence, add-to-hand, place-in-battle-area, Tamer security play routing.
7. **Modifier enforcement:** source-scoped immunity, CannotBeAffected, player-only attack grants, target-scoped expiry.
8. **Card authoring cleanup:** production YAML for core DNA Omnimon cards after primitives are proven.

## Acceptance Gates For Any Gap-Closure Spec

- No `ACTION_SPACE_SIZE` or active tensor contract expansion unless a gap proves impossible with existing pending-selection action IDs. If expansion is needed, split it into a separate action/tensor contract plan.
- Every player-visible choice must be exposed via action mask or pending selection, including one-card choices where the player can decline.
- Each primitive must land with a failing Rust test first.
- DSL syntax should lower to engine primitives; do not add YAML vocabulary that compiles into no-op behavior.
- Tracker updates must distinguish closed, partially closed, and still-open sub-shapes.
- Archetype readiness should be re-run after each capability group, not only after all cards are authored.

## Suggested First Slice

Start with `BT17-078` only if the goal is maximum archetype identity, because Blast DNA exercises Counter, DNA material selection, and mass level-based bottom-decking. Start with `BT22-015` only if the goal is maximum cross-archetype reuse, because Decode, leave-field replacement, source selection, stack formulas, and immediate attacks unlock more future card families.

For a spec intended to tackle cross-archetype gaps, `BT22-015` is the better first anchor: it forces the most reusable primitives and has clear acceptance tests.
