# DCGO as an Agent-Drivable Oracle — Design

Extends the phase-1 job harness
(`docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`) so an
agent can drive DCGO with no human in the loop, and ask it targeted questions
while authoring cards.

Operating manual: `docs/DCGO_HARNESS.md`.
Recording format: `docs/DCGO_RECORDING_SCHEMA.md`.

## Problem

Phase 1 removed the manual-game bottleneck but not the human. Someone still
presses Play and someone still presses Stop, so an agent cannot start a session
mid-task. And the volume policy answers "what did DCGO do", never "what does
DCGO do *here*" — which is the question card authoring actually asks.

## Two independent axes

The autonomy work and the capability work do not gate each other. Everything
runs through the same file-drop channel, so a scripted probe behaves identically
in the Editor and in a player build. Only *who presses Play* differs.

This matters for sequencing: the build is the one unknown that can invalidate
decisions already made, and it is cheap to resolve, so it goes first — but a
failed build does not block the probe.

**Sequence:** compile spike (hard gate, timeboxed) → autonomy → probe.

If the player does not build within a day, take the Editor-auto-play fallback
and move on. AssetRipper damage is a rabbit hole with no floor, and the fallback
preserves the entire design.

**Layer A is the unit of the first implementation plan.** It is independently
valuable — it makes the *existing* phase-1 volume corpus unattended on its own —
and Layer B's plan should be written after the spike has resolved whether the
oracle is a player build or a driven Editor. Layers B and C each get their own
plan.

## Layer A — autonomy

### Build

`Assets/Scripts/Script/Harness/Editor/HarnessBuild.cs` exposes a static
`Build()`, invoked headlessly:

```bash
Unity.exe -quit -batchmode -projectPath <base>/DCGO -executeMethod Digimon.Harness.EditorTools.HarnessBuild.Build -logFile -
```

(`EditorTools`, not `Editor` — matching the namespace `HarnessMenu.cs` already
uses. `Editor` is reserved-ish and collides with `UnityEditor.Editor` in
`using`-heavy files.)

