# Phase 2 Orchestration — Pilot Archetype Unblock

This document coordinates the eight Phase 2 tracks in `.claude/plans/phase-2-track-*.md`. Use it to choose a track to start, to know which tracks can run in parallel, and to land Phase 2 with a clean handoff to Phase 3 (continuous archetype YAML scale-out via `/batch-implement-cards-rust-dsl`).

**Parent spec:** [`docs/superpowers/specs/2026-05-14-substrate-reality-check-plan.md`](../../docs/superpowers/specs/2026-05-14-substrate-reality-check-plan.md). Phase 0 (audit) and Phase 1 (DSL eval-arm lint) are complete. Phase 2 = the eight tracks below. Phase 3 follows.

**Goal:** Move ~200 currently-stuck cards across 6 pilot archetypes (DNA Omnimon, Royal Knights, Puppets, Rocks, Medusamon, BG Imperial) from `PARTIAL`/`BLOCKED` to `IMPLEMENTED` in `qa/qa-reports/validated_cards_dsl.json`, by closing the substrate and DSL surface gaps they actually depend on.

**Out of goal:** Closing every entry in `docs/RUST_ENGINE_GAPS.md`. Cards no pilot decklist uses are deferred.

## Track inventory

| ID | File | Risk | Size | Test refs unblocked | Pilot impact |
|---|---|---|---|---:|---|
| **A** | [phase-2-track-a-dsl-eval-arm-sweep.md](phase-2-track-a-dsl-eval-arm-sweep.md) | low | ½–1 day | ~42 | DNA Omnimon, Medusamon, BG Imp |
| **B** | [phase-2-track-b-activation-cost-builder.md](phase-2-track-b-activation-cost-builder.md) | medium | 1–2 days | ~15 direct + ~10 card migrations | 5 of 6 archetypes |
| **C** | [phase-2-track-c-opt-slot-triggered-enforcement.md](phase-2-track-c-opt-slot-triggered-enforcement.md) | medium | 1–2 days | ~40 | Medusamon (huge), DNA Omnimon |
| **D** | [phase-2-track-d-inherited-stack-dispatch.md](phase-2-track-d-inherited-stack-dispatch.md) | medium-high | 1–2 days | ~25 standalone + ~10 with C | Medusamon, DNA Omnimon |
| **E** | [phase-2-track-e-rocks-pilot-completion.md](phase-2-track-e-rocks-pilot-completion.md) | low | 1–2 days | (authoring) | Rocks (~25 cards) |
| **F** | [phase-2-track-f-dna-omnimon-pilot-completion.md](phase-2-track-f-dna-omnimon-pilot-completion.md) — **closed 2026-05-17** (6 of 7 gaps; INHERITED-SUBSTITUTE deferred). 4 cards IMPLEMENTED (BT23-008 / BT23-018 / EX1-021 / BT16-040), 4 advanced PARTIAL (BT17-015 / BT17-027 / BT22-013 / BT22-026 / P-182). | medium | 2–3 days | ~30 | DNA Omnimon (~15 cards) |
| **G** | [phase-2-track-g-medusamon-pilot-completion.md](phase-2-track-g-medusamon-pilot-completion.md) | medium | 1–2 days | residual after C+D | Medusamon (~12 cards) |
| **H** | [phase-2-track-h-bg-imperial-pilot-completion.md](phase-2-track-h-bg-imperial-pilot-completion.md) | medium | 1–2 days | ~20 | BG Imperial (~12 cards) |
| **I** | [phase-2-track-i-puppets-pilot-completion.md](phase-2-track-i-puppets-pilot-completion.md) | medium-high | 2–3 days | ~25 + 2 UNCLEAR closures | Puppets (~15 cards) |
| **J** | [phase-2-track-j-royal-knights-pilot-completion.md](phase-2-track-j-royal-knights-pilot-completion.md) | high | 3–5 days (split into 3 PRs) | ~30 | Royal Knights (~25 cards) |

Total expected unblock: **~200 ignored tests, ~100 cards advanced to IMPLEMENTED.**

## Dependency graph

