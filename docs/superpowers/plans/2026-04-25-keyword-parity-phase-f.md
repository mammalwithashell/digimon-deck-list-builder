# Keyword Parity Phase F — Remaining Backfill + Scapegoat UX Fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the four remaining DCGO keywords missing from the Rust enum (`Execute`, `Iceclad`, `MindLink`, `Training`) and close the Phase E Scapegoat outer-dialog UX divergence by threading `cause` into the candidate-condition evaluation.

**Architecture:** Five logically-independent landings on top of the Phase B/C/D/E substrate.

1. **Scapegoat substrate fix** adds a `replacement_condition` closure on `Effect` (cause + read-context aware). Re-mounts `<Scapegoat>` so the outer "may" dialog only parks when (`cause != OwnEffect`) AND at least one substitute candidate exists — matching DCGO `CanActivateScapegoat`.
2. **`Keyword::Execute`** auto-installs an `EndOfYourTurn` triggered effect on the carrier that grants `MayAttack` + `CanAttackUnsuspended` for the upcoming end-of-turn attack window and queues an `EndOfAttack` self-deletion observer.
3. **`Keyword::Iceclad`** is consumed in `combat::resolve_battle` — a hard-coded branch swaps the DP compare for a `card_sources.len()` compare when either combatant has Iceclad, with a security-battle exception (RULES_CONTEXT 16-34).
4. **`Keyword::MindLink`** auto-installs a `MainOnField` active skill on Tamers that picks an own Digimon with no non-face-down Tamer source and tucks the Tamer underneath via `place_card_under_permanent_bottom`.
5. **`Keyword::Training`** auto-installs a `MainOnField` active skill (battle area + breeding area) that suspends self and places the controller's deck-top under self, face-down. Adds a `face_down: bool` field on `CardSource` (defaults `false`; only Training sets it). Extends the `[Main]`-on-permanent dispatcher to honor breeding-area carriers for this keyword.

**Tech Stack:** Rust 2021; `digimon-engine` library crate; `cargo test --manifest-path digimon-engine/Cargo.toml` test loop; behavioral tests via `DebugRunner`.

**Reference docs to keep open:**
- Spec: [docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md](../specs/2026-04-24-dcgo-keyword-parity-design.md) §5 Phase F + §6 + §10 + §"Scapegoat outer-dialog UX — substrate gap"
- Parity doc: [docs/DCGO_KEYWORD_PARITY.md](../../DCGO_KEYWORD_PARITY.md)
- Rules manual: [docs/RULES_CONTEXT.md](../../RULES_CONTEXT.md) §16 (16-12 Retaliation, 16-27 MindLink, 16-31 Scapegoat, 16-34 Iceclad, 16-37 Execute, 16-40 Training)
- DCGO sources (in `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/`): `Execute.cs`, `Iceclad.cs`, `MindLink.cs`, `Training.cs`, `Scapegoat.cs`
- Phase D analogues: `digimon-engine/src/cards/keyword_effects.rs:846` (MaterialSave — `MainOnField` active skill, own-Tamer pick + source-pick + `place_card_under_permanent_bottom`), `:614` (Fortitude — `OnDeletion` observer), `:1066` (Scapegoat — current Phase E impl with in-body cause filter)
- Phase E analogues: `digimon-engine/src/effect_context/mod.rs::battle_opponent_of` (read-only accessor on `EffectReadContext`)
- Substrate: `digimon-engine/src/replacement.rs::collect_candidates` (the candidate-condition evaluation site that needs `cause` threading), `digimon-engine/src/effect.rs:443` (`.condition(...)` builder), `digimon-engine/src/game_actions.rs:1044` (`activate_field_main` — battle-area `[Main]` dispatcher)

**Working directory:** Execute in the existing `claude/vigorous-elgamal-453703` worktree (already 13 commits ahead of `origin/main` from Phase E).

---

## File Structure

**Create:**
- `digimon-engine/tests/keyword_phase_f/main.rs` — module index
- `digimon-engine/tests/keyword_phase_f/helpers.rs` — `plain_digimon`, `plain_tamer`, plus an `attach_face_down_source` helper for MindLink tests
- `digimon-engine/tests/keyword_phase_f/scapegoat_cause_filter.rs` — Task 1 tests
- `digimon-engine/tests/keyword_phase_f/execute.rs` — Task 3 tests
- `digimon-engine/tests/keyword_phase_f/iceclad.rs` — Task 4 tests
- `digimon-engine/tests/keyword_phase_f/mind_link.rs` — Task 5 tests
- `digimon-engine/tests/keyword_phase_f/training.rs` — Task 6 tests

**Modify:**
- `digimon-engine/src/effect.rs` — new `replacement_condition: Option<...>` field on `Effect`, new `.replacement_condition(...)` builder method
- `digimon-engine/src/replacement.rs::collect_candidates` (the `push_from_perm` closure, ~line 388-417) — thread `cause` into a new conditional that consults `effect.replacement_condition` after the existing `effect.condition` check
- `digimon-engine/src/cards/keyword_effects.rs` (Scapegoat arm at ~line 1066) — drop the in-body cause filter; add `.replacement_condition(...)` with cause + candidate gating
- `digimon-engine/src/enums.rs:273-326` — add `Execute`, `Iceclad`, `MindLink`, `Training` keyword variants
- `digimon-engine/src/card_data.rs:297-321` — add four parser entries (longest-prefix order: `"Mind Link"` MUST appear before `"Mindl"`-anything if any later keyword starts with it; verify ordering)
- `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect` — add three new arms (Execute, MindLink, Training; Iceclad gets NO arm — see Task 4)
- `digimon-engine/src/combat.rs::resolve_battle` (~line 2133) — add the Iceclad branch
- `digimon-engine/src/card_source.rs:11-23` — add `face_down: bool` field on `CardSource` (default `false`); update `new` / `new_token`; do NOT modify serialization shape unless required (the field can be `#[serde(default)]`)
- `digimon-engine/src/permanent.rs` — add `Permanent::has_non_facedown_tamer_source(&self, data) -> bool` helper for MindLink filter
- `digimon-engine/src/effect_context/mod.rs` — add three new primitives:
  - `attach_tamer_to_digimon(tamer: PermanentHandle, digimon: PermanentHandle)` — MindLink body
  - `training_place_deck_top_under_self_face_down(perm: PermanentHandle)` — Training body
  - (Optional but recommended) a `select_own_battle_or_breeding_permanent` selection helper IF MindLink/Training need to address breeding-area carriers via the same selection primitive — verify against existing `select_own_permanent` first; the existing one is battle-area only.
- `digimon-engine/src/game_actions.rs::activate_field_main` (~line 1044) and the matching mask emitter in `digimon-engine/src/action/mask.rs:339` — extend to also dispatch `MainOnField` effects on the breeding-area permanent of the active player when the carrier has `Keyword::Training` (gated narrowly to avoid surfacing other `MainOnField` effects from breeding)
- `digimon-engine/Cargo.toml` (~line 109) — add `[[test]] name = "keyword_phase_f"`
- `docs/DCGO_KEYWORD_PARITY.md` — flip Execute / Iceclad / MindLink / Training rows to ✅; update Scapegoat row to drop the "Known UX divergence" caveat
- `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md` — add a "Phase F ✅ landed" block with deliverables and any spec deviations
- `docs/RUST_ENGINE_GAPS.md` — record the new face-down field on `CardSource` if it represents a tracked capability

**No changes to:** `game.rs`, `selection.rs`, `effect_queue.rs` (substrate is reused as-is for Tasks 1, 3, 5; Task 4 is combat-only; Task 6 extends `game_actions.rs` and `action/mask.rs`).

---

## Task 1: Substrate — `replacement_condition` builder + Scapegoat re-mount

**Why first:** The Scapegoat fix is the smallest of the five landings, exercises the smallest surface (no new keyword, no parser change, no combat-resolution branch), and proves out the `replacement_condition` mechanism in isolation. Tasks 3-6 are independent and can ship in any order after this.

**Files:**
- Modify: `digimon-engine/src/effect.rs:22-100` (struct + builder), `digimon-engine/src/effect.rs:316-608` (builder methods)
- Modify: `digimon-engine/src/replacement.rs:357-417` (the `push_from_perm` closure inside `collect_candidates`)
- Modify: `digimon-engine/src/cards/keyword_effects.rs:1066-1110` (the Scapegoat arm)
- Test: `digimon-engine/tests/keyword_phase_f/scapegoat_cause_filter.rs` (new)

### Step 1.1: Add the `ReplacementConditionFn` type alias and field on `Effect`

In `digimon-engine/src/effect.rs`, immediately after the existing `pub type ConditionFn = ...` (~line 11), add:

```rust
/// Replacement-effect candidate-filter closure. Evaluated in
/// `replacement::collect_candidates` after `condition` for `WhenWouldBe*`
/// timings, with `cause` threaded in. Returns `true` to keep the candidate
/// in the dispatch list, `false` to skip — used by `<Scapegoat>` to suppress
/// the outer "may" dialog when the deletion cause is `OwnEffect` (RULES_CONTEXT
/// 16-31) and to suppress the dialog when there are no substitute candidates
/// (mirrors DCGO `CanActivateScapegoat`'s `HasMatchConditionPermanent` gate).
///
/// Distinct from `condition`:
/// - `condition` is cause-agnostic and evaluated for every effect timing.
/// - `replacement_condition` is cause-aware and ONLY consulted by the
///   replacement dispatcher when collecting candidates.
///
/// If both are set on a single Effect, both must return true for the
/// candidate to be kept — `condition` runs first.
pub type ReplacementConditionFn =
    Box<dyn Fn(&EffectReadContext, crate::replacement::ReplacementCause) -> bool
        + Send + Sync + 'static>;
```