Output goes to `D:\dcgo-build\<version>\` — outside the submodule. It is
multi-GB derived data and DCGO is an LFS checkout, so it must not land anywhere
git can see it. Same reasoning and same disk as rule 31's per-worktree cargo
directories.

Each build stamps a `manifest.json`:

```jsonc
{
  "dcgo_commit": "be359bb5b...",
  "built_at": "2026-08-20T14:02:11Z",
  "artifact_sha256": "...",
  "action_space_hash": "..."
}
```

### The acceptance test is not "it builds"

It is: **run the golden smoke job at a fixed seed in the Editor and in the
player, and get byte-identical recordings.**

A player that launches but shuffles differently — different asset load order
feeding `RandomUtility.ShuffledDeckCards`, say — is worse than no player,
because it makes the oracle disagree with itself. Everything downstream assumes
the Editor and the player are the same program.

### The action-space compatibility gate

A build embeds a frozen snapshot of `ActionSpace.cs` (rule 27). The moment
`code/digimon-engine/src/action/space.rs` changes, an older build keeps encoding
against the old space, and every recording it produces is corrupt *in a way that
reads as engine divergence*.

This is the frame-ID-vs-compact-index failure again, except versioned and harder
to notice. So `dcgo-harness up` compares the manifest's `action_space_hash`
against the engine's own and **refuses to launch on mismatch**. A loud startup
error instead of a corpus of plausible lies.

`action_space_hash` is a digest of the action-space descriptor emitted by
`cargo run -p action-space-export` — the same artifact `emit_csharp.py` lowers
into `ActionSpace.cs`. Hashing the descriptor rather than the generated C# means
the check tracks the contract, not incidental codegen formatting.

`.github/workflows/action-space-codegen-drift.yml` already guards the
source-level version of this; the manifest extends the same idea to a binary.

### Lifecycle

Chosen shape: a **warm daemon, auto-started**. Card authoring means several
probes in a row, so Unity's startup cost should be paid once per work session
rather than once per question. Every job-submitting command implicitly calls
`up`, so an agent never reasons about the process.

`HarnessConfig.ExitAfterIdleSeconds` replaces a separate auto-quit-on-drain
flag. A one-shot subprocess sets it low; the daemon sets it high. One code path
serves both.

### Health: a PID is not enough

`up` ensures a process exists *and is working* — a PID file plus a heartbeat file
that `JobWatcher` touches every poll. A PID alone reports a hung Unity as
healthy, and hung Unity is the failure mode already hit twice (the unleft Photon
room; the stalled selection).

Stale heartbeat past threshold: kill the PID, requeue whatever sits in
`claimed/`, relaunch. Restarts are bounded and the preceding job is quarantined
after two, so one poisonous job cannot relaunch Unity forever.

### Fallback

If the player will not build, the daemon launches the *Editor* with
`-executeMethod` calling `EditorApplication.EnterPlaymode()`. Identical interface
to `dcgo-harness`, worse startup, no redesign. This is why committing to the
daemon before knowing the build outcome is safe.

Known risk that cannot be retired by inspection: batchmode needs an activated
Unity license, and a license prompt in batchmode manifests as a silent hang
rather than an error.

### Distribution — deferred, not designed away

The artifact stays local for now. Versioning, hashing, the manifest, and the
compatibility gate are built regardless, because the gate is a correctness
requirement rather than a publishing convenience. Since the manifest is exactly
what would be published, deferring costs no rework.

When it is published, the model pipeline is the precedent to copy:
`code/server/storage/spaces.py` plus a manifest with SHA verification and a local
cache, the way `code/src-tauri/src/models.rs` fetches ONNX models.

A DCGO player build redistributes Bandai card art and DCGO2 code. Upstream DCGO
already publishes builds, so a modified build is not a new category — but public
release versus private artifact is a deliberate posture decision, not a side
effect of picking a host.

## Layer B — the probe

An agent builds the position in **our** engine, exports the line, and replays it
into DCGO. Our engine validates every step against its own mask before Unity
launches, so a malformed line fails in milliseconds instead of sixty seconds.
The two engines then run the same line, and the diff is the finding.

Rejected: a "materialize board" cheat that constructs permanents directly. It
would reach any position in one step, but a hand-built board can miss internal
wiring the normal play path sets up — so a divergence might be the cheat's fault
rather than a real parity bug. An oracle must never do that. DCGO reaches the
position by legally playing to it.

(`CheatAction` is also thinner than the phase-1 design assumed: `Draw`,
`TrashCard`, `PlaceCardOnDeck` and friends take no card parameter — they open the
normal selection UI. Usefully, `AllowCheats()` returns true whenever `isAI`,
which is exactly harness mode.)

### DeckStacker, and the trap in it

`RandomUtility.ShuffledDeckCards` is one static helper behind all ~14 shuffle
sites plus mid-game `Shuffle(Player)`, so hooking the helper covers everything.
That is also the trap: **search and shuffle effects route through it too.**

If a job's stack applied to every shuffle, a card reading "shuffle your deck"
would silently re-impose the opening order, and the probe would confidently
answer a question about a game that cannot occur.

So the stack applies to the **initial shuffle only**. Mid-game shuffles fall
through to seeded `GameRandom` — still fully deterministic, but honest.

### InputDriver: assert the prompt, do not just answer it

Actions are fed through the chokepoints already hooked for reading:
`TurnStateMachine.QueueMainPhaseAction` for main-phase decisions,
`UserSelectionManager.SetIntForPlayer` / `SetBoolForPlayer` for selections.

The important property is not feeding actions but refusing to feed them blind. A
driver that pops the next action whenever DCGO asks anything will, on a single
ordering mismatch, desynchronize the entire remainder of the line while every
step still looks successful.

So each scripted step carries the prompt it expects — decision kind plus context
— and the driver asserts before answering. A mismatch aborts the job and reports
itself.

**That is a finding, not an error.** "Our engine expected a choice here and DCGO
never asked" is exactly the divergence class that never surfaces as an illegal
action — the class phase 3 existed to catch. Honest bookkeeping yields it for
free.

### StateDumper

At each decision boundary, a sidecar JSONL keyed by step index so it aligns with
the recording: per permanent (card ID, effective DP, digivolution stack,
suspended, active keywords), plus hand, trash, security count, memory, phase,
and turn.

State dumping is not optional here. Without it a probe reports what was *legal*,
never what *happened* — and what happened is the question card authoring asks.

### Normalization

The two engines order zones differently and track modifiers differently, so the
diff runs over a normalized projection: multisets where order carries no
meaning, effective DP rather than modifier lists.

Under-normalize and the report drowns in noise nobody reads. Over-normalize and
it hides the bug. The rule: **normalize representation, never semantics.**
Effective DP is representation. Whether a Digimon is suspended is semantics.

## Layer C — output and surface

### Differ

Walks aligned step indices over normalized projections, and leads with the
**first** divergence, marking everything after it as downstream. Once the two
engines part they are playing different games; a report ranking fifty
consequences beside one cause is a report nobody finishes.

### Test drafter

Emits a draft `#[test]` into `code/digimon-engine/tests/cards_behavioral/<set>/`
carrying DCGO's observed outcome.

