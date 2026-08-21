# DCGO Automation Harness — Design

**Date:** 2026-08-17
**Status:** Approved design, not yet implemented
**Related:** `docs/DCGO_BUILD.md`, `docs/DCGO_RECORDING_SCHEMA.md`, `docs/DEBUG_MCP.md`,
`openspec/changes/add-dcgo-recording-parity-harness/`

## Problem

The DCGO parity pipeline works but is throttled by a human. Every iteration
costs a manually played Unity game: a person opens DCGO, plays to completion,
and only then can `dcgo-replay` say whether the engine agrees. Six games over
two days surfaced roughly a dozen defects — the funnel is productive, but it
runs at the speed of manual play.

Three capabilities are wanted, and they are usually described as separate
projects:

1. **Volume** — unattended bot-vs-bot games producing a large recording corpus.
2. **Targeted** — drive DCGO to a specific state to see how it resolves one
   card or interaction, as ground truth for card authoring and test writing.
3. **Bidirectional** — feed a recorded action stream back *into* DCGO and
   compare its state against the engine's, step by step.

## Key finding: the automation already exists

DCGO ships an unattended, self-restarting bot-vs-bot loop. It is not exposed in
any menu — it is an Inspector checkbox, `GManager.isAuto` ("オートモード"),
gated to `IsAI` games. It drives four sites in `TurnStateMachine`:

| Line | Behavior |
|---|---|
| 485 | Auto-answers the local player's mulligan (`RandomUtility.IsSucceedProbability(0.5f)`) |
| 875 | Skips the local player's breeding decision (jumps straight to `Main`) |
| 1376 | Runs `autoProcessing.EndTurnProcess()` for the local seat — the AI plays *both* sides |
| 3605 | On game end, reloads `BattleScene` and starts another game |

An earlier assumption in this campaign — that DCGO has no bot-vs-bot mode — was
wrong. The work is not to build automation but to *drive* it, fix its coverage
gaps, and wrap it in a job protocol.

The microscope end also already exists: `digimon-engine-mcp` accepts DCGO JSONL
via `load_recording`, and offers `seek`, `step_forward` / `step_back`,
`replay_step_view`, and `scan_divergences`. `dcgo-replay` is the batch funnel.
What is missing is the loop that ties generation to triage.

### Coverage gap that must be fixed

Line 875 makes the local seat skip breeding entirely. Left alone, every
generated game would have P0 never hatching and never moving from the breeding
area — silently omitting a whole mechanic from the corpus while appearing to
work. The fix routes the local seat through the same AI breeding path the
opponent seat already uses (`doHatch` at ~890).

## Core insight: one substrate, three policies

The three capabilities are not three systems. Each is a **scripted-input driver
for DCGO** — forced deck order, injected actions, injected selection answers —
differing only in what supplies the inputs:

| Capability | Input policy |
|---|---|
| Volume | DCGO's own AI (`isAuto`), looping |
| Targeted | A scripted action list over a stacked deck |
| Bidirectional | A recorded action list, with state dumps to diff |

Building the substrate once and varying the policy is the same "widen the
substrate rather than route around it" discipline as CLAUDE.md rule 28. If the
live-server option is ever wanted, it becomes a fourth policy rather than a
redesign.

Both hard parts are already feasible:

- **Action injection** — `TurnStateMachine.QueueMainPhaseAction(player, action)`
  is public; it is the same chokepoint the recorder hooks for reading.
  Selections go through `UserSelectionManager.SetIntForPlayer` /
  `SetBoolForPlayer`, also already hooked.
- **Deck stacking** — shuffling happens at one call site
  (`CardObjectController.Shuffle`), so forcing a recorded order is the mirror of
  the capture already implemented.
- **`CheatAction`** exists for state setup, gated behind `AllowCheats()`:
  `Draw`, `TrashCard`, `PlaceCardOnDeck`, `PlaceCardInSecurity(+Faceup)`,
  `GainMemory`, `LoseMemory`, `RemoveFromSecurity`.

## Architecture

