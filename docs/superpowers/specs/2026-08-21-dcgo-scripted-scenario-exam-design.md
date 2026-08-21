# DCGO Scripted Scenarios — the Card-Clause Exam

Makes DCGO answer targeted questions while a card is being authored, instead of
only reporting what its AI happened to do.

Builds on:
- `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md` (phase 1: volume)
- `docs/superpowers/specs/2026-08-20-dcgo-agent-puppet-design.md` (Layers A/B/C)

Operating manual: `docs/DCGO_HARNESS.md`. Recording format: `docs/DCGO_RECORDING_SCHEMA.md`.

**This spec is Layers B and C**, scoped concretely and with two corrections to the
2026-08-20 design (see "Corrections" below). Layer A — build, manifest,
action-space hash gate, daemon, `watch` — is already implemented and unchanged.

## Problem

`JobSpec.policy` only ever emits `"ai"`. The corpus answers "what did DCGO do",
never "what does DCGO do *here*" — which is the question card authoring asks.

Concretely: while implementing `EX12-035` you want to know whether its
`[On Play]` clause resolves the way we implemented it. Today the only way is to
generate volume and hope the bot triggered it.

## What already exists

| Piece | State |
|---|---|
| `dcgo-harness` job queue, build + manifest, action-space hash gate, daemon, `watch` | Built |
| DCGO recorder (~27 hooks: `action`, `action_detail`, `selection`, `effect_activation`, `reveal`) | Built |
| `dcgo-replay` — DCGO recording → our engine, divergence report | Built |
| `clause_coverage` — (card, clause) denominator + corpus coverage | Built |
| `DeckStacker` / `InputDriver` / `StateDumper` | **Not built** |
| Scenario format, sim-side runner, state differ, exam layer | **Not built** |

## Corrections to the 2026-08-20 design

Both were verified against the DCGO checkout while writing this spec.

1. **`InputDriver` does not attach at `UserSelectionManager.SetIntForPlayer` /
   `SetBoolForPlayer`.** That is the same error rule 27 documents on the reading
   side: those are the *fallback channel only*, emitting `generic_int` /
   `generic_bool`. The real seam is the **13 live `HarnessAuto.DrivesLocalSeat`
   gate sites across 10 files**, each of which currently routes the local seat to
   `AutoSelect()`:

   | File | Sites |
   |---|---|
   | `CardController.cs` | 700 |
   | `MultipleSkills.cs` | 193, 274 |
   | `OptionalSkill.cs` | 63 |
   | `SelectAttackEffect.cs` | 234 |
   | `SelectCardEffect.cs` | 383 |
   | `SelectCountEffect.cs` | 131 |
   | `SelectDigiXrosClass.cs` | 484, 569 |
   | `SelectHandEffect.cs` | 189 |
   | `SelectPermanentEffect.cs` | 298 |
   | `UserSelectionManager.cs` | 128, 187 |

   Plus `TurnStateMachine.QueueMainPhaseAction` for main-phase decisions.

   **Count these by reading, not by `grep | wc -l`** — the same discipline rule 27
   demands of the recorder hook map. A raw grep returns 15 non-definition hits;
   two of them (`CardController.cs:376`, `ICardEffect.cs:1233`) are *comments
   referencing* the property, not gates. An earlier draft of this spec said "14
   sites" and listed `ICardEffect` among them, on exactly that error.

   Note also that two sites (`CardController.cs:700`, `SelectDigiXrosClass.cs:569`)
   have inverted polarity — they route *to* the AI path rather than gating away
   from it. The driver must handle both shapes.

2. **"Initial shuffle only" is enforceable as one named exclusion, not a latch.**
   `RandomUtility.ShuffledDeckCards` is the single helper behind every shuffle,
   and mid-game re-shuffle is exactly one call site
   (`CardObjectController.cs:1040`, `player.LibraryCards = ...`). `DeckStacker`
   applies in the helper; that one site bypasses it explicitly. A stateful
   first-call latch would be fragile — main deck and egg deck are two separate
   initial calls per player.

## Scope

All three pieces, in one spec, decomposed into separate implementation plans:

1. **Unity** — `DeckStacker`, `InputDriver`, `StateDumper`, `policy: "scripted"`.
2. **Rust** — scenario schema, lowering, sim-side runner, normalized projection, differ.
3. **Workflow** — clause binding, verdict store, assertion backfill, test drafter,
   CI workflow, MCP subcommand, skill.

The second workflow requested — *run a scenario in our sim, replay it in DCGO,
confirm no divergence* — is the same machinery under a different entry point
(`exam --suite`), not additional work.

## Forced constraints

Two things are not choices, and both shape everything downstream.

