# DCGO Oracle work in Linux cloud sessions

Tested 2026-11-14 on Debian 13 x86_64, against repository base
`8b25ab68ed4872f862449c70c7b862eb57d482d8`. The Windows standalone client **runs
under Wine with this repository's MelonLoader recorder**: it loads 29 hooks,
runs bot matches and two scripted PvP seats, and records actual game snapshots
and choices. The two-seat fixture reached turn 6 and exposed a playback
divergence; the cloud setup works, but that fixture is not a complete Oracle pass.
No Steam installation, physical GPU, or Unity editor is needed for this route.

## Choose the right integration

| Route | Player required | Interface | Linux status in this session |
| --- | --- | --- | --- |
| MelonLoader recorder | Official IL2CPP standalone ZIP + `DcgoRecorder.dll` | `auto.txt`, `agent.txt`, `playback.txt`; raw JSONL → converter → `dcgo-replay` | Bot run verified; two scripted PvP seats reached turn 6 and stopped at a recorded divergence |
| Compiled Oracle harness | Source-built instrumented Mono player **and its matching `manifest.json`** | `dcgo-harness exam`, job queue, heartbeat, state sidecars | Native Rust harness built; scenarios lowered; compatible player payload not available |

The public standalone executable does **not** consume the older harness job queue.
Installing MelonLoader does not change that. Do not pass an emitted exam job to
the recorder, fake a manifest, or count a simulator-only pass as an Oracle verdict.
The source-built player supports `-batchmode -nographics`; the tested MelonLoader
route uses a real virtual display. See [Oracle node](oracle-node.md),
[harness](../DCGO_HARNESS.md), and [MelonLoader](../DCGO_MELONLOADER.md).

## Runtime and artifacts

The tested stack is Wine 10.0 (`10.0~repack-6`), DXVK 2.3.1, Mesa Lavapipe
25.0.7, Xorg dummy at 1280×720/~60 Hz, MelonLoader 0.7.3, Windows .NET runtime
6.0.36, and native Linux .NET SDK 8.0.428. MelonLoader reports DCGO **1.19.3**,
Unity **2021.3.45f2**.

