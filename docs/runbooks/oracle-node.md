# Oracle node provisioning

> **Procedural runbook** for standing up a DCGO "oracle node" — a machine that
> answers card-behaviour questions for the QA pipeline by running a prebuilt
> DCGO player against the job queue. For the harness that queues/drains games
> and triages the corpus see [`docs/DCGO_HARNESS.md`](../DCGO_HARNESS.md); for
> the mod that builds the player see [`docs/DCGO_BUILD.md`](../DCGO_BUILD.md);
> for the two platform questions this design rested on see
> [`qa/dcgo-harness/node-platform-findings.md`](../../qa/dcgo-harness/node-platform-findings.md).

## 1. What a node is, and is not

A node **runs** a prebuilt DCGO player. It does not **build** one.

- Running a Unity player requires **no licence** — only building one does.
- A node therefore never clones the Unity project, never opens the Unity
  Editor, and never touches `DCGO/` beyond copying its `Assets/Scripts`
  directory (source text, not a build input).
- The 4.3 GB figure people remember for "DCGO" is the Unity **project**. The
  **artifact** a node needs is a fraction of that — see §2.

A node needs, in total:

1. the built player (run it — no licence needed),
2. DCGO's C# source (`Assets/Scripts` only — source priority #2, for triage:
   see CLAUDE.md's "Source priority for card / keyword / rules questions"),
