# "Would" Replacement-Timings Framework — Design Spec

**Date:** 2026-04-21
**Status:** Design — not yet planned or implemented. Gates Phase 7 of the Rust engine roadmap (`.claude/plans/recursive-coalescing-candle.md`).
**Goal:** Introduce a first-class replacement-effect layer to the Rust engine so effects can *intercept and substitute* an impending state change (deletion, return-to-hand, return-to-deck, trash, de-digivolve, etc.) before it happens, faithfully to the printed Digimon TCG rules and exposed to the RL action space per working rule 17.

---

## 1. Motivation

Phases 0–6 landed triggered observers (`OnDeletion`, `OnLeaveField`, `OnReturn`, `OnTrash`, `OnAnyDeletion`, …). Every observer fires **after** the event — the state change has already happened, and the effect just reacts.

The Digimon TCG also contains a fundamentally different effect family: **replacement effects**. These interrupt an impending state change *before* it occurs and either cancel it, redirect it, or substitute something else. In Magic terms they're "replacement effects"; in DCGO they're modeled as `ICardEffect`s that listen on pre-events and mutate the pending action. The current Rust engine has no analogue — replacement effects are currently either (a) approximated as post-hoc observers (breaking faithfulness), (b) embedded ad hoc inside `delete_permanent_with_effects` / `return_to_hand` (not extensible), or (c) not implemented at all.

The keywords and mechanics this blocks:

| Mechanic | Replaces | Example source |
|----------|----------|----------------|
| **Barrier** | Would-be-deleted → trash top of deck instead | TS Olympos (~9 cards) |
| **Evade** | Would-be-deleted → move to bottom of own deck | Dark Masters / TS Olympos (~6) |
| **Partition** | Would-be-deleted → delete a source instead | Medusamon (~3) |
| **Armor Purge** | Would-be-deleted → trash a source instead | Medusamon (~3) |
| **Fragment(N)** | Would-be-deleted → trash N from top of deck instead | Rocks (~4) |
| **Decode** | Would-be-returned-to-deck/hand → return to hand instead | TS Olympos (~3) |
| "Can't be returned to deck/hand" | Passive cancel of the return | scattered, ~5 cards |
| "Can't be de-digivolved" | Passive cancel of the de-digi | scattered |
| "Can't be trashed by your opponent's effects" | Passive cancel | scattered |
| Counter-redirect | Would-be-attack-target → redirect | overlaps Cluster I |

Cross-archetype audit total: **~60 cards** blocked on this framework (plus the Phase 6 *passive* flavor of each — see §10).

Replacement effects **cannot** be bolted on as more `EffectTiming` observers. They must see the impending event, decide whether to apply, and mutate what happens next — not just observe the aftermath.

## 2. Scope

**In scope:**

1. A new timing family — `EffectTiming::Would*` variants — that fire before the replaced state change commits.
2. A `ReplacementContext` passed to those effect processes, carrying cause attribution and mutating helpers (`cancel`, `redirect_to`, `substitute`).
3. A dispatch algorithm with defined ordering for multiple concurrent replacements.
4. `PendingSelection::Replacement` so **optional** replacements (Barrier, Evade) surface as RL branches — accept and decline both exposed in `valid_action_ids`.
5. A migration for Phase 6's deferred passive restriction modifiers (`CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, `CannotBeReturnedToHand`, `CannotBeTrashedByEffect`) to land as automatic replacements under this framework.
6. A `condition: Option<ReplacementConditionFn>` field on `ModifierEntry` / `PlayerModifierEntry` for passive replacements that depend on live state (e.g. "can't be trashed by *opponent's* effects but can be trashed by your own").
7. Native parsing of Barrier / Evade / Fragment(N) / Decode / Partition / ArmorPurge keywords into the new framework at registry build time (keyword → automatic replacement install).

**Out of scope (for this spec):**

- `WhenWouldAttack` / `WhenWouldBeAttackTarget` full integration — covered by **Phase 9** (combat interrupt completion) because it co-designs with Raid target-switch, Counter, and Collision. The *enum variants* are reserved here so downstream work doesn't re-number, but the dispatch sites live in Phase 9.
- Option-card replacement timings (e.g. "counter this Option"). Covered by **Phase 8** (Option flow).
- Token-specific replacement (e.g. "tokens can't be returned to deck — trash instead"). The generic framework handles this; the token-specific default goes in with Phase 10.

## 3. Design Principles

