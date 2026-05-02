# Rust Engine API Reference

**Audience:** AI agents (and humans) implementing Digimon card effects in Rust against `digimon-engine`.

This document is the canonical scripting reference. Before writing any card effect, read this in full. The engine intentionally exposes a curated API (`EffectContext`); do not reach around it into `Game` internals.

---

## 1. Project layout

```
code/digimon-engine/
├── src/
│   ├── lib.rs                  # Re-exports the public API
│   ├── game.rs                 # Game state + turn machine
│   ├── player.rs, permanent.rs, card_source.rs   # Zones / permanents / card instances
│   ├── card_data.rs            # Static card metadata loaded from cards.json
│   ├── rules.rs                # Rules presets (standard, edh, titan_boss, titan_team)
│   ├── enums.rs                # Phase, Timing, Keyword, Modifier, Expiry, Zone, Color
│   ├── effect.rs               # Effect + EffectBuilder + CardEffect trait
│   ├── effect_context.rs       # EffectContext — THE card-scripting API
│   ├── modifiers.rs            # ModifierRegistry (typed modifiers + expiry)
│   ├── combat.rs               # Attack / battle / security
│   ├── observation.rs          # Observation profile selection and tensor dispatch
│   ├── tensor.rs               # Compact compatibility tensor (standard_compact_v1, 1375 floats)
│   ├── tensor_v2_lite.rs       # Default pilot observation writer (standard_lite_v2, 8320 floats)
│   ├── tensor_profiles/        # Profile layout metadata and card/scalar positions
│   ├── action/                 # Action space + mask (2168 actions, matches Python)
│   ├── cards.rs                # CardEffectRegistry + registration glue
│   ├── cards/test_cards.rs     # TEST-001..005 — hand-written examples
│   └── debug_runner.rs         # Deterministic test harness
└── tests/                      # Integration tests (engine_core, tensor_and_mask,
                                # test_cards_behavioral, combat_scenarios)
```

Each card's effect script lives in `src/cards/<set>/<card_id>.rs`, implements
the `CardEffect` trait, and registers itself via the set's `register` fn.

---

## 2. Writing a card effect

Every card's effects live in a struct that implements `CardEffect`:

```rust
use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword};

/// BT1-010: "On Play: Gain 2 memory. When Digivolving: Draw 1."
pub struct Bt1010;

impl CardEffect for Bt1010 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card)
                .name("Gain 2 memory")
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::when_digivolving(card)
                .name("Draw 1")
                .process(|ctx| {
                    let me = ctx.player;
                    ctx.draw(me, 1);
                })
                .build(),
        ]
    }
}

pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("BT1-010", Arc::new(Bt1010));
}
```

Key rules:

- One struct per `card_id`. The struct is zero-sized (`pub struct Foo;`).
- `effects()` returns **all** of that card's effects — OnPlay, WhenDigivolving, inherited, security, etc.
- Closures in `process(...)` and `condition(...)` are `Fn + Send + Sync + 'static`. They capture `Copy` handles (`CardHandle`, `PermanentHandle`, `PlayerId`) — never borrow.
- Register in the set's `register(&mut CardEffectRegistry)` function.

### Effect builder constructors

| Constructor | Timing flag set | When it fires |
|-------------|-----------------|---------------|
| `Effect::on_play(card)` | `on_play` | When played from hand |
| `Effect::when_digivolving(card)` | `when_digivolving` | Played on top of another permanent |
| `Effect::on_attack(card)` | `on_attack` | When the permanent declares an attack |
| `Effect::on_deletion(card)` | `on_deletion` | Before the permanent is deleted |
| `Effect::inherited(card)` | `inherited` | Always — carried up the stack to the top card |
| `Effect::security(card)` | `security` | Revealed in the opponent's security check |
| `Effect::declarative(card)` | `declarative` | Passive / always-on |

### Builder methods

- `.name("human label")` — used for logs and UI.
- `.condition(|ctx| bool)` — gate the effect. If false, `process` is skipped.
- `.process(|ctx| { ... })` — the mutation body.
- `.optional()` — player may decline (UI prompt).
- `.once_per_turn()` — `max_per_turn = 1`.
- `.timing(EffectTiming::...)` — override the timing enum (rare).
- `.dp_modifier(n)` — static DP buff (for declarative effects).
- `.cost_reduction(n)` — static cost reduction.
- `.build()` — finalize into `Effect`.

---

## 3. `EffectContext` API

The context passed into `condition` and `process` closures.

### Fields

- `ctx.game: &mut Game` — escape hatch (use sparingly).
- `ctx.source_card: CardHandle` — the card whose effect is resolving.
- `ctx.source_permanent: Option<PermanentHandle>` — the permanent containing that card, if any.
- `ctx.player: PlayerId` — the controller of the source. Use this, not hardcoded player IDs.

### Read-only queries

```rust
ctx.memory() -> i16
ctx.turn_count() -> u16
ctx.rules() -> &Rules
ctx.card_data() -> &[CardData]

ctx.player(id) -> &Player        // any player
ctx.my_player() -> &Player        // ctx.player's player
ctx.opponent_id() -> PlayerId     // first clockwise opponent
ctx.opponent() -> &Player
ctx.opponents() -> Vec<PlayerId>  // all non-eliminated opponents

ctx.battle_area(id) -> &[Permanent]
ctx.hand(id) -> &[CardSource]
ctx.trash(id) -> &[CardSource]
ctx.security_count(id) -> usize

ctx.source_permanent() -> Option<&Permanent>
```

### Memory

```rust
ctx.gain_memory(amount: i16)
ctx.lose_memory(amount: i16)
ctx.set_memory(value: i16)
```

Memory is the seesaw — positive favors the active player, negative crosses into the opponent's turn. Use `gain_memory(n)` and `lose_memory(n)`; the engine clamps to `rules.memory_range`.

### Card flow

```rust
ctx.draw(player: PlayerId, count: u8) -> u8        // returns cards actually drawn
ctx.trash_from_top(player: PlayerId, count: u8) -> u8
```

### Field mutations

```rust
ctx.delete_permanent(target: PermanentHandle)
ctx.suspend(target: PermanentHandle)
ctx.unsuspend(target: PermanentHandle)
```

`delete_permanent` removes the permanent and moves all cards in its stack to trash. It also clears modifiers attached to that handle. **Does not fire OnDeletion** — use `Game::delete_permanent_with_effects` for that when you're calling from combat paths. From a card script, `ctx.delete_permanent` is usually what you want (OnDeletion is handled by combat, not effect).

### Modifiers

```rust
ctx.add_dp_modifier(target: PermanentHandle, value: i32, expiry: Expiry)
ctx.add_modifier(target, modifier: ModifierType, value: i32, expiry: Expiry)
ctx.grant_keyword(target, keyword: Keyword, expiry: Expiry)
```

See §5 for `ModifierType` and `Expiry` values.

### OnDeletion cause accessors

Inside an `OnDeletion` (or `OnAnyDeletion`) observer body, the cause of the deletion currently being drained is exposed on the context. Outside such a body all three accessors return `None` / `false`. Phase B §B5.

```rust
ctx.deletion_cause() -> Option<ReplacementCause>   // raw cause: Battle / OwnEffect / OpponentEffect / SecurityCheck / Cost
ctx.was_deleted_by_effect() -> bool                 // matches OwnEffect | OpponentEffect
ctx.was_deleted_by_opponent() -> bool               // matches OpponentEffect
```

The same trio is mirrored on `EffectReadContext` for use inside `condition` closures.

**Retaliation — fire only on battle deletion:**

```rust
Effect::on_deletion(card)
    .name("Retaliation: delete the winner")
    .condition(|ctx| ctx.deletion_cause() == Some(ReplacementCause::Battle))
    .process(|ctx| { /* delete the battling opponent Digimon */ })
    .build()
```

**Mephistomon — "when this is deleted by your opponent's effect":**

```rust
Effect::on_deletion(card)
    .name("[On Deletion] Opponent-effect rider")
    .condition(|ctx| ctx.was_deleted_by_opponent())
    .process(|ctx| { /* play 1 [Gulfmon] / level-6 Dark Masters from hand or trash free */ })
    .build()
```

**Scapegoat — eligibility predicate:**

`Keyword::Scapegoat` cancels deletion when the cause is *not* `OwnEffect` (RULES_CONTEXT 16-31). `was_deleted_by_effect()` is too broad for this — it matches `OwnEffect` too — so use the raw cause:

```rust
.condition(|ctx| !matches!(
    ctx.deletion_cause(),
    Some(ReplacementCause::OwnEffect),
))
```

The slot is populated by `Game::current_deletion_cause`, set by the deletion fire-site for the duration of the OnDeletion drain and cleared once the queue is empty. See `code/code/digimon-engine/tests/combat/deletion_cause_observer.rs` for the canonical regression.

### Replacement-process outcome-setters

Inside a `WhenWouldBe*` replacement-process closure, after installing a
nested player selection via `ctx.select_*`, the user's callback body
sets the replacement outcome via one of these methods:

- **`ctx.cancel_leave()`** — Cancel the original event (Save, Fragment).
  ```rust
  ctx.cancel_leave();
  ```

- **`ctx.handle_replacement()`** — Mark as custom-handled (process body
  has already mutated state; original event should be skipped).
  ```rust
  ctx.handle_replacement();
  ```

- **`ctx.redirect_replacement(zone)`** — Redirect to a different zone
  (Evade-style redirect to deck bottom).
  ```rust
  ctx.redirect_replacement(Zone::Deck);
  ```

- **`ctx.substitute_replacement(subject)`** — Substitute a different
  subject for the parked event (Decoy redirects ally-deletion to self).
  ```rust
  ctx.substitute_replacement(ReplacementSubject::Permanent(decoy_self));
  ```

All four panic in dev builds when called outside a parked-replacement scope
(`Game.parked_replacement.is_none()`). Synchronous replacement processes
that don't install a nested selection use the existing `rctx.cancel() /
handled() / redirect_to() / substitute()` methods on `ReplacementContext`.

### Two parallel APIs

There are two outcome-setting APIs depending on whether the
replacement-process closure parks a player selection or runs synchronously:

| Outcome | Synchronous (`rctx.*` on `ReplacementContext`) | Parked (`ctx.*_replacement` on `EffectContext`) |
|---|---|---|
| Cancelled | `rctx.cancel()` | `ctx.cancel_leave()` |
| CustomHandled | `rctx.handled()` | `ctx.handle_replacement()` |
| Redirected(zone) | `rctx.redirect_to(zone)` | `ctx.redirect_replacement(zone)` |
| Substituted(subject) | `rctx.substitute(subject)` | `ctx.substitute_replacement(subject)` |

The synchronous API runs inside the `replacement_process` closure body itself
(no nested selection). The parked API runs inside the user's `select_*`
callback after the closure parks a selection — `ctx` here is a fresh
`EffectContext` keyed to the same source.

**Mixing the two APIs in one body is supported but uses last-write-wins
semantics.** Calling `rctx.cancel()` before `ctx.select_*(...)` sets the
synchronous outcome as the parked default; the user's nested callback can
override it via `ctx.*_replacement` calls. If the nested callback doesn't
set an outcome, the synchronous default takes effect. This is useful for
"cancel by default; redirect on player choice" patterns.

---

### Selection-bearing keyword authoring pattern (Phase D pattern)

This pattern is for card scripts that need a replacement window that **parks
a player selection** before the replacement outcome is committed. It was
first exercised by the Phase D keyword auto-installs (`Fragment`, `Save`,
`Decoy`, `Partition`, `MaterialSave`). Use it when:

- The effect text reads "you may / must …" and the "what to do" requires a
  player choice (e.g. "place it under one of your Tamers"), AND
- That choice happens *inside* a `WhenWouldBe*` or `OnDeletion` window (not
  a plain `OnPlay`/`WhenDigivolving` trigger).

For the underlying substrate that makes this possible, see
[`docs/superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md`](superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md).

#### The four building blocks

| Building block | Purpose |
|---|---|
| `.optional()` on the `Effect` builder | *Optional* — adds an outer accept/decline (PASS) dialog before the closure runs. Omit when the printed text is mandatory ("must") — the inner `ctx.select_*` will park directly. Phase C substrate dispatches the post-process drain hook for both branches. |
| Self-scope guard in the closure body | `WhenWouldBeDeleted` fires for ALL battle-area permanents' deletions. The body must check `rctx.effect.source_permanent == Some(subject)` and early-return for neighbors. |
| Gate check — early return without parking | Use `.condition(|ctx| ...)` to prevent the outer accept dialog from appearing when the pre-condition fails (e.g. insufficient sources). Failing a `.condition` skips the entire candidate, so no dialog, no parked selection. |
| Parked outcome-setter inside the callback | After the selection resolves, call exactly one of `ctx.cancel_leave()`, `ctx.handle_replacement()`, `ctx.redirect_replacement(zone)`, or `ctx.substitute_replacement(subject)` to write the outcome. These panic in dev builds if called outside a parked-replacement scope. |

#### Worked example — Save auto-install body

Save's auto-install (in `code/code/digimon-engine/src/cards/keyword_effects.rs`) is the
canonical example of an **optional** selection-bearing OnDeletion trigger.
Annotated line-by-line:

```rust
// Save is mounted as an OnDeletion *trigger*, not a WhenWouldBeDeleted
// *replacement*. DCGO `Save.cs` is a post-deletion trigger: deletion commits
// first, Save then plucks the card out of trash and tucks it under a Tamer.
// This means OnDeletion / OnAnyDeletion observers (e.g. Fortitude) still
// fire normally on the deletion, matching DCGO semantics. A WhenWouldBeDeleted
// replacement with cancel_leave() would suppress those observers.
Keyword::Save => vec![Effect::on_deletion(card)
    .name("<Save>")
    // No `.optional()` here — OnDeletion triggers don't carry the outer
    // accept dialog that WhenWouldBeDeleted does. The nested ctx.select_own_permanent
    // below uses is_optional=true as the player's "may" hook instead.
    .process(|ctx| {
        // OnDeletion is keyed on the carrier's permanent handle via
        // TriggerSource::Permanent(handle) — `enqueue_from_permanent` only
        // enumerates effects on the specific deleted permanent, so this trigger
        // is naturally self-scoped. No subject-mismatch guard required.
        let Some(subject) = ctx.source_permanent else {
            return; // Defensive — OnDeletion always carries source_permanent.
        };
        let owner = subject.player;

        // Snapshot the carrier's top-card handle. Deletion is paused on this
        // trigger; the card is still in the carrier's card_sources at this point.
        // We capture the handle (Copy) so the callback closure can use it after
        // the selection resolves.
        let self_card = match ctx
            .game
            .player(owner)
            .battle_area
            .get(subject.index as usize)
        {
            Some(p) => p.top_card().handle(),
            None => return,
        };

        // Park the optional Tamer-pick.
        //   is_optional=true  — PASS = "decline Save"; deletion proceeds normally.
        //   filter closure    — restricts candidates to own Tamers only (same
        //                       controller + is_tamer check, matching DCGO's
        //                       `customMessageArrayTemplate(CanSelectTamer:true)`).
        //   callback          — runs after the player confirms a pick; indices are
        //                       still stable here because the deferred delete_permanent
        //                       hasn't run yet.
        //
        // If the filter yields zero candidates (no own Tamers on field), select_own_permanent
        // no-ops silently — the OnDeletion drain unwinds with no pending_selection; the
        // deletion continues to natural finalization on the same call frame.
        ctx.select_own_permanent(
            "you may place this card under one of your Tamers",
            /*is_optional=*/ true,
            move |g, h| {
                if h.player != owner { return false; }
                let p = match g.players[h.player as usize].battle_area.get(h.index as usize) {
                    Some(p) => p,
                    None => return false,
                };
                p.is_tamer(&g.card_data)
            },
            move |ctx, tamer| {
                // Lift the saved top card off the carrier and place it at the
                // bottom of the chosen Tamer's stack. Indices are stable: the
                // deferred delete_permanent hasn't run yet.
                //
                // After this callback returns, resolve_generic_selection calls
                // resume_pending_deletion, which removes the now-empty-stacked
                // carrier from battle_area and fires OnAnyDeletion — Fortitude
                // observers see the deletion as expected.
                ctx.place_card_under_permanent_bottom(self_card, tamer);
                // Save does NOT call ctx.cancel_leave() — deletion fully commits;
                // Save just intercepts the top card out of the carrier before it
                // falls into trash. Contrast with Fragment / ArmorPurge, which
                // DO call ctx.cancel_leave() and keep the carrier on field.
            },
        );
    })
    .build()],
