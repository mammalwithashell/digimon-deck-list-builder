# Oracle Node Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a fresh machine into a working DCGO oracle node with one command, and let an agent on that machine know — before it spends a token on authoring — whether the oracle can actually answer.

**Architecture:** `dcgo-harness node up|down|status` wraps the existing daemon with a preflight that refuses to start on a stale build. The image is the repo plus three artifacts (~550 MB) copied from your build machine; no Unity, no license, no LFS checkout. A `node_health` MCP tool exposes the same preflight to the agent.

**Tech Stack:** Rust 2021 (`code/tools/dcgo-harness`), PowerShell + bash (image scripts), Markdown (runbook).

**Spec:** `docs/superpowers/specs/2026-08-27-archetype-campaign-fleet-design.md` §5.

**Prerequisites:** the ledger plan (for `exam-verdicts/`) and the MCP plan (Task 4 here adds a tool to that server).

## What makes this possible (measured, not assumed)

- **The built player is 492 MB** (`D:\dcgo-build\scripted-v7\`: `DCGO.exe` + `DCGO_Data` + `UnityPlayer.dll`). The multi-GB licensed thing is the Unity *project* (4.3 GB), not the artifact.
- **Running a player needs no Unity license.** Only building does. Build once on your machine; run anywhere.
- **DCGO's C# source is 53 MB** and the two rules PDFs are **~1 MB** (`general_rule.pdf` 975 KB, `glossary.pdf` 53 KB; `manual.pdf` is 52 MB of UI reference and is not needed). So a node carries everything source-priority #1 and #2 require for ~54 MB more.
- **Agent tokens dominate infrastructure by 20–50×** — the first campaign cost $4,210 in tokens against $30–150/month for a VM. Cold starts are paid in tokens, so **warm nodes are the cheap option**; this plan optimizes for "GO in one command", not for minimal footprint.

## Global Constraints

- **Per-worktree `CARGO_TARGET_DIR`** (CLAUDE.md rule 31). A compile error in a file you did not touch means target contamination.
- **DCGO lives in the base repo, never a worktree** (rule 29). Resolve it as `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`. **Never** run `git submodule update --init DCGO` from a worktree. The base DCGO checkout carries ~8,349 pre-existing dirty asset files: never `git add -A` there.
- **Build output goes outside the submodule** (`D:\dcgo-build\`). Nothing generated may land where git can see it.
- **The rules PDFs are git-ignored and stay that way** (rule 32). They go in the **image**, not the repo.
- **The action-space hash gate is not optional.** A player whose `action_space_hash` disagrees with `code/digimon-engine/src/action/space.rs` encodes against a dead space, and its recordings read as engine divergence rather than as a broken tool. `up` already refuses; nothing here may weaken that.
- **`dcgo-harness` is dev/test tooling** — never imported by `server.*` or `digimon_gym.*`, never in a production build.

## Known platform limits — read before starting

Two things are **unverified** and Task 1 is where they get settled:

1. **The player is launched with only `-logFile <path>`** (`daemon.rs`'s `player_command`) — no `-batchmode`, no `-nographics`. Whether it tolerates them, or needs an attached desktop session, is untested. On a headless Windows VM this is the difference between "any cloud box" and "a box with a virtual display driver".
2. **Photon.** Each node holds a connection and a private one-seat room against the app id baked into the build (`JobWatcher.LoadBattleSceneWhenPhotonReady`). N nodes = N concurrent CCU. The ceiling is unmeasured.

Neither blocks a **local** node, which is the first deliverable. Task 1 measures both and records the answer so nobody re-derives it.

## File Structure

| File | Responsibility |
|---|---|
| `code/tools/dcgo-harness/src/node.rs` (create) | Preflight checks + `up`/`down`/`status` orchestration. |
| `code/tools/dcgo-harness/src/lib.rs` (modify) | `pub mod node;` |
| `code/tools/dcgo-harness/src/main.rs` (modify) | `Node { … }` subcommand. |
| `code/tools/dcgo-harness/src/mcp/handlers.rs` (modify) | `node_health` tool. |
| `code/tools/dcgo-harness/src/mcp/tools.rs` (modify) | `node_health` descriptor. |
| `scripts/build-oracle-node.sh` (create) | Assemble the ~550 MB image payload from a build machine. |
| `docs/runbooks/oracle-node.md` (create) | Provision, refresh, and troubleshoot a node. |
| `qa/dcgo-harness/node-platform-findings.md` (create) | Task 1's measured answers to the two unknowns. |

---

### Task 1: Settle the two platform unknowns, and write down the answers

**Files:**
- Create: `qa/dcgo-harness/node-platform-findings.md`
- Modify: `code/tools/dcgo-harness/src/daemon.rs` (only if a flag change is warranted by what you measure)

**Interfaces:**
- Consumes: an existing build under `D:\dcgo-build\` and the current `daemon.rs`.
- Produces: a findings document later tasks and the runbook cite. **No API.**

This task is **measurement, not construction**. Its output is a document. Do not skip it because it produces no code: Tasks 2–4 and the whole image recipe assume answers that nobody currently has, and guessing them wrong means shipping a runbook that cannot work on the machine it targets.

- [ ] **Step 1: Establish the baseline — does the player run at all here?**

```bash
ls -la /d/dcgo-build/ | tail -5
cat /d/dcgo-build/scripted-v7/manifest.json
```

Record the newest build directory name and its `action_space_hash`. Then confirm the gate agrees with the current engine:

```bash
cargo run -q -p dcgo-harness -- up --build /d/dcgo-build/scripted-v7 2>&1 | tail -5
```

Expected: either a launch, or a **refusal naming an action-space mismatch**. A refusal is a *valid* outcome to record — it means the build predates a `space.rs` change and the fleet needs a rebuild. Either way, `down` afterwards:

```bash
cargo run -q -p dcgo-harness -- down 2>&1 | tail -3
```

- [ ] **Step 2: Test whether the player tolerates headless flags**

Launch the player directly, bypassing the harness, with each flag combination. Record for each: does the process stay alive past 30 seconds, and does its log show it reaching the menu?

```bash
BUILD=/d/dcgo-build/scripted-v7
"$BUILD/DCGO.exe" -logFile /tmp/dcgo-plain.log &
sleep 30; tasklist | grep -i DCGO || echo "EXITED"; taskkill //IM DCGO.exe //F 2>/dev/null

"$BUILD/DCGO.exe" -logFile /tmp/dcgo-nographics.log -batchmode -nographics &
sleep 30; tasklist | grep -i DCGO || echo "EXITED"; taskkill //IM DCGO.exe //F 2>/dev/null
```

Then read the tail of each log. A Unity player that cannot create a graphics device says so explicitly.

**Interpretation, and be careful here:** "the process is alive" is necessary but not sufficient — a player can survive `-nographics` and still never drive its UI coroutines, which is exactly what the harness depends on. If `-nographics` keeps the process alive, verify it can actually *play* by queueing one job and seeing whether it drains:

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
cargo run -q -p dcgo-harness -- --root "$ROOT" submit --count 1 --decks qa/dcgo-exams/EX12/toho_pool.json --seed 1
cargo run -q -p dcgo-harness -- --root "$ROOT" enable
# launch with the flags under test, then:
cargo run -q -p dcgo-harness -- --root "$ROOT" status
```

A job that moves `jobs/ → claimed/ → done/` under `-nographics` is the only proof that matters.

- [ ] **Step 3: Measure the Photon ceiling, or record that you could not**

Start two players simultaneously against the same build and queue jobs to both. Record whether both connect, or whether the second fails on a lobby/room error.

If you cannot run two concurrently on this machine, **say so explicitly in the findings** rather than inferring a number. An unmeasured ceiling recorded as a guess is worse than one recorded as unknown — the fleet-sizing decision would then rest on a fabrication.

- [ ] **Step 4: Write the findings**

Create `qa/dcgo-harness/node-platform-findings.md`:

```markdown
# Oracle node — platform findings

**Measured:** <date> · **Build:** `<build dir>` (`dcgo_commit <hash>`, `action_space_hash <hash>`)
**Machine:** <OS version, GPU present y/n, session type: console / RDP / headless>

## Does the player run headless?

| Flags | Process alive at 30s | Reached menu (log) | Drained a job |
|---|---|---|---|
| `-logFile` only | | | |
| `-logFile -batchmode -nographics` | | | |

**Verdict:** <one of: nographics works / needs an attached desktop / needs a virtual display>

**Evidence:** <the log line that decided it, quoted>

## Photon concurrency

**Measured ceiling:** <N concurrent players, or "not measured — reason">

Each node holds one connection and one private one-seat room
(`JobWatcher.LoadBattleSceneWhenPhotonReady`), so N nodes = N concurrent CCU
against the app id baked into the build.

## What this means for a remote node

<Concrete consequence: e.g. "a Windows VM needs the Desktop Experience and an
active console session; an RDP session that disconnects tears down the desktop
unless redirected with tscon", or "-nographics is sufficient, any headless
Windows VM works".>
```

Fill every cell. A blank cell is an unanswered question that Task 3's runbook would then guess at.

- [ ] **Step 5: Commit**

```bash
git add qa/dcgo-harness/node-platform-findings.md
git commit -m "qa: measure whether the DCGO player runs headless, and the Photon ceiling

Both were assumptions the fleet design rested on and neither had been tested.
The runbook and the image recipe cite this document rather than re-deriving it,
and an unmeasured Photon ceiling is recorded as unmeasured rather than guessed."
```

---

### Task 2: `node` preflight and lifecycle

**Files:**
- Create: `code/tools/dcgo-harness/src/node.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs`, `code/tools/dcgo-harness/src/main.rs`
- Test: inline `#[cfg(test)]` in `node.rs`

**Interfaces:**
- Consumes: `manifest::{load, action_space_hash, check_action_space}`, `daemon::{up, down, read_pid, pid_alive, heartbeat_age, classify_heartbeat}` — read `daemon.rs` and reuse; do not re-implement process handling.
- Produces:
  ```rust
  pub enum CheckStatus { Ok, Warn, Fail }
  pub struct Check { pub name: String, pub status: CheckStatus, pub detail: String, pub remedy: Option<String> }
  pub struct Health { pub go: bool, pub checks: Vec<Check> }
  pub fn health(root: &Path, build: Option<&Path>) -> Health;
  pub fn up(root: &Path, build: &Path) -> Result<String, String>;
  pub fn down(root: &Path) -> Result<String, String>;
  pub fn status(root: &Path, build: Option<&Path>) -> Result<String, String>;
  ```

**Design rule:** `health` **never returns `Err`**. A node that cannot answer must produce a *readable report*, not an error string — the agent's next move depends on which check failed, and `Err(String)` collapses that to one line. `go` is true only when no check is `Fail`.

- [ ] **Step 1: Write the failing tests**

Create `code/tools/dcgo-harness/src/node.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, action_space_hash: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::json!({
                "dcgo_commit": "f6f726088",
                "built_at": "2026-08-22T19:14:11.416640+00:00",
                "artifact_sha256": "b838f60c",
                "action_space_hash": action_space_hash,
                "executable": "DCGO.exe",
            })
            .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn health_reports_every_check_even_when_one_fails() {
        // An agent's next move depends on WHICH check failed, so the report
        // must survive a failure rather than collapsing to one error string.
        let root = std::env::temp_dir().join("node_health_all_checks");
        let build = std::env::temp_dir().join("node_health_all_checks_build");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&build);
        write_manifest(&build, "deadbeef-not-the-current-hash");

        let h = health(&root, Some(&build));

        assert!(!h.go, "a stale action space must not report GO");
        assert!(h.checks.len() >= 3, "every check reports, not just the first failure");
        assert!(
            h.checks.iter().any(|c| c.name == "action_space" && matches!(c.status, CheckStatus::Fail)),
            "the stale hash must be the named failure: {:?}",
            h.checks
        );
    }

    #[test]
    fn a_failing_check_carries_a_remedy() {
        let root = std::env::temp_dir().join("node_health_remedy");
        let build = std::env::temp_dir().join("node_health_remedy_build");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&build);
        write_manifest(&build, "stale");

        let h = health(&root, Some(&build));
        for c in h.checks.iter().filter(|c| matches!(c.status, CheckStatus::Fail)) {
            assert!(
                c.remedy.as_deref().is_some_and(|r| !r.is_empty()),
                "check {:?} fails without telling anyone what to do",
                c.name
            );
        }
    }

    #[test]
    fn a_missing_build_is_a_named_failure_not_a_panic() {
        let root = std::env::temp_dir().join("node_health_nobuild");
        let _ = std::fs::remove_dir_all(&root);
        let h = health(&root, Some(Path::new("does/not/exist")));
        assert!(!h.go);
        assert!(h.checks.iter().any(|c| c.name == "build" && matches!(c.status, CheckStatus::Fail)));
    }

    #[test]
    fn health_without_a_build_path_still_reports_the_other_checks() {
        let root = std::env::temp_dir().join("node_health_nobuildarg");
        let _ = std::fs::remove_dir_all(&root);
        let h = health(&root, None);
        assert!(h.checks.iter().any(|c| c.name == "harness_enabled"));
    }

    #[test]
    fn go_is_false_when_any_check_fails() {
        let root = std::env::temp_dir().join("node_health_go_false");
        let _ = std::fs::remove_dir_all(&root);
        let h = health(&root, Some(Path::new("does/not/exist")));
        assert!(!h.go);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib node`

Expected: FAIL — `cannot find function 'health'`.

- [ ] **Step 3: Implement**

Prepend to `node.rs`:

```rust
//! Node lifecycle: is this machine able to answer as an oracle, and start/stop
//! the player that does it.
//!
//! The preflight exists to fail **before** an agent spends tokens. Authoring a
//! scenario costs real money; discovering afterwards that the player was never
//! going to launch wastes all of it. So `health` is cheap, runs first, and
//! reports every check rather than the first failure — the agent's next move
//! depends on WHICH check failed.
//!
//! `health` deliberately never returns `Err`: a node that cannot answer must
//! produce a readable report, not an error string.

use std::path::Path;

use crate::{daemon, manifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    /// What to do about it. Required on every `Fail`.
    pub remedy: Option<String>,
}

impl Check {
    fn ok(name: &str, detail: String) -> Check {
        Check { name: name.into(), status: CheckStatus::Ok, detail, remedy: None }
    }
    fn warn(name: &str, detail: String, remedy: &str) -> Check {
        Check { name: name.into(), status: CheckStatus::Warn, detail, remedy: Some(remedy.into()) }
    }
    fn fail(name: &str, detail: String, remedy: &str) -> Check {
        Check { name: name.into(), status: CheckStatus::Fail, detail, remedy: Some(remedy.into()) }
    }
}

#[derive(Debug, Clone)]
pub struct Health {
    /// True only when no check failed.
    pub go: bool,
    pub checks: Vec<Check>,
}

impl Health {
    pub fn describe(&self) -> String {
        let mut out = String::new();
        out.push_str(if self.go { "GO\n" } else { "NO-GO\n" });
        for c in &self.checks {
            out.push_str(&format!("  [{}] {}: {}\n", c.status.as_str(), c.name, c.detail));
            if let Some(r) = &c.remedy {
                out.push_str(&format!("        -> {r}\n"));
            }
        }
        out
    }
}

/// Run every preflight check. Never returns `Err` — see the module docs.
pub fn health(root: &Path, build: Option<&Path>) -> Health {
    let mut checks = Vec::new();

    // 1/2. The build and its action-space gate.
    match build {
        None => checks.push(Check::warn(
            "build",
            "no --build given; the action-space gate was not checked".into(),
            "pass --build <dir> to check the player this node would run",
        )),
        Some(dir) if !dir.exists() => checks.push(Check::fail(
            "build",
            format!("{} does not exist", dir.display()),
            "copy the player image to this machine, or pass the right --build path",
        )),
        Some(dir) => match manifest::load(dir) {
            Err(e) => checks.push(Check::fail(
                "build",
                format!("unreadable manifest in {}: {e}", dir.display()),
                "rebuild with `dcgo-harness build`, or re-copy the image",
            )),
            Ok(m) => {
                let exe = dir.join(&m.executable);
                if exe.exists() {
                    checks.push(Check::ok(
                        "build",
                        format!("{} (dcgo_commit {})", exe.display(), m.dcgo_commit),
                    ));
                } else {
                    checks.push(Check::fail(
                        "build",
                        format!("manifest names {} but it is missing", exe.display()),
                        "re-copy the image; the payload is incomplete",
                    ));
                }

                let current = manifest::action_space_hash();
                match manifest::check_action_space(&m, &current) {
                    Ok(()) => checks.push(Check::ok(
                        "action_space",
                        format!("matches the engine ({})", &current[..12.min(current.len())]),
                    )),
                    Err(e) => checks.push(Check::fail(
                        "action_space",
                        e,
                        "this player encodes against a DEAD action space and its recordings \
                         would read as engine divergence. Rebuild on the build machine \
                         (`dcgo-harness build`) and redistribute the image.",
                    )),
                }
            }
        },
    }

    // 3. Is the harness enabled? A disabled harness is indistinguishable from
    //    a hung DCGO once jobs are queued, which is why it gets its own check.
    let marker = root.join("harness.enabled");
    if marker.exists() {
        checks.push(Check::ok("harness_enabled", format!("{} present", marker.display())));
    } else {
        checks.push(Check::fail(
            "harness_enabled",
            format!("{} missing: DCGO will ignore the queue", marker.display()),
            "run `dcgo-harness --root <root> enable`",
        ));
    }

    // 4. Queue directories.
    let missing: Vec<&str> = ["jobs", "claimed", "done", "failed"]
        .into_iter()
        .filter(|d| !root.join(d).exists())
        .collect();
    if missing.is_empty() {
        checks.push(Check::ok("queue", format!("{} has jobs/claimed/done/failed", root.display())));
    } else {
        checks.push(Check::warn(
            "queue",
            format!("missing {missing:?} under {}", root.display()),
            "they are created on first submit; harmless on a fresh node",
        ));
    }

    // 5. Is a player already running, and is its heartbeat fresh?
    match daemon::read_pid(root) {
        Some(pid) if daemon::pid_alive(pid) => {
            let age = daemon::heartbeat_age(root);
            checks.push(Check::ok(
                "player",
                format!("running (pid {pid}, heartbeat {:?})",
                        daemon::classify_heartbeat(age, daemon::DEFAULT_STALE_SECONDS)),
            ));
        }
        Some(pid) => checks.push(Check::warn(
            "player",
            format!("pid file names {pid} but no such process (normal after a crash)"),
            "`dcgo-harness node up` will relaunch",
        )),
        None => checks.push(Check::warn(
            "player",
            "not running".into(),
            "`dcgo-harness node up --build <dir>` starts it",
        )),
    }

    let go = !checks.iter().any(|c| matches!(c.status, CheckStatus::Fail));
    Health { go, checks }
}

/// Preflight, then start the oracle. Refuses on any failing check.
pub fn up(root: &Path, build: &Path) -> Result<String, String> {
    let h = health(root, Some(build));
    if !h.go {
        return Err(format!(
            "node preflight says NO-GO; not starting the oracle.\n{}",
            h.describe()
        ));
    }
    let started = daemon::up(root, build)?;
    Ok(format!("{}\n{}", h.describe(), started))
}

pub fn down(root: &Path) -> Result<String, String> {
    daemon::down(root)
}

pub fn status(root: &Path, build: Option<&Path>) -> Result<String, String> {
    Ok(health(root, build).describe())
}
```

If any `daemon::` item used above is private, make it `pub(crate)` rather than duplicating it here.

Register `pub mod node;` in `lib.rs`, and add to `main.rs`'s flat `Commands` enum:

```rust
    /// Bring this machine up as an oracle node: preflight, then launch.
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
```

```rust
#[derive(Subcommand, Debug)]
enum NodeAction {
    /// Preflight and start the oracle.
    Up {
        #[arg(long)]
        build: PathBuf,
    },
    /// Stop the oracle.
    Down,
    /// Report readiness without changing anything.
    Status {
        #[arg(long)]
        build: Option<PathBuf>,
    },
}
```

with match arms that resolve the root via the existing `require_root` helper and print the returned string. `node status` must exit non-zero when `go` is false, so CI and scripts can gate on it.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness --lib node`

Expected: PASS — 5 tests.

- [ ] **Step 5: Run the real preflight on this machine**

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
cargo run -q -p dcgo-harness -- --root "$ROOT" node status --build /d/dcgo-build/scripted-v7
```

Paste the output in your report. Either GO or NO-GO is a valid result — what matters is that every check prints and each failure carries a remedy.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/node.rs code/tools/dcgo-harness/src/lib.rs code/tools/dcgo-harness/src/main.rs
git commit -m "node: a preflight that fails before an agent spends tokens

Authoring a scenario costs real money; finding out afterwards that the player
was never going to launch wastes all of it. health() runs every check rather
than stopping at the first, because the agent's next move depends on WHICH one
failed -- and it never returns Err, since an error string collapses exactly the
detail that makes the report actionable.

Every failing check carries a remedy, enforced by a test."
```

---

### Task 3: The image recipe and the runbook

**Files:**
- Create: `scripts/build-oracle-node.sh`, `docs/runbooks/oracle-node.md`
- Modify: `docs/INDEX.md`

**Interfaces:**
- Consumes: Task 1's findings, Task 2's `node` subcommand.
- Produces: a script that assembles the payload, and a runbook that uses it.

- [ ] **Step 1: Write the payload script**

Create `scripts/build-oracle-node.sh`:

```bash
#!/usr/bin/env bash
# Assemble an oracle-node payload from THIS (build) machine.
#
# A node needs four things and none of them is Unity:
#   1. the built player            (~492 MB) -- running it needs no licence
#   2. DCGO's C# source            (~53 MB)  -- source priority #2, for triage
#   3. general_rule.pdf + glossary (~1 MB)   -- source priority #1
#   4. the repo itself                        -- cloned separately on the node
#
# The 4.3 GB figure people remember is the Unity PROJECT, not the artifact.
# The PDFs are git-ignored by design (CLAUDE.md rule 32): they belong in the
# image, never in the repo.
set -euo pipefail

BUILD_DIR="${1:-}"
OUT="${2:-./oracle-node-payload}"

if [[ -z "$BUILD_DIR" ]]; then
    echo "usage: $0 <build-dir> [out-dir]" >&2
    echo "  e.g. $0 /d/dcgo-build/scripted-v7 /d/oracle-node-payload" >&2
    exit 2
fi
if [[ ! -f "$BUILD_DIR/manifest.json" ]]; then
    echo "no manifest.json in $BUILD_DIR -- is that a dcgo-harness build?" >&2
    exit 1
fi

# Rule 29: DCGO lives in the base repo. Never init it in a worktree.
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
BASE_DCGO="$BASE/DCGO"
RULES="$BASE/Digimon TCG resources"

for required in "$BASE_DCGO/Assets/Scripts" "$RULES/general_rule.pdf"; do
    if [[ ! -e "$required" ]]; then
        echo "missing $required -- run this on the BUILD machine (the base repo)," >&2
        echo "not in a worktree, where DCGO is an intentionally-empty placeholder." >&2
        exit 1
    fi
done

mkdir -p "$OUT"

echo "==> player"
cp -r "$BUILD_DIR" "$OUT/player"

echo "==> DCGO C# source (scripts only -- no art, no LFS)"
mkdir -p "$OUT/dcgo-src/Assets"
cp -r "$BASE_DCGO/Assets/Scripts" "$OUT/dcgo-src/Assets/Scripts"

echo "==> rules PDFs"
mkdir -p "$OUT/rules"
cp "$RULES/general_rule.pdf" "$OUT/rules/"
cp "$RULES/glossary.pdf" "$OUT/rules/" 2>/dev/null || echo "    (glossary.pdf absent; continuing)"
# manual.pdf is 52 MB of UI reference and is deliberately NOT shipped.

cat > "$OUT/MANIFEST.txt" <<EOF
oracle-node payload
built_from : $BUILD_DIR
dcgo_commit: $(python -c "import json,sys;print(json.load(open(sys.argv[1]))['dcgo_commit'])" "$BUILD_DIR/manifest.json")
action_space_hash: $(python -c "import json,sys;print(json.load(open(sys.argv[1]))['action_space_hash'])" "$BUILD_DIR/manifest.json")
contents   : player/ dcgo-src/Assets/Scripts rules/

The action_space_hash above pins this payload to one engine revision. If
code/digimon-engine/src/action/space.rs changes, this player encodes against a
dead space and \`node up\` will refuse it: rebuild and redistribute.
EOF

echo
du -sh "$OUT"/* 2>/dev/null || true
echo
echo "payload ready: $OUT"
echo "next: copy it to the node, then see docs/runbooks/oracle-node.md"
```

Make it executable: `chmod +x scripts/build-oracle-node.sh`.

- [ ] **Step 2: Run it for real and record the sizes**

```bash
bash scripts/build-oracle-node.sh /d/dcgo-build/scripted-v7 /d/oracle-node-payload
```

Expected: roughly `player 492M`, `dcgo-src 53M`, `rules 1M`. Paste the real `du` output into your report — the runbook quotes these numbers and they must not be invented.

Then delete the payload (it is multi-hundred-MB derived data): `rm -rf /d/oracle-node-payload`.

- [ ] **Step 3: Write the runbook**

Create `docs/runbooks/oracle-node.md` covering, in this order:

1. **What a node is and is not** — it runs a prebuilt player; it does not build one, needs no Unity licence, and never clones the 4.3 GB project.
2. **Payload** — the script, the three artifacts, real sizes from Step 2.
3. **Provisioning** — clone the repo, drop the payload, install the Rust toolchain, `cargo build -p dcgo-harness`, place the rules PDFs where rule 32's resolution expects them, then `node status --build <dir>` until GO.
4. **Running** — `node up`, `watch`, `node down`.
5. **The action-space rule, stated as an operational fact:** changing `space.rs` invalidates every node's player at once; `node up` refuses rather than producing corrupt recordings. Rebuild on the build machine, redistribute, restart. This is a chore, not a hazard.
6. **Platform requirements** — cite `qa/dcgo-harness/node-platform-findings.md` for the headless/display answer and the Photon ceiling. Do not restate the numbers; point at the measurement so there is one copy.
7. **Troubleshooting** — carry across the hard-won ones from `docs/DCGO_HARNESS.md`: a disabled harness looks exactly like a hung DCGO; stop Play before requeueing; DCGO's AI mode is not offline (it needs Photon); a stale `cards_behavioral` process holds the build lock and makes every later cargo command look hung.
8. **Cost note** — a warm node is $30–150/month against ~$4,210 of agent tokens for one archetype campaign. Cold starts are paid in tokens. Keep nodes warm.

- [ ] **Step 4: Register the runbook**

Add a line to `docs/INDEX.md` beside the other runbooks.

- [ ] **Step 5: Verify the runbook against reality**

Follow your own runbook's provisioning section on this machine, in order, and fix anything that does not work as written. A runbook nobody has executed is a draft.

Report which steps you executed and which you could not (e.g. "no second machine available, so the copy step was simulated locally").

- [ ] **Step 6: Commit**

```bash
git add scripts/build-oracle-node.sh docs/runbooks/oracle-node.md docs/INDEX.md
git commit -m "node: the ~550 MB image recipe and its runbook

A node needs the player, DCGO's C# for triage, and the two rules PDFs -- not
Unity, not a licence, not the 4.3 GB project. The PDFs stay git-ignored and
ride in the image, per rule 32.

The payload manifest records the action_space_hash it was built against,
because that hash is what pins a player to an engine revision and what \`node
up\` refuses on."
```

---

### Task 4: `node_health` on the agent surface

**Files:**
- Modify: `code/tools/dcgo-harness/src/mcp/tools.rs`, `code/tools/dcgo-harness/src/mcp/handlers.rs`
- Test: inline `#[cfg(test)]` in `handlers.rs`

**Interfaces:**
- Consumes: `node::{health, Health, Check, CheckStatus}` (Task 2).
- Produces: the `node_health` tool.

- [ ] **Step 1: Write the failing test**

Add to `handlers.rs`'s `mod tests`:

```rust
    #[test]
    fn node_health_reports_go_and_every_check() {
        let dir = std::env::temp_dir().join("mcp_node_health");
        let _ = std::fs::remove_dir_all(&dir);
        let params = json!({"arguments": {"build": "does/not/exist"}});
        let out = node_health(&params, Some(&dir)).expect("health never errors");

        assert_eq!(out["go"], json!(false));
        let checks = out["checks"].as_array().expect("checks array");
        assert!(checks.len() >= 3, "every check reports, not just the first failure");
        assert!(
            checks.iter().any(|c| c["status"] == json!("fail") && c["remedy"].is_string()),
            "a failing check must tell the agent what to do: {checks:?}"
        );
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib mcp::handlers`

Expected: FAIL — `cannot find function 'node_health'`.

- [ ] **Step 3: Implement**

Add the descriptor to `tools::list()` (keep the list's stable order — append after `release`):

```rust
        json!({
            "name": "node_health",
            "description": "Is this machine able to answer as an oracle? Reports every \
                preflight check with a remedy: the player, the action-space gate, whether the \
                harness is enabled, the queue, and whether a player is already running. Run \
                this BEFORE authoring -- a NO-GO discovered afterwards wastes the authoring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "build": {"type": "string", "description": "Player build directory"}
                }
            }
        }),
```

Add to `dispatch`:

```rust
        "node_health" => node_health(params, root),
```

```rust
pub fn node_health(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let root = root
        .map(|r| r.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let build = tools::opt_str_arg(params, "build").map(std::path::PathBuf::from);

    // node::health never fails -- a node that cannot answer must produce a
    // readable report, not an error string.
    let h = crate::node::health(&root, build.as_deref());
    Ok(serde_json::json!({
        "go": h.go,
        "checks": h.checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "status": c.status.as_str(),
            "detail": c.detail,
            "remedy": c.remedy,
        })).collect::<Vec<_>>(),
    }))
}
```

Also update `tools.rs`'s `EXPECTED` test constant to include `node_health`, so the "every tool is listed" test covers it.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness`

Expected: PASS, whole crate.

- [ ] **Step 5: Call it through the server**

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"node_health","arguments":{"build":"/d/dcgo-build/scripted-v7"}}}' \
  | cargo run -q -p dcgo-harness -- mcp | head -c 700
