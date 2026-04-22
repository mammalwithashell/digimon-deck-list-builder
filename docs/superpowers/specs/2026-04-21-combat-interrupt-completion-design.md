# Combat Interrupt Completion — Design Spec

**Date:** 2026-04-21
**Status:** Design — not yet planned or implemented. Gates Phase 9 of the Rust engine roadmap (`.claude/plans/recursive-coalescing-candle.md`).
**Goal:** Close out the combat state machine by wiring the remaining interrupt windows, replacement timings, and keyword consumers so the Rust engine faithfully dispatches every printed combat-window effect — exposed to the RL action space per Working Rule 17.

---

## 1. Motivation

Phases 0–8 landed most of the scaffolding:

- **Phase 1** wired `OnAttack` / `WhenAttacking` / `EndOfAttack` / `EndOfBattle` observer timings (`combat.rs:1120-1243`).
- **Phase 3** parsed native keywords (Rush, Jamming, Blitz, Raid, Alliance, Vortex, Collision, Blocker, …) off the card face into `CardData.keywords`.
- **Phase 4** added selection-kind infrastructure (`SelectTarget`, `SelectBlocker`, optional-selection).
- **Phase 6** layered `CannotAttack`, `CannotBlock`, `CannotCounter`, `CannotAttackTarget` player/permanent-scoped restrictions.
- **Phase 7** shipped the replacement-timing framework and reserved two combat variants (`WhenWouldAttack`, `WhenWouldBeAttackTarget`) without wiring dispatch.
- **Pre-Phase-9 combat partials:** Alliance window (partial — no trait filter), Counter window (blast-digivolve only), Block window (end-to-end).

**What Phase 9 closes** (~30 cards across the 5 audited archetypes):

| Gap | Archetype drivers | Cards |
|-----|-------------------|-------|
| **Counter hand-play path** (non-blast Ace Counter Options + field-triggered Counter abilities) | Dark Masters Ace Lv6 sub-archetype (LM-043 Darkdramon, EX10-010 BlackWarGreymon, BT16-026 Vikemon, EX8-026 MetalSeadramon, EX10-074 Beelzemon, BT16-046 GranKuwagamon, BT21-051 Puppetmon, BT19-064 Justimon: Blitz Arm) | 8 |
| **Effect-driven attack redirect** (`ctx.redirect_attack`) | TS Olympos (BT18-073, ST18-14, ST15-14, EX8-050, EX8-051) + Dark Masters (EX10-010) | 6 |
| **`WhenWouldAttack` + `WhenWouldBeAttackTarget` replacement dispatch** | Scattered across DNA Omnimon passive blockers, TS Olympos Tamer-gated attack denial, Dark Masters attack-target-restriction shells | ~8 |
| **Raid target-switch rider** (attacker re-targets when original target leaves mid-attack) | TS Olympos Raid sub-archetype | 3 |
| **`OnBlock`, `OnAllyAttack`, `OnOpponentAttack` observer dispatch** | Dark Masters (BT15-008 Muchomon), scattered ally-synergy cards | 4 |
| **Collision MUST-block enforcement** (flip `is_optional`, strip PASS) | TS Olympos (BT24-063 Locomon) | 1 |
| **Piercing post-battle security check** | Dark Masters + any Piercing-keyword card | 3 |
| **Reboot unsuspend-phase consumer** | Dark Masters (4) + scattered | 4 |

**Non-combat implications.** Once the combat pipeline is complete and every window exposes its decision node through `pending_selection`, RL can learn policy over the full interrupt space — today an agent training against the Rust engine cannot see Counter plays, cannot choose to redirect, cannot discover the "must block with Collision" constraint.

---

## 2. Scope

**In scope:**