```

#### When to use `.optional()` vs not

| Scenario | Pattern |
|---|---|
| `WhenWouldBe*` replacement with nested selection ("you may pick a Tamer") | Use `.optional()` on the `Effect` builder — this is mandatory for parked selections inside a replacement window. The substrate `debug_assert!` in `run_candidate_inner` will trip if a non-optional process installs a `pending_selection`. |
| `OnDeletion` trigger with nested selection (Save) | Do NOT use `.optional()` — OnDeletion triggers don't carry the outer accept dialog. Use `is_optional=true` on the inner `select_*` call as the "may" hook. |
| Mandatory replacement with no nested selection (ArmorPurge) | Do NOT use `.optional()` — use `rctx.cancel()` / `rctx.handled()` / `rctx.substitute()` synchronously inside the `replacement_process` closure. No `pending_selection` is parked; the outcome is set in-place. |
| Optional replacement with no nested selection (Evade, Barrier) | Use `.optional()` — the outer accept dialog fires and `rctx.redirect_to()` / `rctx.handled()` runs synchronously inside the accepted process closure. |

#### Gate check pattern — preventing the outer dialog when the pre-condition fails

Use `.condition(|ctx| ...)` to gate on pre-conditions at candidate-collection
time. If the condition returns `false`, the candidate is skipped entirely —
no outer accept dialog is presented to the player.

```rust
Keyword::Fragment(n) => vec![Effect::when_would_be_deleted(card)
    .name(&format!("<Fragment ({n})>"))
    // Gate: carrier must have ≥N sources under the top. Evaluated at
    // candidate-collection time with source_permanent set to the carrier.
    .condition(move |ctx| {
        let Some(perm) = ctx.source_permanent() else { return false; };
        perm.card_sources.len() >= (n as usize) + 1
    })
    .replacement_process(move |rctx| {
        // Self-scope guard — prevent firing on a neighbor's deletion.
        // collect_candidates walks ALL battle-area permanents; without
        // this guard, a Fragment carrier on index 0 would intercept
        // the deletion of index 1 or 2.
        let me_perm = rctx.effect.source_permanent;
        let subject = match rctx.subject {
            ReplacementSubject::Permanent(h) => h,
            _ => return,
        };
        if Some(subject) != me_perm { return; }

        // Re-check the gate at process time (belt-and-suspenders: a
        // stack-mutating earlier replacement in the same chain could have
        // shrunk the stack between collection and process).
        let n_usize = n as usize;
        // ... (re-read stack_len from live game state, return if < n+1) ...

        // Park the mandatory source-pick. is_optional_zero=false matches
        // DCGO Fragment.cs:38 `canNoSelect: () => false`.
        rctx.effect.select_count_capped_multi(
            subject.player,
            CountCappedZone::Material(subject),
            n,
            "trash N digivolution cards",
            /*is_optional_zero=*/ false,
            |_g, _src| true,
            move |ctx, picks| {
                for handle in picks {
                    ctx.trash_card_source(subject, handle);
                }
                // Cancel the deletion — carrier survives.
                ctx.cancel_leave();
            },
        );
    })
    .build()],
```

#### When NOT to use the auto-install

The keyword auto-install (`keyword_to_auto_effect`) provides a permissive
default for each keyword. Some cards need custom selection filters that the
auto-install cannot encode:

- **Color-filtered Decoy** (e.g. "Decoy: Black") — auto-install offers any
  ally Digimon; per-card text restricts by color. Override via a hand-rolled
  `CardEffect` with an explicit color filter on the `rctx.subject` permanent.
- **DigiXros-source MaterialSave** — auto-install offers any own source;
  per-card text may restrict to sources that were DigiXros materials.
- **Color-grouped Partition** — auto-install offers any 2 sources; printed
  text often specifies two color-grouped picks (`firstSources` / `secondSources`
  in DCGO `Partition.cs`). Override via hand-rolled to apply per-group logic.

To override: set the card's `effect_class_name` in `cards.json` to a
hand-rolled struct in `code/code/digimon-engine/src/cards/<set>/<card_id>.rs`. The
auto-install is skipped when `CardEffectRegistry` has a hand-rolled entry
for that `card_id` (the registry entry wins).

---

## 4. Handles

- `CardHandle(u16)` — identifies a specific card instance by its unique `card_index`. Copy. Used in effect closures so they can be captured cheaply.
- `PermanentHandle { player, index }` — the battle-area slot holding a permanent. Copy. **Not stable across deletions** — if something earlier in the battle area is deleted, indices shift. If you care about identity across arbitrary game state changes, snapshot the card handle instead.

When a card effect needs to refer to the permanent it came from:

```rust
.process(|ctx| {
    if let Some(me) = ctx.source_permanent {
        ctx.add_dp_modifier(me, 1000, Expiry::EndOfTurn);
    }
})
```

---

## 5. Key enums

### `EffectTiming`

```
OnPlay, WhenDigivolving, OnAttack, OnDeletion, WhenAttacking, OnBlock,
SecurityEffect, CounterEffect,
StartOfYourTurn, StartOfOpponentsTurn, EndOfYourTurn, EndOfOpponentsTurn, EndOfAttack,
OnAllyAttack, OnOpponentAttack, OnDrawCard, OnTrash, OnReturn, OnSuspend, OnUnsuspend,
OnAddToHand, OnReveal, OnPlaceSecurity,
OnEnterField, OnEnterFieldAnyone, OnLeaveField,
BeforePayCost, WhenPlayedFromHand,
OnDigivolve, OnDnaDigivolve, OnDigiXros,
AlwaysActive, Declarative,
OptionMain, OptionSecurity,
None
```

### `Expiry`

```
Permanent             # never expires on its own
EndOfTurn             # cleared at the end of any turn
EndOfOpponentsTurn    # cleared at the end of the source-player's opponent's turn
EndOfAttack           # cleared when the current attack resolves
EndOfBattle           # same as EndOfAttack for most purposes
UntilLeaveField       # cleared when the permanent leaves the field
```

### `ModifierType` (partial — see `enums.rs` for full list)

- DP: `ChangeDp`, `ChangeBaseDp`, `DpFloor`, `DontHaveDp`
- Cost: `ChangePlayCost`, `ChangeDigivolveCost`, `CannotReduceCost`
- Protection: `CannotBeDestroyed`, `CannotBeDestroyedByBattle`, `CannotBeDestroyedByEffect`
- Attack: `CannotAttack`, `CannotAttackPlayer`, `CanAttackUnsuspended`, `CanAttackActivePlayer`
- Suspend: `CannotSuspend`, `CannotUnsuspend`
- Targeting: `CannotBeSelectedByEffect`, `CannotBeAffected`
- Granted keywords: `GrantBlocker`, `GrantRush`, `GrantJamming`, `GrantPiercing`, `GrantReboot`, `GrantBlitz`, `GrantAlliance`, `GrantRaid`, `GrantDecoy`
- Security: `SecurityAttackChange`
- Color/level: `ChangeColor`, `AddColor`, `ChangeLevel`

### `Keyword`

```
Blocker, SecurityAttackPlus(i8), SecurityAttackMinus(i8),
Rush, Jamming, Piercing, Reboot, DeDigivolve(u8), DrawX(u8),
Blitz, Armor, Raid, Alliance, Blast, Save, Fortitude, Overclock,
Barrier, Decoy, Material, Partition
```

---

## 6. Common patterns

### Gain memory on play

```rust
Effect::on_play(card)
    .name("Gain 1 memory")
    .process(|ctx| ctx.gain_memory(1))
    .build()
```

### Conditional draw

```rust
Effect::on_play(card)
    .name("Draw 2 if opponent has 3+ Digimon")
    .condition(|ctx| {
        let opp = ctx.opponent_id();
        ctx.battle_area(opp).iter()
            .filter(|p| p.is_digimon(ctx.card_data()))
            .count() >= 3
    })
    .process(|ctx| {
        let me = ctx.player;
        ctx.draw(me, 2);
    })
    .build()
```

### Buff all your Digimon for the turn

```rust
Effect::on_play(card)
    .name("All your Digimon +1000 DP")
    .process(|ctx| {
        let me = ctx.player;
        let count = ctx.battle_area(me).len();
        for i in 0..count {
            let h = PermanentHandle { player: me, index: i as u8 };
            ctx.add_dp_modifier(h, 1000, Expiry::EndOfTurn);
        }
    })
    .build()
```

### Grant a keyword

```rust
Effect::on_play(card)
    .name("Grant Rush to self")
    .process(|ctx| {
        if let Some(me) = ctx.source_permanent {
            ctx.grant_keyword(me, Keyword::Rush, Expiry::EndOfTurn);
        }
    })
    .build()
```

### Security effect — trigger when this card is revealed in security

```rust
Effect::security(card)
    .name("Deal 3000 DP to attacker")
    .process(|ctx| {
        // Engine wires `ctx.source_permanent` to the attacker when firing
        // SecurityEffect timings (forthcoming — see RUST_ENGINE_GAPS.md).
    })
    .build()
```

---

## 7. Anti-patterns (don't do this)

### ❌ Holding a reference to Player across `ctx.player_mut` calls

```rust
// Wrong — player borrow dies when we call ctx.draw
let p = ctx.player(ctx.player);
let count = p.hand.len();   // OK
ctx.draw(ctx.player, 1);    // ERROR: p still borrowed
```

Fix: read the immutable data into locals first, then mutate.

```rust
let count = ctx.my_player().hand.len();
let me = ctx.player;
ctx.draw(me, 1);
```

### ❌ Stashing `PermanentHandle` across deletions

```rust
// Wrong — after deleting handle 0, handle 2 now points to a different permanent.
let h0 = PermanentHandle { player: me, index: 0 };
let h2 = PermanentHandle { player: me, index: 2 };
ctx.delete_permanent(h0);
ctx.delete_permanent(h2);   // likely wrong target!
```

Fix: delete from highest index to lowest, or collect stable identifiers (card_index) first.

### ❌ Panicking on a missing card

```rust
let id = ctx.player(0).hand[0].card_id(ctx.card_data()); // panic if empty
```

Fix: bounds-check before indexing.

### ❌ Capturing non-Copy state in closures

```rust
// Wrong — Vec<String> isn't Copy, and the closure might outlive it
let names: Vec<String> = collect_some_names();
Effect::on_play(card)
    .process(move |ctx| {
        for n in &names {  // needs 'static
            ...
        }
    })
```

Fix: move the collection into the closure by value (already correct with `move`), or derive everything from `ctx.card_data()` inside the closure.

---

## 8. Testing

Use `DebugRunner` for behavioral tests.

```rust
use digimon_engine::debug_runner::{DebugRunner, make_test_card};

#[test]
fn my_card_gains_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("BT1-010", "Example"))
        .hand(0, &["BT1-010"])
        .start();
    let m_before = r.memory();
    r.play(0, 0);
    assert_eq!(r.memory(), m_before + 2);
}
```

Builder methods:
- `.add_card(CardData)` — register a card.
- `.with_card_data(HashMap<String, CardData>)` — bulk load.
- `.hand(player, &["ID1", "ID2"])` — deterministic hand (no shuffle).
- `.deck(player, &[...])` — deterministic deck (last element = top of deck).
- `.security(player, &[...])` — explicit security stack.
- `.digitama(player, &[...])` — explicit digitama deck.
- `.with_rules(Rules)` — override rules (default standard).
- `.with_registry(CardEffectRegistry)` — inject a custom registry.
- `.start()` — builds and advances past Mulligan into turn 1.
- `.build()` — builds without starting (remains in Mulligan).

Runner methods:
- `.play(player, hand_index)` — `Game::play_from_hand` + OnPlay.
- `.place_on_field(player, "ID", turn_played_override)` — bypasses play for combat setup. Pass `Some(0)` to avoid summoning sickness.
- `.attack_digimon(attacker, defender)` / `.attack_player(attacker, defender_player)`
- `.end_turn()` / `.pass_turn()`
- `.memory()`, `.hand_size(p)`, `.battle_area_size(p)`, `.deck_size(p)`, `.trash_size(p)`, `.security_count(p)`
- `.effective_dp(handle)`, `.dp_of(handle)`
- `.game_over()`, `.winner()`

---

## Phase 5 — Cost-Reduction Builder Hooks

Added in Phase 5 to support closure-valued dynamic cost reduction and a synchronous pay-cost gate on triggered effects. These unblock ~50 cards across Rocks, Dark Masters, and TS Olympos whose cost-reduction predicates read live game state (trash count, trait presence, field state) and cannot be expressed as static `.cost_reduction(n)` values.

Commits: `795c6529`..`6a349787` (8 commits). Full suite: **525 passing** (+26 from Phase 4 baseline of 499).

---

### `Effect::before_pay_cost(card)` constructor

```rust
Effect::before_pay_cost(card)
    .name("…")
    .condition(|ctx| bool)
    .cost_reduction_fn(|ctx| i32)
    .pay_cost_fn(|ctx| bool)   // optional gate
    .build()
```

A dedicated constructor that sets `EffectTiming::BeforePayCost`. Use it for any effect whose card text reads "reduce the play cost of …" when triggered by a live-state predicate. The constructor is a thin wrapper over `.timing(EffectTiming::BeforePayCost)` — all other builder methods apply normally.

---

### `.cost_reduction_fn(|ctx| i32)`

```rust
pub fn cost_reduction_fn<F>(self, f: F) -> EffectBuilder
where
    F: Fn(&EffectReadContext) -> i32 + Send + Sync + 'static
```

**Semantics.** Attaches a closure that computes a dynamic cost reduction amount at the moment the cost is about to be paid. The closure receives a read-only `EffectReadContext` — it can inspect hand, trash, field state, modifiers, and memory, but cannot mutate game state. The return value is clamped to `>= 0` per effect before accumulation; a closure that returns a negative value contributes 0 (never increases cost). Multiple reductions across the field are summed.

**Dispatch site.** The engine calls `Game::scan_before_pay_cost_reduction` in all five pay_memory call sites: play from hand, play from trash, digivolve from hand, digivolve onto breeding, and effect-initiated digivolve. The scan visits every field permanent in stable order, evaluates condition and pay_cost_fn filters (see below), and accumulates the total reduction before `pay_memory` is invoked.

**Clamping.** The per-effect return value is clamped to `max(0, returned_i32)` before accumulation. The final total is then clamped to `max(0, printed_cost - total_reduction)` — cost is never driven below zero.

**Worked example.** A Rocks-archetype Tamer whose text reads "Your Machine Digimon cost 1 less to play for each [Destroy Bomber] in your trash":

```rust
// Hypothetical Rocks card
pub struct RocksTamer;

