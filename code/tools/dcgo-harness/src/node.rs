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
