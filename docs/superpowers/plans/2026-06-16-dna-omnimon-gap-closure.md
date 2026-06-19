# DNA Omnimon Gap Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the two engine/DSL gaps blocking DNA Omnimon / Omnimon-family card authoring — `G-ACTIVATED-DIGIVOLVE-EXECUTION` (via a zero-engine-code DSL re-model) and `G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH` (a small engine fix) — and verify the two upstream-closed gaps are exercised.

**Architecture:** Phase A re-models three `kind: activated_digivolve` cards onto existing machinery (the BT24-016 Lamiamon `main_from_hand` precedent + a static `digivolve` add-source). Phase B teaches `place_self_as_delay_option_permanent` to claim its card from the in-flight `pending_option` slot, after which `dispose_option` naturally no-ops. Phase C runs the now-unblocked verification tests.

**Tech Stack:** Rust (`code/digimon-engine/`), YAML DSL card specs (`code/digimon-engine/cards/`), DebugRunner behavioral/interaction tests. DSL-first + TDD (CLAUDE.md rules 17/18/28). **No `action/space.rs` change → no DCGO `ActionSpace.cs` regen (rule 27).**

**Source of truth for this work:** the design spec `docs/superpowers/specs/2026-06-16-dna-omnimon-gap-closure-design.md`.

---

## Conventions used by every task

**Build isolation (memory `reference_cargo_target_per_worktree`):** within this pre-restart session the shared target dir is still inherited, so prefix every cargo command with the per-worktree target dir. All `Run:` commands below assume:

```bash
export CARGO_TARGET_DIR='D:\cargo-target-wt\infallible-goldwasser-83f65c'
cd "$(git rev-parse --show-toplevel)"   # MUST end in .claude/worktrees/infallible-goldwasser-83f65c
```

Verify the toplevel is the worktree (not the base repo) before any write — a base-repo write is lost (memory: worktree sub-agent dispatch).

**Single-test invocation:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test <suite> <test_name> -- --exact` (add `--ignored` to run a still-`#[ignore]`d test, or `--include-ignored` to run both).

**DCGO crosscheck (read-only):** `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`; C# at `$BASE_DCGO/Assets/Scripts/CardEffect/BT22/Red/BT22_013.cs` etc. Never `cd` there; never `git submodule update --init` in the worktree (rule 29).

---

## Phase A — Retire `G-ACTIVATED-DIGIVOLVE-EXECUTION` via DSL re-model (zero engine code)

### Task A1: Re-model BT22-013 WarGreymon `[Hand][Main]` self-digivolve