1. **Counter-window broadening** — a generic `Effect::counter(card)` builder flag distinct from `Effect::blast_digivolve(card)`, and a `CounterEffect` timing dispatched for hand-play Counter Options + field-Digimon-triggered `[Counter]` abilities. Widens `try_enter_counter` to emit both shapes side-by-side in one selection.
2. **`WhenWouldAttack` + `WhenWouldBeAttackTarget` dispatch** — fire-site wiring for the two Phase 7 reserved variants. Attacker-side fire happens at attack declaration; defender-side fires at target declaration, upstream of Alliance/Counter/Block. Supports cancel (abort attack) + redirect (change attacker or target).
3. **`ctx.redirect_attack(new_target)` + `ctx.cancel_attack()` helpers** on `EffectContext` — script-accessible redirect/cancel primitives. Fires `OnAttackTargetChange` on redirect; cleanly exits the state machine on cancel.
4. **Raid target-switch rider** — when `effective_target` becomes invalid between Block and Battle (deleted by an interrupt, returned to hand, etc.) AND attacker has `<Raid>`, emit a re-target selection. Today Raid is mask-only and the attacker just misses.
5. **`OnBlock` observer timing** with a new `TriggerSource` variant that carries `{attacker, blocker}`. Fired inside `try_enter_block` post-declare.
6. **`OnAllyAttack` / `OnOpponentAttack` observer dispatch** — already enum-declared, fire-site missing. Fired in `begin_attack_impl` fan-out on both sides of the attacker-controller boundary.
7. **Collision mandatory-block enforcement** — when attacker has `<Collision>` AND defender has any legal blocker, the Block-window selection is non-optional (`is_optional = false`), and the mask drops the PASS bit.
8. **Piercing post-battle security check** — after `resolve_battle` deletes the defender (attacker wins, both wiped, or attacker survives), if the attacker has `<Piercing>` and still exists, continue into a security check against the defending player. Today the security check only fires on direct player-target attacks.
9. **Reboot consumer** — in the unsuspend phase at the start of each player's turn, OPPONENT's Digimon with `<Reboot>` also unsuspend. Requires scanning both players' battle areas at the unsuspend step.
10. **Alliance trait filter** — complete Phase 1's Alliance-window partial by honoring `Alliance<Trait>` filters on the alliance-candidate scan. Today every Alliance-keyword ally qualifies regardless of trait filter.

**Out of scope (for this spec):**

- **Counter Options as first-class Option cards.** The Phase 8 Option pipeline handles Option-card play mechanics (cost, OptionMain, trash). Phase 9 wires the Counter **window** to route those Options to a play-from-hand invocation of Phase 8's `play_option_from_hand` with a `CounterEffect` timing overlay. No new Option subtype is added; a Counter Option IS a Standard Option whose effect carries `.counter()`.
- **Nested Counter chains.** A Counter Option fired during the Counter window cannot itself open another Counter window during its resolution. This matches printed rules — Counter is a one-shot interrupt window.
- **`WhenWouldAttackStep` / attack-step-specific timings.** Phase 9 dispatches at *declaration*; per-step replacements (mid-battle interrupts) are not in printed rules and stay unreserved.
- **Mid-battle digivolving.** Blast Digivolve on the Counter window is the current shape; "digivolve during block" is not a printed mechanic.
- **Multi-attack chaining riders.** Some cards grant "after battling, attack again" — this is existing Task 3.5-era scaffolding handled by attack-count bookkeeping; Phase 9 does not touch the multi-attack loop.
- **`SwitchAttacker` observer** (the inverse of `OnAttackTargetChange`). Not present in current printed card pool; hold until a card demands it.

---

## 3. Design Principles

Every decision in this spec obeys these invariants:

1. **No auto-selection (Working Rule 17).** Every Counter candidate, every Block choice under Collision, every Raid re-target surfaces as a `PendingSelection` with every legal branch in `valid_action_ids`. Collision's "must block" collapses the PASS branch, but the choice of *which* blocker is still a selection.
2. **Combat is a state machine, not a stack of fires.** Every new interrupt site is a named state transition (or a named replacement-fire step within an existing state). `advance_pending_attack` drives a loop over states; it never hides an effect behind an ad-hoc if-branch inside a state's body.
3. **Replacements subsume restrictions.** `WhenWouldAttack` with `ctx.cancel()` is the replacement-framework form of "your opponent cannot attack with this Digimon." Where a restriction modifier exists (Phase 6 `CannotAttack`), it auto-installs a WhenWouldAttack passive cancel — keeping the Phase 6 API as the ergonomic surface while the Phase 7 framework does the work underneath.
4. **Observers see post-replacement reality.** If `WhenWouldBeAttackTarget` redirects P0's attack from A to B, the `OnAttack` / `WhenAttacking` / `OnBlock` observers all see target=B, not target=A. Order: replacement first, observers second.
5. **Keyword-driven helpers are auto-installed.** Printed keywords on a card face (Raid target-switch, Collision, Piercing, Reboot) install their runtime effects via the Phase 3 / Phase 7 `keyword_to_auto_effect` pattern at registry build time. No card author writes a `.raid()` builder method; the engine emits the correct Would/observer effect for every Raid-tagged card automatically.
6. **Single entry point per script API.** `ctx.redirect_attack(new_target)` and `ctx.cancel_attack()` are the only script-facing combat mutators. Internal state-machine transitions stay `pub(crate)`.
7. **Counter window is union-zone.** Hand-Counter-Option-play and field-Digimon-Counter-ability-trigger both emit into the same selection. Action IDs encode the source (hand or field) unambiguously.
8. **Parity with Phase 7 framework.** Combat-side replacements (`WhenWouldAttack`, `WhenWouldBeAttackTarget`) use the exact same `try_replace` dispatcher, `ReplacementContext`, `ReplacementOutcome`, `ReplacementCause` as Phase 7's deletion/return replacements. No parallel infrastructure.