impl CardEffect for RocksTamer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::before_pay_cost(card)
                .name("Machine Digimon cost 1 less per Destroy Bomber in trash")
                .condition(|ctx| {
                    // Only applies to Machine Digimon being played
                    // (card-being-played context is read from ctx.game.pending_play_card)
                    ctx.game.pending_play_card
                        .map(|h| {
                            let data = &ctx.card_data()[h.data_index];
                            data.card_kind == CardKind::Digimon
                                && data.traits.iter().any(|t| t == "Machine")
                        })
                        .unwrap_or(false)
                })
                .cost_reduction_fn(|ctx| {
                    let me = ctx.player;
                    ctx.trash(me)
                        .iter()
                        .filter(|cs| cs.contains_card_name("Destroy Bomber"))
                        .count() as i32
                })
                .build(),
        ]
    }
}
```

The closure returns the count of matching cards in trash; the engine clamps each effect's contribution and sums across all field permanents before paying.

---

### `.pay_cost_fn(|ctx| bool)`

```rust
pub fn pay_cost_fn<F>(self, f: F) -> EffectBuilder
where
    F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static
```

**Semantics.** Attaches a synchronous gate closure that fires between condition and process. The closure receives a mutable `EffectContext` so it can perform side effects (e.g. trash cards, suspend a Tamer) as the cost payment. Returning `true` continues to process; returning `false` suppresses process and signals the failure mode described below.

**Two dispatch sites with different failure semantics:**

1. **BeforePayCost timing (play/digivolve path)** — `pay_cost_fn` fires after `cost_reduction_fn` is accumulated but before `pay_memory`. Returning `false` skips this effect's reduction contribution (the effect's reduction is excluded from the total), but play continues at the reduced-by-other-effects cost. This covers cases like "trash 2 cards from the top of your deck to reduce cost by 2 — only if you have 2 cards to trash".

2. **Any other triggered timing (run_queued_effect)** — `pay_cost_fn` fires after condition but before process. Returning `false` aborts the entire effect: process is skipped, no state changes happen beyond what the pay_cost_fn itself may have done. This is the correct semantic for "suspend 1 of your Tamers as cost — if you can't, the effect doesn't fire".

**v1 constraint.** The closure is synchronous and must NOT install a `PendingSelection` inside it. Effects whose cost payment itself requires a player choice (e.g. "choose and suspend one of your Tamers") must model the selection in the `process` body, not in `pay_cost_fn`. A future version may lift this constraint.

**Worked example.** "When you play a Digimon, you may trash 2 cards from the top of your deck to reduce its cost by 2":

```rust
Effect::before_pay_cost(card)
    .name("Trash 2 from top to reduce play cost by 2")
    .pay_cost_fn(|ctx| {
        // Gate: only apply if deck has >= 2 cards to trash as cost
        let me = ctx.player;
        if ctx.my_player().deck.len() < 2 {
            return false; // cannot pay — skip this reduction
        }
        ctx.trash_from_top(me, 2);
        true  // cost paid — accumulate the reduction
    })
    .cost_reduction_fn(|_ctx| 2)
    .build()
```

Note: `pay_cost_fn` fires before `cost_reduction_fn` is accumulated (gate → pay → then contribute the reduction if true). The deck-size check prevents paying a cost the player cannot afford; returning `false` leaves deck and memory untouched.

---

### Scan ordering

`Game::scan_before_pay_cost_reduction` visits effects in stable field-index order: player 0's permanents before player 1's, ascending permanent index within each player, and bottom-source-first within each permanent's digivolution stack (inherited effects). If effect A's `pay_cost_fn` mutates state (e.g. mills cards) that effect B's `cost_reduction_fn` reads (e.g. counts trash), the ordering is deterministic — the mill happens before B's count. Card scripts that interact across effects should document this dependency.

---

## Phase 6 — Flood-Gate & Restriction Modifiers

Added in Phase 6 to clamp entire action categories at both the action-mask layer (RL-visible suppression) and the resolver layer (defense-in-depth). Unblocks Dark Masters lockout shells, Medusamon Petrification, TS Olympos Tamer-anchoring, and Rocks Plug-In lockouts (~55 meta-pool cards across all 5 audited archetypes).

Commits: `69464289`..`6b0bd28a` (8 commits). Full suite: **556 passing** (+31 from Phase 5 baseline of 525).

---

### Player-Scoped `ModifierRegistry`

Phase 6 adds a parallel storage tier to `ModifierRegistry`:

```rust
pub struct ModifierRegistry {
    permanent_modifiers: HashMap<PermanentHandle, Vec<ModifierEntry>>,
    // NEW in Phase 6:
    player_modifiers: HashMap<PlayerId, Vec<PlayerModifierEntry>>,
}
```

`PlayerModifierEntry` has five fields:

```rust
pub struct PlayerModifierEntry {
    pub modifier:         ModifierType,
    pub value:            i32,
    pub expiry:           Expiry,
    pub source_permanent: Option<PermanentHandle>,  // for UntilLeaveField expiry
    pub source_player:    PlayerId,                 // who installed the modifier
}
```

No closure condition in v1 — card scripts gate installation via `.condition` on the `Effect`, and the modifier itself is a simple flag. Phase 7 may add closure conditions to `PlayerModifierEntry` for the would-replacement framework.

**Six new methods on `ModifierRegistry`:**

```rust
// Install
modifiers.add_player_modifier(target_player: PlayerId, entry: PlayerModifierEntry)

// Query
modifiers.player_has(target_player, modifier: ModifierType) -> bool
modifiers.player_modifier_value(target_player, modifier: ModifierType) -> i32
modifiers.player_modifiers_iter(target_player) -> impl Iterator<Item = &PlayerModifierEntry>

// Expiry
modifiers.expire_player_end_of_turn(ending_player: PlayerId)
modifiers.expire_player_on_permanent_leave(handle: PermanentHandle)
```

**Worked example — Shamanmon-style Tamer installing `CannotGainMemoryExceptFromTamers`:**

```rust
// BT18-009 Shamanmon (TS Olympos archetype)
// "[Your Turn] [Main] Trash this Tamer. Your opponent cannot gain memory
//  from sources other than their own Tamer effects until the end of their turn."
pub struct Bt18009;

impl CardEffect for Bt18009 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card)
                .name("Trash self; opponent CannotGainMemoryExceptFromTamers")
                .condition(|ctx| {
                    // Only fires if this card is on the field (not from hand raw — timing is MainOnField)
                    ctx.source_permanent.is_some()
                })
                .process(|ctx| {
                    // Trash this Tamer
                    if let Some(h) = ctx.source_permanent {
                        ctx.delete_permanent(h);
                    }
                    // Install player-scoped flood gate on the opponent
                    let opp = ctx.opponent_id();
                    let src_player = ctx.player;
                    ctx.game.modifiers.add_player_modifier(opp, PlayerModifierEntry {
                        modifier:         ModifierType::CannotGainMemoryExceptFromTamers,
                        value:            1,
                        expiry:           Expiry::EndOfTurn,
                        source_permanent: None,   // self already deleted
                        source_player:    src_player,
                    });
                })
                .build(),
        ]
    }
}
```

`UntilLeaveField` is the dominant expiry for aura-style flood gates (while the emitter is in play). Use `source_permanent: Some(h)` so `expire_player_on_permanent_leave` clears it automatically when the Tamer is deleted or returned.

---

### `PlaySource` Enum

A new typed enum threaded through all five play/digivolve helpers so flood gates like `CannotPlayDigimonByEffect` can discriminate effect-initiated plays from hand-cost plays:

```rust
pub enum PlaySource {
    ByHand,      // normal play-from-hand (player paid memory)
    ByEffect,    // effect-initiated play (e.g. ctx.play_from_hand_with_cost with ByEffect)
    ByDigivolve, // digivolve path
}
```

The five helpers that receive `PlaySource`:
- `play_from_hand_with_cost`
- `play_from_trash_with_cost`
- `digivolve_from_hand`
- `digivolve_onto_breeding`
- `effect_initiated_digivolve`

Call sites: 13 updated (mask, action decoder, effect context wrappers, security play-from-security path). `CannotPlayDigimonByEffect` gates only when `source == PlaySource::ByEffect`, matching DCGO's "by an effect" qualifier.

---

### Flood-Gate Catalog

All 13 new `ModifierType` variants added in Phase 6:

| Variant | Semantics | Enforcement | Example card text |
|---------|-----------|-------------|-------------------|
| `CannotPlayDigimonByEffect` | Opponent cannot play Digimon cards triggered by a card effect | Resolver (play_from_hand/trash/security with `ByEffect`) | "Your opponent can't play Digimon by effect" |
| `CannotGainMemoryByEffect` | Opponent cannot gain memory from any effect | Resolver (gain_memory) | "Your opponent can't gain memory by effect" |
| `CannotGainMemoryExceptFromTamers` | Opponent can only gain memory from Tamer-card effects; all other sources blocked | Resolver (gain_memory, gated by `source_is_tamer`) | "Your opponent can't gain memory except from Tamer effects" |
| `CannotReducePlayCost` | Opponent's play costs cannot be reduced by any effect | Resolver (scan_before_pay_cost_reduction) | "Your opponent can't reduce play costs" |
| `CannotActivateMainEffects` | Opponent's Digimon/Tamer [Main] effects cannot be activated | Mask (`MainOnField` bits zeroed) | "Your opponent's Digimon can't activate their [Main] effects" |
| `CannotActivateWhenDigivolvingEffects` | Opponent's [When Digivolving] effects cannot fire | DORMANT (resolver hook, not yet wired to enforcement site) | "Your opponent's Digimon can't activate their [When Digivolving] effects" |
| `CannotActivateSecurityEffects` | Opponent's security-revealed effects cannot fire | DORMANT (resolver hook) | "Your opponent's Digimon can't activate their [Security] effects" |
| `CannotDigivolveDigimonByEffect` | Opponent cannot effect-initiate a digivolve | DORMANT (resolver hook) | "Your opponent can't digivolve Digimon by effect" |
| `CannotDrawByEffect` | Opponent cannot draw cards via effects | Resolver (draw) | "Your opponent can't draw by effect" |
| `CannotAddSecurityByEffect` | Opponent cannot add cards to their security via effects | Resolver (place_on_security with ByEffect) | "Your opponent can't add to their security by effect" |
| `CannotTrashOpponentSecurity` | Prevents opponent from trashing your security via effects | DORMANT (resolver hook) | Dark Masters lock piece |
| `CannotReduceOpponentSecurity` | Prevents opponent from reducing your security count | DORMANT (resolver hook) | Dark Masters lock piece |
| `IgnoreColorRequirement` | Player may digivolve ignoring color requirements | DORMANT (mask hook) | "You may digivolve ignoring color requirements" |

**DORMANT variants:** The API surface is wired (enum variants, storage, install/query helpers) but the enforcement site has not yet been connected. As real cards arrive and need those variants, each enforcement site is a one-liner addition. Do not ship stubs that auto-apply — connect the enforcement gate at the real call site when the first card needs it.

**Enforcement sites (active):**
- **Mask:** `CannotPlayFromHand` upgraded to player-scoped query; `CannotAttack` enforced in both `Main` and `EndOfTurnAction` phases; `CannotActivateMainEffects` zeroes `MainOnField` bits in the main-phase mask.
- **Resolver:** `CannotDrawByEffect` gates `ctx.draw`; `CannotGainMemoryByEffect` and `CannotGainMemoryExceptFromTamers` gate `ctx.gain_memory`; `CannotAddSecurityByEffect` gates `ctx.place_on_security`; `CannotReducePlayCost` nullifies `scan_before_pay_cost_reduction` for the restricted player; `CannotPlayDigimonByEffect` gates the three effect-play helpers (`play_from_hand_with_cost` + `play_from_trash_with_cost` + `play_from_security`) when `PlaySource::ByEffect`.

---

### Effect Source Kind Helpers

Every resolving effect carries an explicit `EffectSourceKind`: `Digimon`, `Tamer`, `Option`, or `Rule`. This is assigned when the effect is queued and preserved through pending selection callbacks. Security is not a source kind: a Digimon card's security effect is still a Digimon effect. DUAL cards are context-sensitive: Option use is `Option`; effects after Arts Digivolve are `Digimon`.

```rust
// On EffectContext (mutable):
pub fn source_kind(&self) -> EffectSourceKind
pub fn source_is_digimon(&self) -> bool
pub fn source_is_tamer(&self) -> bool
pub fn source_is_option(&self) -> bool

// On EffectReadContext (read-only, for cost-reduction closures):
pub fn source_kind(&self) -> EffectSourceKind
pub fn source_is_digimon(&self) -> bool
pub fn source_is_tamer(&self) -> bool
pub fn source_is_option(&self) -> bool
```

`source_is_tamer()` is retained as a compatibility helper for flood gates like `CannotGainMemoryExceptFromTamers`, but it now reads the explicit source-kind field rather than inferring from card kind at resolver time.

`CannotBeAffected` supports source-kind-aware filters through `ModifierEntry::cannot_be_affected_by_opponents_source_kind(...)`. The central gate is `Game::permanent_is_unaffected_by_effect(target, effect_controller, source_kind)` and card-script mutations route through `EffectContext::can_affect_permanent`.

**Usage in `CannotGainMemoryExceptFromTamers`:**

```rust
// In gain_memory resolver gate:
if ctx.game.modifiers.player_has(target, ModifierType::CannotGainMemoryExceptFromTamers)
    && !ctx.source_is_tamer()
{
    return; // blocked — not a Tamer source
}
```

---

## Phase 7 — Would-Replacement Timings

Phase 7 adds a first-class **replacement-effect layer** to the engine. Replacement effects intercept an impending state change (deletion, return-to-hand, return-to-deck, trash-by-effect, de-digivolve, draw, security-placement, security-loss) **before** it commits and either cancel it, redirect it, substitute the affected subject, or fully handle it in-process.

Unlike observer timings (`OnDeletion`, `OnReturn`, …) which fire *after* the event, `Would*` timings fire *before* and can mutate the outcome. This makes printed keywords like `<Barrier>`, `<Evade>`, and `<Decode>` faithful to their printed rules — Barrier is not an auto-selection that trashes the top of deck; it's an *optional* replacement that surfaces as a `PendingSelection::Replacement` with both accept and decline in the mask, so the RL action space can learn the decision (working rule 17).

### `EffectTiming::Would*` variants

Nine variants dispatch today:

| Variant | Fires at | Default destination | Notes |
|---------|----------|--------------------|-------|
| `WhenWouldLeaveBattleArea` | Every leave-the-field route (super-timing) | varies | Fires before the route-specific timing; cancel here affects all routes. |
| `WhenWouldBeDeleted` | `delete_permanent_with_cause` / battle / security-check | `Zone::Trash` | Barrier / Evade / Partition / ArmorPurge / Fragment replace here. |
| `WhenWouldBeReturnedToHand` | `return_to_hand` | `Zone::Hand` | Decode's hand-timing replaces here. |
| `WhenWouldBeReturnedToDeck` | `return_to_deck` | `Zone::Deck` | Decode's deck-timing replaces here. |
| `WhenWouldBeTrashed` | Effect-driven trash from hand (+ future: battle/security) | varies | `CannotBeTrashedByEffect` passive cancels here. |
| `WhenWouldBeDeDigivolved` | `de_digivolve` | — | `CannotBeDeDigivolved` cancels; `Substituted(other)` re-targets. |
| `WhenWouldDraw` | `EffectContext::draw` | — | `CannotDrawByEffect` interaction orthogonal (Phase 6 flood gate). |
| `WhenWouldPlaceInSecurity` | Effect-driven `place_on_security` | `Zone::Security` | Redirect-to-trash or reorder. |
| `WhenWouldLoseSecurity` | Security-pop during attack | — | Fires before `SecuritySkill` drains. |

Two variants are reserved for Phase 9 (combat-interrupt completion) and do not dispatch yet: `WhenWouldAttack`, `WhenWouldBeAttackTarget`.

### `ReplacementContext` API

Each `Would*` effect process receives a `ReplacementContext` by `&mut`:

```rust
pub struct ReplacementContext<'g> {
    pub effect: &'g mut EffectContext<'g>,
    pub cause: ReplacementCause,
    pub subject: ReplacementSubject,
    pub original_destination: Option<Zone>,
    /* outcome — set via helpers below */
}
```

Mutating helpers (mutually exclusive — call exactly one):

| Helper | Outcome | Meaning |
|--------|---------|---------|
| `rctx.cancel()` | `Cancelled` | Skip the event entirely. No observers fire. |
| `rctx.redirect_to(zone)` | `Redirected(zone)` | Route to a different destination (deletion → bottom-of-deck for Evade). |
| `rctx.substitute(subject)` | `Substituted(subject)` | Apply the original event to a different subject (Partition: delete a source instead). |
| `rctx.handled()` | `CustomHandled` | The process mutated state directly (Barrier: trashed top of deck); skip the original event AND skip observer dispatch. |

Read-only context fields are always available through `rctx.effect.*` (the underlying `EffectContext`) and `rctx.cause` / `rctx.subject` / `rctx.original_destination`.

### `ReplacementCause`

Five variants, **derived at the fire-site** (not threaded through card scripts):

```rust
pub enum ReplacementCause {
    Battle,           // DP battle — only resolve_battle dispatches this
    OwnEffect,        // Target's controller caused the event
    OpponentEffect,   // The other player's effect caused it
    SecurityCheck,    // Security-reveal or SecuritySkill-driven
    Cost,             // Cost-payment trash/suspend (rare)
}
```

The fire-site's inference rules live in `infer_effect_cause` and `infer_deletion_cause`. Card scripts read `rctx.cause` to filter (e.g. "only replace if cause is `OpponentEffect`"); they never compute it themselves.

### `ReplacementSubject`

```rust
pub enum ReplacementSubject {
    Permanent(PermanentHandle),   // field events — the common case
    Card(CardHandle, Zone),        // hand / trash / security events
    Player(PlayerId),              // draws, security-placement by effect
}
```

### Passive-modifier migration (Task 5)

Phase 6 shipped three restriction modifiers as enum variants without enforcement (`CannotBeReturnedToDeck`, `CannotBeReturnedToHand`, `CannotBeDeDigivolved`, `CannotBeTrashedByEffect`). Phase 7 wires these through the replacement framework as **automatic mandatory cancels**. Phase 0's `CannotBeDestroyed` / `CannotBeDestroyedByBattle` / `CannotBeDestroyedByEffect` migrate too.

Builders:

```rust
// Permanent-scoped: this Digimon can't be returned to deck by opponent's effects.
ModifierEntry::passive_replacement(ModifierType::CannotBeReturnedToDeck)
    .opponent_only()
    .attach(&mut game.modifiers, target_handle, source_player, Expiry::Permanent);

