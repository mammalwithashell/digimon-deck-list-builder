# DCGO Layer A (Autonomy) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let an agent start, health-check, and stop DCGO with no human pressing Play, against a versioned build that refuses to run if its action-space encoding has gone stale.

**Architecture:** A Unity Editor script builds a standalone player headlessly. `dcgo-harness build` drives that build and stamps a `manifest.json` (DCGO commit, artifact SHA256, action-space digest). `dcgo-harness up` verifies the manifest's action-space digest against the engine's, launches the player, and watches a heartbeat file that `JobWatcher` touches every poll. `down` stops it. Stale heartbeat means kill, requeue, bounded restart.

**Tech Stack:** Rust (`code/tools/dcgo-harness`, clap + serde + sha2), C# / Unity 2021.3.45f2 (`DCGO/Assets/Scripts/Script/Harness/`).

**Spec:** `docs/superpowers/specs/2026-08-20-dcgo-agent-puppet-design.md` (Layer A).

## Global Constraints

- **Unity version is 2021.3.45f2.** Editor at `C:/Program Files/Unity/Hub/Editor/2021.3.45f2/Editor/Unity.exe`. A `6000.3.5f2` install also exists on this machine — do not use it; opening the project with it would upgrade and corrupt the checkout.
- **DCGO lives in the base repo, never a worktree (CLAUDE.md rule 29).** Resolve it as `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`. Never run `git submodule update --init DCGO` from a worktree.
- **Build output goes to `D:\dcgo-build\`** — outside the submodule. It is multi-GB derived data and DCGO is an LFS checkout. Nothing generated may land where git can see it.
- **Every new `.cs` file needs Unity to import it.** No `.meta` file beside a `.cs` file means Unity has not seen it, and a clean Console is reporting the *previous* compile.
- **Rust target dir is per-worktree (CLAUDE.md rule 31).** Prefix cargo with `CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190'` if the session predates the env change.
- **C# namespaces:** runtime code in `Digimon.Harness`, Editor-only code in `Digimon.Harness.EditorTools` (matching `HarnessMenu.cs`).
- **Existing invariant:** `HarnessConfig.Enabled` defaults false; a stale job file must never hijack a normal play session. Nothing in this plan may change that default.

---

### Task 1: Unity build entry point — the spike gate

This task is a **hard gate**, timeboxed to one day. If a launchable player does not come out the far end, stop and take the Editor fallback (noted in Step 6) rather than fighting AssetRipper damage. The rest of the plan works either way.

**Files:**
- Create: `<BASE_DCGO>/Assets/Scripts/Script/Harness/Editor/HarnessBuild.cs`

**Interfaces:**
- Consumes: nothing.
- Produces: static method `Digimon.Harness.EditorTools.HarnessBuild.Build()`, invoked by Unity via `-executeMethod`. Reads output directory from the command line argument `-harnessBuildOutput <path>`. Exits the Editor with code 0 on success, 1 on failure.

- [ ] **Step 1: Write the build script**

Create `<BASE_DCGO>/Assets/Scripts/Script/Harness/Editor/HarnessBuild.cs`:

```csharp
using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace Digimon.Harness.EditorTools
{
    /// <summary>
    /// Headless standalone build of DCGO with the harness mod, so an agent can
    /// run the oracle without an Editor session.
    /// </summary>
    /// <remarks>
    /// Invoked as:
    ///   Unity.exe -quit -batchmode -nographics -projectPath &lt;DCGO&gt;
    ///     -executeMethod Digimon.Harness.EditorTools.HarnessBuild.Build
    ///     -harnessBuildOutput D:\dcgo-build\&lt;version&gt; -logFile -
    ///
    /// The output directory is passed on the command line rather than baked in
    /// because the host CLI owns versioning: it picks the directory, then hashes
    /// what lands there. Baking a path here would split that ownership across
    /// two languages.
    /// </remarks>
    public static class HarnessBuild
    {
        private const string OutputArg = "-harnessBuildOutput";
        private const string ExecutableName = "DCGO.exe";

        public static void Build()
        {
            try
            {
                string outputDir = ReadOutputArg();
                if (string.IsNullOrEmpty(outputDir))
                {
                    Fail("missing " + OutputArg + " <path> on the command line");
                    return;
                }

                Directory.CreateDirectory(outputDir);

                string[] scenes = EditorBuildSettings.scenes
                    .Where(s => s.enabled)
                    .Select(s => s.path)
                    .ToArray();

                if (scenes.Length == 0)
                {
                    Fail("no enabled scenes in EditorBuildSettings; nothing to build");
                    return;
                }

                Debug.Log("[HarnessBuild] building " + scenes.Length + " scene(s) -> " + outputDir);

                var options = new BuildPlayerOptions
                {
                    scenes = scenes,
                    locationPathName = Path.Combine(outputDir, ExecutableName),
                    target = BuildTarget.StandaloneWindows64,
                    options = BuildOptions.None,
                };

                BuildReport report = BuildPipeline.BuildPlayer(options);
                BuildSummary summary = report.summary;

                if (summary.result == BuildResult.Succeeded)
                {
                    Debug.Log("[HarnessBuild] OK: " + summary.totalSize + " bytes -> "
                              + summary.outputPath);
                    EditorApplication.Exit(0);
                    return;
                }

                // Surface the first few errors inline. In batchmode the log is
                // the only artifact, and BuildReport's own summary says only
                // "Failed" with a count.
                foreach (var step in report.steps)
                {
                    foreach (var msg in step.messages)
                    {
                        if (msg.type == LogType.Error || msg.type == LogType.Exception)
                        {
                            Debug.LogError("[HarnessBuild] " + step.name + ": " + msg.content);
                        }
                    }
                }

                Fail("build result " + summary.result + " with "
                     + summary.totalErrors + " error(s)");
            }
            catch (Exception e)
            {
                Fail("threw " + e.GetType().Name + ": " + e.Message);
            }
        }

        private static string ReadOutputArg()
        {
            string[] args = Environment.GetCommandLineArgs();
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (args[i] == OutputArg)
                {
                    return args[i + 1];
                }
            }
            return null;
        }

        private static void Fail(string reason)
        {
            // EditorApplication.Exit is what makes -quit honour a nonzero code.
            // Throwing instead would exit 0 and report a broken build as a
            // successful one -- the exact silent-pass shape the harness's
            // denominator rules exist to prevent.
            Debug.LogError("[HarnessBuild] FAILED: " + reason);
            EditorApplication.Exit(1);
        }
    }
}
```

- [ ] **Step 2: Let Unity import it**

Open the Unity Editor on the DCGO project and click into the editor window to force a rescan. Then verify:

