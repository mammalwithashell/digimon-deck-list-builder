# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-05-15
Last sweep: 2026-05-15 (post-rebaseline audit cleanup)

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
- **Status:** Partially resolved 2026-05-02 for the BT22-098/Arisa suspend timing slice. Delayed Option permanents now store `DelayTrigger::OnEvent(EffectTiming::OnSuspend)` plus placement turn, observed suspend events carry event-card context, and Delay activation is gated until after the placement turn before trashing itself through the replacement-aware cost path. DSL `kind: delay` can lower `trigger: on_suspend` plus `active_when: { event_card_name_contains: "Arisa Kinosaki" }`.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- event_gated_delay_only_fires_after_placement_turn_and_matching_event`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_event_trigger_lowers_to_on_event_delay`.
- **Updated 2026-05-08:** Self-scoped suspend observers can use `event_permanent_is_source: true` to compare the suspended event permanent with the observer source permanent. BT23-077 Sistermon Ciel uses this to avoid over-firing when another own permanent suspends. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_077`.
- **Remaining related work:** `on_ally_played` now lowers to `EffectTiming::OnAllyPlayed` and has battle-area/trash fan-out, proven by BT20-084's trash-resident observer fixture. P-229's Mirai-played Delay body still needs its card-shaped implementation and reduced-cost hand digivolution plumbing.

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

### `trash_security_card` Verb (Non-Top Security) Missing  [G-TRASH-SELECTED-SECURITY]
- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (2026-04-27).
- **Scope:** DSL + engine (hybrid).
- **Card(s):** BT24-018 — "[When Digivolving] You may trash any 1 of your opponent's security cards."
- **What's missing:** `select_security` can bind a target index but no DSL verb consumes that binding to actually trash the chosen card. Only `trash_top_security` exists. The engine likely has the primitive (security indexing already supported elsewhere); just no DSL bridge.
- **Workaround:** `raw_rust:` escape hatch.

### Trash → Deck-Bottom Move (Without Reveal Phase)  [G-ZONE-TRASH-TO-DECK]
- **Discovered in:** Medusamon archetype, BT24-017 Medusamon (Batch 3, 2026-04-27).
- **Scope:** DSL + engine (hybrid).
- **Card(s):** BT24-017 (return 2 trash to bottom of deck), BT21-029-related, EX11-012 (return 1 trash to bottom).
- **What's missing:** A DSL verb / `EffectContext` API for moving a chosen trash card to the bottom of the owner's main deck. Existing `return_to_deck_from_reveal` works for cards in the reveal zone, not trash.
- **Workaround:** EX11-012 implementer added a `raw_rust: ex11_012_return_trash_to_deck_bottom` (6-line bridge in `src/cards/raw_rust/mod.rs`). Generalizing it as a first-class DSL verb is the proper fix.

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
- **Remaining limits:** Group 4 now covers effect-initiated movement to/from the real breeding slot and bottom-source placement under the `BREEDING_TARGET` selected breeding permanent. The 2026-05-08 `PlayerBreedingArea` slices cover `StartOfYourMainPhase` and security-removal fan-out while the source remains in breeding; other event fan-outs from breeding remain under `G-BREEDING-TRIGGER-DISPATCH`.

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
- **Discovered in:** BT16-085 Davis Motomiya & Ken Ichijoji implementation (2026-05-04)
- **Card(s):** BT16-085 — "[Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from your hand without paying the cost. At the next end of your opponent's turn, return it to the hand."
- **Effect text:** "return it to the hand" — the "it" refers to the permanent that was just played free.
- **What's missing:** `CompiledStep::PlayFromHandFree` has no `bind_as` field. When `execute_play_from_hand_free` runs in `play_digivolve.rs`, the returned `Option<PermanentHandle>` is discarded. The `schedule_delayed` step clones bindings at schedule time, so if the just-played permanent's handle were inserted into bindings via `bind_as`, a subsequent `return_to_hand: { target: played }` in the delayed body could reference it. Without `bind_as`, the delayed return step cannot be expressed — `return_to_hand` requires a bound `PermanentHandle`.
- **Suggested change:** Add `bind_as: Option<String>` to `PlayFromHandFreeArgs` in `digimon-dsl/src/step.rs` and `CompiledStep::PlayFromHandFree` in `compiled.rs`. In `execute_play_from_hand_free` (or its caller in `step.rs`), if the play succeeded and `bind_as` is set, call `bindings.insert_permanent(name, handle)` so the resulting permanent is available for downstream steps (including `schedule_delayed` body steps).
- **Workaround:** None — the delayed-return sub-clause of BT16-085 Clause 0 is omitted from the YAML. The test `bt16_085_start_of_main_played_digimon_returns_at_opponent_eot` is `#[ignore = "BLOCKED: G-PLAY-FROM-HAND-FREE-BIND-AS"]`.