**Files:**
- Modify: `code/digimon-engine/cards/bt22/BT22-013.yaml` (the `alt_paths` block, ~lines 208–228; effects header)
- Test: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_013.rs`

- [ ] **Step 1: Write/adjust the failing per-card test**

In `bt22_013.rs`, add a test that drives the `[Hand][Main]` jump through the real masked Hand-Main action (model it on the BT24-016 Lamiamon per-card test `tests/cards_behavioral/bt24/bt24_016.rs`, which exercises the identical `main_from_hand` → `effect_initiated_digivolve { from_hand: self }` shape). Cast: BT22-084 Nokia Shiramine (Tamer) + an [Agumon] on field + BT22-013 in hand.

```rust
#[test]
fn bt22_013_hand_main_jump_digivolves_agumon_at_cost6_with_nokia() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-013").expect("BT22-013 in pack")
        .dsl_card("BT22-084").expect("BT22-084 Nokia Shiramine in pack")
        .dsl_card("ST20-10").expect("ST20-10 Agumon in pack") // a real [Agumon]
        .hand(0, &["BT22-013"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let agumon = runner.place_on_field(0, "ST20-10", None);
    runner.place_on_field(0, "BT22-084", None);           // Nokia Tamer on field
    runner.game.enter_main_phase();

    // Drive BT22-013's masked Hand-[Main] action (see bt24_016.rs for the exact
    // activate_hand_main / HAND_EFFECT_START driver in this harness), then pick
    // the on-field Agumon as the digivolve base.
    drive_hand_main_then_pick(&mut runner, 0, "BT22-013", agumon);

    // The Agumon stack is now topped by BT22-013 at digivolution cost 6.
    assert_eq!(
        runner.game.players[0].battle_area[agumon.index as usize]
            .top_card().card_id(&runner.game.card_data),
        "BT22-013",
        "Agumon must have digivolved into BT22-013 via the [Hand][Main] jump",
    );
}
```

- [ ] **Step 2: Run it — expect FAIL**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt22_013_hand_main_jump -- --exact`
Expected: FAIL — the current `kind: activated_digivolve` alt-path is never offered (no masked Hand-Main action), so the digivolve never happens.

- [ ] **Step 3: Re-model the YAML** — replace the `activated_digivolve` alt-path with a `main_from_hand` clause

In `BT22-013.yaml`, DELETE the second alt-path entry (`- kind: activated_digivolve … cost: 6 … ignore_requirements: true`, ~lines 222–228), keeping the standard `kind: digivolve { from: { level_eq: 5 }, cost: 4 }`. Then ADD this clause to `effects:` (mirrors BT24-016 Lamiamon clause 1):

```yaml
  # ─── [Hand][Main] Nokia jump — Agumon digivolves into this card, cost 6 ──────
  # "[Hand][Main] If you have [Nokia Shiramine], 1 of your [Agumon] digivolves
  #  into this card for a digivolution cost of 6, ignoring digivolution
  #  requirements."
  # Re-modelled from the unreachable kind: activated_digivolve alt-path onto a
  # main_from_hand clause (BT24-016 Lamiamon precedent) — retires
  # G-ACTIVATED-DIGIVOLVE-EXECUTION with zero engine code. The Nokia precondition
  # is now ENFORCED via condition: (it could not be expressed on an alt-path).
  - when: main_from_hand
    summary: "[Hand][Main] If you have Nokia Shiramine: 1 of your Agumon digivolves into this card for cost 6, ignoring reqs"
    condition:
      all_of:
        - any_permanent:
            of: you
            zone: [battle_area]
            name_is: "Nokia Shiramine"
        - any_permanent:
            of: you
            zone: [battle_area]
            kind: digimon
            name_contains: "Agumon"
    process:
      - select_own_permanent:
          bind_as: base
          filter:
            all_of:
              - kind: digimon
              - name_contains: "Agumon"
          prompt: "Choose an Agumon to digivolve into WarGreymon (cost 6, ignore reqs)"
      - effect_initiated_digivolve:
          target: base
          from_hand: self
          cost: 6
          ignore_requirements: true
```

Also update the stale `G-ALT-PATH-CONDITION` / `G-ACTIVATED-DIGIVOLVE-EXECUTION` header comment block (~lines 44–142) to a one-line note: the jump is now a `main_from_hand` clause; Nokia gate enforced.

- [ ] **Step 4: Lint the YAML**

Run: `cargo run -p dsl-lint -- code/digimon-engine/cards/bt22/BT22-013.yaml`
Expected: no errors.

- [ ] **Step 5: Run the test — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt22_013_hand_main_jump -- --exact`
Expected: PASS. Also run the full `bt22_013` per-card suite to confirm no regression: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt22_013 --`

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/cards/bt22/BT22-013.yaml code/digimon-engine/tests/cards_behavioral/bt22/bt22_013.rs
git commit -m "BT22-013: re-model [Hand][Main] Nokia jump onto main_from_hand (retire activated_digivolve)"
```

### Task A2: Re-model BT22-026 MetalGarurumon (Blue mirror of A1)

**Files:**
- Modify: `code/digimon-engine/cards/bt22/BT22-026.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_026.rs`

- [ ] **Step 1: Add the failing test** — identical to A1 Step 1 but Blue/Gabumon side: cast BT22-084 Nokia + a real [Gabumon] (e.g. `BT22-017`) on field + BT22-026 in hand; assert the Gabumon stack is topped by `BT22-026` after the jump. Name it `bt22_026_hand_main_jump_digivolves_gabumon_at_cost6_with_nokia`.

- [ ] **Step 2: Run — expect FAIL** (`--test cards_behavioral bt22_026_hand_main_jump -- --exact`).

- [ ] **Step 3: Re-model the YAML** — delete the `kind: activated_digivolve` alt-path; add the `main_from_hand` clause exactly as A1 Step 3 but with `name_contains: "Gabumon"` in both the `condition` and the `select_own_permanent` filter, and `summary` naming MetalGarurumon. Keep `from: { level_eq: 5 }, cost: 4`. Update the stale header note.

- [ ] **Step 4: Lint** — `cargo run -p dsl-lint -- code/digimon-engine/cards/bt22/BT22-026.yaml`.

- [ ] **Step 5: Run — expect PASS** (the new test + the full `bt22_026` suite).

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/cards/bt22/BT22-026.yaml code/digimon-engine/tests/cards_behavioral/bt22/bt22_026.rs
git commit -m "BT22-026: re-model [Hand][Main] Nokia jump onto main_from_hand (retire activated_digivolve)"
```

### Task A3: Re-model BT16-027 Imperialdramon: Fighter Mode (static add-source)

**Files:**
- Modify: `code/digimon-engine/cards/bt16/BT16-027.yaml` (alt_paths block, lines 91–105)
- Test: `code/digimon-engine/tests/cards_behavioral/bt16/bt16_027.rs`

- [ ] **Step 1: Add the failing test** — assert a permanent topped by "Imperialdramon: Dragon Mode" can digivolve into BT16-027 at cost 2 (the printed add-source path). Drive a standard digivolve from that base.

```rust
#[test]
fn bt16_027_digivolves_from_dragon_mode_at_cost2() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-027").expect("BT16-027 in pack")
        .dsl_card("BT16-026").expect("Imperialdramon: Dragon Mode in pack") // confirm exact ID
        .hand(0, &["BT16-027"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "BT16-026", None);
    runner.game.enter_main_phase();
    // Drive the digivolve from the Dragon Mode base into BT16-027 (cost 2).
    drive_digivolve_from_hand(&mut runner, 0, "BT16-027", base);
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card().card_id(&runner.game.card_data),
        "BT16-027",
        "Dragon Mode must digivolve into BT16-027 via the cost-2 add-source path",
    );
}
```
(Confirm the exact Dragon Mode card ID via `python code/tools/resolve_cards.py "Imperialdramon: Dragon Mode"`.)

- [ ] **Step 2: Run — expect FAIL** (`kind: activated_digivolve` is unreachable, so no cost-2 path is offered).

- [ ] **Step 3: Change the YAML** — in `BT16-027.yaml` change the second alt-path:

```yaml
  - kind: digivolve
    from:
      of: you
      zone: [battle_area]
      name_contains: "Imperialdramon: Dragon Mode"
    cost: 2
```
(was `kind: activated_digivolve`). If `dsl-lint` or the test shows `from:` name-predicates are unsupported on a `digivolve` alt-path, fall back to the A1 `main_from_hand` shape with the digivolve target selected by name. Update the header `Clause layout` comment.

- [ ] **Step 4: Lint** — `cargo run -p dsl-lint -- code/digimon-engine/cards/bt16/BT16-027.yaml`.

- [ ] **Step 5: Run — expect PASS** (new test + full `bt16_027` suite).

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/cards/bt16/BT16-027.yaml code/digimon-engine/tests/cards_behavioral/bt16/bt16_027.rs
git commit -m "BT16-027: model Dragon Mode add-source as a digivolve alt-path (retire activated_digivolve)"
```

### Task A4: Un-ignore DNA Omnimon combo E + close the gap

**Files:**
- Modify: `code/digimon-engine/tests/archetypes/dna_omnimon.rs` (combo E, lines 711–724)
- Modify: `qa/archetype-qa/engine-gaps.md` (the `G-ACTIVATED-DIGIVOLVE-EXECUTION` entry, ~line 586)
- Modify: `qa/qa-reports/validated_cards_dsl.json` (BT22-013/026, BT16-027 verdicts)

- [ ] **Step 1: Replace the `unimplemented!()` combo E body with a real interaction test**

Remove the `#[ignore]` attribute (lines 712–715) and author the body modeled on `combo_d_*` (same file) + the A1 per-card test. Cast: BT22-084 Nokia + BT22-013 in hand + a field [Agumon] (`ST20-10`). Drive the `[Hand][Main]` jump; assert the Agumon stack is topped by BT22-013 at cost 6 and `[When Digivolving]` fired. Add the **Nokia-absent negative path** in the same or a sibling test (with no Nokia on field, the masked Hand-Main action is absent → no digivolve).

- [ ] **Step 2: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes combo_e_nokia_cost6_lv6_jump -- --exact`
Expected: PASS (no longer ignored).

- [ ] **Step 3: Close the tracker entry**

In `engine-gaps.md`, mark the `G-ACTIVATED-DIGIVOLVE-EXECUTION` entry RESOLVED for BT22-013/026 + BT16-027 (re-modelled, no engine code) and move the body to `qa/resolved-gaps.md`. Update those three cards' verdicts in `validated_cards_dsl.json` to `AUDITED-OK`/`IMPLEMENTED` (never invent a new status — memory `project_starter_decks_battle_tested`).

- [ ] **Step 4: Run the full archetypes + cards_behavioral suites**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes` and `--test cards_behavioral`
Expected: green (no regressions).

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/tests/archetypes/dna_omnimon.rs qa/archetype-qa/engine-gaps.md qa/resolved-gaps.md qa/qa-reports/validated_cards_dsl.json
git commit -m "DNA Omnimon combo E: drive the real BT22-013 Nokia jump; close G-ACTIVATED-DIGIVOLVE-EXECUTION"
```

---

## Phase B — Fix `G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH` (engine)

### Task B1: Un-ignore the two Omnimon ACE combo 1 tests (RED)

**Files:**
- Modify: `code/digimon-engine/tests/archetypes/omnimon_ace.rs` (lines 202–206 and 308–311)

- [ ] **Step 1: Remove the `#[ignore]` attributes** from `combo1_mega_knight_free_plays_agumon_from_trash_and_seats_as_delay` (lines 203–206) and `combo1_mega_knight_declining_recursion_still_seats_delay` (lines 309–311). The test bodies already exist and assert the faithful outcome (BT17-095 seated as `OptionState::Delayed`, not trashed).

- [ ] **Step 2: Run — expect FAIL (the repro)**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes combo1_mega_knight -- --include-ignored`
Expected: FAIL — `delay_option_present(…, "BT17-095")` is false and BT17-095 is in trash, because the place-self step no-ops on the real `play_option_from_hand` path.

- [ ] **Step 3: Commit the failing tests (red baseline)**

```bash
git add code/digimon-engine/tests/archetypes/omnimon_ace.rs
git commit -m "Omnimon ACE combo 1: un-ignore the real-play-path Delay-seat tests (red; G-OPTION-PLACE-SELF-AS-DELAY repro)"
```

### Task B2: Make `place_self_as_delay_option_permanent` claim from `pending_option` (GREEN)

**Files:**
- Modify: `code/digimon-engine/src/effect_context/action/lifecycle.rs:119` (`place_self_as_delay_option_permanent`)

- [ ] **Step 1: Insert the `pending_option`-aware claim**

In the `else` (non-`source_permanent`) branch, between the `pending_security` arm and the `remove_source_option_from_controller_zones()` fallback (current lines 133–149), add a `pending_option` arm:

```rust
        } else {
            if let Some(pending) = self.game.pending_security.take() {
                if pending.played
                    || pending.card.handle() != self.source_card
                    || pending.card.card_kind(&self.game.card_data) != CardKind::Option
                {
                    self.game.pending_security = Some(pending);
                    return;
                }
                pending.card
            } else if let Some(pending) = self.game.pending_option.take() {
                // Real Option-play lifecycle: play_option_core moved the Option
                // into the single-occupancy `pending_option` slot BEFORE running
                // the [Main] body, so it is no longer in hand/trash. Claim it for
                // self-placement when it is OUR source Option. Taking it here
                // leaves `pending_option` empty, so the subsequent
                // `dispose_option` (play_option_core step 8) finds nothing and
                // skips the Standard trash — exactly the desired end state.
                // Fixes G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH.
                if pending.card.handle() != self.source_card
                    || pending.card.card_kind(&self.game.card_data) != CardKind::Option
                {
                    self.game.pending_option = Some(pending);
                    return;
                }
                pending.card
            } else {
                let Some(source_card) = self.remove_source_option_from_controller_zones() else {
                    return;
                };
                source_card
            }
        };