---

## 4. Attack State Machine — Phase 9 additions

Current shape (`combat.rs:250-315`):

```
Declared → AllianceOpen → CounterOpen → BlockOpen → Battle → Cleanup
                                                              ↘  (Vortex short-circuit from Declared)
```

Phase 9 inserts three new states and two replacement-fire points:

```
         [WouldAttack fires]
              ↓
Declared ─────────────────→  [WouldBeAttackTarget fires]
              ↓                         ↓
         AllianceOpen            (cancel or redirect?)
              ↓                         ↓
         CounterOpen ←───────────────────
              ↓
         BlockOpen
              ↓
         PostBlock (NEW — Raid re-target rider if effective_target invalid)
              ↓
         Battle
              ↓
         PostBattle (NEW — Piercing security check)
              ↓
         Cleanup
```

### 4.1 `WouldAttack` + `WouldBeAttackTarget` fire sites

**Placement**: `begin_attack_impl` fires `WhenWouldAttack` on the attacker BEFORE any state transitions. Immediately after, fires `WhenWouldBeAttackTarget` on the declared target. Both are replacement dispatches via `Game::try_replace`.

**Rationale for placement:** these replacements precede ALL triggered observers. A card that says "opponent's Digimon cannot attack this Digimon" wants to cancel before `OnAttack` ever fires — otherwise the attacker's `<Rush>` turn-count bumps, memory transitions, etc. have already happened and cannot be cleanly undone.

**Subject semantics:**
- `WhenWouldAttack`: `ReplacementSubject::Permanent(attacker_handle)`; `cause = ReplacementCause::OwnEffect` or `Battle` (Battle is the default for declaration; OwnEffect only for effect-forced-attack like Jamming).
- `WhenWouldBeAttackTarget`: `ReplacementSubject::Permanent(target_handle)` for Digimon targets, `ReplacementSubject::Player(player_id)` for direct player attacks.

**Outcome mapping:**
| Outcome | Effect on state machine |
|---------|------------------------|
| `None` (no replacement applied) | Proceed to `AllianceOpen` |
| `Cancelled` | Short-circuit to `Cleanup`. No memory swing from the aborted attack. Fires `EndOfAttack` for cleanup symmetry. |
| `Substituted(new_target)` | Rewrite `pending_attack.effective_target = new_target`. Fire `OnAttackTargetChange` for the redirect. Proceed to `AllianceOpen`. |
| `Redirected(_)` | Not meaningful for attack-shape — would require a zone semantics mismatch; `debug_assert!(false)`. |

### 4.2 `PostBlock` state (new)

**Purpose**: Raid target-switch rider. If an interrupt during `BlockOpen` (or a WhenWouldBeAttackTarget redirect that happened earlier) left `effective_target` invalid — e.g. the target was Evade-redirected, deleted by a replacement, or returned to hand — AND the attacker has `<Raid>` AND a legal retarget exists, emit a Raid re-target selection.

**State entry check:**
```rust
if !self.handle_valid(pending.effective_target)
    && self.has_keyword(pending.attacker, Keyword::Raid)
    && self.has_any_raid_retarget_candidate(pending)
{
    self.transition_attack_state(AttackState::PostBlock);
    self.enter_raid_retarget();  // installs PendingSelection::OwnField (opp side)
    return;
}
```

**If no retarget available** (no legal candidate, or attacker lacks Raid): transition to `Cleanup` — the attack fizzles. This is the printed-rules behavior for any attacker whose target vanishes mid-attack.

### 4.3 `PostBattle` state (new)

**Purpose**: Piercing security check. If `resolve_battle` wiped the defender AND attacker survived AND attacker has `<Piercing>`, the attacker continues into a security check against the defending player (as if the attack had been a direct player attack from the start).

**State entry check:**
```rust
if self.attacker_survived(pending)
    && self.defender_was_wiped(pending)
    && self.has_keyword(pending.attacker, Keyword::Piercing)
{
    self.transition_attack_state(AttackState::PostBattle);
    self.enter_piercing_security_check();  // same resolver as direct-player attack
    return;
}
```

