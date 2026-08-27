//! Process lifecycle for the DCGO oracle: launch it, tell whether it is
//! actually working, stop it.
//!
//! The health signal is a heartbeat file, not a PID. A hung Unity keeps its
//! process alive and would report healthy forever -- and hung Unity is the
//! failure mode that has actually happened, twice.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::manifest;

/// PID of the launched player, relative to the harness root.
pub const PID_FILE: &str = "harness.pid";
/// Image name (e.g. "DCGO.exe") of the process recorded in `PID_FILE`,
/// written alongside it. A PID alone is not proof of identity -- PIDs get
/// reused, and a stale PID file pointing at an unrelated process must never
/// be mistaken for "our DCGO is already running" (or force-killed as if it
/// were).
pub const IMAGE_FILE: &str = "harness.image";
/// Written by JobWatcher every poll. Must match HarnessConfig.HeartbeatPath.
pub const HEARTBEAT_FILE: &str = "harness.heartbeat";
/// Where the launched player's own Unity log is written, relative to the
/// harness root -- NOT the build directory, which is versioned derived data
/// that may be shared by several roots. Passed to the player via `-logFile`
/// instead of Unity's "-" (stdout) convention: `Command::spawn` inherits the
/// parent's stdio by default, so `-logFile -` would hold this process's
/// stdout pipe open for the player's entire lifetime, blocking any caller
/// that pipes or captures `up`'s output. See `player_command`.
pub const LOG_FILE: &str = "dcgo-player.log";
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

/// Seconds since the heartbeat was last written. A future mtime (clock skew:
/// VM time sync, DST, a filesystem with coarser granularity) still means the
/// file exists and was just written -- that reads as age 0, not as absent.
/// `Missing` must mean only "the file is not there".
pub fn heartbeat_age(root: &Path) -> Option<u64> {
    let meta = std::fs::metadata(root.join(HEARTBEAT_FILE)).ok()?;
    let modified = meta.modified().ok()?;
    Some(modified.elapsed().map(|d| d.as_secs()).unwrap_or(0))
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
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("removing pid file: {}", e)),
    }
    // The pid and image files must never disagree, so they are cleared
    // together. Otherwise a leftover image file could later be paired (by
    // pid reuse) with an unrelated live process and misidentify it.
    match std::fs::remove_file(root.join(IMAGE_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("removing image file: {}", e)),
    }
}