**DCGO reaches a position only by legally playing to it.** The 2026-08-20 design
rejects a "materialize board" cheat on oracle-trust grounds: a hand-built board
can miss internal wiring the normal play path sets up, so a divergence might be
the cheat's fault rather than a real parity bug. This spec keeps that rejection.

The consequence is significant and must be stated plainly: **existing
`cards_behavioral` tests are not exam-able.** They use `DebugRunner` staging.
Exam scenarios are a new artifact — a legal line from game start over a stacked
deck.

**Both seats are fully scripted.** If DCGO's AI plays the opponent, our engine
has no way to reproduce its choices, so every game diverges for reasons that are
not findings.

## The artifact

`qa/dcgo-exams/<SET>/<CARD-ID>.yaml`:

```yaml
card: EX12-035
clause: on_play                # keys into the clause_coverage denominator
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-035, BT16-082], rest: <deck-name> }
  p1: { stack: [...],                        rest: <deck-name> }
steps:
  - actor: 0
    do:     { hatch: {} }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { play: {card: EX12-035, from: hand} }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { select: {targets: [opp.field.0]} }
    expect: { prompt: select_permanent, count: 1 }
assert:
  - at: 3
    that: { opp.field.0.dp: 3000, p0.memory: -2 }
```

### `stack` is a prefix, and applies to the initial shuffle only

You name the first N cards in draw order; the remainder is seeded-shuffled from a
named deck. Requiring all 50 would make every file unauthorable.

Initial-shuffle-only is a correctness requirement, not an optimization. Search
and shuffle effects route through the same helper. If the stack applied to every
shuffle, a card reading "shuffle your deck" would silently re-impose the opening
order and the exam would confidently answer a question about a game that cannot
occur. Mid-game shuffles fall through to seeded `GameRandom` — still fully
deterministic, but honest.

### `expect` is asserted before the step is answered

A driver that answers whatever it is asked will, on one ordering mismatch,
desynchronize the entire remainder of the line while every step still looks
successful.

So each step carries the prompt it expects, and the driver asserts before
answering. A mismatch aborts the job and reports itself.

**That is a finding, not an error.** "Our engine expected a choice here and DCGO
never asked" is exactly the divergence class that never surfaces as an illegal
action.

### `do` is symbolic, lowered to an action ID

The two engines already share the 2192-slot action space: the recorder encodes to
it, `dcgo-replay` decodes from it, and `ActionSpace.cs` is codegen'd from
`space.rs` behind a CI drift gate (rule 27).

Raw integers would be unwritable by hand, unreviewable in a diff, and would rot
silently on renumbering. So steps are symbolic and a lowering pass resolves each
against our engine's live action mask, failing loudly on illegal or ambiguous
intent.

Lowering runs **once, up front, in our engine**, and the resulting action IDs are
written into the job file — so DCGO and our engine consume literally the same
integers, and a malformed scenario fails in milliseconds rather than after sixty
seconds of Unity.

## Execution

```
scenario.yaml
     |
     +- lower ---> [our engine] HeadlessRunner from a real game start,
     |               same stacked deck, same seed
     |                  \--> trace: [step] -> normalized state
     |
     +- job file -> [DCGO] policy: "scripted"
                      DeckStacker -> InputDriver -> StateDumper
                         \--> recording.jsonl + state.jsonl
                                    |
                               differ ---> first divergence
```

Our runner is **not a special code path**. `digimon_engine::runners::replay`
already defines a `RecordingSource` trait with two implementations —
`NativeAdapter` (engine-generated recordings) and `DcgoAdapter` (the DCGO oracle,
with its own step policy and "attempted vs completed" handling). The sim-side
scenario runner is a **third adapter, `ScenarioAdapter`**, feeding lowered action
IDs into the same `ReplaySession`.

That is the single most load-bearing reuse decision in this spec: the divergence
machinery, step policy, and player-perspective conversion are all inherited
rather than reimplemented, and a scenario run is structurally the same object as
a corpus replay.

If our engine can only reach the position via `DebugRunner` staging, the scenario
is not exam-able and says so — rather than quietly comparing a staged board
against a played one.

### Normalized projection

Emitted by both sides at every decision boundary, keyed by step index so the two
align.

| Field | Normalization |
|---|---|
| battle area | multiset of `{card_id, effective_dp, sources[], suspended, keywords[]}` |
| hand / trash | multiset of card IDs |
| security | count only (contents are hidden information) |
| memory | integer, both from the recording player's perspective |
| phase / turn | as-is |

