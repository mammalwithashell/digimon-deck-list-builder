//! Drives the headless Unity build and stamps its manifest.
//!
//! The host CLI owns versioning rather than the Editor script: it chooses the
//! output directory, then hashes what lands there and records the DCGO commit.
//! Keeping both halves here means the digest and the artifact can never be
//! stamped by two different notions of "current".

use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

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
            "git rev-parse in {}: {}",
            project_path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Reject an `--output` at or beneath `--project`. A build there writes a
/// multi-GB player into the LFS-tracked DCGO submodule -- the disk-bloat
/// scenario CLAUDE.md rule 29 exists to prevent, and painful to undo.
///
/// `output_dir` need not exist yet (`run()` creates it later) -- in fact
/// that is the *common* case, not an edge case: `--output` names a fresh
/// directory on essentially every first build, often as a bare relative
/// path like `--output build`. So each side is first absolutized against
/// the current working directory (a relative path is meaningless for a
/// containment check until it is anchored), then resolved independently by
/// [`resolve_for_containment_check`] (full canonicalize when possible,
/// otherwise canonicalize the deepest existing ancestor and lexically
/// normalize the rest), and compared component-wise -- case-folded on
/// Windows, since NTFS is case-insensitive by default and a case typo
/// genuinely names the same on-disk directory.
///
/// If the current working directory itself can't be determined, this
/// fails closed with an error rather than silently skipping the check --
/// a safety check that can't do its job must not report success.
pub fn validate_output_outside_project(project_path: &Path, output_dir: &Path) -> Result<(), String> {
    let project = resolve_for_containment_check(project_path)?;
    let output = resolve_for_containment_check(output_dir)?;
    if path_contains(&project, &output) {
        return Err(format!(
            "--output {} is at or beneath --project {} -- this would write a \
             multi-GB build artifact into the LFS-tracked DCGO submodule \
             (CLAUDE.md rule 29). Choose an --output outside --project.",
            output_dir.display(),
            project_path.display(),
        ));
    }
    Ok(())
}

/// Resolve `path` into an absolute, normalized form suitable for the
/// component-wise containment check in [`validate_output_outside_project`].
///
/// A relative path is meaningless for a containment check until it is
/// anchored, so the first step -- before any canonicalization or lexical
/// normalization -- is absolutizing against the current working directory
/// (a no-op if `path` is already absolute). Skipping this step is the
/// actual defect this function guards against: a relative `--output` like
/// `build` that doesn't exist yet can't be canonicalized, and a single
/// bare component has no intermediate ancestor for the fallback below to
/// anchor to either, so it would otherwise reach the final lexical-only
/// fallback still relative and only one component long -- short enough
/// that the length guard in [`path_contains`] would wrongly wave it
/// through as "not contained" without ever comparing it against the
/// project path.
///
/// Once absolute: a path that exists is canonicalized outright, which gets
/// the OS to resolve both `..` and (on Windows) case for us. A path that
/// doesn't exist yet -- the normal shape of a fresh `--output` directory
/// -- can't be canonicalized as a whole, so instead: canonicalize the
/// deepest existing ancestor (this always bottoms out, since the
/// filesystem root/drive always exists) and lexically normalize the
/// non-existent trailing components onto it. That resolves `..` in the
/// trailing part even though nothing there exists yet, and gets the real
/// on-disk case for the part that does. If even the ancestor fails to
/// canonicalize (e.g. a permissions error), fall back to a purely lexical
/// normalization of the whole (now-absolute) path so `..` traversal is
/// still caught.
///
/// Fails closed: if the current working directory can't be determined for
/// a relative `path`, this returns an error instead of silently treating
/// the path as if it were already anchored.
fn resolve_for_containment_check(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let cwd = std::env::current_dir().map_err(|e| {
            format!(
                "could not determine the current directory to resolve relative path {}: {} \
                 -- containment against --project cannot be verified",
                path.display(),
                e
            )
        })?;
        cwd.join(path)
    };

    if let Ok(canon) = absolute.canonicalize() {
        return Ok(canon);
    }

    let components: Vec<Component> = absolute.components().collect();
    for split in (0..components.len()).rev() {
        let candidate: PathBuf = components[..split].iter().collect();
        if candidate.as_os_str().is_empty() || !candidate.exists() {
            continue;
        }
        if let Ok(canon_ancestor) = candidate.canonicalize() {
            let mut resolved = canon_ancestor;
            for component in &components[split..] {
                push_component_lexically(&mut resolved, component);
            }
            return Ok(resolved);
        }
    }

    Ok(lexically_normalize(&absolute))
}