// With a live-state condition (e.g. "…while another X is in play").
ModifierEntry::passive_replacement(ModifierType::CannotBeDeDigivolved)
    .with_condition(|ctx, _subj| {
        ctx.read_game().player(ctx.source_player()).battle_area
            .iter()
            .any(|p| p.top_card().contains_name("Bagramon"))
    })
    .attach(&mut game.modifiers, target, source_player, Expiry::Permanent);
```

Cause-filter semantics: `.opponent_only()` sets `cause_filter = Some(OpponentEffect)`. Absent filter = cause-agnostic (fires for any cause).

### Native-keyword auto-install (Task 6)

`<Barrier>`, `<Evade>`, and `<Decode>` ship out-of-the-box via printed-keyword auto-install at `effects_for_card` time — no hand-authored `CardEffect` script required. Put the keyword in `CardData::keywords` and the engine installs the matching replacement.

**Phase D (2026-04-25):** `<Fragment(N)>`, `<ArmorPurge>`, `<Save>`, `<Decoy>`, `<Fortitude>`, `<Partition>`, and `<MaterialSave(N)>` now auto-install alongside Barrier/Evade/Decode. Cards declaring only these keywords need zero hand-rolled `CardEffect` code. See the "Selection-bearing keyword authoring pattern" section above for the template and `code/code/digimon-engine/src/cards/keyword_effects.rs` for the canonical implementations.

### Worked example — a hand-authored Barrier-flavored effect

```rust
use crate::effect::{CardEffect, Effect};
use crate::replacement::ReplacementSubject;

pub struct MyBarrier;

impl CardEffect for MyBarrier {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Barrier>")
            .optional()  // offer accept/decline — required for "may" effects
            .replacement_process(|rctx| {
                // Only fire for my own permanent.
                let me = rctx.effect.source_permanent;
                let ReplacementSubject::Permanent(subj) = rctx.subject else { return };
                if Some(subj) != me { return; }

                // Pay the Barrier cost — trash top of owner's deck via the
                // curated EffectContext helper. Do NOT reach into
                // `rctx.effect.game.players[..].deck` directly; stick to the
                // `EffectContext` surface so observer hooks and future
                // replacement-layer instrumentation stay consistent.
                rctx.effect.trash_from_top(subj.player, 1);

                // Suppress the deletion.
                rctx.handled();
            })
            .build()]
    }
}
```

> Note: the canonical `Keyword::Barrier` arm in `cards/keyword_effects.rs`
> currently uses `game.players[..].deck.pop()` / `.trash.push()` as an
> inlined equivalent. New scripts should prefer the `EffectContext` helper
> (`trash_from_top`) shown above; a follow-up pass will migrate the
> keyword arms to match.

### Phase 7 v1 constraints

1. **Partition / ArmorPurge / Fragment(N)** — resolved in Phase D (2026-04-25); all seven alpha-tier selection-bearing keywords now auto-install. See the "Selection-bearing keyword authoring pattern" section for the template.
2. **Optional replacements for `Card` / `Player` subjects** silently no-op on the commit path — the `commit_deferred_outcome` helper is Permanent-only in v1, guarded by `debug_assert!`. This is unreachable today; documented to flag it if a future fire-site ships a Card/Player optional replacement.
3. **Multi-replacement `TriggerOrder` prompts** are not emitted when both sides have >1 candidates. v1 runs candidates in collection order (own-first, opp-second) and the last non-None outcome wins.
4. **`ACTION_SPACE_SIZE` unchanged at 2168.** `REPLACEMENT_ACCEPT` reuses the existing `EffectChoice` action range (specifically the HAND_EFFECT slot 59) and `PASS` (62) serves as decline, so no tensor/mask regression.
5. **Spec §7.5 once-per-event guard** (Task 7): a `(timing, subject)` pair that already fired in the current call chain is skipped on re-entry. During a callback-commit continuation (`in_replacement_commit`) the guard strengthens to "any prior fire for this subject blocks" — preventing a redirect route from re-prompting for a different Would* timing on the same subject (e.g. Decode's deck→hand redirect must not cascade into a second hand-timing prompt).
6. **Commit-continuation broadening — known v1 limitation.** During the commit-continuation of an optional replacement, the once-per-event guard blocks ANY replacement effect targeting the same subject for the remainder of the call chain, even replacements installed by different cards. This means a passive restriction modifier (e.g. `CannotBeReturnedToDeck`) on a Digimon that is redirected via a `Would*` effect will NOT cancel the subsequent commit-phase zone move. Concrete scenario: P has a `WhenWouldBeDeleted` redirect-to-deck AND a `CannotBeReturnedToDeck` passive; on deletion the redirect fires, accepts, then `return_to_deck` fires `WhenWouldBeReturnedToDeck` — but the passive cancel is suppressed by the broadening and P lands in the deck, violating the passive. Workaround: avoid stacking cancel-passives with redirect-replacements on the same Digimon. A spec-§7.5-narrowing pass in Phase 8 may key the fired-set on `(timing, subject, source_card)` so different source cards' replacements can still fire during commit. Pinning test: `tests/replacements/dispatcher_guard.rs::commit_continuation_broadening_blocks_different_timing_v1_known_limitation` (flip its assertion when Phase 8 narrows).

### Testing a Would* effect

TDD per working rule 18 — write behavioral tests against `DebugRunner` under `code/code/digimon-engine/tests/replacements/` **before** implementing the effect:

```rust
#[test]
fn my_barrier_cancels_battle_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_my_barrier())
        .add_card(big_attacker())
        .start();
    let def = r.place_on_field(0, "MY_BARRIER", Some(0));
    let atk = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let _ = r.attack_digimon(atk, def, false);

    // Barrier installs the optional prompt.
    assert!(r.game.pending_selection.is_some());
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();

    assert_eq!(r.battle_area_size(0), 1);          // defender survived
    assert_eq!(r.game.player(0).deck.len(), initial_deck - 1);
}
```

See `tests/replacements/behavioral_end_to_end.rs` for the canonical end-to-end template.

---

## Phase 8 — Option Card Play Flow

Phase 8 teaches the Rust engine how Option cards actually work. Prior to Phase 8, Option cards either fell through to the generic `play_from_hand` permanent path (wrong: they'd land on the field as vanilla Digimon) or produced no observable effect. Phase 8 adds a dedicated play pipeline: **play → pay cost → fire `OnUseOption` (global observer) + `OptionMain` (this card's body) → dispose per subtype**.

Options are *ephemeral*: they do not normally live on the field. The exceptions are **Delay** and **Training**, which park on the battle area until a scheduled trigger trashes them. **Plug-In** Options attach sideways to a host Digimon via `Permanent.linked_cards` and live as long as the host does. **Standard** Options drain their body and immediately self-trash via a `WhenWouldBeTrashed` replacement window (spec §Phase 7) with `ReplacementCause::Cost`.

### Four Option subtypes

| Subtype | Disposition | Timing(s) fired |
|---------|-------------|-----------------|
| **Standard** | Body drains, then self-trashes via `WhenWouldBeTrashed` (cause `Cost`). | `OnUseOption` (global) → `OptionMain` (this card) → cleanup. |
| **Delay** | Body drains, card parks on field as `OptionState::Delayed`. At the scheduled turn end, a `DelayEffect` fires and the card trashes via `WhenWouldLeaveBattleArea` + `WhenWouldBeDeleted`. | `OnUseOption` → `OptionMain` (install delay) → later: `DelayEffect` → leave/deleted replacement windows → trash. |
| **Plug-In (Link)** | Body drains, player selects a legal host, card attaches sideways into `host.linked_cards`. `OnLink` fires globally after attach. Effects on the attached card flagged `.linked()` fire off the host's timings. | `OnUseOption` → `OptionMain` (runs `.link(cost, filter)` mask + prompt + attach) → `OnLink` (global). |
| **Training** | Body drains, card parks on field as `OptionState::Training`. At the owner's next breeding-hatch, an `OnTrainingTrash` observer fires on the specific Training permanent being trashed, then `delete_permanent_with_cause(Cost)` routes it to the trash. | `OnUseOption` → `OptionMain` → later: `OnTrainingTrash` → deletion. |

### Shape types added in Task 1

```rust
// permanent.rs
pub enum OptionState {
    Standard,
    Delayed { owner: PlayerId, trash_on_turn: u16 },  // absolute turn_count
    Linked { host: PermanentHandle },
    Training { owner: PlayerId },
}

pub struct Permanent {
    // …
    pub option_state: OptionState,
    pub linked_cards: Vec<CardSource>,   // Plug-Ins attached to this host
}

// selection.rs
pub struct PendingOption {
    pub owner: PlayerId,
    pub card: CardSource,
    pub source_kind: OptionUseSource,
    pub resolution_phase: OptionResolutionPhase,
}

pub enum OptionUseSource {
    Hand,
    Trash,
}

pub enum OptionResolutionPhase {
    LinkSelectHost,     // waiting on a host-pick for a Plug-In
    MainEffectDrain,    // body running
    ArtsSelectTarget,   // optional Arts Digivolve target prompt
    Disposing,          // cleanup window
    Done,               // terminal — cleared next tick
}

pub enum OptionPlayResult {
    Trashed,                           // Standard Option fully resolved
    Delayed(PermanentHandle),          // parked as Delayed
    Linked { source: PermanentHandle },// attached to host
    Training(PermanentHandle),         // parked as Training
    Pending,                           // selection owed (e.g. LinkSelectHost)
    Invalid,                           // cost/mask failure
}

// game.rs — single-slot pending state
pub pending_option: Option<PendingOption>;
```

### DUAL cards and Arts Digivolve

DUAL cards are represented as `CardKind::Dual` with explicit `dual.digimon` and
`dual.option` face metadata. Use face-aware helpers such as
`CardSource::option_use_cost`, `CardSource::option_colors`,
`CardSource::digimon_level`, `CardSource::digimon_dp`,
`CardSource::digimon_colors`, and `CardSource::digivolution_costs`; do not read
`play_cost`, `colors`, `level`, `dp`, or `evo_costs` directly when DUAL behavior
depends on a specific face.

When a DUAL card is used as an Option, `PendingOption.source_kind` records the
use source. Arts Digivolve is offered only after true Option use, never after a
direct `[Main]` activation from hand. The optional Arts branch is surfaced as a
`PendingSelection`: PASS declines and sends the Option to normal cleanup; choosing
a legal battle-area or breeding-area target stacks the pending card as a
digivolution card, performs the normal draw and rule check, then fires
`WhenDigivolving` for the new stack.

### `EffectTiming` variants (seven wired)

| Variant | Scope | Fires when |
|---------|-------|-----------|
| `OnUseOption` | Global observer | Any Option card is played (both players' listeners hear it). |
| `OptionMain` | This Option | The played Option's own body — pre-existing variant, now dispatched. |
| `DelayEffect` | This Option | Scheduled turn-end landing for a `Delayed` Option. |
| `OnLink` | Global observer | After a Plug-In attaches to its host. |
| `OnLinkedCardTrashed` | Global observer | A linked card leaves its host via trash (host death, return-to-hand, return-to-deck). Mirrors DCGO `OnLinkCardDiscarded`. |
| `OnUnlink` | Global observer | **Reserved** for clean unlink paths. Rust-engine-specific; DCGO folds unlinks into `OnLinkCardDiscarded` + zone checks. Not yet fired. |
| `OnTrainingTrash` | This Training Option | Fires on the specific `Training` permanent being trashed at the owner's breeding-hatch. |

### `EffectBuilder` methods

```rust
// Standard Option body.
Effect::new(card, EffectTiming::None)
    .option_main()
    .process(|ctx| { ctx.gain_memory(2); })
    .build();

// Delay Option body — trigger is EndOfThisTurn | EndOfYourNextTurn.
Effect::new(card, EffectTiming::None)
    .delay(DelayTrigger::EndOfYourNextTurn)
    .process(|ctx| { ctx.draw(2); })
    .build();

// Plug-In host filter. `cost` is the memory paid to attach.
// `filter(ctx, host_handle) -> bool` gates legal hosts at mask time.
Effect::on_play(card)
    .link(2, |ctx, host| ctx.permanent(host).has_trait("Rocks"))
    .process(|_| {})
    .build();

// Training subtype marker on the body.
Effect::new(card, EffectTiming::None)
    .training()
    .process(|ctx| { /* body */ })
    .build();

// `.linked()` — mark an effect that sideways-inherits onto the host
// while the card is attached. Fires off the host's timing dispatches,
// not the linked card's own. Mutually exclusive with `.inherited()`.
Effect::start_of_your_turn(card)
    .linked()
    .process(|ctx| { ctx.draw(1); })  // "at start of host's turn, draw 1"
    .build();
