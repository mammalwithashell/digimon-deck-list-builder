# DCGO Resource Map for Engine Closure Tracks

**Date:** 2026-05-06
**Companion to:** `2026-05-06-engine-closure-domains-v2-design.md`
**DCGO submodule path:** `DCGO/` (initialized 2026-05-06)

DCGO is C# and Unity-flavored, but its rules engine has already solved most of what our Rust closure tracks need. This document maps DCGO files onto the twelve tracks so each implementer has a behavioral reference at hand. **DCGO is a tiebreaker, not the canonical source** — printed text and the Comprehensive Rules Manual still come first. Read DCGO when our printed-text and rules-manual reads converge but the *processing order* or *internal flow* is unclear.

All paths below are relative to `DCGO/Assets/Scripts/Script/`.

## Engine-wide anchors (read once, cited often)

| File | Lines | What it is |
|---|---|---|
| `GameContext.cs` | 186 | Per-effect blackboard. Carries `EventCard`, `EventPermanent`, source/target identity, attack target, etc. through effect resolution. Direct analog of our future `TriggerPayload`. |
| `GameContextDeterminarion.cs` | 772 | Predicate evaluator. Reads context fields and returns booleans the YAML uses. Maps to Track J. |
| `TurnStateMachine.cs` | 3497 | Phase model and turn flow. Contains every phase-granular timing (`StartOfYourMainPhase`, `EndOfAttack`, `EndOfBattle`, `EndOfOpponentsTurn`). |
| `Permanent.cs` | 4140 | Permanent state + zone scanners + observer dispatch on the carrier side. Provenance, former-controller, suspend cause, etc. live here. |
| `CardSource.cs` | 4323 | Source-stack model + inherited dispatch. Authoritative for `enqueue_from_permanent`-style stack walks. |
| `Effects.cs` | 2306 | Effect resolver loop. Where steps are pumped, where `EffectContext` reads/writes happen, where pending selections suspend resolution. |
| `ContinuousController.cs` | 1843 | Persistent/continuous-effect orchestrator. Re-evaluates auras and modifiers on board change. |
| `AttackProcess.cs` | 628 | Attack state machine. Interrupts, target-switch, Blocker, Counter, Piercing security continuation. |
| `UserSelectionManager.cs` | 161 | Pending-selection lifecycle. Maps to our `selection.rs` + action mask layer. |
| `PermanentEffectFactory.cs` | 137 | Binds aura/persistent effects onto a target permanent. |

---

## Track A — Event Context, Provenance, Observer Fan-Out

DCGO already has every observer timing we listed in the spec, in a single dictionary directory.

### Observer timing dictionary
`CardEffectCommons/CanUseEffects/` — one file per timing, very small (each class wraps a handler signature):