```

Expected: a response containing `"go":` and a `checks` array. Paste it in your report.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/mcp/tools.rs code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "node: expose the preflight to the agent as node_health

The check that saves the most money is the one an agent runs BEFORE it authors
anything, so it belongs on the surface the agent already has open."
```

---

## Self-Review

**Spec coverage** (`2026-08-27-archetype-campaign-fleet-design.md` §5):

| Spec requirement | Task |
|---|---|
| `dcgo-harness node up` — verify digest, launch, health-check, print GO | 2 |
| `node down`, `node status` | 2 |
| ~550 MB image: repo + player + C# mirror + rules PDFs + harness | 3 |
| Rules PDFs ride in the image, not the repo (rule 32) | 3 |
| Fleet version rule: `space.rs` invalidates every player; `up` refuses | 2 (check), 3 (runbook §5) |
| `node_health` on the MCP (spec §3.1) | 4 |
| Ship the `opt-level=2` + mimalloc build fix | **Already on main** — it landed with the exam campaign (`23ef63ee9`, `134506c8e`), so a node building from this repo gets it. Nothing to do; verify in Task 3 Step 5 and say so. |
| Headless display + Photon ceiling risks | 1 |

**Type consistency:** `CheckStatus::as_str()` returns `"ok"`/`"warn"`/`"fail"`, and Task 4's JSON test asserts `"fail"` — matching. `node::health(&Path, Option<&Path>) -> Health` is called identically by Task 2's CLI and Task 4's handler.

**Ordering:** Task 1 first — it is measurement whose answers Task 3's runbook depends on. Task 2 before Task 4 (the handler calls `node::health`). Task 3 can proceed in parallel with Task 4.

**A note on Task 1's honesty requirement:** if the two unknowns cannot be measured on the available hardware, the findings document must say "not measured" with the reason. The fleet-sizing decision rests on the Photon number; a guess recorded as a measurement would be worse than no number at all.
