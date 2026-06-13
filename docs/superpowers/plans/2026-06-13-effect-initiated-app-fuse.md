# Effect-Initiated App Fuse Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an effect-initiated App Fuse primitive (DSL `app_fuse` step + one `EffectContext` entry point) so the "1 of your Digimon may app fuse into a Digimon card in the hand/trash" rider becomes a real, faithful clause on 5 Appmon cards, flipping them from PARTIAL to IMPLEMENTED.

**Architecture:** Effect-initiated App Fuse = play an App-Fusion-capable Digimon card *onto* one of your field Digimon that already has the named App-Fusion materials linked, via two engine-driven selections (permanent → result card) that route through the **existing** alt-play app-fusion commit. The only new engine plumbing is (a) generalizing the result-card source zone (hand|trash) and (b) the two-selection entry point. Eligibility reuses `app_fusion_host_eligible` / `app_fusion_condition_names`; the stack-and-consume-link commit is already shipped.

**Tech Stack:** Rust (`digimon-dsl` lowering crate + `digimon-engine` core), YAML DSL card specs, DebugRunner behavioral tests.

**Spec:** `docs/superpowers/specs/2026-06-13-effect-initiated-app-fuse-design.md`

---

## File Structure

- `code/digimon-dsl/src/step.rs` — `AppFuseArgs`, `AppFuseZone`, `StepSpec::AppFuse` variant + manual-deserialize arm.
- `code/digimon-dsl/src/compiled.rs` — `CompiledStep::AppFuse` + `CompiledAppFuseZone`.
- `code/digimon-dsl/src/compile.rs` — `StepSpec::AppFuse → CompiledStep::AppFuse` lowering.
- `code/digimon-engine/src/effect_context/action/app_fuse.rs` — NEW module: `initiate_effect_app_fuse` + the chained-selection resume handler. Registered in `effect_context/action/mod.rs`.
- `code/digimon-engine/src/game_actions/digivolve.rs` — generalize the app-fusion result-card pull to remove from the card's actual zone (hand|trash) instead of hand-only.
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — dispatch `CompiledStep::AppFuse → ctx.initiate_effect_app_fuse(...)`.
- `code/digimon-engine/tests/cards_behavioral/app_fuse_primitive.rs` — NEW primitive behavioral tests (register in `tests/cards_behavioral/main.rs`).
- 5 card YAMLs + their test files (BT21-084, BT23-079, P-241, BT25-089, BT24-087).
- `qa/qa-reports/validated_cards_dsl.json`, `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` — trackers.

**Reuse anchors (read before starting):**
- `code/digimon-engine/src/dna_digivolve.rs:404` `app_fusion_digivolve_route_for_card`, `:464` `app_fusion_host_eligible`, `:1011` `app_fusion_condition_names`.
- `code/digimon-engine/src/game_actions/digivolve.rs` — the `is_app_fusion` commit branch (`linked` cards drained under the new top via `push_under`).
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:628` `CompiledStep::EffectInitiatedDigivolve` dispatch (signature reference for the new dispatch arm).
- `code/digimon-engine/cards/bt25/BT25-089.yaml` + `tests/cards_behavioral/bt25/bt25_089.rs` (the card whose rider this unblocks; OPT shape).
- Existing chained two-selection step to model the entry point on: the `link_card_to_self` step with `to: chosen_own_digimon` (installs a first selection, then a second on resume) — `code/digimon-engine/src/dsl_cards/step/link_card.rs` + its selection resume wiring.

---

## Task 1: DSL `app_fuse` step type + serde

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Test: `code/digimon-dsl/tests/parse_app_fuse.rs` (create)

- [ ] **Step 1: Write the failing parse test**

Create `code/digimon-dsl/tests/parse_app_fuse.rs`:

```rust
//! Parse/serde coverage for the `app_fuse` step (effect-initiated App Fuse).
use digimon_dsl::step::{AppFuseZone, StepSpec};

fn parse_one(yaml: &str) -> StepSpec {
    serde_yml::from_str(yaml).expect("app_fuse step parses")
}

