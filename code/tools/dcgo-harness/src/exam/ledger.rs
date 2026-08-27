//! The fleet ledger's two mutable halves: the **attempt log** and **claims**.
//!
//! [`verdict::VerdictStore`](super::verdict::VerdictStore) records *current
//! state* — what each clause's verdict is now. This module records the two
//! things that state cannot express:
//!
//! - **History.** `unmeasured` cannot distinguish "nobody has looked" from
//!   "three nodes each burned an afternoon discovering the same dead end".
//!   The attempt log can, and that difference is the whole point of a ledger
//!   shared between nodes.
//! - **Intent.** A claim says a node is working on a card *right now*, so a
//!   second node picks something else.
//!
//! Both are shaped by how they merge. The log is one JSON object per line
//! under a git union merge driver, so concurrent nodes concatenate rather than
//! conflict. Claims are one small file per card, so disjoint claimers touch
//! disjoint files.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Default location of the attempt log, relative to the repo root.
pub const DEFAULT_LOG: &str = "qa/qa-reports/exam-log.jsonl";

/// One recorded attempt at one clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    /// RFC3339 UTC.
    pub ts: String,
    pub job_id: String,
    /// Which node ran it — the operator-facing name, not a hostname UUID.
    pub node: String,
    pub archetype: String,
    pub card: String,
    /// `{card_id}#{zone}#{idx}` — a `clause_coverage` clause id.
    pub clause: String,
    pub verdict_before: String,
    pub verdict_after: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dcgo_build: Option<String>,
    /// Free-form but conventional: `oracle_clean`, `oracle_diverged`,
    /// `sim_failed`, `unreachable`, `abandoned`.
    pub outcome: String,
}

/// Append one attempt as a single line, creating the file and its parents.
///
/// Serialized with `to_string` (never `to_string_pretty`): one record per line
/// is precisely what makes the union merge driver correct.
pub fn append_attempt(path: &Path, a: &Attempt) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {} for the exam log: {e}", parent.display()))?;
        }
    }
    let mut line = serde_json::to_string(a)
        .map_err(|e| format!("failed to serialize attempt: {e}"))?;
    line.push('\n');

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("failed to open exam log {}: {e}", path.display()))?;
    f.write_all(line.as_bytes())
        .map_err(|e| format!("failed to append to exam log {}: {e}", path.display()))
}

/// Read every attempt. A missing file is an empty history, not an error.
pub fn read_attempts(path: &Path) -> Result<Vec<Attempt>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read exam log {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let a: Attempt = serde_json::from_str(line).map_err(|e| {
            format!(
                "exam log {} line {}: {e}",
                path.display(),
                i + 1
            )
        })?;
        out.push(a);
    }
    Ok(out)
}

/// Default claim directory, relative to the repo root.
pub const DEFAULT_CLAIMS: &str = "qa/qa-reports/exam-claims";

/// An advisory lease on one card.
///
/// **Advisory, deliberately.** Two nodes pushing in the same instant can both
/// claim a card: git is the only coordinator here. That is an accepted
/// trade — at roughly $8 of agent time per authored clause, an occasional
/// duplicate costs far less than a lease server, and the duplicate is
/// *detectable* at merge (two verdicts for one clause, normally agreeing).
/// If this ever starts hurting, the MCP is where a real lease would go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub job_id: String,
    pub node: String,
    pub archetype: String,
    /// RFC3339 UTC.
    pub claimed_at: String,
    /// RFC3339 UTC. Past this, the claim is ignored — a crashed node must not
    /// park a card forever.
    pub expires_at: String,
}

/// What a [`claim_cards`] call actually got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimOutcome {
    /// Cards this job may now work on.
    pub granted: Vec<String>,
    /// Cards a *different* live job holds, with that claim, so the caller can
    /// say who has it rather than just skipping silently.
    pub held_by_others: Vec<(String, Claim)>,
}

fn claim_file_name(card_id: &str) -> String {
    format!("{card_id}.claim")
}

