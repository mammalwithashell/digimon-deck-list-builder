# DNA Omnimon Rust DSL/Engine Gap Source Document

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

The archetype remains partially blocked by remaining specialized cross-zone DNA follow-ups, immediate attack, security-removed observers, and mass-selection semantics. The Track B Counter-window DNA, leave-field replacement, and Decode/material-play slices are now implemented for the covered fixtures and should not be treated as whole-archetype blockers without a new failing card case.

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
- **Status update (2026-05-08):** The reusable Counter-window field+hand Blast DNA path is now implemented and covered by BT20-060, BT17-078, BT20-045, BT20-076, BT20-081, EX6-011, and EX6-029. The Counter mask exposes the result-card action, then pending selections choose the field material and hand material, stack both under the result, fire `WhenDigivolving` / `OnDnaDigivolve` / `OnDigivolve`, and preserve `dna_origin` through parked target-selection continuations. `kind: blast_dna_digivolve` lets card YAML carry exact printed material predicates; BT17-078 accepts WarGreymon + MetalGarurumon and rejects broad Greymon + Garurumon. The selected-level mass bottom-deck branch for BT17-078 is also implemented via `bind_permanent_property` + `level_eq_binding`. Proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078_counter_blast_dna`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078_blast_dna_bottom_decks_same_level_then_prompts_delete`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_060_hand_counter_blast_dna_uses_alphamon_and_ouryumon bt20_060_dna_origin_trashes_security_and_recovers`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_045_counter_blast_dna bt20_076_counter_blast_dna bt20_081_counter_blast_dna ex6_029_counter_blast_dna`.
- **Remaining capability:** two-field-material Counter Blast DNA variants, if printed, still need their own pending-selection route. AD1-012's defender-side effect DNA clause remains blocked by effect-initiated DNA during the attack interrupt, not by Counter Blast DNA.
- **First regression:** For any future two-field-material Counter Blast DNA card, set up opponent attack into a Digimon target while the result card is in defender hand and both materials are on the defender field. The Counter mask must expose the Blast DNA action, then pending selections must choose both field materials, perform DNA, fire When Digivolving, and resume the attack state correctly.
- **Implementation hint:** `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, plus DSL lowering for a mixed-material DNA verb.

### 2. Leave-field replacement framework with cause discrimination

- **Type:** engine-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under `WhenWouldBeDeleted` / leave-field replacement-effect framework
- **Blocks:** `BT17-095`, `BT22-015`, `EX4-060`, `AD1-025`, `EX5-015`, `AD1-012`, `AD1-014`
- **Cross-archetype value:** Decode, Partition, Armor Purge, Evade, Fragment, Scapegoat, non-battle leave observers, own-effect vs opponent-effect prevention.
- **Status:** Track B framework closed/narrowed 2026-05-08 for the reusable replacement substrate: leave-field replacement context carries cause, subject, destination, and battle/non-battle semantics; optional replacements decline through `PendingSelection`; non-cancelling subscribers can run side effects then proceed; Decode/material play is live for `BT22-015`, EX4-060, and EX9-021; and inherited/cross-permanent replacement scans are covered in `--test replacements`.
- **Remaining capability:** Card-specific DNA Omnimon residuals now center on follow-up text that is not replacement-specific, such as global security-removed observers, immediate attacks, and broader King Drasil/source-stack plays.
- **First regression:** `BT17-095` in battle area should offer its Delay only when a level 6 Greymon/Garurumon would leave outside battle, not when deleted in battle. Declining the Delay must allow the leave event to continue unchanged.
- **Implementation hint:** Centralize leave-field attempts in engine movement/deletion helpers before destination mutation; thread cause/controller/source-player into replacement evaluation.

### 3. Decode and play-from-material source selection

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under Decode keyword
- **Status:** Closed 2026-05-07 for `BT22-015`; narrowed 2026-05-08 for EX4-060 and EX9-021 sequential source-play follow-ups. Broader batch / different-name material plays remain tracked under `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`.
- **Blocks:** EX10-061-style batch / different-name multi-source plays, not BT22-015, EX4-060, or EX9-021.
- **Cross-archetype value:** Decode-style mechanics, Partition variants, source extraction, play material without paying cost, source-trigger interactions.
- **Closed capability:** `BT22-015` leaving outside battle offers an optional source-selection prompt for matching Lv.3 material, with separate Red/Black and Blue/Yellow color gates. `EX4-060` and `EX9-021` can run sequential named/trait source picks, play those sources without paying costs, and resolve their follow-up security placement tails.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015_decode`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_021`.
- **Implementation hint:** `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/cards/keyword_effects.rs`, plus DSL/keyword metadata for Decode filters.

### 4. Immediate follow-up attack and attack without suspending

- **Type:** engine-gap, dsl-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` under force-follow-up-attack; `qa/dsl-vocab-gaps.md` under `G-MAY-ATTACK-NOW`
- **Status:** resolved for the listed immediate prompt routes as of 2026-05-08.
- **Formerly blocked:** `BT22-015`, `BT20-102`, `AD1-009`, `EX9-013`
- **Cross-archetype value:** Any "then it may attack", "then that Digimon attacks", or "attack without suspending" effect that occurs inside an effect resolution rather than the normal attack window.
- **Implemented capability:** `ctx.may_attack_now_optional(...)` / DSL `may_attack_now` open the centralized attack flow from effect resolution, expose PASS for printed optionality, and support `without_suspending`. Mandatory flows use DSL `force_attack`.
- **Regression evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_013_eot_clause_contains_post_dna_may_attack_now ex9_013_eot_after_dna_one_digimon_may_attack`; BT20-102/AD1-009/BT22-015 card-shaped tests also exercise the same primitive.

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
- **Blocks:** closed for `BT17-078`
- **Cross-archetype value:** Any "choose 1, affect all with same level/play cost/name/trait/color" card text.
- **Status:** resolved 2026-05-07. `bind_permanent_property` binds a selected permanent property, currently `level`, and `level_eq_binding` lets later predicates compare against that captured value.
- **First regression:** `BT17-078` selects one opponent Digimon, returns all opponent Digimon with that selected level to the bottom of deck, then deletes one remaining opponent Digimon if any are legal. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_bind_permanent_level_property_step bind_permanent_level_filters_for_each_same_level_permanents` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078`.
- **Implementation hint:** Use `bind_permanent_property: { from: chosen, property: level, bind_as: chosen_level }` followed by a selector or for-each predicate with `level_eq_binding: chosen_level`.

### 7. Stack-derived formulas and same-level pair counts

- **Type:** dsl-gap, engine-gap if formula inputs are unavailable
- **Status:** Closed 2026-05-07 for BT22-015. Formula-bound `select_count_capped_multi` now supports `{ formula: ... }` counts, `zone: battle_area` permanent picks, and `per_selected` bottom-deck resolution. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015_when_digivolving_bottom_decks_n_opp_digimon_per_same_level_pair`.
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` dynamic formula sections; `qa/dsl-vocab-gaps.md` formula/residual entries
- **Blocks:** `BT22-015`
- **Cross-archetype value:** Stack-depth formulas, source-count formulas, per-N source scaling, source-level grouping.
- **Closed capability:** `BT22-015` groups the source stack by level, sums floor(count / 2), uses the result as a capped target count, and bottom-decks the selected opponent Digimon before the follow-up attack prompt.
- **Regression:** A `BT22-015` stack with two Lv.6 sources, two Lv.5 sources, and one unmatched source allows exactly two opponent Digimon to be selected for bottom-decking.
- **Implementation note:** The reusable formula term remains `same_level_pairs_in_sources`; the reusable selector surface is `select_count_capped_multi.max.formula`.

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
- **Updated 2026-05-08:** DSL now accepts the printed timing token `when: on_any_digimon_played` as an alias of `OnEnterFieldAnyone`, sharing the existing `EnteredField` payload and one fan-out path. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- new_effect_timings_are_constructible` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_any_digimon_played_alias_uses_enter_field_payload`. Hand-resident fan-out and ally-digivolve authoring remain in this gap.
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