```
┌─ DCGO (Unity) ──────────────┐   ┌─ filesystem ────┐   ┌─ host ─────────────┐
│  Digimon.Harness            │   │  jobs/          │   │  dcgo-harness CLI  │
│   JobWatcher  (poll+claim)  │◄──┤   claimed/      ├──►│   submit / status  │
│   DeckStacker (force order) │   │   done/ failed/ │   │   triage           │
│   InputDriver (3 policies)  │   │  recordings/    │   │   (thin MCP mode)  │
│   StateDumper (phase 3)     ├──►│  state/         │   │  dcgo-replay       │
│  Digimon.Recording (exists) │   │                 │   │  engine-mcp        │
└─────────────────────────────┘   └─────────────────┘   └────────────────────┘
```

### Control channel: file-drop

DCGO watches a jobs directory; each job file fully specifies a game. DCGO plays
it and writes back a recording plus, optionally, a per-step state dump. The host
side only reads and writes files.

Chosen over a live HTTP/WebSocket server inside Unity because it needs no
listener, no main-thread marshalling, and no port lifecycle, and because every
run is reproducible from its job file. It covers all three capabilities:
bidirectional comparison does not need interactivity, since "feed actions, dump
state per decision boundary, diff offline" is both sufficient and more
reproducible than live stepping.

Rejected alternatives:

- **Live debug server in DCGO** — better ergonomics for poking at one state, but
  real complexity (threading, ports) for a benefit only the targeted capability
  partly wants. Deferred behind the same job abstraction; addable later without
  redesign.
- **Headless Unity `-batchmode`** — zero channel and CI-friendly, but DCGO is
  animation- and coroutine-heavy and its bot path runs through
  `ContinuousController` coroutines. Likely fragile to broken; not worth betting
  the design on.

### DCGO-side components

A new `Digimon.Harness` namespace beside `Digimon.Recording`:

- **`JobWatcher`** — MonoBehaviour. Polls `jobs/`, claims a job by atomic rename
  into `claimed/`, applies it, starts the game; on `EndGame` files the result and
  takes the next job. Replaces the blind restart at `TurnStateMachine:3605` with
  a job-driven one.
- **`DeckStacker`** — forces post-shuffle main and egg order from the job spec at
  the `CardObjectController.Shuffle` call site.
- **`InputDriver`** — supplies decisions per policy (`ai` / `scripted` /
  `recorded`) through the existing `QueueMainPhaseAction` and
  `UserSelectionManager` chokepoints.
- **`StateDumper`** — phase 3 only. Snapshots DCGO's board at each decision
  boundary.

`GameRecorder` is untouched. It already emits exactly the JSONL wanted, so the
harness decides only *which games are played*, never *how they are recorded*.

### Host-side component

One Rust crate, `code/tools/dcgo-harness/`, a workspace member alongside
`code/tools/dcgo-replay/`. It holds all logic: job-spec types, deck-pool sampling,
corpus triage, and (phase 3) the state differ.

The MCP is a **subcommand of that same binary** (`dcgo-harness mcp`), not a
fourth MCP crate. This keeps every behavior unit-testable without an MCP client
and follows how `dcgo-replay` and `digimon-engine-mcp` already share a core. Per
CLAUDE.md's MCP boundaries it is a write-capable dev/test tool, parallel to
`digimon-scenario-mcp`: local and dev only, never bundled into a production
build, never imported by `server.*` or `digimon_gym.*`.

## Job protocol

A job is one JSON file; its state is its directory.
`jobs/` → (atomic rename) `claimed/` → `done/` or `failed/`.

```jsonc
{
  "job_id": "vol-0042",
  "policy": "ai",                     // ai | scripted | recorded
  "decks":  { "p0": "<DCGO deck code>", "p1": "<DCGO deck code>" },
  "first_player": 0,
  "seed": 12345,
  "deck_order": { "p0": ["EX12-035", ...], "p1": [...] },  // optional; phase 2+
  "inputs": [ ... ],                                       // scripted/recorded only
  "dump_state": false,                                     // phase 3
  "limits": { "max_turns": 40, "timeout_seconds": 180 }
}
```