| DCGO file | Track A timing it maps to |
|---|---|
| `PermanentEnterField/OnPlay.cs` | `OnAnyDigimonPlayed` (entering-permanent payload) |
| `PermanentEnterField/WhenDigivolving.cs` | `OnDigivolve` + trait-filter |
| `PermanentEnterField/PermanentEnterField.cs` | base for entered-play observers (subject = entering permanent, not observer) |
| `OnDeletion.cs` | `OnAnyDeletion` (deleted-object snapshot lives here) |
| `OnAttack.cs` | `WhenAttacking` (attacker payload) |
| `OnAttackTargetSwitch.cs` | `OnAttackTargetChange` (Raid retarget; old/new target) |
| `OnEndAttack.cs` | `EndOfAttack` |
| `OnSuspend.cs` / `OnUnsuspend.cs` | `OnSuspend`, suspend-by-effect attribution |
| `OnMove.cs` | `[When Moving]` |
| `OnDeletion.cs` + `WhenDeleteOpponentDigimon.cs` / `WhenDeleteOpponentDigimonByBattle.cs` | own-side vs. opponent-side deletion + battle-cause discriminator |
| `OnTrashDigivolutionCard.cs` | `OnDigivolutionCardTrashed` (extension to non-`return_to_hand` paths is here) |
| `OnTrashHand.cs` | hand-resident trash observer fan-out |
| `OnTrashLinkCard.cs` / `OnTrashLinkedCard.cs` | linked-Option observer fan-out |
| `OnTrashBySelfDigiBurst.cs` | Digi-Burst path of `OnDigivolutionCardTrashed` |
| `OnAddDigivolutionCards.cs` | source-stack added observer |
| `OnCardsAddedToHand.cs` | hand-add fan-out |
| `OnCardsReturnToHandFromTrash.cs` / `OnCardsReturnToLibraryFromTrash.cs` | trash-resident observer fan-out |
| `OnReturnLibraryBottomDigivolutionCards.cs` | source → deck-bottom path |
| `OnFaceUpSecurityIncrease.cs` | `OnPlaceSecurity` / `OnAddedToSecurity` |
| `WhenLoseSecurity.cs` | `OnOwnSecurityRemoved` / `OnOpponentSecurityRemoved` |
| `WhendAddSecurity.cs` (sic) | security-add observer (mirror) |
| `WhenDiscardSecurity.cs` | `OnDiscardSecurity` |
| `WhenDiscardLibrary.cs` | deck-discard observer |
| `WhenAddHand.cs` | mirror of `OnCardsAddedToHand.cs` from the gaining side |
| `WhenLinked.cs` / `WhenWouldLink.cs` | Link/Plug-In observers |
| `WhenUseDigiBurst.cs` / `WhenUseOption.cs` | activation-window observers |
| `WhenWinBattle.cs` | combat-result observer |
| `WhenRemoveField.cs` | leave-field event (also feeds Track B) |
| `IgnoreBattle.cs` | distinguishes `battle:` effects from real attacks |

### Effect-initiated origin flag
- `CardEffectCommons/IsDigivolvedByTheEffect.cs` (23 lines) — small but exact reference for the `effect_initiated_origin` flag in our `TriggerPayload`.

### Observer fan-out & inherited stack walk
- `CardSource.cs` — search for inherited-effect dispatch around the methods that walk source stacks.
- `Permanent.cs` — search for OnDeletion / OnPlay handlers; this is where DCGO scans for handlers across zones.
- `ContinuousController.cs` — re-fires when zone state changes.

### Hatch and EndOfOpponentsTurn
- DCGO does not have a dedicated `OnHatch.cs` file under CanUseEffects, but `MainPhaseAction/` and breeding handling in `Permanent.cs` carry the equivalent — search `Permanent.cs` for hatch processing.
- `EndOfOpponentsTurn` lives in `TurnStateMachine.cs`'s phase loop.

### Reading order for Track A
1. `GameContext.cs` (small) — copy the field set.
2. `IsDigivolvedByTheEffect.cs` (tiny) — confirm the origin-flag shape.
3. `CanUseEffects/PermanentEnterField/PermanentEnterField.cs` + `OnPlay.cs` + `WhenDigivolving.cs` — see how DCGO distinguishes entering permanent from observer.
4. `CanUseEffects/OnDeletion.cs` + `WhenDeleteOpponentDigimon.cs` — deleted-object snapshot pattern.
5. `CanUseEffects/OnAttackTargetSwitch.cs` — old/new target carriage.
6. `CardSource.cs` (skim around inherited dispatch) — stack walk.
7. `TurnStateMachine.cs` (skim around `EndOf*` and `StartOf*` calls) — phase-granular timing dispatch.

---

## Track B — Replacement and Would-Leave Decisions

### Would-leave / replacement windows
- `CardEffectCommons/CanUseEffects/WhenPermanentWouldDigivolve.cs` — would-digivolve replacement window.
- `CardEffectCommons/CanUseEffects/WhenPermanentWouldPlay.cs` — would-play replacement window.
- `CardEffectCommons/CanUseEffects/WhenWouldLink.cs` — would-link replacement window.
- `CardEffectCommons/CanUseEffects/WhenRemoveField.cs` — leave-field event (the cancellable side of the replacement).

