# RESOLVED FINDING: the scripted main-phase hook never fired under the harness

**Status:** FIXED 2026-08-22 (DCGO 16f7aeda0). Kept as the record of the
misplacement and how it was proven, since the wrong inference that produced it
is an easy one to repeat.
**Found:** 2026-08-21, while wiring the phase-2 `InputDriver`
**Severity:** blocks the exam's primary use case; does NOT affect the phase-1
volume corpus or the `policy: "ai"` determinism guarantee

## What is wrong

`InputDriver`'s main-phase interception is installed at
`TurnStateMachine.QueueMainPhaseAction` (`TurnStateMachine.cs:3430`), which the
recorder also hooks with `LogAction`. That looked like the right chokepoint
because it is where recorded main-phase decisions come from.

It is not, **under the harness**. `QueueMainPhaseAction` is the *human-UI* path.
`MainPhase()`'s wait loop only dequeues main-phase actions under
`!IsAI || TurnPlayer.isYou` (~`TurnStateMachine.cs:1266`). A harness game is
`IsAI && isAuto`, so both seats take the AI brain (~line 1291), which **never
queues an action**: it sets `PlayCard` / `TargetFrameID` / `AttackingPermanent` /
`DefendingPermanent` / `UseCardEffect` directly and the shared processing
consumes those fields.

So a scripted line cannot drive a main-phase decision at all — not a play, not a
digivolve, not an attack, not even a pass.

## Why the recorder is unaffected

The recorder reads `LogAction` from the same method, and the phase-1 corpus is
full of main-phase rows. That is because the recorder ALSO mirrors the AI
brain's chosen action in a second block (~line 1526), which the recorder's own
comment documents. Reading was solved; driving was not.

This is worth stating because "the recorder hooks it, so the driver can too" is
exactly the inference that produced the bug, and it is a reasonable-looking
inference.

## What still works

Verified by running the golden determinism job:

- `DeckStacker` — a `deck_order` prefix lands on top of the initial draw.
  Confirmed: `deck_order.p0 = [ST1-08, ST1-06, ST1-03]` produced
  `initial_state.my.initial_hand = [ST1-08, ST1-06, ST1-03, ST1-13, ST1-15]`,
  while the unstacked opponent hand stayed random.
- `StateDumper` — 48 sidecar rows, every step index present in the recording.
- Determinism at a fixed seed — see the run log below.
- The mulligan and breeding-action hooks fire (they are not on the AI-brain
  path), though the mulligan is now pinned to auto-keep for symmetry with our
  engine (see below).

## The fix

Move the main-phase interception off `QueueMainPhaseAction` and onto the AI
brain at ~`TurnStateMachine.cs:1291`, where the decision is actually made —
i.e. set the same `PlayCard` / `TargetFrameID` / `AttackingPermanent` /
`DefendingPermanent` / `UseCardEffect` fields the brain would have set, from the
scripted action id, instead of letting the brain choose.

`InputDriver.BuildMainPhaseAction` (the partial inverse of `ActionEncoder`
covering the six queueable families) is still the right decoder; only its call
site is wrong.

Keep the `QueueMainPhaseAction` hook as well: it is correct for a
human-in-the-loop game, and a future non-`isAuto` harness mode would use it.

## Related: the mulligan is pinned, not measured

Our `ScenarioAdapter` has no mulligan verb, so it auto-resolves both mulligans
(keep) and its line begins at turn 1. The DCGO side originally consumed a
scripted step for the mulligan, which would have put DCGO one step ahead of our
engine from the very first decision and aborted **every** scripted job on a
prompt mismatch — a desync caused entirely by the harness rather than by the
card under test.

Both sides are now pinned to "keep". They therefore agree, but **neither is
measuring the mulligan**. Giving the scenario format a mulligan verb on both
sides is the real fix; until then this is a hole in the exam's coverage, not a
verified agreement.

## Run log — golden determinism job, 2026-08-21

Job `qa/dcgo-harness/golden-determinism-job.json`, seed `424242`, ST-1 Gaia Red
both seats, player build `D:/dcgo-build/scripted-v2`
(`dcgo_commit a2eb37e10`, `artifact_sha256 8efdfa80…`).

Two runs of the same job, same seed, same binary:

| Artifact | Result |
|---|---|
| State sidecar | **byte-identical** — 0 of 48 lines differ, SHA-256 `d1aa629ec203cf1866db722550896701` both runs |
| Recording | 2 of 79 lines differ, all on line 1. Excluding `game_id` (a fresh GUID per game) and `timestamp`, **all 79 rows identical** |
| Deck order post-shuffle | identical for both seats across runs |

Run 1 completed in 48.7s: 46 `action`, 22 `action_detail`, 6
`effect_activation`, 2 `selection`, 1 `game_end`.

**Not yet checked: Editor vs player.** The design's stated acceptance test is
byte-identical recordings from the *Editor* and the *player* at one seed. Only
player-vs-player is verified here. The Editor half needs headless batchmode
Play, which the phase-2 design deliberately scoped out.

## Also confirmed, still open

`job.first_player` is written by `submit` but remains **unhonored by DCGO**. The
job requested `first_player: 0`; the recording reports `first_player_id: 1`.
Previously a documented suspicion, now observed directly.


---

## Resolution (2026-08-22)

The seam moved onto the AI brain, decoding a scripted step with
`InputDriver.BuildMainPhaseAction` and applying it via
`MainPhaseAction.Execute(this)` -- reusing DCGO's own `SetPlayCard` /
`SetAttackingPermaent` / `SetActSkill` translation instead of duplicating it.

Verified end to end: a 60-step scripted line, **every step carrying an
`expect_prompt` assertion**, ran the full 25-turn cap with **zero mismatches**.
52 action rows = 2 mulligan (auto-kept, no step consumed) + 25 `breeding` + 25
`main_phase`, ids `{0, 62}`.

The bot chose nothing, which is the point: the same deck under `policy: "ai"`
yields 46 actions with 22 `action_detail` plays and 6 `effect_activation`s; this
scripted run has **zero** of either.

Scripted determinism, two runs at seed `424242`:

| Artifact | Result |
|---|---|
| State sidecar | **Byte-identical**, SHA-256 `8846f6dbe4a384fa59146096fa0e7080` |
| Recording | 54/54 rows identical; only line 1 differs (`game_id`, `timestamp`) |

### Still open

- **`job.first_player` remains unhonored.** The scripted line's actor sequence
  had to be authored against DCGO's own roll (seat 1 first). A line authored
  against our engine's assumption would fail its actor assertion immediately.
  This is now the largest remaining obstacle to authoring scenarios from our
  engine's lowering output.
- **The mulligan is pinned to keep on both sides, not measured.**
- **Editor vs player** determinism is still unverified; only player-vs-player is.