**Interaction with `<Jamming>`**: Jamming on the attacker already shields the attacker from security-skill damage during the initial check. Piercing's post-battle check is a *new* check — it honors Jamming independently. No change to Jamming today; just re-enters `drive_security_resolution` with attacker = current attacker.

### 4.4 State-machine invariants preserved

- `advance_pending_attack` remains the sole driver. Each new state has an entry predicate and transitions by the existing `transition_attack_state(new_state)` helper.
- No state skips `EndOfBattle` / `EndOfAttack` dispatch; `Cleanup` is still the sole exit point. A `Cancelled` `WouldAttack` still runs `Cleanup` with a flag suppressing battle-adjacent EndOfBattle (attack never happened) but firing EndOfAttack (attack declaration resolved).

---

## 5. Counter Window Broadening

### 5.1 Today's shape (`combat.rs:468-599`)

`try_enter_counter` scans defender's **hand** for effects carrying `blast_digivolve == true`, cross-joined with defender's field Digimon via `can_digivolve`. Emits `encode_digivolve(h, f)` actions. `execute_blast_digivolve` moves the card to the stack and fires `WhenDigivolving`. No memory cost.

### 5.2 Phase 9 shape

The Counter window emits three candidate shapes into one union-zone selection:

| Shape | Source | Cost model | Resolution |
|-------|--------|-----------|-----------|
| **Blast Digivolve** (existing) | Defender's hand, effect has `.blast_digivolve()` | 0 memory; blast-as-digivolve rules | Move to stack; fire `WhenDigivolving` |
| **Hand Counter Option** (new) | Defender's hand, effect has `.counter()` AND card is `CardKind::Option` | Normal Option play cost (via Phase 8 `play_option_from_hand`) | Fire `CounterEffect` timing + OptionMain body; dispose per Phase 8 |
| **Field Counter Ability** (new) | Defender's field Digimon, effect on-permanent has `.counter()` + `.timing(EffectTiming::CounterEffect)` | Per-effect `.pay_cost_fn` or modifier-gated; default 0 | Fire the effect body directly (the ability is on a permanent; no play-from-hand) |

### 5.3 `EffectBuilder::counter()` method

```rust
impl EffectBuilder {
    /// Mark this effect as eligible to fire in the Counter window (defender
    /// plays this when attacked). Distinct from `.blast_digivolve()` which
    /// implies a free-cost hand-digivolve-as-counter; `.counter()` marks a
    /// generic Counter-window effect that uses normal activation/cost rules.
    ///
    /// Use with `.timing(EffectTiming::CounterEffect)` (or one of the Option
    /// timing builders `.option_main()` for Counter Options).
    pub fn counter(mut self) -> Self {
        self.inner.counter = true;
        self
    }
}
```

The existing `counter: bool` field on `Effect` (set today as a side effect of `.blast_digivolve()`) is now a first-class flag settable via either builder.

### 5.4 `EffectTiming::CounterEffect` dispatch

The variant already exists (`enums.rs:121`) and has zero dispatch sites today. Phase 9 adds a fire point in `try_enter_counter`: when the defender selects a hand Option with `.counter()` OR a field ability with `.counter()`, the effect body fires with `EffectTiming::CounterEffect`.

**No observer fan-out.** `CounterEffect` is a source-specific timing, NOT a global observer. Only the selected card's own `.counter()` effects fire.

### 5.5 Union-zone selection encoding

Using Phase 4's `SelectionKind::UnionZone { hand, field }`:

- Hand candidates: `valid_action_ids` entries encode `PLAY_HAND_START + hand_index` for Option plays, `DIGIVOLVE_START + (field, hand)` for blast.
- Field candidates: `EFFECT_CHOICE_START + field_index` (Counter ability on field permanent).
- PASS bit present (Counter is optional).

The decoder at the resolve-site inspects the action ID to determine candidate shape and routes to `play_option_from_hand`, `execute_blast_digivolve`, or `fire_counter_ability`.

### 5.6 Counter-chain prevention

A Counter Option/ability that itself resolves during the Counter window does NOT re-open another Counter window for its own effect-driven-attack (rare but printed on some cards). Enforced by a `pending_attack.counter_depth: u8` counter: incremented on Counter-window entry, decremented on exit, guard checked at state transition.

Default max depth: **1**. Exceeds printed rules' need. Violations log and no-op (debug_assert in dev).

### 5.7 Parity with Python

