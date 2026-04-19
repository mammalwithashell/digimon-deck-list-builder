# Rust engine gap-closure plan — Rocks

## Goal

Close the capability gaps below so that the **Rocks** archetype (47 unique cards, 6.26% meta share, 118 decklists) can be fully implemented in `digimon-engine/` without stubs, approximations, or auto-selections (per `CLAUDE.md` §17–18).

All 47 cards audited on 2026-04-18 by `/assess-archetype-rust` are currently **🔴 BLOCKED**. 0 🟢 SUPPORTED. 0 🟡 PARTIAL. Rocks' central engine is the inherited-stack trash-and-recur loop (place Mineral/Rock sources → trash them → inherited observers fire → gain value), so the archetype blocks on `OnDigivolutionCardTrashed` plus a handful of cross-cutting primitives. Outside that core, Rocks is a standard Option-heavy control archetype, so it also loads on every pre-existing Option / Delay / native-keyword / phase-granular-timing gap.

Re-run `/assess-archetype-rust Rocks` if card text, `deck_library.json`, or the Rust API evolves. Audit snapshot was generated against `deck_library.json` last updated 2026-04-04.

## Gaps to close (ordered by blast radius across the Rocks card pool)

Primary load-bearing gaps for Rocks — these block the most cards:

1. **OnDigivolutionCardTrashed observer timing** (🔴 NEW, blocks ~25 clauses across 13+ cards) — archetype's central engine; fires when a `CardSource` leaves a permanent's `card_sources` via effect. See `docs/RUST_ENGINE_GAPS.md` §"`OnDigivolutionCardTrashed` observer timing".
2. **Place card at a specific stack position (bottom-source / under another permanent) + alt-digivolve + stack reorder** (🔴, pervasive — nearly every Rocks Digimon and all three "Close" Tamers place-to-bottom-source from trash / hand). See existing entry with Rocks target-scope extensions: self / another trait-matched Digimon / Tamer.
3. **Native printed keyword parsing** (🔴, 13+ cards) — Rocks needs new `Keyword::Fragment(u8)` + `Save` + Collision-grant wiring added to the registry-time auto-emission pipeline. See existing entry.
4. **Option card play flow + Plug-In / Link mechanic** (🔴, 11 Option cards + 1 Plug-In) — EX10-069, LM-031, LM-032, P-107, P-039, EX8-070, BT9-103, P-206, EX7-074, BT23-096, ST22-11. See existing entry.
5. **`<Delay>` keyword + placement-turn gating** (🔴, 7 cards) — adds new non-turn-start triggers (EX10-069 Delay via OnSuspend observer). See existing entry.
6. **Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)** (🔴, 15 cards) — play-from-revealed-free is a new zone (EX8-050). Cost-delta variant is load-bearing (BT21-021).
7. **Selection: opponent-as-selecting-player, cross-side target, union-zone (hand OR trash), DNA-pair, multi-pick from reveal** (🔴, 13 Rocks cards).
8. **Selection: ordered permutation** (🔴, 5 Rocks cards — reveal + return rest in any order).
9. **Phase-granular turn timings** (🔴, 13 Rocks cards — `StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `StartOfYourTurn` actual fire, `[End of Your Turn]`).
10. **De-Digivolve N primitive** (🔴, 6 Rocks cards — inherited + effect-driven).
11. **Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated + pay-cost builder hook)** (🔴, ~10 Rocks cards) — extend Rocks fix-plan to include: digivolve-cost variant with target-trait filter (BT21-055), cross-permanent count-capped source-trash-as-cost (EX10-033, EX11-044), closure reading both players' trash (P-186). Note the Tamer/Digimon-suspend-as-cost-on-triggered-body class is split out as a NEW gap (see #12) since it's not scoped to `calculate_play_cost`.
12. **`.pay_cost()` builder hook for triggered non-cost-reduction effects** (🔴 NEW, 9 Rocks cards) — extends the .pay_cost surface from BeforePayCost-only into arbitrary triggered-queue dispatches (OnDigivolve / [When Moving] / OnAnyDeletion / OnDigivolutionCardTrashed / OnOpponentAttack / OptionMain). See `docs/RUST_ENGINE_GAPS.md` §"`.pay_cost()` builder hook for triggered non-cost-reduction effects".
13. **`<Fragment (N)>` keyword** (🔴 NEW, 6 Rocks cards — all mid/late Rocks Digimon carry Fragment). Leave-field replacement via multi-source self-trash; requires `WhenWouldBeDeleted` framework + native-keyword parsing + `OnDigivolutionCardTrashed` + `ctx.select_n_own_sources(n, target)`.
14. **Cross-permanent count-capped multi-select** (🔴 NEW, 8 Rocks cards) — `ctx.select_source_across_own_permanents` + `ctx.select_n_sources_across_own_permanents`. Rocks needs both single-pick and multi-pick variants with PASS terminator.
15. **Zone-manipulation: reveal-top-N deck + add-to-hand + hatch** (🔴, 7 Rocks cards).
16. **Observer timings tied to specific events** (🔴, ~11 Rocks cards) — `OnDigivolve` trait-filter, `OnSuspend` of named card, `[When Moving]` (archetype-pervasive on DigiEggs + Tamers), `OnOpponentAttack` dispatch (new sub-case; enum declared, not fired), `OnAllyAttack` with trait-filter.
17. **`WhenWouldBeDeleted` / leave-field replacement-effect framework** (🔴, 6 Rocks Fragment cards + EX7-049 "leave other than by one of your effects").
18. **Zone-manipulation: effect-initiated digivolve** (🔴, 8 Rocks cards) — reduced-cost, alt-digivolve at cost-3 ignoring reqs, passive trait-gated reduction, per-activation scoped reduction.
19. **Player-scoped modifier registry** (🔴, 8 Rocks cards) — adds bilateral `CannotPlayDigimonByEffect` (BT14-009), bilateral `CannotReducePlayCost` (ST13-08), `CannotAddSecurityByEffect` (BT9-103). See new gap "`ModifierType::CannotAddSecurityByEffect`" and new gap "Declarative-aura → player-scoped modifier delivery".
20. **Selection: multi-select (count-capped sibling of aggregate-sum)** (🔴, 8 Rocks cards) — Rocks forces the entry to split count-cap from DP-sum-cap explicitly.
21. **Zone-manipulation: return-to-hand / return-to-deck (top/bottom) / bounce self / trash-from-hand** (🔴, 9 Rocks cards) — add return-from-trash-to-deck-TOP (LM-031, LM-032) and trash-from-hand-by-index (EX8-046, EX11-038, EX11-065).
22. **Zone-manipulation: security stack operations** (🔴, 3 Rocks cards — plus see new "`ModifierType::CannotAddSecurityByEffect`").
23. **Conditional security-in-stack trigger** (🔴 NEW, 1 Rocks card — BT20-055). See `docs/RUST_ENGINE_GAPS.md` §"Conditional security-in-stack trigger".
24. **Source-scoped return-immunity modifiers (`CannotBeReturnedToHand/Deck`, `CannotBeDeDigivolved`)** (🔴 NEW, 3 Rocks cards).
25. **`ModifierType::GrantCollision`** (🔴 NEW, 3 Rocks cards). Minor enum addition.
26. **`<Piercing>` combat-time security continuation after winning battle** (🔴 NEW, 3 Rocks cards).
27. **Effect-driven attack cancellation (`ctx.end_pending_attack()`)** (🔴 NEW, 1 Rocks card — EX10-003). Combined with `OnOpponentAttack` dispatch + `.pay_cost()` extension.
28. **Global `OnOptionCardTrashed` observer timing** (🔴 NEW, 1 Rocks card — BT23-059).
29. **Granted triggered ability** (🔴, 1 Rocks card — EX10-034 cross-side grant with controller-anchored "Your Turn" reference).
30. **Condition-gated modifier entries + new Expiry variants** (🔴, 5 Rocks cards — adds `EndOfOpponentsNextTurn` several times + source-scoped return immunity + turn-scoped ImmunityToOpponentEffects).
31. **`<Progress>` keyword + `ImmunityToOpponentEffects` modifier** (🔴, 1 Rocks card — BT23-059 turn-scoped variant).
32. **Raid target-switch interrupt + effect-driven attack redirect** (🔴, 1 Rocks card — EX8-050 `ctx.redirect_attack` on defender-side observer).
33. **Force-follow-up-attack / "may attack without suspending"** (🔴, 1 Rocks card — EX10-034 cross-side forced attack).
34. **Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings** (🔴, 2 Rocks cards — BT8-094, EX11-065).
35. **Global `OnOpponentSecurityRemoved` observer timing** (🔴, 2 Rocks cards — EX10-036 trash opp top security; BT20-055 face-up security-check observer).
36. **`ctx.move_from_breeding()` EffectContext helper** (🔴 NEW, 1 Rocks card — P-130).
37. **Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`)** (🔴 NEW, 1 Rocks card — BT14-009).
38. **DigiXros name alias** (🔴 NEW, 1 Rocks card — BT21-021).
39. **Plug-In re-link from battle area source zone** (🔴 NEW, 1 Rocks card — ST22-11).
40. **`<Digi-Burst N>` keyword** (🟡 NEW, 1 Rocks card — BT4-072, absorbable into #12 .pay_cost hook).
41. **Trait-filter helpers on `CardSource` / `Permanent`** (🟡, archetype-pervasive — every Rocks card needs case-insensitive `has_any_type(&[&str])` for [Mineral]/[Rock]/[LIBERATOR]/[Ice-Snow]/[Rock Dragon]/[Earth Dragon]/[CS]/[Xros Heart]/[Blue Flare]/[Hero]). Promote from 🟡 — likely belongs with the native-keyword-parsing + card-data-ingestion architectural phase.
42. **Ergonomics: dual/tri-timing composite clause builder** (promote from 🟡 → 🔴 for Rocks — pervasive). Also aggregate filter helpers (lowest/highest play cost, cross-kind Digimon-OR-Tamer targeting) and if-effect-didn't-resolve on-decline callback.

## Cards affected (verbatim list, for regression targeting)

All 47 cards below are BLOCKED today. Grouped by deck-frequency tier.

**Tier 1 — 4-of staples (deck_frequency 100+)**:
- EX10-032 Proganomon — gaps [`OnDigivolutionCardTrashed`, place-as-bottom-source+alt-digivolve, `GrantCollision`, cross-permanent count-capped multi-select, De-Digivolve N, trait-filter]
- P-167 Landramon — gaps [`OnDigivolutionCardTrashed`, place-as-bottom-source, reveal-top-N+add-to-hand, ordered permutation, phase-granular (`StartOfYourMainPhase`), De-Digivolve N, cross-permanent source select, trait-filter]
- EX10-069 Unique Emblem: Gravel Hearts — gaps [Option card play flow, `<Delay>` keyword (non-turn-start trigger), play-from-hand/trash free, effect-initiated digivolve, union-zone select, Observer timings (OnSuspend of named), trait-filter]
- EX10-036 Magneticdramon — gaps [`<Fragment (N)>` keyword, `WhenWouldBeDeleted` framework, place-as-bottom-source+alt-digivolve, security stack ops (trash opp top), cross-permanent source select, phase-granular (`WhenAttacking`), trait-filter]
- EX8-067 Close — gaps [Observer timings (`OnDigivolve` trait-filter), place-as-bottom-source, `.pay_cost()` triggered hook (suspend-Tamer), trait-filter]
- EX8-047 Sunarizamon — gaps [reveal-top-N+add-to-hand, multi-pick-from-reveal with two filters, ordered permutation, `OnDigivolutionCardTrashed`, trait-filter]
- EX8-048 Landramon — gaps [play-from-hand-free with name filter, `OnDigivolutionCardTrashed`, trait-filter]
- EX8-005 Tumblemon (DigiEgg) — gaps [`OnDigivolutionCardTrashed` (minimal form)]
- BT21-055 Sunarizamon — gaps [Dynamic cost reduction (digivolve-cost with target-trait filter), `OnDigivolutionCardTrashed`, trait-filter]
- EX8-051 Proganomon — gaps [native keyword parsing (Collision/Piercing/Fragment), `<Piercing>` combat continuation, `<Fragment (N)>`, `WhenWouldBeDeleted` framework, `OnDigivolutionCardTrashed`, De-Digivolve N, trait-filter]
- EX10-033 Pyramidimon — gaps [`<Fragment (N)>`, `WhenWouldBeDeleted`, phase-granular (`WhenAttacking`), place-as-bottom-source, cross-permanent count-capped multi-select, `OnDigivolutionCardTrashed`, Dynamic cost reduction (selection-gated), Condition-gated modifier entries, trait-filter]
- EX10-063 Close — gaps [phase-granular (`StartOfYourMainPhase`), return-to-deck-bottom as activation cost, play-from-hand/trash free, `OnDigivolutionCardTrashed`, `.pay_cost()` triggered hook]
- LM-031 Black Scramble — gaps [Option card play flow, `<Delay>`, effect-initiated digivolve, return-from-trash-to-deck-TOP, play-from-hand/trash free, phase-granular (`StartOfYourTurn`), ordered permutation]
- EX10-025 Sunarizamon — gaps [place-as-bottom-source (from-trash, target another own trait-matched), count-capped multi-select, `OnDigivolutionCardTrashed`, trait-filter]
- P-107 Defense Training — gaps [Option card play flow, `<Delay>`, reveal-top-N, ordered permutation, effect-initiated digivolve (per-activation scoped cost reduction)]
- EX10-028 Landramon — gaps [Observer timings (`OnDigivolve` trait-filter), `OnDigivolutionCardTrashed`, single-source cross-permanent select, trait-filter]

**Tier 2 — 2–3 of (deck_frequency 30–100)**:
- BT16-082 Ukkomon (hatch helper), P-169 Close, EX8-055 Pyramidimon (Fragment), P-039 Black Memory Boost!, EX8-070 Zofr Kabus, EX10-034 Blastmon (Fragment + granted triggered ability), EX8-046 Gotsumon (trash-from-hand), P-215 Icemon ([When Moving] + tri-timing + source-scoped return immunity), EX7-049 Metallicdramon (De-Digivolve 4 + WhenWouldBeDeleted with source attribution + play-from-trash-free), EX8-050 Gogmamon (play-from-revealed-free + `ctx.redirect_attack` on defender-side observer), P-206 Digital Gate Open, BT14-009 Gotsumon (bilateral player-scoped aura), BT9-103 Kongou (`CannotAddSecurityByEffect`), EX11-038 Sunarizamon, EX11-065 Close

**Tier 3 — 1-of tech / engine (deck_frequency 1–20)**:
- EX11-044 Pyramidimon (Fragment + Reboot native), BT20-055 Invisimon (conditional security-in-stack trigger + flip face-up security), ST13-08 Chikurimon (bilateral `CannotReducePlayCost`), BT4-072 Gogmamon (`<Digi-Burst 1>` + `EndOfOpponentsNextTurn`), EX10-003 Tumblemon (`ctx.end_pending_attack()` + `OnOpponentAttack`), BT18-064 Mercurymon (source-scoped return immunity), P-123 Ukkomon (hatch helper), BT23-096 Comet Hammer, LM-032 Purple Scramble, ST22-11 Defense Plug-In F (Plug-In re-link from battle area), BT21-021 OmniShoutmon (DigiXros name alias + native Save), EX7-074 Vortex Resonance, P-186 Gallantmon (closure-valued cost reduction + Recovery +1 Deck + on-decline callback + native Rush+Blocker), BT23-059 Justimon: Blitz Arm (`OnOptionCardTrashed` + trash-Option-as-cost + turn-scoped ImmunityToOpponentEffects), BT8-094 Digimon Emperor, P-130 Lui Ohwada (`ctx.move_from_breeding()`)

## Relevant references

- [`docs/RUST_ENGINE_API.md`](../../docs/RUST_ENGINE_API.md) — existing scripting surface (`EffectContext`, `Effect` builder, `EffectTiming`, `Keyword`, `ModifierType`, `Expiry`)
- [`docs/RUST_ENGINE_GAPS.md`](../../docs/RUST_ENGINE_GAPS.md) — **read all consolidated gap entries before planning**; each carries a `Suggested API shape:` line that the plan should reference rather than re-derive
- [`docs/RUST_PYTHON_PARITY.md`](../../docs/RUST_PYTHON_PARITY.md) — cross-engine divergences; several Rocks gaps are parity-driven (RUST_PYTHON_PARITY §2.3 combat interrupts, §2.5 security resolution, §4.6d-residual `select_source`)
- [`digimon-engine/src/effect_context.rs`](../../digimon-engine/src/effect_context.rs) — where most new helpers land
- [`digimon-engine/src/effect.rs`](../../digimon-engine/src/effect.rs) + [`digimon-engine/src/effect_queue.rs`](../../digimon-engine/src/effect_queue.rs) — timings + triggered-effect plumbing; extend `.pay_cost()` dispatch in `run_queued_effect`
- [`digimon-engine/src/modifiers.rs`](../../digimon-engine/src/modifiers.rs) — modifier registry (`ModifierType` enum expansion: `GrantCollision`, `CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, `CannotAddSecurityByEffect`, `CannotPlayDigimonByEffect` bilateral, `CannotReducePlayCost` bilateral)
- [`digimon-engine/src/combat.rs`](../../digimon-engine/src/combat.rs) — `<Piercing>` security-continuation branch, `ctx.end_pending_attack()`, `OnOpponentAttack` dispatch, conditional security-in-stack iteration from `begin_turn` / `end_turn`
- [`digimon-engine/src/enums.rs`](../../digimon-engine/src/enums.rs) — enum expansions: `EffectTiming::OnDigivolutionCardTrashed`, `SecurityOnEndOpponentsTurn` (+ siblings), `Keyword::Fragment(u8)`, `Keyword::DigiBurst(u8)`, `TrashCause { Effect, Combat, DigivolveOverflow, ReturnToDeck }`, `Expiry::EndOfOpponentsNextTurn` / `EndOfTargetsNextTurn`
- [`digimon-engine/src/card_data.rs`](../../digimon-engine/src/card_data.rs) — `keywords: Vec<Keyword>` ingestion pass + `digixros_aliases: Vec<String>` field for BT21-021
- [`digimon-engine/src/game.rs`](../../digimon-engine/src/game.rs) — aura-query evaluation for player-scoped modifier delivery (BT14-009), DigiXros recipe matching (BT21-021), phase-granular timing dispatch (`enter_main_phase`)
- [`digimon-engine/src/cards/bt17/mod.rs`](../../digimon-engine/src/cards/bt17/mod.rs) — currently empty template; Rocks cards live in a sibling `cards/<set>/` submodule once added (sets touched by Rocks: EX7, EX8, EX10, EX11, BT4, BT8, BT9, BT14, BT16, BT18, BT20, BT21, BT23, LM, P, ST13, ST22)
- [`digimon-engine/tests/test_cards_behavioral.rs`](../../digimon-engine/tests/test_cards_behavioral.rs) — TDD harness; per-clause behavioral tests land alongside each new primitive (CLAUDE.md §18)
- [`qa/archetype-qa/engine-gaps.md`](../../qa/archetype-qa/engine-gaps.md) — Python-scoped gaps; DO NOT edit (scoped to Python engine only, per /assess-archetype-rust skill contract)

## Ask

Produce a phased implementation plan via `superpowers:writing-plans` that:

1. **Groups gaps by subsystem**, in this order (each subsystem is a phase):
   - **Phase A — Timings & dispatch infrastructure** (foundation): add `EffectTiming::OnDigivolutionCardTrashed` + `StartOfYourMainPhase` + `WhenAttacking` + `EndOfAttack` + `SecurityOnEndOpponentsTurn` (+ siblings) + `OnOpponentAttack` + `OnAllyAttack` + `OnDigivolve` + `OnSuspend` + `WhenMoving` + `OnOptionTrashedAnywhere` + `OnAnyDigimonPlayed` + `OnAnyDeletion`. Emit from the correct sites. Scaffolds every downstream phase.
   - **Phase B — Zone-manipulation primitives** (foundation): `reveal_top` + `add_to_hand` + `hatch` + `move_from_breeding` + `trash_from_hand` + `return_permanent_to_hand/deck(top|bottom)` + `return_trash_to_deck(top|bottom)` + `play_from_hand_free` + `play_from_trash_free` + `play_from_hand_with_cost_delta` + `play_from_revealed_free` + `play_from_security_at(index)` + `place_as_bottom_source` + `place_security_top/bottom`. Fan out into `OnAddToHand` / new `OnDigivolutionCardTrashed` where appropriate.
   - **Phase C — Selection-space infrastructure**: `select_source_across_own_permanents` + `select_n_sources_across_own_permanents(ExactN | UpToN)` + `select_multiple_trash(max_count)` + `select_ordering` + `select_hand_or_trash` + `select_any_permanent` + `select_hand_of(player, ...)`. New action-space ranges mirroring Python's `SEL_SOURCE`. Tensor slots updated to expose cross-permanent source handles.
   - **Phase D — Builder hooks**: broaden `EffectBuilder::pay_cost(Fn(&mut EffectContext) -> bool)` to every timing (not only `BeforePayCost`). Hook in `effect_queue::run_queued_effect` before `.process`. Implement sugar: `.pay_cost_suspend_self()`, `.pay_cost_return_self_to_deck_bottom()`, `.pay_cost_trash_self_option()`, `.pay_cost_trash_n_own_sources_by_trait(n, traits)`, `.pay_cost_trash_top_source_of_self()`, `.pay_cost_move_source_to_bottom(source_index)`. Dual-/tri-timing composite builder `EffectBuilder::on_timings(&[EffectTiming])`.
   - **Phase E — Replacement-effect framework**: `EffectTiming::WhenWouldBeDeleted` + `ReplacementContext::cancel_leave()` + `source_player` attribution. Keywords: `Keyword::Fragment(u8)`, `Keyword::ArmorPurge`, `Keyword::Decode(Vec<Color>, u8)`. Per-keyword auto-emission from native-keyword parsing (Phase F).
   - **Phase F — Card-data ingestion**: `CardData::keywords: Vec<Keyword>` populated from `effect_text` at registry-build time (covers Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N, Collision, Fragment, Save). `CardData::digixros_aliases` for BT21-021. `CardData::ace_overflow: Option<i8>` (DNA Omnimon prerequisite; include here to avoid two passes).
   - **Phase G — Modifier extensions**: new `ModifierType` variants — `GrantCollision`, `CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, `CannotReducePlayCost` (bilateral), `CannotAddSecurityByEffect`, `CannotPlayDigimonByEffect` (bilateral). New `Expiry::EndOfOpponentsNextTurn` / `EndOfTargetsNextTurn`. Player-scoped modifier registry (`ModifierRegistry::player_modifiers`). Condition closure on `ModifierEntry`. Declarative aura emission → player-scoped modifier delivery with `UntilLeaveField` scope via `ModifierRegistry::PlayerModifierFromPermanent` bucket.
   - **Phase H — Combat extensions**: `<Piercing>` winning-battle → security-continuation loop (reuse `resolve_player_security_loop` without re-firing interrupts). `ctx.end_pending_attack()` state-machine mutation. Raid `RaidOpen` state with target-switch observer. Effect-driven `ctx.redirect_attack(new_target)` from defender-side observers. Conditional-security-in-stack iterator (`begin_turn` / `end_turn` → enqueue matching security-slot effects). Cross-side granted triggered ability with controller-anchored "Your Turn" reference (EX10-034).
   - **Phase I — Option card play flow + Plug-In**: branch `play_from_hand` on `CardKind::Option`; fire `OptionMain` / `OptionSecurity`; `ctx.place_self_in_battle_area()` / `ctx.trash_self_from_option()` / `ctx.activate_own_main_effects()`. Delay keyword tracking (`turn_played > placed_turn` gate) + activation via conditional triggers (OnSuspend, StartOfYourTurn-with-guard, OnOpponentSecurityRemoved). Plug-In `linked_cards: Vec<CardSource>` storage + `ctx.link_plug_in(PlugInSource::Hand | BattleArea, target)` + link-requirement metadata on `CardData`. Security-effect dispositions: `place-self-in-battle-area`, `add-self-to-hand`.
   - **Phase J — De-Digivolve N + effect-initiated digivolve**: `ctx.de_digivolve(target, amount)` + `ctx.de_digivolve_all_opponent(amount)` (mass variant). Level-3 floor rule. `ctx.prompt_digivolve(base_filter, target_filter, reduction, is_optional, callback)` with per-activation-scoped cost reduction (P-107 distinction). `ctx.digivolve_into_source_from_hand(target, hand_index, bottom_trash_index, cost_override, ignore_reqs)` for alt-digivolve.
   - **Phase K — Cost-reduction extensions**: `.cost_reduction_fn(|&EffectReadContext| i16)` closure-valued variant wired into `calculate_play_cost` / `calculate_digivolve_cost`. Cross-player aggregate reads (both trashes). Target-trait-filtered digivolve-cost reduction (BT21-055). Selection-gated cost reduction that installs a cost-time prompt.
   - **Phase L — Aura / named-target / declarative**: `Effect::aura(card).target_filter(...).grants_keyword(...).dp_modifier(...)`. Declarative-aura emission of player-scoped modifiers with `UntilLeaveField` scope (BT14-009). DigiXros name alias pass (scoped). (Lower-priority than other phases for Rocks specifically — only ~3 Rocks cards need these; deferrable if phases A–K are already massive.)

2. **Orders phases by dependency**: A → B → C → D → E/F/G (parallel) → H/I/J/K (parallel, each depends on A–G) → L. Critical path: A (timings) + B (zones) + C (selections) + D (builder hooks) are blockers for everything else.

3. **For each phase, lists**:
   - Exact function signatures / enum variants added (cite the `Suggested API shape:` line from each gap entry in `docs/RUST_ENGINE_GAPS.md`).
   - Tests to write FIRST (TDD per CLAUDE.md §18) — name the `digimon-engine/tests/*.rs` file and test-case names. Use a canonical Rocks card as the anchor per phase: Phase A → EX8-005 Tumblemon (minimal form of `OnDigivolutionCardTrashed`), Phase B → P-167 Landramon (reveal + place-as-bottom-source), Phase C → EX10-033 Pyramidimon (cross-permanent multi-select), Phase D → EX8-067 Close (triggered `.pay_cost` suspend-Tamer), Phase E → EX8-051 Proganomon (Fragment cancel-delete), Phase F → native-keyword parse of Collision/Piercing/Fragment on EX8-051, Phase G → BT9-103 Kongou (`CannotAttackPlayer` filtered) + BT14-009 Gotsumon (bilateral declarative aura), Phase H → EX10-003 Tumblemon (`ctx.end_pending_attack()`), Phase I → EX10-069 Unique Emblem (Option + Delay + Security), Phase J → EX7-049 Metallicdramon (De-Digivolve 4), Phase K → BT21-055 Sunarizamon (digivolve-cost reduction with trait filter), Phase L → BT14-009 bilateral aura.
   - Affected Rust files — use the `Key files` column in `docs/RUST_ENGINE_GAPS.md` At-a-glance table as the starting point.
   - Parity implications — does this phase resolve or create a `docs/RUST_PYTHON_PARITY.md` entry? (Phase A closes §2.5b for `OnDigivolutionCardTrashed` adjacency; Phase H §2.3 combat interrupts + §2.5 security continuation; Phase C §4.6d-residual `select_source`; Phase F §2.1b / §2.5f native keywords).

4. **Flags any gap that is architectural** and should have its own spec under `docs/superpowers/specs/` before implementation:
   - Native-keyword parsing pipeline (Phase F) — touches `CardData` schema, ingestion, tensor slots. Spec candidate.
   - Replacement-effect framework (Phase E) — touches `WhenWouldBeDeleted` dispatch + replacement context + source-attribution axis. Spec candidate.
   - Option card play flow + Plug-In mechanic (Phase I) — a whole new play subsystem. Spec candidate, referenced in RUST_ENGINE_API §9 as a known gap.
   - Player-scoped modifier registry (Phase G) — extends `ModifierRegistry` storage shape. Lighter spec, but worth documenting the enum expansion contract.

5. **Does NOT attempt to close every gap in one phase**. Phases A–D are the critical path and must land first (likely 2–3 distinct plans each given their scope). Phase H–L can proceed in parallel once A–G are in. Prefer 4–6 distinct `writing-plans` invocations rather than a monolith.

6. **Prioritizes for Rocks first playable**: After Phases A + B + C + D + subsets of E (Fragment) + F (Collision/Piercing/Fragment native parse) + G (modifier variants) land, the **Tier 1 staples are authorable**. Phase I (Option flow) is the second-biggest Rocks gate after the `OnDigivolutionCardTrashed` + place-as-bottom-source pair.

## Audit summary for this archetype

| Metric | Value |
|---|---|
| Archetype | Rocks |
| Meta share | 6.26% (118 decklists in DigimonMeta / Egman / DigiLab) |
| Unique cards audited | 47 |
| Already in Rust registry | 0 |
| 🟢 SUPPORTED | 0 |
| 🟡 PARTIAL | 0 |
| 🔴 BLOCKED | 47 |
| Distinct NEW gaps filed | 15 |
| Existing gaps touched (Rocks cards added) | 24 |

Audit run on 2026-04-18 by `/assess-archetype-rust Rocks` against [`deck_library.json`](../../digimon_gym/engine/data/deck_library.json) last built 2026-04-04 by `tools/meta_loader.py`.