The `inputs` array is left unspecified here because it is a phase-2 concern and
its shape is already determined: it mirrors the `action` and `selection` rows of
`docs/DCGO_RECORDING_SCHEMA.md`, so a recorded game can be replayed as a job
without translation. Pinning it now would duplicate a schema that is still
settling.

Claiming by atomic rename makes crash recovery legible: a job sitting in
`claimed/` past its timeout is a hung Unity run. `done/<job_id>.result.json`
carries the recording path, outcome, step count, and duration.

Jobs carry **card ID lists**, not DCGO deck codes. DCGO's deck code is a base-n
encoding over its internal integer `CEntity_Base.CardIndex`, so reimplementing it
host-side would mean mirroring DCGO's card-index table and its n-ary/m-ary
conversion — fragile, and duplicated logic that would silently rot when DCGO
re-indexes.

Instead the job carries `[EX12-035, ...]`, and DCGO resolves each ID against
`ContinuousController.instance.SortedCardList` (matching `CEntity_Base.CardID`),
builds the code with its own `DeckData.GetDeckCode`, and constructs the
`DeckData`. The encoding stays owned by the one codebase that defines it.

The CLI still owns deck *selection* — sampling real meta decks from
`data/deck_library.json` filtered through the tested-cards gate, so the corpus
exercises real matchups rather than toy piles.

Job root defaults to `<Application.persistentDataPath>/dcgo_harness/`, beside the
existing `dcgo_recordings/`, with a config override so the repo-side CLI can be
pointed at it.

## Determinism

**A job must be reproducible from its spec.** `Random.InitState(job.seed)` at
game start, plus forced deck order, means a divergence found in game 137 of an
overnight batch can be re-run exactly.

This is load-bearing, not a nicety: without it, bulk generation produces findings
that cannot be investigated, which defeats the purpose of generating in bulk. It
is also what makes the golden smoke test (below) meaningful.

**To verify early in phase 1:** DCGO's decisions must not depend on frame timing.
Logical decisions should be safe, but if any coroutine races, determinism
degrades — and that must be known before phases 2 and 3 are built on it.

## Data flow

**Volume loop**

1. `dcgo-harness submit --count 200 --policy ai --decks <pool.json>` writes 200
   jobs with rotated deck pairs and varied seeds.
2. DCGO, running with the harness enabled, drains them unattended.
3. `dcgo-harness triage --corpus <dir>` runs the `dcgo-replay` core over every
   recording, clusters divergences by signature, ranks by frequency, and emits a
   repro command per cluster. A **signature** is
   (failure kind, decoded action range, card ID at the referenced slot) — coarse
   enough that fifty recordings hitting one card's bug collapse into a single
   ranked entry, specific enough that two different bugs on the same card stay
   apart.
4. The top cluster is investigated with `digimon-engine-mcp` (`load_recording` →
   `seek` → `replay_step_view`), diagnosed, and reported.

**Targeted probe**

1. `dcgo-harness probe --stack p0=<cards> --script <actions>` submits one job with
   `policy: scripted` and a forced deck order.
2. DCGO plays exactly that line.
3. The recording is DCGO's ground truth for the interaction, feeding card
   authoring and behavioral-test writing.

**Bidirectional**

1. `dcgo-harness recheck --recording <file>` submits a job with
   `policy: recorded`, `dump_state: true`, and the original deck order.
2. DCGO replays the action stream, dumping state at each decision boundary.
3. `dcgo-harness diff` aligns DCGO's snapshots against the engine's at the same
   boundaries.

### Phase 3's alignment assumption

DCGO resolves effects across coroutines and animations, so "state after step N"
is ambiguous mid-resolution. The well-defined sync point is a **decision
boundary** — the moment DCGO next asks a player for input. That is exactly where
the recording already emits rows, so dumps align with recorded steps by
construction rather than by heuristic.

If this assumption breaks, phase 3 becomes materially harder. Phases 1 and 2 do
not depend on it.

## Throughput

Under the harness, raise `Time.timeScale` and skip animation waits. Hundreds of
games at roughly 40 seconds of animation each is a non-starter; this is the
difference between an overnight corpus and an unusable one. Throughput must be
measured in phase 1, since it sets the realistic corpus size for everything
after.

## Error handling