Every decision in this spec obeys these invariants, carried forward from the roadmap's API Design Principles:

1. **No auto-selection.** Optional replacements emit a `PendingSelection::Replacement` with both accept and decline in `valid_action_ids`. Barrier silently trashing the top of the deck is *wrong* — Barrier is a "may" effect, so even when the only legal choice is "decline the decline" the decision node must exist.
2. **Mandatory replacements do not emit a selection.** "You can't be returned to deck" is a pure restriction — it replaces the return with "nothing happens" without a player decision.
3. **Cause attribution is first-class.** A replacement may apply only for certain causes (most commonly, "opponent's effect" vs. "own effect" vs. "battle"). The framework carries this on `ReplacementContext` so scripts don't need to sniff game state.
4. **Layering follows printed rules.** When multiple replacements apply to the same event, controller-of-the-affected-card chooses the order among their own replacements first; opponent's replacements apply afterward. This matches the DCGO ordering and resolves the Barrier-vs-Evade ambiguity.
5. **Replacement windows are atomic.** A replacement that runs cannot itself be interrupted by a new triggered-effect drain; replacements run to completion (cancel / redirect / substitute) before any observer fires.
6. **Observers fire based on the **post**-replacement reality.** If Evade redirects "would be deleted" to "move to bottom of deck", then `OnDeletion` does NOT fire but `OnReturn`/`OnLeaveField` DOES fire (for whatever leaving-type is chosen). This is the behavior distinguishing Evade from Barrier: Evade avoids the "deletion" classification entirely.
7. **Replacements and Phase 6 flood gates compose.** A flood gate that makes a category of effect-driven action illegal never produces an event to replace. A replacement that cancels a specific event never needs to check flood gates. Orthogonal layers.

## 4. Timing-Family Enumeration

Added to `EffectTiming` in `digimon-engine/src/enums.rs`. Every variant below carries a `ReplacementContext` (see §6) when dispatched.

