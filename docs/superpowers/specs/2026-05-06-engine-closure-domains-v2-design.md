# Engine Closure Domains v2

**Date:** 2026-05-06
**Status:** Draft for implementation planning
**Supersedes:** `docs/superpowers/specs/2026-05-03-latest-archetype-dsl-engine-gap-closure-design.md` (the v1 closure spec)
**Companion to:** `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md` (per-archetype roadmap), `docs/superpowers/specs/2026-05-06-dcgo-resource-map.md` (DCGO file inventory per track)

## Context

Twelve archetypes have completed at least one Rust DSL implement run (BG Imperial, DNA Omnimon, Medusamon, TS Olympos, Rocks, Royal Knights, Puppets, Zephagamon, Chaos Control, Millenniummon, Red Hybrid AncientGreymon, Alter-S Ladder; Dark Masters has an engine audit but no DSL run yet). Across those runs, the same reusable engine blockers recur. The signal is not that individual cards need bespoke handling — it is that several engine domains still do not expose every printed Digimon TCG choice through PendingSelection, action masks, stable event context, observer-safe zone movement, modifier enforcement, and replacement-window dispatch.

This spec consolidates those blockers into engine closure domains. It supersedes v1 by:

- **Splitting modifier plumbing from modifier enforcement.** v1 folded both into Domain 7. The plumbing layer (registry, `Expiry` variants, source-scoped immunity) is a foundation that every later domain consumes; the enforcement layer (per-mutation-path checks) is the expressiveness layer that lands once subjects exist.
- **Promoting the aura system to its own domain.** Named-target auras, security-sourced auras, granted triggered abilities, name overlays, and sourced-keyword stack traversal share a write surface and were absent from v1.
- **Promoting non-replacement keywords to their own domain.** v1 covered Armor Purge / Decode / Barrier / Scapegoat inside replacement (Domain 2). Training, Digi-Burst, Decoy color-filter, Memory Boost, DigiXros alias, Retaliation, and Reboot enforcement have nowhere to live; they are now Domain 10.
- **Restating dependency tiers as parallel waves.** v1's "Implementation Order" was a 1–9 list with a "dependency-aware, not strict waterfall" hedge. This spec replaces that with explicit Tiers 0–4, a track-level parallelism matrix, and four labelled waves. Track J (DSL plumbing) is identified as broadly parallelizable behind a feature flag.

The live gap trackers remain the source of truth for status. This spec organizes work; tracker entries record outcomes.