```

The seating logic below (lines 151–186: build `Permanent` with `OptionState::Delayed`, push to battle_area, enqueue `OnOptionPlaced`, drain) is unchanged and now runs with the claimed card.

- [ ] **Step 2: Run the combo 1 tests — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes combo1_mega_knight -- --exact`
Expected: PASS — BT17-095 is now seated as `OptionState::Delayed` (trash unchanged), because place-self consumed `pending_option` and `dispose_option` (mod.rs:1107, `let Some(pending) = self.pending_option.take() else { return; }`) early-returns.

- [ ] **Step 3: Guard against regression in the lifecycle / disposal**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt17_095 --` and the option-lifecycle unit tests: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes` plus `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib option`
Expected: green. (Watch the `dispose_option` Standard/Delay/Link/Training arms and any `pending_option_placed_turn_check` interplay — if a Standard Option that does NOT self-place regressed, the early-claim guard `handle() != self.source_card` is the place to check.)

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/src/effect_context/action/lifecycle.rs
git commit -m "Engine: place_self_as_delay_option claims from pending_option on the real play path (fix G-OPTION-PLACE-SELF-AS-DELAY)"
```

### Task B3: Un-scaffold DNA Omnimon combo B + close the gap

**Files:**
- Modify: `code/digimon-engine/tests/archetypes/dna_omnimon.rs` (combo B — replace the `seat_as_delay_option` scaffold with the real `play_option_from_hand`)
- Modify: `docs/RUST_ENGINE_GAPS.md` (`G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH` entry, line 384) → `qa/resolved-gaps.md`

- [ ] **Step 1: Replace the scaffold** in DNA Omnimon combo B so BT17-095 is played through the real `runner.game.play_option_from_hand(...)` path (as Omnimon ACE combo 1 now does) instead of the `seat_as_delay_option` helper + `delete_permanent_with_cause`. Keep the reactive Clause-B DNA-merge assertions.

- [ ] **Step 2: Run — expect PASS**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes combo_b -- --exact`
Expected: PASS through the real play path.