In the `pub struct Effect` definition (~line 22-100), add the field next to `condition`:

```rust
    pub condition: Option<ConditionFn>,
    pub replacement_condition: Option<ReplacementConditionFn>,  // ← new
    pub process: Option<ProcessFn>,
```

In `EffectBuilder::new` (~line 327-360), initialize the new field next to `condition: None`:

```rust
                condition: None,
                replacement_condition: None,  // ← new
                process: None,
```

### Step 1.2: Add the `.replacement_condition(...)` builder method

In the `impl EffectBuilder` block (~line 320-608), immediately after the existing `pub fn condition(...)` method (~line 443-449), add:

```rust
    /// Attach a cause-aware candidate filter for `WhenWouldBe*` replacements.
    /// See `ReplacementConditionFn` doc comment for semantics.
    pub fn replacement_condition(
        mut self,
        f: impl Fn(&EffectReadContext, crate::replacement::ReplacementCause) -> bool
            + Send + Sync + 'static,
    ) -> Self {
        self.inner.replacement_condition = Some(Box::new(f));
        self
    }
```

### Step 1.3: Thread `cause` into `replacement::collect_candidates`

In `digimon-engine/src/replacement.rs`, locate the `push_from_perm` closure inside `collect_candidates` (~line 373-417). Find the existing `condition` check:

```rust
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(game, source_card, Some(h), h.player);
                if !cond(&ctx) {
                    continue;
                }
            }
            let _ = cause; // cause_filter on card effects is Task 6 scope
```

Replace it with:

```rust
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(game, source_card, Some(h), h.player);
                if !cond(&ctx) {
                    continue;
                }
            }
            // Phase F: cause-aware candidate filter for WhenWouldBe* timings.
            // Resolves the prior "Task 6 scope" comment. Used by <Scapegoat>
            // (skip OwnEffect cause + skip when no substitute candidates).
            if let Some(rcond) = &effect.replacement_condition {
                let ctx = EffectReadContext::new(game, source_card, Some(h), h.player);
                if !rcond(&ctx, cause) {
                    continue;
                }
            }
```

The `let _ = cause;` discard line is removed. `cause` is now consumed.

### Step 1.4: Re-mount Scapegoat with the cause-aware candidate filter

In `digimon-engine/src/cards/keyword_effects.rs`, locate the Scapegoat arm (~line 1066-1110). Replace the entire body comment block + closure with:

```rust
        // Phase E §E2 + Phase F §F (Scapegoat UX fix) — printed Scapegoat:
        // "When this Digimon would be deleted [other than by your own effect],
        // you may delete another of your Digimon to prevent it." DCGO
        // `Scapegoat.cs`. RULES_CONTEXT 16-31 (Immediate-type, Optional).
        //
        // ## Cause filter (Phase F — moved out of body)
        //
        // `cause != OwnEffect` and ≥1 substitute candidate exists. Evaluated
        // by the replacement dispatcher's candidate filter via the new
        // `.replacement_condition(...)` builder, BEFORE the outer "may"
        // dialog parks. This matches DCGO `CanActivateScapegoat` which
        // pre-filters both halves and is the substrate fix referenced in
        // the Phase E spec deviations.
        //
        // ## Selection chain
        //
        //   1. Outer optional accept dialog ("may"). Only parks when the
        //      replacement_condition returns true. PASS leaves the original
        //      deletion to proceed.
        //   2. On ACCEPT: parked own-permanent pick via
        //      `rctx.effect.select_own_permanent(...)`. Filter: same-
        //      controller, non-self. Mandatory once accepted.
        //   3. On pick: `ctx.substitute_replacement(Permanent(picked))`
        //      writes `Substituted` to the parked slot. The dispatcher's
        //      post-callback hook commits the substituted deletion.
        Keyword::Scapegoat => vec![Effect::when_would_be_deleted(card)
            .name("<Scapegoat>")
            .optional()
            .replacement_condition(|ctx, cause| {
                use crate::replacement::ReplacementCause;
                // Cause filter: skip OwnEffect.
                if matches!(cause, ReplacementCause::OwnEffect) {
                    return false;
                }
                // Candidate-existence gate: at least one OTHER same-controller
                // permanent must exist (otherwise the inner pick has no valid
                // target and DCGO's CanActivateScapegoat returns false).
                let Some(me) = ctx.source_permanent else { return false; };
                let owner = me.player;
                let has_other = ctx.battle_area(owner)
                    .iter()
                    .enumerate()
                    .any(|(i, _)| i as u8 != me.index);
                has_other
            })
            .replacement_process(|rctx| {
                use crate::replacement::ReplacementSubject;

                // Self-scope guard: WhenWouldBeDeleted enumerates effects on
                // the deletion subject's permanent only, so this is naturally
                // self-scoped. Belt-and-suspenders defense remains.
                let me_perm = match rctx.effect.source_permanent {
                    Some(h) => h,
                    None => return,
                };
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if subject != me_perm {
                    return;
                }

                let owner = me_perm.player;
                rctx.effect.select_own_permanent(
                    "select another of your Digimon to delete instead",
                    /*is_optional=*/ false,
                    move |_g, h| h.player == owner && h != me_perm,
                    move |ctx, picked| {
                        ctx.substitute_replacement(ReplacementSubject::Permanent(picked));
                    },
                );
            })
            .build()],
```

Notes for the executor:
- `ctx.battle_area(owner)` already exists on `EffectReadContext` — verify by reading `effect_context/mod.rs`. If the accessor name differs, use the equivalent (`ctx.game.players[owner as usize].battle_area`).
- The redundant in-body `cause` check is dropped because the dispatcher now skips the candidate before the body ever runs.

### Step 1.5: Bootstrap the Phase F test crate scaffold

This step also primes the Cargo.toml entry that Tasks 3-6 will need. Mirrors the Phase D/E layout exactly.

Create `digimon-engine/tests/keyword_phase_f/main.rs`:

```rust
mod execute;
mod helpers;
mod iceclad;
mod mind_link;
mod scapegoat_cause_filter;
mod training;
```

Create empty placeholder files (so `mod` resolves) until later tasks fill them:
- `digimon-engine/tests/keyword_phase_f/execute.rs` (empty)
- `digimon-engine/tests/keyword_phase_f/iceclad.rs` (empty)
- `digimon-engine/tests/keyword_phase_f/mind_link.rs` (empty)
- `digimon-engine/tests/keyword_phase_f/training.rs` (empty)

Create `digimon-engine/tests/keyword_phase_f/helpers.rs` mirroring Phase D's helpers:

```rust
//! Shared fixtures + helpers for Phase F behavioral tests.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

pub fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

pub fn plain_tamer(id: &str) -> CardData {
    CardData {
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        ..plain_digimon(id)
    }
}

pub fn digimon_with_keywords(id: &str, level: u8, dp: i32, kws: Vec<Keyword>) -> CardData {
    CardData {
        level: Some(level),
        dp: Some(dp),
        keywords: kws,
        ..plain_digimon(id)
    }
}

pub fn tamer_with_keywords(id: &str, kws: Vec<Keyword>) -> CardData {
    CardData {
        keywords: kws,
        ..plain_tamer(id)
    }
}

/// Append a `face_down` digivolution source under the given permanent's
/// top — used by MindLink tests to verify the filter ignores face-down Tamer
/// sources. The card is added directly via `card_sources.insert(0, ...)`
/// (bottom of stack) so the existing top remains visible.
pub fn attach_face_down_source(r: &mut DebugRunner, target_player: u8, target_index: usize, card_id: &str) {
    let data_index = r.game.card_data.iter().position(|c| c.card_id == card_id)
        .expect("card_id not registered");
    let next_card_index = r.game.next_card_index();
    let mut source = CardSource::new(data_index, target_player, next_card_index);
    source.face_down = true;
    let perm = r.game.players[target_player as usize].battle_area
        .get_mut(target_index).expect("target permanent");
    perm.card_sources.insert(0, source);
}
```

Note for executor: `r.game.next_card_index()` may not exist — if not, use whatever convention the Phase D `push_source_card` helper uses (read `tests/keyword_phase_d/fragment_n.rs:78-90` for the pattern).

In `digimon-engine/Cargo.toml` (~line 109, after the `keyword_phase_e` entry):

```toml
[[test]]
name = "keyword_phase_f"
path = "tests/keyword_phase_f/main.rs"
```

### Step 1.6: Write the Scapegoat cause-filter behavioral tests

In `digimon-engine/tests/keyword_phase_f/scapegoat_cause_filter.rs`:

```rust
//! Phase F Task 1 — Scapegoat outer-dialog UX fix tests.
//!
//! Promotes the Phase E known-divergence (outer dialog parks even on
//! `OwnEffect` cause and even with zero candidates) to ✅ via the new
//! `.replacement_condition(...)` builder. Tests verify that:
//!
//!   1. `OwnEffect` deletion of a Scapegoat carrier installs NO outer
//!      dialog (the candidate is filtered before parking).
//!   2. `OpponentEffect` deletion with zero other own permanents installs
//!      NO outer dialog (no substitute candidate).
//!   3. `OpponentEffect` deletion WITH a substitute installs the outer
//!      dialog as before — happy path regression.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;

use super::helpers::{digimon_with_keywords, plain_digimon};

#[test]
fn scapegoat_skips_outer_dialog_on_own_effect_cause() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-SCAPE", 3, 3000, vec![Keyword::Scapegoat],
    ));
    r.register_card_data(plain_digimon("TEST-OTHER"));

    let scape = r.place_on_field(0, "TEST-SCAPE", Some(0));
    let _other = r.place_on_field(0, "TEST-OTHER", Some(0));

    // Trigger an own-effect deletion: the controller's own engine path,
    // e.g. via a hand-rolled `delete_permanent_with_cause` from a test.
    r.game.delete_permanent_with_cause(scape, ReplacementCause::OwnEffect);

    // Phase E behavior parked an outer dialog on OwnEffect. Phase F filters
    // the candidate before parking — pending_selection must be None.
    assert!(
        r.game.pending_selection.is_none(),
        "Scapegoat must not park outer dialog on OwnEffect cause"
    );
    // The deletion must have proceeded — the carrier is gone.
    assert!(
        r.game.players[0].battle_area.iter().all(|p| p.top_card().handle() != scape_top_handle(&r, scape)),
        "Scapegoat carrier should have been deleted"
    );
}

#[test]
fn scapegoat_skips_outer_dialog_when_no_other_candidate() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-SCAPE", 3, 3000, vec![Keyword::Scapegoat],
    ));

    let scape = r.place_on_field(0, "TEST-SCAPE", Some(0));

    // OpponentEffect cause but no other permanents to substitute → DCGO
    // CanActivateScapegoat returns false → no outer dialog.
    r.game.delete_permanent_with_cause(scape, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "Scapegoat must not park outer dialog when no substitute candidates exist"
    );
}

#[test]
fn scapegoat_parks_outer_dialog_on_opponent_effect_with_candidate() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-SCAPE", 3, 3000, vec![Keyword::Scapegoat],
    ));
    r.register_card_data(plain_digimon("TEST-OTHER"));

    let scape = r.place_on_field(0, "TEST-SCAPE", Some(0));
    let _other = r.place_on_field(0, "TEST-OTHER", Some(0));

    // OpponentEffect cause WITH a substitute candidate present.
    r.game.delete_permanent_with_cause(scape, ReplacementCause::OpponentEffect);

    // Outer dialog must park as before.
    let pending = r.game.pending_selection.as_ref()
        .expect("Scapegoat must park outer dialog on OpponentEffect with ≥1 candidate");
    assert_eq!(pending.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    assert!(pending.is_optional);

    // PASS to decline — original deletion proceeds.
    r.game.resolve_selection(0, PASS).expect("PASS resolves outer dialog");
}

// `scape_top_handle` helper — derive the carrier's top CardHandle for the
// post-deletion absence check. Implementation lives inline since this is a
// one-off test utility.
fn scape_top_handle(r: &DebugRunner, _scape: digimon_engine::permanent::PermanentHandle)
    -> digimon_engine::card_source::CardHandle
{
    // The Phase E test pattern uses the equivalent — copy from
    // `tests/keyword_phase_e/scapegoat.rs`.
    // ... (executor: pull the helper used in the Phase E tests)
    unimplemented!("see tests/keyword_phase_e/scapegoat.rs for the pattern")
}
```

Note for executor: the `scape_top_handle` helper is a copy from the Phase E tests. Read `digimon-engine/tests/keyword_phase_e/scapegoat.rs` and reuse the existing pattern verbatim — the empty placeholder above is a guide, not authoritative.

### Step 1.7: Run, verify, commit

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_f
cargo test --manifest-path digimon-engine/Cargo.toml
```

Expected: all three Scapegoat tests pass; no Phase E regression.

```bash
git add digimon-engine/src/effect.rs digimon-engine/src/replacement.rs \
        digimon-engine/src/cards/keyword_effects.rs \
        digimon-engine/tests/keyword_phase_f/ \
        digimon-engine/Cargo.toml
git commit -m "feat(engine): cause-aware replacement_condition + Scapegoat re-mount"
```

---

## Task 2: Add Phase F keyword variants to the enum + parser

Bundling the four enum + parser additions into one task because they are mechanical and zero-risk (no behavior wired yet — Tasks 3-6 install the bodies).

**Files:**
- Modify: `digimon-engine/src/enums.rs:273-326` (Keyword enum)
- Modify: `digimon-engine/src/card_data.rs:295-321` (parser longest-prefix table)
- Modify: `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect` — extend the catch-all `_ => Vec::new(),` arm (the new variants will fall through this for now)

### Step 2.1: Add the four enum variants

In `digimon-engine/src/enums.rs`, in the `pub enum Keyword` block (~line 273-326), after the existing Phase E variants `Retaliation` / `Scapegoat`, add:

```rust
    /// DCGO `Execute` — at end of your turn, this Digimon may attack
    /// (including unsuspended Digimon); when the attack ends, this
    /// Digimon is deleted. Trigger-type, Optional (RULES_CONTEXT 16-37).
    /// Wire-up Phase F §F1 — auto-installed `EndOfYourTurn` triggered
    /// effect that grants `MayAttack` + `CanAttackUnsuspended` for the
    /// upcoming attack window and queues an `EndOfAttack` self-deletion.
    Execute,

    /// DCGO `Iceclad` — passive (RULES_CONTEXT 16-34). When this Digimon
    /// is in a Digimon-vs-Digimon battle (NOT a security-Digimon battle),
    /// compare digivolution-card count instead of DP. Higher count wins;
    /// equal count = mutual destruction. No `keyword_to_auto_effect` arm —
    /// consumed directly in `combat::resolve_battle` via `has_keyword`.
    Iceclad,

    /// DCGO `MindLink` — active skill on Tamers (RULES_CONTEXT 16-27).
    /// `[Main]` activation: place this Tamer at the bottom of an own
    /// Digimon's digivolution stack. Target Digimon must have NO Tamer
    /// digivolution sources (DCGO `cardSource.IsTamer && !cardSource.IsFlipped`
    /// — face-down Tamer sources do NOT count, hence the `face_down` field
    /// on `CardSource`). Mandatory processing; optional timing under `[Main]`.
    /// Wire-up Phase F §F3.
    MindLink,

    /// DCGO `Training` — active skill (RULES_CONTEXT 16-40). `[Main]`
    /// activation usable from battle area OR breeding area: suspend self
    /// (cost) + place top deck card under self at stack bottom, face-down.
    /// Cost requires `is_suspended == false`. Wire-up Phase F §F4.
    Training,
