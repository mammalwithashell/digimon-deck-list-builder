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