```bash
ls "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO/Assets/Scripts/Script/Harness/Editor/HarnessBuild.cs.meta"
```

Expected: the path prints. If it does not exist, Unity has not imported the file and the build will fail with "executeMethod method not found".

- [ ] **Step 3: Close the Editor**

A running Editor holds the project lock. Batchmode against a locked project fails with "Multiple Unity instances cannot open the same project".

- [ ] **Step 4: Run the headless build**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"; "C:/Program Files/Unity/Hub/Editor/2021.3.45f2/Editor/Unity.exe" -quit -batchmode -nographics -projectPath "$BASE_DCGO" -executeMethod Digimon.Harness.EditorTools.HarnessBuild.Build -harnessBuildOutput "D:/dcgo-build/spike" -logFile - 2>&1 | tail -40
```

Expected on success: `[HarnessBuild] OK: <n> bytes` and exit code 0. This takes a long time on a first build (shader compilation on a project this size can run 20+ minutes) — do not conclude it hung before 45 minutes.

Expected failure modes and what they mean:
- Hangs with no output at all → Unity license not activated for batchmode. Open the Editor once, confirm the license, retry.
- `executeMethod method not found` → Step 2 was skipped.
- Shader / script compile errors → AssetRipper damage. This is the gate; see Step 6.

- [ ] **Step 5: Confirm the player launches**

```bash
ls -la "D:/dcgo-build/spike/DCGO.exe" && "D:/dcgo-build/spike/DCGO.exe" -logFile - 2>&1 | head -30
```

Expected: the exe exists and logs Unity startup lines without immediately crashing. Close it manually.

- [ ] **Step 6: Record the gate outcome**

If Steps 4-5 succeeded, note it and continue to Task 2 unchanged.

If they failed past the one-day timebox, **stop and switch to the Editor fallback**: Task 5's `up` launches the Editor with `-executeMethod` calling `EditorApplication.EnterPlaymode()` instead of launching a player exe. Record the failure reason in `docs/DCGO_HARNESS.md` under "Known gaps" and tell the user before proceeding — Tasks 2 and 3 change shape (there is no artifact to hash, so `artifact_sha256` becomes the DCGO commit alone).

- [ ] **Step 7: Commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"; git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/Editor/HarnessBuild.cs Assets/Scripts/Script/Harness/Editor/HarnessBuild.cs.meta && git -C "$BASE_DCGO" commit -m "Harness: headless standalone build entry point"
```

---

### Task 2: Build manifest types and the action-space digest

**Files:**
- Create: `code/tools/dcgo-harness/src/manifest.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs` (add `pub mod manifest;`)
- Modify: `code/tools/dcgo-harness/Cargo.toml` (add `sha2`, `action-space-export`)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces:
  - `pub struct BuildManifest { dcgo_commit: String, built_at: String, artifact_sha256: String, action_space_hash: String, executable: String }` with `Serialize`/`Deserialize`.
  - `pub const MANIFEST_FILE: &str = "manifest.json"`
  - `pub fn action_space_hash() -> String`
  - `pub fn digest_descriptor(v: &serde_json::Value) -> String`
  - `pub fn sha256_file(path: &Path) -> Result<String, String>`
  - `pub fn load(build_dir: &Path) -> Result<BuildManifest, String>`
  - `pub fn save(build_dir: &Path, m: &BuildManifest) -> Result<(), String>`
  - `pub fn check_action_space(m: &BuildManifest, engine_hash: &str) -> Result<(), String>`

- [ ] **Step 1: Add the dependencies**

In `code/tools/dcgo-harness/Cargo.toml`, under `[dependencies]`, add:

```toml
sha2 = "0.10"
action-space-export = { path = "../action-space-export" }
```

- [ ] **Step 2: Write the failing tests**

Create `code/tools/dcgo-harness/src/manifest.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn digest_is_stable_across_calls() {
        let v = json!({"a": 1, "b": [1, 2, 3]});
        assert_eq!(digest_descriptor(&v), digest_descriptor(&v));
    }

    #[test]
    fn digest_ignores_object_key_order() {
        // serde_json's Map is a BTreeMap by default but an IndexMap when the
        // `preserve_order` feature is enabled anywhere in the dependency graph.
        // Cargo feature unification means an unrelated crate could turn that on
        // and silently change every digest -- which would make every previously
        // built player fail the gate for no real reason. Canonicalising the key
        // order ourselves makes the digest independent of that.
        let a: serde_json::Value = serde_json::from_str(r#"{"x":1,"y":2}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"{"y":2,"x":1}"#).unwrap();
        assert_eq!(digest_descriptor(&a), digest_descriptor(&b));
    }

    #[test]
    fn digest_respects_array_order() {
        // Array order is semantics, not representation: action indices are
        // positional. Two spaces with the same actions in a different order are
        // different spaces.
        let a = json!([1, 2]);
        let b = json!([2, 1]);
        assert_ne!(digest_descriptor(&a), digest_descriptor(&b));
    }

    #[test]
    fn digest_changes_when_a_value_changes() {
        assert_ne!(digest_descriptor(&json!({"n": 2192})), digest_descriptor(&json!({"n": 2193})));
    }

    #[test]
    fn action_space_hash_is_nonempty_and_stable() {
        let h = action_space_hash();
        assert_eq!(h.len(), 64, "sha256 hex is 64 chars, got {}", h);
        assert_eq!(h, action_space_hash());
    }

    fn manifest_with(hash: &str) -> BuildManifest {
        BuildManifest {
            dcgo_commit: "be359bb5b".into(),
            built_at: "2026-08-20T14:02:11Z".into(),
            artifact_sha256: "deadbeef".into(),
            action_space_hash: hash.into(),
            executable: "DCGO.exe".into(),
        }
    }

    #[test]
    fn gate_accepts_a_matching_hash() {
        let m = manifest_with("abc123");
        assert!(check_action_space(&m, "abc123").is_ok());
    }

    #[test]
    fn gate_rejects_a_stale_hash_and_says_why() {
        let m = manifest_with("aaaaaaaaaaaaaaaa");
        let err = check_action_space(&m, "bbbbbbbbbbbbbbbb").unwrap_err();
        assert!(err.contains("action-space mismatch"), "got: {}", err);
        assert!(err.contains("dcgo-harness build"), "must say how to fix: {}", err);
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let m = manifest_with("abc123");
        let text = serde_json::to_string_pretty(&m).unwrap();
        let back: BuildManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn load_reports_a_missing_manifest_clearly() {
        let dir = std::env::temp_dir().join("dcgo_manifest_missing_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let err = load(&dir).unwrap_err();
        assert!(err.contains("manifest.json"), "got: {}", err);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test -p dcgo-harness manifest 2>&1 | tail -20
```