```

### Step 2.2: Add the four parser entries

In `digimon-engine/src/card_data.rs` (~line 297-321), add to the longest-prefix table. Critical ordering rule: longer prefixes first when keywords share a stem. Verify by reading the existing comment "longest-prefix wins":

```rust
            for (prefix, kw) in [
                ("Blast Digivolve", Keyword::BlastDigivolve),
                ("Blocker", Keyword::Blocker),
                ("Rush", Keyword::Rush),
                ("Jamming", Keyword::Jamming),
                ("Piercing", Keyword::Piercing),
                ("Reboot", Keyword::Reboot),
                ("Blitz", Keyword::Blitz),
                ("Armor Purge", Keyword::ArmorPurge),
                ("Raid", Keyword::Raid),
                ("Alliance", Keyword::Alliance),
                ("Save", Keyword::Save),
                ("Fortitude", Keyword::Fortitude),
                ("Overclock", Keyword::Overclock),
                ("Barrier", Keyword::Barrier),
                ("Evade", Keyword::Evade),
                ("Decode", Keyword::Decode),
                ("Decoy", Keyword::Decoy),
                ("Partition", Keyword::Partition),
                ("Vortex", Keyword::Vortex),
                ("Collision", Keyword::Collision),
                ("Progress", Keyword::Progress),
                ("Retaliation", Keyword::Retaliation),
                ("Scapegoat", Keyword::Scapegoat),
                // Phase F additions:
                ("Mind Link", Keyword::MindLink),  // ← MUST come before any "Mind*"
                ("Iceclad", Keyword::Iceclad),
                ("Execute", Keyword::Execute),
                ("Training", Keyword::Training),
            ] {
```

Card text uses `<Mind Link>` with a space (verify by `grep "<Mind" data/cards.json` — DCGO's classfile is `MindLink.cs` but the printed text retains the space).

### Step 2.3: Add a parser unit test

In `digimon-engine/src/card_data.rs`, locate the existing `parse_retaliation_and_scapegoat` test and add an analogous one:

```rust
#[test]
fn parse_phase_f_keywords() {
    let inputs = [
        ("<Execute>", Keyword::Execute),
        ("<Iceclad>", Keyword::Iceclad),
        ("<Mind Link>", Keyword::MindLink),
        ("<Training>", Keyword::Training),
    ];
    for (text, expected) in inputs {
        let parsed = parse_printed_keywords(text);
        assert!(
            parsed.contains(&expected),
            "expected {:?} in parse of {:?}, got {:?}",
            expected, text, parsed,
        );
    }
}
```

### Step 2.4: Verify exhaustive matches still compile

Adding enum variants will surface `non_exhaustive` errors at any `match Keyword { ... }` site that doesn't have a wildcard. Run:

```bash
cargo build --manifest-path digimon-engine/Cargo.toml
```

Expected: build succeeds. The `keyword_to_auto_effect` match has `_ => Vec::new(),` — so the new variants fall through harmlessly until Tasks 3, 5, 6 wire them.

If any other `match Keyword` site errors, address those by either (a) adding a wildcard arm (preferred when behavior is "no-op for these"), or (b) adding explicit no-op arms (when the executor judges per-variant clarity outweighs concision). Do not change behavior here.

### Step 2.5: Commit

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/card_data.rs
git commit -m "feat(engine): add Execute/Iceclad/MindLink/Training Keyword variants + parser"
```

---

## Task 3: Execute keyword auto-install

Mirrors DCGO `Execute.cs:18-87`. The end-of-turn-attack pattern is the same one Blitz/Vortex/Overclock use today; reference those for the modifier-grant idioms.

**Files:**
- Modify: `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect`
- Test: `digimon-engine/tests/keyword_phase_f/execute.rs`

### Step 3.1: Read the existing end-of-turn-attack patterns

Before writing the body, the executor MUST read `digimon-engine/src/cards/test_cards.rs` and `digimon-engine/src/game_phases.rs:253-347` to see how `Vortex` / `Overclock` / `Blitz` integrate with the `EndOfTurnAction` phase and the `MayAttack` modifier. The Execute body needs to produce the same engine-visible state at end-of-turn:

1. The carrier is `MayAttack`-modified (so the end-of-turn-attack mask emits an attack action for it).
2. The carrier is `CanAttackUnsuspended`-modified (so the mask emits attack bits against unsuspended opp Digimon).
3. An `EndOfAttack` triggered effect on the carrier deletes self after the attack resolves.

Do NOT add a new mechanism if these primitives already exist. The DCGO impl uses `UntilEndAttackEffects` — our analog is timed modifiers with an "until end of next attack" duration OR an `EndOfAttack` triggered observer.

### Step 3.2: Write the Execute auto-install arm

In `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect`, immediately before the catch-all `_ => Vec::new(),`, add:

```rust
        // Phase F §F1 — printed Execute: "At the end of your turn, you may
        // attack with this Digimon. (Including unsuspended Digimon.) When
        // the attack ends, delete this Digimon." DCGO `Execute.cs`.
        // RULES_CONTEXT 16-37 (Trigger-type, Optional).
        //
        // ## Mechanism
        //
        // At `EndOfYourTurn`, install temporary modifiers on self for the
        // upcoming end-of-turn-attack window:
        //   - `MayAttack` — the end-of-turn-attack mask emitter sees this
        //     and exposes the attack action.
        //   - `CanAttackUnsuspended` — the mask emitter relaxes the
        //     "target must be suspended" condition that's normally implied
        //     by the EndOfTurnAction phase (matches DCGO's
        //     `CanAttackTargetDefendingPermanentClass` with
        //     `defenderCondition: !defender.IsSuspended`).
        //
        // Plus an `EndOfAttack` observer on self that calls
        // `delete_permanent_with_cause(self, OwnEffect)` — DCGO
        // `PermanentEffectFactory.DeleteSelfEffect`. Cause = OwnEffect
        // (this is the keyword's own effect deleting self, NOT a battle
        // outcome).
        //
        // ## Optionality
        //
        // The keyword printed effect is "you MAY attack" — DCGO returns
        // `CanActivateExecute` from `PermanentOfThisCard().CanAttack(...)`,
        // and the `[End of Your Turn]` activation parks at the standard
        // end-of-turn-attack player-choice prompt. Our `MayAttack` modifier
        // is by definition an optional grant (the controller may decline
        // the attack mask bit), so no explicit `.optional()` flag is
        // needed on the EndOfYourTurn effect — the optionality lives at
        // the action-mask level.
        //
        // ## Self-delete on no-attack
        //
        // DCGO sequences the deletion via `UntilEndAttackEffects`; if the
        // controller declines the attack (or the attack never resolves),
        // the deletion does NOT fire. Match this: install the
        // EndOfAttack observer ONLY when the attack actually starts. Hook
        // via an `OnAttack` self-observer that, when fired, queues the
        // EndOfAttack self-delete.
        //
        // (executor: if the existing engine has a simpler "until-end-of-attack"
        // duration or a built-in self-delete-after-attack helper, prefer
        // it. Reference `cards/test_cards.rs` for analogous Vortex/Blitz
        // self-modifier patterns.)
        Keyword::Execute => vec![Effect::end_of_your_turn(card)
            .name("<Execute>")
            .process(|ctx| {
                use crate::enums::ModifierType;
                let Some(me) = ctx.source_permanent else { return; };
                // Grant MayAttack + CanAttackUnsuspended for this end-of-turn
                // window. Use whatever duration helper Vortex/Blitz use; if
                // there's a "until end of turn" or "until end of next attack"
                // duration, that's the right one.
                ctx.add_modifier(me, ModifierType::MayAttack, /*value=*/ 0);
                ctx.add_modifier(me, ModifierType::CanAttackUnsuspended, /*value=*/ 0);
                // Note: the above modifiers must be cleaned up when the
                // attack window closes. Reference Vortex/Blitz cleanup at
                // `game_phases.rs::EndOfTurnAction` — the modifier_registry
                // should already handle "until end of turn" expiry. If not,
                // queue an EndOfTurn observer to remove these.
            })
            .build(),
            // Companion: the self-delete on attack end. Mounts as an
            // EndOfAttack observer on self; gates on having actually
            // attacked this turn (hand-rolled marker or a flag the
            // pending_attack stash sets — see executor judgment).
            Effect::end_of_attack(card)
                .name("<Execute self-delete>")
                .condition(|ctx| {
                    // Only fire if THIS Digimon is the attacker that just
                    // ended its attack — guards against unrelated EndOfAttack
                    // events (e.g. a different Digimon attacking).
                    let Some(me) = ctx.source_permanent() else { return false; };
                    let Some(pa) = ctx.game.pending_attack.as_ref() else { return false; };
                    pa.attacker == me
                })
                .process(|ctx| {
                    use crate::replacement::ReplacementCause;
                    let Some(me) = ctx.source_permanent else { return; };
                    ctx.game.delete_permanent_with_cause(me, ReplacementCause::OwnEffect);
                })
                .build(),
        ],
```

Important note for executor: this is a SKETCH. The exact API for `add_modifier` (3-arg vs builder-style), the duration argument, and whether `EndOfAttack` fires for the attacker specifically all need to be verified against the existing engine. Read these references before finalizing:

- `digimon-engine/src/effect_context/mod.rs::add_modifier` (signature)
- `digimon-engine/src/cards/test_cards.rs` (search for `MayAttack`)
- `digimon-engine/src/modifiers.rs` (duration enum)
- `digimon-engine/src/game.rs::pending_attack` (the field used by Phase E's `battle_opponent_of`)

If `MayAttack` modifier doesn't currently support an "until end of turn" duration, EITHER (a) extend the duration enum (small substrate addition), OR (b) install the modifier and a paired `EndOfYourTurn`-after-attack cleanup. (b) is closer to the current pattern.

### Step 3.3: Write Execute behavioral tests

In `digimon-engine/tests/keyword_phase_f/execute.rs`:

```rust
//! Phase F Task 3 — `Keyword::Execute` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Execute]` (no hand-rolled
//! `CardEffect`) must, at end of its controller's turn:
//!
//!   1. Surface an attack action against an UNSUSPENDED opp Digimon (DCGO
//!      `Execute.cs:24-49` — `defenderCondition: !defender.IsSuspended`).
//!   2. After the attack resolves (regardless of outcome), delete self.
//!   3. If the controller declines to attack, self is NOT deleted (DCGO's
//!      `UntilEndAttackEffects` only fires when an attack actually
//!      occurred).

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::{digimon_with_keywords, plain_digimon};

#[test]
fn execute_attack_unsuspended_then_self_delete() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-EXEC", 5, 5000, vec![Keyword::Execute],
    ));
    r.register_card_data(plain_digimon("TEST-DEFENDER"));

    let _atk = r.place_on_field(0, "TEST-EXEC", Some(0));
    let _def = r.place_on_field(1, "TEST-DEFENDER", Some(0));
    // Defender is unsuspended (default).

    r.advance_to_end_of_turn(0);
    // Mask must surface an attack-unsuspended bit for the Execute carrier.
    // (executor: use DebugRunner's mask-inspection helper; the exact API may
    // be `r.action_mask_for_player(0)` or `r.mask().attack_bits(...)`.)
    // ...
    r.attack_digimon(/*from=*/ 0, /*to_player=*/ 1, /*to_index=*/ 0);

    // After attack resolves, the Execute carrier must be in trash.
    assert!(
        r.game.players[0].battle_area.iter().all(|p|
            p.top_card().card_id(&r.game.card_data) != "TEST-EXEC"),
        "Execute carrier should self-delete after attack resolves"
    );
}

#[test]
fn execute_no_self_delete_when_attack_declined() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-EXEC", 5, 5000, vec![Keyword::Execute],
    ));

    let _atk = r.place_on_field(0, "TEST-EXEC", Some(0));
    r.advance_to_end_of_turn(0);
    r.game.resolve_selection(0, PASS).expect("decline end-of-turn attack");

    // Carrier still on field — no self-delete.
    assert!(
        r.game.players[0].battle_area.iter().any(|p|
            p.top_card().card_id(&r.game.card_data) == "TEST-EXEC"),
        "Execute carrier must NOT self-delete when attack is declined"
    );
}