#[test]
fn app_fuse_defaults_to_hand_optional() {
    let s = parse_one("app_fuse: {}");
    match s {
        StepSpec::AppFuse(a) => {
            assert_eq!(a.from, AppFuseZone::Hand, "default zone is hand");
            assert!(a.optional, "default optional is true");
            assert!(a.result_filter.is_none(), "no filter by default");
        }
        _ => panic!("expected StepSpec::AppFuse"),
    }
}

#[test]
fn app_fuse_parses_trash_with_filter() {
    let yaml = r#"
app_fuse:
  from: trash
  result_filter:
    any_of:
      - trait_has: System
      - trait_has: Life
      - trait_has: Transmutation
"#;
    match parse_one(yaml) {
        StepSpec::AppFuse(a) => {
            assert_eq!(a.from, AppFuseZone::Trash);
            assert!(a.result_filter.is_some(), "filter present");
        }
        _ => panic!("expected StepSpec::AppFuse"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p digimon-dsl --test parse_app_fuse`
Expected: FAIL — `StepSpec::AppFuse` / `AppFuseZone` do not exist (compile error).

- [ ] **Step 3: Add the arg types**

In `code/digimon-dsl/src/step.rs`, near the other effect-digivolve arg structs (after `EffectDigivolveArgs`, ~line 1936), add:

```rust
/// Source zone for the result (fusing-in) card in an `app_fuse` step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppFuseZone {
    #[default]
    Hand,
    Trash,
}

/// `app_fuse:` — effect-initiated App Fuse. Plays an App-Fusion-capable Digimon
/// card from `from` ONTO one of your field Digimon that already has the named
/// App-Fusion materials linked. Two engine-driven selections (permanent, then
/// result card); no explicit target binding. See
/// `docs/superpowers/specs/2026-06-13-effect-initiated-app-fuse-design.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppFuseArgs {
    /// Zone holding the result card (`hand` default, or `trash`).
    #[serde(default)]
    pub from: AppFuseZone,
    /// Optional predicate on the result card (e.g. BT24-087's System/Life/
    /// Transmutation trait gate). `None` = any App-Fusion-capable Digimon card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_filter: Option<Predicate>,
    /// "may" — PASS is legal at each selection. Always true for the shipped
    /// riders; defaults true.
    #[serde(default = "crate::step::default_true")]
    pub optional: bool,
}
```

Confirm `Predicate` is the type used by other step filters in this file (e.g. `select_trash`'s `filter`); if it is imported under a different path/alias, match the existing usage. Confirm a `default_true` fn exists in the module (other args reuse it, e.g. the link-step optional default); if not, add `pub(crate) fn default_true() -> bool { true }`.

- [ ] **Step 4: Add the `StepSpec` variant + serialize + deserialize arms**

In the `StepSpec` enum, add: `AppFuse(AppFuseArgs),` (near `EffectInitiatedDigivolve`, ~line 195).

In the `Serialize` impl's match (near line 449, the `kv!` arms), add:
```rust
StepSpec::AppFuse(v) => kv!(s, "app_fuse", v),
```

In the manual `Deserialize` map-key match (near line 706), add:
```rust
"app_fuse" => StepSpec::AppFuse(map.next_value()?),
```

In the known-keys list used for the "unknown step" error (near line 908, where `"effect_initiated_digivolve"` etc. are listed), add `"app_fuse",`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p digimon-dsl --test parse_app_fuse`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add code/digimon-dsl/src/step.rs code/digimon-dsl/tests/parse_app_fuse.rs
git commit -m "dsl: add app_fuse step type + serde (effect-initiated App Fuse)"
```

---

## Task 2: Compiled `app_fuse` step + lowering

**Files:**
- Modify: `code/digimon-dsl/src/compiled.rs`, `code/digimon-dsl/src/compile.rs`
- Test: `code/digimon-dsl/tests/parse_app_fuse.rs` (extend)

- [ ] **Step 1: Write the failing compile test**

Append to `code/digimon-dsl/tests/parse_app_fuse.rs`:

```rust
#[test]
fn app_fuse_lowers_to_compiled_step() {
    use digimon_dsl::compiled::{CompiledAppFuseZone, CompiledStep};
    // Compile a minimal card carrying an app_fuse process step and assert the
    // compiled clause contains CompiledStep::AppFuse with the trash zone.
    let yaml = r#"
card: DSL-APPFUSE-001
name: Test App Fuse
kind: tamer
color: [green]
cost: 3
effects:
  - when: end_of_your_turn
    optional: true
    process:
      - app_fuse: { from: trash }
"#;
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(yaml).expect("card parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("card compiles");
    let found = compiled.effects.iter().any(|clause| {
        clause.process_steps().iter().any(|s| {
            matches!(s, CompiledStep::AppFuse { from_zone: CompiledAppFuseZone::Trash, .. })
        })
    });
    assert!(found, "compiled card contains CompiledStep::AppFuse(Trash)");
}
```

Adjust `process_steps()` / `compile()` / `CardSpec` access to match the crate's actual public API (check how an existing `tests/parse_*.rs` reaches compiled steps; mirror it exactly).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p digimon-dsl --test parse_app_fuse app_fuse_lowers`
Expected: FAIL — `CompiledStep::AppFuse` / `CompiledAppFuseZone` undefined.

- [ ] **Step 3: Add the compiled types**

In `code/digimon-dsl/src/compiled.rs`, near `CompiledStep::EffectInitiatedDigivolve`, add the zone enum and variant:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledAppFuseZone {
    Hand,
    Trash,
}
```

Add to `CompiledStep`:
```rust
/// Effect-initiated App Fuse: two engine-driven selections (own permanent,
/// then result card from `from_zone`) routed through the app-fusion commit.
AppFuse {
    from_zone: CompiledAppFuseZone,
    result_filter: Option<CompiledPredicate>,
    optional: bool,
},
```

Match `CompiledPredicate` to the type used by other compiled steps' filters in this file.

- [ ] **Step 4: Add the lowering arm**

In `code/digimon-dsl/src/compile.rs`, in `compile_step` (the big `match` on `StepSpec`), add:

```rust
StepSpec::AppFuse(a) => CompiledStep::AppFuse {
    from_zone: match a.from {
        crate::step::AppFuseZone::Hand => CompiledAppFuseZone::Hand,
        crate::step::AppFuseZone::Trash => CompiledAppFuseZone::Trash,
    },
    result_filter: a.result_filter.as_ref().map(|p| compile_predicate(p)),
    optional: a.optional,
},
```

Use the same predicate-compile helper that sibling steps use (find how `select_trash`'s `filter` is compiled and call the identical function). Add any needed `use` for `CompiledAppFuseZone`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p digimon-dsl --test parse_app_fuse`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-dsl/tests/parse_app_fuse.rs
git commit -m "dsl: lower app_fuse step to CompiledStep::AppFuse"
```

---

## Task 3: Engine — generalize the app-fusion result-card pull to hand|trash

The existing app-fusion commit (the `is_app_fusion` branch in `game_actions/digivolve.rs`) pulls the result card from **hand**. App Fuse must also pull from **trash** (BT24-087). This task adds a zone-parameterized commit helper without changing alt-play behavior.

**Files:**
- Modify: `code/digimon-engine/src/game_actions/digivolve.rs`
- Test: `code/digimon-engine/tests/app_fuse_commit.rs` (create; register in the engine's integration-test set if needed — most engine `tests/*.rs` are auto-discovered, confirm by mirroring a sibling like `tests/security_effects.rs`)

- [ ] **Step 1: Write the failing test**

Create `code/digimon-engine/tests/app_fuse_commit.rs`. Build a DebugRunner with: an own field permanent whose top + one linked card are the two named App-Fusion materials of a result Digimon card placed in **trash**; call the new commit helper directly; assert the result card is the new top, the matched link is now a digivolution source, and the result card left the trash.

```rust
//! Engine-level: app-fusion commit can pull the result card from trash (not just hand).
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn app_fusion_commit_pulls_result_from_trash_and_stacks() {
    // Use a real shipped App-Fusion result + its named materials. Rebootmon
    // (BT25-060) App-Fuses from [Bootmon] & [Shutmon]. Build a host permanent
    // whose top is Bootmon with Shutmon linked, put Rebootmon in trash, and
    // commit an app-fusion of Rebootmon onto that host from trash.
    //
    // Assert post-state:
    //   - host top card == Rebootmon (BT25-060)
    //   - the consumed link (Shutmon) is now a digivolution source under the top
    //   - Rebootmon is no longer in trash
    //
    // Drive via the new `Game::commit_effect_app_fuse(player, host, result_card, AppFuseSourceZone::Trash)`
    // (or whatever the chosen helper name is) — this test PINS that signature.
    todo!("author with the real card ids once the host-stack builder helper is confirmed; \
           use runner.place_stack / push_linked_owned + seed_trash as in tests/cards_behavioral/bt25/bt25_060.rs");
}
```

NOTE for the implementer: replace the `todo!` with a concrete test using the same fixtures pattern as `tests/cards_behavioral/bt25/bt25_060.rs` (which already stacks Bootmon/Shutmon for Rebootmon's alt-play app-fusion). The test MUST assert the three post-state facts above. Do not land this task with a `todo!` remaining.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test app_fuse_commit`
Expected: FAIL (helper does not exist / `todo!` panics).

- [ ] **Step 3: Extract a zone-parameterized commit**

In `game_actions/digivolve.rs`, factor the `is_app_fusion` commit body so the result-card removal takes a zone. Add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFuseSourceZone { Hand, Trash }

impl Game {
    /// Commit an effect-initiated App Fuse: stack `result_card` (pulled from
    /// `zone`) on top of `host`, then fold `host`'s existing linked cards under
    /// the new top as digivolution sources (consumed), exactly like the alt-play
    /// app-fusion commit. App Fusion is printed Cost 0; no requirement check.
    /// Returns true on success.
    pub fn commit_effect_app_fuse(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        result_card: CardHandle,
        zone: AppFuseSourceZone,
    ) -> bool {
        // 1. Locate + remove `result_card` from `zone` (hand or trash).
        // 2. host.digivolve(removed, turn)  — stack on top.
        // 3. Drain host.linked_cards under the new top via push_under (reverse),
        //    identical to the existing is_app_fusion branch.
        // 4. Emit the Digivolve GameEvent (was_dna: false), mirroring the
        //    existing commit's event emission.
        // Reuse/extract the existing is_app_fusion block rather than duplicating.
        todo!("extract from the existing is_app_fusion commit; only the result-card \
               removal differs by zone")
    }
}
```

Then re-point the existing alt-play app-fusion commit at this helper with `AppFuseSourceZone::Hand` so there is a single implementation (DRY). Keep the alt-play tests green (`tests/cards_behavioral/bt25/app_fusion.rs`).

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test app_fuse_commit --test cards_behavioral -- app_fusion`
Expected: the new commit test PASSES; the existing `bt25::app_fusion::*` tests STILL PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/game_actions/digivolve.rs code/digimon-engine/tests/app_fuse_commit.rs
git commit -m "engine: zone-parameterized app-fusion commit (hand|trash), DRY with alt-play"
```

---

## Task 4: Engine — `initiate_effect_app_fuse` two-selection entry point

This is the core new behavior: install selection #1 (eligible own permanents), then on pick install selection #2 (eligible result cards for that permanent), then commit via Task 3's helper. Model the chained-selection install + resume on the existing `link_card_to_self`/`chosen_own_digimon` two-step flow.

**Files:**
- Create: `code/digimon-engine/src/effect_context/action/app_fuse.rs`
- Modify: `code/digimon-engine/src/effect_context/action/mod.rs` (add `mod app_fuse;`), and the engine's pending-selection resume dispatch (wherever chained step-selections resume — same place `link_card_to_self`'s second selection resumes).
- Test: `code/digimon-engine/tests/cards_behavioral/app_fuse_primitive.rs` (create) — see Task 6 (written first as the executable spec).

- [ ] **Step 1: Confirm the spec tests in Task 6 exist and fail**

Author Task 6's `app_fuse_primitive.rs` FIRST (it is the executable spec for this entry point). Run:
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- app_fuse_primitive`
Expected: FAIL — `initiate_effect_app_fuse` undefined.

- [ ] **Step 2: Implement the entry point**

In the new module `effect_context/action/app_fuse.rs`:

```rust
use crate::effect_context::EffectContext;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;
use digimon_dsl::compiled::{CompiledAppFuseZone, CompiledPredicate};

impl EffectContext<'_> {
    /// Effect-initiated App Fuse. Installs selection #1 over the controller's
    /// field permanents that some eligible result card can app-fuse onto; on
    /// pick, installs selection #2 over eligible result cards in `from_zone`;
    /// on pick, commits via `Game::commit_effect_app_fuse`. If `optional`, PASS
    /// is legal at each step. If no permanent is eligible, this is a silent
    /// no-op (matches DCGO's HasMatchConditionOwnersPermanent guard).
    pub fn initiate_effect_app_fuse(
        &mut self,
        from_zone: CompiledAppFuseZone,
        result_filter: Option<&CompiledPredicate>,
        optional: bool,
    ) {
        // Eligibility for a (permanent P, result card C) pair:
        //   - C is a Digimon card in `from_zone`, passes `result_filter`,
        //   - C has an `app_fusion` alt-path that is app_fusion_host_eligible(path, P)
        //     (Game::app_fusion_host_eligible + app_fusion_condition_names),
        //   - C.owner == P.player == self.controller.
        // 1. permanents = own field permanents with >=1 eligible result card.
        // 2. if empty -> return (no-op).
        // 3. install OwnField-style selection over `permanents` (PASS legal iff optional);
        //    stash (from_zone, result_filter, optional) in the pending-selection payload.
        // 4. resume: on permanent pick P, compute eligible result cards for P and
        //    install the hand/trash selection (PASS legal iff optional).
        // 5. resume: on card pick C, call
        //    self.game.commit_effect_app_fuse(player, P, C, zone_map(from_zone)).
        todo!("install chained selections following the link_card_to_self \
               (to: chosen_own_digimon) two-step pattern; reuse app_fusion_host_eligible \
               for eligibility and Game::commit_effect_app_fuse for resolution")
    }
}
```

Implementation guidance (follow exactly):
- **Eligibility helper:** add a private `Game::app_fuse_eligible_result_for(perm, card_handle, zone, filter)` that checks the card is a Digimon, passes the filter, and has an `app_fusion` alt-path with `app_fusion_host_eligible(path, perm_ref)` true. Reuse `app_fusion_condition_names` / `app_fusion_host_eligible` (already in `dna_digivolve.rs`; make them `pub(crate)` if not already).
- **Selection plumbing:** mirror the two-step install/resume used by the `link_card_to_self` step when `to: chosen_own_digimon` (it installs a card selection then, on resolve, a host selection). Use the SAME pending-selection variant family and the SAME resume-dispatch hook. Do NOT invent a new SelectionKind if an existing own-field + hand/trash kind already serves.
- **No auto-pick:** every eligible entry must be a distinct action ID; PASS present iff `optional`. (CLAUDE.md §17.)

- [ ] **Step 3: Wire the resume dispatch**

Register `mod app_fuse;` in `effect_context/action/mod.rs`. Add the App-Fuse selection's resume handling to the same dispatch site where `link_card_to_self`'s chained selection resumes (search for where the chosen-own-digimon host selection is resumed and add the parallel arm).

- [ ] **Step 4: Run the primitive tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- app_fuse_primitive`
Expected: PASS (all Task 6 tests).

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/effect_context/action/app_fuse.rs code/digimon-engine/src/effect_context/action/mod.rs code/digimon-engine/src/dna_digivolve.rs
git add code/digimon-engine/tests/cards_behavioral/app_fuse_primitive.rs code/digimon-engine/tests/cards_behavioral/main.rs
git commit -m "engine: initiate_effect_app_fuse two-selection entry point"
```

---

## Task 5: Dispatch `CompiledStep::AppFuse` in the step lowerer

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`

- [ ] **Step 1: Add the dispatch arm**

In `play_digivolve.rs`, in the same `match` that handles `CompiledStep::EffectInitiatedDigivolve` (~line 628), add:

```rust
CompiledStep::AppFuse {
    from_zone,
    result_filter,
    optional,
} => {
    ctx.initiate_effect_app_fuse(*from_zone, result_filter.as_ref(), *optional);
    true
}
```

If this `match` is not the one that receives `CompiledStep::AppFuse` (some steps are dispatched in a different module's match), add the arm wherever the catch-all for unhandled steps currently lives so the new variant compiles exhaustively. Confirm exhaustiveness: `cargo build -p digimon-engine` will error on a missing arm and point you to the right match.

- [ ] **Step 2: Verify build + primitive tests**

Run: `cargo build --manifest-path code/digimon-engine/Cargo.toml`
Expected: compiles (no non-exhaustive-match error).
Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- app_fuse_primitive`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/step/play_digivolve.rs
git commit -m "engine: dispatch CompiledStep::AppFuse to initiate_effect_app_fuse"
```

---

## Task 6: Primitive behavioral tests (authored in Task 4 Step 1)

**Files:**
- Create: `code/digimon-engine/tests/cards_behavioral/app_fuse_primitive.rs`
- Modify: `code/digimon-engine/tests/cards_behavioral/main.rs` (add `mod app_fuse_primitive;`)

This file is the executable spec for the entry point. Use an inline `from_dsl_yaml` fixture Tamer (`DSL-APPFUSE-HOST`) carrying a single `app_fuse` process step under a trigger you can fire directly, plus a real App-Fusion result card (Rebootmon BT25-060, materials Bootmon/Shutmon) and a non-App-Fusion control card.

- [ ] **Step 1: Write all primitive tests**

```rust
//! Effect-initiated App Fuse primitive — behavioral coverage.
//! Driven through a minimal inline `app_fuse` fixture; result card is the real
//! BT25-060 Rebootmon (App Fusion [Bootmon] & [Shutmon]).
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::SelectionKind;
use digimon_engine::action::space::PASS;

// Helper: build a runner where P0 has a field permanent eligible to be
// app-fused onto (top = Bootmon, Shutmon linked) and the fixture host carrying
// the app_fuse step. Fire the step and return the runner parked at selection #1.
// (Author the builder using runner.place_stack / push_linked_owned + seed_trash;
//  mirror tests/cards_behavioral/bt25/bt25_060.rs fixtures.)

#[test]
fn app_fuse_hand_happy_path_stacks_result_and_consumes_link() {
    // perm: top Bootmon + linked Shutmon; Rebootmon in P0 hand.
    // fire app_fuse(hand) -> selection #1 (the perm) -> selection #2 (Rebootmon)
    // assert: perm top == BT25-060; Shutmon now a digivolution source; Rebootmon left hand.
}

#[test]
fn app_fuse_trash_happy_path() {
    // Rebootmon in TRASH; app_fuse(trash). Same post-state; Rebootmon left trash.
}

#[test]
fn app_fuse_result_filter_excludes_nonmatching_trash_card() {
    // Two app-fusable result cards in trash, one passing a {trait_has: X} filter
    // and one not; assert selection #2 offers ONLY the passing one.
}

#[test]
fn app_fuse_ineligible_permanent_not_offered() {
    // A field permanent lacking the named materials is NOT in selection #1.
}

#[test]
fn app_fuse_no_eligible_permanent_is_silent_noop() {
    // No permanent has materials -> no selection installs; state unchanged.
}

#[test]
fn app_fuse_decline_at_permanent_pick_does_nothing() {
    // optional: PASS at selection #1 -> no fusion.
    // assert runner.pending_is_optional() was true at install.
}

#[test]
fn app_fuse_decline_at_card_pick_does_nothing() {
    // pick perm, then PASS at selection #2 -> no fusion; perm unchanged.
}

#[test]
fn app_fuse_requires_two_distinct_named_materials() {
    // perm whose top + only link match the SAME single name -> ineligible.
}
```

Fill in each body concretely (no empty bodies at land time). Use `digimon_engine::action::space::*` constants for any action ids; assert via `runner.pending_kind()`, the permanent's `top_card()` id, and digivolution-source / zone membership.

- [ ] **Step 2: Register + run**

Add `mod app_fuse_primitive;` to `tests/cards_behavioral/main.rs`. Run:
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- app_fuse_primitive`
Expected: after Tasks 4–5, all PASS.

- [ ] **Step 3: Commit** (if not already committed with Task 4)

```bash
git add code/digimon-engine/tests/cards_behavioral/app_fuse_primitive.rs code/digimon-engine/tests/cards_behavioral/main.rs
git commit -m "test: effect-initiated App Fuse primitive behavioral coverage"
```

---

## Task 7: BT25-089 Kazuki & Itsuki — standalone OPT hand fuse

The simplest card to convert (no preceding rider effects). Validates the OPT shape.

**Files:**
- Modify: `code/digimon-engine/cards/bt25/BT25-089.yaml`, `code/digimon-engine/tests/cards_behavioral/bt25/bt25_089.rs`

- [ ] **Step 1: Add the failing test**

In `bt25_089.rs`, add a test driving the `[End of Your Turn][OPT]` clause: set up an own permanent with Bootmon-top + Shutmon-linked and Rebootmon in hand, advance to end of P0's turn, resolve the app_fuse selections, assert Rebootmon stacked onto the permanent. Add an OPT-lockout test (second EoT fuse same turn → no selection; clears next turn).

- [ ] **Step 2: Run — expect FAIL** (YAML rider still omitted)

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt25_089`

- [ ] **Step 3: Replace the omitted rider in the YAML**

In `BT25-089.yaml`, replace the documented-omission comment for the `[End of Your Turn][OPT]` app-fuse clause with a real clause:

```yaml
  - when: end_of_your_turn
    once_per_turn: true
    optional: true
    summary: "[End of Your Turn][OPT] 1 of your Digimon may app fuse into a Digimon card in the hand"
    process:
      - app_fuse: { from: hand }
```

Remove the now-stale "BLOCKED/omitted" comment block referencing the App Fuse gap.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt25_089`
Expected: PASS (including OPT lockout).

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/cards/bt25/BT25-089.yaml code/digimon-engine/tests/cards_behavioral/bt25/bt25_089.rs
git commit -m "card(BT25-089): real app_fuse clause (was PARTIAL/omitted)"
```

---

## Task 8: BT24-087 Rei Katsura — trash fuse + trait filter

**Files:**
- Modify: `code/digimon-engine/cards/bt24/BT24-087.yaml`, `code/digimon-engine/tests/cards_behavioral/bt24/bt24_087.rs`

- [ ] **Step 1: Add the failing test**

Drive the full `on_any_link` clause: link a Digimon → suspend Rei → Draw 1 + trash 1 → then the app_fuse-from-trash with a result card carrying System/Life/Transmutation. Assert the filtered fuse works and a non-matching trash card is not offered.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_087`

- [ ] **Step 3: Append the app_fuse step to the existing clause**

In `BT24-087.yaml`, the `on_any_link` clause currently ends after the draw + trash steps (with the rider omitted). Append:

```yaml
      - app_fuse:
          from: trash
          result_filter:
            any_of:
              - trait_has: System
              - trait_has: Life
              - trait_has: Transmutation
```

Remove the stale omission comment.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_087`

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/cards/bt24/BT24-087.yaml code/digimon-engine/tests/cards_behavioral/bt24/bt24_087.rs
git commit -m "card(BT24-087): real app_fuse-from-trash clause with trait filter (was PARTIAL)"
```

---

## Task 9: BT23-079 Eri Karan — hand fuse after DP rider

**Files:**
- Modify: `code/digimon-engine/cards/bt23/BT23-079.yaml`, `code/digimon-engine/tests/cards_behavioral/bt23/bt23_079.rs`

- [ ] **Step 1: Add the failing test**

Drive the `on_any_link` clause through the +3000 DP rider AND the appended app_fuse; assert both effects occur in order (linked host got +3000, then the chosen permanent app-fused a hand card).

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_079`

- [ ] **Step 3: Append the app_fuse step**

In `BT23-079.yaml`, append to the existing `on_any_link` process (after the +3000 DP step):

```yaml
      - app_fuse: { from: hand }
```

Remove the stale omission comment.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_079`

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/cards/bt23/BT23-079.yaml code/digimon-engine/tests/cards_behavioral/bt23/bt23_079.rs
git commit -m "card(BT23-079): real app_fuse clause after DP rider (was PARTIAL)"
```

---

## Task 10: BT21-084 Haru Shinkai — hand fuse after Draw rider

**Files:**
- Modify: `code/digimon-engine/cards/bt21/BT21-084.yaml`, `code/digimon-engine/tests/cards_behavioral/bt21/bt21_084.rs`

- [ ] **Step 1: Add the failing test**

Drive the `on_any_link` suspend-cost clause: Draw 1 → then the appended app_fuse from hand. Assert both.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_084`

- [ ] **Step 3: Append the app_fuse step**

In `BT21-084.yaml`, append to the existing `on_any_link` process (after the `draw` step):

```yaml
      - app_fuse: { from: hand }
```

Remove the stale omission comment.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_084`

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/cards/bt21/BT21-084.yaml code/digimon-engine/tests/cards_behavioral/bt21/bt21_084.rs
git commit -m "card(BT21-084): real app_fuse clause after Draw rider (was PARTIAL)"
```

---

## Task 11: P-241 Yujin Ozora — hand fuse after Vortex/DP rider

**Files:**
- Modify: `code/digimon-engine/cards/p/P-241.yaml`, `code/digimon-engine/tests/cards_behavioral/p/p_241.rs`

- [ ] **Step 1: Add the failing test**

Drive the `on_any_link` clause through the Vortex + +3000 DP grant on a chosen Appmon Digimon, THEN the appended app_fuse from hand. Assert all three (Vortex keyword present, +3000 DP, app-fuse stacked).

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_241`

- [ ] **Step 3: Append the app_fuse step**

In `P-241.yaml`, append to the existing `on_any_link` process (after the Vortex/DP grant steps):

```yaml
      - app_fuse: { from: hand }
```

Remove the stale omission comment.

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_241`

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/cards/p/P-241.yaml code/digimon-engine/tests/cards_behavioral/p/p_241.rs
git commit -m "card(P-241): real app_fuse clause after Vortex/DP rider (was PARTIAL)"
```

---

## Task 12: Trackers, schema, full-suite green

**Files:**
- Modify: `qa/qa-reports/validated_cards_dsl.json`, `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`
- Regen: DSL JSON schema

- [ ] **Step 1: Flip the 5 verdicts to IMPLEMENTED**

In `qa/qa-reports/validated_cards_dsl.json`, for BT21-084, BT23-079, P-241, BT25-089, BT24-087: set `status` to `IMPLEMENTED`, `gap_kind` to `null`, bump `test_count`, and update `notes` to record the app_fuse clause now ships (remove the "OMITTED app-fuse rider" wording). Bump `last_updated`.

- [ ] **Step 2: Close the gap-tracker entries**

In `docs/RUST_ENGINE_GAPS.md`, mark the effect-initiated App Fuse gap RESOLVED (2026-06-13), noting the `app_fuse` DSL step + `initiate_effect_app_fuse` entry point + zone-parameterized commit, and that all 5 riders now ship. In `qa/dsl-vocab-gaps.md`, record the `app_fuse` step as landed.

- [ ] **Step 3: Regenerate the DSL schema**

Run: `cargo run -p dsl-schema-export`
Then verify lint: `cargo run -p dsl-lint -- code/digimon-engine/cards/bt25/BT25-089.yaml code/digimon-engine/cards/bt24/BT24-087.yaml code/digimon-engine/cards/bt23/BT23-079.yaml code/digimon-engine/cards/bt21/BT21-084.yaml code/digimon-engine/cards/p/P-241.yaml`
Expected: the 5 edited cards lint clean.

- [ ] **Step 4: Full engine suite green**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml`
Expected: all pass (no regressions; alt-play app-fusion still green).
Run: `cargo test -p digimon-dsl`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add qa/qa-reports/validated_cards_dsl.json docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md code/digimon-dsl/
git commit -m "App Fuse: close engine gap, flip 5 riders PARTIAL->IMPLEMENTED, regen schema"
```

---

## Notes for the implementer

- **Cross-worktree build cache:** if `cargo test` reports `DSL card … not found in embedded pack` or stale `OnAnyLink`/step-enum parse errors, force a rebuild: `touch code/digimon-engine/build.rs code/digimon-dsl/src/step.rs` then re-run. New card YAMLs require the build script to re-pack.
- **No-approximations (CLAUDE.md §17):** both App-Fuse selections must surface every legal choice as a distinct action id with PASS iff optional — never auto-pick the permanent or the result card.
- **DCGO quirk (BT24-087):** the result card is removed from its actual zone (trash), even though DCGO's `PlayCardClass` nominally passes `Root.Hand`. Task 3's `AppFuseSourceZone` enforces this.
- **Source priority:** printed card text is authoritative; DCGO C# (`BT23_079.cs`, `BT24_087.cs`, `BT25_089.cs`, `CardSource.CanAppFusionFromTargetPermanent`) is the behavioral tiebreaker; see the spec.