/// RFC3339 UTC strings sort lexicographically iff they share an offset. Every
/// writer here emits `+00:00`, so string comparison is the expiry check and no
/// date library is needed.
fn is_live(c: &Claim, now: &str) -> bool {
    c.expires_at.as_str() > now
}

/// Claim `cards` for one job. Cards held by a different live job are reported,
/// not taken. Re-claiming your own card is idempotent, so a resumed job does
/// not deadlock against itself.
pub fn claim_cards(
    dir: &Path,
    cards: &[String],
    c: &Claim,
    now: &str,
) -> Result<ClaimOutcome, String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("failed to create claim directory {}: {e}", dir.display()))?;

    let mut granted = Vec::new();
    let mut held_by_others = Vec::new();

    for card in cards {
        let path = dir.join(claim_file_name(card));
        if path.exists() {
            let existing = read_claim_file(&path)?;
            if is_live(&existing, now) && existing.job_id != c.job_id {
                held_by_others.push((card.clone(), existing));
                continue;
            }
        }
        let text = serde_json::to_string_pretty(c)
            .map_err(|e| format!("failed to serialize claim: {e}"))?;
        std::fs::write(&path, format!("{text}\n"))
            .map_err(|e| format!("failed to write claim {}: {e}", path.display()))?;
        granted.push(card.clone());
    }

    Ok(ClaimOutcome {
        granted,
        held_by_others,
    })
}

/// Release this job's claims on `cards`. Returns how many were removed.
/// Another job's claim is never removed — releasing is not a stealing primitive.
pub fn release_cards(dir: &Path, cards: &[String], job_id: &str) -> Result<usize, String> {
    let mut removed = 0;
    for card in cards {
        let path = dir.join(claim_file_name(card));
        if !path.exists() {
            continue;
        }
        let existing = read_claim_file(&path)?;
        if existing.job_id != job_id {
            continue;
        }
        std::fs::remove_file(&path)
            .map_err(|e| format!("failed to remove claim {}: {e}", path.display()))?;
        removed += 1;
    }
    Ok(removed)
}

/// Every **live** claim, keyed by card id. Expired claims are omitted.
/// A missing directory is no claims, not an error.
pub fn read_claims(dir: &Path, now: &str) -> Result<BTreeMap<String, Claim>, String> {
    let mut out = BTreeMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read claim directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("claim") {
            continue;
        }
        let card = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let c = read_claim_file(&path)?;
        if is_live(&c, now) {
            out.insert(card, c);
        }
    }
    Ok(out)
}