Expected: FAIL to compile — `cannot find function digest_descriptor`, `cannot find type BuildManifest`.

- [ ] **Step 4: Write the implementation**

Prepend to `code/tools/dcgo-harness/src/manifest.rs`, above the test module:

```rust
//! What a built DCGO player *is*, and the gate deciding whether it may run.
//!
//! A build embeds a frozen snapshot of `ActionSpace.cs` (CLAUDE.md rule 27).
//! If `code/digimon-engine/src/action/space.rs` changes afterwards, that build
//! keeps encoding against the old space and every recording it produces is
//! corrupt -- in a way that reads as engine divergence rather than as a broken
//! tool. The manifest stamps the digest so `up` can refuse instead.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Filename of the manifest inside a build directory.
pub const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildManifest {
    /// DCGO submodule commit the player was built from.
    pub dcgo_commit: String,
    /// RFC3339 UTC build timestamp.
    pub built_at: String,
    /// SHA256 of the launchable executable.
    pub artifact_sha256: String,
    /// Digest of the action-space descriptor at build time.
    pub action_space_hash: String,
    /// Executable path relative to the build directory.
    pub executable: String,
}

/// Digest of the engine's current action-space descriptor.
pub fn action_space_hash() -> String {
    digest_descriptor(&action_space_export::build())
}

/// Digest a JSON descriptor in a canonical form.
pub fn digest_descriptor(v: &serde_json::Value) -> String {
    let mut canonical = String::new();
    write_canonical(v, &mut canonical);
    let mut h = Sha256::new();
    h.update(canonical.as_bytes());
    format!("{:x}", h.finalize())
}

/// Serialize with object keys sorted, independent of which map type
/// `serde_json` was compiled with. See the `digest_ignores_object_key_order`
/// test for why this is not left to `serde_json::to_string`.
fn write_canonical(v: &serde_json::Value, out: &mut String) {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).expect("string key serializes"));
                out.push(':');
                write_canonical(&map[k.as_str()], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&serde_json::to_string(scalar).expect("scalar serializes")),
    }
}

/// SHA256 of a file, streamed so a multi-GB artifact does not need to fit in RAM.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)
        .map_err(|e| format!("opening {}: {}", path.display(), e))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| format!("reading {}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

/// Read the manifest out of a build directory.
pub fn load(build_dir: &Path) -> Result<BuildManifest, String> {
    let path = build_dir.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("reading {}: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {}: {}", path.display(), e))
}

/// Write the manifest into a build directory.
pub fn save(build_dir: &Path, m: &BuildManifest) -> Result<(), String> {
    let path = build_dir.join(MANIFEST_FILE);
    let text =
        serde_json::to_string_pretty(m).map_err(|e| format!("serializing manifest: {}", e))?;
    std::fs::write(&path, text).map_err(|e| format!("writing {}: {}", path.display(), e))
}

/// Refuse a build whose action-space digest no longer matches the engine's.
pub fn check_action_space(m: &BuildManifest, engine_hash: &str) -> Result<(), String> {
    if m.action_space_hash == engine_hash {
        return Ok(());
    }
    Err(format!(
        "action-space mismatch: build {} was stamped {}, engine is now {}.\n\
         That build encodes actions against a stale space, so every recording it \
         produced would be corrupt in a way that reads as engine divergence.\n\
         Rebuild with `dcgo-harness build`.",
        m.dcgo_commit,
        short(&m.action_space_hash),
        short(engine_hash),
    ))
}

fn short(hash: &str) -> &str {
    if hash.len() >= 12 {
        &hash[..12]
    } else {
        hash
    }
}
```

Add to `code/tools/dcgo-harness/src/lib.rs`, keeping the module list alphabetical:

```rust
pub mod job;
pub mod manifest;
pub mod pool;
pub mod queue;
pub mod triage;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p dcgo-harness manifest 2>&1 | tail -20
```

Expected: `test result: ok. 9 passed`.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/manifest.rs code/tools/dcgo-harness/src/lib.rs code/tools/dcgo-harness/Cargo.toml Cargo.lock && git commit -m "dcgo-harness: build manifest and action-space digest gate"
```

---

### Task 3: `dcgo-harness build`

**Files:**
- Create: `code/tools/dcgo-harness/src/build.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs` (add `pub mod build;`)
- Modify: `code/tools/dcgo-harness/src/main.rs` (add the `Build` subcommand + dispatch arm)

**Interfaces:**
- Consumes: `manifest::{BuildManifest, action_space_hash, sha256_file, save}` from Task 2.
- Produces:
  - `pub struct BuildRequest { pub unity_exe: PathBuf, pub project_path: PathBuf, pub output_dir: PathBuf }`
  - `pub fn unity_args(req: &BuildRequest) -> Vec<String>`
  - `pub fn git_commit(project_path: &Path) -> Result<String, String>`
  - `pub fn stamp(req: &BuildRequest, exe_name: &str, commit: String, built_at: String) -> Result<BuildManifest, String>`
  - `pub fn run(req: &BuildRequest) -> Result<BuildManifest, String>`

- [ ] **Step 1: Write the failing tests**

Create `code/tools/dcgo-harness/src/build.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> BuildRequest {
        BuildRequest {
            unity_exe: PathBuf::from("C:/Unity/Unity.exe"),
            project_path: PathBuf::from("C:/repo/DCGO"),
            output_dir: PathBuf::from("D:/dcgo-build/v1"),
        }
    }

    #[test]
    fn unity_args_are_batchmode_and_quit() {
        let a = unity_args(&req());
        assert!(a.contains(&"-quit".to_string()));
        assert!(a.contains(&"-batchmode".to_string()));
        assert!(a.contains(&"-nographics".to_string()));
    }

    #[test]
    fn unity_args_name_the_build_method_and_output() {
        let a = unity_args(&req());
        let joined = a.join(" ");
        assert!(
            joined.contains("Digimon.Harness.EditorTools.HarnessBuild.Build"),
            "got: {}",
            joined
        );
        let i = a.iter().position(|s| s == "-harnessBuildOutput").expect("output flag");
        assert_eq!(a[i + 1], "D:/dcgo-build/v1");
    }

    #[test]
    fn unity_args_log_to_stdout() {
        // Without `-logFile -` a batchmode build writes to a platform log path
        // and the caller sees nothing at all on failure.
        let a = unity_args(&req());
        let i = a.iter().position(|s| s == "-logFile").expect("logFile flag");
        assert_eq!(a[i + 1], "-");
    }

    #[test]
    fn stamp_fails_when_the_executable_is_missing() {
        let r = BuildRequest {
            output_dir: std::env::temp_dir().join("dcgo_build_stamp_missing"),
            ..req()
        };
        let _ = std::fs::remove_dir_all(&r.output_dir);
        std::fs::create_dir_all(&r.output_dir).unwrap();
        let err = stamp(&r, "DCGO.exe", "abc".into(), "t".into()).unwrap_err();
        assert!(err.contains("DCGO.exe"), "got: {}", err);
        let _ = std::fs::remove_dir_all(&r.output_dir);
    }

    #[test]
    fn stamp_records_commit_hash_and_current_action_space() {
        let r = BuildRequest {
            output_dir: std::env::temp_dir().join("dcgo_build_stamp_ok"),
            ..req()
        };
        let _ = std::fs::remove_dir_all(&r.output_dir);
        std::fs::create_dir_all(&r.output_dir).unwrap();
        std::fs::write(r.output_dir.join("DCGO.exe"), b"fake player").unwrap();

        let m = stamp(&r, "DCGO.exe", "be359bb5b".into(), "2026-08-20T00:00:00Z".into()).unwrap();

        assert_eq!(m.dcgo_commit, "be359bb5b");
        assert_eq!(m.executable, "DCGO.exe");
        assert_eq!(m.action_space_hash, crate::manifest::action_space_hash());
        assert_eq!(m.artifact_sha256.len(), 64);
        // A stamped manifest must pass its own gate, or `build` would produce
        // an artifact `up` immediately refuses.
        assert!(crate::manifest::check_action_space(&m, &crate::manifest::action_space_hash()).is_ok());

        let _ = std::fs::remove_dir_all(&r.output_dir);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p dcgo-harness build:: 2>&1 | tail -20
```

Expected: FAIL to compile — `cannot find type BuildRequest`.

- [ ] **Step 3: Write the implementation**

Prepend to `code/tools/dcgo-harness/src/build.rs`:

```rust
//! Drives the headless Unity build and stamps its manifest.
//!
//! The host CLI owns versioning rather than the Editor script: it chooses the
//! output directory, then hashes what lands there and records the DCGO commit.
//! Keeping both halves here means the digest and the artifact can never be
//! stamped by two different notions of "current".

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::{self, BuildManifest};

/// Method Unity invokes via `-executeMethod`. Must match `HarnessBuild.cs`.
const BUILD_METHOD: &str = "Digimon.Harness.EditorTools.HarnessBuild.Build";

/// Default executable name produced by `HarnessBuild.cs`.
pub const DEFAULT_EXECUTABLE: &str = "DCGO.exe";

#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub unity_exe: PathBuf,
    pub project_path: PathBuf,
    pub output_dir: PathBuf,
}