### Replacement-class keyword emitters
`CardEffectCommons/KeyWordEffects/`:
- `ArmorPurge.cs` (98) — leave-field replacement via source-trash cost.
- `Barrier.cs` (106) — battle-only leave-field replacement with security-trash cost.
- `Decode.cs` (113) — play-from-source on non-battle leave.
- `Decoy.cs` (70) — color-filtered defender swap as replacement.
- `Fragment.cs` (82) — N-source self-trash replacement.
- `Partition.cs` (178) — source-zone constraint enforcement.
- `Scapegoat.cs` (71) — delete-another-own-Digimon replacement cost.
- `Save.cs` (66) / `MaterialSave.cs` (121) — material redirect on leave.
- `Overclock.cs` (144) — Overclock cause discriminator (feeds Puppets G022 deletion-observer suspend-this-Tamer cost branch).

### Immunity / cannot-be-removed modifier inputs
`CardEffects/`:
- `CanNotBeDestroyedClass.cs`, `CanNotBeDestroyedByBattleClass.cs`, `CanNotBeDestroyedBySkillClass.cs`, `CanNotBeRemovedClass.cs`, `CannotReturnToHandClass.cs`, `CannotReturnToLibraryClass.cs`, `ImmuneFromDeDigivolveClass.cs`, `ImmuneFromStackTrashingClass.cs`, `ImmuneFromDPMinusClass.cs`.
- `CardEffectFactory/CanNotBeDeleted.cs`, `CanNotBeDeletedByBattle.cs`, `CanNotBeDeletedByEffect.cs`, `CanNotBeRemoved.cs`, `CanNotBeTrashedByEffect.cs`, `CanNotReturnToHand.cs`.

### Reading order for Track B
1. `WhenPermanentWouldDigivolve.cs` + `WhenPermanentWouldPlay.cs` + `WhenWouldLink.cs` — pre-move window pattern.
2. `WhenRemoveField.cs` — non-cancelling vs. cancelling response.
3. `KeyWordEffects/Barrier.cs` (most concise) → `ArmorPurge.cs` → `Decode.cs` → `Fragment.cs` → `Scapegoat.cs` — replacement emitter shape.
4. `KeyWordEffects/Overclock.cs` — cause-tagged deletion for Track A's deletion-cause discriminator.
5. `CardEffectFactory/CanNotBeDeleted.cs` family — immunity at the leave-field hook.

---

## Track C — Modifier Registry Foundation

DCGO has the most complete modifier inventory we can mine. Three layers:

### Permanent-attached one-shot modifier classes
`CardEffects/` (73 files) — each is a one-shot modifier emitter applied during effect resolution:
- Play / digivolve immunity: `CanNotPlayClass.cs`, `CanNotEvolveClass.cs`, `CanNotPutFieldClass.cs`, `CannotIgnoreDigivolutionConditionClass.cs`, `CannotReduceCostClass.cs`.
- Suspend / unsuspend: `CanNotSuspendClass.cs`, `CanNotUnsuspendClass.cs`, `CanSuspendByDigisorptionClass.cs`.
- Movement: `CanNotMoveClass.cs`, `CannotReturnToHandClass.cs`, `CannotReturnToLibraryClass.cs`.
- Combat: `CanAttackTargetDefendingPermanentClass.cs`, `CanNotAttackTargetDefendingPermanentClass.cs`, `CanNotSwitchAttackTargetClass.cs`, `CannotBlockClass.cs`, `BlockerClass.cs`, `CollisionClass.cs`, `RushClass.cs`, `RebootClass.cs`, `IcecladClass.cs`.
- Selection: `CanNotSelectBySkillClass.cs`, `CanSelectAssemblyClass.cs`, `CanSelectDigiXrosClass.cs`.
- Source-trash: `CanNotTrashFromDigivolutionCardsClass.cs`.
- Memory / security: `CannotAddMemoryClass.cs`, `CannotAddSecurityClass.cs`, `ChangeEndTurnMinMemoryClass.cs`.
- Affect-immunity: `CanNotAffectedClass.cs`, `ImmuneFromDPMinusClass.cs`, `ImmuneFromDeDigivolveClass.cs`, `ImmuneFromStackTrashingClass.cs`.
- Effect activation: `DisableEffectClass.cs` — direct reference for Track C's timing-suppression modifier (TS Olympos G-TS-TIMING-SUPPRESSION-MODIFIERS).
- Vortex: `VortexCanAttackPlayersClass.cs` (Zephagamon).
- Color: `IgnoreColorConditionClass.cs` (already done in our engine 2026-05-02 — verify behavior matches DCGO).