#[test]
fn execute_attack_target_filter_includes_unsuspended() {
    // (executor: write a test verifying the attack mask DOES surface
    // unsuspended targets when the Execute carrier is the attacker —
    // distinguishing from the default end-of-turn-attack mask which would
    // require suspended targets.)
}
```

Note for executor: `DebugRunner::advance_to_end_of_turn` and `attack_digimon` may have different exact names. Read `digimon-engine/src/debug_runner.rs` to get the actual API. If a helper is missing, write the equivalent inline using the existing primitives.

### Step 3.4: Run, verify, commit

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_f execute
cargo test --manifest-path digimon-engine/Cargo.toml
```

Expected: Execute tests pass; full suite green.

```bash
git add digimon-engine/src/cards/keyword_effects.rs \
        digimon-engine/tests/keyword_phase_f/execute.rs
git commit -m "feat(engine): printed <Execute> keyword auto-install"
```

---

## Task 4: Iceclad combat-resolution branch

`Iceclad` is the only Phase F keyword that does NOT auto-install via `keyword_to_auto_effect`. It's a hard-coded combat-resolution branch.

**Files:**
- Modify: `digimon-engine/src/combat.rs::resolve_battle` (~line 2133-2191)
- Test: `digimon-engine/tests/keyword_phase_f/iceclad.rs`

### Step 4.1: Add the Iceclad branch to `resolve_battle`

In `digimon-engine/src/combat.rs::resolve_battle`, after the `let a_dp = self.effective_dp(attacker)...` and `let d_dp = self.effective_dp(defender)...` lines (~line 2141-2142), insert the Iceclad swap. Replace the existing body:

```rust
    fn resolve_battle(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.battle_occurred = true;
        }
        let a_dp = self.effective_dp(attacker).unwrap_or(0);
        let d_dp = self.effective_dp(defender).unwrap_or(0);

        let outcome = if a_dp > d_dp { /* ... */ };
```

With:

```rust
    fn resolve_battle(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        use crate::enums::Keyword;
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.battle_occurred = true;
        }

        // Phase F §F2 — Iceclad (RULES_CONTEXT 16-34): when EITHER combatant
        // has Iceclad in a Digimon-vs-Digimon battle, compare digivolution-
        // card count (`card_sources.len()`) instead of DP. The security-
        // battle exception is naturally honored because `resolve_battle` is
        // only called for Digimon-vs-Digimon (security battles route through
        // `resolve_player_security_loop`).
        //
        // Tie path: mutual destruction (matches the DP-tie path below).
        // DCGO `Iceclad.cs` registers an `IcecladStaticEffect` consulted by
        // the combat resolver — we collapse that registration into a direct
        // `has_keyword` query at the resolver site.
        let iceclad_active =
            self.has_keyword(attacker, Keyword::Iceclad)
            || self.has_keyword(defender, Keyword::Iceclad);

        let (a_value, d_value) = if iceclad_active {
            // card_sources.len() includes the top Digimon itself. DCGO's
            // `DigivolutionCards` excludes the top, but for comparison the
            // offset cancels (both sides include the +1 top), so length is
            // the correct compare metric.
            let a_count = self.players[attacker.player as usize]
                .battle_area
                .get(attacker.index as usize)
                .map(|p| p.card_sources.len() as i32)
                .unwrap_or(0);
            let d_count = self.players[defender.player as usize]
                .battle_area
                .get(defender.index as usize)
                .map(|p| p.card_sources.len() as i32)
                .unwrap_or(0);
            (a_count, d_count)
        } else {
            (a_dp, d_dp)
        };

        let outcome = if a_value > d_value {
            self.delete_permanent_with_cause(
                defender,
                crate::replacement::ReplacementCause::Battle,
            );
            AttackResult::AttackerWins
        } else if a_value < d_value {
            self.delete_permanent_with_cause(
                attacker,
                crate::replacement::ReplacementCause::Battle,
            );
            AttackResult::DefenderWins
        } else {
            // Tie path — mutual destruction. Matches the existing DP-tie
            // branch and RULES_CONTEXT 16-34 ("if both Digimon have the
            // same number of digivolution cards, both are deleted").
            self.delete_permanent_with_cause(
                defender,
                crate::replacement::ReplacementCause::Battle,
            );
            if self.handle_valid(attacker) {
                self.delete_permanent_with_cause(
                    attacker,
                    crate::replacement::ReplacementCause::Battle,
                );
            }
            AttackResult::MutualDestruction
        };

        // EndOfBattle dispatch (unchanged from prior version).
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::EndOfBattle,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        outcome
    }
```

### Step 4.2: Verify `has_keyword` reads through grant-modifiers