/// Command line for the headless build.
pub fn unity_args(req: &BuildRequest) -> Vec<String> {
    vec![
        "-quit".into(),
        "-batchmode".into(),
        "-nographics".into(),
        "-projectPath".into(),
        req.project_path.display().to_string(),
        "-executeMethod".into(),
        BUILD_METHOD.into(),
        "-harnessBuildOutput".into(),
        req.output_dir.display().to_string(),
        // Without this the log goes to a platform path and a failed build is
        // silent to the caller.
        "-logFile".into(),
        "-".into(),
    ]
}

/// Resolve the DCGO submodule's HEAD, so an answer can be traced to a source commit.
pub fn git_commit(project_path: &Path) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(project_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("running git in {}: {}", project_path.display(), e))?;
    if !out.status.success() {
        return Err(format!(
            "git rev-parse in {} failed: {}",
            project_path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Build the manifest for an artifact already on disk.
pub fn stamp(
    req: &BuildRequest,
    exe_name: &str,
    commit: String,
    built_at: String,
) -> Result<BuildManifest, String> {
    let exe = req.output_dir.join(exe_name);
    if !exe.exists() {
        return Err(format!(
            "build reported success but {} does not exist",
            exe.display()
        ));
    }
    Ok(BuildManifest {
        dcgo_commit: commit,
        built_at,
        artifact_sha256: manifest::sha256_file(&exe)?,
        action_space_hash: manifest::action_space_hash(),
        executable: exe_name.to_string(),
    })
}

/// Run the Unity build, then stamp and save its manifest.
pub fn run(req: &BuildRequest) -> Result<BuildManifest, String> {
    if !req.unity_exe.exists() {
        return Err(format!("unity not found at {}", req.unity_exe.display()));
    }
    std::fs::create_dir_all(&req.output_dir)
        .map_err(|e| format!("creating {}: {}", req.output_dir.display(), e))?;

    let commit = git_commit(&req.project_path)?;

    println!("building DCGO {} -> {}", &commit[..commit.len().min(9)], req.output_dir.display());
    println!("(a first build compiles shaders and can take 20+ minutes)");

    let status = Command::new(&req.unity_exe)
        .args(unity_args(req))
        .status()
        .map_err(|e| format!("launching {}: {}", req.unity_exe.display(), e))?;

    if !status.success() {
        return Err(format!(
            "unity build failed with exit code {:?}. Re-run with the same args to see the log.",
            status.code()
        ));
    }

    let built_at = chrono::Utc::now().to_rfc3339();
    let m = stamp(req, DEFAULT_EXECUTABLE, commit, built_at)?;
    manifest::save(&req.output_dir, &m)?;
    Ok(m)
}
```

Add `chrono = "0.4"` to `code/tools/dcgo-harness/Cargo.toml`. It is already in
`Cargo.lock` (used by `code/src-tauri`), so this pulls in nothing new.

Add `pub mod build;` to `code/tools/dcgo-harness/src/lib.rs` (alphabetically first, before `job`).

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p dcgo-harness build:: 2>&1 | tail -20
```

Expected: `test result: ok. 5 passed`.

- [ ] **Step 5: Wire the subcommand**

In `code/tools/dcgo-harness/src/main.rs`, add to `enum Command`:

```rust
    /// Build a standalone DCGO player and stamp its manifest.
    Build {
        /// Unity editor executable.
        #[arg(long, default_value = "C:/Program Files/Unity/Hub/Editor/2021.3.45f2/Editor/Unity.exe")]
        unity: PathBuf,
        /// DCGO project path (base repo, not a worktree -- CLAUDE.md rule 29).
        #[arg(long)]
        project: PathBuf,
        /// Where the player goes. Must be outside the DCGO submodule.
        #[arg(long)]
        output: PathBuf,
    },
```

And add to the `match &args.command` block in `run`:

```rust
        Command::Build {
            unity,
            project,
            output,
        } => {
            let req = dcgo_harness::build::BuildRequest {
                unity_exe: unity.clone(),
                project_path: project.clone(),
                output_dir: output.clone(),
            };
            let m = dcgo_harness::build::run(&req)?;
            println!("built {}", output.display());
            println!("  dcgo_commit       {}", m.dcgo_commit);
            println!("  artifact_sha256   {}", m.artifact_sha256);
            println!("  action_space_hash {}", m.action_space_hash);
            Ok(ExitCode::SUCCESS)
        }
```

- [ ] **Step 6: Verify the CLI wiring compiles and the help renders**

```bash
cargo run -p dcgo-harness -- --root D:/tmp/harness build --help 2>&1 | tail -20
```

Expected: help text listing `--unity`, `--project`, `--output`.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness/src/build.rs code/tools/dcgo-harness/src/lib.rs code/tools/dcgo-harness/src/main.rs && git commit -m "dcgo-harness: build subcommand drives Unity and stamps the manifest"
```

---

### Task 4: Heartbeat and idle exit

**Files:**
- Modify: `<BASE_DCGO>/Assets/Scripts/Script/Harness/HarnessConfig.cs`
- Modify: `<BASE_DCGO>/Assets/Scripts/Script/Harness/JobWatcher.cs` (`PollLoop`, around line 85-105)

**Interfaces:**
- Consumes: existing `HarnessConfig.Root`, `HarnessConfig.PollSeconds`, `JobWatcher.CurrentJob`, `JobWatcher.DcgoReady`.
- Produces: a file at `<root>/harness.heartbeat` whose mtime advances every `PollSeconds`; process self-exit after `HarnessConfig.ExitAfterIdleSeconds` of no work. Task 5 reads both.

- [ ] **Step 1: Add the config knobs**

In `HarnessConfig.cs`, after the `TimeScale` property, add:

```csharp
        /// <summary>
        /// File the watcher touches every poll so the host can tell a working
        /// DCGO from a hung one.
        /// </summary>
        /// <remarks>
        /// A PID is not enough: a hung Unity keeps its process alive and reports
        /// healthy forever. Both failures actually hit so far -- the unleft
        /// Photon room and the stalled selection -- looked exactly like that.
        /// The heartbeat is touched from the poll loop rather than from job
        /// completion, so it keeps advancing during a long game but stops if the
        /// coroutine itself dies.
        /// </remarks>
        public static string HeartbeatPath => Path.Combine(Root, "harness.heartbeat");

        /// <summary>
        /// Quit after this many seconds with nothing to do. 0 disables it.
        /// </summary>
        /// <remarks>
        /// One knob serves both lifecycles: a one-shot subprocess sets it low so
        /// it terminates when the queue drains; the warm daemon sets it high or
        /// leaves it off. Default 0 so Editor sessions are unaffected -- an
        /// Editor that exits Play mode on its own would be baffling.
        /// </remarks>
        public static float ExitAfterIdleSeconds { get; set; }
```

- [ ] **Step 2: Touch the heartbeat and track idleness**

In `JobWatcher.cs`, replace the `while (true)` block inside `PollLoop` with:

```csharp
            float idleSeconds = 0f;

            while (true)
            {
                TouchHeartbeat();

                if (CurrentJob == null && DcgoReady)
                {
                    TryClaimAndStart();
                }

                if (CurrentJob == null)
                {
                    idleSeconds += HarnessConfig.PollSeconds;
                    if (HarnessConfig.ExitAfterIdleSeconds > 0f
                        && idleSeconds >= HarnessConfig.ExitAfterIdleSeconds)
                    {
                        Debug.Log("[Harness] idle for " + idleSeconds
                                  + "s with an empty queue; exiting.");
                        QuitApplication();
                        yield break;
                    }
                }
                else
                {
                    idleSeconds = 0f;
                }

                yield return new WaitForSecondsRealtime(HarnessConfig.PollSeconds);
            }
```

Then add these two methods to the same class:

```csharp
        private void TouchHeartbeat()
        {
            try
            {
                // Rewrite rather than File.SetLastWriteTime: the content is a
                // useful second signal (which job is in flight) and a rewrite
                // updates mtime on every filesystem, which SetLastWriteTime does
                // not reliably do over a network path.
                Directory.CreateDirectory(HarnessConfig.Root);
                File.WriteAllText(
                    HarnessConfig.HeartbeatPath,
                    (CurrentJob == null ? "idle" : CurrentJob.JobId) + "\n");
            }
            catch (System.Exception e)
            {
                // A failed heartbeat must not kill the batch. The host will see
                // a stale file and restart, which is the correct response to a
                // DCGO that cannot write to its own root.
                Debug.LogWarning("[Harness] heartbeat write failed: " + e.Message);
            }
        }

        private static void QuitApplication()
        {
#if UNITY_EDITOR
            UnityEditor.EditorApplication.isPlaying = false;
#else
            Application.Quit();
#endif
        }
```

Ensure `using System.IO;` is present at the top of `JobWatcher.cs`.

- [ ] **Step 3: Let Unity import and compile**

Open the Editor, click into it, and confirm the Console shows no compile errors.

- [ ] **Step 4: Verify the heartbeat advances**

Enable the harness with an empty queue, press Play, and watch the file:

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"; for i in 1 2 3; do stat -c '%y %n' "$ROOT/harness.heartbeat" 2>/dev/null || echo "missing"; sleep 2; done
```

Expected: the timestamp advances between reads, and the content is `idle`.

- [ ] **Step 5: Verify idle exit does nothing by default**

With `ExitAfterIdleSeconds` at its default 0, leave Play running for a minute with an empty queue.

Expected: Play mode stays active. A default that exited would break every normal Editor session.

- [ ] **Step 6: Commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"; git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/HarnessConfig.cs Assets/Scripts/Script/Harness/JobWatcher.cs && git -C "$BASE_DCGO" commit -m "Harness: heartbeat file and optional idle exit"
```

---

### Task 5: `dcgo-harness up` and `down`

**Files:**
- Create: `code/tools/dcgo-harness/src/daemon.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs` (add `pub mod daemon;`)
- Modify: `code/tools/dcgo-harness/src/main.rs` (add `Up` / `Down` subcommands + dispatch)

**Interfaces:**
- Consumes: `manifest::{load, action_space_hash, check_action_space}` (Task 2); the heartbeat file written by Task 4.
- Produces:
  - `pub enum Health { Healthy, Stale { age_seconds: u64 }, Missing }`
  - `pub fn classify_heartbeat(age_seconds: Option<u64>, threshold_seconds: u64) -> Health`
  - `pub fn heartbeat_age(root: &Path) -> Option<u64>`
  - `pub fn read_pid(root: &Path) -> Option<u32>` / `pub fn write_pid(root: &Path, pid: u32) -> Result<(), String>` / `pub fn clear_pid(root: &Path) -> Result<(), String>`
  - `pub fn pid_alive(pid: u32) -> bool`
  - `pub const PID_FILE: &str` / `pub const HEARTBEAT_FILE: &str` / `pub const DEFAULT_STALE_SECONDS: u64`
  - `pub fn up(root: &Path, build_dir: &Path) -> Result<String, String>`
  - `pub fn down(root: &Path) -> Result<String, String>`

- [ ] **Step 1: Write the failing tests**

Create `code/tools/dcgo-harness/src/daemon.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_heartbeat_is_healthy() {
        assert_eq!(classify_heartbeat(Some(2), 30), Health::Healthy);
    }

    #[test]
    fn an_old_heartbeat_is_stale() {
        assert_eq!(
            classify_heartbeat(Some(120), 30),
            Health::Stale { age_seconds: 120 }
        );
    }

    #[test]
    fn a_heartbeat_exactly_at_the_threshold_is_still_healthy() {
        // Poll jitter around the boundary must not read as a hang, or a healthy
        // DCGO gets killed mid-game every time the scheduler slips.
        assert_eq!(classify_heartbeat(Some(30), 30), Health::Healthy);
    }

    #[test]
    fn no_heartbeat_file_is_missing_not_stale() {
        // Distinct because the responses differ: Missing before launch is
        // normal, Missing after launch means DCGO never reached its poll loop.
        assert_eq!(classify_heartbeat(None, 30), Health::Missing);
    }

    #[test]
    fn pid_round_trips() {
        let root = std::env::temp_dir().join("dcgo_daemon_pid_test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(read_pid(&root), None);
        write_pid(&root, 4242).unwrap();
        assert_eq!(read_pid(&root), Some(4242));
        clear_pid(&root).unwrap();
        assert_eq!(read_pid(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_corrupt_pid_file_reads_as_absent() {
        let root = std::env::temp_dir().join("dcgo_daemon_pid_corrupt");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(PID_FILE), "not a pid").unwrap();
        assert_eq!(read_pid(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn up_refuses_a_build_with_a_stale_action_space() {
        let root = std::env::temp_dir().join("dcgo_daemon_gate_root");
        let build = std::env::temp_dir().join("dcgo_daemon_gate_build");
        for d in [&root, &build] {
            let _ = std::fs::remove_dir_all(d);
            std::fs::create_dir_all(d).unwrap();
        }
        let m = crate::manifest::BuildManifest {
            dcgo_commit: "be359bb5b".into(),
            built_at: "2026-08-20T00:00:00Z".into(),
            artifact_sha256: "deadbeef".into(),
            action_space_hash: "0000000000000000".into(),
            executable: "DCGO.exe".into(),
        };
        crate::manifest::save(&build, &m).unwrap();
        std::fs::write(build.join("DCGO.exe"), b"fake").unwrap();

        let err = up(&root, &build).unwrap_err();
        assert!(err.contains("action-space mismatch"), "got: {}", err);
        // The gate must fire before anything is launched.
        assert_eq!(read_pid(&root), None, "must not record a pid when refusing");

        for d in [&root, &build] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn up_reports_a_missing_executable_before_launching() {
        let root = std::env::temp_dir().join("dcgo_daemon_noexe_root");
        let build = std::env::temp_dir().join("dcgo_daemon_noexe_build");
        for d in [&root, &build] {
            let _ = std::fs::remove_dir_all(d);
            std::fs::create_dir_all(d).unwrap();
        }
        let m = crate::manifest::BuildManifest {
            dcgo_commit: "be359bb5b".into(),
            built_at: "2026-08-20T00:00:00Z".into(),
            artifact_sha256: "deadbeef".into(),
            action_space_hash: crate::manifest::action_space_hash(),
            executable: "DCGO.exe".into(),
        };
        crate::manifest::save(&build, &m).unwrap();

        let err = up(&root, &build).unwrap_err();
        assert!(err.contains("DCGO.exe"), "got: {}", err);

        for d in [&root, &build] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn down_on_a_stopped_daemon_is_not_an_error() {
        let root = std::env::temp_dir().join("dcgo_daemon_down_idempotent");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let msg = down(&root).unwrap();
        assert!(msg.contains("not running"), "got: {}", msg);
        let _ = std::fs::remove_dir_all(&root);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p dcgo-harness daemon:: 2>&1 | tail -20
```

Expected: FAIL to compile — `cannot find function classify_heartbeat`.

- [ ] **Step 3: Write the implementation**

Prepend to `code/tools/dcgo-harness/src/daemon.rs`:

```rust
//! Process lifecycle for the DCGO oracle: launch it, tell whether it is
//! actually working, stop it.
//!
//! The health signal is a heartbeat file, not a PID. A hung Unity keeps its
//! process alive and would report healthy forever -- and hung Unity is the
//! failure mode that has actually happened, twice.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest;

/// PID of the launched player, relative to the harness root.
pub const PID_FILE: &str = "harness.pid";
/// Written by JobWatcher every poll. Must match HarnessConfig.HeartbeatPath.
pub const HEARTBEAT_FILE: &str = "harness.heartbeat";
/// A heartbeat older than this means DCGO is hung.
pub const DEFAULT_STALE_SECONDS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Stale { age_seconds: u64 },
    Missing,
}

/// Classify a heartbeat age. `None` means the file does not exist.
pub fn classify_heartbeat(age_seconds: Option<u64>, threshold_seconds: u64) -> Health {
    match age_seconds {
        None => Health::Missing,
        // Inclusive: a heartbeat landing exactly on the threshold is poll
        // jitter, not a hang. Killing on it would reap healthy games.
        Some(age) if age <= threshold_seconds => Health::Healthy,
        Some(age) => Health::Stale { age_seconds: age },
    }
}

/// Seconds since the heartbeat was last written.
pub fn heartbeat_age(root: &Path) -> Option<u64> {
    let meta = std::fs::metadata(root.join(HEARTBEAT_FILE)).ok()?;
    let modified = meta.modified().ok()?;
    modified.elapsed().ok().map(|d| d.as_secs())
}

pub fn read_pid(root: &Path) -> Option<u32> {
    std::fs::read_to_string(root.join(PID_FILE))
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
}

pub fn write_pid(root: &Path, pid: u32) -> Result<(), String> {
    let path = root.join(PID_FILE);
    std::fs::write(&path, pid.to_string())
        .map_err(|e| format!("writing {}: {}", path.display(), e))
}

pub fn clear_pid(root: &Path) -> Result<(), String> {
    match std::fs::remove_file(root.join(PID_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("removing pid file: {}", e)),
    }
}

/// True if a process with this PID currently exists.
pub fn pid_alive(pid: u32) -> bool {
    // `tasklist` is present on every Windows install and needs no crate.
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()),
        Err(_) => false,
    }
}

/// Path to the launchable executable described by a build's manifest.
fn executable_path(build_dir: &Path, m: &manifest::BuildManifest) -> PathBuf {
    build_dir.join(&m.executable)
}

/// Ensure a DCGO oracle is running against `build_dir`, and return a status line.
pub fn up(root: &Path, build_dir: &Path) -> Result<String, String> {
    std::fs::create_dir_all(root).map_err(|e| format!("creating {}: {}", root.display(), e))?;

    let m = manifest::load(build_dir)?;

    // Gate BEFORE anything is launched. A build encoding against a stale action
    // space produces recordings that are corrupt in a way that reads as engine
    // divergence, so running it is worse than not running at all.
    manifest::check_action_space(&m, &manifest::action_space_hash())?;

    let exe = executable_path(build_dir, &m);
    if !exe.exists() {
        return Err(format!(
            "manifest names {} but it does not exist. Rebuild with `dcgo-harness build`.",
            exe.display()
        ));
    }

    if let Some(pid) = read_pid(root) {
        if pid_alive(pid) {
            return Ok(format!(
                "already running (pid {}, heartbeat {:?})",
                pid,
                classify_heartbeat(heartbeat_age(root), DEFAULT_STALE_SECONDS)
            ));
        }
        // A pid file outliving its process is normal after a crash.
        clear_pid(root)?;
    }

    let child = Command::new(&exe)
        .arg("-logFile")
        .arg("-")
        .spawn()
        .map_err(|e| format!("launching {}: {}", exe.display(), e))?;

    let pid = child.id();
    write_pid(root, pid)?;
    Ok(format!(
        "launched {} (pid {}, dcgo {})",
        exe.display(),
        pid,
        &m.dcgo_commit[..m.dcgo_commit.len().min(9)]
    ))
}

/// Stop a running oracle. Not an error if none is running.
pub fn down(root: &Path) -> Result<String, String> {
    let pid = match read_pid(root) {
        Some(p) => p,
        None => return Ok("not running (no pid file)".to_string()),
    };
    if !pid_alive(pid) {
        clear_pid(root)?;
        return Ok(format!("not running (stale pid {} cleared)", pid));
    }
    let out = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .map_err(|e| format!("running taskkill: {}", e))?;
    clear_pid(root)?;
    if out.status.success() {
        Ok(format!("stopped pid {}", pid))
    } else {
        Err(format!(
            "taskkill on pid {} failed: {}",
            pid,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
```

Add `pub mod daemon;` to `code/tools/dcgo-harness/src/lib.rs` after `pub mod build;`.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p dcgo-harness daemon:: 2>&1 | tail -20
```

Expected: `test result: ok. 9 passed`.

- [ ] **Step 5: Wire the subcommands**

In `main.rs`, add to `enum Command`:

```rust
    /// Ensure a DCGO oracle is running against a build.
    Up {
        /// Build directory containing manifest.json.
        #[arg(long)]
        build: PathBuf,
    },
    /// Stop the running DCGO oracle.
    Down,
```

And to the `match` in `run`:

```rust
        Command::Up { build } => {
            println!("{}", dcgo_harness::daemon::up(&args.root, build)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Down => {
            println!("{}", dcgo_harness::daemon::down(&args.root)?);
            Ok(ExitCode::SUCCESS)
        }
```

- [ ] **Step 6: Extend `status` to report process health**

In the `Command::Status` arm of `main.rs`, immediately after the `println!("harness: {}", ...)` line, add:

```rust
            // A queue that is not draining looks identical whether DCGO is
            // stopped, hung, or simply switched off. Say which.
            match dcgo_harness::daemon::read_pid(&args.root) {
                Some(pid) if dcgo_harness::daemon::pid_alive(pid) => {
                    let health = dcgo_harness::daemon::classify_heartbeat(
                        dcgo_harness::daemon::heartbeat_age(&args.root),
                        dcgo_harness::daemon::DEFAULT_STALE_SECONDS,
                    );
                    println!("process: pid {}, heartbeat {:?}", pid, health);
                }
                Some(pid) => println!("process: pid {} recorded but not alive (crashed)", pid),
                None => println!("process: not running"),
            }
```

- [ ] **Step 7: Verify end to end against the real build**

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"; cargo run -p dcgo-harness -- --root "$ROOT" up --build D:/dcgo-build/spike && sleep 30 && cargo run -p dcgo-harness -- --root "$ROOT" status && cargo run -p dcgo-harness -- --root "$ROOT" down
```

Expected: `launched ... (pid N, dcgo ...)`, then `process: pid N, heartbeat Healthy`, then `stopped pid N`.

- [ ] **Step 8: Verify the gate actually refuses**

Temporarily corrupt the stamped hash and confirm `up` refuses:

```bash
python -c "import json; p='D:/dcgo-build/spike/manifest.json'; m=json.load(open(p)); m['action_space_hash']='0'*64; json.dump(m,open(p,'w'),indent=2)" && cargo run -p dcgo-harness -- --root "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" up --build D:/dcgo-build/spike; echo "exit=$?"
```

Expected: `error: action-space mismatch: build ... was stamped 000000000000, engine is now <hash>` and `exit=2`. No process launched.

Then restore it:

```bash
cargo run -p dcgo-harness -- --root "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" build --project "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO" --output D:/dcgo-build/spike
```

- [ ] **Step 9: Commit**

```bash
git add code/tools/dcgo-harness/src/daemon.rs code/tools/dcgo-harness/src/lib.rs code/tools/dcgo-harness/src/main.rs && git commit -m "dcgo-harness: up/down process lifecycle with heartbeat health"
```

---

### Task 6: Editor-vs-player determinism acceptance, and docs

This is the acceptance gate for the whole layer. A player that launches but plays *differently* from the Editor makes the oracle disagree with itself, and every layer above assumes they are the same program.

**Files:**
- Create: `qa/dcgo-harness/determinism-acceptance.md`
- Modify: `docs/DCGO_HARNESS.md`

**Interfaces:**
- Consumes: `dcgo-harness build` (Task 3), `up` / `down` (Task 5), the existing `submit` / `enable` / `disable` commands.
- Produces: a recorded verdict on whether the player build is usable as the oracle.

- [ ] **Step 1: Queue one fixed-seed job**

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"; cargo run -p dcgo-harness -- --root "$ROOT" submit --count 1 --decks qa/dcgo-harness/pool.json --seed 424242
```

Expected: `submitted 1 job(s)`. If `pool.json` does not exist at that path, locate the deck pool used for the phase-1 golden smoke job and use it — the seed matters, the decks only have to be identical across both runs.

- [ ] **Step 2: Run it in the Editor**

Enable the harness, press Play in the Unity Editor, wait for the job to complete, then stop Play.

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"; cargo run -p dcgo-harness -- --root "$ROOT" status
```

Expected: `completed=1`.

- [ ] **Step 3: Preserve the Editor recording**

```bash
CORPUS="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings"; mkdir -p D:/dcgo-build/acceptance && cp "$(ls -t "$CORPUS"/*.jsonl | head -1)" D:/dcgo-build/acceptance/editor.jsonl && wc -l D:/dcgo-build/acceptance/editor.jsonl
```

Expected: a nonzero line count. Zero rows means the game never started — check `RecorderConfig.FlushEveryNRows` is still 1.

- [ ] **Step 4: Run the same seed in the player**

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"; cargo run -p dcgo-harness -- --root "$ROOT" submit --count 1 --decks qa/dcgo-harness/pool.json --seed 424242 && cargo run -p dcgo-harness -- --root "$ROOT" up --build D:/dcgo-build/spike
```

Wait for `status` to report `completed=1`, then:

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"; cargo run -p dcgo-harness -- --root "$ROOT" down
```

- [ ] **Step 5: Diff the two recordings**

```bash
CORPUS="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings"; cp "$(ls -t "$CORPUS"/*.jsonl | head -1)" D:/dcgo-build/acceptance/player.jsonl && python code/tools/dcgo-harness/scripts/strip_volatile.py D:/dcgo-build/acceptance/editor.jsonl D:/dcgo-build/acceptance/editor.norm.jsonl && python code/tools/dcgo-harness/scripts/strip_volatile.py D:/dcgo-build/acceptance/player.jsonl D:/dcgo-build/acceptance/player.norm.jsonl && diff D:/dcgo-build/acceptance/editor.norm.jsonl D:/dcgo-build/acceptance/player.norm.jsonl && echo "IDENTICAL"
```

Create `code/tools/dcgo-harness/scripts/strip_volatile.py` first (`jq` is not
installed on this machine):

```python
"""Drop wall-clock fields so two runs of the same seed can be compared.

Only `timestamp` and `recording_id` are removed -- they are facts about when a
game was recorded, not about the game. Do NOT widen this list: anything else
differing between an Editor run and a player run of the same seed is a real
divergence, and excluding it would hide exactly what this check exists to find.
"""
import json
import sys

VOLATILE = ("timestamp", "recording_id")

with open(sys.argv[1], encoding="utf-8") as src, open(sys.argv[2], "w", encoding="utf-8") as dst:
    for line in src:
        line = line.strip()
        if not line:
            continue
        row = json.loads(line)
        for key in VOLATILE:
            row.pop(key, None)
        dst.write(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n")
```

Expected: `IDENTICAL`.

- [ ] **Step 6: Record the verdict**

Create `qa/dcgo-harness/determinism-acceptance.md`:

```markdown
# Editor-vs-player determinism acceptance

The oracle must be one program. A player that launches but plays differently
from the Editor makes DCGO disagree with itself, and Layers B and C both assume
a divergence means our engine is wrong.

| Field | Value |
|---|---|
| Date | <fill in> |
| DCGO commit | <from manifest.json> |
| Build | `D:\dcgo-build\spike` |
| Seed | 424242 |
| Editor rows | <n> |
| Player rows | <n> |
| Verdict | IDENTICAL / DIVERGED |

## If DIVERGED

Record the first differing row verbatim and stop. Do not proceed to Layer B.
The likely cause is asset load order reaching `RandomUtility.ShuffledDeckCards`
differently in a player than in the Editor, which would make every stacked-deck
probe unreproducible.
```

Fill in the real values from the run.

- [ ] **Step 7: Document the new commands**

In `docs/DCGO_HARNESS.md`, replace the "Enabling it" section's opening with a new "Running it unattended" section above it:

```markdown
## Running it unattended

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"

# Build the player once (slow; shader compilation dominates)
dcgo-harness --root "$ROOT" build --project "$BASE_DCGO" --output D:/dcgo-build/v1

# Start it, queue work, watch, stop
dcgo-harness --root "$ROOT" up --build D:/dcgo-build/v1
dcgo-harness --root "$ROOT" enable
dcgo-harness --root "$ROOT" submit --count 200 --decks pool.json --seed 1
dcgo-harness --root "$ROOT" status      # includes process pid + heartbeat
dcgo-harness --root "$ROOT" down
```

`up` refuses to launch a build whose stamped `action_space_hash` no longer
matches the engine's. That build would encode actions against a stale space and
every recording it produced would be corrupt *in a way that reads as engine
divergence* — the frame-ID-vs-compact-index failure again, but versioned.
Rebuild rather than working around it.

Health is a heartbeat file, not a PID: a hung Unity keeps its process alive and
would otherwise report healthy forever. Both hangs seen so far (the unleft
Photon room, the stalled selection) looked exactly like that.

`HarnessConfig.ExitAfterIdleSeconds` defaults to 0 (never). Set it low for a
one-shot subprocess; leave it off for the warm daemon and for Editor sessions.
```

- [ ] **Step 8: Commit**

```bash
git add qa/dcgo-harness/determinism-acceptance.md docs/DCGO_HARNESS.md && git commit -m "dcgo-harness: Editor-vs-player determinism acceptance and unattended docs"
```

---

## Self-review notes

**Spec coverage.** Every Layer A requirement maps to a task: build script + headless invocation (T1), manifest with commit/SHA/action-space hash (T2), `build` orchestration (T3), `ExitAfterIdleSeconds` + heartbeat (T4), `up`/`down` + gate + health (T5), Editor-vs-player acceptance + docs (T6).

**Deliberately deferred to Layer B or beyond**, and not gaps in this plan:

- **Bounded restart and requeue-on-stale.** The spec describes killing a hung DCGO, requeueing its claimed job, and quarantining after two. T5 delivers the *detection* (`classify_heartbeat`) and `status` reports it, but nothing acts on it automatically. Automatic restart needs a supervising loop with nowhere natural to live yet — `up` returns immediately. It belongs with the probe runner in Layer B, which is the first caller that needs to wait for a job. Until then a stale heartbeat is reported and the operator runs `down` + `up`.
- **The Editor fallback path in `up`.** Only written if T1's gate fails. Writing both paths speculatively would mean shipping one that never runs.

**Known risk carried into T1:** the Unity license in batchmode. It cannot be verified without running the build, and it fails as a silent hang rather than an error, which is why T4's guidance says not to call the build hung before 45 minutes.