### `event_card_color_has` Predicate Missing (Color-Gate on Digivolve/Enter Observer)  [G-EVENT-CARD-COLOR-IS]
- **Discovered in:** BT16-085 Davis Motomiya & Ken Ichijoji implementation (2026-05-04)
- **Card(s):** BT16-085 — "[Your Turn] When one of your Digimon digivolves into a **blue or green** Digimon, by suspending this Tamer, gain 1 memory." Also related: any card whose observer is conditioned on the entering/digivolving card's color.
- **Effect text:** "digivolves into a blue or green Digimon" — a color-containment check on the new top card of the digivolving permanent.
- **What's missing:** `PredicateSpec` (DSL) and `CompiledPredicate` (engine) have `event_card_trait_has` and `event_card_name_contains` predicates that inspect the entering/digivolving card, but no equivalent predicate for checking color membership. Related: PUPPETS-G023 (`event_card_color_only`, `event_card_color_count`) tracks exact multi-color checks; a single-color containment check (`event_card_color_has: blue`) belongs to the same family and is equally absent. Without it, BT16-085 Clause 1's "blue or green" gate cannot be expressed and the observer over-fires on any own Digimon digivolve.
- **Suggested change:** Add `event_card_color_has: Option<CompiledColor>` to `CompiledPredicate` and the matching leaf to `BoolPredicateSpec` / `PredicateSpec` in `digimon-dsl`. In `eval_predicate` (`predicate.rs`), implement the check by calling `event_target_card(rctx)`, resolving its `digimon_colors` from card_data, and testing for color membership.
- **Workaround:** Color gate omitted from YAML — observer over-fires on any own Digimon digivolve. Test `bt16_085_digivolve_observer_does_not_fire_on_non_blue_non_green_digivolve` is `#[ignore = "BLOCKED: G-EVENT-CARD-COLOR-IS"]`.

### Opponent Digivolution-Card Source Selection Missing  [G-SELECT-OPPONENT-SOURCES]
- **Discovered in:** BT16-085 Davis Motomiya & Ken Ichijoji implementation (2026-05-04)
- **Card(s):** BT16-085 — "[Your Turn] … If DNA digivolving, trash any 3 digivolution cards under your opponent's Digimon."
- **Effect text:** "trash any 3 digivolution cards under your opponent's Digimon" — selects up to 3 source cards from a specific opponent permanent's card_sources stack.
- **What's missing:** DSL has `select_own_sources` / `trash_selected_sources`, and as of 2026-05-07 `select_own_sources.target` can restrict the own-source picker to a specific own permanent binding. There is still no `select_opponent_sources` verb for targeting a specific OPPONENT permanent's card_sources. The opponent permanent itself must also first be selected (requires a field selection step). Both opponent-side pieces are missing.
- **Suggested change:** Add `select_opponent_sources: { target: <binding>, count: N, bind_as: <name> }` DSL verb, mirroring `select_own_sources`. `target` resolves to an opponent `PermanentHandle` binding. `count` specifies how many sources to select (up to the permanent's stack depth). Implement in `step.rs`, lower in `compile.rs`, and execute in a new `CompiledStep::SelectOpponentSources` handler analogous to `execute_select_own_sources`.
- **Workaround:** DNA trash sub-clause of BT16-085 Clause 1 is entirely omitted from the YAML while opponent-source selection is missing. The former `G-DSL-IS-DNA-DIGIVOLVING` blocker is resolved by `dna_origin: true`; the remaining blocker is `G-SELECT-OPPONENT-SOURCES`. Test `bt16_085_dna_digivolve_trashes_3_opp_digi_cards` should narrow its ignore tag when this card is revisited.

### OPT Reset via Attack Cycle  [G-OPT-RESET-VIA-ATTACK-CYCLE]
- **Discovered in:** BT16-040 Wormmon (2026-05-04 batch-implement-cards-rust-dsl)
- **Card(s):** BT16-040, plus all inherited [When Attacking] [OPT] cards.
- **Effect text:** "[When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon."
- **What's missing:** OPT lockout for inherited When Attacking does not reliably reset after a full turn cycle (player end_turn → opponent end_turn → player attacks again). Test `bt16_040_opt_resets_after_turn_cycle` is `#[ignore]`'d.
- **Suggested change:** Investigate OPT key reset path in turn-state machine for inherited triggered clauses. The OPT key may persist across turn boundaries when the carrier permanent's source identity differs from the trigger source.
- **Workaround:** OPT-reset behavioral test is `#[ignore]`'d; structural OPT flag and same-turn lockout still verified.