```

### Worked examples

**1. Standard Option — memory swing**

```rust
use crate::effect::{CardEffect, Effect};

pub struct GainTwoMemory;
impl CardEffect for GainTwoMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .option_main()
            .process(|ctx| { ctx.gain_memory(2); })
            .build()]
    }
}
```

**2. Delay Option — end-of-your-next-turn draw**

```rust
use crate::enums::DelayTrigger;

vec![Effect::on_play(card)
    .delay(DelayTrigger::EndOfYourNextTurn)
    .process(|ctx| { ctx.draw(2); })
    .build()]
```

**3. Plug-In — +2000 DP to a host with the "Rocks" trait**

```rust
vec![
    // The link declaration that attaches this card for 2 memory.
    Effect::on_play(card)
        .link(2, |ctx, host| ctx.permanent(host).has_trait("Rocks"))
        .process(|_| {})                  // no extra body beyond the attach
        .build(),

    // Sideways-inherited DP buff, active while attached.
    Effect::on_play(card)
        .linked()
        .dp_modifier(2000)
        .build(),
]
```

**4. Training Option — body + sideways-inherited effect**

```rust
vec![
    Effect::on_play(card)
        .training()
        .process(|ctx| { ctx.gain_memory(1); })
        .build(),
]
```

### Integration with Phase 7 replacements

- **Standard disposal** fires `WhenWouldBeTrashed` with `ReplacementCause::Cost`. Optional replacements (e.g. a `CannotBeTrashedByEffect` passive on the Option itself — unlikely, but possible) fire normally.
- **Delay expiration** fires `WhenWouldLeaveBattleArea` (super-timing) + `WhenWouldBeDeleted` just like a Digimon's battle death.
- **Plug-In detach on host deletion** — when the host leaves the field, each linked card trashes. V1 does **not** fire `WhenWouldBeTrashed` in the cascade (too recursive during host deletion). This is a known limitation; see constraints below.
- **Training expiration** fires `OnTrainingTrash` as the specific observer, then routes through `delete_permanent_with_cause(Cost)` which dispatches the standard `WhenWouldLeaveBattleArea` / `WhenWouldBeDeleted` replacement windows.

### Phase 8 v1 constraints

1. **Cancel-semantics for non-Permanent trash-replacement subjects.** When a `WhenWouldBeTrashed` replacement with outcome `Cancelled` fires on a `Card` subject (hand-origin) mid-resolution — e.g. a Standard Option's disposal gets cancelled after cost was paid and `OptionMain` already fired — the card returns to owner's hand. The printed-rules outcome is unspecified for this shape (cost was spent, body resolved, but the card rebounds). V1 documents this as hand-return; flagged for spec refinement if a real printed card triggers it.
2. **`Redirected(Deck)` / `Redirected(Hand)` use direct vec manipulation.** Spec §7.3 calls for zone-mover helpers; Phase 8 v1 uses `deck.insert(0, …)` and `hand.push(…)` directly on the Card-subject commit path. This skips any future deck-manipulation observers nested inside the redirect. Acceptable until a printed card surfaces a nested observer; follow-up pass will migrate to the helper surface.
3. **Multi-turn Delays** are not supported. Only `DelayTrigger::EndOfThisTurn` and `DelayTrigger::EndOfYourNextTurn` land in v1. "At the start of each of your next 3 turns" would need an extended trigger model.
4. **Linked-card host-deletion cascade does NOT fire `WhenWouldBeTrashed`.** Too recursive during host deletion; v1 unconditionally trashes each linked card. Marked `TODO(phase-8-followup)` in `combat.rs`. Follow-up if a printed card requires it (none audited today).
5. **Counter-timed Options** (Blast Digivolve Options played during opponent's attack) are deferred to **Phase 9 (Combat Interrupt Completion)**. Phase 2's `.blast_digivolve()` builder plumbing is already in place; Phase 9 wires the activation window.
6. **Nested `PendingSelection::Source` in `OptionMain`** is not supported — shared limitation with Phase 7 Partition/ArmorPurge auto-install. A Standard/Delay/Training Option whose body selects a source off a stacked Digimon needs a `PendingSelection::Source` during `OptionMain` execution; the infrastructure gap is the same one Phase 7 flagged.
7. **Training sideways-inheritance scope is broader than printed rules.** v1 scans any same-owner permanent's timing dispatch for `.linked()` + `.inherited()`-flagged effects on Training cards, rather than restricting to the breeding permanent. Pragmatic interim until `TriggerSource::BreedingArea` exists. Tracked in parity §13.
8. **`ACTION_SPACE_SIZE` unchanged at 2168.** Option plays reuse the existing `PLAY_HAND` action range via a `CardKind`-forked decoder: the action bit is the same as playing a Digimon from hand; the decoder inspects `CardData::card_type` and routes to the Option play pipeline. No tensor or mask regression.

### Testing an Option effect

Per working rule 18, write behavioral tests against `DebugRunner` under `code/code/digimon-engine/tests/option_flow/` before implementing:

```rust
#[test]
fn gain_two_memory_option_drains_and_trashes() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_gain_two_memory_option())
        .start();
    let start_memory = r.game.memory;
    r.play_option_from_hand(0, "GAIN2MEM").unwrap();

    assert_eq!(r.game.memory, start_memory + 2);
    assert!(r.game.pending_option.is_none());           // resolved
    assert_eq!(r.game.player(0).trash.last().map(|c| c.card_id()),
               Some("GAIN2MEM"));                        // card self-trashed
}
```

See `tests/option_flow/behavioral_end_to_end.rs` for the canonical end-to-end template covering multi-turn Delay + Link + Training across a full game.

---

## Phase 9 — Combat Interrupt Completion

Phase 9 completes the combat state machine. Prior to Phase 9, the Counter window only accepted Blast Digivolve Options, the two `Would*`-attack replacement timings were reserved but unfired, `<Raid>` retarget existed as a mask-layer concept only, `<Collision>` printed as a keyword with no enforcement, `<Piercing>` never performed a post-battle security check, `<Reboot>` never unsuspended, and the `OnBlock` / `OnAllyAttack` / `OnOpponentAttack` global observers were declared but not dispatched. Phase 9 wires all of it.

Every interrupt window now exposes its decision node through `pending_selection` (working rule 17). ~30 cards across the five audited archetypes unblock — the entire Dark Masters Ace Counter line, TS Olympos Raid retarget riders, and the Collision-mandate cards across the meta pool.

### Updated attack state machine

```
Declared
  → [WhenWouldAttack]              (replacement: cancel / let attack proceed)
  → [WhenWouldBeAttackTarget]      (replacement: cancel / substitute target)
  → AllianceOpen
  → CounterOpen                    (3 candidate shapes — see §Counter broadening)
  → BlockOpen                      (CannotBlock gates defenders; Collision flips optional → mandatory)
  → PostBlock                      (Raid retarget rider if attacker has <Raid> and target invalidated)
  → Battle
  → PostBattle                     (Piercing post-battle security check if Digimon defender wiped)
  → Cleanup
```

`AttackState::PostBlock` and `AttackState::PostBattle` are net-new states added in Phase 9.

### New replacement timings (Phase 7 variants, dispatched in Phase 9)

Both variants were parsed and built in Phase 7 but never fired. Phase 9 wires the fire-sites at the top of `begin_attack_impl` (attack declaration).

| Timing | Subject | Outcome semantics |
|--------|---------|-------------------|
| `WhenWouldAttack` | `Permanent(attacker)` | `Cancelled` aborts the attack cleanly (no Counter window, no Battle). `Substituted` on attacker-side is a `debug_assert` — no printed card moves the attacker slot; kept for symmetry only. |
| `WhenWouldBeAttackTarget` | `Permanent(target)` or `Player(pid)` | `Cancelled` aborts. `Substituted(new_target)` rewrites `effective_target` and fires `OnAttackTargetChange` before advancing to `AllianceOpen`. |

Both fire before the Alliance window opens, in order. Scripts use them via `Effect::new(card, EffectTiming::WhenWouldAttack)` / `WhenWouldBeAttackTarget` with a `ReplacementContext` body.

### New `EffectBuilder` method

```rust
/// Mark this effect as eligible for the Counter window.
///
/// Distinct from `.blast_digivolve()`: Blast Digivolve implies both Counter-timing
/// AND a digivolve-cost payment path. A plain `.counter()` effect is a Counter-timed
/// body with no digivolve step — used for hand-Option Counter plays and field-ability
/// Counter fires.
pub fn counter(self) -> Self;
```

Three candidate shapes feed into the Counter window:

| Shape | Composition | Dispatch order |
|-------|-------------|----------------|
| **Blast Digivolve** (pre-existing) | `.blast_digivolve()` on a digivolve target | Fires `WhenDigivolving` on the digivolved permanent. |
| **Hand Counter Option** (NEW) | `.counter().option_main()` on an Option card body | `CounterEffect` fires **before** the `OptionMain` body, then the Option resolves through the standard Phase 8 dispose path. |
| **Field Counter Ability** (NEW) | `.counter().timing(CounterEffect)` on a permanent | Fires directly from the permanent's triggered-effect queue during the Counter window. |

**Depth guard**: at most one Counter fires per attack in v1 (`pending_attack.counter_fired` flag set on first Counter commit). See constraints below.

### New `EffectContext` helpers

```rust
/// Redirect the current attack's effective target.
///
/// Only callable during an active attack (otherwise `AttackError::NoActiveAttack`).
/// Validates `new_target` against the current board state (otherwise
/// `AttackError::InvalidTarget` — e.g. a Permanent handle no longer on the field,
/// or a target class disallowed by a modifier).
///
/// Side effect: fires `OnAttackTargetChange` after commit.
pub fn redirect_attack(&mut self, new_target: AttackTarget) -> Result<(), AttackError>;

/// Cancel the current attack. Sets `pending_attack.cancelled = true`; the attack
/// state advance loop short-circuits to `Cleanup` on its next tick.
///
/// Only callable during an active attack (`AttackError::NoActiveAttack` otherwise).
/// Safe to call from `WhenWouldAttack`, `WhenWouldBeAttackTarget`, `CounterEffect`,
/// and any observer that runs during an open attack.
pub fn cancel_attack(&mut self) -> Result<(), AttackError>;
```

```rust
pub enum AttackError {
    NoActiveAttack,   // pending_attack is None
    InvalidTarget,    // target handle stale, destroyed, or class-gated
}
```

### New state machine transitions

**`AttackState::PostBlock`** — after the Block window resolves. The engine checks whether the attacker has `<Raid>` (printed or modifier-granted) AND the effective target has been invalidated since declaration (e.g. destroyed by a Block-window effect, returned to hand, unsuspended such that it's no longer a legal target). If so, the engine scans for a legal retarget; if any exist, it installs a `PendingSelection::AttackRetarget` and pauses advance. The controller picks a new target (or declines, ending the attack). If none exist, advance falls through to `Battle`.

**`AttackState::PostBattle`** — after the Battle state resolves. If the attacker survives, the defender was a Digimon (not a player), the defender was wiped, and the attacker has `<Piercing>`, the engine enters a security check against the defending player (standard `OnSecurityCheck` dispatch; one card). Piercing on direct-player-attack does **not** fire — this is a Piercing-after-Digimon-battle rule only.

### Keyword consumers wired

| Keyword | Consumer site | Behavior |
|---------|---------------|----------|
| `<Piercing>` | `AttackState::PostBattle` | Post-Digimon-battle security check against defending player (§4.3 of spec). |
| `<Collision>` | `AttackState::BlockOpen` mask builder | Flips `is_optional = false` on the block window; the PASS/no-block action bit drops from the mask. `CannotBlock` modifier still gates individual defenders before Collision elevates the choice to mandatory. |
| `<Reboot>` | Opponent's unsuspend phase | Unsuspends the Reboot permanent during the opponent's unsuspend step. Gated by `CannotUnsuspend` / `CannotBeUnsuspendedByEffect` modifiers. |
| `<Raid>` | `AttackState::PostBlock` | Retarget rider as described above. |

### New observer dispatch

| Timing | Scope | Fires when |
|--------|-------|-----------|
| `OnBlock` | Global observer | Fan-out via `TriggerSource::PlayerBattleArea` after block declaration; both players' battle areas scanned. Observers read the post-declare attack state (`effective_target` is the blocker). |
| `OnAllyAttack` | Observer on attacker-controller's OTHER permanents | Fires on every same-controller permanent except the attacker itself. Attacker-filter is structural, not opt-in. |
| `OnOpponentAttack` | Observer on opposing-controller permanents | Fires on every permanent of the opposing controller. |

All three fire after `OnAttack` + `WhenAttacking` resolve, via the standard `drain_effect_queue` path.

### Worked examples

**1. Redirect via `WhenWouldBeAttackTarget`** — "this Digimon redirects any attack declared against it to itself" (trivial tautology form, but shows the surface):

```rust
Effect::new(card, EffectTiming::WhenWouldBeAttackTarget)
    .process(|ctx| {
        // Substitute the attack target back to an ally (example).
        let new_target = AttackTarget::Permanent(ctx.source());
        ctx.redirect_attack(new_target).ok();
    })
    .build()
```

**2. Hand Counter Option body** — play an Option from hand during opponent's attack and gain 2 memory:

```rust
Effect::new(card, EffectTiming::None)
    .counter()
    .option_main()
    .process(|ctx| { ctx.gain_memory(2); })
    .build()
```

`CounterEffect` fires first (the `.counter()` overlay), then `OptionMain` (the Phase 8 body). The Option card disposes through the standard Phase 8 Standard-Option path.

**3. Field Counter Ability** — a permanent that fires a Counter body during the Counter window:

```rust
Effect::new(card, EffectTiming::CounterEffect)
    .counter()
    .process(|ctx| {
        // Cancel the opposing attack.
        ctx.cancel_attack().ok();
    })
    .build()
```

**4. Observing `OnAllyAttack` to buff the attacker**:

```rust
Effect::new(card, EffectTiming::OnAllyAttack)
    .process(|ctx| {
        if let Some(attacker) = ctx.attacker() {
            ctx.add_dp_modifier(attacker, 2000, Expiry::EndOfTurn);
        }
    })
    .build()
