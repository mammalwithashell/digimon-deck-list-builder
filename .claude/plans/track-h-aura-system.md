# Track H: Aura System

You are building the aura subsystem in the Rust game engine at `code/digimon-engine/`. The engine already has single-target grants (`EffectContext::grant_keyword(target, keyword, expiry)`), whole-card self-grants (`dsl_cards/lower_grant_keyword.rs`), Track C's modifier registry with named-target identity overlays (`ChangeTraits`, `ChangeBaseCardName`, `TreatAsDigimon`, etc.), and the UntilCondition continuous controller (PR #458). What's missing is the declarative-aura layer that turns printed text like "all your Holy Digimon get +1000 DP" or "while you have a [Mineral] in play, all your Digimon get Rush" into modifier emissions covering a filtered set of targets.

Without this, archetype migrations need `raw_rust` carve-outs for every printed aura. Several archetypes (Royal Knights, DNA Omnimon, Medusamon, BG Imperial, Zephagamon, Puppets) have aura-heavy card pools.

## Why this matters

* The single-target grant primitive already works, but real cards rarely target a single permanent — printed text almost always targets "all your X" or "all opponent's Y" filtered by name/trait/level/color.
* Track L (production YAML migration) is the next wave. Auras are the largest source of remaining `raw_rust` carve-outs in the archetype backlogs.
* Track C's modifier registry, Track A's event payload, Track B's leave-field hook, Track E's zone-movement helpers, and Track I's option lifecycle (in progress) are stable contracts now. Auras layer cleanly on top — they're modifier emissions, not new primitives.
* The UntilCondition controller landed (PR #458). Conditional auras ("while X is true, all your Y gain Z") are unblocked at the foundation level — H just needs to wire emission and target-filtering.
* Some auras need a cross-side delivery semantic ("opponent's Digimon are immune to X") that today requires authoring two separate single-target grants per matching permanent. Aura emission with a player-scoped target filter handles this in one declaration.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules 17–22 (no-approximations, TDD via DebugRunner, parity tracker check). Source priority: printed text + Comprehensive Rules Manual + fandom wiki come before DCGO.
2. `docs/RUST_ENGINE_API.md` — current `EffectContext` / `Effect` builder / `CardEffect` trait. Note the existing `grant_keyword(target, keyword, expiry)` and the `granted_keyword` field on `Effect`. The aura system extends this — it does not replace it.
3. `code/digimon-engine/src/` surfaces:
   * `effect.rs` (around line 198 `granted_keyword` field, line 583 default, line 808 `granted_keyword` builder method, line 813 `overclock_with_cost_filter`) — `Effect` struct + builder. Aura emission adds builder methods.
   * `effect_context/mod.rs` (around line 3614 `grant_keyword`) — single-target grant. Aura emission adds the filtered-set variants.
   * `dsl_cards/lower_grant_keyword.rs` — the existing whole-card self-grant lowerer. Aura DSL extends this with target-filter parameters.
   * `dsl_cards/predicate.rs` — the existing predicate evaluator. Aura target filters reuse this.
   * `dsl_cards/modifier_map.rs` — keyword-name → enum lookup.
   * `modifiers.rs` — `ModifierRegistry::install`. Aura emission produces modifier installs; do not add a parallel store.
   * `permanent.rs`, `card_source.rs` — identity reads route through `synth_identity()` (Track C). Aura emission may need to refresh consumer caches when an aura adds/removes a permanent's match status.
   * `game.rs` (around line 1917, 2340 — keyword grant consult sites; line 2955 — granted-Progress install pattern) — current consult sites. Confirm aura-emitted modifiers are read identically.
4. `code/digimon-engine/tests/` patterns:
   * `tests/combat/until_condition_controller.rs` — UntilCondition test pattern (PR #458). Conditional auras land tests in this style.
   * `tests/keyword_phase_f/progress.rs` — Progress aura test pattern (PR #457). Inherited / granted / native paths covered here are the template for auras coming from inherited sources.
   * `tests/combat/track_c_deferred_modifiers.rs` — modifier-registry test pattern. Aura emission is a modifier install; tests share the shape.
5. `docs/RULES_CONTEXT.md` — §11 timing rules (continuous effects), §16 keyword section (Vortex, Progress, Decoy interaction with auras). Specifically read the rules for "all your X" aura semantics and how they compose with deletion / control-transfer.
6. DCGO C# reference for processing order, target-filter shape, and consult-site placement only — printed text wins on disagreements:
   * The DCGO aura model is bucket-by-duration effect lists, not a separate aura registry. Every `Permanent` and `Player` carries lists keyed by duration: `UntilOpponentTurnEndEffects`, `UntilOwnerTurnEndEffects`, `UntilEachTurnEndEffects`, `UntilEndAttackEffects`, `UntilNextUntapEffects` (Permanent), and the player mirror lists. Each entry is a `Func<EffectTiming, ICardEffect>` callback. Read sites walk the list and invoke each callback for the relevant timing.
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/GiveEffect/GiveEffectToPermanentOrPlayer.cs` (91 lines) — the routing helper. `AddEffectToPermanent` and `AddEffectToPlayer` route into the right duration bucket. Note the duration-flip rule: `EffectDuration.UntilOpponentTurnEnd` lands in `UntilOwnerTurnEndEffects` when the source is the opponent's card, because "until opponent's turn end" relative to the source player is "until owner's turn end" relative to the receiving permanent. Mirror this rule in the Rust API or the cross-side aura semantics will silently flip.
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/GiveEffect/GiveEffectToPermanent/` (18 files, sized 45–230 lines). Single-target aura emitters. Read in this order:
      * `ChangeDP.cs` (the simplest; 56 lines) — single-target DP aura, shows `CanUseCondition` gate and `CanNotBeAffected` consult.
      * `ChangeOriginDP.cs` (52 lines) — printed-DP variant; adopt the base/origin distinction Track C published in `ModifierPayload::Dp { base, origin }`.
      * `StartOfMainAttack.cs` (78 lines) — the granted-triggered-ability shape. Adds an `ActivateClass` to a target permanent's effect list with `EffectTiming.OnStartMainPhase` filter. This is the closest reference for the granted-triggered-ability work in §3 below.
      * `CanNotSuspend.cs` (71 lines) — single-target keyword-equivalent grant via a CanNot* class.
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/GiveEffect/GiveEffectToPlayer/` (14 files). Filter-aura emitters — `Player`-scoped with a `Func<Permanent, bool> permanentCondition` that the consult site walks every matching permanent against. Read in this order:
      * `ChangeDP.cs` (71 lines, the longest because it does both the install and a flash-buff/debuff visual loop) — filter-aura "all your matching Digimon gain N DP". The filter callback runs per-read, not per-install. Adopt this lazy-evaluation shape: the aura installs a player-scoped modifier carrying a target filter; consult sites consult the modifier and re-evaluate the filter against each candidate.
      * `CanNotSuspend.cs` — filter-aura "your matching Digimon can't be suspended". `isOnlyActivePhase` flag narrows to a phase window. Adopt the active-phase narrowing in your aura builder.
      * `CanNotBeDeletedByBattle.cs` — filter-aura with `Func<Permanent, Permanent, Permanent, CardSource, bool> canNotBeDestroyedByBattleCondition` — a complex predicate signature that takes attacker, defender, blocker, source. The Rust equivalent uses Track A's payload to surface this context.
      * `IgnoreDigivolutionRequirement.cs` — filter-aura that turns a player-scoped flag into a per-permanent gate.
   * `DCGO/Assets/Scripts/Script/PermanentEffectFactory.cs` (137 lines) — reusable per-permanent effect builders (`DigimonEffectImmunity`, `CanNotSwitchAttackTargetEffect`, `CollisionEffect`, `AddDetailClass`). These are the single-target, hand-rolled flavor of the aura system. Mirror as ergonomic wrappers around the generic aura emission.
   * `DCGO/Assets/Scripts/Script/CardEffects/AddSkillClass.cs` (36 lines) — granted-triggered-ability primitive. `Func<CardSource, bool> _cardSourceCondition` filters which cards the skill is granted to; `Func<CardSource, List<ICardEffect>, EffectTiming, List<ICardEffect>> _getEffects` returns the granted effects per timing. `EffectTiming? _limitedTiming` is an optional timing gate. Adopt this exact shape for §3.
   * `DCGO/Assets/Scripts/Script/CardEffects/ChangeBaseDPClass.cs` and `ChangeDPClass.cs` — the receiving-side modifier classes. Track C's `ChangePayload::Dp { value, base, origin }` is the Rust analog; aura emission produces these payloads.
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Vortex.cs` (117 lines) — conditional aura reference. The keyword conditionally grants `VortexCanAttackPlayers` based on board state. Use the UntilCondition controller (PR #458) for the condition gate, not a per-tick re-evaluation.
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/PermanentEnterField/PermanentEnterField.cs` and `OnPlay.cs` — observer firing on aura-induced state changes. When an aura's filter starts/stops matching a permanent (e.g. an aura grants Rush to all Holy Digimon, and a non-Holy Digimon's traits change), consult sites re-read; no separate "aura-membership-change" event is needed.
7. Cross-archetype gap reports — skim, do not exhaustively read; they list the cards that need each capability:
   * `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md` (named-target DP/keyword auras, granted triggered abilities)
   * `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` (filter auras over Omni-named or Holy-trait Digimon, conditional auras gated on tamer count)
   * `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md` (sourced-keyword stack-traversal already covered by Track G; verify; aura over Reptile-traited Digimon)
   * `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md` (DNA-aura cross-side delivery, opponent-side immunities)
   * `qa/archetype-qa/dsl/zephagamon-2026-05-03-dsl-engine-gaps.md` (ZEPH-G004 conditional Vortex aura — `VortexCanAttackPlayer` while opponent has no unsuspended Digimon; the canonical UntilCondition aura fixture)
   * `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` (G008 inherited opponent-security Digimon DP aura DSL bridge)
   * `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md` (Security A. ±N grants, named-target aura over Olympos XII members)
   * `qa/archetype-qa/dsl/red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md` (security-sourced aura)
   * `qa/archetype-qa/dsl/chaos-control.md` (Partition-adjacent aura, granted triggered abilities for Apocalymon-style cards)

## Work to be done

### 1. `Aura` builder on `Effect`

Extend `Effect` / `EffectBuilder` (`code/digimon-engine/src/effect.rs`) with an aura-emission shape. Reuse the existing `granted_keyword` field where the aura grants a keyword; add new fields for DP / cost / immunity auras. Builder API:

```rust
Effect::declarative(card)
    .name("All your Holy Digimon gain +1000 DP")
    .aura()
        .scope(AuraScope::Player(controller))      // or AuraScope::OpponentPlayer / Bilateral
        .target_filter(filter_predicate)            // CompiledPredicate from DSL
        .grants(AuraGrant::Dp { value: 1000, base: false, origin: false })
        .duration(Expiry::EndOfYourTurn)
    .build()
```

Auras emit through `ModifierRegistry::install` (Track C), not a separate store. The aura's `target_filter` becomes part of the modifier's evaluation predicate; consult sites read modifiers from the registry and apply the filter against each candidate.

`AuraGrant` covers:

* `Keyword(Keyword)` — grant a printed-equivalent keyword (Rush, Reboot, Piercing, Blocker, Jamming, Save, Collision, Decoy, Progress, Vortex, etc.).
* `Dp { value, base, origin }` — DP modification matching `ModifierPayload::Dp` from Track C.
* `Cost { value, kind }` (`PlayCost`, `DigivolutionCost`, `LinkCost`) — cost modification.
* `SecurityAttack(i32)` — `Security A. ±N` grants.
* `Immunity(ImmunityKind)` — narrow protections (`OpponentDpReduction`, `OpponentDeDigivolve`, `BattleDeletion`, `EffectDeletion`, etc.).
* `Cannot(CannotKind)` — `Suspend`, `Unsuspend`, `Block`, `Attack`, `Move`, `ReturnToHand`, `ReturnToDeck`, etc.
* `TriggeredEffect(EffectId)` — the granted-triggered-ability case (§3).

### 2. `AuraScope` and target-filter evaluation

`AuraScope` decides who installs the modifier and which candidates the filter sees:

* `Permanent(PermanentHandle)` — single-target aura (the existing `grant_keyword` shape; route through the new builder for consistency).
* `Player(PlayerId)` — filter-aura over the named player's permanents. Filter sees every permanent owned by that player.
* `OpponentPlayer(PlayerId)` — cross-side aura. `PlayerId` is the source player; filter sees opponent's permanents. Honor the duration-flip rule: `Expiry::UntilOpponentTurnEnd` becomes `UntilOwnerTurnEnd` from the receiving side's perspective. Document this in `docs/RUST_ENGINE_API.md`; do not make the caller compute it.
* `Bilateral` — filter sees permanents on both sides. The filter predicate is responsible for narrowing if needed.
* `SecurityZone(PlayerId)` — security-sourced aura. The aura is active while the source card is in the named player's security stack. Filter sees battle-area permanents per printed text.

Target filter evaluation is lazy (DCGO pattern): the modifier carries the filter as a `CompiledPredicate`; consult sites re-evaluate the filter against the candidate at consult time. This avoids the "aura membership tracking" footgun. Per-tick cost is bounded — auras are <10 active at any time.

### 3. Granted triggered ability (`AuraGrant::TriggeredEffect`)

Mirror DCGO's `AddSkillClass.cs`. The grantor publishes a complete `Effect` (with timing + body); the receiving permanent's effect-firing site reads the granted effect from the registry when dispatching observers of the matching timing.

```rust
let granted = Effect::on_play(grantor_card)
    .name("[On Play] grantor's bonus effect")
    .process(|ctx| { /* … */ })
    .build();

Effect::declarative(grantor_card)
    .aura()
        .scope(AuraScope::Permanent(target))
        .grants(AuraGrant::TriggeredEffect(granted.id()))
        .duration(Expiry::UntilLeaveField)
    .build()
```

Two attribution flags on the granted effect dispatch:

* Carrier = the receiving permanent (whose timing fires the effect).
* Source = the grantor card (whose printed text the effect originated from).

Predicates in the granted effect's body should resolve against the carrier when reading "this Digimon" / "its DP" — not the grantor. This matches DCGO's `EffectSourcePermanent` / `EffectSourceCard` distinction.

EX1-068 ("grant a triggered effect to opponent's permanent") is the canonical fixture. Apocalymon-style cards (Track K cross-card refire is different — refire activates an existing effect once; granted-triggered-ability persists and fires on every matching event until expiry).

### 4. Conditional auras via UntilCondition

Auras that say "while X is true" install with `Expiry::UntilCondition(predicate)`. The UntilCondition controller (PR #458) handles eviction. The aura builder should accept the condition as a first-class field:

```rust
Effect::declarative(card)
    .aura()
        .scope(AuraScope::Player(controller))
        .target_filter(...)
        .grants(...)
        .while_condition(predicate)              // installs with Expiry::UntilCondition(predicate)
    .build()
```

`while_condition` is sugar over `.duration(Expiry::UntilCondition(predicate))` — keeps card YAML readable.

ZEPH-G004 Vortex aura is the canonical fixture: `VortexCanAttackPlayer` aura active while opponent has no unsuspended Digimon. When opponent unsuspends a Digimon mid-turn (e.g. via Reboot), the aura evicts immediately; if opponent re-suspends, the aura does not re-install (per the printed-semantics rule from PR #458 — `false → true` does not re-install).

### 5. Security-sourced auras (`AuraScope::SecurityZone`)

Some printed text gives an effect to security cards: "while this card is in your security stack, all your Digimon gain Save". The aura emits while the source is in security; on exit (security trash, Recovery, security activation), the modifier evicts.

Implementation: the aura registers a `WhenLoseSecurity` observer (Track A) on the source player; on fire, evicts the modifier. The aura installs at security-add time (`OnPlaceSecurity` / `WhendAddSecurity`).

This is the only aura scope that requires explicit zone-change observers; the others rely on `Expiry::UntilLeaveField` (which already covers leave-battle-area).

### 6. Inherited auras (sourced-keyword stack-traversal)

When an aura's source is in a digivolution stack (not the top card), the aura should still emit. Track G's Progress fix already verified `Game::has_keyword` walks the stack; aura emission needs to do the same.

The aura's source identity is the inherited source card, not the top card. Predicates that read "this Digimon" should resolve to the carrier (top card); predicates that read "the source of this aura" should resolve to the inherited source.

Reuse Track A's payload contract — the source-card field on the modifier installation distinguishes carrier from inherited source.

### 7. Consult-site audit

Every modifier consult site that reads a `ModifierType` must now also consult auras (which are modifiers with `target_filter` evaluation). Audit:

* DP read: `Game::effective_dp` (and `permanent.synth_identity().dp` from Track C).
* Cost read: play cost, digivolution cost, link cost calculation sites.
* Keyword read: `Game::has_keyword` (already walks stack; verify it consults aura-emitted keyword modifiers).
* Cannot* / Immune* reads: the corresponding mutation paths (suspend/unsuspend/block/attack/return/move).
* Security A. read: `game.rs:2081-2082` security-trigger count site.
* Granted triggered abilities: observer dispatch must include aura-granted effects in the dispatched set.

For each site, confirm aura modifiers are read with the target filter applied. Add a unit test per consult site that an aura-emitted modifier is honored.

### 8. DSL surface

Add YAML schema and lowering:

```yaml
modifiers:
  - kind: aura
    scope: player_self                          # | player_opponent | bilateral | permanent(<sel>) | security(<sel>)
    target:
      filter:                                    # CompiledPredicate
        all_of:
          - trait: holy
          - color: yellow
    grants:
      - dp: { value: 1000, base: false }
      - keyword: rush
    while:                                       # optional condition (UntilCondition)
      opponent_field_count: { lte: 0 }
    duration: until_leave_field                  # default if `while` absent
```

For granted-triggered-abilities:

```yaml
modifiers:
  - kind: aura
    scope: player_self
    target:
      filter: { trait: olympos_xii }
    grants:
      - triggered_effect:
          timing: on_play
          process: { gain_memory: 1 }
    duration: until_leave_field
```

For security-sourced:

```yaml
modifiers:
  - kind: aura
    scope: security(self_owner)
    target:
      filter: { kind: digimon, owner: self }
    grants:
      - keyword: save
```

Schema-only landings behind a feature flag must fail loudly at runtime if the consult site isn't wired.

### 9. Card-shaped fixtures

For each capability, land at least one card-shaped fixture under `code/digimon-engine/tests/cards_behavioral/<set>/<card_id>.rs`. Use `tests/cards_behavioral/bt24/bt24_062.rs` as the template:

* A simple "all your Holy Digimon gain +1000 DP" card — search printed text in `data/cards.json` for "all your" + trait/color/level prefix. Tests `AuraGrant::Dp` + `AuraScope::Player` + filter.
* A Royal Knights named-target keyword aura — "all your Royal Knight Digimon gain Rush". Tests filter-aura with name prefix.
* A cross-side aura: "opponent's Digimon get -2000 DP" — tests `AuraScope::OpponentPlayer` + duration-flip rule.
* A Security A. ±N aura — TS Olympos. Tests `AuraGrant::SecurityAttack`.
* EX1-068 (or modern equivalent) — granted triggered ability on opponent's permanent. Tests `AuraGrant::TriggeredEffect` + carrier/source attribution.
* ZEPH-G004 Vortex — conditional aura via `while_condition`. Tests UntilCondition integration; aura evicts on opponent unsuspend mid-turn; does not re-install on re-suspend.
* A security-sourced aura card — tests `AuraScope::SecurityZone` + `WhenLoseSecurity` eviction.
* Puppets G008 — inherited opponent-security Digimon DP aura DSL bridge. Tests inherited-source attribution.
* An aura with narrow opponent-effect protection — `AuraGrant::Immunity(OpponentDpReduction)` over a filter. Tests immunity narrowness (own-effect DP reduction still applies).

### 10. Tests

Mirror `tests/combat/track_c_deferred_modifiers.rs` and `tests/combat/until_condition_controller.rs`. Add `tests/auras/` directory (or extend existing modifier tests). Cover:

Aura unit tests:

* Each `AuraGrant` variant emits a modifier consultable at the documented consult site.
* Each `AuraScope` variant routes targets correctly (player-scope, opponent-scope, bilateral, permanent, security).
* Target filter is evaluated lazily (per-read), not at install. Confirm a permanent that joins the filter set after aura install is included on next read.
* A permanent that leaves the filter set after aura install is excluded on next read (e.g. trait change via `ChangeTraits` from Track C flips the match).
* Cross-side aura honors the duration-flip rule (`UntilOpponentTurnEnd` from source A becomes `UntilOwnerTurnEnd` from receiving B).
* Inherited aura: source in digivolution stack still emits; carrier identity resolves correctly.
* `while_condition` aura installs as `Expiry::UntilCondition`, evicts on predicate flip false, does not re-install on flip true.
* Granted triggered ability fires on the carrier's matching timing with carrier+source attribution.
* Security-sourced aura installs at security add, evicts on `WhenLoseSecurity`.
* Two auras granting the same keyword to overlapping target sets stack as expected (idempotent — the keyword is granted, not stacked).
* Two auras with conflicting grants (e.g. one grants `CannotSuspend`, another grants `Save`) coexist; both modifiers active.
* Aura survives source's zone-internal moves (e.g. source moves between battle-area positions); evicts on source leaving battle-area unless `SecurityZone` or `UntilCondition`.
* DP aura with `base: true` overrides printed DP; with `base: false` adds.

Consult-site audit tests (one per site in §7):

* DP read consults aura DP modifiers.
* Play / digivolve / link cost reads consult aura cost modifiers.
* `Game::has_keyword` consults aura keyword modifiers.
* `Cannot*` mutation paths consult aura `CannotKind` modifiers.
* Security trigger count consults aura `SecurityAttack` modifiers.
* Observer dispatch includes aura-granted triggered abilities.

Cross-track integration tests:

* Aura interaction with Track B replacement: an aura granting `Cannot::Delete` reaches the leave-field hook before the replacement framework engages.
* Aura interaction with Track C identity overlay: an aura that grants Rush to "all Holy Digimon" includes a Tamer treated as Holy via `ChangeTraits`.
* Aura interaction with Track D combat: an aura granting `Piercing` flows through the attack pipeline correctly.
* Aura interaction with Track G keywords: granted Decoy honors the color filter from Track G's `Decoy(u8)` payload.
* Aura interaction with the UntilCondition controller: a `while_condition` aura evicts at the same event boundary as other UntilCondition modifiers.

## Acceptance gates

* The `Aura` builder is reachable from both raw Rust and YAML; both paths produce identical modifier installs.
* Single-target grants (`grant_keyword`) route through the new builder for consistency; the old call sites continue to work.
* Filter auras evaluate the target filter lazily at consult time.
* Cross-side aura honors the duration-flip rule.
* Inherited aura sources are walked correctly; carrier vs. source identity is preserved.
* Conditional auras install as `Expiry::UntilCondition` and evict via the controller, with `false → true` non-restoration semantics.
* Security-sourced auras install at security add, evict on `WhenLoseSecurity`.
* Granted triggered abilities fire on the carrier's timing with both carrier and source attribution available to predicates.
* Every consult site listed in §7 has a unit test confirming aura modifiers are honored.
* Every player-visible choice introduced by an aura (e.g. an aura that grants an optional `[Main]` activation) surfaces through `pending_selection` and the action mask.

## Constraints

* No-approximations: every player-visible choice surfaces through `pending_selection` and the action mask.
* Do not expand `ACTION_SPACE_SIZE`, active tensor profiles, PyO3 exports, frontend constants, or RL wrappers. Aura emission produces modifier installs that read at consult time; no new actions.
* Source priority: when DCGO and printed text disagree, printed text wins. DCGO is for the lazy-filter pattern, the duration-flip rule, the `AddSkillClass` shape, and consult-site placement only.
* Don't transliterate. DCGO is C# + Unity coroutines + bucket-by-duration effect lists; you are using sync resolution + `ModifierRegistry`. The reference is for what reads what at consult time, not how to model the storage.
* TDD discipline: failing test before implementation. CLAUDE.md Working Rule 18.
* Do not author new Python-side card scripts (Working Rule 21).
* `code/engine_py_legacy/` is sunset reference material; do not import from it (Working Rule 22).
* Aura emissions go through the existing `ModifierRegistry`, not a parallel store. The aura builder is sugar over modifier installs with target filters; do not introduce an `AuraRegistry`.
* The duration-flip rule (DCGO `GiveEffectToPermanentOrPlayer.cs:17-37`) is the single most error-prone part of cross-side auras. Test it explicitly. The temptation to "be helpful" by interpreting `UntilOpponentTurnEnd` from the source's perspective in all cases is wrong — the receiving side's perspective is what the modifier carries.
* `Expiry::UntilCondition` is the right shape for conditional auras. Do not invent a separate "active_when" predicate on `Effect` — that path was deferred for good reason and the controller now handles it cleanly.
* Lazy filter evaluation means the aura installs once and the filter runs at every consult. Don't pre-compute the matching set at install — it goes stale on board changes. Don't re-evaluate the filter on every state change — the consult sites already re-read on demand.
* Granted triggered abilities (§3) are persistent until expiry; cross-card refire (Track K) is one-shot. Do not conflate. If a card prints "activate one of X's effects once", that's Track K refire, not a granted ability.
* Cross-track coordination: Track A owns event payloads (granted-triggered-ability dispatch reads from A's payload); Track B owns the leave-field hook (immunity auras consult here); Track C owns modifier installs and the identity overlay (auras emit through C's registry; identity reads consult both Track C overlays and Track H aura grants in the same call); Track G owns keyword definitions (aura keyword grants reuse Track G's `Keyword` enum, including parameterised `Decoy(u8)` and `Fragment(u8)`); UntilCondition controller (PR #458) owns aura eviction for `while_condition` auras. If you discover a needed payload field, modifier variant, replacement window, keyword, or controller hook, file the gap against the right track rather than building a private substitute.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test auras
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat track_c_deferred_modifiers
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat until_condition_controller
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_f progress
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Watch tensor and mask parity tests; aura DP grants and aura keyword grants flow into observation tensors. Update parity expectations rather than restore the old behavior.

Watch the `cards_behavioral` count — currently 2067 passing. New fixtures should push the count up; nothing should regress.

## Tracker discipline

Mark entries closed, partially closed, or narrowed, with the test command that proves the new status:

* `docs/RUST_ENGINE_GAPS.md` — Named-target declarative aura (DP / keyword grants filtered by name/trait/level); Declarative aura sourced from security zone; Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`); Granted triggered ability — attach an `Effect` to another permanent (EX1-068); Grant `Security A. ±N` — parametric `SecurityAttackChange`; Sourced-keyword stack-traversal (verify Track G's fix covers this); Conditional aura with state predicate; Player-scoped passive modifiers (close anything not closed by Track C).
* `qa/archetype-qa/engine-gaps.md` — legacy archetype-engine entries.
* `qa/dsl-vocab-gaps.md` — DSL `aura` `kind:` slot, scope variants, grants variants, `while:` condition slot.
* The relevant `qa/archetype-qa/dsl/*.md` rollups — Royal Knights, DNA Omnimon, Medusamon, BG Imperial, Zephagamon, Puppets, TS Olympos, Red Hybrid, Chaos Control. Mark cards now expressible with the aura system; demote `raw_rust` carve-outs that were waiting on auras.
* `docs/RUST_ENGINE_API.md` — document the `Aura` builder, `AuraScope` and `AuraGrant` enums, the duration-flip rule, the lazy-filter contract, the granted-triggered-ability carrier/source attribution, and the security-sourced eviction contract.

## What to do next

Start by reading the listed files in order. The `Effect` builder + `ModifierRegistry` extensions are the cross-track contract — get the shape right before writing card-side code.

Land in this order so each piece can ship against the test suite incrementally:

1. `Aura` builder + `AuraScope` + `AuraGrant` enums (§§1–2). Failing tests first for single-target sugar, then filter-aura, then cross-side.
2. Consult-site audit (§7) — write a unit test per site that an aura modifier is honored. Many sites already work because they consult `ModifierRegistry`; the audit confirms.
3. Granted triggered ability (§3). Reuse `AddSkillClass` shape from DCGO. Carrier/source attribution test first.
4. `while_condition` integration with UntilCondition (§4). The controller already evicts; aura builder just routes the predicate. Test ZEPH-G004 explicitly.
5. Security-sourced aura (§5). New observer subscription; small but distinct write surface.
6. Inherited aura (§6). Verify Track G's stack-walk applies; backfill if needed.
7. DSL surface (§8). Schema + lowering for everything above.
8. Card-shaped fixtures (§9). One per capability; YAML + behavioral test.
9. Tracker discipline and ship.

When you encounter a missing event payload, modifier variant, replacement window, selection shape, or controller hook that the engine cannot already produce, file the specific gap clearly in the relevant tracker and proceed using the closest existing API rather than building a private substitute.