/// Image name recorded for the PID in `PID_FILE`, if any was ever written.
/// `None` covers both "never launched" and "launched by an older harness
/// version that did not record identity" -- callers must treat both as
/// "identity unknown", not as "no image, so anything goes".
fn read_image(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(IMAGE_FILE)).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn write_image(root: &Path, image_name: &str) -> Result<(), String> {
    let path = root.join(IMAGE_FILE);
    std::fs::write(&path, image_name).map_err(|e| format!("writing {}: {}", path.display(), e))
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

/// True if `pid` is currently running the executable named `image_name`
/// (e.g. "DCGO.exe"). False for a dead PID and false for a live PID running
/// something else -- callers must not treat "the PID exists" as "our
/// process exists". Uses `tasklist`'s CSV output and parses it rather than
/// substring-matching the raw text, so a PID digit-string that happens to
/// appear inside another column (e.g. a memory-usage field) cannot produce
/// a false positive.
pub fn pid_is_image(pid: u32, image_name: &str) -> bool {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"])
        .output();
    let out = match out {
        Ok(o) => o,
        Err(_) => return false,
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // A non-matching filter prints an unquoted "INFO: No tasks are
        // running..." line, not a CSV row. `parse_csv_line` still returns
        // something for it, but that something is never equal to a real
        // image name, so it falls through to `false` safely.
        if let Some(fields) = parse_csv_line(line) {
            if let Some(found_image) = fields.first() {
                if found_image.eq_ignore_ascii_case(image_name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Parse one line of `tasklist /FO CSV` output: quoted, comma-separated
/// fields (doubled quotes escape a literal quote inside a field). Returns
/// the fields in order; the image name is always the first.
fn parse_csv_line(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '"' {
            chars.next();
            let mut field = String::new();
            while let Some(c) = chars.next() {
                if c == '"' {
                    if chars.peek() == Some(&'"') {
                        field.push('"');
                        chars.next();
                    } else {
                        break;
                    }
                } else {
                    field.push(c);
                }
            }
            fields.push(field);
        } else {
            // Defensive fallback: tasklist's own CSV output always quotes
            // every field, but don't assume it for arbitrary input.
            let mut field = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                field.push(c);
                chars.next();
            }
            fields.push(field);
        }
        if chars.peek() == Some(&',') {
            chars.next();
        }
    }
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// Path to the launchable executable described by a build's manifest.
fn executable_path(build_dir: &Path, m: &manifest::BuildManifest) -> PathBuf {
    build_dir.join(&m.executable)
}

/// Build the command used to launch the player: log to `log_path` instead of
/// stdout, and detach stdin/stdout/stderr from this process entirely.
///
/// Split out from `up` so the regression this guards against -- launching
/// with `-logFile -` and inherited stdio, which holds a caller's pipe open
/// for the oracle's whole lifetime -- can be pinned by asserting on
/// `Command::get_args()` without spawning a real player.
fn player_command(exe: &Path, log_path: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.arg("-logFile")
        .arg(log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
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
    let image_name = Path::new(&m.executable)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| m.executable.clone());

    if let Some(pid) = read_pid(root) {
        if pid_alive(pid) && pid_is_image(pid, &image_name) {
            return Ok(format!(
                "already running (pid {}, heartbeat {:?})",
                pid,
                classify_heartbeat(heartbeat_age(root), DEFAULT_STALE_SECONDS)
            ));
        }
        // A pid file outliving its process is normal after a crash. So is a
        // pid file whose pid has since been recycled to an unrelated
        // process -- both are stale in the same way and get the same
        // treatment: clear and relaunch, rather than silently doing nothing
        // while believing an unrelated process is our oracle.
        clear_pid(root)?;
    }

    // Create/truncate the log up front so a stale prior run's contents can
    // never be mistaken for this launch's, and so the path exists even if
    // Unity is slow to open it.
    let log_path = root.join(LOG_FILE);
    std::fs::write(&log_path, b"")
        .map_err(|e| format!("creating {}: {}", log_path.display(), e))?;

    let child = player_command(&exe, &log_path)
        .spawn()
        .map_err(|e| format!("launching {}: {}", exe.display(), e))?;

    let pid = child.id();
    write_pid(root, pid)?;
    write_image(root, &image_name)?;
    Ok(format!(
        "launched {} (pid {}, dcgo {}, log {})",
        exe.display(),
        pid,
        &m.dcgo_commit[..m.dcgo_commit.len().min(9)],
        log_path.display()
    ))
}

/// Stop a running oracle. Not an error if none is running.
///
/// Kills only after confirming the recorded pid still belongs to the image
/// we launched. `/T` (kill the whole process tree) is deliberate for a real
/// DCGO process, but is exactly why identity must be confirmed first: on
/// pid reuse it would otherwise tear down an unrelated process tree.
pub fn down(root: &Path) -> Result<String, String> {
    let pid = match read_pid(root) {
        Some(p) => p,
        None => return Ok("not running (no pid file)".to_string()),
    };
    if !pid_alive(pid) {
        clear_pid(root)?;
        return Ok(format!("not running (stale pid {} cleared)", pid));
    }
    let image = match read_image(root) {
        Some(img) => img,
        None => {
            // Identity can't be confirmed at all -- state written by an
            // older harness version, or otherwise lost. A safety check
            // that cannot do its job must fail closed: report clearly and
            // leave the process alone rather than guessing.
            return Err(format!(
                "pid {} is alive but its image identity was never recorded ({} not found); \
                 refusing to kill without confirming identity -- stop it manually.",
                pid,
                root.join(IMAGE_FILE).display()
            ));
        }
    };
    if !pid_is_image(pid, &image) {
        // The pid was recycled to an unrelated process since we launched.
        // Our recorded state is stale, but the live process is not ours --
        // clear the stale record and stop, without touching that process.
        clear_pid(root)?;
        return Ok(format!(
            "not running (pid {} no longer belongs to {}; stale state cleared, nothing killed)",
            pid, image
        ));
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

    /// Spawn an easily identified process that is not our image, for tests
    /// that need a genuinely live PID belonging to something else. The
    /// reviewer's own reproduction used the same decoy: "a spawned PING.EXE".
    ///
    /// `-n 30` (~30s), deliberately, NOT `-n 2` (~1s). Every caller reads the
    /// decoy back through `pid_is_image`, which shells out to `tasklist` — so
    /// the decoy has to outlive an external process launch plus whatever
    /// scheduling delay the test harness imposes. At ~1s that held only while
    /// the whole suite ran in ~1.2s; once the crate grew past ~100 tests the
    /// `tasklist` call started landing AFTER the decoy had already exited, and
    /// `pid_is_image` correctly reported "not running" — failing the assertion
    /// for a reason that has nothing to do with the code under test.
    ///
    /// The decoy's lifetime is a fixture assumption, never a property under
    /// test, so widening it weakens no assertion. It stays finite rather than
    /// `-t` (infinite) so a panic before the reap leaves a process that cleans
    /// itself up instead of an immortal orphan.
    fn spawn_decoy() -> std::process::Child {
        let child = Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("spawn decoy process");
        assert!(
            pid_alive(child.id()),
            "decoy must be alive immediately after spawn for the test to mean anything"
        );
        child
    }

    #[test]
    fn pid_is_image_is_true_for_the_actual_image_and_false_for_another() {
        let mut decoy = spawn_decoy();
        let pid = decoy.id();

        assert!(pid_is_image(pid, "PING.EXE"));
        assert!(pid_is_image(pid, "ping.exe"), "must be case-insensitive");
        assert!(!pid_is_image(pid, "DCGO.exe"));

        // Reap so no stray process survives the test. KILL first, then wait:
        // a bare `wait()` blocks for the decoy's full lifetime, which would
        // make every such test pay it in wall-clock.
        let _ = decoy.kill();
        let _ = decoy.wait();
    }

    #[test]
    fn pid_is_image_is_false_for_a_dead_pid() {
        let mut decoy = spawn_decoy();
        let pid = decoy.id();
        let _ = decoy.kill();
        let _ = decoy.wait();

        assert!(!pid_alive(pid), "decoy must actually be dead for this test to mean anything");
        assert!(!pid_is_image(pid, "PING.EXE"));
    }

    #[test]
    fn parse_csv_line_handles_tasklist_style_quoted_fields() {
        let line = r#""DCGO.exe","1234","Console","1","123,456 K""#;
        let fields = parse_csv_line(line).unwrap();
        assert_eq!(fields[0], "DCGO.exe");
        assert_eq!(fields[1], "1234");
        assert_eq!(fields[4], "123,456 K");
    }

    #[test]
    fn parse_csv_line_does_not_let_an_embedded_pid_digit_match_the_image_column() {
        // A pid that happens to appear as a substring of another column
        // (here, inside the memory-usage field) must not make `pid_is_image`
        // match on it -- only the first (image name) field counts.
        let line = r#""cmd.exe","4242","Console","1","4,242 K""#;
        let fields = parse_csv_line(line).unwrap();
        assert_eq!(fields[0], "cmd.exe");
        assert_ne!(fields[0], "4242");
    }

    #[test]
    fn up_relaunches_when_the_recorded_pid_is_a_different_image() {
        let root = std::env::temp_dir().join("dcgo_daemon_wrong_image_root");
        let build = std::env::temp_dir().join("dcgo_daemon_wrong_image_build");
        for d in [&root, &build] {
            let _ = std::fs::remove_dir_all(d);
            std::fs::create_dir_all(d).unwrap();
        }

        // Record a live-but-unrelated pid as if it were a prior harness run --
        // the reviewer-confirmed reproduction (a pid file pointing at an
        // unrelated PING.EXE, e.g. after a crash + pid reuse).
        let mut decoy = spawn_decoy();
        let decoy_pid = decoy.id();
        write_pid(&root, decoy_pid).unwrap();

        let m = crate::manifest::BuildManifest {
            dcgo_commit: "be359bb5b".into(),
            built_at: "2026-08-20T00:00:00Z".into(),
            artifact_sha256: "deadbeef".into(),
            action_space_hash: crate::manifest::action_space_hash(),
            executable: "DCGO.exe".into(),
        };
        crate::manifest::save(&build, &m).unwrap();
        // Not a real launchable binary -- this test only needs to prove `up`
        // does not stop at "already running" for the wrong-image pid and
        // actually attempts a relaunch; a genuinely launchable fixture is
        // exercised by the other `up` tests via the same fake-bytes pattern.
        std::fs::write(build.join("DCGO.exe"), b"fake").unwrap();

        let err = up(&root, &build).unwrap_err();
        assert!(
            !err.to_lowercase().contains("already running"),
            "must not treat an unrelated live pid as our own process: {}",
            err
        );
        assert_eq!(
            read_pid(&root),
            None,
            "the wrong-image pid must be cleared, not kept as if verified"
        );

        // Kill before waiting -- a bare `wait()` would block for the decoy's
        // full lifetime.
        let _ = decoy.kill();
        let _ = decoy.wait();

        for d in [&root, &build] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn down_refuses_to_kill_when_the_image_name_is_missing() {
        let root = std::env::temp_dir().join("dcgo_daemon_down_no_image");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        // Simulate state written by an older harness version: a pid file
        // recording a genuinely live process, but no sibling image file
        // saying what that pid is supposed to be.
        let mut decoy = spawn_decoy();
        let pid = decoy.id();
        write_pid(&root, pid).unwrap();
        assert!(read_image(&root).is_none(), "no image file written yet");

        let err = down(&root).unwrap_err();
        assert!(
            err.to_lowercase().contains("manually"),
            "must fail closed and tell the caller to stop it manually: {}",
            err
        );
        assert!(
            pid_alive(pid),
            "down must not have killed anything it could not identify"
        );

        // We spawned the decoy, so we reap it -- not by calling `down`,
        // which is exactly the destructive path this test proves must not
        // fire without a confirmed identity.
        let _ = decoy.kill();
        let _ = decoy.wait();

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_future_dated_heartbeat_is_healthy_not_missing() {
        let root = std::env::temp_dir().join("dcgo_daemon_future_heartbeat");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(HEARTBEAT_FILE);
        std::fs::write(&path, b"beat").unwrap();

        // Simulate clock skew: a heartbeat mtime an hour ahead of "now".
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_modified(future).unwrap();
        drop(f);

        let age = heartbeat_age(&root);
        assert_eq!(age, Some(0), "a future mtime must read as age 0, not absent");
        assert_eq!(
            classify_heartbeat(age, DEFAULT_STALE_SECONDS),
            Health::Healthy,
            "the file exists and is heartbeating -- clock skew must not read as never-started"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_genuinely_absent_heartbeat_file_still_classifies_as_missing() {
        let root = std::env::temp_dir().join("dcgo_daemon_absent_heartbeat");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        assert_eq!(heartbeat_age(&root), None);
        assert_eq!(
            classify_heartbeat(heartbeat_age(&root), DEFAULT_STALE_SECONDS),
            Health::Missing
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn player_command_redirects_the_log_to_a_file_not_stdout() {
        // Regression: `up` used to launch with `-logFile -`, which is Unity's
        // convention for "write the log to stdout". Combined with
        // `Command::spawn`'s default of inheriting the parent's stdio, that
        // held this process's stdout pipe open for as long as the player
        // lived -- so a caller piping or capturing `up`'s output (e.g.
        // `dcgo-harness up | grep ...`) blocked for the oracle's entire
        // lifetime instead of `up` returning immediately.
        let exe = Path::new("DCGO.exe");
        let log_path = std::env::temp_dir()
            .join("dcgo_daemon_player_command_test")
            .join(LOG_FILE);

        let cmd = player_command(exe, &log_path);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();

        let flag_pos = args
            .iter()
            .position(|a| *a == "-logFile")
            .expect("-logFile must be passed");
        let log_arg = args
            .get(flag_pos + 1)
            .expect("-logFile must be followed by a path");
        assert_ne!(
            *log_arg, "-",
            "must not tell Unity to write its log to stdout"
        );
        assert_eq!(*log_arg, log_path.as_os_str());
    }
}