```

### Phase 9 v1 constraints

1. **Counter-chain depth > 1 is not supported.** `pending_attack.counter_fired` is a boolean, not a counter. Printed rules today do not require recursive Counter-chains; if a future card requires Counter-in-response-to-Counter, this becomes a multi-level fired-set.
2. **Attacker-side substitution (`WhenWouldAttack` with `Substituted`) is a `debug_assert`**, not a printed mechanic. No card moves the attacker slot via a replacement; the variant is kept for shape symmetry with `WhenWouldBeAttackTarget`.
3. **Single Raid retarget per attack.** `AttackState::PostBlock` fires the retarget check once. If the retargeted target also invalidates before `Battle`, the state machine falls through to `Battle`, which handles zero-target cleanup trivially (no damage, no security check).
4. **Raid retarget candidate set is stricter than declaration-time mask.** v1 prefers unsuspended Digimon; suspended fallback only fires when no unsuspended target exists. Declaration-time mask has no such ordering. This is a parity gap between the two selection flows — tracked in `RUST_PYTHON_PARITY.md` §15.4.
5. **Native `<Raid>` parsing is still modifier-only.** Phase 3 parsed `<Collision>`, `<Piercing>`, `<Reboot>` off the card face; `<Raid>` is still mask-layer-only (same pre-existing gap, not introduced by Phase 9). Printed Raid cards require a modifier emission to honor the retarget rider in v1.
6. **Piercing on direct-player-attack does NOT fire.** Piercing is a post-Digimon-battle rule only. An attack declared against the opposing player (not a Digimon) that lands zero damage is not a Piercing trigger.
7. **`ACTION_SPACE_SIZE` unchanged at 2168.** All Phase 9 interrupts reuse existing action ranges: Counter window Option plays route through the Phase 8 `PLAY_HAND` range, Raid retargets reuse the target-selection range, Counter passes reuse the `SEL_REPLACEMENT_PASS` bit. No tensor or mask growth.

### Testing a combat interrupt

Per working rule 18, write behavioral tests against `DebugRunner` under `code/code/digimon-engine/tests/combat/` before implementing:

```rust
#[test]
fn counter_option_from_hand_cancels_attack() {
    let mut r = DebugRunner::builder()
        .p0_battle_area([attacker])
        .p1_battle_area([defender])
        .p1_hand([counter_cancel_option])
        .start();
    r.declare_attack(AttackTarget::Permanent(defender_handle)).unwrap();
    // Counter window is open — P1 plays the Option.
    r.play_counter_option_from_hand(1, "CNTR-001").unwrap();
    // Attack should have short-circuited to Cleanup.
    assert!(r.game.pending_attack.is_none());
    assert!(r.game.player(1).battle_area.contains_handle(defender_handle));
}
```

See `tests/combat/phase9_end_to_end.rs` for the canonical Counter + Raid + Collision integrated scenario (Task 10).

---

## 9. Known gaps (as of Phase 6)

These are documented in `docs/RUST_ENGINE_GAPS.md`. DSL-only vocabulary and lowering gaps live in `qa/dsl-vocab-gaps.md`. Notable items:

- **Block / Counter / Alliance interrupt phases** are wired through the state machine; trait-gated Alliance is incomplete.
- **OnSecurityCheck** / **OnStartBattle** / **OnEndBattle** / **OnEndAttack** timings — OnSecurityCheck is wired in the attack path; OnStartBattle/OnEndBattle are not yet fired.
- **Security effects** — basic SecuritySkill dispatch is wired; re-entrant selections mid-security-resolve are not (blocks most real security cards with selection effects).
- **BeforePayCost cost reduction scanning** — landed in Phase 5. See §Phase 5 above.
- **Option cards** — full play-flow landed in Phase 8. See §Phase 8 above.
- **Flood-gate + restriction modifiers** (player-scoped) — landed in Phase 6. See §Phase 6 above. Several variants are DORMANT pending enforcement-site wiring.
- **"Would" replacement timings** (Barrier, Evade, Partition, Armor Purge) — landed in Phase 7. See §Phase 7 above.

When implementing a card that needs one of these, log the gap and pick a safe fallback.

For a comprehensive Rust ↔ Python divergence catalog with severity and fix order,
see [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md).

---

## 10. Registering a card

1. Create `code/code/digimon-engine/src/cards/<set>/<card_id>.rs` implementing `CardEffect`.
2. Create/update `code/code/digimon-engine/src/cards/<set>/mod.rs` with a `register` function that calls `registry.insert` for every card in the set.
3. Add `pub mod <set>;` to `code/code/digimon-engine/src/cards.rs`.
4. Call `<set>::register(&mut registry)` inside `cards::build_registry()`.

A card is **not** active until it appears in `build_registry()`.

---

## 11. The two registries & what happens when a new set drops

There are **two separate registries**. Don't confuse them.

### `CardRegistry` — card_id ↔ integer index (for the RL tensor)

Defined in [card_registry.rs](../code/code/digimon-engine/src/card_registry.rs). Built with `CardRegistry::from_cards(&HashMap<String, CardData>)`. Provides:

- `get_index(card_id) -> u16` — integer for tensor encoding. `0` = padding/unknown.
- `get_norm_id(card_id) -> f32` — normalized float for non-embedding tensor slots.
- `get_id(u16) -> Option<&str>` — reverse lookup.

**Parity rule:** when cards.json contains an explicit `index` field on each entry (the production format), the Rust registry uses that value verbatim. This is what Python's `CardRegistry.initialize()` does, and it **must** match — pretrained embeddings, serialized replays, and ONNX models all key off these indices.

When cards.json entries omit `index` (legacy arrays, inline test fixtures), the Rust registry falls back to **alphabetically sorted, 1-based** assignment.

Duplicate indices panic at construction. Missing indices in otherwise-production data are silently skipped (treated as unknown).

### `CardEffectRegistry` — card_id → `Arc<dyn CardEffect>` (for effect scripts)

Defined in [cards.rs](../code/code/digimon-engine/src/cards.rs). Populated at compile time by `build_registry()`, which calls each set's `register()` function.

A missing entry here means the card plays as **vanilla** — no effect, no error.

### When a new set drops (e.g. BT25, 100 new cards)

1. **cards.json gets updated** by the card pipeline. New cards are appended with fresh `index` values (likely 4083..4182). **Existing indices never change.** The Rust `CardRegistry` will pick up the new mappings automatically on next load.

2. **New effect scripts** go into `code/code/digimon-engine/src/cards/bt25/*.rs`. Add `pub mod bt25;` to `cards.rs` and `bt25::register(&mut registry)` to `build_registry()`. Cargo rebuild.

3. **Cards with no script yet** play as vanilla — they're in the `CardRegistry` (so they have tensor positions) but not in the `CardEffectRegistry` (so no effects fire). Perfectly fine; the engine silently no-ops.

4. **Cross-engine parity** is preserved: Python's `CardRegistry.initialize()` reads the same `index` field. Python and Rust feed identical tensor positions to the model.

### What would break parity

- Hand-editing cards.json `index` values after the fact → trained models become garbage.
- Relying on the alphabetical-sort fallback in production (don't — always use explicit indices).
- Two different cards.json files between Python and Rust environments.

The test `tests/card_registry_parity.rs` loads the real cards.json and asserts the Rust mapping matches `CardData.index` for every card. Keep that green.

---

## 12. Setup and mulligan

The setup sequence in `Game::new` + `start_game` matches Python's mulligan flow. If you're writing card effects you almost never touch this — it's relevant mainly when writing tests that want to exercise opening-hand decisions, or when building a UI / RL loop that surfaces the mulligan choice.

### Sequence

1. **`Game::new(decks, cards, rules, seed)`** — builds players, shuffles each deck and digitama via the seeded rng, then:
   - Shuffles `turn_order` (first-player coin flip; deterministic under the same seed).
   - Draws `rules.starting_hand` cards for every player.
   - Initializes `mulligan_pending` (clone of `turn_order`) and `mulligan_used` (all false).
   - Leaves the game in `GamePhase::Mulligan`. **Security is not yet laid.**

2. **Each decider answers.** Either the caller walks through `accept_mulligan`, or a convenience caller invokes `start_game` to auto-keep.

3. **`finalize_mulligan` runs automatically** once the last decider answers. It lays `rules.security_count` security cards per player, sets `turn_count = 1`, `memory = 0`, and calls `begin_turn`.

### API surface

```rust
// Read-only
game.mulligan_current_player() -> Option<PlayerId>
game.mulligan_pending          // Vec<PlayerId> — FIFO of remaining deciders
game.mulligan_used             // Vec<bool> indexed by player id

// Mutating
game.accept_mulligan(player, keep: bool) -> Result<(), &'static str>
//   keep == true  : hand stays as-is, advance to next decider
//   keep == false : hand → deck, reshuffle with game.rng, draw starting_hand,
//                   set mulligan_used[player] = true, advance

// Shorthand
game.start_game()  // auto-keep every pending player, then finalize
```

`accept_mulligan` returns `Err` if:
- The caller passes a player who isn't the current decider (the message contains "different player").
- Mulligan is already complete (message: "already complete").

### Action mask

During `GamePhase::Mulligan`:
- Only the current decider's mask has any bits set. Every other perspective is all zeros.
- Bit 0 is **keep** — always available.
- Bit 1 is **mulligan** — suppressed once `mulligan_used[current] == true`.

### DebugRunner

The common test path bypasses mulligan: `DebugRunner::builder().start()` runs `start_game` which auto-keeps, and `build_inner` clears `mulligan_pending` so the test's explicit zones aren't cannibalized by `setup_security`.

Mulligan-specific tests should construct `Game::new` directly with a real deck (so redraws have cards) and optionally wrap with `DebugRunner::wrap(game)` for the convenience helpers:

```rust
let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
let first = game.mulligan_current_player().unwrap();
game.accept_mulligan(first, /* keep */ false).unwrap(); // redraw
game.start_game(); // auto-keep for the rest
```

Or via DebugRunner:
```rust
let mut r = DebugRunner::wrap(game);
r.mulligan_decide(false)?;     // redraw current decider
r.skip_mulligan();              // auto-keep everyone else
```

### Tauri / UI

The DTO returned from every `rust_*` command carries `mulligan_current_player: Option<PlayerId>` and `mulligan_used: Vec<bool>`. The frontend hides the Mulligan button for any player whose `mulligan_used` is true, and only enables keep/mulligan controls for the player whose id matches `mulligan_current_player`.

`rustMulliganDecide(keep: boolean)` from `code/code/frontend/src/api/rustEngine.ts` applies the decision for the current decider and returns the updated state.

### What's NOT here (yet)

- **Rule-driven mulligan variants** (e.g. redraw fewer cards, double mulligan). Digimon TCG has a single, full-size redraw only.
- **Tensor slot for mulligan context.** The current tensor has no dedicated "who's deciding" slot; if RL needs to learn mulligan policy, we'd extend the selection-context section of the tensor. Not required for current training loops.

---

## 13. OPT (once-per-turn) tracking and tensor DP contributions

Card scripts rarely touch this directly, but two things matter when you write an effect with `max_per_turn > 0` or a static `dp_modifier`:

### Once-per-turn activation counters

`Permanent::effect_activations: HashMap<(CardHandle, u8), u8>` tracks how many times each `(source_card, effect_slot_index)` has fired this turn. The slot index is the effect's position in the `Vec<Effect>` returned by `CardEffect::effects(handle)` — so order your effects intentionally and keep them stable across edits, or the counters key into the wrong slot.

Counters reset in `Permanent::new_turn` (called from `Player::new_turn` inside `Game::begin_turn`). You don't need to call any reset manually.

To gate your own effect firing:
```rust
.condition(|ctx| {
    let Some(h) = ctx.source_permanent else { return false; };
    let perm = &ctx.game.players[h.player as usize].battle_area[h.index as usize];
    perm.activation_count(ctx.source_card, /* my slot */ 0) == 0
})
.process(|ctx| {
    // ... do the work ...
    if let Some(h) = ctx.source_permanent {
        let perm = &mut ctx.game.players[h.player as usize].battle_area[h.index as usize];
        perm.record_activation(ctx.source_card, 0);
    }
})
```

(A `ctx`-level `record_activation` helper would be a nice follow-up; not yet wired.)

### Static `dp_modifier` and the tensor

Use `Effect::declarative(card).dp_modifier(n)` for a non-inherited static buff (e.g. "This Digimon gains +1000 DP"), or `Effect::inherited(card).dp_modifier(n)` for an inherited version (carries up the stack). Add a `.condition(...)` if the buff only applies in certain situations — the condition is evaluated at tensor-build time too, so `"[Your Turn] +3000"` contributes 0 on the opponent's turn.

Avoid encoding DP-change effects via `ctx.add_dp_modifier(...)` for *static* buffs — that writes to `ModifierRegistry`, which the tensor's per-source contributions don't currently sum (permanent-level, yes; per-source, no). Stick with `dp_modifier` on `Effect` for anything you want the tensor to see per source.

### How the tensor reads these

`build_tensor` calls four `Game` helpers per permanent slot:

```rust
game.opt_total(handle)            -> u32        // slot offset +3
game.opt_used(handle)             -> u32        // slot offset +4
game.source_dp_contribution(h, i) -> i32 / f32  // per-source offset +2
game.source_opt_state(h, i)       -> f32        // per-source offset +1
```

Each one iterates effects via `CardEffectRegistry::get(card_id).effects(handle)`, applies the inherited/top filter (`is_under == effect.inherited`), and evaluates conditions through `EffectReadContext`. You can call them yourself in tests or diagnostics.

### Tensor profile metadata

The Rust engine exposes tensor profile metadata from `tensor_profiles/` and observation-profile selection from `observation.rs`. Use these APIs when code needs layout metadata such as card-ID positions, scalar positions, section boundaries, or slot/source fields.

`standard_lite_v2` is the default pilot observation profile selected by `observation::default_observation_profile()`. It is an `8320`-float fair-information tensor with structured board, own-hand, known-zone, decision-context, and pending-choice sections. `standard_compact_v1` remains the compact `1375`-float compatibility and baseline profile.

Be precise about the two defaults:

- `digimon_engine::observation::default_observation_profile()` returns the observation default, currently `standard_lite_v2`.
- `digimon_engine::tensor_profiles::default_profile()` returns the compact registry default, currently `standard_compact_v1`, for compatibility with `tensor.rs` and `TENSOR_SIZE`.

```rust
use digimon_engine::observation::{default_observation_profile, observation_layout};
use digimon_engine::tensor_profiles::{default_profile, STANDARD_COMPACT_V1_PROFILE_ID};

let observation_profile = default_observation_profile();
assert_eq!(observation_profile.as_str(), "standard_lite_v2");

let layout = observation_layout(observation_profile);
assert_eq!(layout.tensor_size, 8320);
assert_eq!(layout.card_id_slot_count, 542);
assert_eq!(layout.scalar_slot_count, 7778);

let compact_profile = default_profile();
assert_eq!(compact_profile.id, STANDARD_COMPACT_V1_PROFILE_ID);
assert_eq!(compact_profile.tensor_size, digimon_engine::tensor::TENSOR_SIZE);
```

`digimon_engine::tensor_profile` remains as a temporary compatibility alias, but new code should use `digimon_engine::tensor_profiles`.

The PyO3 bindings expose both observation-layout metadata and compact compatibility metadata:

```python
import digimon_engine

assert digimon_engine.DEFAULT_OBSERVATION_PROFILE == "standard_lite_v2"

layout = digimon_engine.get_observation_layout("standard_lite_v2")
assert layout["tensor_size"] == 8320
assert len(layout["card_id_positions"]) == 542
assert len(layout["scalar_positions"]) == 7778

compact = digimon_engine.get_observation_layout("standard_compact_v1")
assert compact["tensor_size"] == digimon_engine.TENSOR_SIZE == 1375
```

`digimon_engine.TENSOR_SIZE` and `digimon_engine.TENSOR_PROFILE_ID` remain compact compatibility exports. RL feature extractors should use `digimon_gym.tensor_profiles.get_tensor_profile(profile_id)` or `digimon_engine.get_observation_layout(profile_id)` instead of importing the legacy Python tensor layout directly.

---

## Writing a card effect (TDD walkthrough)

This is the onramp the forthcoming Rust `batch-fix-cards` skill will hand to sub-agents. Authors (human or AI) follow it directly until the skill exists. The flow mirrors the Python `/batch-fix-cards` convention: decompose → test-first → implement → verdict.

Worked example pattern: `code/code/digimon-engine/src/cards/test_cards.rs` (the `TEST-001..TEST-022` structs) with paired tests in `code/code/digimon-engine/tests/test_cards_behavioral.rs`. Read both side-by-side before starting a real card.

### 1. Decompose the card text into numbered clauses

Copy the official card text verbatim. Split on each independent clause and number them. Example for a hypothetical card:

```
[On Play]
  (1) Trash the top card of your deck.
  (2) If that card was a Lv.5 Digimon, gain 1 memory.
```

Each clause becomes a discrete assertion in the test. The numbering stays in comments so the implementation's `process` closure mirrors it.

### 2. Write failing behavioral tests first

Create or extend a test file under `code/code/digimon-engine/tests/`. Use `DebugRunner::builder()` to construct a minimal game state — inject only what the clause exercises. One `#[test]` per clause outcome, including both positive and negative branches.