### Persistent modifiers (continuous)
`AutomaticOrder/` (37 files) — continuous-modifier counterparts of the above. Read these to understand `Expiry` and re-evaluation behavior:
- `CanNotAttack.cs`, `CanNotBeAttacked.cs`, `CanNotBeBlocked.cs`, `CanNotBeDeleted.cs`, `CanNotBeDeletedByBattle.cs`, `CanNotBeDeletedByEffect.cs`, `CanNotBeRemoved.cs`, `CanNotBeTrashedByEffect.cs`, `CanNotBlock.cs`, `CanNotDigivolve.cs`, `CanNotReturnToHand.cs`, `CanNoReturnToDeck.cs`, `CanNotSuspend.cs`, `CanNotUnsuspend.cs`.
- `ChangeCardDP.cs`, `ChangeDP.cs`, `ChangeDigivolutionCost.cs`, `ChangeLinkMax.cs`, `ChangeOriginDP.cs`, `ChangePlayCost.cs`, `ChangeSAttack.cs` — DP/cost scaling modifiers.
- `ImmuneFromDPMinus.cs`, `TreatAsDigimon.cs`, `VortexCanAttackPlayers.cs`.
- `AddDigivolutionRequirement.cs`, `AddLinkRequirement.cs`, `AddAppfusionMethod.cs`.

### Continuous orchestration
- `ContinuousController.cs` (1843) — re-evaluator. Read this for `Expiry` lifecycle (`UntilLeaveField`, `UntilEndOfTurn`, `UntilCondition`).

### Reading order for Track C
1. Sample 3–4 small `CardEffects/*Class.cs` files (e.g. `CanNotSuspendClass.cs`, `DisableEffectClass.cs`, `ImmuneFromDPMinusClass.cs`) — modifier-emit shape.
2. Matching files in `AutomaticOrder/` — continuous-modifier shape.
3. `ContinuousController.cs` — orchestration and `Expiry`.

---

## Track D — Effect-Created Attacks and Combat Interrupts

### Attack state machine
- `AttackProcess.cs` (628) — primary reference. Read end-to-end before designing the effect-attack helper.
- `SelectAttackEffect.cs` — attacker/target selection during effect.
- `MainPhaseAction/AttackPermanentAction.cs` — main-phase attack initiation (action-mask analog).

### Combat keywords
`CardEffectCommons/KeyWordEffects/`:
- `Pierce.cs` (85) — `<Piercing>` security continuation.
- `Reboot.cs` (43) — Reboot enforcement during opponent unsuspend.
- `Retaliation.cs` (149) — `<Retaliation>` combat enforcement.
- `Rush.cs` (86) — Rush attack permission.
- `Blitz.cs` (93) — Blitz timing.
- `Raid.cs` (122) — Raid retarget.
- `Blocker.cs` (86) — Blocker interrupt.
- `Collision.cs` (31) — granted Collision.
- `Alliance.cs` (220) — alliance attack-side interrupt.
- `Jamming.cs` (43), `Vortex.cs` (117), `Evade.cs` (84), `Execute.cs` (139), `Fortitude.cs` (98), `Iceclad.cs` (86).

### Attack-restriction / target modifiers
- `CardEffects/CanAttackTargetDefendingPermanentClass.cs`, `CanNotAttackTargetDefendingPermanentClass.cs`, `CanNotSwitchAttackTargetClass.cs`.
- `CardEffectFactory/CanNotAttack.cs`, `CanNotBeAttacked.cs`, `CanNotBeBlocked.cs`, `CanNotBlock.cs`.

### Counter Blast
- `SelectBurstDigivolutionEffect.cs` — Counter Blast `[Hand][Counter]` activation flow (DNA Omnimon G-DNAOmni-03, Zephagamon ZEPH-G008).