Python `combat.py:173-186` already gates Counter on `_is_blast_digivolve`. Phase 9 matches this for the blast case AND extends to `_is_counter` / `_is_counter_ability` flags — Python side ALSO does not implement hand-play Counter Options today (Python gap symmetrical to Rust). Parity entry: `§15 Phase 9 Counter window broadening — Rust leads Python; Python will port to match.`

---

## 6. Attack Replacement API — `ctx.redirect_attack` / `ctx.cancel_attack`

Script-facing helpers on `EffectContext`. Only callable during an active attack (asserts `pending_attack.is_some()`).

### 6.1 `ctx.redirect_attack(new_target: AttackTarget)`

```rust
impl EffectContext<'_> {
    /// Redirect the current attack to a new target. Fires `OnAttackTargetChange`.
    /// Must be called from within an effect dispatched during an attack
    /// (typically from a `WhenWouldBeAttackTarget` replacement process or an
    /// `OnAttack` observer).
    pub fn redirect_attack(&mut self, new_target: AttackTarget) -> Result<(), AttackError>;
}
```

Where `AttackTarget` is the existing `selection.rs` enum `{ Permanent(PermanentHandle), Player(PlayerId) }`.

**Behavior:**
1. Validate: new_target must be legal (own side if redirecting to own, existence check, not Delayed/Training/Linked).
2. Rewrite `pending_attack.effective_target = new_target`.
3. Fire `OnAttackTargetChange` with `TriggerSource::PlayerBattleArea(pid)` for both players (global observer).
4. If called from inside a replacement process, the replacement committer interprets this as `ReplacementOutcome::Substituted(new_target_handle_or_player)`.

### 6.2 `ctx.cancel_attack()`

```rust
impl EffectContext<'_> {
    /// Cancel the current attack declaration. State machine short-circuits
    /// to Cleanup. Memory and attack-count bookkeeping are rolled back.
    pub fn cancel_attack(&mut self) -> Result<(), AttackError>;
}
```

**Behavior:**
1. Flag `pending_attack.cancelled = true`.
2. `advance_pending_attack` detects the flag and transitions directly to `Cleanup`.
3. Cleanup skips `EndOfBattle` (no battle happened) but still fires `EndOfAttack` and expires `EndOfAttack` modifiers.

### 6.3 Composition with `WhenWouldAttack` / `WhenWouldBeAttackTarget`

When a replacement process calls `ctx.cancel_attack()` or `ctx.redirect_attack(...)`, the replacement committer checks `pending_attack.cancelled` and `pending_attack.effective_target` post-process. If cancelled → `ReplacementOutcome::Cancelled`. If target changed → `ReplacementOutcome::Substituted(new_target)`. Otherwise `ReplacementOutcome::None`.

This means a script author chooses between:
```rust
// Imperative style (preferred for readability):
.process(|ctx| { ctx.redirect_attack(new_target); })

// Declarative style (for complex ordering with other replacements):
.replacement_process(|ctx, rctx| { rctx.substitute(subject_handle); })
```

Both paths land in the same committer and produce the same `Substituted` outcome.

---

## 7. New Observer Dispatch (`OnBlock`, `OnAllyAttack`, `OnOpponentAttack`)

### 7.1 `OnBlock`

Fired inside `try_enter_block` AFTER the defender selects a blocker and `effective_target` is rewritten.

**New `TriggerSource` variant:**
```rust
pub enum TriggerSource {
    // ... existing variants ...
    /// Global observer for Block-window declaration. Carries both the
    /// declaring attacker and the chosen blocker so observers on either
    /// side can query context.
    OnBlock { attacker: PermanentHandle, blocker: PermanentHandle },
}
```

Global fan-out: both players' battle areas scanned; every matching `OnBlock` observer fires with the `{attacker, blocker}` context available via `ctx.attacker()` / `ctx.blocker()` helper readers.

### 7.2 `OnAllyAttack` / `OnOpponentAttack`

Fired inside `begin_attack_impl` fan-out AFTER `OnAttack` and `WhenAttacking`.

- **`OnAllyAttack`**: fires on every **allied** Digimon's effects (same controller as attacker, excluding the attacker itself).
- **`OnOpponentAttack`**: fires on every **opponent** Digimon's effects (opposite controller from attacker).

Both are `TriggerSource::PlayerBattleArea(pid)` fan-outs — no new TriggerSource variant needed. Timing-gated at effect scan time (`effect.timing == EffectTiming::OnAllyAttack` && `perm_controller == attacker_controller && perm_handle != attacker`).

**Parity**: Python fires at `combat.py:58-74` with a `{'attacker': ...}` context. Rust fan-out reads attacker via `ctx.attacker()` helper (already available on `EffectContext` from Phase 1).

