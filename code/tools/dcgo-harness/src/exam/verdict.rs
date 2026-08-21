//! Per-clause exam verdicts and their on-disk record.
//!
//! The exam's unit of measurement is a **clause**, identified exactly as
//! `clause_coverage` identifies it: `{card_id}#{zone}#{idx}` (see
//! `code/tools/clause_coverage/models.py`). This module owns the durable
//! record of what the exam has actually established about each one.
//!
//! Two properties matter more than anything else here:
//!
//! 1. **The denominator is never dropped.** [`VerdictStore::summary`] is given
//!    the full list of clause ids under consideration, and a clause that has
//!    no stored verdict reports as [`Verdict::Unmeasured`] — it is never
//!    silently omitted. A coverage number computed only over the clauses we
//!    happened to examine is a number that always looks good and means
//!    nothing.
//! 2. **Clause ids are positional, so card-text drift invalidates them.**
//!    `#effect#2` is "the third clause of the effect box", not a stable name.
//!    An override edit or a re-scrape that inserts, removes, or rewords a
//!    clause silently re-points every later id at *different* text, and a
//!    stale `confirmed` would then vouch for a clause nobody examined. Each
//!    verdict therefore stores the `text_sha256` of the clause text it was
//!    recorded against; [`VerdictStore::get_validated`] returns `None` on a
//!    mismatch, and [`VerdictStore::summary`] counts the mismatch as
//!    `unmeasured` **and** bumps `invalidated`, so drift shows up in the
//!    report instead of being absorbed by it.
//!
//! The on-disk shape is `{"version": 1, "last_updated": "...", "clauses":
//! {...}}`, mirroring `qa/qa-reports/validated_cards_dsl.json`
//! (`version` / `last_updated` / `cards`) so the QA artifacts read alike.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The five exam classes. Every clause in the denominator lands in exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Our engine and DCGO agreed on every projected step of the scenario.
    Confirmed,
    /// The projections disagreed. A finding to triage — DCGO is source
    /// priority #2, below `general_rule.pdf`, so this is never by itself
    /// proof that our engine is the wrong one.
    Diverged,
    /// The clause exists but no legal line of play can reach it (or the
    /// scenario cannot be constructed), so it cannot be examined this way.
    Unreachable,
    /// The oracle cannot answer: DCGO has no script for this card, or the
    /// clause routes through a path the recorder structurally does not see.
    /// Distinguished from `Unmeasured` on purpose — carry the why in
    /// [`ClauseVerdict::reason`].
    Unavailable,
    /// No verdict on record: never authored, or invalidated by clause-text
    /// drift. This is the default for anything absent from the store.
    Unmeasured,
}

impl Verdict {
    /// Stable lowercase name, matching the serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Confirmed => "confirmed",
            Verdict::Diverged => "diverged",
            Verdict::Unreachable => "unreachable",
            Verdict::Unavailable => "unavailable",
            Verdict::Unmeasured => "unmeasured",
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One clause's exam record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClauseVerdict {
    /// `{card_id}#{zone}#{idx}` — from `clause_coverage`, never invented here.
    pub clause_id: String,
    /// The card the clause belongs to (the part before the first `#`).
    pub card_id: String,
    /// Which of the five classes this clause landed in.
    pub verdict: Verdict,
    /// Human label for the clause, e.g. `[On Play]`.
    pub label: String,
    /// SHA-256 of the clause text this verdict was recorded against. Drift in
    /// this value invalidates the verdict — see the module docs.
    pub text_sha256: String,
    /// Repo-relative path of the scenario that produced the verdict, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_path: Option<String>,
    /// Why, for the classes that need a why (`unavailable`, `unreachable`,
    /// `diverged`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Which DCGO build answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dcgo_build: Option<String>,
    /// Which harness job ran it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    /// RFC 3339 timestamp of the recording.
    pub recorded_at: String,
}

/// Counts over a supplied denominator. `total` is the size of that
/// denominator, and the five class counts always sum to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictSummary {
    pub confirmed: usize,
    pub diverged: usize,
    pub unreachable: usize,
    pub unavailable: usize,
    pub unmeasured: usize,
    pub total: usize,
    /// How many of the `unmeasured` are there *because* their stored verdict
    /// was invalidated by clause-text drift, rather than never having been
    /// authored. Not a sixth class — a subset of `unmeasured`.
    pub invalidated: usize,
}

impl VerdictSummary {
    /// One-line report. Always prints the denominator.
    pub fn describe(&self) -> String {
        format!(
            "{}/{} confirmed | {} diverged | {} unreachable | {} unavailable | \
             {} unmeasured ({} invalidated by clause-text drift)",
            self.confirmed,
            self.total,
            self.diverged,
            self.unreachable,
            self.unavailable,
            self.unmeasured,
            self.invalidated,
        )
    }
}