fn read_claim_file(path: &Path) -> Result<Claim, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read claim {}: {e}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("invalid claim file {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(clause: &str, after: &str) -> Attempt {
        Attempt {
            ts: "2026-08-27T12:00:00+00:00".to_string(),
            job_id: "hunters-01".to_string(),
            node: "oracle-a".to_string(),
            archetype: "Hunters".to_string(),
            card: clause.split('#').next().unwrap().to_string(),
            clause: clause.to_string(),
            verdict_before: "unmeasured".to_string(),
            verdict_after: after.to_string(),
            scenario: Some("qa/dcgo-exams/BT12/x.yaml".to_string()),
            dcgo_build: Some("638f4070".to_string()),
            outcome: "oracle_clean".to_string(),
        }
    }

    #[test]
    fn append_then_read_round_trips() {
        let tmp = std::env::temp_dir().join("exam_log_round_trip.jsonl");
        let _ = std::fs::remove_file(&tmp);
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        append_attempt(&tmp, &attempt("BT12-043#effect#0", "diverged")).unwrap();

        let back = read_attempts(&tmp).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].clause, "BT12-042#effect#0");
        assert_eq!(back[1].verdict_after, "diverged");
    }

    #[test]
    fn each_attempt_is_exactly_one_line() {
        // Line-orientation is what makes a union merge driver correct. A
        // pretty-printed record would merge into unparseable interleaving.
        let tmp = std::env::temp_dir().join("exam_log_one_line.jsonl");
        let _ = std::fs::remove_file(&tmp);
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        append_attempt(&tmp, &attempt("BT12-043#effect#0", "confirmed")).unwrap();

        let text = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.ends_with('\n'), "trailing newline keeps appends clean");
        for line in text.lines() {
            serde_json::from_str::<Attempt>(line).expect("each line parses alone");
        }
    }

    #[test]
    fn append_creates_parent_directories() {
        let tmp = std::env::temp_dir()
            .join("exam_log_nested")
            .join("deeper")
            .join("exam-log.jsonl");
        let _ = std::fs::remove_dir_all(tmp.parent().unwrap().parent().unwrap());
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        assert!(tmp.exists());
    }

    #[test]
    fn read_attempts_missing_file_is_empty_not_an_error() {
        let tmp = std::env::temp_dir().join("exam_log_absent.jsonl");
        let _ = std::fs::remove_file(&tmp);
        assert!(read_attempts(&tmp).unwrap().is_empty());
    }

    #[test]
    fn read_attempts_names_the_line_it_could_not_parse() {
        let tmp = std::env::temp_dir().join("exam_log_corrupt.jsonl");
        let _ = std::fs::remove_file(&tmp);
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        let mut text = std::fs::read_to_string(&tmp).unwrap();
        text.push_str("{not json}\n");
        std::fs::write(&tmp, text).unwrap();

        let err = read_attempts(&tmp).expect_err("a corrupt line must be reported");
        assert!(err.contains("line 2"), "error names the line: {err}");
    }

    fn claim(job: &str, expires: &str) -> Claim {
        Claim {
            job_id: job.to_string(),
            node: "oracle-a".to_string(),
            archetype: "Hunters".to_string(),
            claimed_at: "2026-08-27T12:00:00+00:00".to_string(),
            expires_at: expires.to_string(),
        }
    }

    #[test]
    fn claiming_free_cards_grants_all_of_them() {
        let dir = std::env::temp_dir().join("exam_claims_free");
        let _ = std::fs::remove_dir_all(&dir);
        let cards = vec!["BT12-042".to_string(), "BT12-043".to_string()];

        let out = claim_cards(&dir, &cards, &claim("hunters-01", "2026-08-28T12:00:00+00:00"),
                              "2026-08-27T12:00:00+00:00").unwrap();

        assert_eq!(out.granted, cards);
        assert!(out.held_by_others.is_empty());
        assert!(dir.join("BT12-042.claim").exists());
    }

    #[test]
    fn a_card_held_by_another_live_job_is_not_granted() {
        // The overlap case that matters: Beelstar's EX7 cards are a strict
        // subset of Three Musketeers'. Two archetype jobs WILL ask for the
        // same card.
        let dir = std::env::temp_dir().join("exam_claims_contended");
        let _ = std::fs::remove_dir_all(&dir);
        let now = "2026-08-27T12:00:00+00:00";
        claim_cards(&dir, &["EX7-005".to_string()], &claim("musketeers-01", "2026-08-28T12:00:00+00:00"), now).unwrap();

        let out = claim_cards(
            &dir,
            &["EX7-005".to_string(), "EX7-008".to_string()],
            &claim("beelstar-01", "2026-08-28T12:00:00+00:00"),
            now,
        )
        .unwrap();

        assert_eq!(out.granted, vec!["EX7-008".to_string()]);
        assert_eq!(out.held_by_others.len(), 1);
        assert_eq!(out.held_by_others[0].0, "EX7-005");
        assert_eq!(out.held_by_others[0].1.job_id, "musketeers-01");

        // Verify the other job's claim was left intact on disk.
        let live = read_claims(&dir, now).unwrap();
        assert!(live.contains_key("EX7-005"), "other job's claim survives on disk");
        assert_eq!(live["EX7-005"].job_id, "musketeers-01");
    }

    #[test]
    fn an_expired_claim_does_not_block() {
        // A node that crashed must not park a card forever.
        let dir = std::env::temp_dir().join("exam_claims_expired");
        let _ = std::fs::remove_dir_all(&dir);
        claim_cards(&dir, &["BT12-042".to_string()],
                    &claim("dead-job", "2026-08-27T00:00:00+00:00"),
                    "2026-08-26T12:00:00+00:00").unwrap();

        let out = claim_cards(&dir, &["BT12-042".to_string()],
                              &claim("live-job", "2026-08-29T00:00:00+00:00"),
                              "2026-08-28T12:00:00+00:00").unwrap();

        assert_eq!(out.granted, vec!["BT12-042".to_string()]);
        assert!(out.held_by_others.is_empty());
    }

    #[test]
    fn reclaiming_your_own_card_is_idempotent() {
        // Resuming a crashed job must not deadlock against itself.
        let dir = std::env::temp_dir().join("exam_claims_reentrant");
        let _ = std::fs::remove_dir_all(&dir);
        let now = "2026-08-27T12:00:00+00:00";
        let c = claim("hunters-01", "2026-08-28T12:00:00+00:00");
        claim_cards(&dir, &["BT12-042".to_string()], &c, now).unwrap();

        let out = claim_cards(&dir, &["BT12-042".to_string()], &c, now).unwrap();

        assert_eq!(out.granted, vec!["BT12-042".to_string()]);
        assert!(out.held_by_others.is_empty());
    }

    #[test]
    fn reclaiming_your_own_expired_claim_succeeds() {
        // Resuming a crashed job must not deadlock against itself. The machine
        // died, its lease lapsed, and it must be able to pick its own work back up.
        let dir = std::env::temp_dir().join("exam_claims_crash_resume");
        let _ = std::fs::remove_dir_all(&dir);
        let first_now = "2026-08-27T12:00:00+00:00";
        let second_now = "2026-08-28T12:00:00+00:00";

        // Claim with an expiry in the past relative to the second call.
        claim_cards(
            &dir,
            &["BT12-042".to_string()],
            &claim("hunters-01", "2026-08-27T11:00:00+00:00"),
            first_now,
        )
        .unwrap();

        // The same job tries to reclaim after the lease expires.
        let out = claim_cards(
            &dir,
            &["BT12-042".to_string()],
            &claim("hunters-01", "2026-08-28T12:00:00+00:00"),
            second_now,
        )
        .unwrap();

        assert_eq!(out.granted, vec!["BT12-042".to_string()]);
        assert!(out.held_by_others.is_empty());
    }

    #[test]
    fn release_only_removes_your_own_claims() {
        let dir = std::env::temp_dir().join("exam_claims_release");
        let _ = std::fs::remove_dir_all(&dir);
        let now = "2026-08-27T12:00:00+00:00";
        claim_cards(&dir, &["EX7-005".to_string()], &claim("musketeers-01", "2026-08-28T12:00:00+00:00"), now).unwrap();
        claim_cards(&dir, &["EX7-008".to_string()], &claim("beelstar-01", "2026-08-28T12:00:00+00:00"), now).unwrap();

        let removed = release_cards(&dir, &["EX7-005".to_string(), "EX7-008".to_string()], "beelstar-01").unwrap();

        assert_eq!(removed, 1, "only beelstar-01's claim is released");
        assert!(dir.join("EX7-005.claim").exists(), "another job's claim survives");
        assert!(!dir.join("EX7-008.claim").exists());
    }

    #[test]
    fn read_claims_hides_expired_ones() {
        let dir = std::env::temp_dir().join("exam_claims_read");
        let _ = std::fs::remove_dir_all(&dir);
        claim_cards(&dir, &["A-1".to_string()], &claim("live", "2026-08-29T00:00:00+00:00"),
                    "2026-08-27T12:00:00+00:00").unwrap();
        claim_cards(&dir, &["B-2".to_string()], &claim("dead", "2026-08-27T00:00:00+00:00"),
                    "2026-08-26T12:00:00+00:00").unwrap();

        let live = read_claims(&dir, "2026-08-28T12:00:00+00:00").unwrap();

        assert_eq!(live.keys().collect::<Vec<_>>(), vec!["A-1"]);
    }
}