| Failure | Handling |
|---|---|
| Unity hangs or crashes mid-job | Job stalls in `claimed/`; CLI times it out and files it to `failed/` with the `Player.log` tail. **Quarantine after two failures** — never retry indefinitely, or one poisonous job silently stalls the batch |
| Bot loops forever | `limits.max_turns` abandons the game; the partial recording is still filed and still useful. (Our own engine hit exactly this class of bug — see the CannotAttack mask loop) |
| Bad deck code / untested cards | Validated at submit time in the CLI, failing fast with a clear message instead of dying inside Unity |
| Truncated recording | Flush on `game_end` regardless of the row counter; `dcgo-replay` already reports these as PARTIAL |
| Corpus bloat | Recordings are ~8 KB, so 1000 games is trivial; phase-3 state dumps are not — cap and rotate |

**The corpus is derived data and is not committed.** What gets committed is
triage reports and any minimized recording promoted to a regression fixture.

### The failure mode most worth designing against

Silent skipping. If 200 jobs are submitted and 180 die on deck-import errors, a
triage report reading "no divergences" is actively misleading — it looks like the
engine passed. The report therefore **always** states submitted / completed /
partial / failed, and refuses to issue a clean verdict without that denominator.
Same discipline as the "no silent caps" rule in the workflow guidance.

## Testing

**Rust side** — real unit coverage:

- Job-spec round-trip, including forward-compatibility for unknown fields.
- Triage clustering, ranking, and deduplication over synthetic replay outcomes.
- Decklist → DCGO deck-code conversion, round-tripped.
- Timeout and quarantine state machine.
- Phase 3: the state differ, against fixture pairs.

**C# side has no unit-test path, and the design does not pretend otherwise.** The
`Tests~/` asmdef is disabled because it cannot reference `Assembly-CSharp` —
which is why it was renamed out of the build earlier in this campaign. Phase 1
validates the Unity code with a **golden smoke job**: a fixed spec and seed that
must produce a recording replaying to a known depth. That catches the regressions
that actually bite — capture gaps and encoding drift — and it is only meaningful
because of the determinism requirement above.

Reviving the test asmdef properly is a possible later improvement; it is out of
scope here.

**Closing the loop** — each confirmed divergence gets its minimized recording
promoted to a committed fixture, so the fix stays guarded and the finding feeds
back into `/batch-implement-cards-rust-dsl` and
`/archetype-interaction-test-author`.

## Phasing

| Phase | Delivers | Unblocks |
|---|---|---|
| **1 — Volume** | `JobWatcher`, `ai` policy, deck rotation, breeding fix, `Time.timeScale`, `submit` / `status` / `triage`, golden smoke job | Ends the manual-game bottleneck; parity findings in bulk |
| **2 — Targeted** | `DeckStacker`, `scripted` policy, `probe`, `/dcgo-probe` skill | Ground truth for card authoring and test writing |
| **3 — Bidirectional** | `StateDumper`, step-aligned `diff` | Catches divergences that never produce an illegal action |

**Phase 1 is the unit of the first implementation plan.** It is independently
valuable (it removes the manual bottleneck on its own), and phases 2 and 3 each
depend on findings from running it — notably the determinism and throughput
measurements. Each later phase gets its own plan.

## Skills

- **`/dcgo-corpus`** — runs the volume loop end to end and returns a ranked
  triage report.
- **`/dcgo-probe <card or interaction>`** — stacks the deck, scripts the line, and
  reports how DCGO actually resolves it, in a form consumable by
  `/batch-implement-cards-rust-dsl` and `/archetype-interaction-test-author`.

Phase 3 does not warrant its own skill; it is a step inside triage.

## Operating agreement

On a batch's findings: triage and report. Divergences are ranked, the top ones
investigated, and a diagnosis brought back for a decision on what to fix. Fixes
are not applied autonomously under this design.

## Out of scope

- Reviving the disabled C# test asmdef.
- The live HTTP/WebSocket debug server (deferred behind the job abstraction).
- Headless Unity `-batchmode`.
- Running our trained models as the DCGO bot. This design deliberately does not
  address it, though the `InputDriver` policy seam is where such a policy would
  attach.