Download the [official standalone archive](https://www.dcgo.online/Builds/Standalone/DCGO_Standalone.zip).
Keep `DCGO_Standalone/Assets` next to `DCGO_Standalone/Game`; do not copy just the EXE.
The tested archive was 189082325 bytes, SHA256:

```text
5afbac5d8f45c078180419246a3924536264547588c1fb5e9ca47839f7e67adf
```

The URL changes with releases. Archive and hash each tested version. Interop DLLs
must be generated from that same client. Keep runtime packages, the Wine prefix,
downloaded binaries and Rust `target/` out of source control. Allow roughly 10 GB
including SDKs and build output; the client/runtime alone occupied about 3 GB.

## Bootstrap a cloud workspace

All commands below run from the repository root. The scripts default to
`DCGO_ROOT=/workspace/dcgo-cloud`, `WINEPREFIX=$DCGO_ROOT/prefix3`, `DISPLAY=:100`.
Set these explicitly to create another isolated instance. This recipe requires
Debian 13 amd64 with configured apt sources and the base native dependencies.

```bash
export DCGO_ROOT=/workspace/dcgo-cloud
bash scripts/dcgo-cloud/bootstrap-debian13.sh
bash scripts/dcgo-cloud/setup-mod.sh
python3 scripts/dcgo-cloud/seed-deck.py
```

The bootstrap extracts Debian packages into the workspace; it does not need
root or modify system package state. The base image must supply Xorg,
`xdpyinfo`, ImageMagick `import`, Python 3, curl, dpkg-deb, the Vulkan loader and
Wine/Mesa dependencies. Check missing dependencies with `ldd`. Use the cloud's
supported package helper on images that lack them. These Debian packages are
not portable to arbitrary older Ubuntu or ARM images.

The assembled bootstrap is syntax-checked; its individual setup operations were
executed successfully here, but a fresh-machine run of the whole script has not
been tested. Re-running `setup-mod.sh` on this prepared workspace was verified.
The deck generator was verified byte-for-byte against the saved deck used live.

Three Wine relocation fixes are included: use `wineserver64` instead of Debian's
system-path wrapper, supply the relocated NLS-data symlink, and preseed MinGW's
native `zlib1.dll` in the prefix. Without the last, Wine emits misleading
user32/kernel32 errors.

Start the display in a long-lived terminal:

```bash
bash scripts/dcgo-cloud/display.sh
```

In another terminal, launch once **before building the mod**:

```bash
bash scripts/dcgo-cloud/run.sh
```

Wait for the title screen and
`$DCGO_ROOT/client/DCGO_Standalone/Game/MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll`.
First boot downloads the generator and Unity dependencies and can take minutes.
Check `Game/MelonLoader/Latest.log` for completed generation. Then stop this
prefix, build using a Linux .NET SDK, and restart:

```bash
bash scripts/dcgo-cloud/stop.sh
# If dotnet is already in PATH, omit DCGO_DOTNET.
DCGO_DOTNET=/workspace/dotnet/dotnet \
  DOTNET_CLI_HOME=/workspace/dotnet-home bash scripts/dcgo-cloud/build-mod.sh
```

The build deploys `Game/Mods/DcgoRecorder.dll`. The tested build had 38 dependency
version warnings and zero errors, and the resulting DLL loaded successfully.
Wine needs **Windows** .NET in `C:\dotnet`; the compiler needs a **Linux** SDK.
`run.sh` sets the former, and `build-mod.sh` clears that setting for the latter.
If needed, install a local SDK using Microsoft's `dotnet-install.sh --version
8.0.428 --install-dir /workspace/dotnet --no-path`.

## Run and record the bot smoke test

The clean ZIP has no usable saved deck. `seed-deck.py` writes a legal ST1 starter
deck (50 main + 4 eggs) to `Assets/Decks/Cloud_ST1.txt`. It preserves an existing
file. The mod selects the first saved deck; isolate other saved decks when the
identity matters. Random Deck alone did not start the earlier manual attempt.

```bash
GAME="$DCGO_ROOT/client/DCGO_Standalone/Game"
mkdir -p "$GAME/UserData/DcgoRecorder"
printf 'bot\n' > "$GAME/UserData/DcgoRecorder/auto.txt"
bash scripts/dcgo-cloud/run.sh
```

Watch `Game/MelonLoader/Latest.log` for `probes applied: 29`, `scene loaded:
BattleScene`, and `auto: match is running`. Automation chooses first player,
keeps the local hand, passes phases, and eventually surrenders. The opponent is
DCGO's built-in bot. Its own UI warns that the bot is deprecated and may make
illegal plays or softlock: use this for runtime/recorder smoke testing, and use
controlled seats for deterministic effect exams.

Without `auto.txt`, the mod only records. `play.flag` leaves control to a human.
Existing `agent.txt` and `playback.txt` enable different flows; use a clean
`UserData/DcgoRecorder` for this smoke test. Do not enable the diagnostic
`wide.flag`: broad bot hooks have a documented stack-overflow problem.

```bash
bash scripts/dcgo-cloud/capture.sh "$DCGO_ROOT/evidence/match.png"
# After completion, archive the raw file before another launch:
python3 code/dcgo-melon-recorder/convert_raw.py \
  "$GAME/UserData/DcgoRecorder/raw-<session>.jsonl" \
  "$DCGO_ROOT/evidence/cloud-bot.jsonl" --cards data/cards.json
target/debug/dcgo-replay --input "$DCGO_ROOT/evidence/cloud-bot.jsonl" \
  --cards-json data/cards.json --output "$DCGO_ROOT/evidence/cloud-bot-parity.json"
```

Do not add `--allow-truncated` to conceal missing decisions. If needed for
diagnosis, label its output partial and keep the converter's truncation reason.
The converter infers some bot decisions; preserve the `inferred` metadata and
report skipped actions. A replay pass measures the recording it actually read,
not every card in either deck. Save raw log, converted log, converter output,
replay report, client/mod hashes and screenshots together.

The first live cloud recording converted without truncation (5 decisions,
0 inferred) and replayed **1 pass, 0 partial, 0 fail, 0 skipped**. It ended by
surrender during the bot's transient breeding selection. That exposed an
autorun bug: `IsSelecting` alone cannot distinguish a bot choice from a local
prompt. The smoke loop now waits through selections during the bot's turn.
This first recording verifies the recording/replay plumbing, not ST1-15's effect.
Evidence is under `qa/dcgo-mod-recordings/cloud/2026-11-14/`.

The corrected autorun completed through **turn 6**, including Dracomon's
digivolution, Gaia Force, and Phoenixmon. It exposed a second regression: the
converter required queued play packets even for the built-in bot, which bypasses
that queue. The converter now retains those paid bot plays while keeping the
human/PvP queue guard and ignoring free effect plays. Three focused regression
tests pass. Conversion gives **16 decisions and 4 inferred choices**; replay
gives **1 pass, 0 partial, 0 fail, 2 skipped redundant inferred passes**.

This is action-replay evidence, not complete board-state parity. The bot's move
and attack during turn 4 were visible on screen but absent from the hooked action
stream. The converted sequence can therefore replay legally without reproducing
every live transition. Keep the raw snapshots and use controlled seats plus
state comparisons for effect verdicts; do not use this bot trace as training
ground truth or label all its cards Oracle-confirmed.

## Build and exercise the Rust scenario tools

```bash
cargo build --locked -p dcgo-harness -p dcgo-replay -p digimon-engine-cli
```

Here the default ONNX Runtime CDN failed. The working alternative was Microsoft's
official `onnxruntime-linux-x64-1.24.2.tgz` from the `v1.24.2` GitHub release:

```bash
export ORT_LIB_LOCATION=/workspace/onnxruntime-linux-x64-1.24.2/lib
export ORT_PREFER_DYNAMIC_LINK=1
export LD_LIBRARY_PATH="$ORT_LIB_LOCATION${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export RUST_MIN_STACK=268435456
# For limited RAM/debug-disk budgets; the tested build used these settings:
export CARGO_BUILD_JOBS=4 CARGO_PROFILE_DEV_OPT_LEVEL=0 CARGO_PROFILE_DEV_DEBUG=0
cargo build --locked -p dcgo-harness -p dcgo-replay -p digimon-engine-cli
```

Run the new two-target exam:

```bash
target/debug/dcgo-harness --root "$DCGO_ROOT/oracle-evidence/queue" exam \
  --scenario qa/dcgo-exams/ST1/ST1-15-effect0-two-targets.yaml \
  --sim-only --cards-json data/cards.json --emit-job "$DCGO_ROOT/oracle-evidence/jobs"
```

It runs 15 legal steps from game start and checks six expectations: both targets
exist (Biyomon at 3000 DP and Dracomon at the exact 4000-DP ceiling), both leave
the opponent's field, their cards/sources enter trash, the Option enters trash,
memory becomes -3, and the user's own Biyomon survives. Assertions are snapshots
**before** the indexed step: index 14 is after selection at index 13 resolves.
These are authored expectations, not DCGO-observed backfill.

Omit `--decks` for these starter scenarios: the harness has its default starter
book. Passing `data/starter_decks.json` directly as a custom book fails because
that option expects a different wrapper schema (`decks`).

The original `ST1-15-effect0.yaml` also lowers/runs (14 steps), but has no
assertions, so the harness correctly says it checked nothing. The existing
`qa/dcgo-mod-recordings/mod_game3.jsonl` replays with 1 pass, 0 partial, 0 fail,
and 1 skipped redundant inferred action. That is historical recording replay,
not a new live Oracle result.

Current ST1-15 clause denominator: **2 total; confirmed 0, diverged 0,
unreachable 0, unavailable 0, unmeasured 2**. Both the Main and Security clauses
remain unmeasured under current semantic binding; legacy verdicts were
invalidated. This session does not overwrite those verdicts.

## Getting deterministic live effect evidence

For the **compiled harness**, obtain the matching instrumented player payload
and manifest from the existing Oracle build machine, retain its hashes, and
follow [oracle-node.md](oracle-node.md). The node daemon currently uses Windows
process management and launches the EXE directly; it needs a Wine-aware launcher
on Linux. Native Rust `exam --sim-only` working does not establish that daemon
portability. A successful live run needs a fresh heartbeat, consumed jobs,
recording/state sidecars, and an actual diff before promoting a clause.

For the **public client + MelonLoader**, the repository already supports both
seats scripted with native recordings. Follow the procedure below. A native
`sim-record` game supplies the initial ordering and both seats' decisions;
`make_playback_plan.py` supplies the mod's shared shuffle plan. This is also the
route to exercise newly authored effects against the public client.

Do not treat the YAML exam job as a native `GameRecorder` recording: their
formats and control surfaces differ. A future bridge needs to preserve both
seats' legal choices, initial ordering, prompt semantics and recording provenance.

## Run both seats scripted

The live Glowing Dawn vs DNA Omnimon run produced **31 actions and 20 selections
per client, with 0 inferred choices**. Both replay reports are **partial passes**
(0 complete passes, 0 failures, 0 skipped); the match has no recorded winner.
The first 50 decisions match the Windows baseline. The next trigger-order choice
selects `BT25-049` where that baseline declines, leading to a cost-reduction
prompt mismatch after `ST23-15`. A separate persistent trash discrepancy concerns
two `ST23-03` copies with different stacks. See the
[evidence and exact findings](../../qa/dcgo-mod-recordings/cloud/2026-11-14-scripted-pvp/README.md).
Resolve these runner/recording issues before claiming full state parity.

Build the mod and native `digimon-engine-cli` as above, and stop the base Wine
prefix before copying it. Choose a **new** run directory each time; the helper
refuses to overwrite previous evidence. This copies two clients and prefixes
(roughly another 3.5 GB), creates the shared plan, seeds one deck per seat, and
writes each client's `auto.txt`, `agent.txt`, `playback.txt`, and `photon.txt`.

```bash
python3 scripts/dcgo-cloud/prepare-playback.py \
  qa/dcgo-mod-recordings/playback/sim_gd_dna.json \
  /workspace/dcgo-cloud/pvp-next --wss
```

The fixture is Glowing Dawn versus DNA Omnimon: 56 recorded decisions including
two mulligans, with seat 1 going first. Each deck has 50 main cards and 4 eggs.
DCGO requires saved deck filenames containing `name_ID`; `Script.txt` is silently
ignored. The helper writes `Script_0.txt` and `Script_1.txt`.

Start two long-lived agent terminals, once with `SEAT=0` and once with `SEAT=1`:

```bash
RUN=/workspace/dcgo-cloud/pvp-next
SEAT=0 # use 1 in the second agent terminal
export LD_LIBRARY_PATH=/workspace/onnxruntime-linux-x64-1.24.2/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
export RUST_MIN_STACK=268435456
target/debug/digimon-engine-cli --pool all --cards-json data/cards.json \
  dcgo-agent --dir "$RUN/agent$SEAT" \
  --model code/digimon-engine/tests/fixtures/mlp_tiny_value.onnx \
  --converter code/dcgo-melon-recorder/convert_raw.py --python python3 \
  --script qa/dcgo-mod-recordings/playback/sim_gd_dna.json \
  > "$RUN/evidence/agent$SEAT.log" 2>&1
```

The CLI currently requires a model even in script mode. This checked-in tiny
fixture satisfies startup; scripted decisions come from the recording, not that
model. Both agents use the same recording and separate request directories.
The Linux agent translates Wine `Z:\workspace\...` raw paths before conversion
**and** board-zone comparison. Native paths remain unchanged; other Wine drive
letters are not guessed.

Keep the Xorg display running. Start one client terminal per role:

```bash
RUN=/workspace/dcgo-cloud/pvp-next
ROLE=host # use join in the second client terminal
DCGO_ROOT="$RUN/$ROLE" DISPLAY=:100 DCGO_PHOTON_MANAGED_SOCKET=1 \
  bash scripts/dcgo-cloud/run.sh
```

Use the display that actually passed `xdpyinfo` (`:102` for the fresh two-seat
run here). Both clients can share that display while retaining separate prefixes.
The host publishes `shared/room.txt`; the joiner consumes it. Automation selects
the saved decks, readies both players, and starts the battle. Do not reuse a stale
room file from a previous launch.

### Photon networking in proxy-based cloud sessions

The standalone defaults to Photon UDP, which failed here. Its bundled IL2CPP
WebSocket socket also failed under Wine even after a successful TCP/TLS handoff.
A standard .NET 6 `ClientWebSocket` probe inside the same Wine prefix succeeded.
The opt-in `PhotonManagedSocket.cs` replacement therefore uses that implementation
for WebSocket I/O, while retaining Photon's serialization, authentication, rooms,
and game messages. It honors .NET's normal proxy discovery (`HTTPS_PROXY` in this
session) and certificate verification. It does not change game rules or choices.

`photon.txt` containing `wss` and a second line `us` enables secure WebSockets;
`DCGO_PHOTON_MANAGED_SOCKET=1` enables the replacement. Without the opt-ins, the
game uses its normal networking. The cloud route needs outbound access through
the configured proxy to Photon's name, master, and game servers, including
`ns.photonengine.io:19093` and the returned `*.exitgames.com:443` endpoints.
No external tunnel, DNS shim, or TLS verification override is required.

### Check completion and retain evidence

Check both `Game/MelonLoader/Latest.log` files for `managed WebSocket connected`,
the same room with two players, `scene loaded: BattleScene`, and `IsAI=False`.
Then inspect **both agent logs**, the raw captures and their state snapshots.
A room connection alone does not prove a scripted decision or card effect ran.
Preserve any `ZONE-MISMATCH`, script divergence, unsupported action or truncation;
do not convert it into a pass or remove the failing decision to finish a game.
The agent's script fingerprint is built from the engine replay of captured
actions. Raw DCGO zone checks are separate and currently warn without stopping
the script. A script that keeps advancing can therefore still have state mismatches.

The first request can precede creation of the raw file while BattleScene is still
initializing. Both tested clients recovered on the next request and proceeded to
the scripted mulligans. Treat a persistent missing-file error differently: check
the Wine path mapping and whether snapshots are being written.

Archive each client's raw JSONL and logs, both agent logs, the input recording,
shared plan, DLL/client hashes, and terminal winner. Convert and run `dcgo-replay`
on each client's recording using the commands above. Compare those outcomes with
the native recording and inspect board-zone checks. Report the actual number of
executed decisions and any skipped decisions. Keep action-replay parity distinct
from full state parity and clause-level Oracle certification.

For a new fixture, generate a native recording using a compatible policy model
and two JSON card-ID arrays (eggs included), then inspect its actions to confirm
that the effect and choices of interest actually occur:

```bash
target/debug/digimon-engine-cli --pool all --cards-json data/cards.json sim-record \
  --model /path/to/compatible-policy.onnx \
  --deck0 /path/to/card-test-deck.json --deck1 /path/to/opponent-deck.json \
  --seed 1 --out /workspace/dcgo-cloud/card-test.json
```

Use that same recording for `prepare-playback.py` and both `dcgo-agent --script`
processes. The tiny model used only to satisfy scripted-agent startup above is
not a trained pilot for generating useful effect coverage. A card being present
in a deck does not establish that its effect was exercised.

## Operations and failure diagnosis

- DXVK with Xvfb crashed in DXGI with division by zero. Xorg dummy's explicit
  ~60-Hz mode fixed that. `-force-glcore` fails because the shipped player lacks
  OpenGL Core shaders. Keep the tested DXVK/Lavapipe combination for this image.
- `version=n,b` loads MelonLoader; `dxgi,d3d11=n,b` loads DXVK. Missing either
  changes the execution path. Check `Latest.log`, not just the visible menu.
- The initial `TurnStateMachine.Init` snapshot can precede player construction
  and log a null-reference warning. Confirm later pre/post-mulligan snapshots
  exist; do not accept a raw file containing only initialization.
- Banlist fetch completion remained unverified here. Bot and scripted PvP matches
  started. No certificate verification was disabled. Preserve the cloud's
  proxy/CA configuration and the opt-in managed WebSocket setting for this route.
- Cloud sandbox networking and Unix display sockets required normal command
  escalation here. No desktop connector was exposed, so captures used
  ImageMagick on Xorg `:100`; that display is not automatically the shared UI.
- `run.sh` overwrites `evidence/Player.log` and `wine.log`; archive them first.
  `stop.sh` stops only the selected Wine prefix. Stop its display afterward.
  Check `xdpyinfo` and processes after resuming a session: stale X socket files
  do not establish a running display.
- Preserve downloads, package versions, prefix, generated interop and evidence
  in supported cloud artifact storage. `/workspace` persistence across machines
  is not guaranteed. Do not commit those multi-gigabyte artifacts to this repo.
