# DCGO Card-Clause Exam

Makes DCGO answer a targeted question about a card **while you are authoring
it**, instead of only reporting what its bot happened to do.

Design: `docs/superpowers/specs/2026-08-21-dcgo-scripted-scenario-exam-design.md`.
Job harness (phase 1): `docs/DCGO_HARNESS.md`.
Recording format: `docs/DCGO_RECORDING_SCHEMA.md`.

## The question it answers

Phase 1 of the harness generates volume: DCGO's own AI plays both seats, and the
corpus answers **"what did DCGO do"**. That is the right shape for finding
divergences you did not predict, and the wrong shape for card authoring, which
asks the opposite question: *does `EX12-035`'s `[On Play]` clause resolve the way
we implemented it?* The only phase-1 answer to that is "generate more games and
hope the bot triggered it".

The exam asks **"what does DCGO do HERE"**. Both seats are scripted, the deck is
stacked, and the line is a legal play sequence from game start that reaches the
clause on purpose. The unit of measurement is not a game and not a card — it is a
**clause**, keyed to the same identity `clause_coverage` already uses, so "all of
this card's clauses" is checkable rather than a feeling.

Two constraints are forced, not chosen, and they shape everything downstream:

- **DCGO reaches a position only by legally playing to it.** A "materialize
  board" cheat was rejected on oracle-trust grounds — a hand-built board can miss
  internal wiring the normal play path sets up, so a divergence might be the
  cheat's fault rather than a real parity bug. The consequence must be stated
  plainly: **existing `cards_behavioral` tests are not exam-able.** They use
  `DebugRunner` staging. Exam scenarios are a new artifact.
- **Both seats are fully scripted.** If DCGO's AI plays the opponent, our engine
  cannot reproduce its choices, so every game diverges for reasons that are not
  findings.

## Quick start

```bash
# Every committed scenario, our engine only. No Unity. Milliseconds.
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams/ --sim-only \
    --cards-json data/cards.json

# The oracle pass for one card -- deliberate, local, ~40 s of Unity per scenario.
cargo run -p dcgo-harness -- exam --card EX12-035 --cards-json data/cards.json

# Regression replay of the whole scenario suite against the oracle.
cargo run -p dcgo-harness -- exam --suite
```

The oracle pass submits a `policy: "scripted"` job into the phase-1 queue and is
subject to every phase-1 rule: the harness must be enabled (`dcgo-harness
enable`), Unity must be in Play, and a job overdue past `timeout_seconds` is
requeued then quarantined after two failures. See `docs/DCGO_HARNESS.md`.

## The scenario file

`qa/dcgo-exams/<SET>/<CARD-ID>.yaml`. One file per clause.

```yaml
card: EX12-035
clause: EX12-035#effect#2      # a clause_coverage Clause.id -- {card_id}#{zone}#{idx}
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-035, BT16-082], rest: vb-standard }
  p1: { stack: [],                           rest: vb-standard }
steps:
  - actor: 0
    do:     { hatch: {} }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { play: { card: EX12-035, from: hand } }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { select: { targets: [opp.field.0] } }
    expect: { prompt: select_permanent, count: 1 }
assert:
  - at: 3
    that: { opp.field.0.dp: 3000, p0.memory: -2 }
```

The line above is **abridged**. `EX12-035#effect#2` is a Lv.6's
`[On Play]/[When Digivolving]` clause, and a real line reaching it carries a long
legal prefix (hatch, promote, digivolve up the line, pay memory). That authoring
cost is a known risk — see "Known gaps".

| Key | Meaning |
|---|---|
| `card` | The card under exam. |
| `clause` | A `clause_coverage.models.Clause.id`, `{card_id}#{zone}#{idx}` (e.g. `EX12-073#security#0`, produced at `code/tools/clause_coverage/card_sources.py:149`). **Not free text** — see below. |
| `seed` | Passed through to the job. The whole run is seed-deterministic on both sides (`docs/DCGO_HARNESS.md`, "Determinism"). |
| `decks.<seat>.stack` | Prefix of the draw order. See below. |
| `decks.<seat>.rest` | Named deck the remainder is seeded-shuffled from. |
| `steps[].actor` | `0` or `1`. Both seats are scripted. |
| `steps[].do` | A **symbolic** action — `hatch`, `pass`, `play`, `digivolve`, `attack`, `select`. |
| `steps[].expect` | The prompt this step expects to be answering. Optional per step, asserted **before** answering. |
| `assert` | `at: <step index>` + `that: {path: value}`. Backfilled from the oracle — see "Assertion backfill". |