The governing rule: **normalize representation, never semantics.** Effective DP
is representation — the two engines track modifiers differently and a
modifier-list diff would be pure noise. Whether a Digimon is suspended is
semantics, and must diff. Under-normalize and the report drowns in noise nobody
reads; over-normalize and it hides the bug.

### Differ

Walks aligned step indices and **leads with the first divergence, marking
everything after it downstream.** Once the two engines part they are playing
different games; a report ranking fifty consequences beside one cause is a report
nobody finishes.

## The exam layer

`clause_coverage` already produces the denominator: it splits a card's printed
text into clauses by timing marker (`[On Play]`, `[When Digivolving]`,
`[Security]`, ...) and by angle-bracket keyword, resolving text from the official
Bandai bundle first, `cards.json` + overrides second, and emitting an explicit
`image-required` slot rather than concluding "no clause" from a lossy source.

The exam keys into that same clause identity, so "all its clauses" is checkable
rather than a feeling.

The scenario's `clause:` field is **not free text** — it must resolve to a clause
key that `clause_coverage.extract` actually emits for that card, and the exam
rejects a scenario naming a clause the extractor does not produce. Otherwise a
typo silently creates a 6th, invisible verdict class: a scenario that passes
while covering nothing in the denominator. Pinning the exact key format is the
first task of the exam-layer plan, since it is `extract`'s output shape and not a
choice this spec gets to make.

### Verdict store

`qa/qa-reports/dcgo_exam_verdicts.json`, one row per `(card, clause)`:

| Verdict | Meaning |
|---|---|
| `confirmed` | A scenario exercised this clause and both engines agreed for the whole line |
| `diverged` | Both engines ran it and disagreed |
| `unreachable` | Scenario exists but the line could not legally reach the clause |
| `unavailable` | DCGO's pool does not contain this card, so no oracle exists |
| `unmeasured` | No scenario authored yet |

`unmeasured` is the point of the table. `/dcgo-exam EX12-035` reports
**8 clauses: 5 confirmed, 1 diverged, 2 unmeasured** — the denominator is always
printed, matching the rule phase 1 earned the hard way: a batch where most jobs
died must never read as a pass.

### Three honesty constraints

- **`unavailable` is a real outcome and is determined per card, not per set.**
  DCGO's `Assets/Scripts/CardEffect/` spans AD1, BT1–BT26, EX1–EX12, ST1–ST24, LM,
  P and RB1 — broad coverage, so "newer than DCGO" is *not* the right test. The
  gap is per-card: a set directory can exist while an individual card has no
  script. `unavailable` therefore means "DCGO has no implementation for this
  card", resolved by looking for `<SET>/<COLOR>/<CARD_ID>.cs` (underscored
  filename, per the C# naming convention). No oracle exists for such a card, and
  that must read as "not verified", never as "passed".
- **`diverged` does not mean we are wrong.** Source priority puts
  `general_rule.pdf` above DCGO. A divergence is ranked and diagnosed; the fix
  stays a decision, not an automation.
- **25 cards route through `SetIsBackgroundProcess(true)`** and bypass the
  `effect_activation` hook entirely (rule 27, and `docs/DCGO_RECORDING_SCHEMA.md`).
  Their clauses are structurally unmeasurable by activation matching — they get
  `unreachable` carrying that specific reason, not a silent pass.

### Assertion backfill

On a `confirmed` verdict the exam writes the confirmed DCGO state into the
scenario's `assert:` block. You author the line; the oracle records what
happened; that becomes the permanent guard that survives into CI after the oracle
has gone home.

This is what makes `assert:` load-bearing rather than decorative.

### Test drafter

Emits a draft `#[test]` into `code/digimon-engine/tests/cards_behavioral/<set>/`
with the header `DCGO build <hash>, job <id>, scenario <path> observed:` — never
asserting correctness, never auto-committing.

DCGO is source-priority #2, below `general_rule.pdf`. A drafted test encodes
strong evidence, not truth. An auto-generated test asserting a behavior nobody
read would launder a DCGO quirk into a permanent guard, which under the
no-approximations policy is worse than no test.

## CI split

DCGO **cannot** gate PRs on GitHub-hosted runners. Four independent blockers:

- **Photon.** DCGO's AI mode is not offline (`docs/DCGO_HARNESS.md`); it creates
  a private one-seat Photon room. CI would depend on a live third-party service.
- **The build.** Multi-GB Unity LFS checkout plus batchmode license activation.
  Layer A deliberately routes builds to `D:\dcgo-build\`, outside anything git sees.
- **Display.** Headless `-batchmode` *play* is explicitly out of scope; the
  harness drives a real windowed player.
- **Redistribution.** A build contains Bandai card art and DCGO2 code; pushing it
  to a CI artifact store is a posture decision, not an implementation detail.

So the two halves split:

```bash
# gates every PR -- no Unity, milliseconds, GitHub-hosted
dcgo-harness exam --sim-only qa/dcgo-exams/

