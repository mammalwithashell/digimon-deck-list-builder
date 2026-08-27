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
}