### Reading order for Track D
1. `AttackProcess.cs` (full read; this is the definitive reference).
2. `KeyWordEffects/Raid.cs` + `Blocker.cs` + `Collision.cs` — interrupt patterns.
3. `KeyWordEffects/Pierce.cs` — security continuation.
4. `KeyWordEffects/Retaliation.cs` + `Reboot.cs` — non-redirect enforcement.
5. `SelectBurstDigivolutionEffect.cs` — Counter Blast.

---

## Track E — Zone Movement and Source/Material Operations

### Source-stack ops
- `CardSource.cs` (4323) — zone identity + source-stack ops + `OnDigivolutionCardTrashed` dispatch.
- `CardEffectCommons/TrashDigivolutionCards.cs` — stack-peel.
- `CardEffectCommons/TrashLinkedCards.cs` — linked-Option trash.

### Reveal / library
- `CardEffectCommons/RevealLibrary.cs` — reveal-zone overlay.

### Security
- `SecurityObject.cs`, `SecurityBreakGlass.cs` — security stack model.

### Effect-driven play / digivolve
- `Effects.cs` (2306) — effect resolver including effect-driven plays.
- `IsDigivolvedByTheEffect.cs` — distinguishes effect-driven from natural digivolve (provenance).
- `MainPhaseAction/PlayCardAction.cs` — natural play action analog.
- `Draggable_HandCard.cs` — UI hand interaction (less useful, but documents which paths must remain on the natural play side).

### Cast-time stack construction
- `SelectAssemblyClass.cs` + `CanSelectAssemblyClass.cs` — printed "place N cards under" mechanics.

### Reading order for Track E
1. `CardSource.cs` zone-move methods (skim around the source-stack mutators).
2. `RevealLibrary.cs` — reveal-zone overlay.
3. `IsDigivolvedByTheEffect.cs` — provenance flag.
4. `SelectAssemblyClass.cs` — cast-time stack construction.
5. `MainPhaseAction/PlayCardAction.cs` — natural play (so effect-driven play can stay distinct).

---

## Track F — Selection Primitives and Action-Mask Surfaces

### Pending-selection lifecycle
- `UserSelectionManager.cs` (161) — small, complete. Read first.
- `SelectCardPanel.cs`, `SelectCommand.cs`, `SelectCommandPanel.cs` — UI side; helps understand "what choice is being asked".

