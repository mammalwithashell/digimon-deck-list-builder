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
/// Digest identifying a *built player* by the content that actually decides
/// how it behaves.
///
/// NOT `sha256_file(exe)`. DCGO is a Unity Mono build, so the executable is a
/// launcher stub and every line of game logic — the whole recorder mod, the
/// scripted input driver, the deck stacker — lives in
/// `DCGO_Data/Managed/Assembly-CSharp.dll`. A C#-only rebuild does not rewrite
/// the stub, so hashing it makes two builds with completely different
/// behaviour share an identity.
///
/// That is not hypothetical: builds at `dcgo_commit` 8c4f98cb6 and a2eb37e10,
/// 656 lines of new C# apart, stamped the SAME `artifact_sha256` while their
/// `Assembly-CSharp.dll`s differed (0d8dc951… vs a5d3fdd7…). The freshness
/// check in `build.rs` already knew the stub was not the artifact; identity
/// had not caught up.
///
/// It matters because the manifest is provenance. The test drafter cites
/// "DCGO build <hash>" in the header of every drafted test, and the design
/// defers publishing the artifact behind exactly this digest. A hash that
/// cannot distinguish two builds turns both into plausible lies.
///
/// Digest = SHA-256 over `<rel-path>` + LF + `<file-sha256>` + LF, for the launcher plus
/// every file under `DCGO_Data/Managed`, sorted by relative path so the walk
/// order cannot change the answer.
pub fn sha256_build_identity(dir: &Path, exe_name: &str) -> Result<String, String> {
    let exe = dir.join(exe_name);
    if !exe.is_file() {
        return Err(format!("no executable at {}", exe.display()));
    }

    let managed = dir.join("DCGO_Data").join("Managed");
    if !managed.is_dir() {
        return Err(format!(
            "no managed assembly directory at {} — a DCGO player without one is malformed,              and hashing the launcher stub alone would silently produce an identity that              cannot distinguish two builds",
            managed.display()
        ));
    }

    let mut entries: Vec<(String, std::path::PathBuf)> = vec![(exe_name.to_string(), exe)];
    for e in std::fs::read_dir(&managed)
        .map_err(|e| format!("reading {}: {}", managed.display(), e))?
    {
        let e = e.map_err(|e| format!("reading {}: {}", managed.display(), e))?;
        let path = e.path();
        if path.is_file() {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("non-UTF8 filename under {}", managed.display()))?;
            entries.push((format!("DCGO_Data/Managed/{name}"), path));
        }
    }

    // Sort so a filesystem that enumerates differently cannot change identity.
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut h = Sha256::new();
    for (rel, path) in entries {
        h.update(rel.as_bytes());
        h.update(b"
");
        h.update(sha256_file(&path)?.as_bytes());
        h.update(b"
");
    }
    Ok(format!("{:x}", h.finalize()))
}

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

#[cfg(test)]
mod build_identity_tests {
    use super::*;

    fn write(p: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    /// Build two output dirs that differ ONLY in the managed assembly --
    /// exactly what a C#-only rebuild produces.
    fn two_builds(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let base = std::env::temp_dir().join(format!("dcgo_build_identity_{tag}"));
        let _ = std::fs::remove_dir_all(&base);
        let (a, b) = (base.join("a"), base.join("b"));
        for (d, dll) in [(&a, &b"old C#"[..]), (&b, &b"new C# with InputDriver"[..])] {
            // Byte-identical launcher stub in both: Unity does not rewrite it
            // when only scripts changed.
            write(&d.join("DCGO.exe"), b"identical launcher stub");
            write(&d.join("DCGO_Data").join("Managed").join("Assembly-CSharp.dll"), dll);
            write(&d.join("DCGO_Data").join("Managed").join("UnityEngine.dll"), b"engine");
        }
        (a, b)
    }

    #[test]
    fn hashing_only_the_exe_cannot_tell_two_builds_apart() {
        // Pins the defect this function exists to fix, so nobody "simplifies"
        // build identity back to the executable. Observed live: a build at
        // dcgo_commit 8c4f98cb6 and one at a2eb37e10, 656 lines of new C#
        // apart, stamped the SAME artifact_sha256.
        let (a, b) = two_builds("exe_only");
        assert_eq!(
            sha256_file(&a.join("DCGO.exe")).unwrap(),
            sha256_file(&b.join("DCGO.exe")).unwrap(),
            "the stub is identical -- which is precisely why it is not an identity"
        );
    }

    #[test]
    fn build_identity_distinguishes_a_csharp_only_rebuild() {
        let (a, b) = two_builds("csharp_only");
        let ha = sha256_build_identity(&a, "DCGO.exe").unwrap();
        let hb = sha256_build_identity(&b, "DCGO.exe").unwrap();
        assert_ne!(
            ha, hb,
            "a build whose game logic changed must not share an identity with one that did not"
        );
    }

    #[test]
    fn build_identity_is_stable_for_identical_content() {
        let (a, _) = two_builds("stable");
        assert_eq!(
            sha256_build_identity(&a, "DCGO.exe").unwrap(),
            sha256_build_identity(&a, "DCGO.exe").unwrap(),
            "identity must not depend on directory-walk order"
        );
    }

    #[test]
    fn build_identity_covers_the_launcher_too() {
        let (a, b) = two_builds("launcher");
        // Same managed assemblies, different stub -> still a different build.
        std::fs::write(b.join("DCGO_Data").join("Managed").join("Assembly-CSharp.dll"), b"old C#")
            .unwrap();
        std::fs::write(b.join("DCGO.exe"), b"a DIFFERENT launcher stub").unwrap();
        assert_ne!(
            sha256_build_identity(&a, "DCGO.exe").unwrap(),
            sha256_build_identity(&b, "DCGO.exe").unwrap()
        );
    }

    #[test]
    fn build_identity_errors_when_managed_is_missing() {
        // Silently hashing just the stub is how the original defect read as
        // success. A build with no managed dir is malformed; say so.
        let dir = std::env::temp_dir().join("dcgo_build_identity_nomanaged");
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir.join("DCGO.exe"), b"stub");
        let err = sha256_build_identity(&dir, "DCGO.exe").unwrap_err();
        assert!(err.to_lowercase().contains("managed"), "got: {err}");
    }
}