/// Append one path component onto `base`, resolving `.` and `..` lexically
/// (without touching the filesystem) instead of appending them literally.
fn push_component_lexically(base: &mut PathBuf, component: &Component) {
    match component {
        Component::CurDir => {}
        Component::ParentDir => {
            base.pop();
        }
        other => base.push(other.as_os_str()),
    }
}

/// Lexically normalize a path -- collapse `.` and `..` by walking
/// components and maintaining a stack, without ever touching the
/// filesystem. Used only as a last-resort fallback when not even the
/// deepest existing ancestor of a path can be canonicalized.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        push_component_lexically(&mut result, &component);
    }
    result
}

/// Component-wise "is `output` at or beneath `project`", case-folded on
/// Windows (NTFS is case-insensitive by default, so a case-only difference
/// names the same directory there). Component-wise -- not a string
/// prefix -- so a sibling directory with a shared name prefix (`DCGO` vs
/// `DCGO-sibling`) is correctly treated as outside.
fn path_contains(project: &Path, output: &Path) -> bool {
    let project_components: Vec<Component> = project.components().collect();
    let output_components: Vec<Component> = output.components().collect();
    if project_components.len() > output_components.len() {
        return false;
    }
    project_components
        .iter()
        .zip(output_components.iter())
        .all(|(p, o)| component_eq(p, o))
}