### `clause:` is an identity, not a label

The exam keys into `clause_coverage`'s denominator: it splits a card's printed
text into clauses by timing marker (`[On Play]`, `[When Digivolving]`,
`[Security]`, …) and by angle-bracket keyword, resolving text from the official
Bandai bundle first and `cards.json` + overrides second, and emitting an explicit
`image-required` slot rather than concluding "no clause" from a lossy source.

```bash
PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX12-035
# Clause extraction: 1 cards -> 8 clauses
#   by zone:   effect=6  inherited=1  security=1
```

**That 8 is the denominator every later report must print.**

A scenario naming a clause the extractor does not produce for that card is
rejected. Otherwise a typo silently creates a sixth, invisible verdict class: a
scenario that passes while covering nothing in the denominator.

The id's stability is conditional on the card's text, because `idx` is positional
within a zone. If an override or a re-scrape changes the text, ids shift and
previously-recorded verdicts silently re-point at *different* clauses. So the
verdict store records the clause `label` and `text_sha256` alongside the id, and
a mismatch on re-read invalidates that verdict back to `unmeasured` rather than
reporting a stale `confirmed`.

### `stack` is a PREFIX, and applies to the INITIAL SHUFFLE ONLY

You name the first N cards in draw order; the remainder is seeded-shuffled from
the named deck. Requiring all 50 would make every file unauthorable.

Initial-shuffle-only is a **correctness requirement, not an optimization.** If
the stack applied to every shuffle, a card reading "shuffle your deck" would
silently re-impose the opening order, and the exam would confidently answer a
question about a game that cannot occur. Mid-game shuffles fall through to seeded
`GameRandom` — still fully deterministic, but honest.

**This is structural, not enforced.** The stack is applied at the two phase-1
harness deck short-circuits, which are the only deck-construction path a job
takes:

| Site | Deck |
|---|---|
| `CardObjectController.cs:141` — `DeckRecipie(player)` | Main |
| `CardObjectController.cs:235` — `DigitamaDeckRecipie(player)` | Egg |

Mid-game `CardObjectController.Shuffle(Player)` (line 1040) never passes through
a harness short-circuit (`Shuffle(Player)` is an ordinary
`RandomUtility.ShuffledDeckCards` call), so there is nothing to exclude and no
latch to get
wrong. `rest` still has to name a **tournament-legal** list: `Game::new` does not
validate deck legality, but DCGO gates battles on `DeckData.IsValidDeckData()`
(50 main, ≤5 egg, per-card legality), so an illegal list runs on our side and
refuses on DCGO's. Both short-circuits already resolve the seat
(`player == MasterPlayer ? P0 : P1`), so the stacker gets player identity for
free — which is also why it does **not** hook `RandomUtility.ShuffledDeckCards`,
a helper that receives no player argument and would have forced the stacker to
guess the seat from call order.

### `expect:` is asserted BEFORE the step is answered

A driver that answers whatever it is asked will, on one ordering mismatch,
**desynchronize the entire remainder of the line while every step still looks
successful.** Every subsequent answer lands on the wrong prompt, the game plays
out to a plausible-looking end, and the report says nothing.

So each step carries the prompt it expects, and the driver asserts *first*, then
answers. A mismatch aborts the job and reports itself.

**That is a finding, not an error.** "Our engine expected a choice here and DCGO
never asked" is exactly the divergence class that never surfaces as an illegal
action, and it is invisible to the phase-1 replay funnel.

DCGO has **13 decision kinds** a scripted line can be asked to answer — the 10
`LogSelectionRow` prompts plus three families that are not selections:

| Kind | DCGO seam |
|---|---|
| `SelectCardEffect` | `SetTargetCardAndIndicies` |
| `SelectHandEffect` | `SetTargetHandCards` |
| `SelectPermanentEffect` | `SetTargetFrames` |
| `SelectAttackEffect` | `SetAttackTarget` |
| `SelectCountEffect` | `SetCount` |
| `SelectDigiXrosClass` | `SetTargetDigiXrossIndex` |
| `MultipleSkills` | `SetTargetSkill` |
| `OptionalSkill` | `SetUseOptional` |
| `generic_int` | `UserSelectionManager.SetIntForPlayer` |
| `generic_bool` | `UserSelectionManager.SetBoolForPlayer` |
| mulligan | `TurnStateMachine.LogMulligan` |
| breeding | `TurnStateMachine.LogBreedingAction` |
| main phase | `TurnStateMachine.QueueMainPhaseAction` |