fn default_version() -> u32 {
    1
}

/// The durable per-clause verdict record.
///
/// `current_text_shas` is scratch state, not part of the file: callers load
/// the store, tell it what each clause's text hashes to *right now*, and the
/// summary then reports drifted verdicts as `unmeasured`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerdictStore {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    clauses: BTreeMap<String, ClauseVerdict>,
    #[serde(skip)]
    current_text_shas: BTreeMap<String, String>,
}

impl Default for VerdictStore {
    fn default() -> Self {
        VerdictStore {
            version: 1,
            last_updated: String::new(),
            clauses: BTreeMap::new(),
            current_text_shas: BTreeMap::new(),
        }
    }
}

impl VerdictStore {
    /// Load from disk. A missing file is an error — callers that want
    /// "start empty" should check for the file and use [`Default`], so a
    /// typo'd path can never masquerade as an empty store.
    pub fn load(path: &Path) -> Result<VerdictStore, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read verdict store {}: {e}", path.display()))?;
        VerdictStore::from_json(&text)
            .map_err(|e| format!("failed to parse verdict store {}: {e}", path.display()))
    }

    /// Write to disk, creating parent directories.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "failed to create {} for verdict store: {e}",
                        parent.display()
                    )
                })?;
            }
        }
        let mut text = self.to_json()?;
        text.push('\n');
        std::fs::write(path, text)
            .map_err(|e| format!("failed to write verdict store {}: {e}", path.display()))
    }

    /// Serialize to the `{version, last_updated, clauses}` JSON shape.
    pub fn to_json(&self) -> Result<String, String> {
        let mut snapshot = self.clone();
        if snapshot.last_updated.is_empty() {
            snapshot.last_updated = chrono::Utc::now().to_rfc3339();
        }
        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("failed to serialize verdict store: {e}"))
    }

    /// Parse the `{version, last_updated, clauses}` JSON shape.
    pub fn from_json(text: &str) -> Result<VerdictStore, String> {
        let mut store: VerdictStore =
            serde_json::from_str(text).map_err(|e| format!("invalid verdict store JSON: {e}"))?;
        // The clause id is the map key; keep the embedded copy honest so a
        // hand-edited file cannot hand out a verdict labelled with someone
        // else's clause id.
        for (key, cv) in store.clauses.iter_mut() {
            if cv.clause_id.is_empty() {
                cv.clause_id = key.clone();
            } else if &cv.clause_id != key {
                return Err(format!(
                    "verdict store key {key:?} disagrees with its clause_id {:?}",
                    cv.clause_id
                ));
            }
        }
        Ok(store)
    }

    /// Insert or replace one clause's verdict.
    pub fn record(&mut self, v: ClauseVerdict) {
        self.last_updated = v.recorded_at.clone();
        self.clauses.insert(v.clause_id.clone(), v);
    }

    /// The stored verdict, drift or no drift. Use [`Self::get_validated`]
    /// when the answer will be trusted.
    pub fn get(&self, clause_id: &str) -> Option<&ClauseVerdict> {
        self.clauses.get(clause_id)
    }

    /// The stored verdict **only if** it was recorded against the same clause
    /// text the caller is looking at now. `None` on drift.
    pub fn get_validated(
        &self,
        clause_id: &str,
        current_text_sha256: &str,
    ) -> Option<&ClauseVerdict> {
        self.clauses
            .get(clause_id)
            .filter(|cv| cv.text_sha256 == current_text_sha256)
    }

    /// Tell the store what a clause's text hashes to right now, so
    /// [`Self::summary`] can spot drift.
    pub fn set_current_text_sha(&mut self, clause_id: &str, sha: &str) {
        self.current_text_shas
            .insert(clause_id.to_string(), sha.to_string());
    }

    /// True when a stored verdict was recorded against different clause text
    /// than the one most recently supplied via [`Self::set_current_text_sha`].
    pub fn is_invalidated(&self, clause_id: &str) -> bool {
        match (
            self.clauses.get(clause_id),
            self.current_text_shas.get(clause_id),
        ) {
            (Some(cv), Some(current)) => &cv.text_sha256 != current,
            _ => false,
        }
    }

    /// Every stored verdict, in clause-id order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ClauseVerdict)> {
        self.clauses.iter()
    }

    /// How many verdicts are stored. Note this is **not** the denominator —
    /// that is supplied by the caller of [`Self::summary`].
    pub fn len(&self) -> usize {
        self.clauses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Count the five classes over `all_clause_ids` — the denominator.
    ///
    /// A clause with no stored verdict, and a clause whose stored verdict was
    /// invalidated by text drift, both count as `unmeasured`; the drifted ones
    /// additionally count in `invalidated`.
    pub fn summary(&self, all_clause_ids: &[String]) -> VerdictSummary {
        let mut sum = VerdictSummary {
            total: all_clause_ids.len(),
            ..Default::default()
        };
        for id in all_clause_ids {
            let verdict = match self.clauses.get(id) {
                None => Verdict::Unmeasured,
                Some(_) if self.is_invalidated(id) => {
                    sum.invalidated += 1;
                    Verdict::Unmeasured
                }
                Some(cv) => cv.verdict,
            };
            match verdict {
                Verdict::Confirmed => sum.confirmed += 1,
                Verdict::Diverged => sum.diverged += 1,
                Verdict::Unreachable => sum.unreachable += 1,
                Verdict::Unavailable => sum.unavailable += 1,
                Verdict::Unmeasured => sum.unmeasured += 1,
            }
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(clause: &str, verdict: Verdict, sha: &str) -> ClauseVerdict {
        ClauseVerdict {
            clause_id: clause.to_string(),
            card_id: clause.split('#').next().unwrap().to_string(),
            verdict,
            label: "[On Play]".to_string(),
            text_sha256: sha.to_string(),
            scenario_path: None,
            reason: None,
            dcgo_build: None,
            job_id: None,
            recorded_at: "2026-08-21T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn records_and_reads_back() {
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "abc"));
        assert_eq!(
            s.get("EX12-035#effect#0").unwrap().verdict,
            Verdict::Confirmed
        );
    }

    #[test]
    fn a_clause_absent_from_the_store_is_unmeasured_not_missing() {
        // The denominator is the point: an unauthored clause must appear in the
        // report as `unmeasured`, never be silently omitted.
        let s = VerdictStore::default();
        let sum = s.summary(&[
            "EX12-035#effect#0".to_string(),
            "EX12-035#security#0".to_string(),
        ]);
        assert_eq!(sum.unmeasured, 2);
        assert_eq!(sum.total, 2);
        assert_eq!(sum.confirmed, 0);
    }

    #[test]
    fn summary_totals_every_class() {
        let mut s = VerdictStore::default();
        s.record(v("A#effect#0", Verdict::Confirmed, "x"));
        s.record(v("B#effect#0", Verdict::Diverged, "x"));
        s.record(v("C#effect#0", Verdict::Unavailable, "x"));
        let ids: Vec<String> = ["A#effect#0", "B#effect#0", "C#effect#0", "D#effect#0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let sum = s.summary(&ids);
        assert_eq!(
            (sum.confirmed, sum.diverged, sum.unavailable, sum.unmeasured),
            (1, 1, 1, 1)
        );
        assert_eq!(sum.total, 4);
    }

    #[test]
    fn a_verdict_whose_clause_text_changed_is_invalidated() {
        // Clause ids are positional within a zone, so an override or re-scrape
        // that changes a card's text silently re-points every later id at a
        // DIFFERENT clause. A stale `confirmed` would then vouch for a clause
        // nobody examined.
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "old-sha"));
        assert!(s.get_validated("EX12-035#effect#0", "old-sha").is_some());
        assert!(
            s.get_validated("EX12-035#effect#0", "new-sha").is_none(),
            "text drift must invalidate the verdict"
        );
    }

    #[test]
    fn invalidated_verdicts_count_as_unmeasured_in_the_summary() {
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "old-sha"));
        s.set_current_text_sha("EX12-035#effect#0", "new-sha");
        let sum = s.summary(&["EX12-035#effect#0".to_string()]);
        assert_eq!(sum.confirmed, 0);
        assert_eq!(sum.unmeasured, 1);
        assert_eq!(sum.invalidated, 1);
    }

    #[test]
    fn round_trips_through_json() {
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "abc"));
        let text = s.to_json().unwrap();
        let back = VerdictStore::from_json(&text).unwrap();
        assert_eq!(
            back.get("EX12-035#effect#0").unwrap().verdict,
            Verdict::Confirmed
        );
    }

    #[test]
    fn unavailable_carries_a_reason() {
        // "DCGO has no script for this card" must be distinguishable from
        // "we never got around to it" in the stored data, not just in prose.
        let mut cv = v("BT27-001#effect#0", Verdict::Unavailable, "x");
        cv.reason = Some("no DCGO script at BT27/Red/BT27_001.cs".to_string());
        let mut s = VerdictStore::default();
        s.record(cv);
        assert!(s
            .get("BT27-001#effect#0")
            .unwrap()
            .reason
            .as_ref()
            .unwrap()
            .contains("BT27_001.cs"));
    }
}