```rust
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn clause1_trashes_top_of_deck() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("YOUR-001", "Example"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["YOUR-001"])
        .deck(0, &["FILLER"])
        .memory(5)
        .start();

    r.play(0, 0);

    assert_eq!(r.deck_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn clause2_gains_memory_when_trashed_card_is_lv5() {
    // … inject a Lv.5 card at the top of the deck, assert +1 memory …
}

#[test]
fn clause2_no_memory_when_trashed_card_is_not_lv5() {
    // … negative branch …
}
```

`DebugRunner` setup helpers (`.hand`, `.deck`, `.memory`, `.add_card`, `.start`) are the canonical surface — don't reach into `Game` directly from tests.

### 3. Run the tests and confirm they fail

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test your_card_behavioral
```

Expect compile failures (no `CardEffect` impl yet) or assertion failures. A passing test at this point means the test doesn't actually exercise the clause — rewrite it until it fails for the right reason.

### 4. Implement the `CardEffect`

Add a zero-sized struct in `code/code/digimon-engine/src/cards/` (under a set-scoped submodule for real cards — e.g. `src/cards/bt16/bt16_052.rs`). Implement `CardEffect::effects` using the `Effect` builder. Use `EffectContext` for every mutation — never reach into `Game` internals from a `process` closure.

```rust
use std::sync::Arc;
use crate::card_source::CardHandle;
use crate::cards::CardEffectRegistry;
use crate::effect::{CardEffect, Effect};

pub struct YourCard001;

impl CardEffect for YourCard001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Trash top, gain 1 if Lv.5")
            .process(|ctx| {
                // (1) Trash the top card of your deck.
                let me = ctx.player;
                let trashed = ctx.trash_from_top(me, 1);
                // (2) If that card was a Lv.5 Digimon, gain 1 memory.
                if trashed.first().is_some_and(|c| c.level() == Some(5) && c.is_digimon()) {
                    ctx.gain_memory(1);
                }
            })
            .build()]
    }
}
```

Numbered comments inside the closure match the clause decomposition from Step 1.

### 5. Register the effect

Wire the card into the registry. Real cards register from their set module; test cards register from `test_cards::register`. Follow the existing pattern in `code/code/digimon-engine/src/cards.rs` and `code/code/digimon-engine/src/cards/test_cards.rs`.

```rust
// code/digimon-engine/src/cards/bt16/mod.rs
pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("BT16-001", Arc::new(bt16_001::BT16_001));
    // …
}
```

### 6. Run the tests and confirm they pass

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test your_card_behavioral
```

All clause tests green. Then run the full suite to catch regressions:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

### 7. Emit a verdict

Use the same verdict vocabulary as the Python `/batch-fix-cards` skill so the eventual Rust skill inherits it cleanly:

- **IMPLEMENTED** — every clause has a passing test and a faithful implementation.
- **PARTIAL** — some clauses landed; the rest are blocked on a specific engine gap. Document which clauses and why.
- **BLOCKED** — the card requires infrastructure that doesn't yet exist (new `EffectTiming` variant, modifier type, selection kind, etc.). Do not ship a stub. File missing Rust engine primitives in `docs/RUST_ENGINE_GAPS.md`; if the engine primitive exists but the DSL cannot express or lower it, file the gap in `qa/dsl-vocab-gaps.md` and move on.

### No-approximations checklist (Rust)

Before claiming IMPLEMENTED, re-read the card text against the implementation and confirm:

1. No clause is silently dropped.
2. Every player choice uses `ctx.select_*` — never auto-select a target when the card text allows multiple.
3. Optional effects (`(Optional)`, "you may") are modeled with the `optional` builder flag + a declined branch in the test.
4. Memory cost is paid through the standard play/digivolve path, not re-implemented inside `process`.
5. Static keywords (Blitz, Rush, Piercing, Blocker, …) on the face of the card are handled via `grant_keyword` or the appropriate `Keyword` query, not hard-coded booleans.
6. Inherited effects use `Effect::inherited(card)`, not `Effect::on_play(card)` with a manual under-the-stack check.
7. Trait / name matching uses `CardSource::contains_card_name` / trait accessors (case-insensitive), not raw string equality.
8. Every closure is `Send + Sync + 'static` — if you have lifetime errors, you're capturing a borrow; use handles (`Copy`) instead.

---

## DSL Card Authoring Primitives

These YAML primitives are preferred over raw Rust closures for common option legality, immunity, and DP-extrema targeting text.

### DSL Option Use Requirements

Top-level `use_requirement` declares an Option-use permission that can satisfy the normal color requirement when an Option is used from hand:

```yaml
use_requirement:
  any_field_permanent:
    of: you
    trait_has: BEATBREAK
```

`any_field_permanent` scans the player's battle area and breeding area. DUAL option faces may declare their own requirement under `dual.option.use_requirement`; that face-specific requirement is the one used for option-face legality.

### DSL Source-Kind Effect Immunity

Use `grant_effect_immunity` for text like "isn't affected by your opponent's Digimon effects":

```yaml
- grant_effect_immunity:
    target: self
    source_kind: digimon
    source_controller: opponent
    expiry: end_of_opponents_turn
```

`source_kind` supports `digimon`, `tamer`, `option`, and `rule`. `source_controller` supports controller filters such as `opponent` and `you`; inherited effects on a stack are Digimon effects because they belong to the top Digimon.

### DSL DP-Extrema Field Selection

Use a field selection `selector` when card text restricts the target to the lowest-DP or highest-DP eligible permanent:

```yaml
- select_opponent_permanent:
    bind_as: tgt
    filter: { kind: digimon }
    selector: lowest_dp
    prompt: "Choose lowest DP Digimon"
```

Selectors are applied after predicate filtering and at selection install time. Ties remain legal choices; if no filtered candidate has effective DP, no pending selection is installed.

---

## 14. Zone Manipulation (Phase 2)

Added in Phase 2 to support card-script movement between zones (hand, deck, trash, battle area, digivolution stack, security, reveal pool) and effect-initiated plays. All methods live on `EffectContext` and delegate to `Game`-level helpers. Card-moving methods return `Option<PermanentHandle>` or `Option<CardHandle>` for provenance — follow-up effects thread the handle into the next primitive.

### Shared types

```rust
pub enum CostDelta {
    Free,               // pay 0
    Reduce(i16),        // max(0, printed - n); negative n increases cost
    Fixed(i16),         // max(0, n); replaces printed cost
}

pub enum StackPosition { Top, Bottom, Random }

pub enum CardSourceRef {
    Hand(PlayerId, usize),
    Trash(PlayerId, usize),
    DeckTop(PlayerId),
    Reveal(CardHandle),
}
```

### Play from zone

| Method | Purpose |
|--------|---------|
| `play_from_hand_with_cost(player, hand_index, CostDelta) -> Option<PermanentHandle>` | Play from hand at a computed cost. `CostDelta::Free` bypasses printed cost. |
| `play_from_trash_with_cost(player, trash_index, CostDelta) -> Option<PermanentHandle>` | Play from trash. Same cost-delta contract. |

Example — free play from hand inside an OnPlay effect:

```rust
Effect::on_play(card).process(|ctx| {
    ctx.play_from_hand_with_cost(ctx.player, 0, CostDelta::Free);
}).build()
```

### Card movement

| Method | Purpose |
|--------|---------|
| `add_to_hand_from_deck(player, CardHandle)` → `bool` | Move a specific deck card to hand. Does NOT shuffle. |
| `add_to_hand_from_trash(player, CardHandle)` → `bool` | Same, from trash. |
| `add_to_hand_from_reveal(player, CardHandle)` → `bool` | Same, from the reveal pool. |
| `trash_from_hand_by_index(player, hand_index)` → `Option<CardHandle>` | Trash a specific hand slot. |
| `trash_from_reveal(player, CardHandle)` → `bool` | Trash a revealed card. |
| `return_to_hand(PermanentHandle)` → `Option<CardHandle>` | Bounce a permanent: top → hand, sources under → trash. |
| `return_to_deck(PermanentHandle, StackPosition)` → `bool` | Bounce to deck at Top/Bottom/Random. |
| `return_to_deck_from_reveal(player, CardHandle, StackPosition)` → `bool` | Reveal pool → deck. |
| `shuffle_deck(player)` | Pair with `add_to_hand_from_deck` for "search and shuffle" effects. |

### Reveal pool

`reveal_top_deck(player, n) -> Vec<CardHandle>` — move up to N cards from deck top into the transient reveal pool (`game.revealed_cards`, cleared on turn rotation).

`revealed() -> &[CardSource]` — read-only snapshot of the pool. Scripts inspect it to decide follow-up moves.

### Placement

| Method | Purpose |
|--------|---------|
| `place_as_bottom_source(CardSourceRef, target: PermanentHandle)` → `bool` | Insert a card at the bottom of target's digivolution stack. |
| `place_on_security(player, CardSourceRef, StackPosition, face_up: bool)` → `bool` | Move to security stack at Top/Bottom/Random; optionally face-up. |
| `hatch(player) -> bool` | Move top of digitama deck to breeding area. Returns false if breeding is occupied or digitama deck is empty. |
| `effect_initiated_digivolve(player, hand_index, target, CostDelta, ignore_color)` → `bool` | Script-driven digivolve. Validates level match; optionally bypasses color check. Fires WhenDigivolving. |

### No-approximations note

Each of these primitives is a pure movement or cost-payment operation. *Which* card to move is always the caller's responsibility, and the choice must surface through a `PendingSelection` built with `select_hand`, `select_trash`, `select_reveal`, or `select_own_permanent`. Never let a script auto-pick a target without a selection — the RL action space must observe the branch.

---

## Phase 1 — Timing Dispatch

Added in Phase 1 to wire every declared-but-unfired `EffectTiming` variant + 2 new observer variants for Medusamon and Rocks archetypes. Card scripts can now hook into turn phases, combat events, and global observers via dedicated `Effect::*` builders.

### Turn phases

| Timing | Builder | Fire site |
|--------|---------|-----------|
| `StartOfYourTurn` | `Effect::start_of_your_turn(card)` | `begin_turn` (before Unsuspend) |
| `StartOfYourMainPhase` | `Effect::start_of_your_main_phase(card)` | `enter_main_phase` (before phase set to Main) |
| `EndOfYourTurn` | `Effect::end_of_your_turn(card)` | `fire_end_of_your_turn` (already wired) |
| `EndOfOpponentsTurn` | `Effect::end_of_opponents_turn(card)` | `rotate_turn_player` (between EndOfYourTurn drain and turn advance) |

### Combat timings