| Variant | Fires when | Replaces | Notes |
|---------|------------|----------|-------|
| `WhenWouldBeDeleted` | Permanent is about to be deleted (any cause). | Deletion → alternative route (Barrier: trash top deck; Evade: bottom-deck; Partition/ArmorPurge: delete/trash a source instead). | Most common Would timing. |
| `WhenWouldLeaveBattleArea` | Permanent is about to leave the battle area by any route — deletion, return-to-hand, return-to-deck, trash-by-effect. **Superset** of `WhenWouldBeDeleted`, `WhenWouldBeReturnedToHand`, `WhenWouldBeReturnedToDeck`, `WhenWouldBeTrashed`. | Leave → stay-on-field, or redirect to a different leaving-route. | Fires **before** the route-specific Would; see §7 ordering. |
| `WhenWouldBeReturnedToHand` | Permanent is about to be returned to hand. | Return → cancel / redirect. | |
| `WhenWouldBeReturnedToDeck` | Permanent is about to be returned to deck. | Return → cancel / redirect. | |
| `WhenWouldBeTrashed` | Card would be trashed — from battle area (route handled via leave-field too), from hand (opponent's discard), or from security by effect. | Trash → cancel / substitute. | The `target: CardHandle` on ReplacementContext distinguishes battle-area vs. hand vs. security cases. |
| `WhenWouldBeDeDigivolved` | Permanent is about to be de-digivolved (by any amount). | De-digivolve → cancel / reduce N. | `ReplacementContext::dedigivolve_count: u8` is mutable; substitute sets a lower number. |
| `WhenWouldLoseSecurity` | Security card is about to leave the stack (attacker's security check is popping it). | Loss → leave on stack, or redirect to a different zone. | Fires **before** `SecuritySkill` drains. Resolves "the attacker can't reduce your security" passive form. |
| `WhenWouldDraw` | Player is about to draw N cards. | Draw → reveal-and-choose (rare), or cancel + substitute. | `count` mutable on ctx; `cancel()` means no draw. Rarely used but present. |
| `WhenWouldPlaceInSecurity` | Card is about to be placed into the security stack by effect. | Placement → reorder (top/bottom/random), or redirect to trash. | |
| `WhenWouldAttack` | Permanent is about to declare an attack. | Attack → cancel, or redirect target. | **Reserved — Phase 9 wires dispatch.** |
| `WhenWouldBeAttackTarget` | Permanent/player is about to be declared as an attack target. | Target → cancel, redirect to another target. | **Reserved — Phase 9 wires dispatch.** Overlaps with Raid target-switch. |

### 4.1 Why a separate `WhenWouldLeaveBattleArea` *and* per-route Would timings?

Some cards are route-agnostic: "this Digimon can't leave the battle area by your opponent's effects" is a single passive modifier that must apply to deletion, return-to-hand, return-to-deck, *and* effect-driven trash. Modeling it as four parallel replacements is redundant.

Other cards are route-specific: "this Digimon can't be returned to the deck" does NOT apply to deletion. Those need the per-route timing.

So both exist. A `WhenWouldLeaveBattleArea` that runs with `ctx.cancel()` simply cancels whatever leave-route was about to fire; if it's absent, dispatch falls through to the per-route timing.

## 5. Cause Attribution

A replacement effect must be able to ask "who caused this?". The five cause types, matching DCGO's event attribution:

```rust
pub enum ReplacementCause {
    /// DP battle — attacker or defender losing.
    Battle,
    /// The affected card's controller caused it (own trash effect,
    /// self-sacrifice, own return-to-hand cost, own de-digi by trait).
    OwnEffect,
    /// The opponent's effect caused it (most common replacement trigger —
    /// "cannot be trashed by opponent's effects" flavor).
    OpponentEffect,
    /// A security check popped this card from the security stack, or a
    /// security-revealed card caused deletion via its SecuritySkill.
    SecurityCheck,
    /// Cost payment — trashed as cost, suspended as cost, etc. Rare as a
    /// replacement source but present (e.g. "you can't pay costs with
    /// Digimon on the field"). Most replacement cards don't distinguish
    /// Cost from OwnEffect, but we preserve the distinction because some
    /// C# scripts do.
    Cost,
}
```

Rust cause attribution is *derived*, not *threaded* — every state-change site that can fire a Would timing must compute the cause before dispatch. Three inference rules:

1. **Battle** — only `combat::resolve_battle` dispatches with `Battle`.
2. **SecurityCheck** — only `drive_security_resolution` / `delete_permanent_with_effects` paths that run *during* a `SecurityResolutionState` dispatch with `SecurityCheck`. Detection: `self.security_resolution.is_some()` at dispatch time.
3. **OwnEffect / OpponentEffect / Cost** — dispatched from `EffectContext` helpers. The helper knows the acting player (from `ctx.effect_source_player()`); it compares to the target's controller:
   - acting == controller → `OwnEffect`
   - acting != controller → `OpponentEffect`
   - Cost-time helpers (pay_cost_fn interior trashes) pass `Cost` explicitly.

The `acting_player` snapshot for this decision is already tracked on `EffectContext` for Phase 5/6 flood-gate dispatch and Phase 6 `source_is_tamer`. Phase 7 reuses it.

## 6. `ReplacementContext` API

Handed to every `Would*` effect process. Provides read-only access to surrounding state plus three mutating helpers.

```rust
/// Passed to Would* effect processes. Mutations shape what happens *after*
/// the replacement window closes. Outside the window, the fields are
/// snapshot values — mutating them has no effect.
pub struct ReplacementContext<'g> {
    /// The underlying effect context — all normal ctx methods (reads,
    /// selections, memory, modifier queries, source_is_tamer, …) are
    /// available. Selections installed here suspend the replacement
    /// window and resume once the selection resolves (see §9).
    pub effect: &'g mut EffectContext<'g>,

    /// What's happening.
    pub cause: ReplacementCause,

    /// The event being replaced. `Permanent` for field events;
    /// `Card` for hand/security/deck events; `Player` for player-scoped
    /// events (draws, security-placements by effect).
    pub subject: ReplacementSubject,

    /// Where the subject was about to go, if applicable. `None` for pure
    /// cancels (draws, deletions with no redirect default).
    pub original_destination: Option<Zone>,

    /// Mutation target — set by the mutating helpers below. `None` before
    /// the replacement helper is called; the dispatcher reads this back
    /// after the process returns to decide what to do.
    pub(crate) outcome: ReplacementOutcome,
}

pub enum ReplacementSubject {
    Permanent(PermanentHandle),
    Card(CardHandle, Zone),       // carrying the source zone
    Player(PlayerId),              // for draws, security placements
}

pub(crate) enum ReplacementOutcome {
    /// No replacement applied; fall through to the default route.
    None,
    /// Cancel the event entirely.
    Cancelled,
    /// Redirect to a different zone (return-to-deck instead of delete, etc.).
    Redirected(Zone),
    /// Substitute a different permanent/card as the affected subject
    /// (Partition: delete a source instead of the permanent).
    Substituted(ReplacementSubject),
    /// Run a custom alternative — the process has already mutated state
    /// via `effect` (e.g. trashed the top of the deck for Barrier) and
    /// the engine should simply skip the original event.
    CustomHandled,
}
```

Mutating helpers on `ReplacementContext`:

```rust
impl<'g> ReplacementContext<'g> {
    /// Cancel the replaced event. The state change does not commit.
    pub fn cancel(&mut self) { self.outcome = ReplacementOutcome::Cancelled; }

    /// Redirect to a different destination zone.
    pub fn redirect_to(&mut self, dest: Zone) {
        self.outcome = ReplacementOutcome::Redirected(dest);
    }

    /// Substitute a different subject. Used by Partition / ArmorPurge.
    pub fn substitute(&mut self, subject: ReplacementSubject) {
        self.outcome = ReplacementOutcome::Substituted(subject);
    }

    /// Mark the replacement as fully handled in-process. Used by Barrier
    /// (which trashes the top of the deck and skips deletion) and
    /// Fragment(N). The process is responsible for the side effects; the
    /// engine just suppresses the original event.
    pub fn handled(&mut self) {
        self.outcome = ReplacementOutcome::CustomHandled;
    }
}
```

**Why an enum `ReplacementOutcome` instead of separate bool flags?** Because the outcomes are mutually exclusive, and we want the type system to enforce "you applied exactly one of cancel/redirect/substitute/handled", not "you set two conflicting flags".

## 7. Dispatch Algorithm

### 7.1 Fire sites (exact)

Every state-change helper that can be replaced must call `try_replace` before committing the change. Listing current call-sites that must change:

| Helper | File | Current line | New Would timing(s) to fire |
|--------|------|--------------|-----------------------------|
| `delete_permanent_with_effects` | `combat.rs` | 1223 | `WhenWouldLeaveBattleArea` → `WhenWouldBeDeleted` (before existing `OnDeletion` enqueue). |
| `return_to_hand` | `game_actions.rs` | 610 | `WhenWouldLeaveBattleArea` → `WhenWouldBeReturnedToHand`. |
| `return_to_deck` | `game_actions.rs` | 658 | `WhenWouldLeaveBattleArea` → `WhenWouldBeReturnedToDeck`. |
| Battle resolution | `combat.rs::resolve_battle` | 1172 | `WhenWouldBeDeleted` fires per the pre-delete dispatch above; cause = `Battle`. |
| Security pop | `combat.rs::drive_security_resolution` | 989 area | `WhenWouldLoseSecurity` before the `Dispose` phase trashes the card. Cause = `SecurityCheck`. |
| `place_on_security` (effect-driven) | `game_actions.rs` | 1229 | `WhenWouldPlaceInSecurity` at entry. |
| `EffectContext::draw` | `effect_context/mod.rs` | (existing) | `WhenWouldDraw` at entry of the draw loop (once per draw event, not once per card). |
| `EffectContext::de_digivolve` | (forthcoming Phase 10) | — | `WhenWouldBeDeDigivolved` at entry. |
| Effect-driven trash (hand, battle, security) | multiple helpers in `effect_context/mod.rs` | various | `WhenWouldBeTrashed`. |
| Attack declaration | `combat.rs::begin_attack` | — | `WhenWouldAttack` / `WhenWouldBeAttackTarget` — **Phase 9**, site reserved. |

### 7.2 Pseudocode

```
fn try_replace(&mut self,
               timing: EffectTiming,
               subject: ReplacementSubject,
               cause: ReplacementCause,
               original_destination: Option<Zone>) -> ReplacementOutcome
{
    // 1. Collect all candidate replacements (scan registry + keywords +
    //    card-face effects at this timing that pass their condition).
    let candidates = self.collect_replacement_candidates(timing, &subject, cause);
    if candidates.is_empty() {
        return ReplacementOutcome::None;
    }

    // 2. Order by layering rule (§8). Controller-of-affected-subject's
    //    replacements first (in that player's chosen order if >1); then
    //    opponent's replacements (in opponent's chosen order if >1).
    let ordered = self.layer_replacements(candidates, &subject);

    // 3. Walk replacements. Each replacement either:
    //    a) runs unconditionally (mandatory — passive modifier), or
    //    b) installs a PendingSelection::Replacement with accept/decline
    //       branches (optional).
    //    Each replacement sees the CURRENT outcome (set by the previous
    //    one) so a stack of replacements can compound.
    for rep in ordered {
        let outcome = self.run_replacement(rep, &mut ctx);
        // If a replacement fully cancels, further replacements targeting
        // the same subject become moot — but they still fire because the
        // rules layer replacements on the post-replacement state. Example:
        // if Evade redirects to bottom-of-deck, a subsequent "cannot be
        // returned to deck" replacement can then cancel that.
        //
        // In practice: pass the current outcome through the chain, each
        // replacement can mutate further.
    }

    ctx.outcome
}
```

### 7.3 Commit step

The *caller* (e.g. `delete_permanent_with_effects`) reads the returned outcome and acts:

- `None` → commit the original event (existing code path — no change).
- `Cancelled` → skip the event entirely. Observers do NOT fire.
- `Redirected(zone)` → call the appropriate zone-mover helper (`return_to_deck(StackPosition::Bottom)` for Evade, etc.) and let *that* helper re-enter its own Would-chain if applicable (see §7.5 on recursion).
- `Substituted(subject)` → apply the original event to the substituted subject (Partition: delete the chosen source instead).
- `CustomHandled` → the replacement process already did the work; skip the original event AND skip observer dispatch for the original event.

### 7.4 Post-replacement observers

Observers are dispatched based on the *final* outcome:

- If deletion is cancelled → no `OnDeletion`, no `OnAnyDeletion`, no `OnLeaveField`.
- If deletion is redirected to bottom-of-deck → no `OnDeletion`, but `OnLeaveField` AND `OnReturn` fire (leaving-to-deck is a real return).
- If the event is substituted (Partition: the source gets deleted instead of the permanent) → the source gets full observer dispatch as if it had been directly deleted, and the permanent gets no observers.

This is the layer where "Evade is not a deletion" semantically matters. The engine enforces it in the commit step, not in the replacement.

### 7.5 Recursion and termination

`Redirected` routes must not infinite-loop. Two guards:

1. **Per-event depth counter** on `Game`: `replacement_depth: u8`. Incremented on entry to `try_replace`, decremented on exit. If depth exceeds a cap (say 8), the engine logs an error and commits the *original* event without further replacement. This is a belt-and-suspenders rule; a well-designed card set won't hit it, but the engine must not hang the RL loop.
2. **Once-per-event guard**: a replacement that has already fired for a given (timing, subject) within this call chain is skipped. This prevents a `WhenWouldLeaveBattleArea` from firing when its own redirect routes through another leave-the-field helper that would fire the same timing again.

## 8. Layering / Ordering Rules

When multiple replacements apply to the same event:

### 8.1 Controller-of-affected-subject first

The rule from the printed game: **the controller of the affected permanent chooses the order in which their own replacement effects apply, then the opponent chooses among theirs.** This matches DCGO's `ICardEffect.Priority` ordering for replacement effects.

### 8.2 Implementation

`layer_replacements(candidates, subject)` returns a `Vec<ReplacementCandidate>` in two passes:

```
own_reps  = candidates.filter(c.source_controller == subject.controller)
opp_reps  = candidates.filter(c.source_controller != subject.controller)

if own_reps.len() > 1 { emit TriggerOrder-style selection to subject.controller }
if opp_reps.len() > 1 { emit TriggerOrder-style selection to opponent  }
```

In most cases one player will have ≤1 relevant replacement and no selection is needed. In the rare multi-replacement stack (e.g. Barrier + Evade on the same Digimon), the controller gets a `PendingSelection::TriggerOrder` prompt exactly as the Phase 1 drainer does for simultaneous triggers. Reusing `TriggerOrder` here keeps the mask/decoder unchanged.

### 8.3 Multi-subject events

For events that touch multiple subjects at once (a tie in battle deleting both permanents, or a mass-deletion effect), each subject dispatches its *own* replacement window independently. The replacement for attacker does not see the replacement for defender; neither can redirect the other's outcome.

Exception: if a replacement *substitutes* a different subject (Partition substitutes a source), the substituted subject does NOT get a fresh replacement pass within the same event — only after control returns to the top-level event loop.

## 9. Optional Replacements — `PendingSelection::Replacement`

Mandatory passive replacements (`CannotBeReturnedToDeck`, `CannotBeTrashedByEffect`) run silently. Optional "may" replacements (Barrier, Evade, Decode, Partition, ArmorPurge, Fragment) **must** emit a selection with both accept and decline branches in `valid_action_ids`.

New variant on `SelectionKind`:

```rust
pub enum SelectionKind {
    // …existing variants…

    /// Player may accept or decline an optional replacement effect. The
    /// `valid_action_ids` holds exactly two entries: a stable "accept"
    /// ID and a stable "decline" ID (re-using PASS for decline where
    /// possible, else reusing EffectChoice infrastructure).
    Replacement,
}
```

### 9.1 Action-space encoding

Reuse the existing `EffectChoice` action range — no net-new action IDs:

- **Accept** — the effect runs, outcome is set by the process.
- **Decline** — `ctx.outcome` stays `None` from this replacement; the next replacement in the chain (if any) runs, and the original event commits if nothing else applies.

Concretely: `is_optional = true` on `PendingSelection::Replacement`; `PASS` (62) is the decline path. Accept fires the `callback`; decline fires `on_decline`.

### 9.2 Replacement-inside-replacement

A replacement process may install a selection (e.g. "delete *one of* the defender's sources" for Partition). The existing `PendingSelection` machinery handles it — the replacement window suspends while the nested selection is open, resumes once it resolves, and continues layering.

### 9.3 Why reuse `TriggerOrder` for multi-replacement ordering (§8.2)?

Because the semantics are identical: "you have two pending effects, pick which fires next." The only difference is payload — the drainer uses it for `QueuedEffect`s, the replacement layer uses it for replacement candidates. The UI label and action range are reusable as-is.

## 10. Interaction With Phase 6 Passive Restriction Modifiers

Phase 6 landed 13 player-scoped active flood gates (`CannotPlayDigimonByEffect`, `CannotGainMemoryByEffect`, …). A separate **passive** family was deferred to Phase 7 because it needs the replacement framework to land correctly:

| Modifier (deferred from Phase 6) | Fires as | Cause filter |
|---------------------------------|----------|--------------|
| `CannotBeReturnedToDeck` | `WhenWouldBeReturnedToDeck` → `ctx.cancel()` | `OpponentEffect` by default; some printed cards are cause-agnostic. |
| `CannotBeReturnedToHand` | `WhenWouldBeReturnedToHand` → `ctx.cancel()` | ditto |
| `CannotBeTrashedByEffect` | `WhenWouldBeTrashed` → `ctx.cancel()` | `OpponentEffect` (printed card text always specifies "opponent's"). |
| `CannotBeDeDigivolved` | `WhenWouldBeDeDigivolved` → `ctx.cancel()` | ditto |
| `CannotBeDestroyedByBattle` (exists since Phase 0) | `WhenWouldBeDeleted` → `ctx.cancel()` | `Battle` |
| `CannotBeDestroyedByEffect` (exists since Phase 0) | `WhenWouldBeDeleted` → `ctx.cancel()` | `OpponentEffect` or `OwnEffect` depending on printed text |

These modifiers have been sitting as enum variants without enforcement. Phase 7 wires them up: at `try_replace` time, the scanner walks the target's attached modifiers and installs automatic cancel-replacements for any that match (timing + cause).

**New per-modifier field required:**

```rust
pub struct ModifierEntry {
    // …existing fields…

    /// Cause filter for replacement modifiers. `None` means the modifier
    /// applies to any cause. For modifiers that apply only to opponent's
    /// effects (the common case for "cannot be X'd"), set this to
    /// `Some(ReplacementCause::OpponentEffect)`.
    pub cause_filter: Option<ReplacementCause>,
}
```

Likewise for `PlayerModifierEntry`. Builders default to `None` (cause-agnostic).

### 10.1 Closure-valued replacement condition

Some passive replacements depend on runtime state — "can't be returned to hand while another Bagramon is in play". Per §2(6), we add a closure field:

```rust
pub type ReplacementConditionFn =
    Box<dyn Fn(&EffectReadContext, &ReplacementSubject) -> bool + Send + Sync + 'static>;

pub struct ModifierEntry {
    // …existing fields…
    pub replacement_condition: Option<ReplacementConditionFn>,
}
```

This closure is evaluated at `try_replace` time; if it returns false, the replacement is skipped. The `ReplacementSubject` argument lets the closure inspect which specific permanent the event targets (necessary because one modifier can apply to multiple permanents in the area).

## 11. Security-Check Interaction

Audit-flagged open question: does Barrier fire when the revealed security card would be deleted by a security-check battle? Resolution:

**Yes — but only for `WhenWouldBeDeleted` effects whose source is *the revealed card itself* (intrinsic Barrier) or *a card already on the revealed card's controller's field*.**

Rationale: the revealed card is technically in a transient "revealed" state during `SecurityResolutionState`, not in the battle area. Its own effects are scoped by `TriggerSource::SecurityRevealed` (existing Phase 1 plumbing). A Barrier on the revealed card itself is part of the SecuritySkill window and must have a chance to fire; a Barrier granted to another defender's Digimon via an aura must also be able to fire because the defender is the owner of the security event.

Mechanically: `drive_security_resolution` at `SecurityPhase::BattleResolved` must invoke `try_replace(WhenWouldBeDeleted, …, cause = SecurityCheck)` **before** calling `delete_permanent_with_effects` on the revealed card. The cause is `SecurityCheck` specifically (not `Battle`), so "cannot be destroyed by battle" does NOT apply to security-revealed cards — matching printed rules.

## 12. Action-Space Impact

| Change | Net new action IDs |
|--------|--------------------|
| `PendingSelection::Replacement` (accept + decline) | **0** — reuses `EffectChoice` range for accept and `PASS` for decline. |
| Multi-replacement ordering via `TriggerOrder` | **0** — existing range. |
| Substitute-subject selections (Partition: pick a source) | **0** — existing `SourceSelect` range. |
| Redirect-zone selection (e.g. "return to hand OR deck — your choice") | **0** — existing `EffectChoice` range. |

**Net action-space growth: 0.** `ACTION_SPACE_SIZE` remains 2168. This is by design — replacement selections reuse the existing kinds so tensor/mask parity with Python (when that's ever needed) is unaffected.

## 13. Test Plan Preview

(Full test enumeration deferred to the plan file. High-level coverage required:)

1. **Unit — per Would timing dispatch.** One DebugRunner test per timing verifying the effect's process is invoked exactly once with the correct `cause` and `subject`.
2. **Cancel.** A cancelling replacement skips the original event and skips post-event observers.
3. **Redirect.** An Evade-style redirect fires `OnLeaveField`+`OnReturn` but NOT `OnDeletion`.
4. **Substitute.** Partition-style substitution fires `OnDeletion` for the source, not the permanent.
5. **CustomHandled.** Barrier-style trashes top-of-deck, skips original deletion, skips `OnDeletion`.
6. **Layering.** Two replacements from same controller → controller chooses via `TriggerOrder`. One per side → controller's runs first, opponent's runs against the post-replacement state.
7. **Optional accept/decline.** `PendingSelection::Replacement` with both action IDs present in mask; `on_decline` invoked when player picks PASS.
8. **Recursion cap.** Constructed loop (A redirects to B, B redirects back to A) terminates at depth 8 without hanging.
9. **Phase 6 passives migration.** `CannotBeDestroyedByEffect` now installs as an auto-cancel replacement; existing tests still pass.
10. **Cause attribution.** Own vs opponent vs battle vs security-check paths each exercised.
11. **Action-mask stability.** Mask size unchanged at 2168 under replacement prompts.
12. **Native keyword parsing.** Printed Barrier/Evade/Fragment(N)/Decode/Partition/ArmorPurge on `card_data` produces the expected replacement at registry-build time.

## 14. Open Questions (resolve during plan authoring)

1. **Does `WhenWouldLeaveBattleArea` fire for deletion-cause events that end up substituted (Partition deletes a source)?** Tentative: yes — the permanent *would* have left the battle area, then a replacement substituted. This fits the "fire before commit, commit after all replacements close" rule. Verify against a DCGO reference card that uses both a LeaveField replacement and Partition.
2. **Barrier top-of-deck cost — interaction with empty deck.** If deck is empty, does Barrier fail and fall back to normal deletion, or is it a declined replacement? Tentative: process attempts the trash, if deck is empty it's a no-op but still consumed the replacement (the "may" was accepted). Confirm against printed card text.
3. **`WhenWouldDraw` interaction with `CannotDrawByEffect` (Phase 6).** A flood gate says "cannot draw by effect" — does a `WhenWouldDraw` replacement fire before the gate is checked, or not at all? Tentative: flood gates clamp the action mask and prevent the effect from firing `draw` in the first place, so no replacement fires. But if a script bypasses the mask (it shouldn't per rule 1a), the resolver must still honor the gate.
4. **Security-check cause for replay-from-security.** If a `SecuritySkill` effect uses `play_from_security`, is the card's "leaving security" event caused by `SecurityCheck` or `OwnEffect`? Tentative: `SecurityCheck` — the reveal was what initiated the departure.
5. **Layering when a replacement grants another replacement mid-event.** Unlikely but possible. Tentative: no — replacements are collected at the start of `try_replace` and re-collection during the walk is forbidden. A script that would grant a replacement in-flight has to pre-arm it.
6. **Do we model counter-reduction (`CannotReduceOpponentSecurity` passive form) as a replacement or keep it as a Phase 6-style gate?** Tentative: gate-style, because it clamps an amount rather than replacing an event. Leave where Phase 6 placed it.

## 15. Implementation Phases (preview — not the plan itself)

The forthcoming plan file (`docs/superpowers/plans/2026-04-21-rust-engine-phase-7-would-replacements.md`) will break this spec into TDD tasks along these lines:

| Task | Scope | Files |
|------|-------|-------|
| 7.1 | Enum additions + `ReplacementContext` + `ReplacementOutcome` + new `SelectionKind::Replacement`. Pure data; no dispatch. | `enums.rs`, `selection.rs`, new `replacement.rs`. |
| 7.2 | `try_replace` core, layering, recursion guard. Unit-tested against fake effects registered in test cards. | `game.rs`, new `replacement.rs`. |
| 7.3 | Wire `WhenWouldBeDeleted` + `WhenWouldLeaveBattleArea` at `delete_permanent_with_effects`. Test Barrier, Evade, Partition, ArmorPurge, Fragment. | `combat.rs`, `effect_context/mod.rs`. |
| 7.4 | Wire the return/trash/de-digi/loses-security/draw/place-in-security Would timings at their fire sites. | `game_actions.rs`, `combat.rs`, `effect_context/mod.rs`. |
| 7.5 | Migrate Phase 6 deferred passives (`CannotBeReturnedToDeck`, `CannotBeDeDigivolved`, `CannotBeTrashedByEffect`, `CannotBeReturnedToHand`) to auto-replacement install. Add `cause_filter` + `replacement_condition` fields to `ModifierEntry`. | `modifiers.rs`, `enums.rs`, `replacement.rs`. |
| 7.6 | Native keyword parsing — Barrier, Evade, Fragment(N), Decode, Partition, ArmorPurge emitted as auto-install replacements at `CardData` build. | `card_data.rs`, `card_registry.rs`. |
| 7.7 | Docs — `RUST_ENGINE_API.md` §Phase 7 section with worked examples for each keyword; `RUST_PYTHON_PARITY.md` §7.x entries closing the replacement gaps; update roadmap at `.claude/plans/recursive-coalescing-candle.md`. | docs only. |

Reserved (no dispatch here, but enum variants reserved so Phase 9 doesn't renumber): `WhenWouldAttack`, `WhenWouldBeAttackTarget`.

## 16. Non-Goals

- Not modeling "continuous modifier layering" in the MTG CR 613 sense. Digimon TCG doesn't have that — its passives and replacements are flat.
- Not refactoring the existing observer timings (OnDeletion, OnReturn, etc.) — they stay. Would timings are *added*, not *replaced*.
- Not implementing in-flight replacement grant (§14 Q5).
- No tensor-layout changes. All replacement state is transient and doesn't observe into the RL tensor.

## 17. Verification

This is a design spec, not an implementation. Verification here = acceptance of the enum shape, the `ReplacementContext` API, the dispatch algorithm, the layering rule, the `PendingSelection::Replacement` flow, and the Phase 6 passive-modifier migration story. Once accepted, the implementation plan at `docs/superpowers/plans/2026-04-21-rust-engine-phase-7-would-replacements.md` can be authored against this spec.

**Downstream verification (once implementation begins):**

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green after each task.
2. Per-timing DebugRunner behavioral tests land green (working rule 18).
3. Barrier / Evade / Partition / ArmorPurge / Fragment / Decode keyword tests pass against printed card text parsed from `card_data`.
4. Phase 6 deferred passives (CannotBeReturnedToDeck, …) enforced end-to-end.
5. `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v` still green (mask size unchanged).
6. Re-audit one of the five archetypes (suggest TS Olympos, which has the largest Barrier surface) and confirm blocked-card count drops as projected (~60 cards unblocked cumulatively with Phase 7).