3. `general_rule.pdf` + `glossary.pdf` (source priority #1),
4. a clone of this repo (for `dcgo-harness` itself and `data/`), separate from
   the payload below.

## 2. Payload

[`scripts/build-oracle-node.sh`](../../scripts/build-oracle-node.sh) assembles
items 1–3 from a build machine (one that has the base repo — DCGO submodule
and the rules PDFs live only there, per rules 29 and 32) into a directory
that gets copied to the node:

```bash
scripts/build-oracle-node.sh <build-dir> [out-dir]
# e.g.
scripts/build-oracle-node.sh /d/dcgo-build/scripted-v9 /d/oracle-node-payload
```

`<build-dir>` is a `dcgo-harness build` output (must contain `manifest.json`).
The script resolves the base repo itself
(`BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"`,
rule 29) rather than taking it as an argument, refuses to run against an
uninitialized worktree DCGO placeholder, and writes a `MANIFEST.txt` recording
which build and which `action_space_hash` the payload was cut from (see §5).

**Real sizes**, measured 2026-08-28 against `/d/dcgo-build/scripted-v9`
(`dcgo_commit 5019e73a3`):

| Artifact | Size |
|---|---|
| `player/` (the built DCGO.exe + `_Data`) | 492 MB |
| `dcgo-src/Assets/Scripts` (C# source, no art, no LFS) | 52 MB |
| `rules/` (`general_rule.pdf` + `glossary.pdf`) | 1008 KB |
| **Total** | **544 MB** |

These match the ~492/53/1 MB figures the design assumed going in — call the
whole payload **~550 MB**. `manual.pdf` (52 MB, UI reference) is deliberately
excluded; it is not part of source priority #1.

Copy `<out-dir>` to the node by whatever means fits your fleet (the harness
itself doesn't care — scp, a shared volume, a VM image bake).

## 3. Provisioning

On the node:

1. **Clone this repo.** The node needs its own working tree for `data/`,
   `code/tools/dcgo-harness`, and the Cargo workspace — it does *not* need the
   DCGO submodule initialized (that stays uninitialized, same as any
   worktree; rule 29) because the player and the C# mirror already arrived in
   the payload.
2. **Drop the payload.** Place the copied `player/`, `dcgo-src/`, and `rules/`
   directories wherever convenient on the node; only `player/`'s path is
   passed to the harness (as `--build`).
3. **Place the rules PDFs where rule 32's resolution expects them.** Rule 32
   resolves the PDFs at `$(dirname "$(git rev-parse --path-format=absolute
   --git-common-dir)")/Digimon TCG resources/`. On a node this is a plain
   clone (not a worktree), so `--git-common-dir` is that clone's own `.git`
   and the resolved directory is `<clone-root>/Digimon TCG resources/`. Copy
   the payload's `rules/general_rule.pdf` and `rules/glossary.pdf` there —
   this directory is git-ignored by design (it never lived in the repo; it
   rides in the image, per rule 32) so the copy has to happen on every node.
4. **Install the Rust toolchain** (same version the repo's CI uses; see
   `rust-toolchain.toml` if present, otherwise stable).
5. **Build the harness**: `cargo build -p dcgo-harness`. This pulls in
   `action_space_export` (an engine-adjacent crate), so it needs the whole
   workspace present — which the clone in step 1 provides. Nothing here
   touches Unity; only Cargo.
6. **Run preflight until GO**:

   ```bash
   cargo run -q -p dcgo-harness -- --root <harness-root> node status --build <player-dir>
   ```

   `node status` exits non-zero on NO-GO, so it is scriptable/CI-gateable —
   wire it into whatever brings a node into rotation. Worked example, run on
   the build/dev machine against `scripted-v9` (2026-08-28):

   ```
   $ cargo run -q -p dcgo-harness -- --root "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" node status --build /d/dcgo-build/scripted-v9

   GO
     [ok] build: D:/dcgo-build/scripted-v9\DCGO.exe (dcgo_commit 5019e73a3a8ac3b71b76929caac63ad79307e260)
     [ok] action_space: matches the engine (5e91c239908b)
     [ok] harness_enabled: C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness\harness.enabled present
     [ok] queue: C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness has jobs/claimed/done/failed
     [warn] player: not running
           -> `dcgo-harness node up --build <dir>` starts it

   exit: 0
   ```

   `player: not running` is a `warn`, not a `fail` — `node status` reports GO
   because nothing here blocks starting the oracle; `node up` is what launches
   it (§4). A `fail` on `harness_enabled` here would mean running
   `dcgo-harness --root <root> enable` first (see `DCGO_HARNESS.md`).

## 4. Running

```bash
# Preflight, then launch. Refuses (no launch) if health() reports NO-GO.
dcgo-harness --root <root> node up --build <player-dir>

# Supervise a batch end-to-end: keeps the oracle running, restarts it on a
# hang, drains the queue. Exits 0 when drained, 1 if the restart budget runs
# out with work remaining. (Top-level `watch`, not `node watch` — it predates
# the `node` subcommand and covers the same lifecycle with restart/poll logic
# `node` doesn't duplicate.)
dcgo-harness --root <root> watch --build <player-dir>

# Stop it.
dcgo-harness --root <root> node down
```

`node up` launches the player with `-batchmode -nographics` (already wired
into `daemon.rs`'s `player_command` — nothing to add here), so **no attached
desktop session, no virtual display, and no `tscon` session redirection are
needed**. See §6 for the measurement this rests on.

## 5. The action-space rule, as an operational fact

A build embeds a frozen snapshot of `ActionSpace.cs` (CLAUDE.md rule 27) and
the manifest stamps its digest as `action_space_hash`. Changing
`code/digimon-engine/src/action/space.rs` on the build machine invalidates
**every node's player at once** — each one keeps encoding actions against the
old space, and every recording it produces would read as engine divergence
rather than as a stale tool.

`node up` (and `node status`) check the running engine's current
`action_space_hash` against the one baked into the player's manifest and
**refuse rather than produce corrupt recordings**. This is a chore, not a
hazard: rebuild the player on the build machine (`dcgo-harness build`),
re-cut the payload (`scripts/build-oracle-node.sh`), redistribute it to every
node, restart.

## 6. Platform requirements

See [`qa/dcgo-harness/node-platform-findings.md`](../../qa/dcgo-harness/node-platform-findings.md)
for the measured answers to the two questions the fleet design depended on:
whether the player runs headless (it does — full games, no attached display),
and how many players can hold a Photon connection to the queue concurrently
(measured clean at 2; the ceiling above that is **not measured** — don't plan
past it without re-measuring). This runbook intentionally does not restate
those numbers so there is one copy of the finding.

## 7. Troubleshooting

Carried across from `docs/DCGO_HARNESS.md` — these are the ones that cost
real time to find and will look exactly as confusing on a node:

- **A disabled harness looks exactly like a hung DCGO.** `HarnessConfig.Enabled`
  defaults to false. If jobs sit in `jobs/` and nothing claims them, check
  `node status` for the `harness_enabled` line before assuming the player
  crashed — `dcgo-harness --root <root> enable` (writes the `harness.enabled`
  marker) is the fix, not a restart.
- **Stop Play/the player before requeueing.** A running harness claims jobs
  the instant they appear; moving files into `jobs/` while it's live feeds
  them to whatever code version is currently loaded. Bring the node down
  (`node down`), requeue, bring it back up.
- **DCGO's AI mode is not offline.** It connects to Photon and creates a
  private one-seat room even for solo/bot games. Without leaving that room
  between jobs, the next job's `Init` waits forever for a lobby it can't join
  while still in one — this is why the harness explicitly leaves the room
  after each job, and why a node needs outbound network access to Photon, not
  just local disk access to the queue.
- **A stale `cards_behavioral` process holds the build lock.** This is an
  engine-repo gotcha, not a DCGO one, but it bites on a node the same way it
  bites in a worktree: a hung test binary from a prior run holds the exe name
  (next build fails `LNK1104`) *and* the `CARGO_TARGET_DIR` build lock, so
  every later `cargo` command on that node looks hung. Before debugging a
  "hung" node, check for and kill stray `cards_behavioral`/`cargo` processes
  first (CLAUDE.md rule 33).

## 8. Cost note

One archetype campaign (2026-08-21 → 08-27) cost **$4,210** in agent tokens
running against a *manually* attended DCGO session. A dedicated long-lived VM
running as an oracle node costs **$30–150/month**. Infrastructure is a
rounding error against token spend; a cold start (no node warm and ready when
an agent needs the oracle) is paid for in tokens, not dollars. Keep nodes
warm — the whole point of the `node status` gate is to make "is this node
usable right now" a cheap question an agent can ask before it starts
authoring, rather than an expensive one it discovers the answer to by failing.

## Provenance

This runbook's provisioning section (§3) was executed, not just written, on
2026-08-28 on the build/dev machine (which serves as both build machine and a
stand-in node here — no second machine was available):

- Steps 4–6 (toolchain present, `cargo build -p dcgo-harness`, `node status`
  until GO) ran for real — see the worked example in §3 step 6, captured from
  this machine.
- Steps 1–3 (clone the repo on a *separate* node, copy the payload over, place
  the PDFs at the resolved path) could not be executed end-to-end for lack of
  a second machine. They were validated in the sense that mattered:
  `scripts/build-oracle-node.sh` was run for real (§2's sizes are its actual
  output, not estimates), and the destination path in step 3 was verified
  against this machine's own `git rev-parse --path-format=absolute
  --git-common-dir` resolution rather than assumed.