### Primary input artifacts

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md`
- `qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md`
- `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`
- `qa/archetype-qa/dsl/chaos-control.md`
- `qa/archetype-qa/dsl/millenniummon.md`
- `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`
- `qa/archetype-qa/dsl/red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md`
- `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/zephagamon-2026-05-03-dsl-engine-gaps.md`
- `docs/superpowers/specs/2026-05-03-latest-archetype-dsl-engine-gap-closure-design.md` (v1)

## Goals

1. Define the minimum reusable engine capabilities needed to close every recurring Rust archetype blocker logged through 2026-05-05.
2. Group work by reusable engine domain with explicit dependency tiers, so multiple tracks can run in parallel without colliding on write surfaces.
3. Preserve the no-approximations rule: every gameplay choice that affects legal outcomes must be surfaced through an engine action or `PendingSelection`.
4. Keep `ACTION_SPACE_SIZE`, active tensor profiles, PyO3 exports, frontend constants, and RL wrappers stable. Any primitive that cannot fit existing infrastructure splits into a separate contract-change spec.
5. Provide concrete first fixtures and acceptance gates per domain.
6. Make tracker hygiene part of closure: a primitive is not closed until its tracker entries and source archetype reports are updated, with the test command that proves the new status.

## Non-Goals

- This spec does not implement any engine, DSL, YAML, or test changes.
- This spec does not authorize card-effect stubs, hidden auto-selection, raw-Rust no-op placeholders, or UI-owned rules logic.
- This spec does not authorize broad archetype YAML authoring. Production card migration happens only after the required reusable primitive exists and has a card-shaped regression test.
- This spec does not expand the action or tensor contract.
- This spec does not treat legacy Python or DCGO as authoritative. Printed text, the Comprehensive Rules Manual, and the fandom wiki remain higher priority — DCGO is a tiebreaker only.

## Closure Model

Engine closure proceeds in vertical slices. Each slice starts with one engine fixture that proves the primitive, then one card-shaped regression from an archetype report. DSL schema and lowering are included only when required to prove the primitive is usable without raw Rust.

The completion loop for every slice:

1. Read printed text for the chosen fixture in `data/cards.json`.
2. Write a failing Rust test under `code/digimon-engine/tests/`.
3. Implement the engine primitive through existing subsystems: action masks, `PendingSelection`, `EffectContext`, replacement dispatch, effect queue, zone movement, or modifier enforcement.
4. Add DSL schema/lowering only after engine behavior exists.
5. Add one card-shaped behavioral test from an archetype report — preferably a card already carrying an ignored gap test or a YAML comment referencing the gap ID.
6. Run targeted Rust tests (see Verification Matrix below).
7. Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and the source `qa/archetype-qa/dsl/*.md` rollup with the new status and proving test command.

### Using DCGO as a reference

DCGO (`DCGO/Assets/Scripts/Script/`) is the C# implementation reference. Its rules engine maps almost 1:1 onto our twelve tracks — every observer timing, replacement window, modifier class, keyword, and aura shape we need already has a single small file there. Each track below names the load-bearing DCGO files; `docs/superpowers/specs/2026-05-06-dcgo-resource-map.md` has the full inventory and a per-track reading order.

Use DCGO for **processing order, payload shape, and zone-fan-out behavior** — the things printed text and the rules manual underspecify. Per `CLAUDE.md` source priority: when DCGO and printed text disagree, printed text wins. Don't transliterate — DCGO is C# + Unity coroutines; Rust uses sync resolution + pending-selection state machines. The reference is for what happens, not how it's spelled.

## Impact Tiers

| Tier | Tracks | Why this tier |
|---|---|---|
| **T0** Foundation | A (event payloads & fan-out), B (replacement framework), C (modifier registry foundation) | Every later domain consumes payloads, leave-field hooks, or modifier taxonomy from these tracks. |
| **T1** Critical, parallelizable on T0 | D (combat interrupts), E (zone movement), F (selection primitives) | Each unblocks a high-frequency archetype core; disjoint write surfaces enable parallel work once T0 publishes contracts. |
| **T2** High value, mostly disjoint | G (keyword library), H (aura system), I (Option/Delay/Plug-In/security disposition) | Reach declarative coverage for cards whose engine subjects already exist after T0–T1. |
| **T3** Expressiveness | J (DSL predicate/formula plumbing), K (cross-card effect re-firing) | Lands once subjects, payloads, and selection shapes exist. J can land schema-only earlier behind a feature flag. |
| **T4** Migration gate | L (production YAML and regression gates) | Continuous; lags every primitive by exactly one card-shaped regression test. |

## Domains

### Track A — Event Context, Provenance, and Observer Fan-Out  [T0]

**Archetypes hit:** Medusamon, Royal Knights, Puppets, Zephagamon, Red Hybrid, DNA Omnimon, Rocks, Chaos Control, BG Imperial, TS Olympos.

**Problem.** Existing event dispatch carries enough information for simple triggers but not enough for cross-zone fan-out, deletion-cause discrimination, effect-initiated origin tracking, or stable provenance over later cleanup. Several archetypes also need observers in zones that are not currently scanned (breeding, hand, trash, security, linked Option, Delay).

**Required capabilities.**
- A trigger payload model carrying: event card, event permanent, former controller, affected player, source player, cause category, source effect/card, old/new attack targets, selected result bindings, moved-card sets, and effect-initiated origin flag.
- Fan-out policies for: battle-area, inherited stack (`enqueue_from_permanent` walking the digivolution stack), breeding-area, hand-resident, trash-resident, security-resident, linked/field Option, Delay observers.
- Deleted-object snapshots for `OnAnyDeletion` predicates: owner, kind, trait, level, DP, cause.
- Entered-play and digivolved payloads identifying the entering/resulting permanent — not the observer.
- Suspend/unsuspend payloads identifying the suspended permanent, whether an effect suspended it, and which effect/card caused it (Overclock cause discriminator).
- Stable provenance tokens returned by effect-play and effect-digivolve helpers for later cleanup, suppression, and result predicates.
- Reveal-zone overlay: declarative type/level synthesized while a card is in deck or being revealed.
- New observer timings: `OnHatch`, `EndOfOpponentsTurn`, `OnPlaceSecurity` / `OnAddedToSecurity`, `OnDiscardSecurity`, `OnAttackTargetChange`, `OnSuspend`, `[When Moving]`, `OnAllyAttack` / `OnOpponentAttack`, `OnAnyDigimonPlayed`, `OnAnyDeletion`, `OnOwnSecurityRemoved`, `OnOpponentSecurityRemoved`, `OnDigivolutionCardTrashed` (already partial — extend to `return_to_deck` / de-digivolve / Armor Purge / Fragment / Digi-Burst paths).
- Phase-granular timings: `StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle`.
- `OnDigivolve` trait-filter observer + `event_target_not_source` predicate.

**DCGO reference patterns.** `CardEffectCommons/CanUseEffects/` is a complete observer-timing dictionary — one tiny file per timing. Treat it as the canonical inventory; if our `Timing` enum lacks a variant DCGO has, that variant goes on Track A's backlog. Concretely:
- `GameContext.cs` (186 lines) is the model for `TriggerPayload`. Field set: `EventCard`, `EventPermanent`, source/target identity, attack target, etc. Copy field semantics, not the C# allocator.
- `IsDigivolvedByTheEffect.cs` (23 lines) is the effect-initiated origin flag — adopt the same boolean attribution rather than threading a parallel field.
- `CanUseEffects/PermanentEnterField/{PermanentEnterField.cs, OnPlay.cs, WhenDigivolving.cs}` show the entered-play vs. observer split: the entering permanent is the subject; the observer is wherever the handler lives. Adopt this split or our predicates will read the wrong card.
- `CanUseEffects/OnDeletion.cs` + `WhenDeleteOpponentDigimon.cs` + `WhenDeleteOpponentDigimonByBattle.cs` model the deleted-object snapshot + own/opponent + battle-cause discriminator. Snapshot is taken pre-removal so predicates see owner/kind/trait/level/DP/cause after the permanent leaves.
- `CanUseEffects/OnAttackTargetSwitch.cs` carries old/new target on the same payload.
- `CardSource.cs` is the source-stack walker for inherited dispatch (`enqueue_from_permanent` analog). Read the inherited-effect dispatch sites; the rule "one observer reachable through exactly one path" comes directly from how DCGO scopes scans here.
- `KeyWordEffects/Overclock.cs` (144 lines) tags suspend-cause for the Puppets G022 / Arisa Kinosaki branch. Cause-tagging on suspend events lives at the keyword emitter, not the observer.

**Representative first fixtures.**
- BT24-001 Gigimon: inherited-stack trigger walks the digivolution stack and fires once per event with stable host/source attribution.
- BT4-097 Kari Kamiya: own-side `OnOwnSecurityRemoved` observer + cause discriminator distinguishes battle damage from effect-driven trash.
- BT20-084 Sistermon Ciel (Awakened): trash-resident observer sees a Digimon-played event and digivolves the trash card onto the field.
- EX11-060 Arisa Kinosaki: deletion observer distinguishes Token/Puppet deletion from Overclock-caused deletion via cause category, and requires a visible suspend-this-Tamer cost.

**Acceptance gates.**
- Event predicates read the event subject, not the carrier or observer.
- Former controller and deletion cause are available after the permanent has left the battle area.
- Fan-out does not double-trigger the same effect through overlapping zone scans (one observer reachable through exactly one path).
- Once-per-turn and max-per-turn accounting still apply to inherited and off-field observers.
- The action mask exposes any optional follow-up choice the observer creates.
- Provenance tokens survive zone movement and battle-area shifts; later cleanup or suppression keys off the token, not an index.

### Track B — Replacement and Would-Leave Decisions  [T0]

**Archetypes hit:** Royal Knights, Puppets, TS Olympos, DNA Omnimon, Alter-S Ladder, Medusamon, Chaos Control, Red Hybrid, Rocks.

**Problem.** Cards saying "would leave", "would be deleted", "prevent it from leaving", "other than by your effects", Armor Purge, Decode, Partition, Barrier, Scapegoat, Fragment(N), Delay-as-prevention, and inherited Token/Puppet protection all require a pre-movement replacement window. The threatened subject and the replacement source are often distinct permanents, and the replacement may pay a cost via pending selection.

**Required capabilities.**
- Centralized leave-field attempt before destination mutation.
- Replacement context with threatened subject, source permanent/card, cause player, cause category, destination, battle/non-battle flag, and effect source kind.
- Pending-selection cost payment inside replacement resolution (no auto-pay, no auto-sacrifice).
- Non-cancelling would-leave observers that react to the same pre-move event and then allow the original move to continue (Royal Knights RK-G004 "play from hand without cancelling").
- Inherited replacement scanning under the threatened permanent (carrier identity preserved).
- Cross-permanent subject-guard so the protector's identity is validated before applying the replacement (TS G-TS-CROSS-PERMANENT-REPLACEMENT-PREVENTION; Medusamon replacement subject-guard).
- Inherited Token/Puppet leave-prevention dispatch where the threatened subject is a Token/Puppet on a different permanent (Puppets G019).
- Keyword-provided replacement emitters for: `<Armor Purge>`, `<Decode>`, `<Barrier>`, `<Scapegoat>`, `<Fragment (N)>`, Partition-adjacent source play, Delay-as-deletion-prevention.

**DCGO reference patterns.**
- `CanUseEffects/WhenPermanentWouldDigivolve.cs`, `WhenPermanentWouldPlay.cs`, `WhenWouldLink.cs` are the three pre-move replacement windows DCGO already names. Mirror these as named windows — not a single generic "would happen" hook — so cards subscribing to one window don't see firings from another.
- `CanUseEffects/WhenRemoveField.cs` is the cancellable side of the leave-field event. The non-cancelling would-leave observer (RK-G004 "play from hand without cancelling") is just a `WhenRemoveField` subscriber that returns `proceed=true` after running its side-effect. Don't model non-cancelling as a separate hook.
- Replacement-emitter shape: read in this order — `KeyWordEffects/Barrier.cs` (106, most concise), then `ArmorPurge.cs` (98), `Decode.cs` (113), `Fragment.cs` (82), `Scapegoat.cs` (71). Each is a self-contained emitter that registers a replacement at activation time and tears it down on resolve.
- `KeyWordEffects/Decoy.cs` (70) shows the color-filter parameter wired into the replacement framework — adopt the parameter shape directly so Track G's `<Decoy>` work doesn't reinvent it.
- `KeyWordEffects/Partition.cs` (178) shows source-zone constraint enforcement applied as a replacement, not as a play-time check. Partition-adjacent source play (Chaos Control) follows this model.
- `CardEffectFactory/CanNotBeDeleted.cs` family is the hook sites where Track C's source-scoped immunity is consulted. Track B's leave-field hook reads these via the modifier registry; Track C publishes the variants.
- `KeyWordEffects/Overclock.cs` is the canonical example of a replacement emitter that simultaneously tags cause for Track A's deletion-cause discriminator. Cross-track contract: emitters set the cause; observers read it.

**Representative first fixtures.**
- BT22-036 / EX11-022 inherited Puppet source: when the carrier would leave by an opponent effect, offer a Token/other Puppet deletion cost and prevent only that leave event.
- BT20-100 The Last Guardian: Delay replacement prevents a matching Omnimon from leaving by trashing the Option.
- BT20-091 Cool Boy: when a Royal Knight would leave, optionally play Omekamon without cancelling the leave event.
- Rocks Fragment(3) card (EX8-055, EX10-033/036, EX11-044): leave-field replacement via N-source self-trash.

**Acceptance gates.**
- Declining an optional replacement allows the original leave event to proceed.
- Paying a replacement cost cancels only the specific pending leave event.
- Cause filters distinguish own effects, opponent effects, battle deletion, non-battle deletion, return, deck bottom, and security placement where printed text requires it.
- Inherited replacement effects preserve source-card attribution and carrier identity.
- No replacement auto-pays a cost or auto-selects a sacrifice.
- Cross-permanent subject-guard rejects replacements where the protector is no longer eligible at the moment of resolution.

### Track C — Modifier Registry Foundation  [T0]

**Archetypes hit:** Rocks, Medusamon, Millenniummon, BG Imperial, TS Olympos, DNA Omnimon, Puppets, Zephagamon.

**Problem.** Many archetype blockers are listed as "missing modifier X". Implementing each one card-by-card creates an inconsistent enum and registry. The plumbing layer must publish its taxonomy and `Expiry` variants early so Tracks B, D, G, H, and I can consume them.

**Required capabilities.**
- Player-scoped modifier registry with named variants: `CannotPlayFromTrash`, `CannotPlayDigimonByEffect`, `CannotPlayTamerByEffect`, `OpponentCannotReduceDigivolveCost`, `IgnoreColorRequirement` (already done 2026-05-02), `MayAttackPlayerOnly`, `CannotReducePlayCost-bilateral`, `CannotAddSecurityByEffect`.
- Permanent-scoped modifier registry: `CannotAttackPlayer`, `CannotBeRedirectedAsAttackTarget`, timing-suppression (TS G-TS-TIMING-SUPPRESSION-MODIFIERS), `GrantCollision`.
- Source-scoped modifier registry: `CannotBeReturnedToHand`, `CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, with by-opponent-effects-only flag (Rocks BT18-064, EX8-005, BT21-055, EX10-025, EX8-047).
- Condition-gated modifier entries with new `Expiry` variants (`UntilLeaveField`, `UntilCondition`, etc.).
- Player-scope mass `CannotSuspend` aura (condition-gated / stack-depth-filtered).
- Storage layer that lets B (replacement) read source-scoped immunity at the leave-field hook, and lets D (combat) read attack-restriction modifiers at attack initiation.

**DCGO reference patterns.**
- DCGO splits one-shot from continuous: `CardEffects/*Class.cs` (73 files) emit one-shot modifiers during effect resolution; `AutomaticOrder/*.cs` (37 files) are the continuous counterparts re-evaluated on board change. Mirror this split. Don't model continuous as "many one-shots" — `Expiry` lifecycle and re-evaluation belong on the continuous side.
- `ContinuousController.cs` (1843) is the re-evaluator. Read it for the cycle order: board mutates → controller invalidates affected modifiers → modifiers re-attach → predicates re-fire. Our `Expiry` variants (`UntilLeaveField`, `UntilEndOfTurn`, `UntilCondition`) match what the controller already supports.
- DCGO's modifier-class inventory is the canonical naming source. Use these names in our `ModifierType` enum so cross-engine archetype reports stay greppable: `CanNotPlay`, `CanNotEvolve`, `CanNotPutField`, `CanNotSuspend`, `CanNotUnsuspend`, `CanNotMove`, `CanNotReturnToHand`, `CanNotReturnToLibrary`, `CanAttackTargetDefendingPermanent`, `CanNotAttackTargetDefendingPermanent`, `CanNotSwitchAttackTarget`, `CannotBlock`, `CannotAddMemory`, `CannotAddSecurity`, `CanNotAffected`, `ImmuneFromDPMinus`, `ImmuneFromDeDigivolve`, `ImmuneFromStackTrashing`, `DisableEffect`, `VortexCanAttackPlayers`, `IgnoreColorCondition`.
- `CardEffects/DisableEffectClass.cs` is the direct reference for permanent-scoped timing-suppression (TS G-TS-TIMING-SUPPRESSION-MODIFIERS). Modifier carries the suppressed timing as a parameter; the dispatch hook in Track A reads it before firing observers.
- Source-scoped immunity (Rocks BT18-064 etc.) maps to DCGO's `ImmuneFromStackTrashingClass.cs` + `CannotReturnToHandClass.cs` + `CannotReturnToLibraryClass.cs`. Read these to confirm the by-opponent-effects-only flag attaches to the modifier itself rather than to the consulting hook.
- `IgnoreColorConditionClass.cs` (already 🟢 in our engine 2026-05-02) — diff against this DCGO file to verify our implementation matches behavior, especially around digivolution color requirements vs. play color requirements.

**Representative first fixtures.**
- Rocks BT18-064: source-scoped `CannotBeReturnedToHand` blocks opponent's return-to-hand effect but allows own return.
- Medusamon: `CannotPlayTamerByEffect` modifier blocks effect-driven Tamer plays.
- TS Olympos timing-suppression slice: a permanent-scoped modifier suppresses one specific timing window without disabling the whole effect.

**Acceptance gates.**
- Every modifier variant lists which mutation paths must check it.
- Source-scoped modifiers are evaluated against the threatened subject in B's replacement context.
- Modifier enforcement does not silently expand to broader categories than printed text states (no broad `CannotBeAffected` for narrow protection).
- `Expiry` variants are tested for both expiration timing and re-application.

### Track D — Effect-Created Attacks and Combat Interrupts  [T1]

**Archetypes hit:** TS Olympos, Royal Knights, Zephagamon, Red Hybrid, DNA Omnimon, Alter-S Ladder, Medusamon, BG Imperial.

**Problem.** Many cards create an attack from inside effect resolution, allow a Digimon to attack without suspending, force an opponent's Digimon to attack, redirect an existing attack target, observe attack-target changes, or grant Collision. Treating these as automatic attacks hides legal choices and breaks combat timing. Several non-redirect combat keywords also need enforcement.

**Required capabilities.**
- `EffectContext` helper for optional and mandatory effect-created attacks (`initiate_attack` with PASS where optional).
- Attack-flow state openable from an effect outside the normal main-phase attack window.
- Flags for `without_suspending`, player-only targets, Digimon-only targets, selected-attacker-only, forced-opponent attack.
- Combat interrupt for retargeting an existing attack: Raid, Collision, Blocker-adjacent redirects, card-specific target switches.
- `OnAttackTargetChange` payload with attacker, old target, new target, controller, reason.
- Distinction between `battle:` effects and attacks: effect battles must not trigger attack-only security continuation or `<Piercing>`.
- `<Retaliation>` keyword + combat enforcement.
- `<Reboot>` enforcement during opponent's unsuspend phase.
- `<Piercing>` security continuation after a winning battle.
- `ModifierType::GrantCollision` honored by `combat::try_enter_block`.
- Counter window + `<Blast Digivolve>` activation flow ([Hand][Counter] play path) — DNA Omnimon G-DNAOmni-03, Zephagamon ZEPH-G008.
- Effect-driven attack cancellation (`ctx.end_pending_attack()`) — Rocks BT20-055, EX10-003.

**DCGO reference patterns.**
- `AttackProcess.cs` (628) is the definitive attack state machine. Read it end-to-end before designing the effect-attack helper. The phases (declaration, target lock, interrupts window, Blocker resolution, Counter window, damage step, security trigger, end-of-attack) carry over directly — Rust just expresses them with `pending_selection` instead of Unity coroutines.
- `KeyWordEffects/Raid.cs` (122), `Blocker.cs` (86), `Collision.cs` (31), `Alliance.cs` (220) are the four interrupt patterns. Each registers an interrupt at attack-flow open and consumes it at the right phase. Our `OnAttackTargetChange` payload (Track A) is fed by Raid/Collision/effect-redirect interrupts uniformly.
- `KeyWordEffects/Pierce.cs` (85) gates security continuation on attack-context only. Use this as the proof point for the "effect battles must not trigger Piercing" acceptance gate — `IgnoreBattle.cs` flag from Track A's payload is the input.
- `KeyWordEffects/Retaliation.cs` (149) and `Reboot.cs` (43) are non-redirect enforcement. Reboot in particular has a tiny implementation — it's a phase-window mod registered against opponent-unsuspend.
- `SelectAttackEffect.cs` is how DCGO opens an attack flow from inside effect resolution: it pushes a selection, suspends the effect, and resumes after the attack closes. Adopt this resume pattern — don't try to inline attack resolution inside the effect step.
- `SelectBurstDigivolutionEffect.cs` is the Counter Blast `[Hand][Counter]` activation flow. The Counter window opens during damage step; Blast Digivolve is offered as a pending selection from hand. Same suspend/resume shape as effect-created attacks.
- `MainPhaseAction/AttackPermanentAction.cs` is the natural attack action — diff against the effect-attack helper to confirm restriction flags (`without_suspending`, player-only targets) don't leak into ordinary future attacks.

**Representative first fixtures.**
- BT24-037 TS Olympos may-attack branch: after the effect resolves, select one eligible Digimon and expose a normal attack flow with PASS available before attack commitment.
- BT20-102 Royal Knights end-of-turn attack-without-suspending.
- ST18-14 Zephagamon: suspend-this-Tamer cost redirects an existing attack to another Digimon or the player.
- AD1-012 attack-redirect step verb fixture for `redirect_attack_target`.

**Acceptance gates.**
- Optional effect-created attacks expose decline/PASS through the action mask.
- Mandatory attacks still use target legality and pending selection where a target choice exists.
- Attack-state restrictions do not leak into ordinary future attacks.
- `OnAttackTargetChange` fires once per successful retarget and carries enough context for inherited predicates.
- Effect battles remain distinct from attacks (no Piercing security continuation).
- `<Retaliation>` and `<Reboot>` are enforced at every relevant mutation path; `<Piercing>` only triggers on attacks, not effect battles.

### Track E — Zone Movement and Source/Material Operations  [T1]

**Archetypes hit:** Royal Knights, Alter-S Ladder, DNA Omnimon, TS Olympos, Red Hybrid, Puppets, Rocks, Millenniummon, Medusamon, BG Imperial.

**Problem.** Effects need to move cards between hand, trash, security, deck, breeding, source stacks, and battle area while preserving ownership, event dispatch, trigger provenance, and player-visible choices. Raw list mutation and index-based references skip observers and can move the wrong card after battle-area shifts.

**Required capabilities.**
- Stable source/material selectors for battle and breeding carriers (already partial — extend `select_own_sources` outer-tail when selection is empty).
- Play from source/materials with name uniqueness, source-zone identity, On Play suppression flags.
- Effect play from hand/trash/security with `PlayOptions` (suppress_on_play, cost override, ignore color requirement).
- Effect digivolve from hand, trash, source, security, mixed material zones — with source/result provenance and DNA-origin context (BG Imperial G-BG-02/03).
- Security movement helpers: selected security, top/bottom face-up/face-down placement, top security to hand, self-to-security, stacked-card-to-security.
- Bulk zone moves: `return_all_trash_to_deck_bottom` (BT17-077), `trash_top_n_digivolution_cards` (BT12-028), forced opponent hand reduction (`ctx.trash_opponent_hand_to_count`).
- Owner-routed deck-bottom/top placement for cards moved from shared or opponent-controlled zones.
- Cast-time stack construction for cost reduction: place N differently-named cards from battle-area/trash UNDER the played card during play.
- Effect-played permanent cleanup provenance (Puppets G003 / G030; BT5-106 played-Digimon On Play suppression).
- `ctx.move_from_breeding()` helper (Rocks P-130).
- Trash all digivolution cards of a permanent (unbounded stack-peel).
- Pop-top-source from a named permanent.
- Search-own-security-stack primitive (reveal full stack + select by filter).
- Effect-initiated digivolve from non-hand source zones (already partial — extend to security stack with trait filter).

**DCGO reference patterns.**
- `CardSource.cs` (4323) is the source-stack mutator. Every zone move that touches a digivolution stack routes through methods here — none mutate raw vectors. Adopt the same discipline: a `move_*` helper per logical operation, never an in-place `Vec` splice.
- `Permanent.cs` (4140) carries owner vs. controller separately. Owner-routed deck-bottom/top placement (acceptance gate "owner routing is correct even when control differs") relies on this split — copy it into our `Permanent` so movement helpers don't need to thread an owner argument.
- `CardEffectCommons/IsDigivolvedByTheEffect.cs` (23) is the provenance flag. Our effect-digivolve helpers set this; the natural digivolve path leaves it `false`. Down-stream observers (Track A) read the same flag rather than threading "effect-initiated" through every call.
- `CardEffectCommons/RevealLibrary.cs` is the reveal-zone overlay. The overlay attaches to revealed cards and tears down on resolve — predicates checking type/level read the overlay rather than the printed card data.
- `CardEffectCommons/SelectAssemblyClass.cs` + `CanSelectAssemblyClass.cs` are cast-time stack construction (place N cards under played card during play). The selection happens before On Play, with the cards installed into the source stack pre-On-Play. Track E's "cast-time stack construction places the chosen cards into the played permanent's source stack before On Play resolves" acceptance gate is a direct port.
- `CardEffectCommons/TrashDigivolutionCards.cs` is unbounded stack-peel. Don't model as N-step loop — it's a single bulk operation that fires `OnDigivolutionCardTrashed` per source from Track A.
- `MainPhaseAction/PlayCardAction.cs` is the natural play action; diff against effect-driven play to confirm `suppress_on_play` and cost-override flags don't bleed into the natural path.
- DCGO has no `move_from_breeding` helper — breeding-area movement happens through `Permanent.cs` directly. Our helper (Rocks P-130) is a small ergonomic addition; mirror DCGO's underlying transitions.

**Representative first fixtures.**
- BT13-112 Royal Knights: choose distinct-name Royal Knight sources under King Drasil, play them, suppress On Play where printed, trash King Drasil, grant Rush.
- BT5-106 Demonic Disaster: Security plays a level 3 purple Digimon from trash while suppressing only that played Digimon's On Play effects.
- BT17-077 Imperialdramon: Paladin Mode: trash all opponent sources, return a chosen player's trash to deck bottom, bind moved-card results for memory gain.
- EX10-032 Proganomon (Rocks first regression): cross-permanent source selection trashes one chosen `[Mineral]`/`[Rock]` source and fires only that source's inherited "when this card is trashed from digivolution cards" effect.

**Acceptance gates.**
- Moving security cards fires security-removed observers exactly when it should.
- Source/material movement removes the exact selected source from the original carrier.
- Effect-created permanents return a stable handle (provenance from Track A) for scheduled cleanup or later branch predicates.
- Owner routing is correct even when control differs from ownership.
- No helper mutates raw vectors without routing through observer-safe movement APIs.
- Cast-time stack construction places the chosen cards into the played permanent's source stack before On Play resolves.

### Track F — Selection Primitives and Action-Mask Surfaces  [T1]

**Archetypes hit:** Royal Knights, DNA Omnimon, Alter-S Ladder, Medusamon, Red Hybrid, Puppets, Rocks, Zephagamon, BG Imperial, TS Olympos.

**Problem.** The engine needs a complete set of reusable pending-selection shapes for printed choices: aggregate DP-budget multi-select, exact-N trash selection, up-to-N partial selection, reveal buckets, ordered remainders, opponent-as-selecting-player, union-zone choices, security selection, source-stack selection, DNA pair selection, and selected-property binding.

**Required capabilities.**
- Aggregate-sum multi-select with running DP or play-cost budget.
- Exact-N and up-to-N multi-select with PASS/finish semantics.
- Ordered permutation over revealed or deck-remainder cards.
- Union-zone selectors that preserve origin zone identity (hand ⋃ trash, hand ⋃ digi-stack, breeding-permanent + battle-permanent).
- Opponent-controlled selections over opponent hand/security/trash where printed.
- Security selectors respecting visibility and stable index behavior.
- DNA and Blast DNA pair selectors, including mixed field plus hand materials.
- Binding selected properties (level, play cost, color, trait, DP, source count) for later mass application.
- Filtered breeding-permanent selection (RK-G001).
- Search-own-security-stack: reveal full stack + select by filter.
- Multi-bucket reveal search with ordered-reveal semantics (G-TS-MULTI-BUCKET-REVEAL-SEARCH).
- Place self at top/bottom of own security stack with face-up/face-down orientation choice (EX4-060, EX9-021).
- Outer-tail continuation when `select_*` has no candidates (G-SELECT-EMPTY-OUTER-TAIL, BT21-024).

**DCGO reference patterns.**
- `UserSelectionManager.cs` (161, small) is the pending-selection lifecycle. Read first — its push/peek/resume shape is what `code/digimon-engine/src/selection.rs` already mirrors, so the gap analysis is mostly "which selection effect classes haven't we ported yet".
- DCGO has one selection effect class per shape, all in the same directory. Use these as the primitive checklist — when our shape inventory is missing one, the gap is real:
  - `SelectCountEffect.cs` — generic count-based (closest to exact-N / up-to-N).
  - `SelectCardEffect.cs` — generic card pick.
  - `SelectHandEffect.cs` — hand-specific.
  - `SelectPermanentEffect.cs` — permanent pick (filtered breeding-permanent / RK-G001 lives here).
  - `SelectAssemblyClass.cs` — cast-time multi-pick under-card placement.
  - `SelectDigiXrosClass.cs`, `SelectAppFusionEffect.cs`, `SelectBurstDigivolutionEffect.cs`, `SelectJogressEffect.cs` — pair / multi-source for DigiXros / App Fusion / Counter Blast / DNA. Adopt the per-mechanic class shape; don't try to unify them prematurely.
  - `SelectDNACondition.cs` — DNA-pair condition predicate.
  - `SelectAttackEffect.cs` — attacker / target.
- `CardEffectCommons/MinMax_DP_Cost_Level/{Cost,DP,Level,DigivolutionCards}/` is the aggregate-constraint family. DP-budget multi-select reads the running aggregate from the matching `DP/` evaluator; cost-budget multi-select reads from `Cost/`. Property bindings (level, play cost, color, trait, DP, source count) for later mass application are computed here — bind into the selection state, not into card-local scope.
- `Combinations.cs` is the combinatorial enumerator. For "any number of opponent Digimon whose total DP is 15000 or less" (BT17-018), DCGO doesn't enumerate combinations — it walks the selection incrementally, marking each candidate as enabled/disabled based on running budget. Adopt this shape; combinatorial enumeration explodes the action space.
- DCGO uses `HideCannotSelectObject.cs` to gray out illegal candidates without removing them. Our action-mask layer already does the same — the acceptance gate "candidate identities remain stable after earlier selected candidates move" is a direct port of DCGO's identity behavior.

**Representative first fixtures.**
- BT17-018 Gallantmon: Crimson Mode: choose any number of opponent Digimon whose total DP is 15000 or less.
- BT24-017 Medusamon: exact-count opponent trash selection gates follow-up token creation.
- BT17-078 Omnimon: select one opponent Digimon, then bottom-deck all opponent Digimon with the same level.
- BT21-024 select-hand outer-tail: empty select continues outer tail synchronously.

**Acceptance gates.**
- The mask exposes all legal combinations through a sequence of legal choices and a finish action.
- PASS is illegal before a mandatory minimum is met and legal once optional completion is allowed.
- Candidate identities remain stable after earlier selected candidates move.
- Separate zones are not decomposed into sequential prompts when printed text offers one combined choice.
- Property bindings persist into later effect steps (formula evaluation in J).

### Track G — Keyword Library  [T2]

**Archetypes hit:** Rocks, DNA Omnimon, Medusamon, Royal Knights, BG Imperial, Chaos Control, Puppets, Zephagamon.

**Problem.** Several non-replacement keywords have no domain home in v1. They share a uniform implementation shape (parser → enum variant → resolver hook) and have disjoint write surfaces from the P0 chain.

**Required capabilities.**
- `<Training>` keyword (Rocks P-107, P-169) — alt-source mechanics.
- `<Digi-Burst N>` keyword — `[Main]` activation cost trashing N own digivolution cards.
- `<Decoy>` color-filter parameter wired into the replacement framework (depends on B).
- `<Memory Boost>` keyword variant (Rocks EX8-067, P-039/107/169).
- `<Progress>` keyword + `ImmunityToOpponentEffects` — already 🟢 in audit table; verify and demote in trackers.
- DigiXros name alias ("treated as [X] for DigiXros").
- Native printed keyword parsing for any remaining variants in `RUST_ENGINE_GAPS.md:304` (Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N, Fragment, Save, Collision, Retaliation) not already landed.
- Replacement-class keywords from Track B's window: `<Armor Purge>`, `<Decode>`, `<Barrier>`, `<Scapegoat>`, `<Fragment (N)>`, Partition-adjacent source play.

**DCGO reference patterns.**
- `CardEffectCommons/KeyWordEffects/` is the canonical keyword library — one self-contained file per keyword, sized 31–220 lines. Each keyword has a parser entry + an emitter class + a resolver hook. Mirror this 1:1; our `Keyword` enum variants and `KeyWordEffects/*.cs` filenames should match.
- Read in dependency order so cross-track contracts stay clean:
  1. `Training.cs` (31, smallest) — alt-source pattern, no replacement, no combat — proves the parser-enum-resolver scaffolding.
  2. `Reboot.cs` (43) — phase-window enforcement (Track D contract).
  3. `Collision.cs` (31) — granted-keyword pattern (`ModifierType::GrantCollision` from Track C).
  4. `Decoy.cs` (70) — color-filter parameter wired through replacement framework (Track B contract).
  5. `Fragment.cs`, `Barrier.cs`, `Scapegoat.cs`, `Decode.cs`, `ArmorPurge.cs`, `Partition.cs` — replacement-class (need Track B's leave-field hook live).
- Memory Boost has no dedicated `MemoryBoost.cs` in DCGO — it's resolved through `CardEffectFactory/ChangePlayCost.cs` and `ChangeDigivolutionCost.cs`. Don't add a new keyword class for it; resolve through the cost-modification factory matching DCGO.
- `Progress.cs` (110) is already 🟢 in our audit table. Diff against this file before demoting in trackers — verify our `ImmunityToOpponentEffects` modifier scope matches DCGO's, especially around inherited Progress and stack-depth.
- DigiXros name alias lives in `CardEffectCommons/DigiXrosEffects.cs` + `CardEffects/ChangeCardNamesForDigiXrosClass.cs`. Don't put it in the keyword library — it's a DigiXros-mechanic helper.
- `KeyWordEffects/Alliance.cs` (220, largest) is the most complex — read last and only when implementing Alliance directly.

**Representative first fixtures.**
- Rocks P-107: `<Training>` keyword as printed.
- A `<Digi-Burst 2>` card from the relevant set.
- Rocks BT16-082 (already partial) for Memory Boost variant.

**Acceptance gates.**
- Each keyword's parser/enum slot is reachable from YAML.
- Resolver hooks are tested at every printed text effect — no silent no-ops.
- Replacement-class keywords route through Track B's replacement framework.
- Color-filter parameters on `<Decoy>` correctly narrow the replacement candidate set.

### Track H — Aura System  [T2]

**Archetypes hit:** Royal Knights, DNA Omnimon, Medusamon, BG Imperial, Puppets, Zephagamon.

**Problem.** Declarative auras have specific infrastructure needs separate from per-card modifiers. The aura system is the bridge between Track C's modifier registry and printed "all your X gain Y" / "while Z, all opponent's W are immune to V" text.

**Required capabilities.**
- Named-target declarative aura (DP / keyword grants filtered by name/trait/level/color).
- Declarative aura sourced from security zone.
- Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`).
- Granted triggered ability — attach an `Effect` to another permanent (EX1-068). Distinct from K (cross-card refire) because granted effects persist.
- Grant `Security A. ±N` — parametric `SecurityAttackChange`.
- Digivolution-stack name overlay ("has all names of materials").
- Sourced-keyword stack-traversal (Medusamon `has_keyword` fix; DNA Omnimon).
- Conditional aura with state predicate (Zephagamon ZEPH-G004 conditional Vortex).

**DCGO reference patterns.**
- DCGO splits aura targets across three directories. Adopt the same shape so Track J's predicate scope matches DCGO's:
  - `CardEffectCommons/GiveEffect/GiveEffectToPermanent/` (18 files) — named-target aura applied to one or more permanents.
  - `CardEffectCommons/GiveEffect/GiveEffectToPlayer/` (14 files) — player-scoped aura.
  - `CardEffectCommons/GiveEffect/GiveEffectToPermanentOrPlayer.cs` — bilateral or either-target aura.
- `PermanentEffectFactory.cs` (137, small) is the binding shape — given an aura source and a target filter, attaches an `Effect` to each matching permanent. Read this before `ContinuousController.cs` (1843); the controller orchestrates re-evaluation but the factory holds the contract.
- `ContinuousController.cs` re-evaluation cycle: source mutation → controller invalidates affected auras → factory re-binds → modifiers re-attach. Acceptance gate "no stale modifier residue" maps directly to this cycle.
- `CardEffects/AddSkillClass.cs` is granted-triggered-ability (EX1-068, distinct from Track K refire). The granted effect persists on the receiving permanent until aura expiry; refire (Track K) fires once and is gone. Don't conflate.
- `CardEffects/ChangeCardNamesClass.cs`, `ChangeBaseCardNameClass.cs` — name overlay. The overlay attaches as a continuous modifier on the permanent; predicates checking `contains_card_name` walk both the overlay and the source stack.
- Sourced-keyword stack-traversal lives in `CardSource.cs` keyword-population code — search around `has_keyword` analogs. The Medusamon fix is to walk every source in the stack, not just the top card; DCGO already does this.
- Conditional aura (Zephagamon ZEPH-G004 `VortexCanAttackPlayer`) maps to `KeyWordEffects/Vortex.cs` + `CardEffectFactory/VortexCanAttackPlayers.cs` + `CardEffects/VortexCanAttackPlayersClass.cs`. The condition is evaluated at `ContinuousController` re-eval time — don't try to model "while opponent has no unsuspended Digimon" as a one-shot trigger.
- DP / cost / level auras: DCGO factors these into `CardEffectFactory/Change*.cs` files (`ChangeDP`, `ChangeCardDP`, `ChangeOriginDP`, `ChangeSAttack`, `ChangePlayCost`, `ChangeDigivolutionCost`, `ChangeLinkMax`). Each is sized 76–271 lines and worth diffing against our existing implementations to surface missing scaling rules (per-stack-depth, per-color, per-opponent-board).

**Representative first fixtures.**
- EX1-068: grant a triggered effect to opponent's permanent (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT).
- Zephagamon ZEPH-G004 Vortex aura active only while opponent has no unsuspended Digimon (`VortexCanAttackPlayer`).
- A digivolution-stack name overlay card so `contains_card_name` lookups walk the source stack.

**Acceptance gates.**
- Auras are recomputed on relevant board changes; no stale modifier residue.
- Granted triggered abilities fire on the receiving permanent and attribute correctly to the grantor.
- Name overlays are visible to predicates checking `contains_card_name` / `self_digivolution_contains_name`.
- Sourced-keyword traversal walks the entire source stack, not just the top card.

### Track I — Option, Delay, Plug-In, and Security Disposition  [T2]

**Archetypes hit:** Zephagamon, TS Olympos, DNA Omnimon, Rocks, Chaos Control, Red Hybrid, Royal Knights, Puppets.

**Problem.** Options can resolve then trash, place themselves in battle as Delay, link as Plug-Ins, activate from security, add themselves to hand, stay on field, trash as cost, or re-link from battle. These flows must preserve placement-turn Delay gating, event-gated Delay windows, Option trash observers, color/use requirements, and security disposition.

**Required capabilities.**
- Field Option lifecycle distinguishing Delay, Plug-In/link, orphaned Plug-In, ordinary field Option states.
- Option use from hand by effect with cost-ceiling formulas and color-requirement bypass where printed.
- Security activation disposition: add resolving Option to hand, place in battle, trash after resolution, play a Tamer/Digimon from hand/trash/security.
- Global `OnOptionTrashed` observer context for field Options and linked Options.
- Plug-In link/re-link source zones: hand, free field Option, linked card on a carrier.
- Delay activation as a visible main-phase or event-gated action with placement-turn restrictions.
- Event-gated Delay windows (Puppets G004; BT22-098, P-229).
- Trash-main option action (Chaos Control branch choice with cost upgrade).
- Option self-disposition operators (Rocks G-ROCKS-OPTION-SELF-DISPOSITION; ASL).

**DCGO reference patterns.**
- `OptionResolutionClass.cs` is the resolution-branching reference — distinguishes "resolve and trash" from "place on field" from "stay in security" based on the Option's printed disposition. Read before designing the Option lifecycle state machine.
- `OptionUtility.cs` carries Option-state queries (Delay vs. Plug-In vs. orphaned Plug-In vs. ordinary field Option). Adopt the state taxonomy directly — DCGO's mutually-exclusive states match what the spec lists.
- `MainPhaseAction/ActivateCardAction.cs` (Option from hand) and `ActivatePermanentAction.cs` (Delay from field) are the two visible main-phase action sites. Acceptance gate "Delay activation as a visible main-phase or event-gated action" maps to these two action shapes.
- `CardEffectCommons/CanUseEffects/WhenUseOption.cs`, `WhenLinked.cs`, `WhenWouldLink.cs` are Plug-In observers. Plug-In re-link source-zone (hand / free field Option / linked card on a carrier) is dispatched through these — the source zone is in the event payload from Track A.
- `KeyWordEffects/MindLink.cs` (82) is the Plug-In/MindLink mechanic reference. Plug-In state survives carrier loss because MindLink stores the linked card identity outside the carrier — adopt this storage shape so orphaned Plug-Ins remain valid selection candidates.
- `CardEffectCommons/TrashLinkedCards.cs` is the linked-Option trash flow. The `OnOptionTrashed` observer (Track A) fires per-card from this flow — Track I just registers subscribers, doesn't reimplement the trash.
- `SecurityEffect.cs` (under `CardEffectFactory/`) is the security-activation entry point. Security disposition variants (add to hand, place in battle, trash after resolve, play a Tamer/Digimon from hand/trash/security) all branch from here.
- DCGO doesn't have a dedicated `Delay.cs` — placement-turn gating lives inside `OptionEffect.cs` and `OptionResolutionClass.cs`. Search for "Delay" or "turn placed" in those files to find the gating logic.
- `CardEffects/AddLinkConditionClass.cs`, `ChangeLinkCostClass.cs`, `ChangeLinkMaxClass.cs` cover Plug-In cost / max-link modifiers. Same modifier-shape as Track C.

**Representative first fixtures.**
- EX11-072 Zephagamon Option: play from hand/trash, place itself in battle, later activate Delay on a Shoto suspend event.
- BT24-085 TS Olympos Tamer: use one TS Option from hand with use cost ≤ opponent memory, then open may-attack.
- ST22-11 Defense Plug-In F: link from hand or battle area into the specified Digimon.

**Acceptance gates.**
- Delay cannot activate the turn the Option was placed unless printed text explicitly permits it.
- Option self-disposition does not skip security/option trash observers.
- Option effects invoked by another card resolve through the same Option flow as ordinary use.
- Plug-In state survives carrier loss according to printed rules and can become a later selection candidate.

### Track J — DSL Predicate, Formula, and Modifier-Enforcement Plumbing  [T3]

**Archetypes hit:** Zephagamon, Puppets, TS Olympos, DNA Omnimon, BG Imperial, Alter-S Ladder, Medusamon, Red Hybrid, Rocks.

**Problem.** Several card blockers are no longer missing broad engine movement, but lack runtime predicates, result bindings, formulas, or enforcement checks. These should attach to concrete blocked-card fixtures, not be implemented as speculative vocabulary. J is parallel-friendly: predicate parsing/lowering can land schema-only behind a feature flag before evaluators wire through real subjects.

**Required capabilities.**
- Result-bound predicates: "if this effect suspended your Digimon", `any_returned_card`, `binding_present` / `binding_absent`.
- Formula-backed target counts and DP ceilings: suspended-count floor division (BT20-101), source-stack aggregate formulas, formula-valued `gain_memory` (EX1-021), formula DP cap for green Avian/Bird play (EX11-035).
- Source-relative predicates: `stack_size_lte_source` (BT12-028, BT16-025/027), `self_digivolution_contains_trait/_name` (EX1-014, BT12-028, BT16-027), `carrier_has_keyword` (BT3-002), rules-text-contains (BT22-017; Puppets G025).
- Event predicates: `event_target_is_source` / `event_target_not_source`, `is_dna_digivolving`, `is_effect_initiated`, deletion-cause-equals-Overclock, `attacker_trait_has` (BT21-025).
- Hidden-zone DP predicate for hand/trash free-play (Puppets G021).
- Modifier enforcement for narrow protection: opponent DP reduction, opponent De-Digivolve, source-kind immunity, timing suppression, `VortexCanAttackPlayer`, security-effect suppression, player-scoped security-placement blocks.
- Lowering hooks: `condition:` field on `AltPathSpec` (G-ALT-PATH-CONDITION); inverse `digivolve INTO X` direction (ST20-10); `active_when` consumed on `kind: grant_keyword` (BT12-022); `redirect_attack_target` step verb; `lose_count_bound` step (BT17-018); `return_all_trash_to_deck_bottom` step + player-choice target (BT17-077); `trash_top_n_digivolution_cards` step (BT12-028); `play_cost_lte` formula-valued variant (BT21-102); `opp_security_count_lte` (BT21-024); `has_on_deletion_effect` permanent predicate (EX1-021); `distinct_tamer_colors_on_field` BoolPredicate (ST20-10); `if-no-target` / `binding_is_none` (BT16-025); `count_gte` / `count_lte` general predicate (BT24-008, EX9-066); `event_target_is_source` (BT15-101); `aura` declarative target scoping (G-DSL-AURA-TARGET-SOURCE-PERMANENT, EX1-014); `entering_permanent_trait` gate (EX11-054); `text_contains` (BT22-017).

**DCGO reference patterns.**
- `GameContextDeterminarion.cs` (772, flat) is the predicate evaluator. Read end-to-end before designing the DSL evaluator — every event/source/result predicate the spec lists is here, and DCGO's predicate naming maps cleanly onto our DSL field names. When a DCGO predicate exists with the same semantics as a missing DSL predicate, port the implementation rather than redesign.
- DCGO predicates read directly off `GameContext.cs`'s named fields (Track A's payload). Schema-only landings behind feature flag fail when reached at runtime — but if the corresponding `GameContext` field is already published by Track A, the evaluator is a one-liner. Sequence Track J behind Track A's payload contract.
- `MinMax_DP_Cost_Level/{Cost,DP,Level,DigivolutionCards}/` are the formula evaluators feeding selection counts and DP ceilings. Suspended-count floor division (BT20-101) lives in the same family — add a `Suspended/` evaluator next to the existing four. Formula-valued `gain_memory` (EX1-021) reads from these evaluators.
- `IsDigivolvedByTheEffect.cs` (23) is the canonical implementation of `is_dna_digivolving` / `is_effect_initiated` analogs — small enough to cite as "do exactly this".
- `CheckEffectDisabledClass.cs` is the `active_when` consumer pattern. Track J's `active_when` lowering on `kind: grant_keyword` (BT12-022) reads the same disabled-state predicate at modifier-emit time.
- For modifier-enforcement narrow protection (opponent DP reduction, opponent De-Digivolve, source-kind immunity), the call sites live in `Permanent.cs` and `CardSource.cs`. Spot-read 2–3 `AutomaticOrder/Cannot*.cs` files (e.g. `CanNotBeDeletedByEffect.cs`, `CanNotReturnToHand.cs`) to confirm which mutation paths consult which modifiers. Acceptance gate "modifiers are enforced at every mutation path they claim to block" maps to a checklist diff against these consult sites.
- `CardEffectFactory/ChangeCardDP.cs` (76) is the hidden-zone DP overlay reference (Puppets G021 hand/trash free-play). The overlay attaches at reveal/select time and tears down on resolve.
- DCGO does not have a dedicated text-contains predicate (`text_contains` for BT22-017 / Puppets G025 rules-text-contains). Rules-text matching in DCGO happens at parse time; Track J's runtime text-contains is a small extension — implement against the parsed effect text, not the printed string.

**Representative first fixtures.**
- EX11-074 Zephagamon: branch only if this effect suspended your own Digimon, then grant DP/immunity.
- BT16-055 Namakemon: immune to opponent DP reduction and De-Digivolve while security count ≥ 3.
- BT20-101 Zephagamon: suspended-count divided by 2 drives count-capped bottom-deck selection.

**Acceptance gates.**
- Predicates evaluate at runtime against the intended subject, not parsed and ignored.
- Formula values feed selection counts, mutation amounts, and effect ceilings.
- Modifiers are enforced at every mutation path they claim to block.
- Broad `CannotBeAffected` is not used for narrow category-specific protection.
- Schema-only landings behind feature flag fail loudly when reached at runtime, never silently.

### Track K — Cross-Card Effect Re-Firing  [T3]

**Archetypes hit:** TS Olympos, Dark Masters, Apocalymon-style decks, Royal Knights.

**Problem.** Some effects choose another permanent and activate one of that card's `[On Play]` or `[When Digivolving]` effects outside its normal timing. This is neither a fake play nor a fake digivolution. It must enumerate eligible effects, expose a choice if more than one exists, preserve source attribution, and define once-per-turn interaction.

**Required capabilities.**
- Enumerate registered effects on a selected permanent by timing.
- Present a pending choice when multiple eligible effects exist.
- Re-run the chosen effect with explicit attribution to: (a) the card that caused the refire, (b) the card whose text is being activated.
- Preserve once-per-turn semantics unless printed text explicitly permits bypassing.

**DCGO reference patterns.**
- `MultipleSkills.cs` is the multi-effect skill enumeration on a permanent — the "enumerate registered effects by timing" requirement. Read first; it defines the per-permanent skill list shape.
- `OptionalSkill.cs` is the choice-of-effect dispatch when more than one eligible effect exists. Acceptance gate "the mask exposes both target choice and effect choice where applicable" maps directly to `OptionalSkill`'s pending-selection shape.
- `CardEffects/AddSkillClass.cs` grants a skill — distinct from refire (refire activates an existing registered skill rather than adding one). Use `AddSkillClass` as the contrast point so the implementer doesn't conflate the two paths.
- `CardEffects/ActivateClass.cs` + `MainPhaseAction/ActivateCardAction.cs` / `ActivatePermanentAction.cs` are the activation entry points. Refire re-enters at the same hook the natural activation uses, with two extra attribution flags: cause-card and effect-source-card. Both flags live on Track A's payload.
- Once-per-turn accounting in DCGO lives on `Permanent.cs` (per-permanent counters keyed by effect ID). Refire consults the same counter — bypass only when the printed source explicitly permits, gated by a `bypass_once_per_turn` flag set by the refire emitter.
- DCGO does not have a dedicated `Refire.cs` — search `Effects.cs` (2306) for cross-card activation calls. Likely site: methods that look up a target permanent's skill registry and invoke a chosen entry by ID with attribution overrides.

**Representative first fixture.**
- BT24-102 Homeros: choose an Olympos XII Digimon and activate one of its On Play or When Digivolving effects at end of turn after paying the printed suspend cost.

**Acceptance gates.**
- The target permanent is not treated as newly played or newly digivolved.
- Effects already consumed by once-per-turn limits do not refire unless printed source permits.
- The mask exposes both target choice and effect choice where applicable.

### Track L — Production YAML and Regression Gates  [T4]

**Archetypes hit:** all twelve.

**Problem.** Some reports list production YAML absence as a gap. That is a valid readiness blocker but not always an engine blocker. YAML authoring proceeds only after the primitive required by the omitted printed text exists and has its own card-shaped regression test.

**Required capabilities.**
- Structural tests rejecting load-only stubs for cards marked ready.
- Behavioral tests for every player-visible choice in a card's claimed implemented slices.
- Clear YAML comments for omitted blocked clauses, naming the exact gap ID.
- Tracker update discipline that demotes a reusable gap to card-local authoring once engine and DSL support land.
- Trait-filter helpers on `CardSource` / `Permanent` (`RUST_ENGINE_GAPS.md:510`) — pervasive ergonomics that most card-shaped tests depend on.

**DCGO reference patterns.** DCGO does not directly help here — there is no YAML in DCGO, and per-card scripts are implementation references, not test fixtures. Use DCGO only to confirm processing order on the card whose YAML is being promoted: read the matching script under `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs` (DCGO uses `_` not `-` in card-ID filenames per `feedback_csharp_naming.md`). Diff DCGO's resolution sequence against the YAML's `kind:` step order before promoting — divergences are usually our gap, not DCGO's bug.

**Representative first fixtures.**
- BT20-101 Zephagamon structural readiness test before full behavior.
- One Royal Knights high-frequency card currently carrying gap comments after its source-play primitive lands (Track E).
- One Puppets core card after Track A deletion context and Track B replacement dispatch are complete.

**Acceptance gates.**
- No production YAML claims full card readiness while printed text remains omitted.
- Ignored tests name the blocking gap and become active when the primitive lands.
- Raw-Rust helpers are retired or fenced to behavior that cannot yet be expressed; no no-op placeholders.

## Parallelism Matrix

Each track lists its primary write surface, upstream dependencies, and tracks it can run alongside.

| Track | Tier | Primary write surface | Depends on | Parallel with |
|---|---|---|---|---|
| **A** Event payloads & fan-out | T0 | `effect_queue.rs`, `enums.rs` (TriggerPayload), `permanent.rs` zone scanners | — | C, F, G subset, J schema-only |
| **B** Replacement framework | T0 | `effect.rs` replacement window, `game.rs` leave-field hooks | A (cause/source payloads), C (source-scoped immunity) | F, G subset, J schema-only |
| **C** Modifier registry foundation | T0 | `modifiers.rs`, new `ModifierType`, `Expiry` enum | — | A, F, G subset, J schema-only |
| **D** Combat interrupts | T1 | `combat.rs`, attack-state machine | A (`OnAttackTargetChange` payload), C (attack-restriction modifiers) | E, F, G, I |
| **E** Zone movement helpers | T1 | `effect_context.rs` zone helpers, `game.rs` security ops | A (`OnDigivolutionCardTrashed` extension), C (cleanup-provenance modifier flag) | D, F, G, I |
| **F** Selection primitives | T1 | `selection.rs`, `action/mask.rs` | — (mask infra exists) | A, B, C, D, E, G, I, J |
| **G** Keyword library | T2 | `card_data.rs` parser, `cards.rs`, `enums.rs` keyword variants | C (modifier-granting keywords); B (replacement-class keywords) | A, F, J — Training/Digi-Burst/Memory Boost fully independent |
| **H** Aura system | T2 | `effect.rs` aura builders, `tensor.rs` for DP, modifier delivery | C (modifier registry), B (`UntilLeaveField` with replacement) | F, J |
| **I** Option/Delay/Plug-In | T2 | `game.rs` Option lifecycle, `effect.rs` Delay scheduling | A (`OnOptionTrashed`), B (Delay-as-replacement), E (zone helpers) | G, J |
| **J** DSL predicate/formula plumbing | T3 | `code/digimon-dsl/` lowering, predicate evaluator | — (consumes A's payload at evaluation time; schema-only landings need nothing) | every other track |
| **K** Cross-card effect re-firing | T3 | `effect.rs` enumeration, `effect_context.rs` refire helper | A (attribution payloads), F (effect-choice selection) | H, I |
| **L** Production YAML gates | T4 | `code/digimon-engine/cards/`, behavioral tests | per-primitive: A–K | continuous, lags by one card-shaped regression |

## Implementation Waves

### Wave 1 — start day 1, no upstream blockers
- **A** Event payloads & fan-out (publish payload contract first).
- **C** Modifier registry foundation (publish `ModifierType` and `Expiry` taxonomy first).
- **F** Selection primitives — DP-budget multi-select, exact/up-to-N, ordered permutation, union-zone.
- **G subset** — Training, Digi-Burst, Memory Boost (parser/enum-only keywords).
- **J schema-only** — lower predicates that already have evaluator support; queue the rest behind a feature flag that fails loudly at evaluation.

### Wave 2 — starts as A and C publish their first slices
- **B** Replacement framework — needs A's cause payloads + C's `Expiry`.
- **D** Combat interrupts — needs A's `OnAttackTargetChange` + C's attack-restriction modifiers.
- **E** Zone helpers — needs A's `OnDigivolutionCardTrashed` extension.
- **H** Aura system — needs C; benefits from B for `UntilLeaveField`.
- **G remainder** — Decoy, replacement-class keyword fixtures (need B).

### Wave 3 — after Wave 2 has tested slices
- **I** Option / Delay / Plug-In — pulls from A, B, E.
- **K** Cross-card refire — pulls from A and F.
- **J full evaluation** — predicates land against real subjects.

### Wave 4 — continuous, lags every primitive by one card-shaped regression
- **L** YAML migration — each primitive's first card-shaped regression is the gate; archetype YAML promotion follows.

## Verification Matrix

| Track | Minimum targeted tests |
|---|---|
| A | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context`; card-shaped Puppets/Royal Knights event tests; `timing_dispatch` coverage of inherited-stack walk |
| B | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context`; one inherited replacement card test; one cross-permanent subject-guard test |
| C | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test modifiers`; one source-scoped immunity test; one `Expiry` lifecycle test |
| D | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- effect_granted_attack`; one DSL may-attack test; `<Retaliation>` / `<Reboot>` enforcement tests |
| E | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- zone_movement`; one source/material play card test; `move_from_breeding` coverage |
| F | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection`; one DP-budget or exact-N card test; `select_*` outer-tail empty-continuation test |
| G | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keywords`; one fixture per landed keyword |
| H | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test auras`; one named-target aura test; granted-triggered-ability test |
| I | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- option`; one field Option or security-disposition card test |
| J | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- predicates formula`; one modifier-enforcement card test |
| K | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_refire`; Homeros card-shaped test |
| L | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- <card_filter>` for each card promoted from blocked to ready |

## Tracker Policy

Every closure PR updates the tracker that originally named the gap:

- Engine primitives: `docs/RUST_ENGINE_GAPS.md`
- Legacy archetype-engine tracker entries: `qa/archetype-qa/engine-gaps.md`
- DSL vocabulary and lowering entries: `qa/dsl-vocab-gaps.md`
- Source implementation-run rollups: `qa/archetype-qa/dsl/*.md`
- API authoring details when public helpers are added: `docs/RUST_ENGINE_API.md` or `docs/RUST_DSL_TEST_API.md`

Updates mark entries as closed, partially closed, narrowed, or demoted to card-local authoring. They include the test command that proves the new status.

## Success Criteria

The engine closure wave is successful when:

- T0 tracks (A, B, C) each have at least one reusable primitive test and one card-shaped regression.
- No T0 or T1 gap requires hidden auto-selection or raw vector mutation.
- Implemented primitives are reachable from DSL/YAML or documented as engine-only helpers with a follow-up DSL gap.
- Action and tensor contracts remain unchanged.
- At least one high-frequency archetype (Royal Knights, DNA Omnimon, or Rocks) moves from blocked to mostly-card-authoring after tracker refresh.
- Every newly active card behavior has a targeted Rust test.
- Tracks J and L lag the implementation primitives but never lead them.

## Open Risks

- Some pending-selection shapes may exceed the practical capacity of current generic action IDs. If that happens, pause and create a contract-change spec rather than expanding action space ad hoc.
- Event fan-out can accidentally double-trigger inherited effects if zone scanners overlap. Tests must cover one observer reachable through exactly one path.
- Stable permanent handles may expose old index assumptions in existing code. Handle-based code coexists with current indices only where tests prove the index remains stable.
- Broad immunity/modifier checks are easy to over-apply. Category-specific protection must be tested against both blocked and allowed effect kinds.
- Production YAML migration can look like progress while leaving omitted printed choices. Readiness claims must remain tied to behavioral tests, not file count.
- Track J's schema-only feature flag must fail loudly when reached at runtime. Silent no-op evaluation re-creates the very class of bug this spec is trying to prevent.
- The aura system (Track H) and the modifier registry (Track C) share a mental model but have distinct write surfaces. Avoid landing aura logic that bypasses the registry — aura → modifier delivery is the contract.

## Spec Self-Review

- **Placeholder scan:** no TBD/TODO sections.
- **Scope check:** master design spec for engine closure domains; each track can become its own implementation plan.
- **Contract check:** explicitly forbids action/tensor expansion.
- **Ambiguity check:** each track names required capabilities, first fixtures, acceptance gates, parallelism, and verification command.
- **Coverage check:** every gap surfaced by the twelve completed implement runs maps to exactly one track. Gaps that previously had no home (Training, Digi-Burst, Decoy, Memory Boost, DigiXros alias, Retaliation, Reboot, Piercing security continuation, granted triggered ability, name overlay, sourced-keyword traversal, reveal-zone overlay, cast-time stack construction, `move_from_breeding`, stack-peel, search-own-security, `OnHatch`, `EndOfOpponentsTurn`, inherited-stack walk) are now assigned.
- **Parallelism check:** the matrix and waves identify three concurrent T0 tracks, three concurrent T1 tracks, and three concurrent T2 tracks. J is parallel-friendly across all tiers via schema-only landings; L is continuous.