- [ ] **Step 3: Close the tracker** — mark `G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH` RESOLVED in `docs/RUST_ENGINE_GAPS.md`, move the entry to `qa/resolved-gaps.md`, update BT17-095's verdict in `validated_cards_dsl.json`.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/tests/archetypes/dna_omnimon.rs docs/RUST_ENGINE_GAPS.md qa/resolved-gaps.md qa/qa-reports/validated_cards_dsl.json
git commit -m "DNA Omnimon combo B + Omnimon ACE combo 1: real play path; close G-OPTION-PLACE-SELF-AS-DELAY"
```

---

## Phase C — Verify the upstream-closed gaps for the DNA Omnimon pool

### Task C1: Verify BT22-013/026 branch-0 + inherited name-gate; scrub stale headers

**Files:**
- Modify (comments only): `code/digimon-engine/cards/bt22/BT22-013.yaml`, `code/digimon-engine/cards/bt22/BT22-026.yaml`
- Possibly modify: their `cards_behavioral` tests if any branch-0 test is still `#[ignore]`d

- [ ] **Step 1: Find any still-ignored branch-0 / name-gate tests**

Run: `grep -rn "ignore" code/digimon-engine/tests/cards_behavioral/bt22/bt22_013.rs code/digimon-engine/tests/cards_behavioral/bt22/bt22_026.rs`
Expected: enumerate any `#[ignore]` citing `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` or `G-DSL-SOURCE-NAME-CONTAINS` (both RESOLVED upstream).