| Timing | Builder | Fire site |
|--------|---------|-----------|
| `OnAttack` | `Effect::on_attack(card)` | `fire_on_attack` (already wired; per-attacker) |
| `WhenAttacking` | `Effect::when_attacking(card)` | `fire_on_attack` (observer — attacker's battle area) |
| `EndOfBattle` | `Effect::end_of_battle(card)` | `resolve_battle` (Digimon-vs-Digimon only) |
| `EndOfAttack` | `Effect::end_of_attack(card)` | `cleanup_attack` (global) |
| `OnAttackTargetChange` | `Effect::on_attack_target_change(card)` | Block interrupt, after `effective_target` rewrite |

### Global observers

| Timing | Builder | Fire site |
|--------|---------|-----------|
| `OnEnterFieldAnyone` | `Effect::on_enter_field_anyone(card)` | `play_from_hand_with_cost` + `play_from_trash_with_cost` (after OnPlay) |
| `OnAnyDeletion` | `Effect::on_any_deletion(card)` | `delete_permanent_with_effects` (single chokepoint for all deletions) |
| `OnSuspend` | `Effect::on_suspend(card)` | `Game::suspend` (guarded on state change) |
| `OnUnsuspend` | `Effect::on_unsuspend(card)` | `Game::unsuspend` (bulk unsuspend_all does NOT fire — StartOfYourTurn is the canonical turn-start timing) |
| `OnHatch` | `Effect::on_hatch(card)` | `Game::hatch` (after successful hatch) |
| `OnDigivolve` | `Effect::on_digivolve(card)` | After `WhenDigivolving` drains in both digivolve paths |

### New archetype-specific observers

| Timing | Builder | Fire site | Archetype |
|--------|---------|-----------|-----------|
| `OnOpponentSecurityRemoved` | `Effect::on_opponent_security_removed(card)` | `SecurityPhase::Dispose` (attacker's battle area only) | Medusamon core |
| `OnDigivolutionCardTrashed` | `Effect::on_digivolution_card_trashed(card)` | Per-source in `return_to_hand` / `return_to_deck` (sources-below-top, not linked_cards) | Rocks core |

### Scoping

All observer fire sites use `TriggerSource::PlayerBattleArea(PlayerId)` — effects with the given timing in a player's battle area fire.

Per-permanent events (`OnAttack` on the attacker) use `TriggerSource::Permanent(handle)`.

The global observer pattern iterates every player and enqueues per-player; the queue drainer handles turn-order resolution. Scripts can observe events caused by opponents by registering effects with the global timings.

### No-approximations note

Every observer fire site eagerly drains the effect queue, so chained effects resolve before the originating event returns. The no-auto-selection principle (see §2) applies: optional effects registered against these timings must surface as `PendingSelection` branches for RL to observe.

---

## Phase 3 — Native Keyword Parsing

Added in Phase 3 to honor keywords printed on a card's face (not just
modifier-granted keywords). Closes parity §2.1b (native Rush) and §2.5f
(native Jamming).

### CardData surface

`CardData::keywords: Vec<Keyword>` — populated at load time by
`parse_printed_keywords(effect_text, inherited_text, security_text)`.
Parametric keywords (`Security A. ±N`, `De-Digivolve N`, `Draw N`) are
parsed into their typed variants.

### Unified query

`Game::has_keyword(handle, Keyword) -> bool` — the canonical keyword
lookup. Returns true if the permanent has the keyword either printed
natively on its top card OR granted by an active modifier.

**Call-site policy:** engine code never accesses
`game.modifiers.has_keyword(...)` directly — that only sees granted
keywords and would miss native printed keywords. Always use
`game.has_keyword(...)`. All 14 pre-existing keyword check sites
(combat.rs, action/mask.rs, game_phases.rs) migrated in Phase 3.

### Keyword extraction patterns

Keywords appear in card text as `＜Keyword＞` (full-width angle brackets).
The parser recognizes the 19 non-parametric keywords in the `Keyword`
enum plus three parametric patterns:

- `＜Security A. +N＞` / `＜Security A. -N＞` → `SecurityAttackPlus(N)` / `SecurityAttackMinus(N)`
- `＜De-Digivolve N＞` → `DeDigivolve(N)`
- `＜Draw N＞` → `DrawX(N)`

Unrecognized keyword names are ignored silently. Cards that need
behavior not covered by the `Keyword` enum must use the modifier-based
API via `Effect` builders.

---

## Phase 4 — Selection Kinds

Added in Phase 4 to surface ordered permutations, union-zone picks, count-capped multi-selects, and opponent-as-selector flows through `PendingSelection` so the RL action space observes every branch.

All four helpers live in `code/code/digimon-engine/src/effect_context/selections.rs`. Three new `SelectionKind` variants (`UnionZone`, `OrderedPermutation`, `CountCappedMultiSelect`) and three new `GamePhase` variants (`SelectUnion`, `SelectPermutation`, `SelectBudgeted`) are added in `selection.rs` and `enums.rs` respectively. No new action-range constants — all four helpers reuse existing ranges (Python-parity pattern).

Commits: `67e0afa4`..`65f0b3a6` (8 commits). Full suite: **495 passing** (+32 from Phase 3 baseline of 463).

**No-approximations note (applies to all four helpers):** Singleton or trivially-small selections must still surface through `PendingSelection` — never auto-select. A 1-item permutation still installs the selection; a 1-card union-zone still installs it. The RL action space must observe every branch.

---

### `select_union_zone`

```rust
pub fn select_union_zone<F, C>(
    &mut self,
    of_player: PlayerId,
    zones: UnionZoneSet,      // bitset: UnionZoneSet::HAND | UnionZoneSet::TRASH
    prompt: &str,
    is_optional: bool,
    filter: F,
    callback: C,
)
where
    F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, CardHandle) + Send + Sync + 'static,
```

**Semantics.** Installs a single `PendingSelection` that lets the active player choose one card from the player's hand, trash, or both (per the `zones` bitset). The selection reuses existing action ranges — hand picks map to `PLAY_HAND_START + i`, trash picks map to `TRASH_EFFECT_START + i` — so no new action range is needed. The resolver classifies the incoming `action_id` by range and reconstructs the `CardHandle` from the appropriate zone. The callback receives a zone-agnostic `CardHandle`, so call-sites do not need to branch on the source zone.

**Filter signature difference vs `select_hand`/`select_trash`.** The filter here is `Fn(&Game, &CardSource) -> bool` — zone-agnostic. `select_hand` and `select_trash` take `Fn(&Game, usize) -> bool` (index-based). This lets cross-zone predicates (e.g. "any Digimon with level ≥ 5") be expressed without duplicating logic.

**Python parity.** Python's `effect_play_from_zone(player, 'hand_or_trash', ...)` populates both `SEL_HAND_START` and `SEL_TRASH_START` into `valid_indices` on one `PendingSelection`. Rust follows the same range-reuse pattern; the action-space encoding is compatible.

```rust
// "Search your hand or trash for a Digimon card and return it to your hand."
Effect::on_play(card)
    .name("Search hand or trash")
    .process(|ctx| {
        let me = ctx.player;
        ctx.select_union_zone(
            me,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "Choose a Digimon from your hand or trash",
            false,  // not optional
            |_g, cs| cs.is_digimon(),
            |ctx, handle| {
                let me = ctx.player;
                // handle is zone-agnostic; add_to_hand_from_trash / from_hand routes internally
                ctx.add_to_hand_from_trash(me, handle);
                // (hand→hand is a no-op; a complete script would check zone)
            },
        )
    })
    .build()
```

---

### `select_ordered_permutation`

```rust
pub fn select_ordered_permutation<C>(
    &mut self,
    items: Vec<CardHandle>,   // cards to place in order; N_max = 10 (debug_assert)
    prompt: &str,
    callback: C,
)
where
    C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync + 'static,
```

**Semantics.** Lets the player place N items in their chosen order. The implementation is sequential: one `PendingSelection` is installed per slot using `GamePhase::SelectPermutation` and `SelectionKind::OrderedPermutation { remaining }`. Each step's callback re-installs for the next slot with the chosen item removed from `remaining`. After all N picks the final callback fires with `Vec<CardHandle>` in chosen order. The internal state (accumulator + remaining list) is captured in the step closures — no heap-allocated mutex needed.

Each step reuses the `SEL_REVEAL_START` action range; the resolver maps `action - SEL_REVEAL_START` to an index into the `remaining` list. N is capped at 10 (`debug_assert!(items.len() <= 10)`).

**Empty items.** If `items` is empty, the callback fires immediately with an empty `Vec` — no `PendingSelection` is installed.

**Singleton.** A 1-item list still installs a 1-choice selection (no auto-selection — RL sees the branch).

**Python parity.** Python has no ordered-permutation primitive; the closest analog is a sequential multi-pass over a reveal pool (`effect_reveal_and_select_multi`). Rust's sequential re-install pattern is the correct analog. This is effectively net-new RL decision surface.

```rust
// "Return the rest to the bottom of the deck in any order."
// (called after reveal_top_deck has returned the candidate handles)
// Prefer `place_remainder_on_deck` for this pattern (see the next section).
Effect::on_play(card)
    .name("Order deck bottom")
    .process(|ctx| {
        ctx.reveal_top_deck(ctx.player, 4);
        // (select_reveal here to take some cards)
        // Then place the remainder directly:
        ctx.place_remainder_on_deck(ctx.player, StackPosition::Bottom);
    })
    .build()
```

---

### `place_remainder_on_deck`

```rust
pub fn place_remainder_on_deck(
    &mut self,
    player: PlayerId,
    position: StackPosition,
)
```

**Semantics.** A convenience wrapper for the canonical Digimon TCG "scry-and-return" pattern: reveal N cards, take some matching cards (via `select_reveal`), then place the remainder on top or bottom of the deck in player-chosen order.

`place_remainder_on_deck` snapshots all `CardHandle`s currently in `game.revealed_cards` for `player` and calls `select_ordered_permutation` over them. The permutation callback places each card at `position` with the correct iteration direction so `ordered_vec[0]` is drawn first among the placed cards.

**Iteration direction by position:**
- `StackPosition::Top` — reverse iteration + `deck.push()`. `ordered_vec[0]` is pushed last → lands at Vec-end (deck top) → drawn first.
- `StackPosition::Bottom` — forward iteration + `deck.insert(0)`. Each insert at index 0 pushes previous inserts one step higher. Final state: `ordered_vec[0]` occupies the highest index among the placed group (closest to top within the bottom group) → drawn first.
- `StackPosition::Random` — forward iteration; each card is placed at a random position via the single-card helper. The permutation selection is still surfaced — strategically irrelevant but required by the no-approximations policy (§17).

**Empty pool.** If `game.revealed_cards` is empty, the method is a silent no-op — no `PendingSelection` installed, deck unchanged.

**Singleton.** A 1-card remainder still installs a 1-choice `OrderedPermutation` selection (no auto-selection).

**Canonical search pattern (worked example):**

```rust
// Card: "Reveal the top 5 cards of your deck. Add 1 Royal Knight Digimon
//        to your hand. Place the rest on the bottom of your deck in any order."
ctx.reveal_top_deck(p, 5);
ctx.select_reveal(
    "Add 1 Royal Knight to hand",
    false,
    |g, idx| {
        let cs = &g.revealed_cards[idx];
        let data = &g.card_data[cs.data_index];
        data.card_kind == CardKind::Digimon && data.traits.iter().any(|t| t == "Royal Knight")
    },
    move |ctx, _idx| {
        // The selected card is moved to hand inside the callback.
        // (Use add_to_hand_from_reveal for the chosen handle.)
        // Remaining cards stay in revealed_cards for place_remainder_on_deck.
    },
);
// After select_reveal resolves, revealed_cards holds only the unchosen cards.
ctx.place_remainder_on_deck(p, StackPosition::Bottom);
```

**Note on chaining follow-up effects.** `place_remainder_on_deck` installs its own `PendingSelection` callback internally. If the card text requires another selection after the placement (e.g., "…then your opponent chooses a card to trash"), install that selection *after* `place_remainder_on_deck` resolves, in a separate step — not chained inside the same callback. See `code/code/digimon-engine/tests/selection/behavioral_end_to_end.rs` for an example of this two-step pattern.

---

### `select_count_capped_multi`

```rust
pub enum CountCappedZone { Hand, Trash }

pub fn select_count_capped_multi<F, C>(
    &mut self,
    of_player: PlayerId,
    zone: CountCappedZone,    // Hand or Trash
    max: u8,                  // upper bound; debug_assert!(max <= 10)
    prompt: &str,
    is_optional_zero: bool,   // true → player may pick 0; PASS available from first step
    filter: F,
    callback: C,
)
where
    F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, Vec<CardHandle>) + Send + Sync + 'static,
```

**Semantics.** Lets the player pick up to `max` items from a single zone, one pick at a time. Each step uses `GamePhase::SelectBudgeted` / `SelectionKind::CountCappedMultiSelect { max, picked }`. Toggle actions reuse the existing zone range (`PLAY_HAND_START + i` for hand, `TRASH_EFFECT_START + i` for trash). The PASS action (id 62) is the early-commit sentinel; once submitted, the final callback fires with the accumulated `Vec<CardHandle>`.

PASS availability is gated: available when `is_optional_zero || picked >= 1`. Reaching `picked == max` auto-commits (no extra PASS required — the last pick itself finalizes).

**Empty filter.** If no cards pass the filter at install time, the callback fires immediately with an empty `Vec`.

**Python parity.** Python has no clean count-capped multi-select primitive; some Python scripts (e.g. Baalmon) auto-mill N cards without offering a selection, violating the no-approximations policy. Rust must NOT copy this pattern — this helper mandates explicit per-pick actions.

```rust
// "Trash up to 2 cards from your hand."
Effect::on_play(card)
    .name("Trash up to 2 from hand")
    .process(|ctx| {
        let me = ctx.player;
        ctx.select_count_capped_multi(
            me,
            CountCappedZone::Hand,
            2,
            "Choose up to 2 cards to trash",
            true,  // is_optional_zero: player may pass immediately
            |_g, _cs| true,  // all cards eligible
            |ctx, picked| {
                // picked is Vec<CardHandle> in pick order; trash each
                for handle in picked {
                    let me = ctx.player;
                    if let Some(idx) = ctx.my_player().hand.iter()
                        .position(|cs| cs.card_handle() == handle) {
                        ctx.trash_from_hand_by_index(me, idx);
                    }
                }
            },
        )
    })
    .build()
```

---

### `as_selecting_player` builder

```rust
impl<'g> EffectContext<'g> {
    pub fn as_selecting_player(&mut self, player: PlayerId) -> EffectContextSelectorScope<'_, 'g>;
}

pub struct EffectContextSelectorScope<'a, 'g> {
    ctx: &'a mut EffectContext<'g>,
    selecting_player: PlayerId,
}
```

**Semantics.** An opt-in player override for opponent-as-selector flows. `as_selecting_player(opp)` returns a scope that forwards the eight selection helpers listed below — but overrides the `selecting_player` field on the installed `PendingSelection` so the opponent's action mask lights up, not the active player's.

`EffectContextSelectorScope` forwards:

| Method forwarded | Notes |
|---|---|
| `select_own_permanent` | Selects from effect-controller's battle area; opponent is the chooser |
| `select_opponent_permanent` | Selects from selector's battle area (opponent's own Digimon) |
| `select_effect_choice` | Arbitrary N-option branch, chosen by opponent |
| `select_hand` | Picks from a zone of the effect-controller's hand |
| `select_trash` | Same, trash |
| `select_union_zone` | Cross-zone (hand or trash) with opponent as chooser |
| `select_count_capped_multi` | Up-to-N multi-pick with opponent as chooser |
| `select_ordered_permutation` | Permutation ordered by the opponent |

**Not forwarded:** `select_material`, `select_reveal`, `select_security` — these are rarely opponent-driven and have no audited card pattern requiring them; defer until a real card demands it.

**Python parity.** Python has no analog. No Python script calls `request_selection(..., selecting_player=opponent, ...)`. The `selecting_player` field exists on Rust's `PendingSelection` and the mask layer already routes on it. `as_selecting_player` is net-new Rust capability with no Python precedent.

```rust
// "Your opponent chooses one of your Digimon and deletes it."
Effect::on_play(card)
    .name("Opponent trashes one of your Digimon")
    .process(|ctx| {
        let me = ctx.player;
        let opp = ctx.opponent_id();
        ctx.as_selecting_player(opp).select_own_permanent(
            "Opponent: choose one of your Digimon to trash",
            false,  // not optional — must choose
            |_g, perm| perm.is_digimon(),
            |ctx, handle| {
                ctx.delete_permanent(handle);
            },
        );
    })
    .build()
```

Sequential cross-side chaining (one of your Digimon, then one of theirs) is handled by two back-to-back calls — a dedicated "choose both" primitive is not needed:

```rust
// "Your opponent chooses one of your Digimon to return to your hand,
//  then you choose one of their Digimon to suspend."
.process(|ctx| {
    let me = ctx.player;
    let opp = ctx.opponent_id();
    ctx.as_selecting_player(opp).select_own_permanent(
        "Opponent: choose one of your Digimon to return",
        false,
        |_g, _p| true,
        |ctx, handle| {
            ctx.return_to_hand(handle);
            let me = ctx.player;
            let opp = ctx.opponent_id();
            ctx.select_opponent_permanent(
                "Choose one of your opponent's Digimon to suspend",
                false,
                |_g, _p| true,
                |ctx, h| { ctx.suspend(h); },
            );
        },
    );
})
```

## Phase 10 — Tokens & De-Digivolve N

Phase 10 ships two additive `EffectContext` primitives plus a new
`CardKind::Token` variant and its companion `TokenRegistry`.

### `ctx.play_token(controller, token_name) -> Option<PermanentHandle>`

Materializes a token directly on `controller`'s battle area, bypassing
hand / deck / play-cost. Looks up `token_name` in
`game.token_registry`; no-op `None` on unknown name or full field.

**Registered tokens (Phase 10):**

| `token_name`   | `card_id`              | Colors  | DP   | Printed effects                                               |
|----------------|------------------------|---------|------|---------------------------------------------------------------|
| `petrification`| `TOKEN_PETRIFICATION`  | White   | 3000 | [On Deletion] Trash the top card of this Digimon's owner's security stack. <br>(Printed [Your Turn] CannotSuspend rider deferred — depends on condition-gated modifier entries, tracked in `RUST_ENGINE_GAPS.md` §"Condition-gated modifier entries") |
| `familiar`     | `TOKEN_FAMILIAR`       | Yellow  | 3000 | [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn. |

**Worked example** (TEST-023):

```rust
impl CardEffect for PlayPetrificationToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Petrification Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "petrification");
            })
            .build()]
    }
}
```

**Removal-from-game semantics.** When a token leaves the battle area
via `player::delete_permanent`, its `card_sources` and `linked_cards`
are dropped (removed from the game) rather than appended to the
owner's trash. OnDeletion effects still fire from the
`effect_queue` observer path before the card leaves the game. No
equivalent hook yet exists for return-to-hand or return-to-deck —
those zone-manipulation primitives are deferred to a later phase
(see `RUST_ENGINE_GAPS.md` §"Zone-manipulation: return-to-hand /
return-to-deck").

### `ctx.de_digivolve(target, stop_at_level, amount) -> u8`

Pops up to `amount` sources off `target`'s digivolution stack,
trashing each into the target owner's trash. Respects a level floor
when `stop_at_level = Some(L)`. Returns the actual count popped.

**Arguments:**
- `target: PermanentHandle` — the opponent (or self) permanent to
  de-digivolve.
- `stop_at_level: Option<u8>` — stop early if popping would leave a
  top whose level is strictly below this value. `Some(3)` for
  standard "you can't trash past level 3 cards" wording. `None`
  for TS Olympos Ikkakumon-style unbounded pop.
- `amount: Option<u8>` — cap on number of pops. `None` = unbounded
  (bounded only by stack depth and the level floor).

**Worked examples:**

```rust
// Standard "De-Digivolve 2" — pop up to 2, stop at Lv3.
ctx.de_digivolve(target, Some(3), Some(2));

// "De-Digivolve 4" — pop up to 4, stop at Lv3.
ctx.de_digivolve(target, Some(3), Some(4));

// TS Olympos Ikkakumon: pop until base.
ctx.de_digivolve(target, None, None);
```

**Invariants:**
- The base card (`card_sources[0]`) is never popped —
  `Permanent::stack_size() >= 1` is preserved.
- Popped sources always go to the **target owner**'s trash, not the
  caller's. This matters for Dark Masters' cross-side effects.
