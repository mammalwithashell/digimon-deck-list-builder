# DCGO Build Setup

This guide covers building the DCGO submodule from source — required for the
DCGO game-recording mod (see `openspec/changes/add-dcgo-recording-parity-harness/`).
DCGO is a Unity client; we modify it to log every decision made by either
player as a 2192-space action ID, and a Rust harness replays those recordings
through `digimon-engine` to validate parity.

You only need this build if you are working on Phase 1 of the recording
campaign (running the bot-vs-bot fuzzer or testing the recorder mod). The
Rust replay harness, codegen pipeline, and engine opaque-deck work all build
without Unity.

## Prerequisites

| Component                    | Version                | Notes |
|------------------------------|------------------------|-------|
| Unity Hub                    | latest                 | Free, https://unity.com/download |
| Unity Editor                 | **2021.3.45f2** exactly | Older or newer versions of 2021 LTS are not supported by DCGO upstream. |
| Git LFS                      | latest                 | Required for DCGO's bundled binary assets. |
| Visual Studio 2019 or 2022 (Windows) / Rider / VS Code with C# extension | latest | For editing the mod's C# files. |
| .NET desktop dev + Game dev w/ Unity workloads (VS only) | latest | Both required for Unity 2021 LTS. |

## Step-by-step

### 1. Acquire the DCGO Assets bundle

DCGO splits its repository into source (the `DCGO` submodule you already
have) and a separate bundle of art / audio / prefabs that the upstream
maintainers ship out-of-band. The `DCGO/README.md` step 4 references this:
"DCGO (Specifically the Assets folder to make things easier later)".

The bundle is **not** committed to the submodule for size/licensing reasons.
Acquire it through one of:

- The DCGO community Discord (`#downloads` channel) — pinned message links to the latest bundle.
- A teammate who has already set up DCGO — they can zip their `DCGO/Assets/` and share it.

The bundle is a `.zip` containing the contents of `DCGO/Assets/`. Extract it
into your local `DCGO/Assets/` directory (it will merge with the files the
submodule has shipped in source control — e.g. `DCGO/Assets/Scripts/`).

**Do not commit the bundle.** It is large and out-of-scope for our patch.
`.gitignore` already excludes the bundle's prefab/audio/image directories
where applicable; verify with `git status` before committing.

### 2. Initialize and verify the submodule

From the repository root:

```bash
git submodule update --init --recursive DCGO
git -C DCGO status                    # should show "HEAD detached at <sha>" — that's correct
git -C DCGO rev-parse HEAD            # record this SHA; it is our pinned upstream commit
```

The submodule's pinned SHA lives in our repo's `gitlink` for `DCGO`. Do not
update it casually — see "Updating the upstream pin" below.

### 3. Install Unity 2021.3.45f2

1. Open Unity Hub.
2. Go to **Installs** → **Install Editor**.
3. Click the **Archive** tab → "download archives" link in your browser.
4. Browse to Unity 2021 LTS → find **2021.3.45f2** → install.
5. When prompted, install the **Visual Studio Community 2019** integration
   (or your preferred IDE). Required workloads: .NET desktop development,
   Game development with Unity.
6. When asked to **Initialize Git LFS**, accept.

### 4. Open the DCGO project

1. In Unity Hub → **Projects** → **Add** → **Add project from disk**.
2. Navigate to the repo root, select `DCGO/`, click Open.
3. When prompted by errors during initial load, click **Ignore** if asked to
   enter Safe Mode. Upstream README confirms this is expected.
4. Wait for the asset import to finish (10–30 minutes on first run depending
   on hardware).

### 5. Smoke test: run one Bot Match end-to-end

1. In the Unity Editor, open the **Scenes/Opening** scene
   (`Assets/Scenes/Opening.unity`).
2. Press **Play** at the top of the Editor.
3. From the home screen, click **Battle**.
4. Select **Bot Match** (the third option after Random / Room match — Japanese
   label is "Bot戦").
5. Pick any of the bundled starter decks.
6. The game should advance turn-by-turn unattended; both sides are bot-driven.
7. Wait for the result screen to show a winner.

If this works, your DCGO build is healthy and ready for the recorder mod.

## Updating the upstream pin

To pull a newer upstream commit (e.g. for a new card set), follow this order:

1. **Branch first** — DCGO updates can break our patch, so do this on a side
   branch, never directly on `main`.
2. `git -C DCGO fetch && git -C DCGO checkout <new-sha>`
3. Re-apply the recording patch on top (see the diff under
   `DCGO/Assets/Scripts/Script/Recording/` and the call-site additions in
   `TurnStateMachine.cs` and `UserSelectionManager.cs`).
4. Re-run the codegen drift check:
   `cargo run -p action-space-export | python code/tools/action-space-export/emit_csharp.py --check --out DCGO/Assets/Scripts/Script/Recording/ActionSpace.cs`
5. Re-run the bot-match smoke test (step 5 above) and verify a recording is
   produced under `Application.persistentDataPath/dcgo_recordings/`.
6. Commit the submodule pointer update and any patch updates as one atomic
   change.

## Troubleshooting

- **"NormalMap settings" warning on first open** — click **Ignore** per the
  DCGO README. Cosmetic only.
- **"Could not find Photon AppId"** — the bundled `PhotonServerSettings.asset`
  ships the community AppId; if you see this, you missed the Assets bundle
  extraction step. Bot Match works without Photon, so this only affects
  Random/Room match.
- **Editor crashes during asset import** — the bundle has known issues with
  Unity versions other than 2021.3.45f2. Verify your editor version.
- **C# files in `DCGO/Assets/Scripts/Script/Recording/` show as red** —
  Unity may not have refreshed after `ActionSpace.cs` was added. From the
  Editor menu: **Assets → Refresh** (or `Ctrl+R`).

## What the recorder mod adds

Once Phase 1 is implemented, this directory will exist with files:

```
DCGO/Assets/Scripts/Script/Recording/
├── ActionSpace.cs       — generated from code/tools/action-space-export
├── GameRecorder.cs      — MonoBehaviour writing JSONL recordings
├── ActionEncoder.cs     — maps DCGO actions → 2192-space IDs
└── RecorderConfig.cs    — bot-only vs PvP toggles
```

Plus minimal one-line additions inside:
- `TurnStateMachine.QueueMainPhaseAction` (one chokepoint for all 6 main-phase action types)
- `UserSelectionManager.SetIntForPlayer_External` / `SetBoolForPlayer_External` (one chokepoint each for all 15+ Select* effects)

The DCGO patch is intentionally surgical so it can be rebased onto upstream
commits cheaply.

## See also

- `openspec/changes/add-dcgo-recording-parity-harness/` — the OpenSpec change driving this work
- `docs/DCGO_RECORDING_SCHEMA.md` *(Phase 1 deliverable)* — JSONL format reference
- `code/tools/action-space-export/` — Rust→C# codegen for the action enum
- `code/tools/dcgo-replay/` *(Phase 1 deliverable)* — Rust replay harness