```
                       PHASE 2 DEPENDENCIES

                  ┌──────────────┐   ┌──────────────┐
                  │   Track A    │   │   Track B    │
                  │  DSL eval-arm│   │activation_   │
                  │  sweep       │   │cost builder  │
                  └───────┬──────┘   └───────┬──────┘
                          │  (soft)          │  (soft consumer)
                          ▼                  ▼
            ┌─────────────────────┐   ┌─────────────────────┐
            │ Track F (DNA Omnimon)│   │ Track I (Puppets)   │
            │ Track H (BG Imp)     │   │ Track J (RK)        │
            └─────────────────────┘   └─────────────────────┘

                  ┌──────────────┐
                  │   Track C    │
                  │  OPT slot    │
                  │  enforcement │
                  └───────┬──────┘
                          │  (hard — slot key shape)
                          ▼
                  ┌──────────────┐
                  │   Track D    │
                  │  inherited   │
                  │  dispatch    │
                  └───────┬──────┘
                          │  (hard for full payoff)
                          ▼
                  ┌──────────────┐
                  │   Track G    │
                  │  (Medusamon) │
                  └──────────────┘

      Track E (Rocks)  ──── fully standalone, no deps
      Track J (RK)     ──── largest; soft dep on B only
```

Read this as: arrows are *unlocks*, not blocks. A consumer track can START before its supplier lands — it just can't deliver full payoff until the supplier merges. Track G's plan explicitly handles each combination (C+D done, C only done, neither done) with a different residual scope.

## File conflict map

Tracks that touch the same file may conflict at merge time. None of these are real dependencies — they're rebase chores.

```
   File                                            Tracks touching
   ──────────────────────────────────────────────  ───────────────
   dsl_cards/predicate.rs                          A, F, G, H
   dsl_cards/formula_eval.rs                       A, F
   dsl_cards/step/*.rs                             E, F, G, H, J  (different files mostly)
   effect.rs                                       B (sole owner)
   effect_context/mod.rs                           B, F, G (different methods)
   effect_queue.rs                                 B (cost short-circuit), C (OPT slot), D (dispatch walk)
   dna_digivolve.rs                                F (alt-path direction)
   replacement.rs                                  J (consumer only — no edits)
   option_lifecycle.rs                             G, I (different fields)
   token_registry.rs                               J (Atho/Rene/Por only)
```

Highest-conflict file: `effect_queue.rs` (B + C + D). Sequence those three or rebase carefully.

Second-highest: `dsl_cards/predicate.rs` (A, F, G, H all add eval arms). Use distinct match-arm regions; conflicts will be in the surrounding `is_none()` guards. Mechanical to resolve.

## Suggested execution shapes

### Solo developer

```
   Week 1:  A    (½ day — confidence build, ~42 tests freed)
            B    (2 days — substrate foundation)
            E    (1 day — independent, pure DSL/authoring)

   Week 2:  C    (1–2 days)
            D    (1–2 days, after C)
            F    (2 days, after A — DNA Omnimon)

   Week 3:  G    (1–2 days, after C+D — Medusamon)
            H    (1–2 days — BG Imperial)
            I    (2 days, after B — Puppets)

   Week 4:  J    (3–5 days, split 3 PRs — Royal Knights)
            Tracker rollup PR.
```

### 2-person team

```
   Week 1:  Dev1: A → C
            Dev2: B → E

   Week 2:  Dev1: D → G
            Dev2: F → H

   Week 3:  Dev1: J (PR 1: substrate enablers)
            Dev2: I → J (PR 2: RK card authoring)

   Week 4:  Dev2: J (PR 3: RK card authoring wave 2)
            Both: Tracker rollup, Phase 3 handoff.
```

### Max parallel (4+ contributors)

```
   Day 0:   A, B, E, H all start (zero overlap)
   Day 2:   C starts; F starts (after A)
   Day 4:   D starts (after C); I starts (after B)
   Day 5:   G starts (after C+D); J starts
   Day 10:  All in PR review or merged.
```

Single contributor running serial: **~3–4 calendar weeks**. Max parallel: **~2 weeks** + review.

## Coordination protocol

For tracks with hard dependencies:

**Track C → Track D — slot key handoff.**
Track C's PR description must include a "diagnosis paragraph" (per Track C plan § Phase 1) pinning down: the slot-key shape used by `record_activation` / `activation_count`, where the reset happens, how it's keyed. Track D author reads this BEFORE writing the digivolution-stack walk.

**Track B → Tracks I, J — card migration deferral.**
Track B explicitly does NOT migrate the ~10 consumer cards (Tamer triggered abilities). Track B's PR description lists them. Tracks I and J inherit those migrations.

**Track A → Tracks F, H — tag-ref absorption.**
Track A's PR description must list every test it un-ignored. Tracks F and H authors re-grep their archetype's test files post-A-merge to confirm the residual is smaller than the plan projected, and may de-scope their plan accordingly.

