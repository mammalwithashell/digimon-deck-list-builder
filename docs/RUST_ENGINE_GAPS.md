# Rust Engine Gaps

Capability gaps in the Rust engine's scripting surface (`digimon-engine/`), discovered during archetype audits by `/assess-archetype-rust`. Distinct from [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md), which tracks Rust↔Python divergences in shared subsystems — this document catalogs **net-new primitives** the Rust scripting API needs before a given archetype can be implemented under the no-approximations policy (CLAUDE.md §17–18).

Format and conventions mirror `qa/archetype-qa/engine-gaps.md` (Python-scoped). Gap titles are **capability-centric**, never card-centric.

Each entry lists the cards that surfaced it, but the entry itself describes a reusable engine primitive. If two cards need the same primitive, they share one entry — not two. When a new archetype audit surfaces the same primitive, the existing entry's `Discovered in:` and `Card(s):` lines accumulate — they do not fork into a new entry.

> **Canonical API signatures live here.** Fix-plans in `.claude/plans/rust-engine-gaps-*.md` should reference gap titles rather than restate signatures, to prevent divergence as the engine evolves.

## Severity legend

- **🔴 BLOCKING** — no faithful workaround exists; affected cards cannot be authored without this primitive.
- **🟡 PARTIAL** — a workaround exists with a specific fidelity cost. Sub-kinds marked inline: *ergonomics / sugar* (expressible today but awkward; scripts reach around `EffectContext`); *primitive-with-fidelity-cost* (modifier or keyword exists but its scope is too coarse for the card text's restriction).
- Pure verification / test-coverage items are **not** filed as gaps — see the "Deferred" section at the bottom of this file.

## Audit index

| Archetype | Audited | Cards | 🟢 Supported | 🟡 Partial | 🔴 Blocked |
|---|---|---|---|---|---|
| Medusamon | 2026-04-17 | — | — | — | — |
| DNA Omnimon | 2026-04-17 | 64 | 1 | 4 | 59 |
| TS Olympos | 2026-04-18 | 105 | 1 | 4 | 100 |

## At a glance

Rows link to the detailed entry below. `#cards` is the Medusamon-archetype count — most primitives unblock many more cards archetype-wide (DNA Omnimon audit surfaces ~30 of the 32 entries below as further evidence; see per-entry `Card(s):` lines). `Key files` is the primary surface the fix touches.

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
| [Granted triggered ability — attach an `Effect` to another permanent](#granted-triggered-ability--attach-an-effect-to-another-permanent) | 🔴 | 1 | `modifiers.rs`, `effect_queue.rs`, `effect_context.rs` |
| [Named-target declarative aura (DP / keyword grants filtered by name/trait/level)](#named-target-declarative-aura-dp--keyword-grants-filtered-by-nametraitlevel) | 🔴 | 3+ | `effect.rs`, `modifiers.rs`, `tensor.rs`, `combat.rs` |
| [Declarative aura sourced from security zone](#declarative-aura-sourced-from-security-zone) | 🔴 | 1 | `effect.rs`, `tensor.rs`, `game.rs` |
| [Digivolution-stack name overlay ("has all names of materials")](#digivolution-stack-name-overlay-has-all-names-of-materials) | 🔴 | 1 | `effect.rs`, `card_source.rs`, `permanent.rs` |
| [Decode keyword (play from own digivolution stack without paying cost on non-battle leave)](#decode-keyword-play-from-own-digivolution-stack-without-paying-cost-on-non-battle-leave) | 🔴 | 1 | `enums.rs`, `effect.rs` (builds on replacement framework + SelectSource) |
| [Ergonomics partials](#ergonomics-partials) | 🟡 | pervasive | `effect.rs`, `effect_context.rs` |

## Open gaps

### Global `OnOpponentSecurityRemoved` observer timing
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-008 Elizamon, BT21-017 Dimetromon, BT21-025 Lamiamon, BT24-018 Styracomon, BT24-016 Lamiamon, BT21-001 Gigimon, BT24-008 Elizamon, BT18-087 Owen Dreadnought, BT21-093 Raging Serpentine, BT21-029 Medusamon, EX11-008 Elizamon, P-189 Dimetromon, BT24-012 Dimetromon, BT24-001 Gigimon, BT14-001 Koromon — DNA Omnimon adds: BT22-013 WarGreymon (inherited trash top opp security), BT17-015 WarGreymon (inherited trash top opp security), EX4-073 Omnimon Alter-B (trash top 2 opp security)
- **Effect text:** "[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, gain 1 memory." (and many archetype variants: play a trait-matched Digimon free, digivolve with −1 cost, delete low-DP Digimon, play a Petrification token)
- **What's missing:** The engine's existing `OnLoseSecurity` fires only on the revealed card itself via `TriggerSource::SecurityRevealed`. There is **no global fan-out** that enqueues the observer against every other permanent / inherited-stack effect on either side. `EffectTiming::OnSecurityCheck` exists but is unfired (see parity §2.5b). This timing is the **archetype's core engine** — 15+ cards pivot on it.
- **Suggested API shape:** Fire `EffectTiming::OnSecurityCheck` (or introduce `EffectTiming::OnOpponentSecurityRemoved`) from `combat::resolve_security_card` after per-card `OnLoseSecurity`, dispatching to all battle-area + inherited-stack effects with a context snapshot `{attacker, defender, revealed_card}`. Must also fire for non-attack security removal (effect-driven security trashing).
- **Workaround:** None — BLOCKED. Without it, 15+ cards' main recurring payoff never fires.
- **Related:** RUST_PYTHON_PARITY.md §2.5b (OnSecurityCheck not fired), §2.5g, §2.5m

### Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** EX8-074 MedievalGallantmon ("When Digimon are played"), BT21-029 Medusamon ("When any of your opponent's Digimon are deleted"), BT21-026 WarGreymon ("When any of your opponent's Digimon are deleted") — DNA Omnimon adds: BT22-005 Tsumemon (trait-filtered OnPlayed observer), EX9-066 Tai Kamiya & Matt Ishida (suspend-self cost on any ally played), BT17-081 Tai Kamiya & Matt Ishida (observer on ally played or digivolves), EX9-019 WereGarurumon: Sagittarius Mode / EX9-012 MetalGreymon: Alterous Mode / AD1-001 Greymon / AD1-010 Garurumon (hand-resident observers on ally played or digivolves — expands the fan-out target from "battle area" to "hand"), EX4-061 Matt Ishida & Tai Kamiya
- **Effect text:** "[All Turns] [Once Per Turn] When Digimon are played, you may activate…" / "When any of your opponent's Digimon are deleted, this Digimon may unsuspend."
- **What's missing:** `EffectTiming::OnEnterFieldAnyone` is declared but no dispatch site in `play_from_hand` / digivolve paths. `OnDeletion` fires only on the deleted permanent; no cross-zone fan-out for deletion events.
- **Suggested API shape:** Enqueue `OnEnterFieldAnyone` from every play / digivolve entry site with `{player, card_id, kind}` trigger context. Add `EffectTiming::OnAnyDeletion` (or promote `OnDeletion` fan-out via `TriggerSource::GlobalDeletion`).
- **Workaround:** None — BLOCKED.
- **Related:** None (both enum variants declared, neither fired).

### Phase-granular turn timings (`StartOfYourTurn`, `StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle`)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-081 Owen Dreadnought (`StartOfYourMainPhase`), BT24-016 Lamiamon (`WhenAttacking`), LM-021 Agumon – Bond of Bravery (`WhenAttacking`), BT23-014 Gallantmon (`WhenAttacking`), BT17-018 Gallantmon: Crimson Mode (`WhenAttacking`), BT21-029 Medusamon (`EndOfAttack`), EX11-012 Medusamon (`EndOfAttack`), BT21-015 Cyclonemon (`EndOfBattle` sub-timing within security resolution) — DNA Omnimon adds: BT17-019 Gabumon (`StartOfYourMainPhase`), BT22-084 Nokia Shiramine (`StartOfYourMainPhase`), BT22-089 Mirei Mikagura (`StartOfYourMainPhase`), BT17-007 Agumon (`StartOfYourMainPhase`), BT15-020 Gabumon (`StartOfYourMainPhase`), BT5-093 Tai Kamiya & Matt Ishida (`StartOfYourTurn` — enum declared, never enqueued), BT21-102 Tai Kamiya (`StartOfYourTurn`), EX9-021 Omnimon Alter-S (`EndOfAttack`), EX4-073 Omnimon Alter-B (`WhenAttacking`), EX1-068 Ice Wall! (granted `WhenAttacking` to targeted permanent), ST20-11 WarGreymon (`WhenAttacking`)
- **Effect text:** Various — "[Start of Your Main Phase] …", "[When Attacking] …", "[End of Attack] …", "[Security] At the end of the battle …"
- **What's missing:** `EffectTiming::WhenAttacking` and `EffectTiming::EndOfAttack` are in the enum but `Effect::when_attacking(card)` / `Effect::end_of_attack(card)` builder constructors don't exist, and combat doesn't enqueue either. `StartOfYourMainPhase` is entirely absent — existing `StartOfYourTurn` fires before Draw, not at Main-phase entry. `EndOfBattle` sub-timing for security effects is also absent (cards that say "[Security] At the end of the battle, …" need to fire after the Digimon-vs-security resolution, not on reveal).
- **Suggested API shape:** Add `EffectTiming::StartOfYourMainPhase` + fire from `enter_main_phase`. Add builder constructors and fire sites for `WhenAttacking` (in `combat::begin_attack` pre-block) and `EndOfAttack` (in `combat::cleanup_attack` before clearing `is_attacking`). For security sub-timing, either add `.security_timing(SecurityTiming::EndOfBattle)` on `Effect::security` or extend `OnEndBattle` firing to include security-card effects while `pending_security` is still live.
- **Workaround:** Collapse into nearest existing timing — violates no-approximations policy (order-sensitive with Block / Alliance / OnLoseSecurity).
- **Related:** RUST_ENGINE_API.md §9 ("OnEndBattle / OnEndAttack timings are not yet fired").

### Observer timings tied to specific events (`OnDigivolve` trait-filter, `OnSuspend`, `OnAttackTargetChange`, `[When Moving]`, `OnHatch`)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT24-082 Owen Dreadnought (`OnDigivolve` trait-filtered, with DP + extra-attack riders), BT24-089 Unique Emblem: Blazing Conductor (`OnSuspend` of named card), BT21-025 Lamiamon (`OnAttackTargetChange`), P-137 Flamedramon (`OnAttackTargetSwitched`), EX11-008 Elizamon (`[When Moving]` breeding→battle), BT16-082 Ukkomon (`[When Moving]` observer) — DNA Omnimon adds: BT15-101 MetalGarurumon (`OnSuspend` self-trigger), BT13-012 GeoGreymon (`OnSuspend` ally Tamer observer), P-123 Ukkomon (`[When Moving]` breeding→battle — shares shape with EX11-008/BT16-082), BT17-093 Tai Kamiya & Kari Kamiya (`OnHatch` — new sub-variant, fires when controller hatches in breeding), AD1-012 CresGarurumon (`OnAttackTargetChange` / `ctx.redirect_attack` primitive), EX4-039 Gabumon (`OnDigivolve` ally observer), EX4-003 Tsunomon (`OnDigivolve` ally — DigiEgg inherited), EX4-061 Matt Ishida & Tai Kamiya (`OnDigivolve` with digivolved-card name filter)
- **Effect text:** Various — "When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, …" / "When any of your [Owen Dreadnought]s suspend, …" / "When any of your … trait Digimon's attack targets change, …" / "[When Moving] [On Play] …"
- **What's missing:** `OnDigivolve` and `OnSuspend` enum variants exist but no trigger sources fire them. `OnAttackTargetChange` enum variant doesn't exist at all — no `combat.rs` emission from Block / Raid / Alliance redirect paths. `[When Moving]` has no variant (`OnEnterField` exists but is not observably fired from `Game::move_from_breeding` and doesn't broadcast to global observers).
- **Suggested API shape:** Wire `OnDigivolve` from `digivolve_from_hand` with `{digivolver, target}` context. Fire `OnSuspend` from `Permanent::set_suspended(true)`. Add `EffectTiming::OnAttackTargetChange` + emit from block-accept / raid-redirect / collision-redirect paths. Add `EffectTiming::WhenMoving` + fire from `Game::move_from_breeding` alongside a broadcast `OnEnterFieldAnyone`.
- **Workaround:** None — BLOCKED. Approximating with `OnEnterField` or periodic condition checks misses the causal link to the originating event.
- **Related:** None.

### `WhenWouldBeDeleted` / leave-field replacement-effect framework
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT24-018 Styracomon (Armor Purge + prevent-leave), EX11-012 Medusamon (delete token to cancel leave), BT24-012 Dimetromon (return self to hand to cancel leave), P-137 Flamedramon (Armor Purge), BT20-016 Paildramon (would-be-deleted → DNA digivolve) — DNA Omnimon adds: BT17-095 Miraculous Mega Knight (observer on "level 6 Greymon/Garurumon would leave outside of battle" — non-battle leave-cause discriminator), BT22-015 Omnimon (Decode — would-leave-outside-battle + play material free), AD1-025 Omnimon (Partition — "would leave other than by own effects or battle", requires source-player attribution), EX4-060 Omnimon Alter-S ("would leave other than by one of your effects" + play from own digivolution stack + place self to security bottom face-down), EX5-015 Gabumon (X Antibody) (pre-deletion-in-battle interrupt with optional cost), BT15-101 MetalGarurumon (Evade — suspend-self-to-prevent-delete), AD1-012 CresGarurumon (Evade), AD1-014 MetalGarurumon (Evade)
- **Effect text:** "When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion." / "When this Digimon would leave the battle area, by deleting 1 Token, it doesn't leave." / "When any of your other Digimon with the [Reptile] or [Dragonkin] trait would leave the battle area by your opponent's effects, by returning this Digimon to the hand, they don't leave." / "When any of your [Paildramon] or [Dinobeemon] would be deleted, 2 of your Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in the hand."
- **What's missing:** Rust's zone-transition paths (`delete_permanent`, return-to-hand, trash, bounce) complete unconditionally. There is no pre-resolution replacement hook that can (a) install a "may pay cost" prompt, (b) cancel the original mutation on acceptance, and (c) attribute the original cause to an opponent effect vs. combat vs. self. `OnDeletion` / `OnLeaveField` (where wired) are observers, not replacements.
- **Suggested API shape:** `EffectTiming::WhenWouldBeDeleted` / `EffectTiming::WouldLeaveField` that fires before resolution, receives a mutable `ReplacementContext { cancel: bool, source_player: PlayerId }`, and gates on the resolver checking `cancel`. Authors pay cost inside the closure and call `ctx.cancel_leave()`. Must carry source-attribution so "by your opponent's effects" filters work.
- **Workaround:** None — BLOCKED. Observer-style `OnLeaveField` cannot undo the transition.
- **Related:** None.

### Selection: multi-select with aggregate-sum constraint
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT17-018 Gallantmon: Crimson Mode, LM-021 Agumon – Bond of Bravery — DNA Omnimon adds: EX4-073 Omnimon Alter-B ("delete up to 6 play cost's total worth"), AD1-014 MetalGarurumon (pick N opponents per 2 own-tamer-colors), ST20-11 WarGreymon (per-Tamer-color N-Digimon immunity assignment), BT15-101 MetalGarurumon (pick 3 distinct opponent permanents for CannotUnsuspend), EX4-073 (self-stack materials multi-select with level filter — related pattern, may need a sibling `select_materials_multi` primitive)
- **Effect text:** "Choose any number of your opponent's Digimon whose total DP adds up to 15000 or less and delete them." / "Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP."
- **What's missing:** All `select_*` helpers pick exactly one. No primitive for "pick a subset with running aggregate ≤ N" with a PASS-to-finish terminator.
- **Suggested API shape:** `ctx.select_multiple_opponent_permanents(prompt, is_optional, filter_each, running_predicate: Fn(&Game, &[PermanentHandle], PermanentHandle) -> bool, callback: Fn(&mut Ctx, Vec<PermanentHandle>))`. Install a new `SelectionKind::MultiField` emitting PASS as terminator; accumulate picks until PASS or no valid remaining.
- **Workaround:** None — BLOCKED. Simplifying to "single highest-DP ≤ threshold" violates no-approximations.
- **Related:** Parity §4.6d-residual (selection-kind coverage).

### Selection: ordered permutation (place N cards in any order)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** P-035 Red Memory Boost!, P-151 Digimon Liberator, BT21-008 Elizamon, BT24-018 Styracomon, P-103 Offense Training, P-206 Digital Gate Open, EX7-074 Vortex Resonance, BT16-082 Ukkomon — DNA Omnimon adds: EX4-039 Gabumon, EX4-038 Agumon, BT12-059 Agumon, BT22-099 Kuremi Detective Agency, LM-034 Wisteria Memory Boost!, BT22-094 Yuugo Kamishiro, EX5-015 Gabumon (X Antibody)
- **Effect text:** "Return the rest to the bottom of the deck in any order." / "Place the remaining cards at the bottom of your deck in any order."
- **What's missing:** No `select_order(items, callback)` primitive. Cards commonly need to permute up to ~4 revealed cards for deck-bottom placement or digivolution-stack ordering.
- **Suggested API shape:** `ctx.select_ordering(prompt, candidate_count, callback: Fn(Vec<usize>))` — modeled either as a chain of single-select prompts with a running exclusion set, or as an action-space encoding of a permutation over ≤8 items.
- **Workaround:** Chained `select_reveal` with exclusion state in captured `Arc<Mutex<Vec<usize>>>` — functional but ergonomically expensive. Fidelity-preserving.
- **Related:** None.

### Selection: opponent-as-selecting-player, cross-side target, union-zone (hand OR trash), DNA-pair, multi-pick from reveal
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-024 Cyberdramon (opponent picks from own hand), BT24-016 Lamiamon (opponent places as security), BT21-029 Medusamon (opponent plays a token), BT20-102 Omnimon (X Antibody) (cross-side target), BT21-013 Agunimon (hand OR trash), P-189 Dimetromon (hand OR trash), EX7-074 Vortex Resonance (hand OR trash), P-206 Digital Gate Open (hand OR trash in [Security]), EX9-013 BlitzGreymon (DNA-pair), BT20-016 Paildramon (DNA-pair) — DNA Omnimon adds: BT17-095 Miraculous Mega Knight (hand OR trash in security effect + DNA-pair via effect), EX4-061 Matt Ishida & Tai Kamiya (hand OR trash play), BT17-093 Tai Kamiya & Kari Kamiya, BT22-008 Agumon (DNA-pair inherited EOT), BT22-017 Gabumon (DNA-pair inherited EOT), BT17-019 Gabumon (DNA-pair inherited EOT), BT17-007 Agumon (DNA-pair inherited EOT), AD1-009 BlitzGreymon (DNA into specific named hand card), AD1-012 CresGarurumon (defender-side reactive DNA on opp attack), BT17-078 Omnimon (Blast DNA pair), BT22-017 / EX4-039 / EX4-038 / BT12-059 (multi-pick from reveal with per-category filters — extends ordered-permutation), EX9-066 Tai Kamiya & Matt Ishida (if-effect-didn't-resolve on-decline callback hook), BT16-082 Ukkomon (optional hatch tail with on-decline)
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
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-017 Dimetromon, BT21-025 Lamiamon, BT24-016 Lamiamon, BT24-082 Owen Dreadnought, BT24-089 Unique Emblem, LM-027 Red Scramble, P-189 Dimetromon, P-151 Digimon Liberator, EX7-074 Vortex Resonance, P-206 Digital Gate Open, BT9-112 DeathXmon (dynamic cost) — DNA Omnimon adds (~16 more cards): BT17-095 Miraculous Mega Knight, BT22-084 Nokia Shiramine, BT22-089 Mirei Mikagura, BT17-081 Tai Kamiya & Matt Ishida, BT5-092 Nokia Shiramine, BT17-102 Greymon, BT21-102 Tai Kamiya, BT17-093 Tai Kamiya & Kari Kamiya, BT22-026 MetalGarurumon (via `[Hand][Main]`), EX4-061 Matt Ishida & Tai Kamiya, ST20-15 Island of Adventure, LM-034 Wisteria Memory Boost!, BT22-099 Kuremi Detective Agency, BT23-018 Garurumon (cost-reduction variant), BT22-094 Yuugo Kamishiro, BT13-012 GeoGreymon (play from security stack after search — needs a `play_from_security_at(player, security_index)` variant distinct from `play_from_security()` which reads `pending_security`)
- **Effect text:** "you may play 1 [X] from your hand without paying the cost" / "play 1 [X] from your trash without paying the cost" / "play 1 Tamer card … with the play cost reduced by 4"
- **What's missing:** `ctx.play_from_security()` exists; no analogous `play_from_hand_free(hand_index)` / `play_from_trash_free(trash_index)` / `play_from_hand_with_cost_delta(hand_index, delta)`. The cost-override variant is load-bearing for P-206's Delay sub-effect.
- **Suggested API shape:** `ctx.play_from_hand_free(player, hand_index) -> Option<PermanentHandle>`; `ctx.play_from_trash_free(player, trash_index) -> Option<PermanentHandle>`; `ctx.play_from_hand_with_cost_delta(player, hand_index, delta: i16)`. Each must fire `OnPlay` through the standard queue.
- **Workaround:** None — BLOCKED. Raw `player.hand.remove(i)` + `battle_area.push(Permanent::new(…))` skips OnPlay observers.
- **Related:** RUST_PYTHON_PARITY §1.1 (play cost deduction — this gap is upstream of the free-play variant but distinct), §2.5a (play_from_security landed).

### Zone-manipulation: effect-initiated digivolve (free / reduced / with trait filter / ignore requirements / DNA / Blast / detect-DNA-origin)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-001 Gigimon, BT21-093 Raging Serpentine, LM-027 Red Scramble, BT24-089 Unique Emblem, EX7-074 Vortex Resonance, BT23-005 Elizamon (passive trait-gated reduction), BT21-013 Agunimon, P-103 Offense Training — DNA Omnimon adds: BT22-013 WarGreymon (digivolve ignoring requirements + Tamer-gated cost override), BT17-015 WarGreymon, BT17-027 MetalGarurumon, BT22-026 MetalGarurumon, BT17-095 Miraculous Mega Knight (DNA digivolve via effect into named hand card with both materials still in hand), BT17-078 Omnimon (Blast DNA Digivolve from Counter — extends Counter window to pair two field materials with hand card), EX10-010 BlackWarGreymon (Blast Digivolve from Counter with Ace Overflow), AD1-009 BlitzGreymon (effect-initiated DNA into named hand card on EOT, outside Main phase), ST20-10 Agumon (alt-digivolve source registration with cost override + ignore reqs — `_alt_digi_*` scripting data channel), BT15-101 MetalGarurumon (alt-digivolve from hand), EX9-021 Omnimon Alter-S (needs `ctx.was_dna_digivolve()` context flag in `WhenDigivolving` — detect DNA origin), EX9-019 / EX9-012 / AD1-001 / AD1-010 (free-digivolve-from-hand-on-observer-trigger — hand-resident effects that initiate digivolve onto self when observer fires)
- **Effect text:** "1 of your Digimon may digivolve into a [X] trait Digimon card in the hand with the digivolution cost reduced by N" (and "without paying the cost" variants)
- **What's missing:** `Game::digivolve_from_hand` exists as an action entry but is not surfaced through `EffectContext`, and there is no way to apply a one-shot cost reduction or full-free flag to an effect-driven digivolve. `ModifierType::ChangeDigivolveCost` is permanent-keyed, not event-keyed. Passive "reduce by N when digivolving into trait-matched X" (BT23-005) needs a `BeforePayCost`-style hook during `digivolve_from_hand` that the current cost path doesn't consult.
- **Suggested API shape:** `ctx.prompt_digivolve(base_filter, target_filter, reduction: u8, is_optional, callback)` installs a chained own-permanent + hand-card selection and performs the digivolve at reduced/free cost. Extend `Game::digivolve_from_hand` to scan `ChangeDigivolveCost` modifiers with trait-filter predicates.
- **Workaround:** None faithful. The whole archetype's recurring digivolve-from-hand payoff (7+ cards) routes through this primitive.
- **Related:** RUST_ENGINE_API §9 ("BeforePayCost for cost reduction … not implemented").

### Zone-manipulation: return-to-hand / return-to-deck (top/bottom) / bounce self / trash-from-hand
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT24-012 Dimetromon (return self to hand), BT24-082 Owen Dreadnought (return Tamer to deck bottom), BT20-102 Omnimon (X Antibody) (return opp permanent to deck bottom), EX11-012 Medusamon (return card from opp trash to deck bottom), P-151 Digimon Liberator / BT16-082 Ukkomon / P-206 Digital Gate Open (return revealed rest to deck bottom), BT24-017 Medusamon (opponent returns 2 cards from trash to deck bottom as cost) — DNA Omnimon adds: BT17-078 Omnimon (return-to-bottom all opp Digimon of same level), BT22-015 Omnimon (return-to-bottom opp per-2-source-count), BT22-026 MetalGarurumon (return opp lowest-level to hand), BT22-089 Mirei Mikagura / BT21-102 Tai Kamiya / BT22-094 Yuugo Kamishiro / BT17-093 Tai Kamiya & Kari Kamiya (self-return to deck bottom as activation cost — needs `.pay_cost_return_self_to_deck_bottom()` builder hook), AD1-025 Omnimon, EX4-060 Omnimon Alter-S, EX1-021 MetalGarurumon (return opp On-Deletion-having Digimon to deck bottom), AD1-012 CresGarurumon (bounce lowest-level to hand), BT22-008 Agumon / EX9-066 Tai Kamiya & Matt Ishida / BT17-007 Agumon (return-from-trash-to-hand), BT22-089 Mirei Mikagura / EX5-015 Gabumon (X Antibody) (trash-from-hand by index — no `ctx.trash_from_hand(player, hand_index)` helper today)
- **Effect text:** Various — "return this Digimon to the hand" / "By returning this Tamer to the bottom of the deck" / "return 1 of your opponent's Digimon to the bottom of the deck" / "Return the rest to the bottom of the deck"
- **What's missing:** No helpers for: `return_permanent_to_hand(handle)`, `return_permanent_to_deck(handle, DeckEnd)`, `return_trash_to_deck(player, trash_index, DeckEnd)`, `return_revealed_to_deck(index, DeckEnd)`. `delete_permanent` trashes everything; there's no "extract top card, send materials to trash, move top to X" primitive.
- **Suggested API shape:** `ctx.return_permanent_to_hand(handle)`; `ctx.return_permanent_to_deck(handle, end: DeckEnd)`; `ctx.return_trash_to_deck(player, trash_index, end)`; `ctx.return_revealed_to_deck(reveal_index, end)`. Each must correctly route digivolution materials per the rules (top card → destination, others → trash) and fire appropriate triggers.
- **Workaround:** None — BLOCKED.
- **Related:** None.

### Zone-manipulation: reveal-top-N deck + add-to-hand + hatch
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-008 Elizamon, P-103 Offense Training, P-035 Red Memory Boost!, EX7-074 Vortex Resonance, BT16-082 Ukkomon, P-151 Digimon Liberator, P-206 Digital Gate Open, BT21-007 Agumon (return trash to hand — shares `add_to_hand`) — DNA Omnimon adds: BT22-017 Gabumon (reveal 3 + multi-pick by name + text filter + return rest to deck bottom), EX4-039 Gabumon, EX4-038 Agumon, BT12-059 Agumon, LM-034 Wisteria Memory Boost!, BT22-099 Kuremi Detective Agency, BT22-094 Yuugo Kamishiro, EX5-015 Gabumon (X Antibody) (reveal 4 + pick 2), P-123 Ukkomon (optional hatch via `ctx.hatch` helper), BT17-093 Tai Kamiya & Kari Kamiya (OnHatch trigger + hatch helper)
- **Effect text:** "Reveal the top N cards of your deck. Add 1 [card-kind/trait] card among them to the hand. Return the rest to the bottom of the deck. [Then, you may hatch in your breeding area.]"
- **What's missing:** `Game.revealed_cards` exists (§3.4 tensor scaffold) and `select_reveal` helper exists (§4.6d), but there is no `ctx.reveal_top(player, n)` that populates `revealed_cards`. No `ctx.add_to_hand(player, card)` (required by many search/recursion effects). No `ctx.hatch(player)` — `Game::hatch` is action-decoder-only.
- **Suggested API shape:** `ctx.reveal_top(player, n) -> &[CardSource]`; `ctx.move_revealed_to_hand(reveal_index)`; `ctx.move_revealed_to_deck_bottom_ordered(order)` (couples with ordered-selection gap); `ctx.add_to_hand(player, CardSource)`; `ctx.hatch(player)`.
- **Workaround:** Direct `ctx.game.player_mut(...)` mutation works but violates curated-API contract and bypasses `OnAddToHand` + hand-size-limit checks.
- **Related:** Parity §3.4 (revealed_cards scaffold landed).

### Zone-manipulation: security stack operations (trash top, place top/bottom, trash N, Recovery +N, shuffle security)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-025 Lamiamon, BT24-016 Lamiamon, LM-021 Agumon – Bond of Bravery, BT17-018 Gallantmon: Crimson Mode ("trash N from top"), BT24-016 Lamiamon / BT21-024 Cyberdramon (place as bottom security), P-137 Flamedramon (move top security to hand) — DNA Omnimon adds: BT22-013 WarGreymon / BT17-015 WarGreymon (trash opp security top from inherited effect), EX4-073 Omnimon Alter-B (trash top 2 opp security), ST20-15 Island of Adventure (add own top security to hand + place self on security top face-up), EX9-021 Omnimon Alter-S (place permanent as own top security), EX4-060 Omnimon Alter-S (place self at own security bottom face-down), BT13-012 GeoGreymon (Recovery +1 Deck — deck-top → security-top + shuffle security)
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

### Place card at a specific stack position (bottom-source / under another permanent) + alt-digivolve + stack reorder
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-013 Agunimon, BT24-016 Lamiamon (alt-digivolve variant) — DNA Omnimon adds: BT23-008 Greymon (move own top stacked card to own bottom source as activation cost — `ctx.move_source_to_bottom(target, source_index)`), BT23-018 Garurumon (same primitive)
- **Effect text:** "place 1 [Hybrid] or [Hero] trait Digimon card from your hand or trash as this Digimon's bottom digivolution card or under any of your red Tamers with inherited effects." / "by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."
- **What's missing:** No primitive to append a `CardSource` to the **bottom** of a permanent's `card_sources` (current digivolve always pushes top). No helper to target "another own permanent" as attachment point. No alt-digivolve primitive with override cost + ignore-digivolution-requirements flag.
- **Suggested API shape:** `ctx.place_as_bottom_source(target, card)`; `ctx.place_as_top_source(target, card)`; `ctx.digivolve_into_source_from_hand(target, hand_index, bottom_trash_index, cost_override: u16, ignore_reqs: bool)`.
- **Workaround:** None — BLOCKED. Raw `battle_area[i].card_sources.insert(0, ...)` skips OnEnterField / inherited-stack recomputation.
- **Related:** None.

### Native printed keyword parsing (Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-081, BT24-018, BT24-011, BT24-017, BT21-025, EX11-012, P-189, BT21-072, BT21-029, EX10-010, BT21-026, BT20-102, EX8-074, EX9-013, BT17-018, P-137, EX9-008 (inherited Raid), ST1-07 (inherited Sec A+1) — 17+ cards in Medusamon alone. DNA Omnimon adds: BT17-078 Omnimon (native Raid + Blocker), BT23-018 Garurumon (native Jamming), P-182 WarGreymon (native Security A.+1 + Blocker), AD1-001 Greymon (inherited Raid), AD1-010 Garurumon (inherited Jamming), BT17-095 Miraculous Mega Knight (printed Delay)
- **Effect text:** `<Rush>`, `<Raid>`, `<Piercing>`, `<Blocker>`, `<Reboot>`, `<Jamming>`, `<Blitz>`, `<Vortex>`, `<Alliance>`, `<Security A. +N>` printed on the card face
- **What's missing:** `CardData` has no `keywords: Vec<Keyword>` field; printed keywords live inside `effect_text: String`. Combat / mask / security modules consult only modifier-granted keywords via `ModifierRegistry::has_keyword`. Parity catalogs sub-cases for Rush (§2.1b), Blitz (§4.3b), Jamming (§2.5f) — this gap unifies them: an architectural cross-cutting fix covering all statically-printed keywords.
- **Suggested API shape:** Add `keywords: Vec<Keyword>` to `CardData` + ingest parse pass from `effect_text`, **or** auto-emit `Effect::declarative(card).grants_keyword(kw)` at `CardEffectRegistry` build time. Combat / mask helpers then OR modifier-granted with native. Native parsing must capture parametric variants (`SecurityAttackPlus(N)`, `DeDigivolve(N)`).
- **Workaround:** Per-card `Effect::on_play(card).process(|ctx| ctx.grant_keyword(self, Keyword::X, Expiry::Permanent))` — medium fidelity, but brittle for cards placed via Blast Digivolve / Training / material-reveal and doesn't populate face-keyword tensor slots.
- **Related:** RUST_PYTHON_PARITY §2.1b, §4.3b, §2.5f.

### `<Progress>` keyword + `ImmunityToOpponentEffects` modifier
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-025 Lamiamon, BT24-018 Styracomon, BT24-017 Medusamon, BT21-029 Medusamon, EX11-012 Medusamon, P-189 Dimetromon — DNA Omnimon adds (same `ImmunityToOpponentEffects` underlying need, non-Progress wording): EX9-021 Omnimon Alter-S ("opponent's effects don't affect this Digimon for the turn" — turn-scoped variant), AD1-009 BlitzGreymon (per-target opponent-only immunity until opp turn ends), EX10-010 BlackWarGreymon (DP-gated "opponent's Digimon's effects don't affect this Digimon" — couples with condition-gated modifier entries), ST20-11 WarGreymon (per-Tamer-color N-Digimon assignment of the same immunity)
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
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** P-103 Offense Training, LM-027 Red Scramble, BT24-089 Unique Emblem, P-035 Red Memory Boost!, BT21-093 Raging Serpentine, P-206 Digital Gate Open — DNA Omnimon adds: BT17-095 Miraculous Mega Knight (Delay on Ace Option), LM-034 Wisteria Memory Boost! (`<Delay>` gain 2 memory), BT22-099 Kuremi Detective Agency, BT23-096 Comet Hammer
- **Effect text:** "`<Delay>` (By trashing this card after the placing turn, activate the effect below.)"
- **What's missing:** Delay Options (a) stay on the battle area after initial resolution (tied to Option play-flow gap), (b) become activatable on turns after placement (`turn_played` tracking exists but no activation mask path), (c) activate by trashing self via an `[Main]` or reactive observer prompt. Multiple Delay variants have conditional activation triggers (BT24-089: OnSuspend of named card; LM-027: StartOfYourTurn + opponent-has-Digimon; BT21-093: OnOpponentSecurityRemoved).
- **Suggested API shape:** `Keyword::Delay` on Option-card permanents. `EffectTiming::DelayMain` (gated on `turn_count > turn_played`). Mask emits at a `FIELD_EFFECT_DELAY` slot when activation is legal. Activation trashes self from battle area and runs the post-Delay body.
- **Workaround:** None — BLOCKED. Intertwined with Option-card play flow.
- **Related:** See Option card play flow gap.

### Raid target-switch interrupt (scripting-surface, not mask-only) + effect-driven attack redirect
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT24-017 Medusamon, BT24-011 Cyclonemon, EX11-012 Medusamon, P-137 Flamedramon, BT21-025 Lamiamon (target-switch observer), plus every other Raid card — DNA Omnimon adds: BT23-008 Greymon (native Raid with switch-to-highest-DP), EX10-010 BlackWarGreymon (Raid on Ace), AD1-012 CresGarurumon (`ctx.redirect_attack(new_target)` primitive — "change the attack target to 1 of your Digimon" during opp-turn observer)
- **Effect text:** "`<Raid>` (When this Digimon attacks, you may switch the target of attack to 1 of your opponent's unsuspended Digimon with the highest DP.)"
- **What's missing:** Raid is currently mask-only (§4.4) — gates legal target bits at Main-phase selection. The card text is an **optional mid-attack switch** after declaration. No `RaidOpen` state in the attack state machine; no `OnAttackTargetChange` event fires even when redirection occurs through Block.
- **Suggested API shape:** Add `RaidOpen` state to `PendingAttack` between `Declared` and `AllianceOpen`. `combat::try_enter_raid` installs a may-switch selection of highest-DP unsuspended opponent Digimon; attacker PASS keeps declared target. Fire `EffectTiming::OnAttackTargetChange` after any switch (Block / Raid / Collision).
- **Workaround:** Mask-time Raid (§4.4) covers "pick the Raid target up front" but fails the text when attacking into security (no mid-attack redirect).
- **Related:** Parity §4.4 (Raid mask), §2.3 (combat interrupts).

### De-Digivolve N primitive (single + mass)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** EX9-013 BlitzGreymon ("De-Digivolve 3" single target), BT9-112 DeathXmon ("De-Digivolve 1" all opponent Digimon) — DNA Omnimon adds: EX4-073 Omnimon Alter-B (De-Digivolve 3 single), BT23-096 Comet Hammer (De-Digivolve 4), EX9-019 WereGarurumon: Sagittarius Mode (inherited [When Attacking][Once Per Turn] De-Digivolve 1)
- **Effect text:** "`<De-Digivolve N>` 1 of your opponent's Digimon. (Trash up to N cards from the top. You can't trash past level 3 cards.)"
- **What's missing:** `Keyword::DeDigivolve(u8)` exists in enums.rs, but no implementation that pops top N `card_sources` from a target permanent, stopping at the first Lv.3-or-lower revealed, moving popped sources to trash. No mass variant.
- **Suggested API shape:** `ctx.de_digivolve(target: PermanentHandle, amount: u8) -> u8` — pops while `popped < amount && next_top.level > 3`, moves each popped source to owner's trash, fires `OnTrash` / `OnLoseField` as appropriate. `ctx.de_digivolve_all_opponent(amount)` sugar for the mass case.
- **Workaround:** None — BLOCKED. Level-3 floor rule and trash routing need centralized handling.
- **Related:** None.

### Ace Overflow: inherited memory penalty on zone-change from field / under-card
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** EX10-010 BlackWarGreymon, EX9-013 BlitzGreymon, BT17-018 Gallantmon: Crimson Mode, LM-021 Agumon – Bond of Bravery — DNA Omnimon adds: BT17-078 Omnimon (Ace Overflow -5), BT17-095 Miraculous Mega Knight (Ace Option), ST20-11 WarGreymon (Ace)
- **Effect text:** "Ace Overflow `<-N>` (As this card moves from the field or under a card to an area other than those, lose N memory.)"
- **What's missing:** No Ace-card identification, no Overflow metadata, no zone-transition firing of a penalty effect. `EffectTiming::OnLeaveField` is declared but I couldn't locate a dispatch site. "Under a card" (digivolution stack) zone distinction needs modeling separately from `BattleArea`.
- **Suggested API shape:** `CardData::ace_overflow: Option<i8>` + firing of `OnLeaveField` with `LeaveFieldContext { destination: Zone }` from every zone-change path (permanent trash, return-to-hand/deck/security, digivolution-source → out-of-stack). `Effect::ace_overflow(n)` builder sugar.
- **Workaround:** None — BLOCKED.
- **Related:** None.

### Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated + suspend/self-return as cost)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT8-097 Crimson Blaze, BT9-112 DeathXmon, BT21-026 WarGreymon, EX8-074 MedievalGallantmon (selection-gated) — DNA Omnimon adds: BT17-027 MetalGarurumon (Tamer-name-gated play-cost reduction — BeforePayCost scan must include battle-area-effect condition closures, not just on-card effects), BT17-015 WarGreymon (same), BT5-092 Nokia Shiramine (suspend-this-Tamer-as-cost to reduce digivolve cost), BT23-008 Greymon (move-top-source-to-bottom-as-cost), BT22-094 Yuugo Kamishiro (return-self-to-deck-bottom-as-cost), BT23-018 Garurumon, ST21-13 Matt Ishida & T.K. Takaishi (suspend-this-Tamer-as-cost by trait filter). Surfaces a general `.pay_cost(|ctx| bool)` builder hook on `EffectBuilder` covering suspend-self, return-self-to-deck-bottom, trash-from-hand, and trash-material payment shapes
- **Effect text:** "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play." / "reduce its memory cost by 3 for each Digimon and Tamer your opponent has in play" / "reduce the play cost by 2 for each of your opponent's Digimon" / "by suspending 2 Digimon, reduce the play cost by 4"
- **What's missing:** `.cost_reduction(n)` accepts only a static `i32`. `BeforePayCost` dispatch is not wired into `calculate_play_cost` (API §9). No closure-valued reduction and no selection-at-cost-time (for the "suspend 2 Digimon" variant).
- **Suggested API shape:** `.cost_reduction_fn(|&EffectReadContext| i16)` closure-valued variant, evaluated inside `calculate_play_cost`. For selection-gated variants: `Effect::before_pay_cost(card).with_optional_payment(cost_delta, select_filter, execute)` — offers a prompt at cost-time, completes before resolving payment.
- **Workaround:** None — BLOCKED for BT8-097 (cost-reduction scanning isn't wired at all per §9).
- **Related:** RUST_ENGINE_API §9, Parity §4.7e (DigiXros cost reduction).

### Dynamic DP scaling modifier (per-stack-depth / per-opponent-board / per-color)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-072 Arresterdramon: Superior Mode (per digivolution cards), BT24-017 Medusamon (per opponent Digimon) — DNA Omnimon adds: P-182 WarGreymon (+1000 DP per distinct color across own Digimon + Tamers)
- **Effect text:** "This Digimon gets +1000 DP for each of its digivolution cards." / "this Digimon gets +2000 DP for each of your opponent's Digimon until their turn ends."
- **What's missing:** `EffectBuilder::dp_modifier(n)` is static. Per §13, modifier-registry DP grants are NOT summed into `source_dp_contribution` tensor slots — so `add_dp_modifier` also can't express tensor-correct dynamic DP.
- **Suggested API shape:** `.dp_modifier_fn(|&EffectReadContext| i16)` closure-valued variant evaluated at tensor-build time. Or `ModifierType::ChangeDpDynamic(Box<dyn Fn(...)>)` with tensor-aware summation.
- **Workaround:** Static snapshot at cast time for the opponent-scaling variant — fails faithfulness when opponent board changes. Per-stack-depth has no snapshot equivalent.
- **Related:** RUST_ENGINE_API §13.

### Condition-gated modifier entries + new `Expiry` variants
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** EX10-010 BlackWarGreymon (already listed) — DNA Omnimon adds: EX9-021 Omnimon Alter-S (source-scope condition — "opponent's effects only"), AD1-009 BlitzGreymon (same source-scope filter), AD1-014 MetalGarurumon ("can't suspend until their turn ends" — needs `Expiry::EndOfTargetsNextTurn` where the anchor is the MODIFIER TARGET's next turn end, not the source player's), EX1-068 Ice Wall! ("until the end of their next turn" — same `EndOfOpponentsNextTurn` / `EndOfTargetsNextTurn` need). Both the condition-closure and the new `Expiry` variants are prerequisites for `ModifierEntry` to faithfully represent these clauses
- **Effect text:** "While your opponent has a Digimon with 13000 DP or more, your opponent's Digimon's effects don't affect this Digimon, and it gets +3000 DP."
- **What's missing:** `ModifierEntry` has no condition closure (parity §4.7x). Can't express "active only while opp has ≥13k DP Digimon" without an observer for arbitrary DP-threshold transitions.
- **Suggested API shape:** Add `condition: Option<Box<dyn Fn(&EffectReadContext) -> bool>>` to `ModifierEntry`; or passive `Effect::declarative(card).modifier_when(type, value, condition)` builder that the affect-resolution code consults per query.
- **Workaround:** Permanent grant over-applies when condition is false.
- **Related:** Parity §4.7x.

### Player-scoped modifier registry (CannotPlayFromTrash, CannotPlayDigimonByEffect, OpponentCannotReduceDigivolveCost, IgnoreColorRequirement, MayAttackPlayerOnly)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT23-014 Gallantmon (CannotPlayFromTrash by effect), BT8-097 Crimson Blaze (CannotPlayDigimonByEffect), BT5-008 Gaossmon (opp cannot reduce digivolution costs), P-151 Digimon Liberator / EX7-074 Vortex Resonance / P-206 Digital Gate Open / ST22-08 Offensive Plug-In V (IgnoreColorRequirement aura) — DNA Omnimon adds: BT8-097 Crimson Blaze (also listed; `CannotPlayDigimonByEffect` specifically distinct from `CannotPlayFromHand` which is player-action-only), LM-034 Wisteria Memory Boost! / BT22-099 Kuremi Detective Agency / BT23-096 Comet Hammer / ST20-15 Island of Adventure (IgnoreColorRequirement variants, several with condition closures). BT17-081 Tai Kamiya & Matt Ishida surfaces a sibling need — `ModifierType::MayAttackPlayerOnly` (grant attacks to a named permanent scoped to player-target only, unlike the existing `MayAttack` which mask-emits both Digimon and player targets)
- **Effect text:** "Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash." / "Your opponent can't play Digimon by effects until the end of their turn." / "[Opponent's Turn] Your opponent can't reduce digivolution costs." / "While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's color requirements."
- **What's missing:** `ModifierRegistry` is keyed by `PermanentHandle` only — no player-scoped store. Missing variants: `CannotPlayFromTrash`, `CannotPlayDigimonByEffect`, `OpponentCannotReduceDigivolveCost`, `IgnoreColorRequirement`. Effect-vs-action-initiated play distinction isn't modeled either.
- **Suggested API shape:** Extend `ModifierRegistry` with `player_modifiers: HashMap<PlayerId, Vec<ModifierEntry>>` + `add_player_modifier / has_player_modifier / expire_*` with shared `Expiry` handling. Add the missing `ModifierType` variants. Consult `has_player_modifier` from every effect-play helper and the color-check mask.
- **Workaround:** None — BLOCKED.
- **Related:** Parity §4.2b (IgnoreColorRequirement), §4.7x (context-aware modifier queries).

### Option card play flow (resolve + trash vs. place-on-field; [Main]/[Security] activation) + Plug-In / Link mechanic + Security-effect return-to-hand / place-on-field
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** P-103, LM-027, BT24-089, BT8-097, P-035, BT21-093, EX7-074, P-151, P-206, BT1-090 (Option cards); ST22-08 Offensive Plug-In V (Plug-In / Link mechanic) — DNA Omnimon adds: BT17-095 Miraculous Mega Knight (Ace Option, Delay, security-effect "Then, add this card to the hand" — new disposition distinct from "trash" and "place on field"), LM-034 Wisteria Memory Boost! / BT22-099 Kuremi Detective Agency / BT23-096 Comet Hammer / ST20-15 Island of Adventure / ST2-13 Hammer Spark / EX1-068 Ice Wall! (Option cards needing [Main] flow). ST20-15 surfaces a further sub-gap: security-effect "place this card in the battle area" disposition (OptionSecurity → battle-area permanent)
- **Effect text:** All [Main] top-line clauses of Option cards; all "[Main] You may link this card to 1 of your Digimon without paying the cost" of Plug-In cards.
- **What's missing:** Per RUST_ENGINE_API §9, "Option cards have no play flow yet (they hit the field as a permanent like Digimon)." Need: (a) play path that fires `OptionMain` then trashes the card (or places it on field when `<Delay>`); (b) security effects that re-activate the card's [Main]; (c) `ctx.place_self_in_battle_area()` / `ctx.trash_self_from_option()` / `ctx.activate_own_main_effects()` helpers; (d) Plug-In / Link mechanic: `Permanent.linked_cards: Vec<CardSource>` storage exists but no `ctx.link_card_to_permanent(card, target)` API, no play-flow for Plug-In card kind, no link-cost evaluation, no interaction between linked Plug-Ins and their carrier.
- **Suggested API shape:** `Game::play_option_from_hand` branched inside `play_from_hand` based on `CardKind::Option`. `EffectTiming::OptionMain` and `OptionSecurity` (variants exist; need dispatch). `ctx.place_self_in_battle_area()` + `ctx.activate_own_main_effects()`. For Plug-In: `ctx.link_from_hand_to_own_permanent(filter, callback)` + `EffectTiming::OnLink` / `WhileLinked` + `ModifierType::LinkRequirement` metadata on `CardData`.
- **Workaround:** None — BLOCKED. Option-card play flow is a foundational architectural gap; Plug-In is arguably its own sub-spec.
- **Related:** RUST_ENGINE_API §9.

### Scheduled end-of-turn effect queue (for transient Options)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT1-090 Gravity Crush (shared across both audits — "[Main] Gain 2 memory. At end of turn, lose 2 memory.")
- **Effect text:** "[Main] Gain 2 memory. At end of turn, lose 2 memory."
- **What's missing:** Transient Option cards resolve + trash before `end_turn`; existing `EndOfYourTurn` timing only walks live permanents. No mechanism to enqueue a closure from an Option's `[Main]` that fires at turn end after the card is in trash.
- **Suggested API shape:** `ctx.schedule_end_of_turn(|ctx| { … })` enqueues a boxed closure onto `Game.scheduled_eot: Vec<…>`. `Game::end_turn` drains after standard `EndOfYourTurn` triggers, before memory reset.
- **Workaround:** None — BLOCKED.
- **Related:** Couples with Option-card play-flow gap.

### Effect re-firing / cross-timing self-trigger
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** EX8-074 MedievalGallantmon — DNA Omnimon additionally surfaces a *related-but-distinct* "grant triggered ability to another permanent" need; see "Granted triggered ability — attach an `Effect` to another permanent" new entry below
- **Effect text:** "[All Turns] [Once Per Turn] When Digimon are played, you may activate 1 of this Digimon's [When Digivolving] effects."
- **What's missing:** No API to invoke another of the source card's effects from within a `process` closure. `EffectContext` has mutation helpers but no `ctx.fire_effect_of_self(timing: EffectTiming)`.
- **Suggested API shape:** `ctx.fire_effect_of_self(timing)` — looks up source card's effects via `CardEffectRegistry::get(card_id)`, filters by timing + matching condition, enqueues via `effect_queue`.
- **Workaround:** Duplicating the [When Digivolving] body inline is brittle if the primary effect changes.
- **Related:** None.

### Force-follow-up-attack / "may attack without suspending" script helpers
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-081 Owen Dreadnought ("Then, that Digimon attacks"), BT24-082 Owen Dreadnought ("it may attack"), BT20-016 Paildramon ("this Digimon may attack"), BT20-102 Omnimon (X Antibody) ("attack without suspending"), BT21-072 Arresterdramon: Superior Mode ("This Digimon may attack without suspending"), EX9-013 BlitzGreymon ("1 of your Digimon may attack") — DNA Omnimon adds: BT20-102 Omnimon (X Antibody) (Rush + attack without suspending), AD1-009 BlitzGreymon ([End of Your Turn] "1 of your Digimon may attack"), BT22-015 Omnimon ("Then, this Digimon may attack" after WhenDigivolving — grant-attack-after-digivolve, expected to work even on negative memory). Related: BT17-081 Tai Kamiya & Matt Ishida needs the `MayAttackPlayerOnly` variant captured in the player-scoped-modifier entry above
- **Effect text:** Variants of "it may attack" / "attack without suspending" immediately following another effect.
- **What's missing:** Engine internally supports `PendingAttack::is_overclock = true` to skip suspend (§4.6c-residual) but the flag is not exposed to scripts. No `ctx.force_follow_up_attack(attacker)` / `ctx.grant_may_attack_without_suspend(target, expiry)`. `ModifierType::MayAttack` exists but is EndOfTurnAction-scoped; the immediate force-attack case needs a distinct primitive.
- **Suggested API shape:** `ctx.force_follow_up_attack(attacker: PermanentHandle)` installs an EndOfTurnAction-slotted attack scoped to that attacker. `ctx.grant_may_attack_without_suspend(target, expiry)` / new `ModifierType::AttackWithoutSuspend(u8)` consumed by `begin_attack_impl`.
- **Workaround:** None — BLOCKED.
- **Related:** Parity §4.6c / §4.6c-residual.

### Trait-filter helpers on `CardSource` / `Permanent`
- **Severity:** 🟡 PARTIAL — *ergonomics / sugar*
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-093, EX11-008, P-189, BT21-029, EX11-012 (LIBERATOR-typed), plus many search / filter effects — DNA Omnimon adds: BT22-005 Tsumemon ([Unidentified]/[CS]), BT22-089 Mirei Mikagura ([CS] / [Holy Beast] / [Angel] / [Archangel] / [Fallen Angel]), ST20-10 Agumon (cross-tamer trait union), BT22-099 Kuremi Detective Agency ([CS]), BT22-094 Yuugo Kamishiro ([CS]), BT22-084 Nokia Shiramine (named-trait aura), ST21-13 Matt Ishida & T.K. Takaishi ([ADVENTURE])
- **Effect text:** "1 of your [Reptile] or [Dragonkin] trait Digimon …" / "1 card with the [LIBERATOR] trait …"
- **What's missing:** `CardData.type_eng` is present, but no ergonomic `CardSource::has_type(&str)` / `Permanent::has_any_type(&[&str])` accessor. Authors dip into `ctx.card_data()[idx].type_eng.contains(...)` directly — verbose, case-sensitivity bugs likely.
- **Suggested API shape:** `CardSource::has_type(card_data, trait_name)` + `Permanent::top_card_has_type(...)` / `has_any_type(...)`, case-insensitive.
- **Workaround:** Raw card_data scan — functional but API-convention-violating.
- **Related:** Parity §2.1b (same effect-listing / text-parsing class).

### Granted triggered ability — attach an `Effect` to another permanent
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX1-068 Ice Wall! ("All of your opponent's Digimon gain \"[When Attacking] lose 2 memory\" until the end of their next turn.")
- **Effect text:** As above.
- **What's missing:** Distinct from "Effect re-firing / cross-timing self-trigger" (same-card timing invocation). `ModifierRegistry` carries scalar `ModifierType` values + `grant_keyword`; no primitive attaches a full `Effect` (timing + condition + process) to another permanent with bounded expiry. Python has `effect_grant_ability`.
- **Suggested API shape:** Extend `ModifierRegistry` (or add a sibling `GrantedEffectRegistry`) to hold `(target: PermanentHandle, effect: Arc<Effect>, expiry: Expiry)`. `enqueue_from_permanent` also walks `granted_effects[target]` when building the fire list. Expose `ctx.grant_effect(target, effect, expiry)`. Depends on `Expiry::EndOfOpponentsNextTurn` variant (see "Condition-gated modifier entries + new Expiry variants").
- **Workaround:** None — BLOCKED. Scalar modifiers can't represent "when this Digimon attacks, run X."
- **Related:** "Condition-gated modifier entries + new Expiry variants", "Effect re-firing / cross-timing self-trigger", "Named-target declarative aura".

### Named-target declarative aura (DP / keyword grants filtered by name/trait/level)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-084 Nokia Shiramine ("[All Turns] All your Digimon with [Greymon], [Garurumon] or [Omnimon] in their names get +1000 DP."), BT5-093 Tai Kamiya & Matt Ishida ("[Your Turn] All of your Digimon with [Omnimon] in their name gain ＜Security A. +1＞"), ST21-13 Matt Ishida & T.K. Takaishi ("[Your Turn] All of your level 5 or higher Digimon with the [ADVENTURE] trait gain ＜Rush＞")
- **Effect text:** As above.
- **What's missing:** `Effect::declarative(card).dp_modifier(n)` buffs only the source permanent and consumes a static integer ("Dynamic DP scaling modifier" entry addresses formula values on SELF). This gap is about broadcasting DP/keyword grants from one permanent to OTHER permanents on the same side, filtered by a live predicate (name/trait/level), re-evaluated as the field changes. Distinct from `grant_keyword`+`add_dp_modifier` manual iteration: leaks on new plays and can't be revoked when the source leaves.
- **Suggested API shape:** `Effect::aura(card).target_filter(|rctx, h| bool).grants_keyword(Keyword).dp_modifier(n)` consulted by `effective_dp`, `has_keyword`, and mask at query time. Alternative: `ModifierRegistry::query_aura` pass that iterates live aura sources whenever a permanent's effective value is asked.
- **Workaround:** None — BLOCKED. Manual per-permanent modifier application on every state change is not faithful.
- **Related:** "Dynamic DP scaling modifier", "Granted triggered ability", "Effect re-firing / cross-timing self-trigger", "Native printed keyword parsing".

### Declarative aura sourced from security zone
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** ST20-15 Island of Adventure ("[Security] [All Turns] All of your level 3 or higher Digimon get +2000 DP.")
- **Effect text:** As above.
- **What's missing:** Tensor / mask / modifier passes iterate only `battle_area` permanents. No "active aura sourced from a face-up security card" query path; `ctx.source_permanent` is `Option<PermanentHandle>` with no security-source variant. Card is an Option living in the security stack, not on the field — but its [All Turns] aura must still apply to friendly Digimon.
- **Suggested API shape:** Promote face-up security entries to enumerable effect sources; extend DP aggregation / keyword queries / tensor walks to include face-up security entries; add a `SecuritySource { player, security_index, card_index }` variant on effect-source handles.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY §3.3 (face_up_security tensor scaffolding); "Named-target declarative aura".

### Digivolution-stack name overlay ("has all names of materials")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-102 Greymon ("[All Turns] This Digimon has all the names of level 3 and lower cards in its digivolution cards.")
- **Effect text:** As above.
- **What's missing:** `Permanent::contains_card_name` already walks the stack for self-checks, but external name lookups on this permanent from other cards see only the top card's printed name. No "virtual name overlay" mechanism that synthesizes additional names for external queries (e.g., another Tamer's aura that checks "[Koromon]" should see the overlay names).
- **Suggested API shape:** `Effect::declarative(card).name_overlay_from_sources(|src, data| src.level(data).map_or(false, |l| l <= 3))`; update all name-lookup surfaces (aura filters, inherited-effect name checks, trait-from-name derivations) to union overlays into the lookup set.
- **Workaround:** None — BLOCKED for external observers that query names on this permanent.
- **Related:** "Named-target declarative aura".

### Decode keyword (play from own digivolution stack without paying cost on non-battle leave)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-015 Omnimon ("＜Decode (Red/Black Lv.3)＞ — When this Digimon would leave the battle area other than in battle, you may play 1 Red or Black Level 3 Digimon card from its digivolution cards without paying the cost.")
- **Effect text:** As above.
- **What's missing:** Composite of three primitives already tracked above: (a) `WhenWouldBeDeleted` / leave-field replacement framework with cause-discriminator (Battle vs Effect vs other); (b) `SelectSource` helper for choosing which material card to play; (c) a new `ctx.play_material_without_paying(target, source_index)` that pops a `CardSource` from the triggering permanent's `card_sources` and instantiates it as a fresh battle-area permanent firing OnPlay without memory cost.
- **Suggested API shape:** `Keyword::Decode(Vec<Color>, u8)` + a Decode-aware enqueue in the leave-field replacement path; `ctx.select_source(perm, filter, callback)`; `ctx.play_material_without_paying(source_perm, source_index)`.
- **Workaround:** None — BLOCKED. Auto-selecting a material violates §17 no-approximations; faking a "top-of-stack" play misses the selection semantics of "any material card" which the card text grants.
- **Related:** "WhenWouldBeDeleted / leave-field replacement-effect framework"; "Zone-manipulation: return-to-hand / return-to-deck / bounce self" (sibling for trash-stack-to-destination disposition); `select_source` is listed as 🔴-residual in RUST_PYTHON_PARITY §4.6d.

### Ergonomics partials

🟡 PARTIAL — *ergonomics / sugar*. These are expressible today but awkward; scripts currently reach around `EffectContext` or duplicate state. Filed to keep the authoring surface approachable as more cards land.

- **Per-permanent OPT activation recording** (BT23-008 Greymon, BT15-020 Gabumon, any `[Once Per Turn]` clause with compound sub-effects). `ctx.record_activation()` / `ctx.activation_count()` sugar over the existing `Permanent::record_activation` / `activation_count` methods, keyed by slot — flagged in RUST_ENGINE_API.md §13 as "nice follow-up".
- **Dual-timing composite clause builder** (ST20-11 WarGreymon, BT15-020 Gabumon — "[When Digivolving] [When Attacking] …"). `EffectBuilder::on_timings(&[EffectTiming])` that stamps out multiple `Effect` records sharing an `Arc`'d process closure, avoiding manual closure duplication.
- **Aggregate filter helpers** (BT22-013 lowest DP, BT22-026 lowest level, AD1-012 lowest level, ST20-11 lowest DP, EX10-010 Raid highest DP). `ctx.select_opp_permanent_min_by(|perm| extractor, …)` / `_max_by` sugar over the existing `select_opponent_permanent` filter closure — fully expressible today, just verbose.
- **If-effect-didn't-resolve on-decline callback** (EX9-066 Tai Kamiya & Matt Ishida, BT16-082 Ukkomon optional hatch tail). `PendingSelection.on_decline` field exists; no builder exposes it. Either `select_*_with_decline(..., on_decline)` or making the callback take `Option<usize>` where `None` means declined. Marked *primitive-with-fidelity-cost* (not pure sugar): today's closure-captured-bool workaround depends on the callback firing synchronously in the no-valid-targets / declined cases, which isn't guaranteed.

### `<Barrier>` keyword (battle-only leave-field replacement with security-trash cost)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** P-194 Aegiomon (face + inherited), BT24-034 Aegiomon (face + inherited), BT24-035 Gatomon (inherited), BT24-062 MasterBlimpmon (face), P-165 ShoeShoemon (inherited), BT24-039 Piximon (face), BT24-033 Salamon (inherited), EX11-019 Shoemon (inherited), BT24-024 Submarimon (face)
- **Effect text:** "＜Barrier＞ (When this Digimon would be deleted in battle, by trashing the top card of your security stack, prevent that deletion.)"
- **What's missing:** `Keyword::Barrier` and `ModifierType::GrantBarrier` are declared in `enums.rs` but `combat.rs` never consults either — the keyword has no behavior. Barrier is a leave-field replacement scoped specifically to "deleted in battle" (cause = combat), paying the top security card as cost. Requires: (a) the leave-field replacement framework with a `cause = Battle` discriminator (existing gap); (b) `ctx.trash_top_security(player, 1)` as a cost-payment helper (listed under the security-stack-operations gap); (c) auto-emission from native-keyword parsing so face-printed Barrier works without per-card scripts; (d) inherited-stack application (Effect::inherited(card) + the replacement-framework body).
- **Suggested API shape:** Built atop the "WhenWouldBeDeleted / leave-field replacement-effect framework" gap (cause = `Battle` branch). `Keyword::Barrier` + auto-emit `Effect::on_would_be_deleted(card).cause(Battle).optional().process(|ctx| { if ctx.trash_top_security(ctx.player, 1) > 0 { ctx.cancel_leave(); } })` at registry-build time when native keyword is parsed.
- **Workaround:** "None — BLOCKED." Sibling keyword to `<Armor Purge>` (same framework, different cost shape — trash top of own security instead of trash top of self's digivolution stack).
- **Related:** "WhenWouldBeDeleted / leave-field replacement-effect framework"; "`<Armor Purge>` keyword (leave-field replacement variant)"; "Native printed keyword parsing"; "Zone-manipulation: security stack operations".

### `<Collision>` keyword (attack-scoped opposing Blocker aura + must-block enforcement)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-063 Locomon (face + inherited)
- **Effect text:** "＜Collision＞ (During this Digimon's attack, all of your opponent's Digimon gain ＜Blocker＞, and must block if possible.)"
- **What's missing:** Three sub-parts: (a) `Keyword::Collision` variant + native-printed parsing; (b) an attack-scoped aura that grants `Keyword::Blocker` to every opposing battle-area permanent for the duration of the current attack (aura scope = "all opponents of attacker's controller", expiry = `EndOfAttack`); (c) a **must-block-if-possible** compulsion that, during the Block interrupt window, forces an unsuspended opposing Blocker to block rather than making it optional. (c) inverts the Block interrupt's "may block" default into a "must block unless no legal blocker" rule.
- **Suggested API shape:** `Keyword::Collision` + auto-emit a declarative attack-aura at native-keyword parse time. `AuraScope::OpposingBattleAreaDuringSelfAttack` + a `MustBlockIfPossible` flag consumed by the Block interrupt resolver — the defender's mask exposes block-only (skipping "decline block") whenever a legal blocker exists and the attacker has Collision.
- **Workaround:** "None — BLOCKED." Per-attack grant of `GrantBlocker` to every opponent permanent is expressible once Block interrupts land, but the must-block compulsion has no representation in the mask/interrupt surface.
- **Related:** "Native printed keyword parsing"; "Phase-granular turn timings"; RUST_PYTHON_PARITY §2.3 (combat interrupts); RUST_ENGINE_API.md §9 (Block / Counter / Alliance interrupt phases not yet wired).

### `Keyword::Decoy` color-filter parameter + replacement-framework wiring
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** ST12-12 Sistermon Blanc
- **Effect text:** "this Digimon gains ＜Decoy (Red/Black)＞ (When your other Red or Black Digimon would be deleted by an opponent's effect, you may delete this Digimon to prevent 1 of those Digimon's deletion.)"
- **What's missing:** `Keyword::Decoy` in `enums.rs` is a unit variant with no payload. The card text specifies which colors of Digimon the Decoy protects; there is no way to encode the color filter. `Decoy` and `GrantDecoy` are also entirely absent from `combat.rs` — no resolution path consults either. Requires both the color-parameter and the leave-field replacement framework.
- **Suggested API shape:** `Keyword::Decoy(Vec<Color>)` (or bitmask `u8` for copy-safety). Update `ModifierType::GrantDecoy` to carry the same payload. Replacement-framework hook filters candidate protected-ally slots by the color list. Native-keyword parsing emits `Decoy(colors)` when it parses `<Decoy (Red/Black)>` from card text.
- **Workaround:** "None — BLOCKED." A parameterless Decoy overapplies (protects all colors) and still requires the unimplemented replacement framework.
- **Related:** "WhenWouldBeDeleted / leave-field replacement-effect framework"; "Native printed keyword parsing".

### Trash all digivolution cards of a permanent (unbounded stack-peel)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-040 Venusmon
- **Effect text:** "Trash all digivolution cards of 1 of your opponent's Digimon."
- **What's missing:** No primitive to pop the entire `card_sources` stack beneath the top card of a target permanent and route them to trash. `ctx.de_digivolve(target, n)` (itself a tracked gap) stops at the first Lv.3-or-lower source — this clause explicitly does NOT stop at Lv.3, so it is a distinct primitive. `delete_permanent` trashes the whole permanent including the top card. Preserves the top card as a vanilla Digimon.
- **Suggested API shape:** `ctx.trash_all_digivolution_cards(target: PermanentHandle) -> u8` — drains the below-top sources to trash, preserves the top card, fires `OnTrash` per popped source.
- **Workaround:** "None — BLOCKED." Looping `de_digivolve(target, 1)` stops at Lv.3 and silently drops lower sources; raw stack drain skips trash-routing and observer triggers.
- **Related:** "De-Digivolve N primitive (single + mass)" (sibling primitive with bounded stopping rule).

### Permanent-scoped modifier to suppress effect activation by timing
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT10-042 Venusmon ("opponent's Digimon with ＜Security A.＞ … can't activate [When Attacking] and [When Digivolving] effects"), BT24-040 Venusmon ("2 of their Digimon or Tamers can't suspend or activate [When Digivolving] effects")
- **Effect text:** "can't activate [When Attacking] and [When Digivolving] effects" / "can't suspend or activate [When Digivolving] effects"
- **What's missing:** No `ModifierType` variant gates a permanent's effects out of a specific `EffectTiming`. `ModifierRegistry` covers DP, suspension, targeting, destruction; no per-target suppression of specific triggered-ability timings. `effect_queue::enqueue_from_permanent` and the digivolve-flow trigger dispatch must consult this modifier when fanning triggered abilities. Permanent-scoped (distinct from the player-scoped registry gap) and timing-parametric.
- **Suggested API shape:** `ModifierType::CannotActivateEffectsByTiming(EffectTiming)` applied via `ctx.add_modifier(target, ModifierType::CannotActivateEffectsByTiming(EffectTiming::WhenDigivolving), 1, Expiry::EndOfOpponentsTurn)`. Consult in every triggered-effect enqueue site; skip effects whose `timing` is suppressed. Or an aura form: `Effect::aura(card).filter(predicate).suppress_timings(&[WhenAttacking, WhenDigivolving])`.
- **Workaround:** "None — BLOCKED." Over-applying `CannotBeAffected` blocks far more than the card's text allows; per-effect enable flags would race with dispatch.
- **Related:** "Player-scoped modifier registry" (sibling, different scope); "Granted triggered ability — attach an Effect to another permanent".

### Grant Security A. ±N modifier to a targeted permanent (parametric `SecurityAttackChange`)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT10-042 Venusmon, P-134 Shoemon
- **Effect text:** "[When Digivolving] All of your opponent's Digimon gain ＜Security A. -1＞ until the end of your opponent's turn."
- **What's missing:** `Keyword::SecurityAttackMinus(i8)` / `SecurityAttackPlus(i8)` and `ModifierType::SecurityAttackChange` exist in enums.rs, but (a) no primitive grants a parametric Security A. keyword to a target with an expiry (`grant_keyword` takes a plain `Keyword`, which accepts `SecurityAttackMinus(1)`, but the security-check pipeline doesn't appear to consume granted variants — only the modifier's i32 delta); (b) the security-check pipeline in `combat.rs` must sum both native and modifier-granted Security A. deltas on the attacker; (c) no iteration helper `ctx.for_each_opponent_permanent(|h| …)` for mass application.
- **Suggested API shape:** `ctx.grant_security_attack_change(target, delta: i8, expiry)` wrapping `add_modifier(target, ModifierType::SecurityAttackChange, delta as i32, expiry)`. Plus `ctx.for_each_opponent_permanent(|h| …)` sugar for mass application (or an aura form for ongoing).
- **Workaround:** Manual loop over `battle_area(opp_id)` at firing time covers the snapshot variant — fidelity-acceptable for WhenDigivolving cards. Aura form ("all opp Digimon have…") remains BLOCKED.
- **Related:** "Named-target declarative aura (DP / keyword grants filtered by name/trait/level)"; "Native printed keyword parsing".

### Play / digivolve origin context flag ("if played by effects", "if digivolved by this effect")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-023 Calmaramon ("if played by effects, 1 of their Digimon or Tamers can't suspend"), BT14-033 Patamon ("If digivolved by this effect, you may place 1 yellow Vaccine card from hand to security bottom")
- **Effect text:** "if played by effects, …" / "If digivolved by this effect, …"
- **What's missing:** `EffectContext` carries no flag distinguishing action-initiated plays from effect-initiated plays, and no per-activation identifier for "digivolved by THIS effect" vs. any other effect-driven digivolve. Sibling to the `ctx.was_dna_digivolve()` need tracked inside the "Zone-manipulation: effect-initiated digivolve" gap.
- **Suggested API shape:** Add `PlayCause { Action, Effect { source_card: CardHandle } }` threaded through `Game::play_from_hand` / `digivolve_from_hand`. Expose `ctx.was_played_by_effect()`, `ctx.was_digivolved_by_effect(self_source_card) -> bool` sugar. Fold into the same context struct as `was_dna_digivolve`.
- **Workaround:** "None — BLOCKED." Auto-firing the rider unconditionally violates no-approximations; dropping it silently drops a clause.
- **Related:** "Zone-manipulation: effect-initiated digivolve" (setter site); `ctx.was_dna_digivolve()` item within that entry.

### Search-own-security-stack primitive (reveal full stack + select by filter)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT14-033 Patamon
- **Effect text:** "Search your security stack." (full-stack reveal feeding a downstream filtered choice; must be followed by a shuffle)
- **What's missing:** No `ctx.search_security(player, filter, callback)` prompt. Naive iteration would leak hidden-info through an unregistered `PendingSelection` and bypass mask emission. Distinct from `play_from_security` (which consumes the currently-revealed `pending_security` card on security-check resolution).
- **Suggested API shape:** `ctx.search_own_security(prompt, filter, optional, callback: Fn(&mut Ctx, Option<usize>))` — pushes a full-stack selection, returns the chosen security index; downstream code pairs with `shuffle_security` to re-hide info.
- **Workaround:** "None — BLOCKED." Mask-time exposure of security card IDs is a hidden-info correctness concern.
- **Related:** "Zone-manipulation: security stack operations"; "Effect-initiated digivolve from security stack" (co-required for Patamon).

### Effect-initiated digivolve from security stack (free, trait-filtered)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT14-033 Patamon
- **Effect text:** "This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them [= searched security stack] without paying the cost."
- **What's missing:** The existing "Zone-manipulation: effect-initiated digivolve" gap is framed around digivolving from the **hand**; its suggested `ctx.prompt_digivolve` reads the target out of `player.hand`. Patamon digivolves using a card pulled from the **security stack** — a source zone not covered by the existing API.
- **Suggested API shape:** `ctx.prompt_digivolve_from_security(base, security_index, cost_override, ignore_reqs, callback)` — or extend the existing `prompt_digivolve` with a `source_zone: Zone` parameter. Must remove the chosen card from `player.security` (not hand), push onto `base`'s `card_sources`, fire `WhenDigivolving`, honor the search-then-shuffle ordering.
- **Workaround:** "None — BLOCKED." Faking a hand transit (move security → hand → digivolve → discard leftover) violates no-approximations (wrong zones, wrong OnAddToHand/OnLoseSecurity fan-out).
- **Related:** "Zone-manipulation: effect-initiated digivolve"; "Search-own-security-stack primitive".

### `OnPlaceSecurity` / `OnAddedToSecurity` observer timing dispatch
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT14-033 Patamon (inherited "[Your Turn] [Once Per Turn] When a card is added to your security stack, gain 1 memory.")
- **Effect text:** "When a card is added to your security stack, gain 1 memory."
- **What's missing:** `EffectTiming::OnPlaceSecurity` is declared in `enums.rs:129` but has no fire sites. Every code path that pushes onto `Player.security` (setup, effect-driven place-top, Recovery +N, opponent-forced placement) must enqueue a fan-out to battle-area + inherited-stack observers. Sibling to `OnOpponentSecurityRemoved` — mirror primitive for addition events.
- **Suggested API shape:** Fire `EffectTiming::OnPlaceSecurity` after each push onto `Player.security` from all entry points. Context `{player, count_added, cause: PlaceCause::{Setup, Recovery, EffectPlaceTop, EffectPlaceBottom, OpponentForced}}`. Fan-out to every battle-area + inherited-stack effect.
- **Workaround:** "None — BLOCKED." Pure dispatch wiring; the enum variant already exists.
- **Related:** "Global `OnOpponentSecurityRemoved` observer timing" (mirror event).

### `OnDiscardSecurity` — effect-driven security-card trash trigger
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT13-106 Odin's Breath
- **Effect text:** "When an effect trashes this card from the security stack, activate this card's [Main] effect."
- **What's missing:** `OnLoseSecurity` fires unconditionally inside `resolve_security_card` (attack path) and `SecuritySkill` fires on the revealed card during attack checks. Neither models `OnDiscardSecurity`, which fires on the trashed security card only when removed by an **effect** (not by the normal attack reveal). Card should NOT trigger on attack-reveal (that fires SecuritySkill → [Security] re-activation) but SHOULD trigger when an effect trashes the top security card.
- **Suggested API shape:** `EffectTiming::OnDiscardedFromSecurityByEffect`. Fire from `ctx.trash_top_security(player, n)` iterating each popped card's own effects with this timing; condition = "trash initiated by an effect object, not the attack engine".
- **Workaround:** "None — BLOCKED." Even with `trash_top_security`, no mechanism fires the trashed card's observer.
- **Related:** "Zone-manipulation: security stack operations"; "Option card play flow"; RUST_PYTHON_PARITY §2.5b (OnSecurityCheck).

### `<Reboot>` keyword enforcement in opponent's unsuspend phase
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-058 Blimpmon (inherited `<Reboot>`)
- **Effect text:** "＜Reboot＞ (Unsuspend this Digimon during your opponent's unsuspend phase.)"
- **What's missing:** `Keyword::Reboot` and `ModifierType::GrantReboot` exist, but `game_phases.rs::begin_turn` calls only `player_mut(tp).unsuspend_all()` — no cross-player Reboot pass over the opposing battle area. Distinct from the native-keyword-parsing gap: even with Reboot correctly granted, there is no enforcement pass.
- **Suggested API shape:** In `begin_turn`, after `unsuspend_all()` on the active player, iterate `player(opponent_id).battle_area`/breeding for permanents with `Keyword::Reboot` and unsuspend each. Also fire `EffectTiming::OnUnsuspend` once that timing is wired.
- **Workaround:** "None — BLOCKED." Manual modifier grant doesn't help; the enforcement pass is absent.
- **Related:** "Native printed keyword parsing" (prerequisite for automatic grant); "Observer timings tied to specific events" (OnUnsuspend).

### Digivolution-stack source extraction (`pop_top_source` from named permanent)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-093 Temple of Beginnings
- **Effect text:** "You may place the top stacked card of any of your Digimon with [Aegiochusmon] or [Jupitermon] in their names as the top security card."
- **What's missing:** No helper to extract a `CardSource` from the top of a specific permanent's `card_sources` for arbitrary re-routing. `ctx.de_digivolve` pops+trashes and does not return the extracted card. Needs `ctx.pop_top_digivolution_source(target) -> Option<CardSource>` that removes the topmost digivolution source (not the active top card), returning it for caller placement (e.g., to security top), with no `OnDeletion` fire since the card is moved not deleted.
- **Suggested API shape:** `ctx.pop_top_digivolution_source(target: PermanentHandle) -> Option<CardSource>` — removes `card_sources.last()`, returns it for caller re-routing. Combined with `ctx.place_security_top(player, card)` from the security-stack-operations gap.
- **Workaround:** "None — BLOCKED." Raw `battle_area[i].card_sources.pop()` skips any `OnLeaveField` / inherited-stack recomputation and breaks the curated-API contract.
- **Related:** "Zone-manipulation: security stack operations"; "Zone-manipulation: return-to-hand / return-to-deck / bounce self".

### Fixed attack target — `CannotBeRedirectedAsAttackTarget` modifier
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-062 MasterBlimpmon (inherited "[Your Turn] This Digimon's attack target can't change.")
- **Effect text:** "[Your Turn] This Digimon's attack target can't change."
- **What's missing:** No `ModifierType` gates Block/Raid/Collision target-redirection per permanent. `try_enter_block` and Raid redirect paths unconditionally rewrite `effective_target`. Distinct from `CannotBeAffected` (suppresses effect-driven mutations, not combat-interrupt paths).
- **Suggested API shape:** `ModifierType::AttackTargetCannotChange`. In `try_enter_block` / `try_enter_raid` / any `effective_target` mutation site, guard: `if modifiers.has_modifier(declared_target, ModifierType::AttackTargetCannotChange) { skip redirect }`. Expose via `ctx.add_modifier(target, ..., Expiry::EndOfTurn)`.
- **Workaround:** "None — BLOCKED." No scripting-surface equivalent prevents Block/Raid redirect.
- **Related:** "Raid target-switch interrupt (scripting-surface, not mask-only)"; RUST_PYTHON_PARITY §2.3.

### In-effect branch-choice selector (`select_effect_choice` / "choose one of N effects")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** P-195 Inori Misono ("[On Play] Activate 1 of the effects below: ・…  ・…")
- **Effect text:** "[On Play] Activate 1 of the effects below: ・You may play 1 [Elecmon] from hand without paying cost. ・1 of your Digimon may digivolve into [Aegiomon] in the hand without paying cost."
- **What's missing:** No `ctx.select_effect_choice` primitive that presents a named menu of discrete branches and routes into the selected branch. `PendingSelection` covers zone-targeting but not abstract "which sub-effect" decisions. Auto-selecting violates §17; encoding as two independent optional effects changes semantics from "exactly one" to "at most one each".
- **Suggested API shape:** `ctx.select_effect_choice(prompt, choices: Vec<(label, process_fn)>, callback)` — installs `SelectionKind::EffectChoice(n)` in `PendingSelection`; action space maps to `EFFECT_CHOICE_0..N` slots; on resolution routes into `choices[i].process(ctx)`.
- **Workaround:** "None — BLOCKED." Two optional effects with a shared "already-chosen" flag corrupts the RL action distribution.
- **Related:** "Selection: opponent-as-selecting-player, cross-side target, union-zone, DNA-pair" (same selection infrastructure class).

## Deferred — verification / test coverage only

Items where the existing primitive **likely works** but no behavioral test covers the specific pathway. Not engine gaps; filed here so they surface when the archetype moves to `/batch-implement-cards-rust` and a faithful DebugRunner test must be written. **Do not count toward BLOCKING / PARTIAL tallies.**

- **Tamer play-from-security pipeline** — `ctx.play_from_security` was written against `CardKind::Digimon`; `CardKind::Tamer` routing through the same path + subsequent `[Your Turn]` / `[All Turns]` observers is unverified. Cards: BT17-081 Tai Kamiya & Matt Ishida, BT22-089 Mirei Mikagura, BT5-092 Nokia Shiramine, EX9-066 Tai Kamiya & Matt Ishida, ST20-15 Island of Adventure, EX4-061 Matt Ishida & Tai Kamiya (DNA Omnimon). See RUST_PYTHON_PARITY §2.5a, §2.5j.
- **Option multi-color match semantics** — RUST_PYTHON_PARITY §4.2 implements color match; verify multi-color Options require at least one matching own-side permanent **per** printed color (intersection), not any-one (union). Card: BT17-095 Miraculous Mega Knight (Red/Blue Option, DNA Omnimon). See RUST_PYTHON_PARITY §4.2, §4.2b.
- **Conditional inherited DP based on top-card name** — fully expressible today via `Effect::inherited(card).dp_modifier(n).condition(|ctx| ctx.source_permanent().map_or(false, |p| p.contains_card_name("X", ctx.card_data())))`. Confirm the per-source walker passes the correct `source_permanent` into the read context. Cards: BT12-059 Agumon, BT23-008 Greymon (DNA Omnimon).

## Resolved gaps

_(None yet — this document was created on 2026-04-17.)_
