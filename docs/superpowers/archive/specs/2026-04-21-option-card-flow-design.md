# Option Card Play Flow — Design Spec

**Date:** 2026-04-21
**Status:** Design — not yet planned or implemented. Gates Phase 8 of the Rust engine roadmap (`.claude/plans/recursive-coalescing-candle.md`).
**Goal:** Faithfully implement the Option card type — play pipeline, post-resolution trash, persistent Delay state, Plug-In (Link) attachment to Digimon, and training-aid semantics — under the curated `EffectContext` API and the no-approximations policy (CLAUDE.md rule 17).

---

## 1. Motivation

Phases 0–7 landed Digimon play, digivolve, combat, replacement effects, and the full flood-gate framework — but **Option cards are still treated as Digimon at the fire-site**. `game_actions::play_from_hand_with_cost` (line 69) pays cost, removes from hand, pushes a `Permanent` to `battle_area`, fires `OnPlay`, and stops. For a Digimon this is correct. For an Option card it's catastrophically wrong:

- The Option card sits on the battle_area indefinitely as a "permanent" — it should trash after resolving.
- No `OnUseOption` / `OptionMain` dispatch site exists. Hand-authored scripts hijack `OnPlay` as a workaround.
- Plug-In / Link cards cannot attach to another Digimon — the `Permanent` struct has no `linked_cards` slot (Python has one, audits §Permanent reference it).
- Delay cards (effects that *linger* and resolve at a later timing — e.g. "at the end of your next turn…") have no persistent state model.
- Training cards (Option cards that sit *alongside* the breeding area providing inherited effects to pre-digi) have no home zone.

The cross-archetype audits logged this as Cluster E with **~70 blocked cards** across all 5 audited archetypes:

| Archetype | Option-blocked cards |
|-----------|----------------------|
| TS Olympos | 25+ control Options (Counter, Plug-In, Delay) |
| Dark Masters | 7 Option support |
| Rocks | 11 Plug-In suite (Link + stat modifiers) |
| DNA Omnimon | 8 Option enablers |
| Medusamon | 19 (Petrification-side Options + Familiar Plug-Ins) |

Phase 6 flood gates (`CannotPlayOptionByEffect` adjacent) and Phase 7 replacement-effect plumbing (`WhenWouldBeTrashed` on security-revealed Options) both depend on Option plays following the correct pipeline. Implementing Option flow is therefore a prerequisite for correctness on 70 cards plus knock-on corrections to any hand-authored scripts that embraced the current OnPlay-as-OptionMain workaround.

## 2. Scope

**In scope:**