It **never auto-commits**, and its header reads
`DCGO build <hash>, job <id>, recording <path> observed:` rather than asserting
correctness. DCGO is source-priority #2, below `general_rule.pdf` — a drafted
test encodes strong evidence, not truth.

Provenance is the point, especially once builds are versioned and an old one
might be pinned. An auto-generated test asserting a behavior nobody read is
worse than no test; under the no-approximations policy it would launder a DCGO
quirk into a permanent guard.

### Agent surface

`dcgo-harness mcp` as a subcommand of the same binary, not a new crate, so every
behavior stays unit-testable without an MCP client — matching how `dcgo-replay`
and `digimon-engine-mcp` share a core. Tools: `probe`, `corpus`, `status`.
Skills `/dcgo-probe` and `/dcgo-corpus` sit on top.

Per CLAUDE.md's MCP boundaries this is write-capable dev/test tooling, parallel
to `digimon-scenario-mcp`: local and dev only, never bundled into a production
build, never imported by `server.*` or `digimon_gym.*`.

## Failure taxonomy

Each failure gets a distinct, non-silent outcome:

| Failure | Response |
|---|---|
| Action-space hash mismatch | Refuse at `up`. Never run. |
| Line illegal in our engine | Fail before Unity launches, in milliseconds |
| Prompt mismatch mid-line | Abort job, report **as a finding** |
| Unity hung | Stale heartbeat → kill, requeue, bounded restart, quarantine after two |
| Job timeout | Requeue, then quarantine |

Plus the standing rule phase 1 earned: always print the full denominator. A
batch where most jobs died must never read as a pass.

## Testing

The Rust side — job specs, normalization, differ, drafter — is unit-testable with
no Unity at all. That is the main reason the MCP is a subcommand rather than a
separate crate.

A committed golden recording plus state-dump fixture lets the differ run in CI,
where no Unity exists. The Editor-vs-player determinism check covers the build.

Net effect: **the only thing that needs Unity to test is Unity.**

## Operating agreement

Unchanged: triage and report, ask before fixing. Findings are ranked and
diagnosed; fixes stay a decision, not an automation.

## Out of scope

- Publishing the build (deferred, with the manifest already in the shape it needs).
- A "materialize board" cheat (rejected above, on oracle-trust grounds).
- Live HTTP/WebSocket debug server; headless Unity `-batchmode` *play* (as before).
- Running our trained models as the DCGO bot. The `InputDriver` policy seam is
  where such a policy would attach.