The driver intercepts at these **RPCs**, not at the 13 `HarnessAuto.
DrivesLocalSeat` gates. Same reasoning rule 27 documents on the reading side: the
gates are 13 heterogeneous sites across 10 files with two polarities and two
control-flow shapes, while the RPCs are one uniform shape — and they are the same
places `GameRecorder` already hooks, so **what is recorded is exactly what is
scriptable**, and a scenario authored from a recording can always match by
construction.

### `do:` is symbolic, lowered to an action ID once, up front

Both engines already share the 2192-slot action space: the recorder encodes to
it, `dcgo-replay` decodes from it, and `ActionSpace.cs` is codegen'd from
`space.rs` behind a CI drift gate (rule 27). Raw integers in a YAML file would be
unwritable by hand, unreviewable in a diff, and would rot silently on
renumbering.

Lowering resolves each symbolic step against **our engine's live action mask**,
failing loudly on illegal or ambiguous intent, and writes the resulting integers
into the job file. So DCGO and our engine consume literally the same integers,
and a malformed scenario fails in milliseconds instead of after sixty seconds of
Unity.

`Game::decode_action` returns `()` and silently ignores an out-of-range id, so
the mask bit is the only check there is — lowering asserts it before applying.

## Selection steps (`select:`) — added 2026-08-22

Clauses whose resolution prompts a selection are authorable. Five forms, exactly
one per step:

```yaml
- actor: 0
  do: { select: { cards: [EX12-020, EX12-020] } }   # hand/trash/reveal picks, by identity
  expect: { prompt: SelectHandEffect }
- actor: 0
  do: { select: { targets: [opp.field.0] } }        # permanent picks, by OUR slot ref
  expect: { prompt: SelectPermanentEffect }
- actor: 0
  do: { select: { value: 3 } }                      # count/int prompts (the VALUE, not an index)
- actor: 0
  do: { select: { yes: true } }                     # optional yes/no (OptionalSkill)
- actor: 0
  do: { select: { decline: true } }                 # cancel an optional prompt
```