# the oracle pass -- deliberate, local, ~40s of Unity per scenario
dcgo-harness exam --card EX12-035

# scenario-suite regression replay against the oracle
dcgo-harness exam --suite
```

`--sim-only` runs the line in our engine and checks the `assert:` block. It
cannot find a divergence — it asserts our engine still agrees with what the
oracle previously confirmed. That is the regression half, and it is the half
GitHub can run. A new `dcgo-exam-sim.yml` joins the existing `dsl-guards` /
`engine-clone-safety` gates.

A self-hosted Windows runner for a nightly oracle pass stays an open option,
deliberately not built now.

## Agent surface

`dcgo-harness mcp`, a subcommand of the existing binary rather than a new crate,
so every behavior stays unit-testable with no MCP client — matching how
`dcgo-replay` and `digimon-engine-mcp` share a core. Tools: `exam_card`,
`run_scenario`, `exam_status`.

Per CLAUDE.md's MCP boundaries this is write-capable dev/test tooling, parallel
to `digimon-scenario-mcp`: local and dev only, never bundled into a production
build, never imported by `server.*` or `digimon_gym.*`.

A `/dcgo-exam` skill sits on top and composes with existing card work:
`/batch-implement-cards-rust-dsl` implements, per-card tests go green,
`/dcgo-exam` cross-examines against the oracle, confirmed clauses backfill
assertions and draft `cards_behavioral` tests.

## Failure taxonomy

Extends the taxonomy phase 1 established. Every failure gets a distinct,
non-silent outcome.

| Failure | Response |
|---|---|
| Action-space hash mismatch | Refuse at `up`. Never run. |
| Scenario line illegal in our engine | Fail during lowering, before Unity launches |
| Prompt mismatch mid-line | Abort job, report **as a finding** |
| DCGO lacks the card | Verdict `unavailable`, never `confirmed` |
| Unity hung | Existing `watch` path: stale heartbeat, kill, requeue, quarantine after two |
| Job timeout | Requeue, then quarantine |

Plus the standing rule: always print the full denominator.

## Testing

**The only thing that needs Unity to test is Unity.**

Lowering, the normalized projection, the differ, the verdict store, and the
drafter are pure Rust over fixtures. A committed golden scenario plus its DCGO
state dump lets the differ run in CI where no Unity exists.

The Unity side is covered by the Editor-vs-player determinism check the build
already uses: run a golden scripted scenario at a fixed seed in both and require
byte-identical recordings and state dumps.

## Sequencing

Piece 1 carries the real unknowns and gates the rest.

1. `DeckStacker` + `StateDumper` — verifiable against an existing `policy: "ai"`
   job. Proves state-extraction fidelity before anything depends on it.
2. `InputDriver` + `policy: "scripted"` — the hard part. Timeboxed spike first
   (see Risks).
3. Rust: scenario schema, lowering, sim-side runner, differ.
4. Exam layer: verdict store, clause binding, assertion backfill, drafter.
5. CI workflow, MCP subcommand, skill.

Each of 1-2, 3, and 4-5 gets its own implementation plan.

## Risks

**`InputDriver` across 14 heterogeneous sites is the main technical risk.** Each
`DrivesLocalSeat` site has a different prompt shape, and the `expect:` contract
requires describing all of them uniformly enough to assert against. If that
surface will not unify, the fallback is asserting prompt *kind* only, without
context — weaker, still useful, but it changes what a "prompt mismatch" finding
can tell you. Resolve with a timeboxed spike at step 2, before committing to the
richer contract.

**Hand-authored lines may prove too expensive per clause.** An `[On Deletion]`
clause on a Lv.6 needs a long legal prefix. If authoring cost dominates, the
escape hatch is a declarative planner (state the target position, search our
engine for a legal line reaching it) — considered and set aside for now, not
ruled out. The `unreachable` verdict class exists partly so this shows up as data
rather than as quiet gaps.

## Operating agreement

Unchanged from phase 1: triage and report, ask before fixing. Findings are ranked
and diagnosed; fixes stay a decision.

## Out of scope

- A "materialize board" cheat in DCGO (rejected on oracle-trust grounds).
- A declarative position planner (deferred; see Risks).
- Self-hosted nightly CI runner (option preserved, not built).
- Publishing the DCGO build (deferred; the manifest is already in the shape it needs).
- Running our trained models as the DCGO policy. `InputDriver`'s policy seam is
  where such a policy would attach.