**Track C+D → Track G — Medusamon timing.**
Track G's first step is a "sequencing pre-check" that scales scope by what's landed. If C and D are both in: full scope. If C only: defer inherited-dispatch tests. If neither: ship the small Medusamon-specific residual only.

For tracks with file-level conflicts:

- The author who merges first writes a brief "merge notes" addendum in their PR description: "I claimed lines X–Y in `predicate.rs`; please rebase around them."
- The author who merges second runs the variant-coverage lint (`cargo test --test dsl_eval_arm_coverage`) before pushing — that test will catch most accidental regressions of the first author's arms.

## Per-track success criteria (rolled up)

Each track's plan has its own acceptance gates. The Phase 2 rollup adds:

- **Ignored test count drops from 596 (current baseline) to ≤ 350** post all 10 tracks (the four-phase spec's 595→<200 target was scoped before C+D residue was visible; this number is realistic for "pilot-archetype unblock" framing, not for "all 12 BLOCKING items closed").
- **At least 100 cards advance from PARTIAL/BLOCKED to IMPLEMENTED** in `qa/qa-reports/validated_cards_dsl.json`.
- **Zero new substrate gap entries** filed in `docs/RUST_ENGINE_GAPS.md` during Phase 2 (the discovery riders in each plan tell authors to FILE a gap rather than absorb scope creep — this is fine and counts as expected work, not a failure mode).
- **`raw_rust` escapes drop from 110 (current) to ≤ 70** in production YAML.

## Phase 2 rollup PR (after all tracks merge)

A small "Phase 2 closure" PR cleans up:

1. Move all Phase-2-closed entries from `qa/dsl-vocab-gaps.md` / `qa/archetype-qa/engine-gaps.md` to a consolidated `qa/resolved-gaps.md` § "Phase 2 closure — 2026-XX-XX".
2. Update `docs/RUST_ENGINE_GAPS.md` "At a glance" table — open headings should drop from 42 to ~25.
3. Update `docs/superpowers/specs/2026-05-14-substrate-reality-check-plan.md` § "Phase 2" with `Status: ✅ Complete` and link the 10 closing PRs.
4. Update `qa/qa-reports/validated_cards_dsl.json` with rollup statistics.
5. Add a "Phase 3 readiness check" note in this orchestration doc.

## Phase 3 readiness gate

Phase 3 (continuous YAML scale-out via `/batch-implement-cards-rust-dsl`) starts when:

- All 10 Phase 2 tracks have merged OR explicit defer-to-Phase-3 notes.
- `/batch-implement-cards-rust-dsl` can run a full meta-deck archetype without any sub-agent reporting a substrate gap that's already filed open in this orchestration doc. (New gap discoveries are fine and route to a mini-Phase-2 cycle.)
- The variant-coverage lint passes and no track introduced a regression.
- `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py` still passes (PyO3 boundary stable).

## What this orchestration plan is NOT

- Not a substitute for reading each track's plan before starting it.
- Not a license to skip TDD discipline (CLAUDE.md §18) for any track.
- Not authority to expand `ACTION_SPACE_SIZE` or change PyO3 exports in any track.
- Not a commitment date — calendar estimates are guidance, not deadlines.

## Tracker shape after Phase 2

Expected end state (rough):

```
   docs/RUST_ENGINE_GAPS.md             ~25 open headings (down from 42)
   qa/archetype-qa/engine-gaps.md       ~12 open entries  (down from 27)
   qa/dsl-vocab-gaps.md                 ~30 open entries  (down from 80+)
   qa/resolved-gaps.md                  +Phase 2 closure section
   qa/qa-reports/validated_cards_dsl.json
                                        ~190 IMPLEMENTED (up from 93)
                                        ~80 PARTIAL      (down from 177)
                                        ~10 BLOCKED      (down from 23)
   raw_rust escapes                     ≤ 70 (down from 110)
   #[ignore] tests                      ≤ 350 (down from 596)
```

The remaining open items after Phase 2 are the **non-pilot** cards / archetypes (Dark Masters, Apocalymon family, Decoy color-filter, Training keyword, declarative overlays, etc.) — these are real engine gaps but no pilot deck uses them in the current meta. They're Phase 4 work — or they get a mini-Phase-2 cycle when a future deck-pool refresh brings them into a pilot.

## Reading this doc in 30 seconds

Need to start working? Read **the row in the inventory table** for your track, then read its plan file. Need to know if you can run in parallel? Read **the dependency graph**. Need to merge-rebase? Read **the file conflict map**. Need to declare "we're done"? Read **the rollup PR + Phase 3 readiness gate** sections.