fn component_eq(a: &Component, b: &Component) -> bool {
    if cfg!(windows) {
        a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Guard against Unity exiting 0 without producing a new artifact -- e.g. a
/// rebuild into an `--output` directory that already holds an executable
/// from a previous build, where Unity silently no-ops instead of replacing
/// it. Without this, `run()` would stamp the stale bytes as if they were the
/// just-built commit: a silent, plausible lie (see module doc).
///
/// Freshness is judged from the *whole build output directory*, not just
/// `exe`, because DCGO is a Unity Mono build: `exe` is only a launcher
/// stub, and the actual C# lives in
/// `DCGO_Data/Managed/Assembly-CSharp.dll`. An incremental build that only
/// recompiled C# does not rewrite the stub, so checking the executable
/// alone is a false positive on the most common kind of rebuild there is --
/// confirmed live: `DCGO.exe` predating `started` next to a same-build
/// `Assembly-CSharp.dll` and `globalgamemanagers` that postdated it, from a
/// build that genuinely shipped new C# behavior. If ANY file anywhere
/// under the output directory is at or newer than `started`, something was
/// genuinely rebuilt.
///
/// The executable's own existence is still checked first and reported as
/// its own distinct error -- "unity produced nothing at all" is a
/// different failure from "unity touched nothing during this run", and the
/// caller should be able to tell them apart.
///
/// `started` must be captured immediately before spawning Unity. A build
/// takes minutes, so a plain `>=` comparison against mtimes is enough -- no
/// tolerance slop that would reopen the hole this closes.
pub fn ensure_fresh(exe: &Path, started: SystemTime) -> Result<(), String> {
    std::fs::metadata(exe).map_err(|e| {
        format!(
            "unity reported success but produced no new artifact: {} ({}). \
             The output directory may hold a stale build.",
            exe.display(),
            e
        )
    })?;

    let output_dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let newest = newest_mtime(output_dir)
        .map_err(|e| format!("checking build output {}: {}", output_dir.display(), e))?;

    match newest {
        Some(modified) if modified >= started => Ok(()),
        _ => Err(format!(
            "unity reported success but produced no new artifact: nothing under {} \
             was modified during this build -- nothing was rebuilt. The output \
             directory may hold a stale build.",
            output_dir.display()
        )),
    }
}

/// Newest modification time of any file under `dir`, walked recursively.
/// Returns `None` if the directory holds no files at all.
///
/// Only file mtimes are compared -- a directory entry can be touched
/// without any new content landing inside it, so trusting directory mtimes
/// would risk exactly the kind of false positive [`ensure_fresh`] exists to
/// avoid.
fn newest_mtime(dir: &Path) -> Result<Option<SystemTime>, String> {
    let mut newest: Option<SystemTime> = None;
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("reading directory {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("reading an entry in {}: {}", dir.display(), e))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("reading file type of {}: {}", path.display(), e))?;

        let candidate = if file_type.is_dir() {
            newest_mtime(&path)?
        } else {
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .map_err(|e| format!("reading metadata for {}: {}", path.display(), e))?;
            Some(modified)
        };

        if let Some(candidate) = candidate {
            newest = Some(newest.map_or(candidate, |n| n.max(candidate)));
        }
    }
    Ok(newest)
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

/// Prepare the output directory before Unity runs: ensure it exists, and
/// remove any `manifest.json` left over from a previous build.
///
/// `ensure_fresh` runs *after* `BuildPipeline.BuildPlayer` has already
/// rewritten the output directory. If it rejects the build -- or Unity
/// itself fails -- the directory is left holding brand-new build artifacts
/// next to a `manifest.json` that still describes the *previous*, different
/// build. That stale manifest still passes its own action-space gate (the
/// gate only compares it against the current engine, not against what is
/// actually on disk), so `up` would happily launch a build the manifest
/// positively misdescribes -- worse than the staleness `ensure_fresh`
/// exists to catch in the first place. Deleting the manifest before Unity
/// runs means a failed or rejected build leaves no manifest at all, so
/// `manifest::load` in `up` fails loudly instead of launching a lie.
fn prepare_output_dir(output_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("creating {}: {}", output_dir.display(), e))?;

    let manifest_path = output_dir.join(manifest::MANIFEST_FILE);
    if manifest_path.exists() {
        std::fs::remove_file(&manifest_path)
            .map_err(|e| format!("removing stale manifest {}: {}", manifest_path.display(), e))?;
    }
    Ok(())
}

/// Run the Unity build, then stamp and save its manifest.
pub fn run(req: &BuildRequest) -> Result<BuildManifest, String> {
    // Fail fast, before the 20-minute Unity invocation: a mistaken --output
    // inside --project would write a multi-GB player into the DCGO
    // submodule (rule 29).
    validate_output_outside_project(&req.project_path, &req.output_dir)?;

    if !req.unity_exe.exists() {
        return Err(format!("unity not found at {}", req.unity_exe.display()));
    }
    prepare_output_dir(&req.output_dir)?;

    let commit = git_commit(&req.project_path)?;

    println!("building DCGO {} -> {}", &commit[..commit.len().min(9)], req.output_dir.display());
    println!("(a first build compiles shaders and can take 20+ minutes)");

    // Captured immediately before spawning Unity so `ensure_fresh` can tell
    // a genuinely new artifact from a stale one left by a previous build
    // into the same --output directory.
    let started = SystemTime::now();

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

    ensure_fresh(&req.output_dir.join(DEFAULT_EXECUTABLE), started)?;

    let built_at = chrono::Utc::now().to_rfc3339();
    let m = stamp(req, DEFAULT_EXECUTABLE, commit, built_at)?;
    manifest::save(&req.output_dir, &m)?;
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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

    // --- ensure_fresh ---------------------------------------------------
    //
    // Guards against Unity exiting 0 without producing a new artifact (e.g.
    // rebuilding into an --output dir that already held a player from a
    // prior build, and the new build silently no-ops).

    #[test]
    fn ensure_fresh_accepts_an_artifact_modified_after_start() {
        let dir = std::env::temp_dir().join("dcgo_build_ensure_fresh_ok");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("DCGO.exe");

        // started is in the past relative to the write below, so the
        // artifact's mtime should land at or after it.
        let started = SystemTime::now() - std::time::Duration::from_secs(5);
        std::fs::write(&exe, b"fresh player").unwrap();

        assert!(ensure_fresh(&exe, started).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_fresh_rejects_an_artifact_modified_before_start() {
        let dir = std::env::temp_dir().join("dcgo_build_ensure_fresh_stale");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("DCGO.exe");
        std::fs::write(&exe, b"stale player from a previous build").unwrap();

        // started is in the future relative to the write above, simulating a
        // rebuild into an output dir that already held an executable, where
        // Unity exited 0 without actually replacing it.
        let started = SystemTime::now() + std::time::Duration::from_secs(60);

        let err = ensure_fresh(&exe, started).unwrap_err();
        assert!(
            err.contains("produced no new artifact"),
            "must say plainly that unity produced nothing new: {}",
            err
        );
        assert!(
            err.to_lowercase().contains("stale"),
            "must warn the output directory may hold a stale build: {}",
            err
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_fresh_reports_a_missing_artifact_clearly() {
        let dir = std::env::temp_dir().join("dcgo_build_ensure_fresh_missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("DCGO.exe");

        let err = ensure_fresh(&exe, SystemTime::now()).unwrap_err();
        assert!(
            err.contains("produced no new artifact"),
            "must say plainly that unity produced nothing new: {}",
            err
        );
        assert!(err.contains("DCGO.exe"), "must name the missing file: {}", err);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Sets a file's mtime to `time`, so a test can simulate an artifact
    /// left behind by a previous build without sleeping in real time.
    fn set_mtime(path: &Path, time: SystemTime) {
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    #[test]
    fn ensure_fresh_accepts_when_only_a_nested_dcgo_data_file_is_newer() {
        // The exact live regression: DCGO is a Unity Mono build, so
        // DCGO.exe is just a launcher stub and the actual C# lives in
        // DCGO_Data/Managed/Assembly-CSharp.dll. An incremental build that
        // only recompiled C# does not rewrite the stub -- observed live as
        // DCGO.exe at 18:18:10 (the previous build) next to
        // Assembly-CSharp.dll at 19:02:35 and globalgamemanagers at
        // 19:05:52 (this build, C# demonstrably recompiled and shipped).
        // Freshness must be judged from the whole output directory, not
        // just the executable, or every C#-only incremental rebuild is a
        // false positive.
        let dir = std::env::temp_dir().join("dcgo_build_ensure_fresh_nested_fresh");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let exe = dir.join("DCGO.exe");
        std::fs::write(&exe, b"launcher stub from a previous build").unwrap();
        // Age the stub so it predates `started`, mirroring the live case
        // where the launcher genuinely was not rewritten this build.
        set_mtime(&exe, SystemTime::now() - Duration::from_secs(300));

        let started = SystemTime::now() - Duration::from_secs(60);

        let managed = dir.join("DCGO_Data").join("Managed");
        std::fs::create_dir_all(&managed).unwrap();
        // Written after `started` was captured -- the recompiled DLL.
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"recompiled C#").unwrap();

        assert!(
            ensure_fresh(&exe, started).is_ok(),
            "a rebuild that only touched DCGO_Data must be accepted -- the launcher \
             stub is not rewritten by an incremental Mono build"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_fresh_rejects_when_nothing_in_the_output_dir_is_newer_than_start() {
        let dir = std::env::temp_dir().join("dcgo_build_ensure_fresh_nothing_rebuilt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let exe = dir.join("DCGO.exe");
        std::fs::write(&exe, b"stale player from a previous build").unwrap();
        let managed = dir.join("DCGO_Data").join("Managed");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"stale dll").unwrap();

        // started is in the future relative to every write above, so
        // nothing anywhere under the output directory was touched during
        // "this build" -- a genuine no-op rebuild, not the false-positive
        // case above.
        let started = SystemTime::now() + Duration::from_secs(60);

        let err = ensure_fresh(&exe, started).unwrap_err();
        assert!(
            err.to_lowercase().contains("nothing") && err.to_lowercase().contains("rebuilt"),
            "must say plainly that nothing was rebuilt: {}",
            err
        );
        assert!(
            err.to_lowercase().contains("stale"),
            "must warn the output directory may hold a stale build: {}",
            err
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- prepare_output_dir ----------------------------------------------
    //
    // ensure_fresh runs after BuildPipeline.BuildPlayer has already
    // rewritten the output directory, so a rejected or failed build must
    // not leave a stale manifest.json behind describing the previous
    // build's bytes -- `up` would load it, see it passes its own
    // action-space gate, and launch a build the manifest misdescribes.

    #[test]
    fn prepare_output_dir_removes_a_stale_manifest_before_the_build() {
        let dir = std::env::temp_dir().join("dcgo_build_prepare_output_removes_manifest");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest_path = dir.join(manifest::MANIFEST_FILE);
        std::fs::write(&manifest_path, b"{\"stale\": true}").unwrap();
        assert!(manifest_path.exists());

        prepare_output_dir(&dir).unwrap();

        assert!(
            !manifest_path.exists(),
            "a stale manifest must not survive prepare_output_dir -- otherwise a \
             failed or rejected rebuild leaves it describing artifacts that are no \
             longer accurate"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prepare_output_dir_creates_a_missing_directory() {
        let dir = std::env::temp_dir().join("dcgo_build_prepare_output_creates_dir");
        let _ = std::fs::remove_dir_all(&dir);

        prepare_output_dir(&dir).unwrap();

        assert!(dir.is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- validate_output_outside_project ---------------------------------
    //
    // Rejects an --output at or beneath --project so a mistaken build can't
    // write a multi-GB player into the LFS-tracked DCGO submodule (rule 29).

    #[test]
    fn validate_output_outside_project_rejects_a_dir_inside_the_project() {
        let root = std::env::temp_dir().join("dcgo_build_validate_output_inside");
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("DCGO");
        let output = project.join("build-output");
        std::fs::create_dir_all(&output).unwrap();

        let err = validate_output_outside_project(&project, &output).unwrap_err();
        assert!(
            err.contains("DCGO submodule") || err.contains("submodule"),
            "should explain why this is rejected: {}",
            err
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_output_outside_project_rejects_the_project_dir_itself() {
        let root = std::env::temp_dir().join("dcgo_build_validate_output_is_project");
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("DCGO");
        std::fs::create_dir_all(&project).unwrap();

        let err = validate_output_outside_project(&project, &project).unwrap_err();
        assert!(err.contains("submodule"), "got: {}", err);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_output_outside_project_accepts_a_sibling_dir() {
        let root = std::env::temp_dir().join("dcgo_build_validate_output_sibling");
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("DCGO");
        // "DCGO-sibling" is unambiguously a different directory from
        // "DCGO" even under case folding, so this exercises the
        // shared-name-prefix pitfall (a naive string-prefix check would
        // wrongly reject it) without overlapping with the case-typo
        // test's intent below.
        let output = root.join("DCGO-sibling").join("v1");
        std::fs::create_dir_all(&project).unwrap();
        // output_dir need not exist yet -- run() creates it later.
        let _ = std::fs::remove_dir_all(&output);

        assert!(validate_output_outside_project(&project, &output).is_ok());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(windows)]
    fn validate_output_outside_project_rejects_a_case_only_difference() {
        // NTFS is case-insensitive by default, so "dcgo" and "DCGO"
        // genuinely refer to the same on-disk directory -- a very
        // plausible typo on Windows. `output_dir` doesn't exist yet
        // (the common case for a first build), so this exercises the
        // non-canonicalizable fallback path, not the both-exist path.
        let root = std::env::temp_dir().join("dcgo_build_validate_output_case_typo");
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("DCGO");
        std::fs::create_dir_all(&project).unwrap();
        let output = root.join("dcgo").join("v1");

        let err = validate_output_outside_project(&project, &output).unwrap_err();
        assert!(err.contains("submodule"), "got: {}", err);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_output_outside_project_rejects_dot_dot_traversal_back_inside() {
        // `Path::components()` does not collapse a `Normal("DCGO-sibling")`
        // component against a following `ParentDir`, so a naive
        // component-wise prefix walk over the raw (non-normalized) path
        // never notices this resolves back inside the project. The
        // non-existent trailing components must be lexically normalized
        // before the containment check.
        let root = std::env::temp_dir().join("dcgo_build_validate_output_dotdot");
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("DCGO");
        std::fs::create_dir_all(&project).unwrap();
        let output = root.join("DCGO-sibling").join("..").join("DCGO").join("v1");

        let err = validate_output_outside_project(&project, &output).unwrap_err();
        assert!(err.contains("submodule"), "got: {}", err);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_output_outside_project_falls_back_to_prefix_check_when_neither_exists() {
        // Neither path exists, so canonicalize() fails for both; the
        // function must not silently pass just because it couldn't resolve
        // symlinks/relative components -- it should still catch the
        // nested-path case via a plain prefix comparison.
        let root = std::env::temp_dir().join("dcgo_build_validate_output_no_canon");
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("DCGO");
        let output = project.join("nested").join("build");

        let err = validate_output_outside_project(&project, &output).unwrap_err();
        assert!(err.contains("submodule"), "got: {}", err);
    }

    #[test]
    fn validate_output_outside_project_rejects_a_relative_output_resolving_inside() {
        // A relative --output that is a single bare component (the natural
        // `--output build` on a first build) doesn't exist yet, so it can't
        // be canonicalized. Crucially, `resolve_for_containment_check`'s
        // ancestor-search fallback can *never* anchor a single-component
        // relative path to a real directory: the only candidate split
        // point (0) yields an empty path, which the loop explicitly skips.
        // So, before the fix, it fell straight through to pure lexical
        // normalization of the still-relative "build" -- one component --
        // with zero CWD context. Compared against the many-component
        // absolute --project, `path_contains`'s length guard fired and
        // returned "not contained" without comparing anything -- the
        // reviewer's exact repro. Anchor `project` on the actual CWD
        // (rather than assuming a fixed CWD) and never mutate the
        // process's CWD, so this test holds no matter where the suite
        // runs from -- and needs no filesystem setup, since `project`
        // already exists (it *is* the CWD).
        let cwd = std::env::current_dir().expect("current_dir");
        let project = cwd.clone();
        let relative_output = Path::new("dcgo_build_validate_output_relative_probe_inside");
        // Guard against stray state from an interrupted prior run -- this
        // test creates nothing, but a leftover directory of the same name
        // would still be a real (if accidental) match and should not be
        // mistaken for this test's own signal.
        let _ = std::fs::remove_dir_all(cwd.join(relative_output));

        let err = validate_output_outside_project(&project, relative_output).unwrap_err();
        assert!(err.contains("submodule"), "got: {}", err);
    }

    #[test]
    fn validate_output_outside_project_accepts_a_relative_output_resolving_outside() {
        // Mirrors the previous test's single-bare-component shape, but the
        // relative --output resolves to a sibling of --project under CWD,
        // not beneath it, so it must be accepted -- confirming the
        // CWD-anchoring fix doesn't turn into a blanket rejection of every
        // relative path.
        let cwd = std::env::current_dir().expect("current_dir");
        let project = cwd.join("dcgo_build_validate_output_relative_outside_project");
        let _ = std::fs::remove_dir_all(&project);
        std::fs::create_dir_all(&project).unwrap();

        let relative_output = Path::new("dcgo_build_validate_output_relative_outside_sibling");
        let _ = std::fs::remove_dir_all(cwd.join(relative_output));

        assert!(validate_output_outside_project(&project, relative_output).is_ok());

        let _ = std::fs::remove_dir_all(&project);
    }
}