Confirm `Game::has_keyword` consults both printed `card_data.keywords` and any `Grant*` modifiers. Iceclad does NOT have a `GrantIceclad` modifier today (the spec doesn't add one), so the printed-only path is sufficient. If a future card grants Iceclad temporarily, add `GrantIceclad` then; out of Phase F scope.

### Step 4.3: Iceclad behavioral tests

In `digimon-engine/tests/keyword_phase_f/iceclad.rs`:

```rust
//! Phase F Task 4 — `Keyword::Iceclad` combat-branch tests.
//!
//! When EITHER combatant has Iceclad, `resolve_battle` swaps the DP compare
//! for a `card_sources.len()` compare. RULES_CONTEXT 16-34. Security-Digimon
//! battles are NOT affected (they route through `resolve_player_security_loop`,
//! not `resolve_battle`).

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::{digimon_with_keywords, plain_digimon};

#[test]
fn iceclad_higher_card_count_wins_despite_lower_dp() {
    let mut r = DebugRunner::new();
    // Iceclad attacker, low DP, but more digivolution sources.
    r.register_card_data(digimon_with_keywords(
        "TEST-ICECLAD-ATK", 5, 1000, vec![Keyword::Iceclad],
    ));
    // High-DP plain defender with fewer sources.
    r.register_card_data(plain_digimon("TEST-DEF"));

    let atk = r.place_on_field(0, "TEST-ICECLAD-ATK", Some(0));
    let def = r.place_on_field(1, "TEST-DEF", Some(0));

    // Stack two extra sources under the attacker (total 3 vs defender's 1).
    // Use `attach_face_down_source` or its non-face-down equivalent; for
    // this test face-down doesn't matter — len() counts both.
    // (executor: copy the push-source-card pattern from
    //  tests/keyword_phase_d/fragment_n.rs:78-90)

    r.attack_digimon(0, 1, 0);

    // Defender deleted; attacker survives.
    assert!(r.game.handle_valid(atk), "Iceclad attacker survives");
    assert!(!r.game.handle_valid(def), "defender deleted by Iceclad win");
}

#[test]
fn iceclad_equal_count_mutual_destruction() {
    // Both sides have Iceclad and equal source counts → both deleted.
    // ...
}

#[test]
fn iceclad_lower_count_loses_despite_higher_dp() {
    // High-DP Iceclad attacker with 1 source vs plain defender with 3 sources.
    // Attacker loses despite DP advantage — Iceclad swap takes precedence.
    // ...
}

#[test]
fn iceclad_does_not_affect_security_battle() {
    // Iceclad attacker attacks player; security reveal triggers a security
    // Digimon battle. The security battle path uses DP compare regardless of
    // Iceclad. Verify the Iceclad attacker loses to a higher-DP security
    // Digimon despite having more sources.
    // (executor: use `attack_player` and stack a security Digimon with
    //  higher DP but lower source count.)
}
```

### Step 4.4: Run, verify, commit

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_f iceclad
cargo test --manifest-path digimon-engine/Cargo.toml
```

```bash
git add digimon-engine/src/combat.rs digimon-engine/tests/keyword_phase_f/iceclad.rs
git commit -m "feat(engine): printed <Iceclad> combat-resolution branch"
```

---

## Task 5: MindLink — face-down filter helper + auto-install

**Files:**
- Modify: `digimon-engine/src/card_source.rs:11-23` — add `face_down: bool` (default `false`)
- Modify: `digimon-engine/src/permanent.rs` — add `has_non_facedown_tamer_source` helper
- Modify: `digimon-engine/src/effect_context/mod.rs` — add `attach_tamer_to_digimon` primitive
- Modify: `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect` — add `MindLink` arm
- Test: `digimon-engine/tests/keyword_phase_f/mind_link.rs`

### Step 5.1: Add the `face_down` field to `CardSource`

In `digimon-engine/src/card_source.rs`, modify the struct (~line 11-23):

```rust
#[derive(Debug, Clone)]
pub struct CardSource {
    pub data_index: usize,
    pub owner: PlayerId,
    pub card_index: u16,
    pub is_token: bool,
    pub also_treated_as: Vec<String>,
    /// `true` when this source was placed face-down (DCGO `IsFlipped` analog
    /// for digivolution-stack sources). Set only by `<Training>` (Phase F);
    /// false for all other sources. Consulted by `<Mind Link>`'s "no Tamer
    /// source" filter (DCGO `MindLink.cs:25` — face-down Tamer sources do
    /// NOT count). Out of scope for face-up reveal mechanics; if a future
    /// effect needs to flip these face-up, add a `flip_face_up` primitive.
    pub face_down: bool,
}
```

Update `CardSource::new` and `CardSource::new_token` to initialize `face_down: false`. If the struct is `Serialize`/`Deserialize`, mark the new field `#[serde(default)]` so existing recordings deserialize cleanly.

### Step 5.2: Add the `has_non_facedown_tamer_source` helper

In `digimon-engine/src/permanent.rs`, add a method on `Permanent`:

```rust
    /// Returns `true` if this permanent's digivolution stack contains at
    /// least one Tamer source that is NOT face-down. Used by the `<Mind Link>`
    /// candidate filter — DCGO `MindLink.cs:25`:
    /// `cardSource.IsTamer && !cardSource.IsFlipped`.
    ///
    /// The top card itself is included in the scan; the top of a Tamer
    /// permanent is by definition a non-face-down Tamer source, so a Tamer
    /// permanent is correctly excluded as a target by this helper (a Tamer
    /// is its own controller's Tamer; MindLink should not target Tamers).
    pub fn has_non_facedown_tamer_source(&self, data: &[CardData]) -> bool {
        self.card_sources
            .iter()
            .any(|src| src.is_tamer(data) && !src.face_down)
    }
```

### Step 5.3: Add the `attach_tamer_to_digimon` primitive

In `digimon-engine/src/effect_context/mod.rs`, immediately after `place_card_under_permanent_bottom` (~line 1189-1208), add:

```rust
    /// Place `tamer`'s top card at the bottom of `digimon`'s digivolution
    /// stack, replicating DCGO `MindLink.cs:71-79`:
    /// `IPlacePermanentToDigivolutionCards(new[] { tamer, selectedDigimon })`.
    ///
    /// The Tamer permanent itself is removed from battle area; its top
    /// CardSource becomes the new bottom of the target Digimon's stack.
    /// The face-down flag is NOT set (MindLink places face-up).
    ///
    /// Used by: `<Mind Link>` keyword auto-install (Phase F Task 5).
    pub fn attach_tamer_to_digimon(
        &mut self,
        tamer: PermanentHandle,
        digimon: PermanentHandle,
    ) {
        // Remove the tamer permanent and grab its top card.
        let removed = match self.game.player_mut(tamer.player)
            .battle_area
            .get(tamer.index as usize)
        {
            Some(_) => self.game.player_mut(tamer.player).battle_area.remove(tamer.index as usize),
            None => return,
        };
        // The Tamer permanent's stack becomes the bottom of the target's
        // stack. In practice DCGO Tamers have a single source (no
        // digivolution); place the top card under the target.
        let top = match removed.card_sources.into_iter().last() {
            Some(t) => t,
            None => return,
        };
        let target = self.game.player_mut(digimon.player);
        if let Some(target_perm) = target.battle_area.get_mut(digimon.index as usize) {
            target_perm.card_sources.insert(0, top);
        }
        // (executor: cleanup of any modifiers on the removed Tamer handle —
        // mirror what `commit_permanent_deletion`/`finalize_permanent_deletion`
        // do for the modifier registry. If the Tamer had attached modifiers,
        // they need to be removed.)
    }
```

### Step 5.4: Add the `MindLink` auto-install arm

In `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect`, add:

```rust
        // Phase F §F3 — printed Mind Link: `[Main]` active skill on Tamers.
        // "Place this Tamer at the bottom of one of your Digimon's
        // digivolution stack. Target Digimon must have no Tamer cards in its
        // digivolution stack (face-down Tamer sources don't count)." DCGO
        // `MindLink.cs`. RULES_CONTEXT 16-27.
        //
        // ## Activation gate
        //
        //   1. Self is a Tamer on battle area.
        //   2. Controller has ≥1 own Digimon with no non-face-down Tamer
        //      source (DCGO `cardSource.IsTamer && !cardSource.IsFlipped`).
        //
        // ## Body
        //
        // Optional pick (DCGO `canNoSelect: true`, line 60). Player selects
        // the target Digimon; on pick, `attach_tamer_to_digimon(self, picked)`.
        //
        // ## Self-scope
        //
        // The keyword is printed on the Tamer; the `[Main]` mask emission
        // iterates the Tamer's stack at `activate_field_main`, so the
        // auto-install fires only on the Tamer carrying the keyword.
        Keyword::MindLink => vec![Effect::declarative(card)
            .name("<Mind Link>")
            .timing(EffectTiming::MainOnField)
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else { return false; };
                if !perm.is_tamer(&ctx.game.card_data) {
                    return false;
                }
                let owner = ctx.player;
                ctx.battle_area(owner)
                    .iter()
                    .enumerate()
                    .any(|(i, p)| {
                        i as u8 != perm.index_within_battle_area().unwrap_or(255)
                            && !p.is_tamer(&ctx.game.card_data)
                            && !p.has_non_facedown_tamer_source(&ctx.game.card_data)
                    })
            })
            .process(move |ctx| {
                let Some(me) = ctx.source_permanent else { return; };
                let owner = me.player;
                ctx.select_own_permanent(
                    "select a Digimon to receive the Mind Link Tamer",
                    /*is_optional=*/ true,
                    move |g, h| {
                        if h.player != owner || h == me {
                            return false;
                        }
                        let Some(p) = g.players[h.player as usize]
                            .battle_area.get(h.index as usize) else { return false; };
                        !p.is_tamer(&g.card_data)
                            && !p.has_non_facedown_tamer_source(&g.card_data)
                    },
                    move |ctx, picked| {
                        ctx.attach_tamer_to_digimon(me, picked);
                    },
                );
            })
            .build()],
```

Note for executor: `ctx.source_permanent()` returns the live `Permanent` (read-context); `ctx.source_permanent` (no parens) is the `PermanentHandle` field. There may be no `index_within_battle_area()` on `Permanent` — if so, replace the index check with the existing pattern from `MaterialSave` (it uses `perm.index` directly via the handle).

### Step 5.5: MindLink behavioral tests

In `digimon-engine/tests/keyword_phase_f/mind_link.rs`:

```rust
//! Phase F Task 5 — `Keyword::MindLink` auto-install behavioral tests.
//!
//! A Tamer card declaring ONLY `keywords: vec![Keyword::MindLink]` (no
//! hand-rolled `CardEffect`) must, when activated as a `[Main]` skill:
//!
//!   1. Surface in the action mask only when the controller has ≥1 own
//!      Digimon with no non-face-down Tamer source.
//!   2. Park an optional pick selection (DCGO `canNoSelect: true`).
//!   3. On pick, remove the Tamer permanent and place its card at the
//!      bottom of the chosen Digimon's stack.
//!   4. NOT surface a Digimon whose stack already contains a non-face-down
//!      Tamer source.
//!   5. SURFACE a Digimon whose stack contains only face-down Tamer sources.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::{
    attach_face_down_source, plain_digimon, plain_tamer, tamer_with_keywords,
};

#[test]
fn mind_link_attaches_tamer_to_digimon_with_no_tamer_source() {
    let mut r = DebugRunner::new();
    r.register_card_data(tamer_with_keywords("TEST-TAM", vec![Keyword::MindLink]));
    r.register_card_data(plain_digimon("TEST-DGM"));

    let _tamer = r.place_on_field(0, "TEST-TAM", Some(0));
    let dgm = r.place_on_field(0, "TEST-DGM", Some(0));

    // Activate MindLink. (executor: identify the action_id from the mask.)
    r.activate_field_main(0, /*tamer_index=*/ 0);
    // Pick selection parks; pick the Digimon.
    r.resolve_pick(/*player=*/ 0, /*target=*/ dgm);

    // Tamer permanent removed; Digimon's stack has the Tamer at bottom.
    assert!(r.game.players[0].battle_area.iter().all(|p|
        p.top_card().card_id(&r.game.card_data) != "TEST-TAM"));
    let dgm_perm = &r.game.players[0].battle_area[/*dgm idx=*/ 0];
    assert_eq!(dgm_perm.card_sources[0].card_id(&r.game.card_data), "TEST-TAM");
}

#[test]
fn mind_link_skips_digimon_with_existing_tamer_source() {
    // Plant a non-face-down Tamer source under the Digimon. MindLink's
    // condition gate must fail — no `[Main]` activation surfaces.
    // ...
}

#[test]
fn mind_link_targets_digimon_with_only_facedown_tamer_source() {
    let mut r = DebugRunner::new();
    r.register_card_data(tamer_with_keywords("TEST-TAM", vec![Keyword::MindLink]));
    r.register_card_data(plain_digimon("TEST-DGM"));
    r.register_card_data(plain_tamer("TEST-OLDTAM"));

    let _tamer = r.place_on_field(0, "TEST-TAM", Some(0));
    let _dgm = r.place_on_field(0, "TEST-DGM", Some(0));
    // Plant a face-down Tamer source under the Digimon.
    attach_face_down_source(&mut r, 0, /*dgm idx=*/ 1, "TEST-OLDTAM");

    // MindLink should still surface this Digimon as a target — face-down
    // Tamer sources don't count.
    let mask = r.action_mask(0);
    // ... (executor: assert the MindLink activation bit IS set)
}
```

### Step 5.6: Run, verify, commit

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_f mind_link
cargo test --manifest-path digimon-engine/Cargo.toml
```

```bash
git add digimon-engine/src/card_source.rs digimon-engine/src/permanent.rs \
        digimon-engine/src/effect_context/mod.rs \
        digimon-engine/src/cards/keyword_effects.rs \
        digimon-engine/tests/keyword_phase_f/mind_link.rs
git commit -m "feat(engine): printed <Mind Link> keyword auto-install + face_down source flag"
```

---

## Task 6: Training — primitive + auto-install + breeding-area dispatch

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` — add `training_place_deck_top_under_self_face_down`
- Modify: `digimon-engine/src/cards/keyword_effects.rs::keyword_to_auto_effect` — add `Training` arm
- Modify: `digimon-engine/src/game_actions.rs::activate_field_main` (~line 1044) — extend to ALSO dispatch Training from breeding area when the carrier has `Keyword::Training`
- Modify: `digimon-engine/src/action/mask.rs:339` — extend the `[Main]` mask emitter to surface Training-bearing breeding-area carriers
- Test: `digimon-engine/tests/keyword_phase_f/training.rs`

### Step 6.1: Add the `training_place_deck_top_under_self_face_down` primitive

In `digimon-engine/src/effect_context/mod.rs`:

```rust
    /// DCGO `Training.cs:25-30`: pop the controller's deck top card and
    /// append it at the BOTTOM of `perm`'s digivolution stack, marked
    /// face-down. If the deck is empty, no-op.
    ///
    /// `perm` must be either a battle-area or breeding-area permanent of
    /// the controller; the helper does not enforce this — the caller (the
    /// `<Training>` auto-install) gates via `pay_cost_fn` on suspension.
    ///
    /// Used by: `<Training>` keyword auto-install (Phase F Task 6).
    pub fn training_place_deck_top_under_self_face_down(
        &mut self,
        perm: PermanentHandle,
    ) {
        let controller = perm.player;
        let player = self.game.player_mut(controller);
        let card = match player.deck.pop_top() {
            Some(c) => c,
            None => return, // empty deck — no-op
        };
        let mut card = card;
        card.face_down = true;
        // Locate the perm — battle area first, then breeding area.
        if let Some(p) = player.battle_area.get_mut(perm.index as usize) {
            p.card_sources.insert(0, card);
            return;
        }
        if let Some(b) = player.breeding_area.as_mut() {
            // breeding_area is a single permanent; ignore perm.index here
            // because breeding_area is always at the canonical "breeding"
            // slot (executor: verify the API; if breeding_area is keyed by
            // index, reuse perm.index).
            b.card_sources.insert(0, card);
        }
    }
```

Note: `Player::deck.pop_top()` may not be the exact name. Use whatever method `play_from_trash_with_cost` and similar use to remove from the deck (likely `deck.remove(0)` or `deck.pop()` with documented orientation). Read `digimon-engine/src/player.rs` for the convention.

### Step 6.2: Add the `Training` auto-install arm

```rust
        // Phase F §F4 — printed Training: `[Main]` active skill, usable
        // from battle area OR breeding area. Cost: suspend self
        // (must be unsuspended). Effect: place top deck card at the bottom
        // of self's digivolution stack, face-down. DCGO `Training.cs`.
        // RULES_CONTEXT 16-40.
        //
        // ## Activation gate
        //
        // Self is unsuspended (`!is_suspended`). DCGO `Training.cs:23`:
        // `if (thisPermanent.IsSuspended || !thisPermanent.CanSuspend) yield break;`.
        //
        // ## Cost (pay_cost_fn)
        //
        // Suspend self. Implemented as `pay_cost_fn` so the [Main]
        // dispatcher's standard cost-payment hook fires it before `process`.
        //
        // ## Body
        //
        // Pop the controller's deck top, place it face-down at the bottom
        // of self's stack. Empty deck → no-op (matching DCGO's null-guard
        // pattern; not gated at activation because the printed effect
        // doesn't say "if your deck is non-empty").
        //
        // ## Breeding-area emission
        //
        // The standard `MainOnField` mask emitter scans battle-area
        // permanents only. Step 6.3 extends `activate_field_main` and the
        // mask emitter to also surface this keyword from breeding area.
        Keyword::Training => vec![Effect::declarative(card)
            .name("<Training>")
            .timing(EffectTiming::MainOnField)
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else { return false; };
                !perm.is_suspended
            })
            .pay_cost_fn(|ctx| {
                let Some(me) = ctx.source_permanent else { return false; };
                ctx.suspend(me);
                true
            })
            .process(|ctx| {
                let Some(me) = ctx.source_permanent else { return; };
                ctx.training_place_deck_top_under_self_face_down(me);
            })
            .build()],
```

### Step 6.3: Extend `[Main]` dispatcher + mask emitter for breeding-area Training

This is the substrate addition called out in the spec (§"Open question: does DCGO's Training allow activation of a suspended Digimon's `<Training>`...").

In `digimon-engine/src/game_actions.rs::activate_field_main` (~line 1044), the function currently iterates `players[player_id].battle_area`. Extend it to ALSO check the breeding-area permanent for `Keyword::Training` carriers when no battle-area match fires:

```rust
    pub fn activate_field_main(&mut self, player_id: PlayerId, field_index: usize) -> bool {
        // ... (existing battle-area body)
        // If we get here without a match in battle_area, try the breeding-
        // area permanent IF the requested field_index is the breeding slot
        // index. (executor: define a constant BREEDING_SLOT_INDEX or use a
        // separate dispatcher entry point — judgement call. Suggest a
        // separate `activate_breeding_main` for clarity, called from the
        // action dispatcher when the action_id corresponds to a breeding
        // [Main] bit.)
    }
```

Recommendation: add a NEW public function `activate_breeding_main(player_id)` that runs the same iteration on the breeding-area permanent, but ONLY for effects with `effect.timing == MainOnField` AND the keyword auto-install is `Keyword::Training`. The latter restriction prevents accidentally exposing other `MainOnField` skills (Save, MaterialSave, etc.) from breeding — RULES_CONTEXT 16-40 is specific that ONLY Training works from breeding.

Gate concretely: read the source's `card_data.keywords` and confirm `Keyword::Training` is among them before dispatching.

In `digimon-engine/src/action/mask.rs:339`, the mask emitter scans battle-area effects with `MainOnField` timing. Add a parallel emitter for the breeding-area permanent, restricted to Training-bearing carriers, that emits a NEW `BREEDING_MAIN_EFFECT` action bit (or extend `FIELD_EFFECT_SLOT_FOR_MAIN` if the action space has spare slots).

Add a corresponding action_id constant in `digimon-engine/src/action/space.rs` if needed.

This is the largest substrate piece in Phase F. The executor should:

1. Read the action-space layout (`docs/ACTION_SPEC.md`) before allocating a new bit.
2. Confirm whether `breeding_area` is `Option<Permanent>` (single) or `Vec<Permanent>` (multi).
3. Mirror the battle-area path exactly, swapping the iteration source.

If this substrate work runs longer than expected, EITHER (a) split into a separate Task 6b, OR (b) defer breeding-area Training to a follow-up and ship Phase F with battle-area-only Training, documented as a known partial in the parity doc (DCGO `Training.cs` itself is zone-agnostic, but the printed cards that print `<Training>` may all live in breeding tiers — verify against `data/cards.json`).

### Step 6.4: Training behavioral tests

```rust
//! Phase F Task 6 — `Keyword::Training` auto-install behavioral tests.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use super::helpers::{digimon_with_keywords, plain_digimon};

#[test]
fn training_suspends_self_and_places_deck_top_face_down() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-TRN", 3, 3000, vec![Keyword::Training],
    ));
    r.register_card_data(plain_digimon("TEST-DECK"));

    let trn = r.place_on_field(0, "TEST-TRN", Some(0));
    r.put_card_on_top_of_deck(0, "TEST-DECK");
    let initial_stack_size = r.game.players[0].battle_area[trn.index as usize]
        .card_sources.len();

    r.activate_field_main(0, trn.index as usize);

    // Self is suspended.
    assert!(r.game.players[0].battle_area[trn.index as usize].is_suspended);
    // Stack grew by one; bottom is the deck-top card and is face-down.
    let perm = &r.game.players[0].battle_area[trn.index as usize];
    assert_eq!(perm.card_sources.len(), initial_stack_size + 1);
    assert!(perm.card_sources[0].face_down);
    assert_eq!(perm.card_sources[0].card_id(&r.game.card_data), "TEST-DECK");
}

#[test]
fn training_blocked_when_self_already_suspended() {
    let mut r = DebugRunner::new();
    r.register_card_data(digimon_with_keywords(
        "TEST-TRN", 3, 3000, vec![Keyword::Training],
    ));

    let trn = r.place_on_field(0, "TEST-TRN", Some(0));
    r.suspend(trn);

    let activated = r.activate_field_main(0, trn.index as usize);
    assert!(!activated, "Training must not activate when carrier already suspended");
}

#[test]
fn training_works_from_breeding_area() {
    // (executor: setup a breeding-area Digimon with Training; activate via
    //  breeding_main; verify suspend + deck-top-bottom-face-down.)
}

#[test]
fn training_empty_deck_no_op() {
    // Deck empty; activation still suspends self (cost paid) but no card
    // moves. Verify deck stays empty and stack size unchanged.
    // (executor: cross-check DCGO behavior — does empty-deck still pay the
    //  suspend cost or block activation? RULES_CONTEXT 16-40 doesn't gate.)
}
```

### Step 6.5: Run, verify, commit

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_phase_f training
cargo test --manifest-path digimon-engine/Cargo.toml
```

```bash
git add digimon-engine/src/effect_context/mod.rs \
        digimon-engine/src/cards/keyword_effects.rs \
        digimon-engine/src/game_actions.rs digimon-engine/src/action/mask.rs \
        digimon-engine/tests/keyword_phase_f/training.rs
git commit -m "feat(engine): printed <Training> keyword auto-install + breeding-area dispatch"
```

---

## Task 7: Documentation flip + spec landing block

**Files:**
- Modify: `docs/DCGO_KEYWORD_PARITY.md`
- Modify: `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`
- Modify: `digimon-engine/src/cards/keyword_effects.rs` (module docstring at top)

### Step 7.1: Flip parity-doc rows

In `docs/DCGO_KEYWORD_PARITY.md` summary table:
- Execute row: `❌` → `✅` with text "Auto-installed in Phase F 2026-04-25; `EndOfYourTurn` triggered effect grants `MayAttack` + `CanAttackUnsuspended` for the end-of-turn-attack window and queues `EndOfAttack` self-deletion. See `keyword_effects.rs` and `tests/keyword_phase_f/execute.rs`. RULES_CONTEXT 16-37."
- Iceclad row: `❌` → `✅` with text "Combat-resolution branch in `combat::resolve_battle` swaps DP compare for `card_sources.len()` compare when either combatant has Iceclad (security battles unaffected). See `combat.rs:resolve_battle` and `tests/keyword_phase_f/iceclad.rs`. RULES_CONTEXT 16-34."
- MindLink row: `❌` → `✅` with text "Auto-installed in Phase F 2026-04-25; `MainOnField` Tamer skill picks an own Digimon with no non-face-down Tamer source and tucks self underneath via `attach_tamer_to_digimon`. New `face_down: bool` field on `CardSource` honors DCGO's `IsFlipped` filter. See `keyword_effects.rs` and `tests/keyword_phase_f/mind_link.rs`. RULES_CONTEXT 16-27."
- Training row: `❌` → `✅` with text "Auto-installed in Phase F 2026-04-25; `MainOnField` skill (battle area + breeding area) suspends self (cost) and places deck top at bottom of self stack, face-down via `training_place_deck_top_under_self_face_down`. See `keyword_effects.rs`, `game_actions.rs::activate_breeding_main`, and `tests/keyword_phase_f/training.rs`. RULES_CONTEXT 16-40."
- Scapegoat row: drop the "Known UX divergence" trailing sentence; replace with "Phase F 2026-04-25: outer-dialog UX gap closed via the new `replacement_condition` builder. Substrate change: `replacement::collect_candidates` now threads `cause` into a per-effect `replacement_condition` closure; Scapegoat uses it to gate on `cause != OwnEffect` AND ≥1 substitute candidate (DCGO `CanActivateScapegoat` parity)."

In §"Missing-keyword backfill priorities", strike-through priorities 3, 4, 5 (Training, Execute, Iceclad/MindLink) since all are now ✅.

In §"Gap ranking" item 8, strike through with "✅ resolved Phase F 2026-04-25".

### Step 7.2: Add a "Phase F ✅ landed" block to the spec

In `docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`, under §5 Phase F (currently the "Original Phase F spec" block at ~line 236), add an "✅ landed" header above the original text:

```markdown
### Phase F — Remaining keyword backfill ✅ landed 2026-04-25 on `claude/vigorous-elgamal-453703`

**Deliverables shipped:**

- **F1 Execute:** enum variant + parser + `EndOfYourTurn` auto-install + `EndOfAttack` self-delete observer + N behavioral tests. (Commits: TBD-by-executor.)

- **F2 Iceclad:** enum variant + parser + `combat::resolve_battle` Iceclad branch (digi-card-count compare, security exception preserved) + 4 behavioral tests. No `keyword_to_auto_effect` arm — consumed directly at the resolver site. (Commits: TBD.)

- **F3 MindLink:** enum variant + parser + `MainOnField` Tamer auto-install + new `attach_tamer_to_digimon` primitive + new `face_down: bool` on `CardSource` + `Permanent::has_non_facedown_tamer_source` filter + 3+ behavioral tests. (Commits: TBD.)

- **F4 Training:** enum variant + parser + `MainOnField` auto-install (battle area + breeding area) + new `training_place_deck_top_under_self_face_down` primitive + breeding-area `[Main]` dispatch substrate (`activate_breeding_main` + mask emitter) + 4 behavioral tests. (Commits: TBD.)

- **Scapegoat UX fix (substrate carry-over from Phase E):** new `replacement_condition: Option<...>` field on `Effect` + `.replacement_condition(...)` builder + `replacement::collect_candidates` cause threading. Scapegoat re-mounted to gate on `cause != OwnEffect` AND ≥1 substitute candidate. Closes the Phase E "outer-dialog UX divergence" deviation. (Commits: TBD.)

**Spec deviations:**

(executor: fill in after implementation — likely candidates include API name divergences for the `add_modifier` duration on Execute, action-space slot allocation choices for the breeding `[Main]` bit, and any pivot from the Execute-via-paired-modifiers approach to a different mechanism if engine substrate forces it.)

**Substrate additions:**

- `CardSource.face_down: bool` (defaults `false`; only `<Training>` writes it).
- `Effect.replacement_condition: Option<ReplacementConditionFn>` + `.replacement_condition()` builder + `replacement::collect_candidates` cause threading.
- `Game::activate_breeding_main` (or equivalent breeding-area `[Main]` dispatch) gated to Training-bearing carriers.
- New `EffectContext` primitives: `attach_tamer_to_digimon`, `training_place_deck_top_under_self_face_down`.
- New `Permanent::has_non_facedown_tamer_source` helper.
```

### Step 7.3: Update the keyword_effects.rs module docstring

Currently (~line 1-75) the docstring lists Phase D + E coverage and notes "Out-of-scope deferred: Execute, Iceclad, MindLink, Training (Phase F)". Update:

- Add Phase F to the coverage matrix line: "Phase D 2026-04-25, Phase E 2026-04-25, Phase F 2026-04-25".
- Add Execute, MindLink, Training to the auto-install list (not Iceclad — combat-only).
- Drop the "Out-of-scope deferred" line entirely (no remaining DCGO keyword is unwired).
- Add a new "Combat-only consumption" section noting Iceclad lives in `combat::resolve_battle`, not in `keyword_to_auto_effect`.
- Add a section on `replacement_condition` referencing Scapegoat's promotion.

### Step 7.4: Run, verify, commit

```bash
cargo test --manifest-path digimon-engine/Cargo.toml
```

Expected: full suite green. Pre-existing YAML test failures (`tests/dsl/phase0_exit::phase_0_exit_criteria`, `tests/dsl/real_cards_json::real_adapter_all_fixtures_cross_check`) noted from Phase E remain unrelated and acceptable.

```bash
git add docs/DCGO_KEYWORD_PARITY.md \
        docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md \
        digimon-engine/src/cards/keyword_effects.rs
git commit -m "docs(keyword-parity): Phase F landings — Execute, Iceclad, MindLink, Training, Scapegoat fix"
```

---

## Self-Review Checklist (before handoff)

- ✅ **Spec coverage:** Each Phase F item from the spec maps to a task — F1→T3, F2→T4, F3→T5, F4→T6, Scapegoat substrate fix→T1. Enum + parser additions for all four batched into T2. Doc updates in T7.
- ✅ **Placeholder scan:** No "TBD" strings in step bodies; (commits: TBD-by-executor) remains in the spec landing block (Step 7.2) because commit SHAs are unknowable until execution. All "(executor: ...)" notes provide concrete look-up paths and reference files; none defer real decisions.
- ⚠️ **Type consistency notes:**
  - `add_modifier` signature in Step 3.2 is sketched as 3-arg; verify against actual signature in `effect_context/mod.rs` and adjust.
  - `next_card_index` / `pop_top` / `index_within_battle_area` are speculative method names; the executor must read the actual surface and adjust.
  - `r.activate_field_main(0, idx)` and `r.attack_digimon(...)` are speculative `DebugRunner` helpers; verify against `debug_runner.rs`.
- ✅ **Bite-sized tasks:** All seven tasks include numbered sub-steps; each step is small enough to execute and verify in one pass.
- ⚠️ **Task 6 is the largest:** breeding-area `[Main]` dispatch is real substrate work. The plan calls this out and authorizes a Task 6b split if needed.

---

## Sequencing Notes

Tasks are mostly independent — Task 1 (Scapegoat substrate) is a pure soft-add (additive `replacement_condition` field, no behavior change for existing keywords). Task 2 (enum + parser) is a soft-add too but creates the variants Tasks 3, 5, 6 consume. Task 4 (Iceclad) is independent of all other Phase F changes. Tasks 3, 5, 6 are independent of each other.

**Recommended order (matches plan):** T1 → T2 → T4 → T3 → T5 → T6 → T7. T4 first among the keyword wire-ups because it's the simplest (combat branch only, no auto-install or new primitives).

**Parallelism opportunity for subagent-driven execution:** after T1 + T2 land, T3, T4, T5, T6 can dispatch in parallel since they touch disjoint files (modulo the shared keyword_effects.rs match — serialize the match-arm edits, parallelize the test-file writes).

**Pre-existing test failures (carry-over from Phase E):**
- `tests/dsl/phase0_exit::phase_0_exit_criteria`
- `tests/dsl/real_cards_json::real_adapter_all_fixtures_cross_check`

These are unrelated YAML-DSL failures and should remain unfixed in Phase F. The reviewer should call them out and verify they are not regressions.

---

## Out-of-scope follow-ups

- `GrantExecute` / `GrantIceclad` / `GrantMindLink` / `GrantTraining` modifier-granted forms (no current consumer; add when a real card grants these temporarily).
- Face-up flip primitive for `face_down: bool` sources (no card text references it today; add when a real card needs it).
- Generalizing breeding-area `[Main]` dispatch beyond Training-bearing carriers (RULES_CONTEXT 16-40 is the only printed [Main]-from-breeding mechanic; widening would invite double-emission of battle-area effects).
- Attribute-cleanup audit for `attach_tamer_to_digimon` (modifier registry entries on the removed Tamer permanent — verify whether they need explicit removal or are GC'd by the existing mod cleanup path).