**The wire carries identities, never engine-internal indices.** Our selection
encodings and DCGO's `ActiveCardList` CardIndex / frame ids are both internal;
the job ships card-ID strings (for permanents, the target's top-card id) and
each engine resolves them against its *own* candidate list —
`SelectionAnswer.MatchCardIds` on the DCGO side, `selection_resolve::resolve_next`
on ours. Duplicates resolve in occurrence order; an identity the prompt cannot
offer **aborts the job as a finding** ("DCGO does not offer what our engine
offered").

Two lowering invariants make selection mistakes loud at sim time, before any
Unity is spent:

- a pending selection **must** be answered by the next scenario step
  ("our engine asks a selection here; the scenario must answer it");
- a `select:` while nothing is pending is an error, unless the engine
  auto-resolved the prompt (allowed and logged — DCGO may still ask).

Phase at a selection boundary is **representation**: our engine parks in a
`Select*`/`EffectChoice` `GamePhase` while DCGO's `TurnPhase` stays on the
interrupted phase, so the differ compares every state field but not `phase` on
those steps (`Breeding`-vs-`Main` still diffs).

Proven end to end on `qa/dcgo-exams/ST1/ST1-15-effect0.yaml` ("Delete up to 2 of
your opponent's Digimon with 4000 DP or less"): CLEAN, 14/14 steps.

Out of scope, stated per the brief: multi-pick DigiXros/Assembly material
declarations and `indexes`-payload prompts; `generic_int`/`generic_bool` values
can be scripted but their candidates cannot be asserted (no candidate list on
that channel).

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
                         \--> recording.jsonl + <name>.state.jsonl
                                    |
                               differ ---> first divergence
```

Our runner is **not a special code path**. `digimon_engine::runners::replay`
already defines a `RecordingSource` trait with two implementations —
`NativeAdapter` and `DcgoAdapter`. The sim-side scenario runner is a third,
`ScenarioAdapter`, feeding lowered action IDs into the same `ReplaySession`. The
divergence machinery, step policy, and player-perspective conversion are
inherited rather than reimplemented, so a scenario run is structurally the same
object as a corpus replay.

If our engine can only reach the position via `DebugRunner` staging, the scenario
is **not exam-able and says so**, rather than quietly comparing a staged board
against a played one.

### Normalized projection

Both sides emit one row per decision boundary, keyed by the recorder's step index
so the two align. DCGO's side is `StateDumper`, writing the recording path with
`.jsonl` replaced by `.state.jsonl`.

| Field | Normalization |
|---|---|
| battle area | multiset of `{card_id, effective_dp, sources[], suspended, keywords[]}` |
| hand / trash | multiset of card IDs |
| security | **count only** — contents are hidden information |
| memory | integer, both from the recording player's perspective |
| phase / turn | as-is |

The governing rule: **normalize representation, never semantics.** Effective DP
is representation — the two engines track modifiers differently and a
modifier-list diff would be pure noise. Whether a Digimon is suspended is
semantics, and must diff. Under-normalize and the report drowns in noise nobody
reads; over-normalize and it hides the bug.

Two accessor traps on the DCGO side, both of which manufacture false
divergences:

- Use `Player.GetBattleAreaPermanents()` (`Player.cs:621`), **not**
  `GetFieldPermanents()` (`:669`) — the latter walks every frame including
  BREEDING, so a hatched egg shows up as though it were on the battle field. The
  recorder learned this already (`Recording/ActionEncoder.cs:577-584`).
- Use `Permanent.DP` (the property that walks `IChangeDPEffect`, and the value
  `CardController.CompareStats()` actually compares), **not** `Permanent.BaseDP`.

Keywords have no collection in DCGO — there are 22 independent bool properties,
each of which walks the whole field's effect lists. Dumping all of them per
permanent per step is O(22 × field × effects), so **keyword dumping is behind a
flag, default OFF**, enabled only for scenarios whose clause is a keyword clause.
The projection must treat an absent keyword list as **"not measured", never as
"no keywords"** — otherwise every non-keyword scenario reports a false keyword
divergence.

### Differ

Walks aligned step indices and **leads with the first divergence, marking
everything after it downstream.** Once the two engines part they are playing
different games; a report ranking fifty consequences beside one cause is a report
nobody finishes.

## The five verdict classes

`qa/qa-reports/dcgo_exam_verdicts.json`, one row per `(card, clause)`.

| Verdict | Meaning |
|---|---|
| `confirmed` | A scenario exercised this clause and both engines agreed for the whole line |
| `diverged` | Both engines ran it and disagreed |
| `unreachable` | Scenario exists but the line could not legally reach the clause |
| `unavailable` | DCGO's pool does not contain this card, so no oracle exists |
| `unmeasured` | No scenario authored yet |

`unmeasured` is the point of the table. The report reads:

```
EX12-035: 8 clauses -- 5 confirmed, 1 diverged, 2 unmeasured
```

**never** `EX12-035: passed`. This is the same rule phase 1 earned the hard way:
a batch where most jobs died must never read as a pass
(`docs/DCGO_HARNESS.md`, "Rules that matter").

`VerdictSummary::describe()` always prints the denominator and the count of
verdicts invalidated by clause-text drift.

### Three honesty constraints

- **`unavailable` is determined per card, not per set.** DCGO's
  `Assets/Scripts/CardEffect/` spans AD1, BT1–BT26, EX1–EX12, ST1–ST24, LM, P and
  RB1 — broad coverage, so "newer than DCGO" is the *wrong* test. A set directory
  routinely exists while an individual card in it has no script. The check globs
  for `<SET>/<COLOR>/<CARD_ID>.cs` (underscored filename: `BT17-001` →
  `BT17_001.cs`). No oracle exists for such a card, and that must read as
  "not verified", never as "passed".

  The checker **panics** if the DCGO card-effect directory is missing rather than
  returning `false`. In a worktree the local `./DCGO` is an intentionally-empty
  placeholder (rule 29) and a checker pointed there would report *every* card
  unavailable — quietly turning the whole exam into a no-op that reads as
  "nothing to verify". Point it at the base repo:
  `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO`.

- **`diverged` does not mean we are wrong.** Source priority puts
  `general_rule.pdf` above DCGO. A divergence is ranked and diagnosed; the fix
  stays a decision, not an automation. The operating agreement is unchanged from
  phase 1: **triage and report, ask before fixing.**

- **25 cards are structurally unmeasurable.** They register
  `SetIsBackgroundProcess(true)` and are activated by
  `AutoProcessing.ActivateBackgroundEffectsOfCards`, which calls
  `ActivateICardEffect.Activate(hashtable)` directly, bypassing the
  `effect_activation` hook entirely (rule 27; `docs/DCGO_RECORDING_SCHEMA.md`).
  Their clauses get `unreachable` carrying that specific reason — **not a silent
  pass.**

### Assertion backfill

On a `confirmed` verdict the exam writes the confirmed DCGO state into the
scenario's `assert:` block. You author the line; the oracle records what
happened; that becomes the permanent guard that survives into CI after the oracle
has gone home.

This is what makes `assert:` load-bearing rather than decorative, and it is the
only reason the Unity-free CI half has anything to check.

### Test drafter

Emits a draft `#[test]` into `code/digimon-engine/tests/cards_behavioral/<set>/`
with the header `DCGO build <hash>, job <id>, scenario <path> observed:` — never
asserting correctness, never auto-committing.

DCGO is source-priority #2, below `general_rule.pdf`. A drafted test encodes
**strong evidence, not truth.** An auto-generated test asserting a behavior
nobody read would launder a DCGO quirk into a permanent guard, which under the
no-approximations policy is worse than no test.

## The two run modes, and the CI split

| Mode | Needs Unity | Can find a divergence | Where it runs |
|---|---|---|---|
| `exam --sim-only` | No | **No** | Every PR, GitHub-hosted, milliseconds |
| `exam --card` / `exam --suite` | Yes | Yes | Deliberate, local, ~40 s per scenario |

`--sim-only` replays the scripted line in our engine alone and checks the
backfilled `assert:` block. **There is no oracle in that job** — nothing observes
DCGO. A green run means "no regression against previously recorded oracle state",
never "our engine matches DCGO". Do not describe the gate as if it could discover
a divergence, and never read a green run as a per-card pass.

`.github/workflows/dcgo-exam-sim.yml` joins the existing `dsl-guards` /
`engine-clone-safety` gates, and checks out with `submodules: false`. **If that
job ever needs the DCGO submodule, the design has drifted** — the whole point of
the split is that the only thing needing Unity is Unity.

### Why DCGO cannot gate PRs on GitHub-hosted runners

Four independent blockers. Any one of them is sufficient.

1. **Photon.** DCGO's AI mode is not offline (`docs/DCGO_HARNESS.md`); it creates
   a private one-seat Photon room. Gating PRs on it would make every PR depend on
   a live third-party service.
2. **The build.** A multi-GB Unity LFS checkout plus batchmode license
   activation. Layer A deliberately routes builds to `D:\dcgo-build\`, outside
   anything git sees.
3. **Display.** Headless `-batchmode` *play* is explicitly out of scope; the
   harness drives a real windowed player, which a GitHub-hosted runner does not
   have. (Headless *compile* and EditMode tests are available and are used for
   the Unity-side unit tests.)
4. **Redistribution.** A build contains Bandai card art and DCGO2 code; pushing
   one to a CI artifact store is a posture decision, not an implementation
   detail.

A self-hosted Windows runner for a nightly oracle pass stays an open option,
deliberately not built.

## Testing

**The only thing that needs Unity to test is Unity.** Lowering, the normalized
projection, the differ, the verdict store, and the drafter are pure Rust over
fixtures; a committed golden scenario plus its DCGO state dump lets the differ
run in CI where no Unity exists. A hand-written sidecar would make the differ
agree with a file nobody's DCGO ever produced — if the golden pair does not exist
yet, the test is `#[ignore]`d with a comment naming the dependency, never
fabricated.

The Unity side is covered by the Editor-vs-player determinism check the build
already uses: run a golden scripted scenario at a fixed seed in both and require
byte-identical recordings and state dumps.

## Failure taxonomy

Extends the taxonomy phase 1 established. Every failure gets a distinct,
non-silent outcome.

| Failure | Response |
|---|---|
| Action-space hash mismatch | Refuse at `up`. Never run. |
| Scenario line illegal in our engine | Fail during lowering, before Unity launches |
| Scenario names a clause the extractor does not produce | Reject the scenario |
| Prompt mismatch mid-line | Abort the job, report **as a finding** |
| DCGO lacks the card | Verdict `unavailable`, never `confirmed` |
| Clause text drifted since the verdict | Invalidate to `unmeasured`, never stale `confirmed` |
| Unity hung | Existing `watch` path: stale heartbeat, kill, requeue, quarantine after two |
| Job timeout | Requeue, then quarantine |

Plus the standing rule: **always print the full denominator.**

## Known gaps

Stated plainly, because each one is a way a report can read better than the
evidence supports.

- ~~`job.first_player` not honored~~ — **fixed 2026-08-22** (see
  `docs/DCGO_HARNESS.md`). DCGO seats the requested player first; lines are
  authored against our engine's convention (seat 0 first) and verified both
  directions at one seed.

- **The 25 `SetIsBackgroundProcess(true)` cards are structurally unmeasurable.**
  They bypass the `effect_activation` hook entirely, so their clauses can never
  be confirmed by activation matching. They get `unreachable` with that reason.
  This is **not detected automatically** — no tool enumerates those cards from
  the DCGO source, because doing so needs a C# scan that belongs with the Unity
  work, and a hard-coded list of 25 ids would rot silently. Until then the reason
  is written by hand.

- **`generic_int` / `generic_bool` selection rows carry no candidate list.** They
  are `UserSelectionManager`'s fallback channel — every *typed* prompt routes
  through its own dedicated hook (rule 27). Without a candidate list there is no
  way to recover what a value MEANS for a given prompt, which is a real limit on
  scripting them: a scenario can supply the integer, but neither the author nor
  the reviewer can tell from the row what was chosen.

- **A possible further local-seat gate inside `TurnStateMachine.SetMainPhase()`
  (line 1211) is unverified.** It was not read during recon. If one exists and is
  unhooked, main-phase decisions could route around the driver in some path.
  Confirm it before trusting a clean main-phase line.

- **The scenario `prompt:` vocabulary is a free string, not yet an enum.** The
  YAML uses snake_case names (`main_phase`, `select_permanent`) and DCGO's own
  vocabulary is the 13 kinds tabulated above; the mapping between them is the one
  place the two sides must be kept in step, and a typo currently reads as a
  prompt mismatch rather than a schema error.

- **Authoring cost may dominate.** An `[On Deletion]` clause on a Lv.6 needs a
  long legal prefix. If this proves too expensive per clause, the escape hatch is
  a declarative planner (state the target position, search our engine for a legal
  line reaching it) — considered and set aside, not ruled out. The `unreachable`
  verdict class exists partly so this shows up as **data rather than quiet gaps.**

## Gotchas found the hard way

These are inherited from phase 1 and apply unchanged to every scripted job. Full
list in `docs/DCGO_HARNESS.md`.

- **DCGO work happens in the base repo** (rule 29). In a worktree `./DCGO` is an
  intentionally-empty placeholder; never `git submodule update --init` it.
- **Stop Play before requeueing.** A running harness claims jobs the instant they
  appear, so files moved into `jobs/` during Play are fed to whatever code
  version is currently loaded.
- **Unity must have imported new scripts.** No `.meta` beside a `.cs` means Unity
  has not seen it, and a "clean" Console is reporting the *previous* compile.
- **After ANY change to `code/digimon-engine/src/action/space.rs`**, regenerate
  `ActionSpace.cs` in the base-repo DCGO (rule 27). Lowered action IDs are the
  wire between the two engines; drift there silently re-points every step.

## See also

- `docs/DCGO_HARNESS.md` — the phase-1 job harness this runs on top of
- `docs/DCGO_RECORDING_SCHEMA.md` — the JSONL rows, including `effect_activation`
- `docs/DCGO_BUILD.md` — building the modded client
- `qa/dcgo-exams/README.md` — the scenario directory's own format reference
- `code/tools/clause_coverage/` — the clause denominator and `exam_binding.py`
- `code/tools/dcgo-harness/src/exam/` — scenario, lowering, projection, differ,
  verdict store, backfill, drafter
- `qa/qa-reports/dcgo_exam_verdicts.json` — the verdict store itself