1. A new `OptionPlay` fire-site (`game_actions::play_option_from_hand` + `play_option_from_trash`) that pays cost, emits `OnUseOption` / `OptionMain` triggers, and disposes the card per its type (default: trash; Delay: park on field; Link: attach to target Digimon; Training: park alongside breeding).
2. An `OptionState` enum on `Game` distinguishing `Delayed { owner, effect_slot, trash_at_end_of_turn: u8 }`, `Linked { host: PermanentHandle }`, `Training { owner }` from the default "trash after resolve".
3. Extension of `Permanent` with `pub linked_cards: Vec<CardSource>` (mirrors Python) so Plug-Ins can attach sideways to a Digimon without polluting the digivolution stack.
4. New `EffectTiming::OnUseOption` (fires when any Option card is played), `EffectTiming::OnLink` (observer; fires when a card is linked to a Digimon — mirrors DCGO `WhenLinked`, load-bearing for Appmon-trait cards like BT21-053/054/059/073), and `EffectTiming::OnTrashLinkedCard` / `EffectTiming::OnUnlink` observer timings so existing cards can respond.
5. New `EffectBuilder` flags: `.option_main()`, `.delay()`, `.link(cost, digimon_filter)`, `.training()` — one per Option subtype.
6. `EffectContext::play_option(card_id)` / `trash_option(perm)` / `link_card(source, host)` helpers so effect scripts can instantiate Options at effect-init time (rare; mostly useful for "play an Option from trash" effects).
7. Mask integration — Main-phase Option plays already render in `HAND_EFFECT` range per Phase 3, but the Option-specific mask gates (color requirement already present; add: Delay activation mask bit, Link target-selection mask bit) must land here.
8. Replacement interaction — Option cards that are about to be trashed after resolution fire `WhenWouldBeTrashed` (Phase 7 fire-site extension).
9. Delay cards fire their parked effect at the correct timing (end of owner's next turn for most; end-of-turn for some). Concrete timing resolution is part of each Delay card's `.delay(timing)` declaration.
10. Linked cards trash when their host is trashed, returned, or returns to hand/deck — the `linked_cards` vec cleanup is wired into the deletion / return / trash fire-sites.
11. Serialization — `PendingOption`, `LinkedCards` expose into the JSON UI view for frontend rendering.
12. Tauri `invoke` handler surface — if Options are played via the action space, this lands automatically through `play_from_hand` routing; otherwise expose a dedicated command.

**Out of scope (deferred to later phases or permanent nonfgoals):**

- **Counter-timed Options** (e.g. Blast Digivolve Option cards) — covered by Phase 9 Combat Interrupt Completion (spec to follow). The Option trigger dispatch here does NOT handle counter-window activation.
- **Option cards that redirect into combat phases** (e.g. "when this would be trashed, battle resumes") — expressible via Phase 7 replacement framework; no new plumbing needed.
- **"Search deck for an Option and play it for free"** effects — unblocked by the `EffectContext::play_option(card_id)` helper in this phase.
- **Multi-turn Delay with per-turn triggers** (e.g. "at the start of each of your next 3 turns") — Phase 8 v1 supports "trash at end of owner's next turn" and "trash at end of owner's this-turn" only. Multi-turn is rare enough to defer; document as v1 constraint.
- **Option color requirement mutations** (e.g. "you may play Options ignoring color"). Phase 6 flood-gate inverse — add as `ModifierType::IgnoreOptionColorRequirement` if a card needs it; not in this phase's core scope.
- **Option cards with their OWN linked cards** (chained plug-ins). Not known to exist in printed rules at audit time.

## 3. Design Principles

1. **No auto-selection.** The Link-target selection, Delay-reveal choice, Training slot-assignment all surface as `PendingSelection`s. Every branch is RL-visible.
2. **Option is a flow, not a zone.** An Option card is briefly a "pending play" (during resolution) and then settles into one of four terminal states: Trashed / Delayed / Linked / Training. The `OptionState` enum is the single source of truth — zone membership (trash, battle_area, breeding sideline) follows from state, not the other way around.
3. **Reuse `Permanent` for Linked and Training states.** A Linked Option card is `card_sources.is_empty() == true, linked_cards.len() == 1` — i.e. its data lives inside its host's linked_cards slot, not as a standalone permanent. A Training Option card IS a standalone Permanent but carries `OptionState::Training` — it renders alongside the breeding area but doesn't count for field-slot limits the way battle-area Digimon do. Delayed Option cards ARE standalone Permanents that occupy a battle-area slot with `OptionState::Delayed` and are restricted from attacking / being targeted by combat.
4. **Observer timings are first-class.** `OnUseOption`, `OnTrashLinkedCard`, `OnUnlink`, and (if needed) `OnTrainingTrash` all get enum variants and dispatch sites. No hidden hook firing.
5. **Fire-site cleanliness.** `play_option_from_hand` is a dedicated entry point, not a monkey-patched branch in `play_from_hand_with_cost`. Callers that know they want to play an Option specifically call it; the action-decoder routes HAND action IDs through a single `play_from_hand_any` dispatcher that forks on `CardKind` and delegates.
6. **Phase 7 replacement interaction is respected.** When an Option's resolution would trash it, `try_replace(WhenWouldBeTrashed, Card(option_card, Zone::Hand → Trash), cause, Some(Zone::Trash))` fires. A replacement that cancels sends the card back to hand. A replacement that redirects might send it into Link/Delay state instead of trash. The trash dispose step consults the outcome.
7. **Persistence must survive selection-unwind.** A Link card that installs `PendingSelection::Target` to pick its host — and the player resolving that selection — must not leak state. The `OptionState` enum + a transient `pending_option: Option<PendingOption>` slot on `Game` (mirroring `pending_security`, `pending_attack`) provide the re-entry point.

## 4. Option Subtypes

Printed Digimon TCG has four Option-card sub-shapes. The engine distinguishes via per-effect flags on the `Effect` struct (mirroring Python's `_is_delay`, `_is_training`, etc.).

| Subtype | Post-resolution state | Characteristic timing | Examples |
|---------|------------------------|------------------------|----------|
| **Standard** | `Trashed` (default) | `OptionMain` fires once, card trashes. | Most Option cards. |
| **Delay** | `Delayed { owner, trash_at_end_of_turn }` | Main effect fires on play. Delayed effect fires at `EndOfYourTurn` (owner's next turn end in most printed cards). Card then trashes. | TS Olympos Miracle-style Delays. |
| **Plug-In (Link)** | `Linked { host: PermanentHandle }` | Main effect fires on play (may be absent for pure stat-sticks). Sideways-inherited effects fire off the host's timings. Trashes when host trashes/returns/de-digivolves. | Rocks Plug-In suite. |
| **Training** | `Training { owner }` | Main effect fires on play. Inherited effects on the Training card benefit the owner's breeding permanent. Trashes when the breeding permanent is promoted or trashed. | Training cards (DNA Omnimon, others). |

### 4.1 Standard Option

The default shape. Printed text: "Play this card. Resolve the effect. Trash it."

Engine path:
1. `play_option_from_hand(player, hand_index)` — validates phase, hand index, color requirement, flood gates.
2. Pay cost (via existing `pay_memory` + `scan_before_pay_cost_reduction`).
3. Remove card from hand; synthesize a transient `PendingOption { card_source, owner, resolution_phase }` on `Game`; do NOT push to battle_area.
4. Emit `OnUseOption` observer (global) + the card's `OptionMain` effect.
5. Drain queued effects.
6. Fire `WhenWouldBeTrashed` replacement (Phase 7 interaction). If cancelled/redirected, route accordingly; otherwise trash the card (`player.trash.push(card_source)`), clear `pending_option`.
7. `check_turn_end` (memory may have dropped during resolution).

### 4.2 Delay

Printed text: "Delay: trash this card at the end of your next turn. At that time, [effect]."

Engine path:
1. Steps 1–5 from Standard.
2. Instead of trashing (step 6), push the card to `battle_area` as a `Permanent` with `option_state = OptionState::Delayed { owner, trash_at_end_of_turn: trigger_turn }`, where `trigger_turn = game.turn_count + 1` for "end of your next turn" (most common) or `game.turn_count` for "end of this turn".
3. Delayed permanents are **excluded** from attack-target selection, digivolution targeting, and other Digimon-specific combat interactions. They are visible on the field, they count against the field-slot limit, and they can be the subject of generic targeting (e.g. "trash an opponent's card on the field") unless the delayed effect text excludes that class.
4. At `EndOfYourTurn` dispatch, for every Delayed permanent whose `trash_at_end_of_turn == game.turn_count`: fire the card's delayed effect (stored as `EffectTiming::DelayEffect`), then `delete_permanent` via the existing Phase 7 replacement path (so `WhenWouldBeTrashed` can fire).

### 4.3 Plug-In / Link

Printed text: "Link (Cost: N): attach this card to one of your Digimon that meets [condition]. Sideways-inherit the following effects while attached."

Engine path:
1. `play_link_from_hand(player, hand_index)` or unified `play_option_from_hand` with dispatch on effect flag.
2. Validates the card has at least one eligible host Digimon. If not, the action is masked out.
3. Pay link cost (may differ from the card's printed cost — Link cards carry two costs: play-cost and link-cost; printed convention is "play this card, then link with X memory"). For v1, treat link cost as the sole cost (most printed Link cards don't have a separate play phase — they're played AS linking).
4. Install `PendingSelection::OppField` or `OwnField` for the host — with the card's `digimon_filter` closure.
5. On resolve: the selection callback moves the CardSource into the chosen host's `linked_cards: Vec<CardSource>`. The card does NOT become a standalone Permanent.
6. Emit `OnLink` observer timing (global — either-controller observer; mirrors DCGO `WhenLinked`). This is what Appmon-trait cards like BT21-053 ("Syakomon") / BT21-054 / BT21-059 / BT21-073 listen on to fire their "when this Digimon gains a linked card" effects. Main effect of the Link card (`OptionMain` — if declared) fires BEFORE `OnLink`.
7. Sideways-inherited effects on the Link card now contribute to the host's effect scan per `inherited=true` (already in `Effect` struct) with a new flag `linked=true` indicating sideways inheritance.

**Cleanup:**
- When the host is deleted / returned to hand / returned to deck: each linked card is trashed (same owner's trash), firing `OnTrashLinkedCard`.
- When the host de-digivolves through the stack level where the Link card was attached (rare — Link cards don't enter the stack, but a Link card with a "level requirement" may auto-detach if the host drops below it — check printed rules).
- When a replacement or effect explicitly removes the Link (via `EffectContext::unlink(host, linked_index)`): the CardSource moves to owner's trash, fires `OnUnlink`.

### 4.4 Training

Printed text: "Training: while this card is on the field alongside your breeding area, the Digimon in your breeding area gains [effect]."

Engine path:
1. Same pay-cost + effect-fire flow as Delay.
2. Card pushes to `battle_area` with `option_state = OptionState::Training { owner }`.
3. Training permanents are excluded from combat (can't be attacked, can't attack, can't be blocked).
4. Training permanent's effects are scanned alongside the breeding permanent's own effects — e.g. on OnHatch the breeding permanent's `WhenDigivolving` observer set includes the Training card's effects via an extra scan.
5. Cleanup:
   - When the breeding permanent is hatched (promoted to battle_area): Training card trashes. Fires `OnTrainingTrash` (new observer variant) so cards can react.
   - When the breeding permanent is de-digivolved to Lv.2 (egg): Training card persists.
   - When owner's turn ends and breeding area is empty: Training card persists (awaits a new egg).

## 5. New Types

### 5.1 `OptionState` — on `Permanent`

```rust
/// Additional state a Permanent carries when its top card is an Option.
/// For Digimon/Tamer/DigiEgg permanents this is always `Standard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionState {
    /// Not an Option, OR an Option mid-resolution with no lingering state.
    Standard,
    /// Delay card parked on the field; fires delayed effect at `trash_at_end_of_turn`.
    Delayed { owner: PlayerId, trash_at_end_of_turn: u16 },
    /// Plug-In card attached to `host`. Not a standalone Permanent —
    /// lives inside host.linked_cards. Reserved here for future "linked
    /// permanent" queries (e.g. `is_linked`); a Linked card's own
    /// Permanent struct is never constructed.
    Linked { host: PermanentHandle },
    /// Training card parked alongside the owner's breeding area.
    Training { owner: PlayerId },
}
```

Field added to `Permanent`:
```rust
pub option_state: OptionState,            // Default: Standard
pub linked_cards: Vec<CardSource>,        // Phase 8 addition — Plug-Ins attached to this Digimon
```

Both `Default::default()` for `Permanent` must preserve existing behavior (Standard + empty linked_cards). Grep all `Permanent::new(...)` call sites and verify.

### 5.2 `PendingOption` — on `Game`

Transient state for an Option mid-resolution. Mirrors `PendingSecurity` / `PendingAttack`.

```rust
/// Transient state set at the start of `play_option_from_hand` and cleared
/// after the resolve-and-dispose sequence finishes (or after a selection
/// unwind re-enters it). Carries the card being played so effect closures
/// can reference it via `ctx.source_card`.
#[derive(Debug, Clone)]
pub struct PendingOption {
    pub owner: PlayerId,
    pub card: CardSource,
    pub resolution_phase: OptionResolutionPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionResolutionPhase {
    /// Drain main effect + OnUseOption observers.
    MainEffectDrain,
    /// Disposed in progress — trashing, parking as Delay, attaching as Link.
    Disposing,
    /// Link-specific: waiting for host selection to resolve.
    LinkSelectHost,
    /// Done; clears `pending_option`.
    Done,
}
```

Field added to `Game`:
```rust
pub(crate) pending_option: Option<PendingOption>,
```

### 5.3 `EffectTiming` additions

```rust
// Phase 8 option timings
/// Fires when any Option card is played (global observer). Used by
/// cards that react to "any Option played by either player".
OnUseOption,

/// The main effect of an Option card. This is the "when you play this"
/// body of the card's text. Set via `.option_main()` on `EffectBuilder`.
/// At dispatch time, fires once when the Option enters the resolution
/// pipeline (before `OnUseOption`).
OptionMain,

/// Fires when the Option's delayed body resolves (at end of owner's
/// next turn for most printed cards). Set via `.delay(trigger)` on
/// `EffectBuilder`.
DelayEffect,

/// Observer: fires when a linked card is attached to a host Digimon.
/// Global — either controller's cards can observe. Mirrors DCGO's
/// `WhenLinked` timing (ICardEffect.cs:992). Load-bearing for Appmon-
/// trait "when this Digimon gains a linked card" effects — e.g.
/// BT21-053 (Syakomon), BT21-054, BT21-059, BT21-073, AD1-005.
OnLink,

/// Observer: fires when a linked card is trashed from its host
/// (opponent effect, host deletion cascade, etc.). Mirrors DCGO's
/// `OnLinkCardDiscarded` (ICardEffect.cs:996).
OnTrashLinkedCard,

/// Observer: fires when a linked card is cleanly unlinked (returned to
/// hand/trash without its host leaving play). Rare.
OnUnlink,

/// Observer: fires when a Training card is trashed (either because the
/// breeding permanent promoted, or because of an opponent effect).
OnTrainingTrash,
```

### 5.4 `EffectBuilder` flag additions

```rust
impl EffectBuilder {
    /// Mark this effect as the Option's main body. Builder shortcut for
    /// `.timing(EffectTiming::OptionMain)`.
    pub fn option_main(mut self) -> Self {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.option_main = true;
        self
    }

    /// Mark this effect as a Delay body. `trigger` determines when the
    /// delayed effect fires.
    ///
    /// - `DelayTrigger::EndOfYourNextTurn` — most common; fires at
    ///   `EndOfYourTurn` on the owner's turn counted one step after the
    ///   play (this turn if enemy played, next turn if owner played).
    /// - `DelayTrigger::EndOfThisTurn` — fires at the end of the SAME
    ///   turn it was played.
    pub fn delay(mut self, trigger: DelayTrigger) -> Self {
        self.inner.timing = EffectTiming::DelayEffect;
        self.inner.delay_trigger = Some(trigger);
        self
    }

    /// Mark this effect as a Plug-In Link body. `cost` is the link memory
    /// cost (may differ from the card's printed play_cost — most Link
    /// cards list their link cost separately).
    ///
    /// `digimon_filter` picks eligible host Digimons. The host must be an
    /// own-field Digimon; the filter narrows further (e.g. trait: Machine).
    pub fn link<F>(mut self, cost: u16, digimon_filter: F) -> Self
    where
        F: Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static,
    {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.link_cost = Some(cost);
        self.inner.link_filter = Some(Box::new(digimon_filter));
        self
    }

    /// Mark this effect as a Training body. The Option card parks
    /// alongside the breeding area and provides its inherited effects to
    /// the breeding permanent.
    pub fn training(mut self) -> Self {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.training = true;
        self
    }
}
```

New fields on `Effect`:
```rust
pub option_main: bool,
pub delay_trigger: Option<DelayTrigger>,
pub link_cost: Option<u16>,
pub link_filter: Option<Box<dyn Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static>>,
pub training: bool,
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayTrigger {
    EndOfYourNextTurn,
    EndOfThisTurn,
}
```

## 6. Fire-Sites

### 6.1 `play_option_from_hand`

Signature:
```rust
pub fn play_option_from_hand(
    &mut self,
    player_id: PlayerId,
    hand_index: usize,
) -> OptionPlayResult;

pub enum OptionPlayResult {
    /// Option played and resolved; card went to trash.
    Trashed,
    /// Option played and parked as Delay.
    Delayed(PermanentHandle),
    /// Option linked to a host.
    Linked { source: PermanentHandle },
    /// Option parked as Training.
    Training(PermanentHandle),
    /// Resolution paused on a PendingSelection (e.g. Link host selection).
    /// `pending_option` holds the resolution state; caller drives the
    /// selection and the engine re-enters on resolve.
    Pending,
    /// Invalid action (wrong phase, unaffordable, flood-gated, etc.).
    Invalid,
}
```

Body:
1. Validate `player_id`, `hand_index`, `game.current_phase == Main`.
2. Resolve card from hand. Assert `card_kind == CardKind::Option`.
3. Check color requirement (already in `action::mask`; re-check here for resolver backstop).
4. Check Phase 6 flood gates (`CannotPlayDigimonByEffect` does NOT apply; there may be a future `CannotPlayOption*` variant; if so, check it).
5. Compute play cost via `scan_before_pay_cost_reduction(player_id)` — same codepath as Digimon plays.
6. `pay_memory(effective_cost)`. If unaffordable → `OptionPlayResult::Invalid`.
7. Remove the card from hand. Install `pending_option = Some(PendingOption { owner, card, resolution_phase: MainEffectDrain })`.
8. Enqueue `OnUseOption` (global observer) and `OptionMain` effects of the card.
9. Drain effect queue. If a selection installed (Link host pick, target selection, etc.), set `pending_option.resolution_phase = LinkSelectHost` (or MainEffectDrain for mid-resolution), return `OptionPlayResult::Pending`.
10. Once drain completes with no pending selection, dispatch by subtype:
    - Link → `attach_linked_card(host)` (host picked via earlier selection callback).
    - Delay → push Permanent with `Delayed { owner, trash_at_end_of_turn }`; return `Delayed(handle)`.
    - Training → push Permanent with `Training { owner }`; return `Training(handle)`.
    - Standard → `try_replace(WhenWouldBeTrashed, ...)` → if None: owner.trash.push(card); return `Trashed`.
11. Clear `pending_option`. `check_turn_end`.

### 6.2 `play_option_from_trash`

Analogous; called from effects like "play an Option from your trash for free". Same flow, source zone = trash, `PlaySource::ByEffect`.

### 6.3 `activate_delayed_option` (end-of-turn)

Fires from `end_of_turn` dispatch. Iterates `active_player.battle_area` looking for `Permanent` whose `option_state == OptionState::Delayed { trash_at_end_of_turn == game.turn_count }`. For each match:
1. Enqueue the card's `DelayEffect`-timed effects.
2. Drain.
3. `delete_permanent_with_cause(handle, ReplacementCause::Cost)` — cause=Cost because the card is "paying itself off" by trashing. Phase 7 replacement may cancel (rare — most Delay cards are self-inflicted and no replacements match).
4. Fire `OnTrainingTrash` if the card was actually Training — no, that's a separate cleanup path; Training doesn't use this.

### 6.4 `attach_linked_card` (Link resolution)

Called from the Link selection's callback after host is picked:
1. Validate host handle is still valid (the selection may have parked for a long time; the host may have left play — in which case the Link action aborts and the card goes to trash per printed rules).
2. Move CardSource into `host_permanent.linked_cards`.
3. Fire the card's `OptionMain` body (which is the "on play/link" effect per printed convention) + emit `OnLink` observer globally (both players' battle areas scan for `OnLink`-timed effects; Appmon-trait cards on either side can react).
4. Clear `pending_option`.

### 6.5 Linked-card cleanup — deletion / return cascade

In `combat::commit_permanent_deletion` and `game_actions::return_to_*`:
- Before clearing the permanent, for each `linked_card` in `perm.linked_cards`:
  - Push to owner's trash.
  - Fire `OnTrashLinkedCard` observer.
- Drop the permanent.

### 6.6 Training-card cleanup — on breeding promotion

In the breeding-area-to-battle-area promotion (hatch path):
- For each Training permanent the owner has parked: fire `OnTrainingTrash`, then `delete_permanent_with_cause(handle, Cost)`.

## 7. Action-Space & Mask

### 7.1 Mask

Option plays are already emitted into the `HAND_EFFECT` range (30..60) for the Option's hand index, gated by color requirement (existing). Phase 8 adds:

- **Link host selection** — when the Option's main action resolves, the host selection is a standard `SelectionKind::OwnField` / `OppField` prompt. Reuses existing field-selection mask slots. Zero new action IDs.
- **Delay activation** — delay firing is **not an action**; it's a dispatch at end of turn. No mask changes.
- **Training host** — breeding-area is always a single slot per owner. No host-selection needed.

**Net action IDs added: 0.** `ACTION_SPACE_SIZE` remains 2168.

### 7.2 Decoder

The existing `decode_main` branch for `HAND_EFFECT` actions (hand-index play) dispatches by card kind. Currently every kind routes to `play_from_hand`. Phase 8 splits:

```rust
match card.card_kind(&game.card_data) {
    CardKind::Digimon => game.play_from_hand(player, hand_index),
    CardKind::Tamer   => game.play_from_hand(player, hand_index), // stays as Permanent
    CardKind::Option  => {
        match game.play_option_from_hand(player, hand_index) {
            OptionPlayResult::Invalid => /* error */,
            _ => Ok(()),
        }
    }
    CardKind::DigiEgg => /* breeding-only; invalid in main */,
    CardKind::Token => /* not playable from hand */,
}
```

Tamer cards keep the current flow (play as Permanent to battle_area, no trash). Digimon keep the current flow. Option branches to the new dispatcher.

## 8. Cross-Phase Interactions

### 8.1 Phase 6 flood gates

- `CannotPlayDigimonByEffect` does NOT apply to Options (already true — the gate checks `CardKind::Digimon`).
- Potential new variant: `CannotPlayOptionByEffect`. **Not in Phase 8 core scope** but add enum variant with empty enforcement if a card surfaces in audits; wiring lives in the existing flood-gate pipeline.
- `CannotReducePlayCost` applies to Option plays identically (already true — the scan is cost-type-agnostic).

### 8.2 Phase 7 replacement framework

- **Standard Option trash** fires `WhenWouldBeTrashed` for `Card(option_card, Zone::Hand)` subject, cause = Cost. A replacement that cancels sends the card back to hand (rare, but expressible). A replacement that redirects to Delay / Training state is **not supported in v1** (no printed card does this).
- **Delay dispose** at end-of-turn also fires `WhenWouldBeTrashed`, cause = Cost.
- **Linked-card host deletion** currently fires `OnTrashLinkedCard` after trashing the linked card. To compose with Phase 7: the trash step should go through `try_replace(WhenWouldBeTrashed, Card(linked_card, Zone::BattleArea), OpponentEffect | OwnEffect, Some(Zone::Trash))`. A replacement that cancels keeps the linked card attached to… nothing? The host is gone. V1 behavior: if the host leaves the battle area, the linked card unconditionally trashes (replacement does NOT fire). Document as a v1 constraint with a `TODO(phase-9-or-followup)`.
- **WhenWouldBeTrashed** cause attribution: for Option-resolution trash, cause is `Cost`; for linked-card trash on host deletion, cause is `Own/OpponentEffect` (same as the deletion cause — the linked card inherits the deletion's cause).

### 8.3 Phase 10 tokens

No interaction. Tokens are CardKind::Token, can't be played from hand as Options.

### 8.4 Phase 9 combat interrupts

- Counter-timed Options (Blast Digivolve, `<Counter>` Options) are a Phase 9 problem. Phase 8 can LAND the `OptionMain` dispatch now; the Counter window (when during an opponent's attack) is wired in Phase 9.
- Blast Digivolve Options currently use `.blast_digivolve()` on `EffectBuilder` (Phase 2 feature) — they get picked up from the hand during CounterTiming. The play flow is special (not through play_from_hand). Phase 8 does not touch this path.

## 9. Test Plan Preview

Full test enumeration deferred to the plan file. High-level coverage required:

1. **Unit — `OptionPlayResult`.** Each of the 5 result variants returned correctly for the corresponding scenario.
2. **Standard Option trashes after resolve.** `OptionMain` fires once, `OnUseOption` fires, card goes to trash.
3. **Standard Option's `OnUseOption` observer fires for opponent-played Options.** Cross-player visibility.
4. **Delay parks on field.** After play, `option_state == Delayed`, permanent in battle_area, `battle_area.len() += 1`.
5. **Delay fires at end-of-next-turn.** `DelayEffect` resolves, card trashes, `OnAnyDeletion` fires.
6. **Delay with `EndOfThisTurn` trigger.** Fires at end of the SAME turn it was played.
7. **Delay card excluded from attack targeting.** Attack mask does not emit actions targeting Delayed permanents.
8. **Link card installs host-selection.** `pending_option.resolution_phase == LinkSelectHost` after main drain; `PendingSelection::OwnField` installed.
9. **Link attaches to chosen host.** CardSource moved into `host.linked_cards`; Option card not standalone on field.
10. **Linked-card trashes on host deletion.** `OnTrashLinkedCard` fires, linked card in owner's trash.
11. **Linked-card survives host de-digivolve unless level constraint violated.** Printed rules vary; v1 keeps attached unconditionally.
12. **Training card parks alongside breeding.** `option_state == Training`, attack masks exclude.
13. **Training card trashes on breeding promotion.** `OnTrainingTrash` fires.
14. **Training card persists if breeding empty.**
15. **Color-requirement gate.** Option hand-index mask bit is 0 if no matching-color Digimon on field or breeding.
16. **Cost-reduction closure (Phase 5) applies to Option plays.** A `.cost_reduction_fn` on a field effect reduces Option play cost via the same `scan_before_pay_cost_reduction` call.
17. **Phase 7 WhenWouldBeTrashed fires for Option self-trash.** Install a trash-replacement on the Option's owner; Option card gets cancelled-trash (returns to hand per redirect).
18. **`OptionMain` executes under the same TDD harness as other timings.** `DebugRunner` scenarios are recordable and replayable.
19. **End-to-end behavioral test.** A TS Olympos Counter Option + a Dark Masters Delay Option play concurrently in one game, exercising the full Option flow.
20. **Parity test (if desired).** Python's `_option_stays_on_field` and `_trash_option_after_resolution` paths produce the same end state for Standard, Delay, and Link scenarios — validates via a bounded headless-game comparison.

## 10. Open Questions (to resolve during plan authoring)

1. **Link cost vs. play cost distinction.** Printed Link cards sometimes list "Play: X" and "Link: Y" as separate costs. Phase 8 v1: the play cost is the printed value; the link cost is an OPTIONAL secondary cost paid WHEN attaching. Confirm by sampling a handful of printed Link cards (Rocks, Medusamon Familiar).
2. **Delay cards that last multiple turns.** The `DelayTrigger` enum in §5.4 has two variants. Are there printed cards requiring `EndOfYourNextNTurns(u8)` or conditional triggers? Audit.
3. **Option cards with no main effect.** Some Plug-Ins are pure stat sticks — attach, grant +N DP and a keyword while attached. No `OptionMain` timing needed. Ensure the `.link()` flow doesn't require a `.process()` on the same effect.
4. **`OnUseOption` for both players or just controller?** Audits suggest global — any card on either side reacting to an Option play. Confirm vs. DCGO `ICardEffect.OnUseOption` shape.
5. **Training card sideways-inheritance scope.** A Training card on the field: do its inherited effects flow to the breeding permanent immediately, or only when the breeding permanent engages its OWN effect timings (OnHatch, WhenDigivolving)? Per printed rules, the latter. Verify in the breeding-scan patch.
6. **Field-slot counting.** Do Delayed / Training Option permanents count against `rules.field_slots` (14 for standard)? Printed rules say they DO occupy a slot. Confirm and respect.
7. **Option card with Barrier / replacement keyword.** Not known to exist in printed rules (replacement keywords are on Digimon). If one did exist on a Delay Option, the Phase 7 framework would need to fire `WhenWouldBeDeleted` against the Delayed permanent. V1 doesn't introduce any new interaction; just confirm the existing Phase 7 fire-site covers Delayed permanents as regular battle-area permanents.

## 11. Implementation Phases (preview — not the plan itself)

| Task | Scope | Files |
|------|-------|-------|
| 8.1 | Enum + data types: `OptionState` on `Permanent`, `PendingOption` on `Game`, `EffectTiming::OnUseOption/OptionMain/DelayEffect/OnLink/OnTrashLinkedCard/OnUnlink/OnTrainingTrash`, `DelayTrigger`, `.option_main() / .delay() / .link() / .training()` builders. Pure data; no dispatch. | `enums.rs`, `permanent.rs`, `game.rs` (field), `selection.rs`, `effect.rs`. |
| 8.2 | `play_option_from_hand` + `play_option_from_trash` + dispatch in decoder. Standard Option flow only (trash after resolve); no Delay/Link/Training. | `game_actions.rs`, `action/decode.rs`. |
| 8.3 | Delay flow. `EndOfYourTurn` dispatch scans `OptionState::Delayed` and fires `DelayEffect`. | `game_actions.rs`, `game_phases.rs`. |
| 8.4 | Link flow. `.link()` builder wires host selection; `attach_linked_card` on resolve. Extends Permanent with `linked_cards` Vec. Cleanup hooks on deletion / return. | `game_actions.rs`, `combat.rs`, `permanent.rs`, `effect_context/selections.rs`. |
| 8.5 | Training flow. `OnTrainingTrash` observer + breeding-promotion hook. Sideways-inheritance scan from Training cards to the breeding permanent at breeding-effect timings. | `game_actions.rs`, `game_phases.rs`, `effect_queue.rs`. |
| 8.6 | Phase 7 replacement integration: `WhenWouldBeTrashed` fires for Option self-trash, Delay end-of-turn trash. Document the v1 constraint that linked-card host-deletion trash does NOT fire the replacement. | `game_actions.rs` / `combat.rs` trash helpers. |
| 8.7 | Docs — `RUST_ENGINE_API.md` §Phase 8 section (Option subtypes + builders + worked examples), `RUST_PYTHON_PARITY.md` §8 entry closing Option-flow gaps, roadmap flip. | docs only. |
| 8.8 | Behavioral end-to-end: TS Olympos Counter Option into a Dark Masters Delay Option; verify state transitions across turn boundaries. | `tests/option_flow/behavioral_end_to_end.rs`. |

## 12. Non-Goals

- No changes to Tamer card flow (already correct — Tamers are Permanents that stay on the field).
- No changes to the tensor layout (the Option states observe through existing Permanent tensor slots; `linked_cards.len()` is a Phase 9 tensor addition if needed, but v1 Phase 8 relies on the existing field-slot observability).
- No new mask slots. `ACTION_SPACE_SIZE` stays 2168.
- No Counter-timed Option activation (Phase 9).
- No nested `PendingSelection::Source` inside Option-main processes (same limitation as Phase 7 — cards that want "pick which source of your Digimon to trash as an Option cost" are blocked by the same infrastructure gap).
- No multi-turn Delay beyond "next turn end" (1 turn lookahead). Multi-turn cards (if they exist in printed pool) are flagged BLOCKED until a follow-up.
- No `WhenWouldLink` replacement timing. DCGO has it (ICardEffect.cs:991) — a pre-link replacement window that could cancel or redirect a link. No printed card in the audited pool uses it; deferred as a Phase 7-style follow-up if a real use surfaces. (`OnLink` post-attach observer IS in scope — mirrors DCGO `WhenLinked` and is required by Appmon-trait cards.)

## 13. Verification

This is a design spec, not an implementation. Verification = acceptance of:
- The `OptionPlayResult` shape + the 5 result variants.
- The `OptionState` enum decomposition — Standard / Delayed / Linked / Training.
- The `linked_cards: Vec<CardSource>` extension on `Permanent`.
- The 6 new `EffectTiming` variants.
- The 4 new `EffectBuilder` flag helpers.
- The `pending_option: Option<PendingOption>` transient state slot on `Game`.
- The mask-level decision: zero new action IDs.
- The Phase 7 interaction rules (§8.2).
- The v1 limitations called out in §§2, 3, 10.

**Downstream verification** (once implementation begins per-task):

1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full suite green after each task.
2. Per-task DebugRunner behavioral tests land green (working rule 18).
3. Re-audit one archetype (Rocks — heaviest Plug-In user) after the Link task and verify Plug-In-blocked cards drop from the gap log.
4. `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v` — should remain green (tensor/mask shape preserved).
5. Parity sample: pick 3 printed Option cards (1 Standard, 1 Delay, 1 Link), author both Python and Rust scripts, compare end-state across a 20-turn DebugRunner scenario. End states match.
