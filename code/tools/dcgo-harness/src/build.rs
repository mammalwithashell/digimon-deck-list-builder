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
