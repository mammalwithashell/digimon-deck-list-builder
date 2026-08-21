# DCGO Job Harness

Generates a DCGO recording corpus without anyone playing games by hand, then
triages it against our engine.

Design: `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`.
Recording format: `docs/DCGO_RECORDING_SCHEMA.md`.

This is **phase 1 (volume)** only: DCGO's own AI plays both seats. Scripted and
recorded input policies are later phases.

## Quick start

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"

# Queue N games from a deck pool
dcgo-harness --root "$ROOT" submit --count 200 --decks pool.json --seed 1

# Turn the harness on, then press Play in Unity
dcgo-harness --root "$ROOT" enable

# Watch it drain
dcgo-harness --root "$ROOT" status

# When the batch is done: turn it off, then triage the corpus
dcgo-harness --root "$ROOT" disable
dcgo-harness --root "$ROOT" triage \
    --corpus "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings" \
    --cards-json data/cards.json
```

## Enabling it

`HarnessConfig.Enabled` is **false by default** so a stale job file can never
hijack a normal play session. Two ways to turn it on, checked in this order:

1. **A marker file** — `<root>/harness.enabled`, written by `dcgo-harness
   enable` and removed by `disable`. This is the reliable path: it needs no
   Editor code and works in a player build.
2. **EditorPrefs** — the `Digimon/Harness/Enabled` menu toggle, read directly in
   the property getter.

The getter deliberately does not cache into a static field. Entering Play mode
triggers a domain reload that wipes static state, so a value set before Play is
gone by the time the `[RuntimeInitializeOnLoadMethod]` bootstrap reads it — the
harness would silently do nothing.

`status` prints the enable state, and says so explicitly when jobs are queued
against a disabled harness. That state is otherwise indistinguishable from a
hung DCGO.

## Directories

`<root>/{jobs,claimed,done,failed}`. A job file moves between them by atomic
rename, and **that rename is the claim** — it is what makes crash recovery
legible. A job sitting in `claimed/` past its timeout is a hung run, not a lost
one.

## Job and result

```jsonc
{
  "job_id": "vol-00042",
  "policy": "ai",
  "decks": { "p0": ["EX12-035", ...], "p1": ["BT16-082", ...] },
  "first_player": 0,
  "seed": 424242,
  "limits": { "max_turns": 40, "timeout_seconds": 180 }
}
```

Jobs carry **card ID strings, never DCGO deck codes**. The deck code is a base-n
encoding over DCGO's internal `CEntity_Base.CardIndex`; mirroring it host-side
would duplicate a table that rots whenever DCGO re-indexes. `DeckBuilder`
resolves IDs against `ContinuousController.SortedCardList` and builds the code
with DCGO's own `GetDeckCode`, so the encoding stays owned by the codebase that
defines it. Digitama cards are split out by `CardKind.DigiEgg`, so a job ships
one flat array per seat.

```jsonc
{
  "job_id": "vol-00042",
  "outcome": "completed",          // completed | partial | failed
  "recording_path": "...jsonl",
  "steps": 0,
  "duration_seconds": 41.7,
  "message": ""
}
```

## Measured behavior

**Determinism — verified, and load-bearing.** Two jobs with seed `424242` and
identical decks produced byte-identical main and egg shuffles, the same first
player, and all 27 decisions identical, ending on the same winner. Phases 2 and
3 of the design rest on this.

Two real bugs had to be fixed to get there, and neither was findable without
running it:

- The job seed was applied in `JobWatcher.ApplyJob`, then **overwritten** by
  `TurnStateMachine.Init`'s `SetRandom` RPC handshake, which fires for AI games
  too — AI mode creates a one-seat Photon room, so the client is master. The
  authoritative seeding point is that handshake.
- Every AI selection (`SelectCardEffect`, `SelectHandEffect`,
  `SelectPermanentEffect`) drew from `IEnumerableExtension.GetRandom`, which
  used `new System.Random()` — seeded from wall-clock time, entirely outside
  `GameRandom`. Same-seed games shuffled identically and then diverged at the
  first bot choice.

**Throughput — ~40 s per game** at `HarnessConfig.TimeScale = 8`, consistent
across six games (~85 games/hour). Raising the timescale further risks starving
coroutines that yield per frame; re-measure if you change it.

## Rules that matter

- A job overdue past `timeout_seconds` is requeued, and quarantined after two
  failures. Never retried indefinitely — one poisonous job would otherwise stall
  the whole batch silently.
- `status` and `triage` always print the full denominator
  (pending/claimed/completed/partial/failed, and files
  seen/read-failed/parse-failed/replayed). A batch where most jobs died must
  never read as a pass.
- `triage` refuses a clean verdict without at least one fully replayed game, and
  discloses partial and skipped counts inline. "Didn't crash" is not "verified".
- **The corpus is derived data and is NOT committed.** Only triage reports and
  minimized regression fixtures are.

## Gotchas found the hard way

- **Unity must have imported new scripts.** No `.meta` file next to a `.cs` file
  means Unity has not seen it, and a "clean" Console is reporting the *previous*
  compile. Click into the editor to force a rescan.
- **Stop Play before requeueing.** A running harness claims jobs the instant
  they appear, so moving files into `jobs/` while Play is active feeds them to
  whatever code version is currently loaded.
- **DCGO's AI mode is not offline.** It connects to Photon and creates a private
  one-seat room. The harness leaves that room before loading the next job;
  without it, `Init` waits forever for a lobby it cannot join while in a room —
  the second game of every batch hangs on "Now Loading" with no error at all.
- **The result screen is skipped under the harness.** In an AssetRipper-derived
  checkout `resultObject.transform.GetChild(3)` throws, and that abort came
  before `EndGame`'s `StopAllCoroutines` calls, leaking the finished game's
  coroutines into the next scene load.
- **`DeckData.IsValidDeckCode` is not a usable gate.** It requires `Split(',')`
  to yield 5 or 6 fields, but `GetDeckCode` emits 7. It has no other caller in
  DCGO, so upstream never noticed. Use `IsValidDeckData()` — the real legality
  check (50 main, ≤5 egg, per-card legality) that DCGO gates battles on.

## Count payloads carry a value, not an index

`SelectCountEffect` answers "how many?" / "which cost?" with the chosen NUMBER —
its buttons are labelled with the values themselves. Our engine models the same
question as an indexed branch list, so the two must be reconciled.

The resolver matches the recorded value against the engine's branch **labels**,
which embed it (`"Digivolve for cost 2"`). That compares what each side thinks
an option MEANS rather than where it sits, and it works on recordings made
before the recorder emitted `candidates`. When labels do not carry the value,
the recorded `candidates` array supplies the index; without either, the payload
fails loudly.

This was the top cluster in the first clean corpus. The resolver had folded
count in with `int_value` (`int_value.or(count)`) and indexed branches with a
raw quantity, so `count: 2` meant "pick branch 2".

The error was the harmless half. With costs `{0, 1}` the value equals the index,
so positional matching looked correct in every early test; with costs `{2, 3}`
the same code silently picked the wrong branch and replayed clean. Fixing it did
not reduce the failure count — it moved five games *past* a bogus stop and into
their real divergence, growing the genuine clusters from 3+1 to 6+2.

## Known gaps

- `job.first_player` is written by `submit` but not yet honored by DCGO, so seat
  assignment comes from DCGO's own roll and a corpus is seat-biased in a
  dimension the job spec claims to control.
- `JobResultWriter` reports `steps: 0`; the real count is in the recording.
- `triage` passes `card_at_slot: None`, so clusters key on action range only and
  the ranked output is coarser than the signature allows. Deriving it from the
  divergence's board snapshot is the obvious next refinement.
- `job.first_player` is still not honored by DCGO (see above).