### Selection effect classes (one per shape)
- `SelectCardEffect.cs` — generic card selection.
- `SelectCountEffect.cs` — count-based selection (closest to our exact-N / up-to-N).
- `SelectHandEffect.cs` — hand-specific selection.
- `SelectPermanentEffect.cs` — permanent selection (Track F's filtered breeding-permanent need).
- `SelectAttackEffect.cs` — attacker/target.
- `SelectAssemblyClass.cs` — assembly (cast-time stack picks).
- `SelectDigiXrosClass.cs` — DigiXros pair/multi-source.
- `SelectAppFusionEffect.cs` — App Fusion pair.
- `SelectBurstDigivolutionEffect.cs` — Counter Blast pair.
- `SelectJogressEffect.cs` — DNA pair.
- `SelectDNACondition.cs` — DNA-pair condition predicate.

### Aggregate / formula constraints
- `CardEffectCommons/MinMax_DP_Cost_Level/{Cost,DP,Level,DigivolutionCards}/` — aggregate constraint shapes (DP-budget multi-select, exact level totals, source-count formulas).
- `Combinations.cs` — combinatorial enumeration.

### Reading order for Track F
1. `UserSelectionManager.cs` (small, foundational).
2. `SelectCountEffect.cs` (closest to our generic shape).
3. `SelectAssemblyClass.cs` — multi-pick under-card placement.
4. `SelectJogressEffect.cs` + `SelectDigiXrosClass.cs` — pair/multi-source selection.
5. `MinMax_DP_Cost_Level/DP/*` — DP-budget aggregate.

---

## Track G — Keyword Library

DCGO has every keyword our spec lists, in `CardEffectCommons/KeyWordEffects/`:

| DCGO file | Track G keyword | Lines |
|---|---|---|
| `Training.cs` | `<Training>` | 31 |
| `Fragment.cs` | `<Fragment (N)>` | 82 |
| `Decoy.cs` | `<Decoy>` w/ color-filter param | 70 |
| `Decode.cs` | `<Decode>` | 113 |
| `Barrier.cs` | `<Barrier>` | 106 |
| `Scapegoat.cs` | `<Scapegoat>` | 71 |
| `Partition.cs` | Partition source enforcement | 178 |
| `ArmorPurge.cs` | `<Armor Purge>` | 98 |
| `Save.cs` / `MaterialSave.cs` | `<Save>` | 66 / 121 |
| `Overclock.cs` | Overclock cause discriminator | 144 |
| `Pierce.cs` | `<Piercing>` | 85 |
| `Reboot.cs` | `<Reboot>` | 43 |
| `Retaliation.cs` | `<Retaliation>` | 149 |
| `Rush.cs` | `<Rush>` | 86 |
| `Blitz.cs` | `<Blitz>` | 93 |
| `Raid.cs` | `<Raid>` | 122 |
| `Blocker.cs` | `<Blocker>` | 86 |
| `Collision.cs` | `<Collision>` (granted) | 31 |
| `Alliance.cs` | `<Alliance>` | 220 |
| `Jamming.cs` | `<Jamming>` | 43 |
| `Vortex.cs` | `<Vortex>` | 117 |
| `Progress.cs` | `<Progress>` (verify our impl matches) | 110 |
| `Evade.cs` | `<Evade>` | 84 |
| `Execute.cs` | `<Execute>` | 139 |
| `Fortitude.cs` | `<Fortitude>` | 98 |
| `Iceclad.cs` | `<Iceclad>` | 86 |
| `MindLink.cs` | Plug-In/MindLink mechanics | 82 |
| `Ascension.cs` | Ascension | 96 |

DigiXros name alias lives in `CardEffectCommons/DigiXrosEffects.cs` and `CardEffects/ChangeCardNamesForDigiXrosClass.cs`.

Memory Boost keyword: search `Effects.cs` and `CardEffectFactory/` — DCGO does not have a dedicated `MemoryBoost.cs`; it's resolved through cost-modification factories.

### Reading order for Track G
Read keywords in order of dependency: replacement-class first (block on Track B contracts):
1. `Training.cs` (31, smallest) — alt-source pattern.
2. `Reboot.cs` (43) — phase-window enforcement.
3. `Collision.cs` (31) — granted-keyword pattern.
4. `Decoy.cs` (70) — color-filter parameter.
5. `Pierce.cs`, `Retaliation.cs` — combat enforcement.
6. `Fragment.cs`, `Barrier.cs`, `Scapegoat.cs`, `Decode.cs`, `ArmorPurge.cs`, `Partition.cs` — replacement-class.

---

## Track H — Aura System

### Aura emitters by target
- `CardEffectCommons/GiveEffect/GiveEffectToPermanent/` (18 files, 45–230 lines each) — named-target aura applied to one permanent.
- `CardEffectCommons/GiveEffect/GiveEffectToPlayer/` (14 files) — player-scoped aura.
- `CardEffectCommons/GiveEffect/GiveEffectToPermanentOrPlayer.cs` — bilateral/either-target aura.
- `PermanentEffectFactory.cs` (137) — binds aura → permanent.
- `ContinuousController.cs` (1843) — re-evaluates aura on board change.

### Granted triggered abilities
- `CardEffects/AddSkillClass.cs` — grants a triggered effect to another permanent (EX1-068 / G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT).
- `CardEffects/AddDetailClass.cs` — grants printed detail/keyword.

### Name overlays
- `CardEffects/ChangeCardNamesClass.cs`, `ChangeBaseCardNameClass.cs` — name overlay mechanics (digivolution-stack name overlay analog).
- `CardEffects/ChangeCardNamesForDigiXrosClass.cs` — DigiXros name alias.

### Sourced-keyword stack traversal
- `CardSource.cs` — search for keyword-population code that walks the source stack (Medusamon `has_keyword` fix).

### Conditional aura
- `KeyWordEffects/Vortex.cs` + `CardEffectFactory/VortexCanAttackPlayers.cs` + `CardEffects/VortexCanAttackPlayersClass.cs` — Zephagamon ZEPH-G004 reference.

### DP/level/cost auras
- `CardEffectFactory/ChangeDP.cs` (146), `ChangeCardDP.cs` (76), `ChangeOriginDP.cs` (90), `ChangeSAttack.cs` (271), `ChangePlayCost.cs` (114), `ChangeDigivolutionCost.cs` (137), `ChangeLinkMax.cs` (142).
- `CardEffects/ChangeBaseDPClass.cs`, `ChangeCardDPClass.cs`, `ChangeDPClass.cs`, `ChangeDPDeleteEffectMaxDPClass.cs`, `ChangePermanentLevelClass.cs`, `ChangeTraitsClass.cs`.

### Reading order for Track H
1. `PermanentEffectFactory.cs` (small) — binding shape.
2. `GiveEffect/GiveEffectToPermanent/ChangeDP.cs` — concrete aura shape.
3. `GiveEffect/GiveEffectToPlayer/CanNotSuspend.cs` — player-scoped aura shape.
4. `GiveEffectToPermanentOrPlayer.cs` — bilateral aura.
5. `AddSkillClass.cs` — granted triggered ability.
6. `ContinuousController.cs` (skim re-eval cycles).

---

## Track I — Option, Delay, Plug-In, Security Disposition

### Option lifecycle
- `OptionEffect.cs` — Option resolution flow.
- `OptionUtility.cs` — Option-state queries.
- `OptionResolutionClass.cs` — Option resolve + trash vs. place-on-field.
- `OptionalSkill.cs` — optional-effect dispatch.
- `OptionPanel.cs` — UI side.
- `MainPhaseAction/ActivateCardAction.cs` — Option main-phase activation as visible action.

### Linked / Plug-In
- `CardEffectCommons/CanUseEffects/WhenUseOption.cs`, `WhenLinked.cs`, `WhenWouldLink.cs`.
- `CardEffectCommons/TrashLinkedCards.cs`.
- `CardEffects/AddLinkConditionClass.cs`, `ChangeLinkCostClass.cs`, `ChangeLinkMaxClass.cs`.
- `CardEffectFactory/AddLinkRequirement.cs`, `ChangeLinkMax.cs`.
- `KeyWordEffects/MindLink.cs` — Plug-In/MindLink mechanics.

### Security disposition
- `SecurityEffect.cs` (under CardEffectFactory) — security activation flow.
- `SecurityBreakGlass.cs`, `SecurityObject.cs`.
- `CanUseEffects/WhenLoseSecurity.cs`, `WhenDiscardSecurity.cs`, `WhendAddSecurity.cs` — security observer hooks.

### Delay (search-only — DCGO doesn't have a single `Delay.cs`)
- Search `Effects.cs` and `OptionEffect.cs` for `Delay`-related keywords; the placement-turn gating logic is inside Option lifecycle.
- `MainPhaseAction/ActivatePermanentAction.cs` — Delay activation as main-phase visible action.

### Reading order for Track I
1. `OptionResolutionClass.cs` — resolution branching.
2. `OptionUtility.cs` — state queries.
3. `MainPhaseAction/ActivateCardAction.cs` + `ActivatePermanentAction.cs` — visible main-phase actions.
4. `KeyWordEffects/MindLink.cs` — Plug-In linkage.
5. `SecurityEffect.cs` (search by name in `CardEffectFactory/`) — security flow.
6. `WhenLinked.cs` + `WhenWouldLink.cs` — Plug-In events.

---

## Track J — DSL Predicate, Formula, Modifier-Enforcement Plumbing

### Predicate evaluator
- `GameContextDeterminarion.cs` (772) — primary reference. This is the file Track J implementers read first. Every event/source/result predicate is here.

### Formula and aggregate
- `MinMax_DP_Cost_Level/{Cost,DP,Level,DigivolutionCards}/` — formula evaluators feeding selection counts and DP ceilings.
- `Combinations.cs` — combinatorial enumeration.
- `IsDigivolvedByTheEffect.cs` — `is_dna_digivolving` / `is_effect_initiated` analog.

### Modifier-enforcement check sites
- `CheckEffectDisabledClass.cs` — disabled-effect predicate (DSL `active_when` consumer).
- `Permanent.cs` + `CardSource.cs` — search for the call sites that consult modifiers (e.g. `CanBeReturnedToHand`, `CanSuspend`). Confirms which mutation paths must check which modifiers.

### Hidden-zone DP
- `CardEffectFactory/ChangeCardDP.cs` (76) — printed DP overlay while in hidden zones (Puppets G021).

### Reading order for Track J
1. `GameContextDeterminarion.cs` end-to-end (long but flat).
2. `MinMax_DP_Cost_Level/DP/*.cs` — formula-valued constraints.
3. `IsDigivolvedByTheEffect.cs` (23 lines) — origin predicate.
4. `CheckEffectDisabledClass.cs` — `active_when` consumer pattern.
5. Spot-read 2–3 `AutomaticOrder/Cannot*.cs` — modifier-enforcement call sites.

---

## Track K — Cross-Card Effect Re-Firing

### Skill enumeration
- `MultipleSkills.cs` — multi-effect skill enumeration on a permanent.
- `OptionalSkill.cs` — optional-effect dispatch (helps with the "expose a choice if more than one" requirement).
- `SkillInfo.cs` — skill metadata.
- `CardEffects/AddSkillClass.cs` — grants a skill (compare with refire — refire activates an existing skill rather than granting one).

### Activation hook
- `CardEffects/ActivateClass.cs` — activation-class shape.
- `MainPhaseAction/ActivateCardAction.cs` / `ActivatePermanentAction.cs` — main-phase activation.
- `Effects.cs` — search for re-firing helpers; DCGO sometimes expresses "re-fire X's [On Play]" by re-running the same effect dispatch with an attribution flag.

### Once-per-turn semantics
- Search `Effects.cs` and `Permanent.cs` for `once-per-turn` accounting; the refire path must respect it unless printed text bypasses.

### Reading order for Track K
1. `MultipleSkills.cs` — multi-effect enumeration.
2. `OptionalSkill.cs` — choice-of-effect dispatch.
3. `Effects.cs` (search around skill activation by ID/name).
4. `ActivateClass.cs` + `MainPhaseAction/ActivateCardAction.cs` — activation entry points.

---

## Track L — Production YAML and Regression Gates

DCGO does not directly help here — there is no YAML in DCGO, and DCGO's per-card scripts are implementation references, not test fixtures. **Use DCGO only to confirm processing order on the card whose YAML is being promoted.** The actual YAML and behavioral test work is on the Rust side: `code/digimon-engine/cards/` and `code/digimon-engine/tests/cards_behavioral/`.

For each card promoted from blocked → ready, also read the matching card's DCGO script under `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs` (DCGO uses `_` not `-` in card-ID filenames per `feedback_csharp_naming.md`).

---

## How to use this map

When an agent picks up a track:

1. Read the track's "Reading order" list — ordered by ramp-up cost.
2. Compare DCGO's behavior against the Comprehensive Rules Manual (`docs/RULES_CONTEXT.md`) and printed text. **Where DCGO and printed text disagree, printed text wins.** DCGO is a tiebreaker for processing order, not for what a card should do (CLAUDE.md "Source priority for card / keyword / rules questions").
3. Where DCGO's scope is broader than what we need (e.g. DigiXros across multiple cards), grab only the slice that maps to the spec's required capability.
4. Note interactions DCGO models that we don't yet (e.g. App Fusion if we don't have any App Fusion cards in scope) — these are out of scope and should not creep into the track's slice.
5. Don't transliterate. DCGO is C# + Unity coroutines; Rust will use sync resolution + pending-selection state machines. The reference is for *what happens*, not *how it's spelled*.