- [ ] **Step 2: Un-ignore and run them**

Remove any such `#[ignore]` and run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt22_013 --` and `bt22_026 --` (with `--include-ignored` first to see them).
Expected: PASS (the gaps are closed). If a test FAILS, it is a real regression → STOP, treat as a finding, root-cause via `superpowers:systematic-debugging`, do NOT weaken the assertion.

- [ ] **Step 3: Scrub the stale header comments** in `BT22-013.yaml` / `BT22-026.yaml` — delete the "Branch 0 here will exhibit the same observable… the digivolve never happens" and `G-DSL-SOURCE-NAME-CONTAINS` "degenerates to true" notes, since those gaps are resolved.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/cards/bt22/BT22-013.yaml code/digimon-engine/cards/bt22/BT22-026.yaml code/digimon-engine/tests/cards_behavioral/bt22/
git commit -m "BT22-013/026: verify branch-0 + Omnimon name-gate (upstream-resolved); scrub stale headers"
```

---

## Final verification (after all phases)

- [ ] **Full engine test sweep**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` and `--test archetypes` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib`
Expected: all green. No new `#[ignore]`s introduced; the two target gaps no longer appear in the OPEN trackers.

- [ ] **Confirm no action-space drift** — `git diff --stat origin/main -- code/digimon-engine/src/action/` is EMPTY (rule 27: no DCGO codegen needed).

---

## Self-review notes (author)

- **Spec coverage:** Phase A (A1–A4) ⇒ spec Phase A; Phase B (B1–B3) ⇒ spec Phase B; Phase C (C1) ⇒ spec Phase C. All covered.
- **Open assumptions flagged inline:** (1) BT16-027 `from:` name-predicate support on a `digivolve` alt-path (A3 Step 3 fallback given); (2) exact Hand-Main test driver helper name (`activate_hand_main` / `HAND_EFFECT_START`) — resolve from `bt24_016.rs`; (3) exact "Imperialdramon: Dragon Mode" card ID — resolve via `resolve_cards.py`. None block the approach.
- **Type/name consistency:** `place_self_as_delay_option_permanent`, `dispose_option`, `pending_option`, `OptionState::Delayed`, `OptionSubtype::Standard` all match the engine source read at plan time.