### 7.3 `OnAttackTargetChange` — completion

Already fires on Block-redirect (`combat.rs:713-720`). Phase 9 extends fire sites to:
- `WhenWouldBeAttackTarget` `Substituted` outcome (via `ctx.redirect_attack` or `rctx.substitute`).
- `ctx.redirect_attack` called from any observer during an attack.
- Raid re-target selection resolve.

Parameters: fire with `{old_target, new_target, attacker}` context accessible via `ctx.old_target()` / `ctx.new_target()` / `ctx.attacker()`.

---

## 8. Collision MUST-block Enforcement

Current (`combat.rs:632-634`): Attacker's Collision makes every opponent Digimon a blocker **candidate**, but the selection remains `is_optional = true` — defender can PASS.

Phase 9 change:
- `try_enter_block` checks: if attacker has `<Collision>` AND the generated blocker-candidate list is non-empty, set `is_optional = false` on the `PendingSelection`.
- Action-mask layer (`mask.rs:517-542`): selection-phase mask drops the PASS bit when the underlying `PendingSelection` is non-optional.

**Parity clarification**: Python `permanent.can_be_blocker` honors Collision's mass-Blocker grant but doesn't force the selection mandatory. Python parity entry: `§15.2 Collision enforcement — Rust leads Python; Python passive-optionality-leak needs separate fix (file as Python issue).`

**Defensive corner case**: if attacker has Collision but ALL opponent Digimon are `<Blocker>`-restricted by a `CannotBlock` flood gate, the candidate list is empty — `is_optional = true` (no force-block possible, mask still emits PASS), attacker proceeds to Battle unblocked.

---

## 9. Keyword Auto-install for Combat Keywords

Phase 3's `cards/keyword_effects.rs::keyword_to_auto_effect` is the auto-install hook for printed keywords at registry build time. Phase 9 extends it for:

### 9.1 `<Raid>`

Auto-installs a `WhenWouldBeAttackTarget` redirect-rider: when the card attacks and its target leaves mid-flight (by another effect, Block-redirect, etc.), the rider tries to re-emit a retarget selection. This is the declarative form of §4.2's Raid re-target — it's the engine-side invariant, and the `AttackState::PostBlock` check inspects the auto-installed rider state rather than scanning keywords inline.

**Shape:**
```rust
Effect::new(card, EffectTiming::None)
    .timing(EffectTiming::WhenWouldBeAttackTarget)  // attacker-side rider
    .replacement_process(|ctx, rctx| {
        if rctx.subject == pending_attack.effective_target
            && !ctx.handle_valid(rctx.subject_handle())
        {
            let retargets = ctx.raid_retarget_candidates(pending_attack);
            if !retargets.is_empty() {
                ctx.install_retarget_selection(retargets);
                // Returns Handled — selection resolves asynchronously
                rctx.handled();
            }
        }
    })
    .build()
```

### 9.2 `<Piercing>`

Auto-installs a declarative hook at `AttackState::PostBattle`. No replacement timing — just a state-transition check that consumes `Keyword::Piercing`. No Effect object emitted for Piercing; the keyword is read directly by the state machine (like Rush / Jamming).

### 9.3 `<Collision>`

Same — no Effect emitted; keyword consumed directly by `try_enter_block` for the `is_optional = false` flip + candidate expansion.

### 9.4 `<Reboot>`

No Effect emitted; keyword consumed by `game_phases.rs::begin_turn` unsuspend fan-out: during `turn_player`'s unsuspend step, also scan `turn_player.opponent()`'s battle area and unsuspend every Digimon with `Keyword::Reboot`.

**Parity**: Python consumes Reboot at `game.py:412` (or equivalent — unsuspend phase).

### 9.5 `Alliance<Trait>` filter

Not a separate keyword variant. Instead, the Alliance-candidate scan (`combat.rs:363`) reads the attacker's `<Alliance>` effect text via existing `card_data.alliance_trait_filter: Option<Trait>` (to be parsed from the card face — currently missing). If set, `alliance_candidates` filters to allies carrying that trait.

