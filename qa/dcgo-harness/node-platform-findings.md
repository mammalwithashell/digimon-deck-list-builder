# Oracle node — platform findings

**Measured:** 2026-08-28 · **Build:** `D:\dcgo-build\scripted-v9`
(`dcgo_commit 5019e73a3a8ac3b71b76929caac63ad79307e260`,
`action_space_hash 5e91c239908b8927e5ce6c8eac83c8ab744468469c88793e0679c0411e2ce079`)
**Machine:** Windows 11 Home 10.0.26200, GPU present, launched from an attached
Console session.

Both questions this document exists to settle were **assumptions the fleet design
rested on and had never been tested**. Both are now measured, not inferred.

## 1. Does the player run headless? **Yes — `-batchmode -nographics` plays a full game.**

| Flags | Process alive at 30–35 s | Reached the harness loop (log) | Drained a job |
|---|---|---|---|
| `-logFile` only | yes | yes — `bootstrap: enabled=True`, `card database ready; claiming jobs` | yes (heartbeat 0 s, gate passed via `dcgo-harness up`) |
| `-logFile -batchmode -nographics` | yes | yes — same two lines | **yes — full game, recording + state sidecar, job reached `done/`** |

**Evidence for the headless run**, which is the load-bearing one:

```
[Harness] bootstrap: enabled=True root=...\dcgo_harness
[Harness] waiting for DCGO to finish loading its card database...
ERROR: Shader UI/Default shader is not supported on this GPU (none of subshaders/fallbacks are suitable)
[Harness] card database ready; claiming jobs.
Game Initialize - random number sequence initialization, GameRandom.Seed:3280220032992229464
```

and the queue/artifact timeline for one submitted job:

```
09:08  vol-00000.json           claimed/
09:09  20260828T140845Z_….jsonl        + .state.jsonl   (recording AND sidecar)
09:10  vol-00000.result.json    done/
```

**The shader ERROR is non-fatal and expected.** `-nographics` gives Unity no
graphics device, so a UI shader cannot compile; the harness path never renders,
so the game still initializes, plays, and records. Do not read that line as a
failure.

**Verdict: a node does NOT need an attached desktop session or a virtual display
driver.** Any headless Windows VM can run the oracle. This removes the risk the
fleet design flagged as the one that could change what a node has to be — the
"Windows VM with Desktop Experience + `tscon` session redirection" contingency is
not needed.

Throughput observed: **~1–2 minutes per game**, artifact-to-artifact.

## 2. Photon concurrency: **2 concurrent players verified clean; ceiling above 2 not measured.**

Two `-batchmode -nographics` players were run simultaneously against the same
build. Both bootstrapped, both reported `card database ready; claiming jobs`, and
both reached `Game Initialize` — which occurs *after* the Photon room handshake
(`JobWatcher.LoadBattleSceneWhenPhotonReady`), so both genuinely held a
connection at once.

4 jobs submitted → **8 new recording files** (4 games × `.jsonl` + `.state.jsonl`)
within ~80 s, with the queue draining to `jobs=0`.

The only Photon lines in either log across the whole run were the expected
inter-job cleanup:

```
[Harness] leaving the previous job's Photon room
```

**No lobby errors, no join failures, no disconnects.**

**Ceiling: not measured.** Only 2 were run, on one machine. Each node holds one
connection and one private one-seat room against the app id baked into the
build, so N nodes = N concurrent CCU. Whether that app id has a CCU cap, and
where, is **unknown** — recorded as unknown rather than guessed, because fleet
sizing would otherwise rest on a fabricated number. Measure again before N > 4.

## 3. What this means for a remote node

- **Any headless Windows VM works.** No GPU requirement beyond what `-nographics`
  tolerates, no desktop session, no RDP session-redirection workaround.
- The launcher should pass `-batchmode -nographics`. `daemon.rs`'s
  `player_command` currently passes only `-logFile`; adding the headless flags is
  safe on a machine with a display (measured above: both modes work) and is
  *required* on one without.
- **One queue per node.** Both players in the concurrency test shared a single
  harness root and both claimed from it. That worked here, but two players racing
  one queue is not the fleet shape — the design gives each node its own root, and
  nothing in this measurement validates queue-sharing.
- The 492 MB player, the 53 MB DCGO C# mirror and the ~1 MB rules PDFs remain the
  whole payload; nothing measured here changes the ~550 MB image.
