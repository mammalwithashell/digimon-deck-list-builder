# Rust Engine Gaps

Capability gaps in the Rust engine's scripting surface (`digimon-engine/`), discovered during archetype audits by `/assess-archetype-rust`. Distinct from [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md), which tracks Rust↔Python divergences in shared subsystems — this document catalogs **net-new primitives** the Rust scripting API needs before a given archetype can be implemented under the no-approximations policy (CLAUDE.md §17–18).

Format and conventions mirror `qa/archetype-qa/engine-gaps.md` (Python-scoped). Gap titles are **capability-centric**, never card-centric.

Each entry lists the cards that surfaced it, but the entry itself describes a reusable engine primitive. If two cards need the same primitive, they share one entry — not two.

> **Canonical API signatures live here.** Fix-plans in `.claude/plans/rust-engine-gaps-*.md` should reference gap titles rather than restate signatures, to prevent divergence as the engine evolves.

## At a glance

Rows link to the detailed entry below. `#cards` is the Medusamon-archetype count — most primitives unblock more cards archetype-wide. `Key files` is the primary surface the fix touches.

| Gap | Severity | #cards | Key files |
|---|---|---|---|
| [Global `OnOpponentSecurityRemoved` observer timing](#global-onopponentsecurityremoved-observer-timing) | 🔴 | 15 | `combat.rs`, `effect_queue.rs`, `enums.rs` |
| [Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings](#global-onanydigimonplayed--onanydeletion-observer-timings) | 🔴 | 3 | `game.rs`, `permanent.rs`, `effect_queue.rs` |
| [Phase-granular turn timings (`StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle`)](#phase-granular-turn-timings-startofyourmainphase-whenattacking-endofattack-endofbattle) | 🔴 | 8 | `game.rs`, `combat.rs`, `effect.rs` |
| [Observer timings tied to specific events (`OnDigivolve` trait-filter, `OnSuspend`, `OnAttackTargetChange`, `[When Moving]`)](#observer-timings-tied-to-specific-events-ondigivolve-trait-filter-onsuspend-onattacktargetchange-when-moving) | 🔴 | 6 | `game.rs`, `permanent.rs`, `combat.rs`, `enums.rs` |
| [`WhenWouldBeDeleted` / leave-field replacement-effect framework](#whenwouldbedeleted--leave-field-replacement-effect-framework) | 🔴 | 5 | `game.rs`, `effect.rs`, `enums.rs` |
| [Selection: multi-select with aggregate-sum constraint](#selection-multi-select-with-aggregate-sum-constraint) | 🔴 | 2 | `effect_context.rs`, `action/` |
| [Selection: ordered permutation (place N cards in any order)](#selection-ordered-permutation-place-n-cards-in-any-order) | 🔴 | 8 | `effect_context.rs`, `action/` |
| [Selection: opponent-as-selecting-player, cross-side target, union-zone (hand OR trash), DNA-pair](#selection-opponent-as-selecting-player-cross-side-target-union-zone-hand-or-trash-dna-pair) | 🔴 | 10 | `effect_context.rs`, `action/` |
| [Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)](#zone-manipulation-play-from-hand--trash-without-paying-cost--cost-override) | 🔴 | 11+ | `effect_context.rs`, `game.rs` |
| [Zone-manipulation: effect-initiated digivolve (free / reduced / with trait filter)](#zone-manipulation-effect-initiated-digivolve-free--reduced--with-trait-filter) | 🔴 | 8 | `effect_context.rs`, `game.rs`, `modifiers.rs` |
| [Zone-manipulation: return-to-hand / return-to-deck (top/bottom) / bounce self](#zone-manipulation-return-to-hand--return-to-deck-topbottom--bounce-self) | 🔴 | 7 | `effect_context.rs`, `permanent.rs` |
| [Zone-manipulation: reveal-top-N deck + add-to-hand + hatch](#zone-manipulation-reveal-top-n-deck--add-to-hand--hatch) | 🔴 | 10 | `effect_context.rs`, `game.rs` |
| [Zone-manipulation: security stack operations (trash top, place bottom, trash N)](#zone-manipulation-security-stack-operations-trash-top-place-bottom-trash-n) | 🔴 | 6 | `effect_context.rs`, `combat.rs` |
| [Token creation + `CardKind::Token` + Petrification Token definition](#token-creation--cardkindtoken--petrification-token-definition) | 🔴 | 3 | `card_data.rs`, `cards.rs`, `effect_context.rs` |
| [Place card at a specific stack position (bottom-source / under another permanent) + alt-digivolve](#place-card-at-a-specific-stack-position-bottom-source--under-another-permanent--alt-digivolve) | 🔴 | 2 | `effect_context.rs`, `permanent.rs`, `game.rs` |
| [Native printed keyword parsing (Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N)](#native-printed-keyword-parsing-rush-raid-piercing-blocker-reboot-jamming-blitz-vortex-alliance-security-a%C2%B1n) | 🔴 | 17+ | `card_data.rs`, `cards.rs`, `card_registry.rs` |
| [`<Progress>` keyword + `ImmunityToOpponentEffects` modifier](#progress-keyword--immunitytoopponenteffects-modifier) | 🔴 | 6 | `enums.rs`, `modifiers.rs`, `combat.rs`, `effect_context.rs` |
| [`<Armor Purge>` keyword (leave-field replacement variant)](#armor-purge-keyword-leave-field-replacement-variant) | 🔴 | 2 | `enums.rs`, `effect.rs` (builds on replacement framework) |
| [`<Training>` keyword](#training-keyword) | 🔴 | 1 | `enums.rs`, `card_source.rs`, `effect_context.rs`, `action/` |
| [`<Delay>` keyword + placement-turn gating for Option cards](#delay-keyword--placement-turn-gating-for-option-cards) | 🔴 | 6 | `enums.rs`, `effect.rs`, `action/` (builds on Option flow) |
| [Raid target-switch interrupt (scripting-surface, not mask-only)](#raid-target-switch-interrupt-scripting-surface-not-mask-only) | 🔴 | 5+ | `combat.rs`, `enums.rs` |
| [De-Digivolve N primitive (single + mass)](#de-digivolve-n-primitive-single--mass) | 🔴 | 2 | `effect_context.rs`, `permanent.rs` |
| [Ace Overflow: inherited memory penalty on zone-change from field / under-card](#ace-overflow-inherited-memory-penalty-on-zone-change-from-field--under-card) | 🔴 | 4 | `card_data.rs`, `game.rs`, `effect.rs` |
| [Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated)](#dynamic-cost-reduction-at-beforepaycost-closure-valued--selection-gated) | 🔴 | 4 | `effect.rs`, `game.rs` |
| [Dynamic DP scaling modifier (per-stack-depth / per-opponent-board)](#dynamic-dp-scaling-modifier-per-stack-depth--per-opponent-board) | 🔴 | 2 | `effect.rs`, `tensor.rs` |
| [Condition-gated modifier entries](#condition-gated-modifier-entries) | 🔴 | 1 | `modifiers.rs`, `effect.rs` |
| [Player-scoped modifier registry (CannotPlayFromTrash, CannotPlayDigimonByEffect, OpponentCannotReduceDigivolveCost, IgnoreColorRequirement)](#player-scoped-modifier-registry-cannotplayfromtrash-cannotplaydigimonbyeffect-opponentcannotreducedigivolvecost-ignorecolorrequirement) | 🔴 | 6+ | `modifiers.rs`, `enums.rs`, `action/`, `effect_context.rs` |
| [Option card play flow (resolve + trash vs. place-on-field; [Main]/[Security] activation) + Plug-In / Link mechanic](#option-card-play-flow-resolve--trash-vs-place-on-field-mainsecurity-activation--plug-in--link-mechanic) | 🔴 | 11 | `game.rs`, `effect.rs`, `effect_context.rs`, `action/` |
| [Scheduled end-of-turn effect queue (for transient Options)](#scheduled-end-of-turn-effect-queue-for-transient-options) | 🔴 | 1 | `game.rs`, `effect_context.rs` |
| [Effect re-firing / cross-timing self-trigger](#effect-re-firing--cross-timing-self-trigger) | 🔴 | 1 | `effect_context.rs`, `effect_queue.rs` |
| [Force-follow-up-attack / "may attack without suspending" script helpers](#force-follow-up-attack--may-attack-without-suspending-script-helpers) | 🔴 | 6 | `effect_context.rs`, `modifiers.rs`, `combat.rs` |
| [Trait-filter helpers on `CardSource` / `Permanent`](#trait-filter-helpers-on-cardsource--permanent) | 🟡 | pervasive | `card_source.rs`, `permanent.rs` |

## Open gaps

### Global `OnOpponentSecurityRemoved` observer timing
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-008 Elizamon, BT21-017 Dimetromon, BT21-025 Lamiamon, BT24-018 Styracomon, BT24-016 Lamiamon, BT21-001 Gigimon, BT24-008 Elizamon, BT18-087 Owen Dreadnought, BT21-093 Raging Serpentine, BT21-029 Medusamon, EX11-008 Elizamon, P-189 Dimetromon, BT24-012 Dimetromon, BT24-001 Gigimon, BT14-001 Koromon
- **Effect text:** "[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, gain 1 memory." (and many archetype variants: play a trait-matched Digimon free, digivolve with −1 cost, delete low-DP Digimon, play a Petrification token)
- **What's missing:** The engine's existing `OnLoseSecurity` fires only on the revealed card itself via `TriggerSource::SecurityRevealed`. There is **no global fan-out** that enqueues the observer against every other permanent / inherited-stack effect on either side. `EffectTiming::OnSecurityCheck` exists but is unfired (see parity §2.5b). This timing is the **archetype's core engine** — 15+ cards pivot on it.
- **Suggested API shape:** Fire `EffectTiming::OnSecurityCheck` (or introduce `EffectTiming::OnOpponentSecurityRemoved`) from `combat::resolve_security_card` after per-card `OnLoseSecurity`, dispatching to all battle-area + inherited-stack effects with a context snapshot `{attacker, defender, revealed_card}`. Must also fire for non-attack security removal (effect-driven security trashing).
- **Workaround:** None — BLOCKED. Without it, 15+ cards' main recurring payoff never fires.
- **Related:** RUST_PYTHON_PARITY.md §2.5b (OnSecurityCheck not fired), §2.5g, §2.5m

### Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX8-074 MedievalGallantmon ("When Digimon are played"), BT21-029 Medusamon ("When any of your opponent's Digimon are deleted"), BT21-026 WarGreymon ("When any of your opponent's Digimon are deleted")
- **Effect text:** "[All Turns] [Once Per Turn] When Digimon are played, you may activate…" / "When any of your opponent's Digimon are deleted, this Digimon may unsuspend."
- **What's missing:** `EffectTiming::OnEnterFieldAnyone` is declared but no dispatch site in `play_from_hand` / digivolve paths. `OnDeletion` fires only on the deleted permanent; no cross-zone fan-out for deletion events.
- **Suggested API shape:** Enqueue `OnEnterFieldAnyone` from every play / digivolve entry site with `{player, card_id, kind}` trigger context. Add `EffectTiming::OnAnyDeletion` (or promote `OnDeletion` fan-out via `TriggerSource::GlobalDeletion`).
- **Workaround:** None — BLOCKED.
- **Related:** None (both enum variants declared, neither fired).

### Phase-granular turn timings (`StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle`)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-081 Owen Dreadnought (`StartOfYourMainPhase`), BT24-016 Lamiamon (`WhenAttacking`), LM-021 Agumon – Bond of Bravery (`WhenAttacking`), BT23-014 Gallantmon (`WhenAttacking`), BT17-018 Gallantmon: Crimson Mode (`WhenAttacking`), BT21-029 Medusamon (`EndOfAttack`), EX11-012 Medusamon (`EndOfAttack`), BT21-015 Cyclonemon (`EndOfBattle` sub-timing within security resolution)
- **Effect text:** Various — "[Start of Your Main Phase] …", "[When Attacking] …", "[End of Attack] …", "[Security] At the end of the battle …"
- **What's missing:** `EffectTiming::WhenAttacking` and `EffectTiming::EndOfAttack` are in the enum but `Effect::when_attacking(card)` / `Effect::end_of_attack(card)` builder constructors don't exist, and combat doesn't enqueue either. `StartOfYourMainPhase` is entirely absent — existing `StartOfYourTurn` fires before Draw, not at Main-phase entry. `EndOfBattle` sub-timing for security effects is also absent (cards that say "[Security] At the end of the battle, …" need to fire after the Digimon-vs-security resolution, not on reveal).
- **Suggested API shape:** Add `EffectTiming::StartOfYourMainPhase` + fire from `enter_main_phase`. Add builder constructors and fire sites for `WhenAttacking` (in `combat::begin_attack` pre-block) and `EndOfAttack` (in `combat::cleanup_attack` before clearing `is_attacking`). For security sub-timing, either add `.security_timing(SecurityTiming::EndOfBattle)` on `Effect::security` or extend `OnEndBattle` firing to include security-card effects while `pending_security` is still live.
- **Workaround:** Collapse into nearest existing timing — violates no-approximations policy (order-sensitive with Block / Alliance / OnLoseSecurity).
- **Related:** RUST_ENGINE_API.md §9 ("OnEndBattle / OnEndAttack timings are not yet fired").

### Observer timings tied to specific events (`OnDigivolve` trait-filter, `OnSuspend`, `OnAttackTargetChange`, `[When Moving]`)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-082 Owen Dreadnought (`OnDigivolve` trait-filtered, with DP + extra-attack riders), BT24-089 Unique Emblem: Blazing Conductor (`OnSuspend` of named card), BT21-025 Lamiamon (`OnAttackTargetChange`), P-137 Flamedramon (`OnAttackTargetSwitched`), EX11-008 Elizamon (`[When Moving]` breeding→battle), BT16-082 Ukkomon (`[When Moving]` observer)
- **Effect text:** Various — "When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, …" / "When any of your [Owen Dreadnought]s suspend, …" / "When any of your … trait Digimon's attack targets change, …" / "[When Moving] [On Play] …"
- **What's missing:** `OnDigivolve` and `OnSuspend` enum variants exist but no trigger sources fire them. `OnAttackTargetChange` enum variant doesn't exist at all — no `combat.rs` emission from Block / Raid / Alliance redirect paths. `[When Moving]` has no variant (`OnEnterField` exists but is not observably fired from `Game::move_from_breeding` and doesn't broadcast to global observers).
- **Suggested API shape:** Wire `OnDigivolve` from `digivolve_from_hand` with `{digivolver, target}` context. Fire `OnSuspend` from `Permanent::set_suspended(true)`. Add `EffectTiming::OnAttackTargetChange` + emit from block-accept / raid-redirect / collision-redirect paths. Add `EffectTiming::WhenMoving` + fire from `Game::move_from_breeding` alongside a broadcast `OnEnterFieldAnyone`.
- **Workaround:** None — BLOCKED. Approximating with `OnEnterField` or periodic condition checks misses the causal link to the originating event.
- **Related:** None.

### `WhenWouldBeDeleted` / leave-field replacement-effect framework
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-018 Styracomon (Armor Purge + prevent-leave), EX11-012 Medusamon (delete token to cancel leave), BT24-012 Dimetromon (return self to hand to cancel leave), P-137 Flamedramon (Armor Purge), BT20-016 Paildramon (would-be-deleted → DNA digivolve)
- **Effect text:** "When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion." / "When this Digimon would leave the battle area, by deleting 1 Token, it doesn't leave." / "When any of your other Digimon with the [Reptile] or [Dragonkin] trait would leave the battle area by your opponent's effects, by returning this Digimon to the hand, they don't leave." / "When any of your [Paildramon] or [Dinobeemon] would be deleted, 2 of your Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in the hand."
- **What's missing:** Rust's zone-transition paths (`delete_permanent`, return-to-hand, trash, bounce) complete unconditionally. There is no pre-resolution replacement hook that can (a) install a "may pay cost" prompt, (b) cancel the original mutation on acceptance, and (c) attribute the original cause to an opponent effect vs. combat vs. self. `OnDeletion` / `OnLeaveField` (where wired) are observers, not replacements.
- **Suggested API shape:** `EffectTiming::WhenWouldBeDeleted` / `EffectTiming::WouldLeaveField` that fires before resolution, receives a mutable `ReplacementContext { cancel: bool, source_player: PlayerId }`, and gates on the resolver checking `cancel`. Authors pay cost inside the closure and call `ctx.cancel_leave()`. Must carry source-attribution so "by your opponent's effects" filters work.
- **Workaround:** None — BLOCKED. Observer-style `OnLeaveField` cannot undo the transition.
- **Related:** None.

### Selection: multi-select with aggregate-sum constraint
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT17-018 Gallantmon: Crimson Mode, LM-021 Agumon – Bond of Bravery
- **Effect text:** "Choose any number of your opponent's Digimon whose total DP adds up to 15000 or less and delete them." / "Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP."
- **What's missing:** All `select_*` helpers pick exactly one. No primitive for "pick a subset with running aggregate ≤ N" with a PASS-to-finish terminator.
- **Suggested API shape:** `ctx.select_multiple_opponent_permanents(prompt, is_optional, filter_each, running_predicate: Fn(&Game, &[PermanentHandle], PermanentHandle) -> bool, callback: Fn(&mut Ctx, Vec<PermanentHandle>))`. Install a new `SelectionKind::MultiField` emitting PASS as terminator; accumulate picks until PASS or no valid remaining.
- **Workaround:** None — BLOCKED. Simplifying to "single highest-DP ≤ threshold" violates no-approximations.
- **Related:** Parity §4.6d-residual (selection-kind coverage).

### Selection: ordered permutation (place N cards in any order)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** P-035 Red Memory Boost!, P-151 Digimon Liberator, BT21-008 Elizamon, BT24-018 Styracomon, P-103 Offense Training, P-206 Digital Gate Open, EX7-074 Vortex Resonance, BT16-082 Ukkomon
- **Effect text:** "Return the rest to the bottom of the deck in any order." / "Place the remaining cards at the bottom of your deck in any order."
- **What's missing:** No `select_order(items, callback)` primitive. Cards commonly need to permute up to ~4 revealed cards for deck-bottom placement or digivolution-stack ordering.
- **Suggested API shape:** `ctx.select_ordering(prompt, candidate_count, callback: Fn(Vec<usize>))` — modeled either as a chain of single-select prompts with a running exclusion set, or as an action-space encoding of a permutation over ≤8 items.
- **Workaround:** Chained `select_reveal` with exclusion state in captured `Arc<Mutex<Vec<usize>>>` — functional but ergonomically expensive. Fidelity-preserving.
- **Related:** None.

### Selection: opponent-as-selecting-player, cross-side target, union-zone (hand OR trash), DNA-pair
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-024 Cyberdramon (opponent picks from own hand), BT24-016 Lamiamon (opponent places as security), BT21-029 Medusamon (opponent plays a token), BT20-102 Omnimon (X Antibody) (cross-side target), BT21-013 Agunimon (hand OR trash), P-189 Dimetromon (hand OR trash), EX7-074 Vortex Resonance (hand OR trash), P-206 Digital Gate Open (hand OR trash in [Security]), EX9-013 BlitzGreymon (DNA-pair), BT20-016 Paildramon (DNA-pair)
- **Effect text:** Various — "Your opponent places 1 card from their hand as the bottom security card" / "Choose 1 of both players' Digimon" / "You may play 1 card … from your hand or trash without paying the cost" / "2 of your Digimon may DNA digivolve into [X] in the hand"
- **What's missing:** All `select_*` helpers install `selecting_player = self.player` and scope to a single zone / single side. Four distinct selection-kind gaps:
  1. Opponent-as-selecting-player variants (`select_hand_of(player, …)`).
  2. `select_any_permanent` that walks both players' battle areas.
  3. `select_hand_or_trash` unified prompt (action-space has room — PLAY_HAND 0-29 and TRASH_EFFECT 1150-1194 are disjoint).
  4. `select_dna_pair` that validates a pair of own Digimon against a hand card's DNA costs.
- **Suggested API shape:** `ctx.select_hand_of(player, prompt, filter, callback)`; `ctx.select_any_permanent(prompt, filter, callback)`; `ctx.select_hand_or_trash(player, prompt, filter_hand, filter_trash, callback)`; `ctx.select_dna_pair(hand_index, callback)`.
- **Workaround:** Two-step `select_effect_choice` decomposition gives the player two prompts where the card describes one — degrades RL action-tree shape.
- **Related:** Parity §4.6d-residual.

### Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-017 Dimetromon, BT21-025 Lamiamon, BT24-016 Lamiamon, BT24-082 Owen Dreadnought, BT24-089 Unique Emblem, LM-027 Red Scramble, P-189 Dimetromon, P-151 Digimon Liberator, EX7-074 Vortex Resonance, P-206 Digital Gate Open, BT9-112 DeathXmon (dynamic cost)
- **Effect text:** "you may play 1 [X] from your hand without paying the cost" / "play 1 [X] from your trash without paying the cost" / "play 1 Tamer card … with the play cost reduced by 4"
- **What's missing:** `ctx.play_from_security()` exists; no analogous `play_from_hand_free(hand_index)` / `play_from_trash_free(trash_index)` / `play_from_hand_with_cost_delta(hand_index, delta)`. The cost-override variant is load-bearing for P-206's Delay sub-effect.
- **Suggested API shape:** `ctx.play_from_hand_free(player, hand_index) -> Option<PermanentHandle>`; `ctx.play_from_trash_free(player, trash_index) -> Option<PermanentHandle>`; `ctx.play_from_hand_with_cost_delta(player, hand_index, delta: i16)`. Each must fire `OnPlay` through the standard queue.
- **Workaround:** None — BLOCKED. Raw `player.hand.remove(i)` + `battle_area.push(Permanent::new(…))` skips OnPlay observers.
- **Related:** RUST_PYTHON_PARITY §1.1 (play cost deduction — this gap is upstream of the free-play variant but distinct), §2.5a (play_from_security landed).

### Zone-manipulation: effect-initiated digivolve (free / reduced / with trait filter)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-001 Gigimon, BT21-093 Raging Serpentine, LM-027 Red Scramble, BT24-089 Unique Emblem, EX7-074 Vortex Resonance, BT23-005 Elizamon (passive trait-gated reduction), BT21-013 Agunimon, P-103 Offense Training
- **Effect text:** "1 of your Digimon may digivolve into a [X] trait Digimon card in the hand with the digivolution cost reduced by N" (and "without paying the cost" variants)
- **What's missing:** `Game::digivolve_from_hand` exists as an action entry but is not surfaced through `EffectContext`, and there is no way to apply a one-shot cost reduction or full-free flag to an effect-driven digivolve. `ModifierType::ChangeDigivolveCost` is permanent-keyed, not event-keyed. Passive "reduce by N when digivolving into trait-matched X" (BT23-005) needs a `BeforePayCost`-style hook during `digivolve_from_hand` that the current cost path doesn't consult.
- **Suggested API shape:** `ctx.prompt_digivolve(base_filter, target_filter, reduction: u8, is_optional, callback)` installs a chained own-permanent + hand-card selection and performs the digivolve at reduced/free cost. Extend `Game::digivolve_from_hand` to scan `ChangeDigivolveCost` modifiers with trait-filter predicates.
- **Workaround:** None faithful. The whole archetype's recurring digivolve-from-hand payoff (7+ cards) routes through this primitive.
- **Related:** RUST_ENGINE_API §9 ("BeforePayCost for cost reduction … not implemented").

### Zone-manipulation: return-to-hand / return-to-deck (top/bottom) / bounce self
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-012 Dimetromon (return self to hand), BT24-082 Owen Dreadnought (return Tamer to deck bottom), BT20-102 Omnimon (X Antibody) (return opp permanent to deck bottom), EX11-012 Medusamon (return card from opp trash to deck bottom), P-151 Digimon Liberator / BT16-082 Ukkomon / P-206 Digital Gate Open (return revealed rest to deck bottom), BT24-017 Medusamon (opponent returns 2 cards from trash to deck bottom as cost)
- **Effect text:** Various — "return this Digimon to the hand" / "By returning this Tamer to the bottom of the deck" / "return 1 of your opponent's Digimon to the bottom of the deck" / "Return the rest to the bottom of the deck"
- **What's missing:** No helpers for: `return_permanent_to_hand(handle)`, `return_permanent_to_deck(handle, DeckEnd)`, `return_trash_to_deck(player, trash_index, DeckEnd)`, `return_revealed_to_deck(index, DeckEnd)`. `delete_permanent` trashes everything; there's no "extract top card, send materials to trash, move top to X" primitive.
- **Suggested API shape:** `ctx.return_permanent_to_hand(handle)`; `ctx.return_permanent_to_deck(handle, end: DeckEnd)`; `ctx.return_trash_to_deck(player, trash_index, end)`; `ctx.return_revealed_to_deck(reveal_index, end)`. Each must correctly route digivolution materials per the rules (top card → destination, others → trash) and fire appropriate triggers.
- **Workaround:** None — BLOCKED.
- **Related:** None.

### Zone-manipulation: reveal-top-N deck + add-to-hand + hatch
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-008 Elizamon, P-103 Offense Training, P-035 Red Memory Boost!, EX7-074 Vortex Resonance, BT16-082 Ukkomon, P-151 Digimon Liberator, P-206 Digital Gate Open, BT21-007 Agumon (return trash to hand — shares `add_to_hand`)
- **Effect text:** "Reveal the top N cards of your deck. Add 1 [card-kind/trait] card among them to the hand. Return the rest to the bottom of the deck. [Then, you may hatch in your breeding area.]"
- **What's missing:** `Game.revealed_cards` exists (§3.4 tensor scaffold) and `select_reveal` helper exists (§4.6d), but there is no `ctx.reveal_top(player, n)` that populates `revealed_cards`. No `ctx.add_to_hand(player, card)` (required by many search/recursion effects). No `ctx.hatch(player)` — `Game::hatch` is action-decoder-only.
- **Suggested API shape:** `ctx.reveal_top(player, n) -> &[CardSource]`; `ctx.move_revealed_to_hand(reveal_index)`; `ctx.move_revealed_to_deck_bottom_ordered(order)` (couples with ordered-selection gap); `ctx.add_to_hand(player, CardSource)`; `ctx.hatch(player)`.
- **Workaround:** Direct `ctx.game.player_mut(...)` mutation works but violates curated-API contract and bypasses `OnAddToHand` + hand-size-limit checks.
- **Related:** Parity §3.4 (revealed_cards scaffold landed).

### Zone-manipulation: security stack operations (trash top, place bottom, trash N)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-025 Lamiamon, BT24-016 Lamiamon, LM-021 Agumon – Bond of Bravery, BT17-018 Gallantmon: Crimson Mode ("trash N from top"), BT24-016 Lamiamon / BT21-024 Cyberdramon (place as bottom security), P-137 Flamedramon (move top security to hand)
- **Effect text:** "trash your opponent's top security card" / "trash 1 card from the top of your opponent's security stack" / "places 1 card from their hand as the bottom security card" / "your opponent adds the top card of their security stack to the hand"
- **What's missing:** `EffectContext` exposes `trash_from_top(player, N)` for **decks** only. No helpers for: `trash_top_security(player, N)` (must fire `OnLoseSecurity` per card popped), `place_security_bottom(player, card)`, `place_security_top(player, card)`, `move_top_security_to_hand(player)`.
- **Suggested API shape:** `ctx.trash_top_security(of_player: PlayerId, count: u8) -> u8`; `ctx.place_security_bottom(player, CardSource)`; `ctx.place_security_top(player, CardSource)`; `ctx.move_top_security_to_hand(player)`. All must fire `OnLoseSecurity` / `OnAddToHand` and update `face_up_security` bookkeeping.
- **Workaround:** Raw `player.security` manipulation skips observer triggers — correctness class same as the global-security-observer gap.
- **Related:** Parity §2.5k (face_up_security stale entries), §2.5m (security_reveal event).

### Token creation + `CardKind::Token` + Petrification Token definition
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-017 Medusamon, BT21-029 Medusamon, EX11-012 Medusamon
- **Effect text:** "they play 1 [Petrification] Token. (Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend. [On Deletion] Trash your top security card.)"
- **What's missing:** No `CardKind::Token` variant (parity §4.6b-residual). No `ctx.play_token(player, token_id)` helper. No Petrification Token data or registered `CardEffect`. Token baked-in abilities (CannotSuspend gated on [Your Turn], OnDeletion trash-top-security) also need their own primitives, though CannotSuspend's modifier exists.
- **Suggested API shape:** Introduce `CardKind::Token` + `TokenRegistry`. `ctx.play_token(controller, token_id) -> Option<PermanentHandle>` creates a synthetic `CardSource`, places a `Permanent`, fires `OnPlay`. Ship Petrification Token data + `CardEffect` (CannotSuspend [Your Turn] + OnDeletion → `trash_top_security`).
- **Workaround:** None — BLOCKED.
- **Related:** Parity §4.6b-residual.

### Place card at a specific stack position (bottom-source / under another permanent) + alt-digivolve
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-013 Agunimon, BT24-016 Lamiamon (alt-digivolve variant)
- **Effect text:** "place 1 [Hybrid] or [Hero] trait Digimon card from your hand or trash as this Digimon's bottom digivolution card or under any of your red Tamers with inherited effects." / "by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."
- **What's missing:** No primitive to append a `CardSource` to the **bottom** of a permanent's `card_sources` (current digivolve always pushes top). No helper to target "another own permanent" as attachment point. No alt-digivolve primitive with override cost + ignore-digivolution-requirements flag.
- **Suggested API shape:** `ctx.place_as_bottom_source(target, card)`; `ctx.place_as_top_source(target, card)`; `ctx.digivolve_into_source_from_hand(target, hand_index, bottom_trash_index, cost_override: u16, ignore_reqs: bool)`.
- **Workaround:** None — BLOCKED. Raw `battle_area[i].card_sources.insert(0, ...)` skips OnEnterField / inherited-stack recomputation.
- **Related:** None.

### Native printed keyword parsing (Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-081, BT24-018, BT24-011, BT24-017, BT21-025, EX11-012, P-189, BT21-072, BT21-029, EX10-010, BT21-026, BT20-102, EX8-074, EX9-013, BT17-018, P-137, EX9-008 (inherited Raid), ST1-07 (inherited Sec A+1) — 17+ cards in this archetype alone
- **Effect text:** `<Rush>`, `<Raid>`, `<Piercing>`, `<Blocker>`, `<Reboot>`, `<Jamming>`, `<Blitz>`, `<Vortex>`, `<Alliance>`, `<Security A. +N>` printed on the card face
- **What's missing:** `CardData` has no `keywords: Vec<Keyword>` field; printed keywords live inside `effect_text: String`. Combat / mask / security modules consult only modifier-granted keywords via `ModifierRegistry::has_keyword`. Parity catalogs sub-cases for Rush (§2.1b), Blitz (§4.3b), Jamming (§2.5f) — this gap unifies them: an architectural cross-cutting fix covering all statically-printed keywords.
- **Suggested API shape:** Add `keywords: Vec<Keyword>` to `CardData` + ingest parse pass from `effect_text`, **or** auto-emit `Effect::declarative(card).grants_keyword(kw)` at `CardEffectRegistry` build time. Combat / mask helpers then OR modifier-granted with native. Native parsing must capture parametric variants (`SecurityAttackPlus(N)`, `DeDigivolve(N)`).
- **Workaround:** Per-card `Effect::on_play(card).process(|ctx| ctx.grant_keyword(self, Keyword::X, Expiry::Permanent))` — medium fidelity, but brittle for cards placed via Blast Digivolve / Training / material-reveal and doesn't populate face-keyword tensor slots.
- **Related:** RUST_PYTHON_PARITY §2.1b, §4.3b, §2.5f.

### `<Progress>` keyword + `ImmunityToOpponentEffects` modifier
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-025 Lamiamon, BT24-018 Styracomon, BT24-017 Medusamon, BT21-029 Medusamon, EX11-012 Medusamon, P-189 Dimetromon
- **Effect text:** "`<Progress>` (While attacking, your opponent's effects don't affect this Digimon.)"
- **What's missing:** No `Keyword::Progress` variant. No `ModifierType::ImmunityToOpponentEffects`. Every opponent-targeting effect-resolution site would need to consult the gate (select_opponent_permanent filter, delete_permanent when called from opp effect, security check pipeline).
- **Suggested API shape:** Add `Keyword::Progress` + `ModifierType::ImmunityToOpponentEffects`; add `Game::is_immune_to_opponent_effects(handle, source_player)` consulted by every opponent-directed mutation. Already partially scoped by parity §2.5c (security branch) — extend across all effect sites.
- **Workaround:** None — the keyword is load-bearing for every attacker in the archetype.
- **Related:** RUST_PYTHON_PARITY §2.5c.

### `<Armor Purge>` keyword (leave-field replacement variant)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-018 Styracomon, P-137 Flamedramon
- **Effect text:** "`<Armor Purge>` (When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion.)"
- **What's missing:** A specific instance of the leave-field replacement framework gap, plus a dedicated `Keyword::ArmorPurge` and a "trash top source of self" primitive.
- **Suggested API shape:** Built atop the `WhenWouldBeDeleted` replacement framework (separate gap). `Keyword::ArmorPurge` + `ctx.trash_top_source_of_self()` primitive; auto-emit the replacement effect from native-keyword parsing.
- **Workaround:** None — BLOCKED without the replacement-effect framework.
- **Related:** See "WhenWouldBeDeleted / leave-field replacement-effect framework" above.

### `<Training>` keyword
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX9-008 Biyomon
- **Effect text:** "`<Training>` (In the main phase, by suspending this Digimon, place your deck's top card face down as this Digimon's bottom digivolution card. This effect can also activate in the breeding area.)"
- **What's missing:** (a) No `Keyword::Training` variant. (b) No primitive to move top-of-deck onto a permanent's `card_sources` at bottom position. (c) No `face_down: bool` flag on `CardSource` (with hidden-info tensor implications). (d) `[Main]` activation mask doesn't extend to breeding-area permanents.
- **Suggested API shape:** `Keyword::Training` + `ctx.push_deck_top_under_self(face_down: bool)` + `CardSource::face_down` field (zero-out data_index in observation tensors) + extend `MainOnField` activation to breeding-area when effect keyword is Training.
- **Workaround:** None — BLOCKED.
- **Related:** None.

### `<Delay>` keyword + placement-turn gating for Option cards
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** P-103 Offense Training, LM-027 Red Scramble, BT24-089 Unique Emblem, P-035 Red Memory Boost!, BT21-093 Raging Serpentine, P-206 Digital Gate Open
- **Effect text:** "`<Delay>` (By trashing this card after the placing turn, activate the effect below.)"
- **What's missing:** Delay Options (a) stay on the battle area after initial resolution (tied to Option play-flow gap), (b) become activatable on turns after placement (`turn_played` tracking exists but no activation mask path), (c) activate by trashing self via an `[Main]` or reactive observer prompt. Multiple Delay variants have conditional activation triggers (BT24-089: OnSuspend of named card; LM-027: StartOfYourTurn + opponent-has-Digimon; BT21-093: OnOpponentSecurityRemoved).
- **Suggested API shape:** `Keyword::Delay` on Option-card permanents. `EffectTiming::DelayMain` (gated on `turn_count > turn_played`). Mask emits at a `FIELD_EFFECT_DELAY` slot when activation is legal. Activation trashes self from battle area and runs the post-Delay body.
- **Workaround:** None — BLOCKED. Intertwined with Option-card play flow.
- **Related:** See Option card play flow gap.

### Raid target-switch interrupt (scripting-surface, not mask-only)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-017 Medusamon, BT24-011 Cyclonemon, EX11-012 Medusamon, P-137 Flamedramon, BT21-025 Lamiamon (target-switch observer), plus every other Raid card
- **Effect text:** "`<Raid>` (When this Digimon attacks, you may switch the target of attack to 1 of your opponent's unsuspended Digimon with the highest DP.)"
- **What's missing:** Raid is currently mask-only (§4.4) — gates legal target bits at Main-phase selection. The card text is an **optional mid-attack switch** after declaration. No `RaidOpen` state in the attack state machine; no `OnAttackTargetChange` event fires even when redirection occurs through Block.
- **Suggested API shape:** Add `RaidOpen` state to `PendingAttack` between `Declared` and `AllianceOpen`. `combat::try_enter_raid` installs a may-switch selection of highest-DP unsuspended opponent Digimon; attacker PASS keeps declared target. Fire `EffectTiming::OnAttackTargetChange` after any switch (Block / Raid / Collision).
- **Workaround:** Mask-time Raid (§4.4) covers "pick the Raid target up front" but fails the text when attacking into security (no mid-attack redirect).
- **Related:** Parity §4.4 (Raid mask), §2.3 (combat interrupts).

### De-Digivolve N primitive (single + mass)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX9-013 BlitzGreymon ("De-Digivolve 3" single target), BT9-112 DeathXmon ("De-Digivolve 1" all opponent Digimon)
- **Effect text:** "`<De-Digivolve N>` 1 of your opponent's Digimon. (Trash up to N cards from the top. You can't trash past level 3 cards.)"
- **What's missing:** `Keyword::DeDigivolve(u8)` exists in enums.rs, but no implementation that pops top N `card_sources` from a target permanent, stopping at the first Lv.3-or-lower revealed, moving popped sources to trash. No mass variant.
- **Suggested API shape:** `ctx.de_digivolve(target: PermanentHandle, amount: u8) -> u8` — pops while `popped < amount && next_top.level > 3`, moves each popped source to owner's trash, fires `OnTrash` / `OnLoseField` as appropriate. `ctx.de_digivolve_all_opponent(amount)` sugar for the mass case.
- **Workaround:** None — BLOCKED. Level-3 floor rule and trash routing need centralized handling.
- **Related:** None.

### Ace Overflow: inherited memory penalty on zone-change from field / under-card
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX10-010 BlackWarGreymon, EX9-013 BlitzGreymon, BT17-018 Gallantmon: Crimson Mode, LM-021 Agumon – Bond of Bravery
- **Effect text:** "Ace Overflow `<-N>` (As this card moves from the field or under a card to an area other than those, lose N memory.)"
- **What's missing:** No Ace-card identification, no Overflow metadata, no zone-transition firing of a penalty effect. `EffectTiming::OnLeaveField` is declared but I couldn't locate a dispatch site. "Under a card" (digivolution stack) zone distinction needs modeling separately from `BattleArea`.
- **Suggested API shape:** `CardData::ace_overflow: Option<i8>` + firing of `OnLeaveField` with `LeaveFieldContext { destination: Zone }` from every zone-change path (permanent trash, return-to-hand/deck/security, digivolution-source → out-of-stack). `Effect::ace_overflow(n)` builder sugar.
- **Workaround:** None — BLOCKED.
- **Related:** None.

### Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT8-097 Crimson Blaze, BT9-112 DeathXmon, BT21-026 WarGreymon, EX8-074 MedievalGallantmon (selection-gated)
- **Effect text:** "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play." / "reduce its memory cost by 3 for each Digimon and Tamer your opponent has in play" / "reduce the play cost by 2 for each of your opponent's Digimon" / "by suspending 2 Digimon, reduce the play cost by 4"
- **What's missing:** `.cost_reduction(n)` accepts only a static `i32`. `BeforePayCost` dispatch is not wired into `calculate_play_cost` (API §9). No closure-valued reduction and no selection-at-cost-time (for the "suspend 2 Digimon" variant).
- **Suggested API shape:** `.cost_reduction_fn(|&EffectReadContext| i16)` closure-valued variant, evaluated inside `calculate_play_cost`. For selection-gated variants: `Effect::before_pay_cost(card).with_optional_payment(cost_delta, select_filter, execute)` — offers a prompt at cost-time, completes before resolving payment.
- **Workaround:** None — BLOCKED for BT8-097 (cost-reduction scanning isn't wired at all per §9).
- **Related:** RUST_ENGINE_API §9, Parity §4.7e (DigiXros cost reduction).

### Dynamic DP scaling modifier (per-stack-depth / per-opponent-board)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-072 Arresterdramon: Superior Mode (per digivolution cards), BT24-017 Medusamon (per opponent Digimon)
- **Effect text:** "This Digimon gets +1000 DP for each of its digivolution cards." / "this Digimon gets +2000 DP for each of your opponent's Digimon until their turn ends."
- **What's missing:** `EffectBuilder::dp_modifier(n)` is static. Per §13, modifier-registry DP grants are NOT summed into `source_dp_contribution` tensor slots — so `add_dp_modifier` also can't express tensor-correct dynamic DP.
- **Suggested API shape:** `.dp_modifier_fn(|&EffectReadContext| i16)` closure-valued variant evaluated at tensor-build time. Or `ModifierType::ChangeDpDynamic(Box<dyn Fn(...)>)` with tensor-aware summation.
- **Workaround:** Static snapshot at cast time for the opponent-scaling variant — fails faithfulness when opponent board changes. Per-stack-depth has no snapshot equivalent.
- **Related:** RUST_ENGINE_API §13.

### Condition-gated modifier entries
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX10-010 BlackWarGreymon
- **Effect text:** "While your opponent has a Digimon with 13000 DP or more, your opponent's Digimon's effects don't affect this Digimon, and it gets +3000 DP."
- **What's missing:** `ModifierEntry` has no condition closure (parity §4.7x). Can't express "active only while opp has ≥13k DP Digimon" without an observer for arbitrary DP-threshold transitions.
- **Suggested API shape:** Add `condition: Option<Box<dyn Fn(&EffectReadContext) -> bool>>` to `ModifierEntry`; or passive `Effect::declarative(card).modifier_when(type, value, condition)` builder that the affect-resolution code consults per query.
- **Workaround:** Permanent grant over-applies when condition is false.
- **Related:** Parity §4.7x.

### Player-scoped modifier registry (cannot-play-from-trash by effect, opponent-cannot-reduce-cost, ignore-color-requirement)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT23-014 Gallantmon (CannotPlayFromTrash by effect), BT8-097 Crimson Blaze (CannotPlayDigimonByEffect), BT5-008 Gaossmon (opp cannot reduce digivolution costs), P-151 Digimon Liberator / EX7-074 Vortex Resonance / P-206 Digital Gate Open / ST22-08 Offensive Plug-In V (IgnoreColorRequirement aura)
- **Effect text:** "Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash." / "Your opponent can't play Digimon by effects until the end of their turn." / "[Opponent's Turn] Your opponent can't reduce digivolution costs." / "While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's color requirements."
- **What's missing:** `ModifierRegistry` is keyed by `PermanentHandle` only — no player-scoped store. Missing variants: `CannotPlayFromTrash`, `CannotPlayDigimonByEffect`, `OpponentCannotReduceDigivolveCost`, `IgnoreColorRequirement`. Effect-vs-action-initiated play distinction isn't modeled either.
- **Suggested API shape:** Extend `ModifierRegistry` with `player_modifiers: HashMap<PlayerId, Vec<ModifierEntry>>` + `add_player_modifier / has_player_modifier / expire_*` with shared `Expiry` handling. Add the missing `ModifierType` variants. Consult `has_player_modifier` from every effect-play helper and the color-check mask.
- **Workaround:** None — BLOCKED.
- **Related:** Parity §4.2b (IgnoreColorRequirement), §4.7x (context-aware modifier queries).

### Option card play flow (resolve + trash vs. place-on-field; [Main]/[Security] activation) + Plug-In / Link mechanic
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** P-103, LM-027, BT24-089, BT8-097, P-035, BT21-093, EX7-074, P-151, P-206, BT1-090 (Option cards); ST22-08 Offensive Plug-In V (Plug-In / Link mechanic)
- **Effect text:** All [Main] top-line clauses of Option cards; all "[Main] You may link this card to 1 of your Digimon without paying the cost" of Plug-In cards.
- **What's missing:** Per RUST_ENGINE_API §9, "Option cards have no play flow yet (they hit the field as a permanent like Digimon)." Need: (a) play path that fires `OptionMain` then trashes the card (or places it on field when `<Delay>`); (b) security effects that re-activate the card's [Main]; (c) `ctx.place_self_in_battle_area()` / `ctx.trash_self_from_option()` / `ctx.activate_own_main_effects()` helpers; (d) Plug-In / Link mechanic: `Permanent.linked_cards: Vec<CardSource>` storage exists but no `ctx.link_card_to_permanent(card, target)` API, no play-flow for Plug-In card kind, no link-cost evaluation, no interaction between linked Plug-Ins and their carrier.
- **Suggested API shape:** `Game::play_option_from_hand` branched inside `play_from_hand` based on `CardKind::Option`. `EffectTiming::OptionMain` and `OptionSecurity` (variants exist; need dispatch). `ctx.place_self_in_battle_area()` + `ctx.activate_own_main_effects()`. For Plug-In: `ctx.link_from_hand_to_own_permanent(filter, callback)` + `EffectTiming::OnLink` / `WhileLinked` + `ModifierType::LinkRequirement` metadata on `CardData`.
- **Workaround:** None — BLOCKED. Option-card play flow is a foundational architectural gap; Plug-In is arguably its own sub-spec.
- **Related:** RUST_ENGINE_API §9.

### Scheduled end-of-turn effect queue (for transient Options)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT1-090 Gravity Crush
- **Effect text:** "[Main] Gain 2 memory. At end of turn, lose 2 memory."
- **What's missing:** Transient Option cards resolve + trash before `end_turn`; existing `EndOfYourTurn` timing only walks live permanents. No mechanism to enqueue a closure from an Option's `[Main]` that fires at turn end after the card is in trash.
- **Suggested API shape:** `ctx.schedule_end_of_turn(|ctx| { … })` enqueues a boxed closure onto `Game.scheduled_eot: Vec<…>`. `Game::end_turn` drains after standard `EndOfYourTurn` triggers, before memory reset.
- **Workaround:** None — BLOCKED.
- **Related:** Couples with Option-card play-flow gap.

### Effect re-firing / cross-timing self-trigger
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX8-074 MedievalGallantmon
- **Effect text:** "[All Turns] [Once Per Turn] When Digimon are played, you may activate 1 of this Digimon's [When Digivolving] effects."
- **What's missing:** No API to invoke another of the source card's effects from within a `process` closure. `EffectContext` has mutation helpers but no `ctx.fire_effect_of_self(timing: EffectTiming)`.
- **Suggested API shape:** `ctx.fire_effect_of_self(timing)` — looks up source card's effects via `CardEffectRegistry::get(card_id)`, filters by timing + matching condition, enqueues via `effect_queue`.
- **Workaround:** Duplicating the [When Digivolving] body inline is brittle if the primary effect changes.
- **Related:** None.

### Force-follow-up-attack / "may attack without suspending" script helpers
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-081 Owen Dreadnought ("Then, that Digimon attacks"), BT24-082 Owen Dreadnought ("it may attack"), BT20-016 Paildramon ("this Digimon may attack"), BT20-102 Omnimon (X Antibody) ("attack without suspending"), BT21-072 Arresterdramon: Superior Mode ("This Digimon may attack without suspending"), EX9-013 BlitzGreymon ("1 of your Digimon may attack")
- **Effect text:** Variants of "it may attack" / "attack without suspending" immediately following another effect.
- **What's missing:** Engine internally supports `PendingAttack::is_overclock = true` to skip suspend (§4.6c-residual) but the flag is not exposed to scripts. No `ctx.force_follow_up_attack(attacker)` / `ctx.grant_may_attack_without_suspend(target, expiry)`. `ModifierType::MayAttack` exists but is EndOfTurnAction-scoped; the immediate force-attack case needs a distinct primitive.
- **Suggested API shape:** `ctx.force_follow_up_attack(attacker: PermanentHandle)` installs an EndOfTurnAction-slotted attack scoped to that attacker. `ctx.grant_may_attack_without_suspend(target, expiry)` / new `ModifierType::AttackWithoutSuspend(u8)` consumed by `begin_attack_impl`.
- **Workaround:** None — BLOCKED.
- **Related:** Parity §4.6c / §4.6c-residual.

### Trait-filter helpers on `CardSource` / `Permanent`
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT21-093, EX11-008, P-189, BT21-029, EX11-012 (LIBERATOR-typed), plus many search / filter effects
- **Effect text:** "1 of your [Reptile] or [Dragonkin] trait Digimon …" / "1 card with the [LIBERATOR] trait …"
- **What's missing:** `CardData.type_eng` is present, but no ergonomic `CardSource::has_type(&str)` / `Permanent::has_any_type(&[&str])` accessor. Authors dip into `ctx.card_data()[idx].type_eng.contains(...)` directly — verbose, case-sensitivity bugs likely.
- **Suggested API shape:** `CardSource::has_type(card_data, trait_name)` + `Permanent::top_card_has_type(...)` / `has_any_type(...)`, case-insensitive.
- **Workaround:** Raw card_data scan — functional but API-convention-violating.
- **Related:** Parity §2.1b (same effect-listing / text-parsing class).

## Resolved gaps

_(None yet — this document was created on 2026-04-17.)_