**Trait filter parsing**: extend `card_data.rs:276` Alliance parser to accept `Alliance<Armor>`, `Alliance<Holy>`, etc. Store as `Keyword::AllianceFiltered(Trait)` parametric variant (mirrors Phase 3's `Keyword::Fragment(u8)`).

---

## 10. Cross-Phase Interactions

### 10.1 Phase 6 restriction modifiers ↔ Phase 9 replacements

Phase 6's `CannotAttack`, `CannotAttackTarget`, `CannotBlock`, `CannotCounter` restrictions are player/permanent-scoped modifiers. Phase 9 adds `WhenWouldAttack` / `WhenWouldBeAttackTarget` as the replacement layer.

**Composition:** Phase 6 remains the ergonomic API (set a `CannotAttack` modifier with cause/target filter; mask-layer hides the illegal action). Phase 9 introduces `try_replace(WhenWouldAttack, ...)` at the attack-declare fire-site; a passive Phase 6 modifier auto-installs a replacement that calls `rctx.cancel()`.

**Mandatory vs optional:** Phase 6 restrictions are mandatory (never surface as a PendingSelection). The auto-installed replacement is similarly mandatory — no Accept/Decline prompt.

**Mask-layer vs replacement-layer:** Both. Phase 6 mask-layer hides forbidden actions from `valid_action_ids` at mask-build time. Phase 9 replacement-layer catches any attack declaration that nonetheless reaches the engine (defensive — in case the mask is stale or a test cheats).

### 10.2 Phase 7 WhenWouldBe* ↔ Phase 9 combat Would*

Phase 7's `WhenWouldBeDeleted` / `WhenWouldLeaveBattleArea` fire inside `delete_permanent_with_cause`. A Phase 9 `<Piercing>` security check can trigger another `delete_permanent_with_cause` for a new defender's cards — Phase 7 framework handles the nesting correctly (once-per-event guard + depth guard already in place).

### 10.3 Phase 8 Option flow ↔ Phase 9 Counter window

Counter Options play through Phase 8's `play_option_from_hand` pipeline. The Counter-window selection routes the hand-index action to `play_option_from_hand` with an overlay flag `in_counter_window = true`, so the Phase 8 `play_option_core` knows to fire `CounterEffect` BEFORE `OptionMain`.

**Counter-depth guard**: set in `pending_attack.counter_depth`, checked at `try_enter_counter` entry. Prevents Counter Options from opening recursive Counter windows (§5.6).

### 10.4 Phase 4 selection kinds

Counter window uses `SelectionKind::UnionZone { hand, field }` (Phase 4). No new kind needed.

Raid re-target uses `SelectionKind::SelectTarget { own: false }` (existing Block/attack kind). Rebranded as `RaidRetarget` for mask disambiguation but otherwise identical.

---

## 11. Keyword Interaction Matrix

How new Phase 9 combat keywords interact with existing mechanics:

| Phase 9 keyword | Interacts with | Behavior |
|-----------------|----------------|----------|
| Piercing | Jamming | Piercing post-battle security check honors Jamming on attacker (no security-skill damage even during Piercing pass). |
| Piercing | Barrier (Phase 7) | If Piercing post-battle check reveals a security card with Barrier, Barrier fires normally. |
| Piercing | `<Security Attack +N>` | Security Attack stacks with Piercing's post-battle check — each card consumed is subject to the attacker's Security Attack modifier. |
| Collision | `CannotBlock` (Phase 6 flood gate) | Opponent's `CannotBlock` filter still applies — Collision only forces blocks from *legal* blockers. If all blockers are CannotBlock-gated, Collision is a no-op. |
| Reboot | `CannotBeUnsuspended` (Phase 6) | Reboot-driven unsuspend is effect-driven; a `CannotBeUnsuspendedByEffect` modifier applies. Specific card text TBD — default Reboot honors the restriction. |
| Raid | Collision | Raid re-target happens at `PostBlock`; Collision's force-block resolved first during `BlockOpen`. No direct interaction. |
| OnBlock observer | Collision | `OnBlock` fires even when the block was Collision-forced. Observers can inspect `ctx.block_was_forced: bool` if helpful. |
| WouldBeAttackTarget replacement | Raid re-target rider | If the declared target was Raid-redirectable AND a WouldBeAttackTarget replacement fires first (canceling), the Raid rider never triggers. Replacement > rider. |

---

## 12. Action-Mask Contract

Phase 9 preserves the ACTION_SPACE_SIZE = 2168 invariant — no new action IDs.

| Window | Action range reused | Selection kind |
|--------|---------------------|----------------|
| Counter hand-play Option | `PLAY_HAND` (100–113) | `UnionZone { hand, field }` + phase gate `CounterTiming` |
| Counter blast digivolve | `DIGIVOLVE` (114–155) | same union |
| Counter field ability | `EFFECT_CHOICE` (extend existing range or reuse `SEL_TARGET`) | same union |
| Raid re-target | `ATTACK` (100–249) | `RaidRetarget` (phase-gated re-use of SelectTarget) |
| Collision mandatory block | `ATTACK` (100–249) | `SelectTarget` with `is_optional = false` |

Any flood-gated action stays zeroed at mask-build per Phase 6. PASS bit drops when the selection is mandatory.

---

## 13. Testing Strategy

Phase 9 lands ~40 new tests across:

- `digimon-engine/tests/combat/counter_hand_play.rs` — 6 tests (Option Counter, Field Counter, blast+option coexistence, memory cost payment, CounterEffect firing, counter-chain depth guard).
- `digimon-engine/tests/combat/would_attack_replacements.rs` — 8 tests (WouldAttack cancel, WouldBeAttackTarget cancel, WouldBeAttackTarget redirect, Phase 6 CannotAttack auto-replacement, same-target-multiple-replacements layering, opt-in optional replacement emits selection).
- `digimon-engine/tests/combat/redirect_and_cancel.rs` — 5 tests (`ctx.redirect_attack`, `ctx.cancel_attack`, OnAttackTargetChange fire on redirect, mid-attack cancel memory rollback).
- `digimon-engine/tests/combat/raid_retarget.rs` — 4 tests (target leaves mid-attack, Raid retarget selection offered, no retarget → fizzle, non-Raid attacker → fizzle).
- `digimon-engine/tests/combat/collision_mandatory.rs` — 3 tests (Collision + legal blockers → mandatory selection + PASS dropped, Collision + all-illegal blockers → optional fallback, Collision + CannotBlock combinations).
- `digimon-engine/tests/combat/piercing_security.rs` — 4 tests (Piercing passes post-battle to security, Piercing + Jamming suppresses security damage, Piercing + Security Attack modifier stacks, attacker wiped → no Piercing).
- `digimon-engine/tests/combat/reboot_unsuspend.rs` — 3 tests (Reboot unsuspends on opponent's turn, Reboot + Overclock composition, Reboot + CannotBeUnsuspendedByEffect).
- `digimon-engine/tests/combat/on_block_observer.rs` — 3 tests (OnBlock fires globally, attacker-side observer sees attacker/blocker, Collision-forced block fires OnBlock).
- `digimon-engine/tests/combat/on_ally_opponent_attack.rs` — 3 tests (OnAllyAttack fires on same-controller permanents, OnOpponentAttack fires on opposite, ctx.attacker resolution).

Plus +1 e2e multi-interrupt scenario test in `behavioral_end_to_end.rs`.

**Target total: 671 → ~711 passing**.

---

## 14. Migration & v1 Constraints

### 14.1 Per-phase-close follow-ups

Documented as deferred items:

- **Counter-chain depth > 1.** Printed rule allows a single Counter fire; future expansion rules may allow chains. Default max depth stays 1 in v1.
- **`SwitchAttacker` replacement**. Not printed today; reserve enum variant but no fire site.
- **Multi-target attacks.** Some printed cards "attack all of your opponent's Digimon" — single-target state machine stays; multi-attack loops are a separate mechanic (Task 3.5 era).
- **Python-side Counter hand-play parity.** Python does not implement Counter Options either; Rust leads. Parity entry tracks this for future Python port.

### 14.2 API stability guarantees

- `Effect::counter(card)` builder lands as `pub fn counter()` on `EffectBuilder` — stable.
- `ctx.redirect_attack` + `ctx.cancel_attack` stable on `EffectContext`.
- `TriggerSource::OnBlock { attacker, blocker }` stable enum variant.
- Counter-window union-zone selection shape (action ID encoding) stable.

### 14.3 Post-merge tracking

- Python Counter-hand-play port.
- Python Collision mandatory enforcement port.
- DCGO cross-check for `CollisionKeyword.cs` mandatory-conversion logic.
- Multi-player turn-math generalization (shared with Phase 8 `compute_delay_trash_turn`).

---

## 15. Summary

Phase 9 closes Cluster I of the meta-roadmap: Counter hand-play, replacement-based attack gating, Raid rider, Collision mandate, Piercing post-battle, Reboot unsuspend, OnBlock / OnAllyAttack / OnOpponentAttack dispatch, Alliance trait filter. ~30 meta-pool cards unblocked. ~40 new tests. ACTION_SPACE_SIZE unchanged.

After Phase 9, combat is faithful: every interrupt window exposes every decision node through `pending_selection`, every keyword has a consumer, every replacement timing dispatches. Phase 10 (tokens + residuals) is the only remaining engine phase; the Rust engine reaches ~100% coverage of the meta pool.
