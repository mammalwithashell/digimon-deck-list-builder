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
use std::path::{Path, PathBuf};

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

    /// Load every `<CARD-ID>.json` under `dir` and merge them into one store.
    ///
    /// A **missing directory is not an error** — it is a fresh checkout, and
    /// every clause then honestly reports `unmeasured`. (Contrast [`load`],
    /// which takes an explicitly named file and must fail on a typo.)
    ///
    /// A row whose `card_id` does not match the file it was found in is
    /// rejected rather than merged: that is the per-card analogue of the
    /// key-vs-`clause_id` check in [`from_json`], and it catches a bad merge
    /// or a hand-edit that would otherwise file a verdict under a card that
    /// never earned it.
    pub fn load_dir(dir: &Path) -> Result<VerdictStore, String> {
        let mut merged = VerdictStore::default();
        if !dir.exists() {
            return Ok(merged);
        }
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("failed to read verdict directory {}: {e}", dir.display()))?;
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                paths.push(path);
            }
        }
        // Deterministic order so an error is reproducible.
        paths.sort();

        for path in paths {
            let expected_card = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let one = VerdictStore::load(&path)?;
            for (clause_id, cv) in one.clauses.into_iter() {
                if cv.card_id != expected_card {
                    return Err(format!(
                        "verdict file {} holds a verdict for card {:?} (clause {:?}); \
                         each file holds exactly one card's verdicts",
                        path.display(),
                        cv.card_id,
                        clause_id
                    ));
                }
                merged.clauses.insert(clause_id, cv);
            }
            if one.last_updated > merged.last_updated {
                merged.last_updated = one.last_updated;
            }
        }
        Ok(merged)
    }

    /// Write one file per card under `dir`.
    ///
    /// Disjoint writers never touch the same file, which is what makes two
    /// nodes' branches merge cleanly. Card files that no longer have any
    /// verdicts are removed, so a re-extraction that drops a card cannot
    /// leave a stale verdict behind to be read back later.
    pub fn save_dir(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("failed to create verdict directory {}: {e}", dir.display()))?;

        let mut by_card: BTreeMap<String, VerdictStore> = BTreeMap::new();
        for (clause_id, cv) in self.clauses.iter() {
            let per = by_card.entry(cv.card_id.clone()).or_default();
            per.clauses.insert(clause_id.clone(), cv.clone());
            if cv.recorded_at > per.last_updated {
                per.last_updated = cv.recorded_at.clone();
            }
        }

        for (card_id, per) in by_card.iter() {
            per.save(&dir.join(card_file_name(card_id)))?;
        }

        // Prune files for cards we no longer carry.
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("failed to read verdict directory {}: {e}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            if !by_card.contains_key(&stem) {
                std::fs::remove_file(&path).map_err(|e| {
                    format!("failed to prune stale verdict file {}: {e}", path.display())
                })?;
            }
        }
        Ok(())
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

// ---------------------------------------------------------------------------
// Clause text book — the bridge between a scenario's clause id and the clause
// text the verdict must be hashed against.
// ---------------------------------------------------------------------------

/// One clause as `clause_coverage extract` emitted it. Only the fields the
/// verdict store needs; the extract file carries more.
#[derive(Debug, Clone, Deserialize)]
pub struct ClauseText {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct ClauseTextFile {
    clauses: Vec<ClauseText>,
}

/// The clause-text index loaded from a `clause_coverage extract` output
/// (`{"clauses": [{"id": ..., "label": ..., "text": ...}, ...]}`).
///
/// The scenario file only names a clause **id**; the label and text live in
/// the extractor's output, which is also the denominator. Recording a verdict
/// for an id absent from this book is refused (the orphan-scenario rule): a
/// verdict keyed outside the denominator would count toward nothing while
/// looking like coverage.
#[derive(Debug, Clone)]
pub struct ClauseTextBook {
    by_id: BTreeMap<String, ClauseText>,
    source: String,
}

impl ClauseTextBook {
    pub fn load(path: &Path) -> Result<ClauseTextBook, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read clause-text file {}: {e}", path.display()))?;
        Self::from_json(&text, &path.display().to_string())
    }

    pub fn from_json(text: &str, source: &str) -> Result<ClauseTextBook, String> {
        let file: ClauseTextFile = serde_json::from_str(text)
            .map_err(|e| format!("invalid clause-text JSON ({source}): {e}"))?;
        let mut by_id = BTreeMap::new();
        for c in file.clauses {
            by_id.insert(c.id.clone(), c);
        }
        Ok(ClauseTextBook {
            by_id,
            source: source.to_string(),
        })
    }

    pub fn get(&self, clause_id: &str) -> Option<&ClauseText> {
        self.by_id.get(clause_id)
    }

    /// Every clause id in the book, sorted — the denominator for
    /// [`VerdictStore::summary`].
    pub fn clause_ids(&self) -> Vec<String> {
        self.by_id.keys().cloned().collect()
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// SHA-256 of a clause's text, lowercase hex — the drift fingerprint stored in
/// [`ClauseVerdict::text_sha256`].
pub fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    format!("{:x}", h.finalize())
}

/// File name a card's verdicts live in.
///
/// Card ids are already filesystem-safe (`[A-Z0-9-]`), so this is a plain
/// suffix rather than a sanitizer — if that ever stops being true, this is the
/// one place that has to learn about it.
pub fn card_file_name(card_id: &str) -> String {
    format!("{card_id}.json")
}

/// Record one scenario run's verdict, or refuse.
///
/// Refusal (the orphan-scenario rule): a clause id absent from `book` gets
/// **no** verdict and an error naming the id, because the book IS the
/// `clause_coverage` denominator — a verdict keyed outside it would silently
/// create a sixth, invisible class: a scenario that passes while covering
/// nothing. The store is left untouched on refusal.
pub fn record_scenario_verdict(
    store: &mut VerdictStore,
    book: &ClauseTextBook,
    clause_id: &str,
    verdict: Verdict,
    scenario_path: Option<String>,
    reason: Option<String>,
    recorded_at: String,
) -> Result<(), String> {
    let ct = book.get(clause_id).ok_or_else(|| {
        format!(
            "refusing to record a verdict for clause `{clause_id}`: it is not in the \
             clause-text file ({}). A verdict keyed outside the clause_coverage \
             denominator would count toward nothing (the orphan-scenario rule) -- \
             re-run `clause_coverage extract` to include the card, or fix the \
             scenario's `clause:`.",
            book.source()
        )
    })?;
    let card_id = clause_id
        .split('#')
        .next()
        .unwrap_or_default()
        .to_string();
    store.record(ClauseVerdict {
        clause_id: clause_id.to_string(),
        card_id,
        verdict,
        label: ct.label.clone(),
        text_sha256: sha256_hex(&ct.text),
        scenario_path,
        reason,
        dcgo_build: None,
        job_id: None,
        recorded_at,
    });
    Ok(())
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

    // --- record_scenario_verdict / ClauseTextBook --------------------------

    const EXTRACT_JSON: &str = r#"{
        "clauses": [
            {"id": "ST1-12#effect#0", "card_id": "ST1-12", "zone": "effect",
             "label": "[Your Turn]", "kind": "timing", "timings": ["Your Turn"],
             "keyword": null,
             "text": "[Your Turn] All of your Digimon get +1000 DP.",
             "source": "bundle"},
            {"id": "ST1-12#security#0", "card_id": "ST1-12", "zone": "security",
             "label": "[Security]", "kind": "timing", "timings": ["Security"],
             "keyword": null,
             "text": "[Security] Play this card without paying its memory cost.",
             "source": "bundle"}
        ]
    }"#;

    #[test]
    fn clause_text_book_parses_the_extract_shape() {
        let book = ClauseTextBook::from_json(EXTRACT_JSON, "test").unwrap();
        let ct = book.get("ST1-12#effect#0").expect("clause present");
        assert_eq!(ct.label, "[Your Turn]");
        assert!(ct.text.contains("+1000 DP"));
        assert_eq!(
            book.clause_ids(),
            vec!["ST1-12#effect#0".to_string(), "ST1-12#security#0".to_string()]
        );
    }

    #[test]
    fn recording_a_confirmed_verdict_hashes_the_book_text() {
        let book = ClauseTextBook::from_json(EXTRACT_JSON, "test").unwrap();
        let mut store = VerdictStore::default();
        record_scenario_verdict(
            &mut store,
            &book,
            "ST1-12#effect#0",
            Verdict::Confirmed,
            Some("qa/dcgo-exams/ST1/ST1-12.yaml".to_string()),
            None,
            "2026-08-22T00:00:00Z".to_string(),
        )
        .expect("clause is in the book");

        let cv = store.get("ST1-12#effect#0").unwrap();
        assert_eq!(cv.verdict, Verdict::Confirmed);
        assert_eq!(cv.card_id, "ST1-12");
        assert_eq!(cv.label, "[Your Turn]");
        assert_eq!(
            cv.text_sha256,
            sha256_hex("[Your Turn] All of your Digimon get +1000 DP."),
            "the stored sha must be the hash of the book's clause text"
        );
        // And the round trip validates against the same text.
        assert!(store
            .get_validated("ST1-12#effect#0", &cv.text_sha256.clone())
            .is_some());
    }

    #[test]
    fn a_diverged_verdict_carries_its_reason() {
        let book = ClauseTextBook::from_json(EXTRACT_JSON, "test").unwrap();
        let mut store = VerdictStore::default();
        record_scenario_verdict(
            &mut store,
            &book,
            "ST1-12#effect#0",
            Verdict::Diverged,
            None,
            Some("step 11: p1.memory ours=2 dcgo=1".to_string()),
            "2026-08-22T00:00:00Z".to_string(),
        )
        .unwrap();
        assert!(store
            .get("ST1-12#effect#0")
            .unwrap()
            .reason
            .as_ref()
            .unwrap()
            .contains("p1.memory"));
    }

    #[test]
    fn an_orphan_clause_id_is_refused_and_the_store_is_untouched() {
        // The orphan-scenario rule: a clause id outside the extract denominator
        // must never grow a verdict -- it would count toward nothing while
        // looking like coverage.
        let book = ClauseTextBook::from_json(EXTRACT_JSON, "test").unwrap();
        let mut store = VerdictStore::default();
        let err = record_scenario_verdict(
            &mut store,
            &book,
            "ST1-12#effect#7",
            Verdict::Confirmed,
            None,
            None,
            "2026-08-22T00:00:00Z".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("ST1-12#effect#7"), "got: {err}");
        assert!(store.is_empty(), "a refused verdict must not touch the store");
    }

    #[test]
    fn sha256_hex_is_the_standard_digest() {
        // Pin against a known vector so the fingerprint is stable across
        // writers (the Python side hashes the same text).
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn save_dir_writes_one_file_per_card() {
        let tmp = std::env::temp_dir().join("exam_verdicts_per_card");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.record(v("EX12-035#effect#1", Verdict::Unreachable, "sha-ex12-035"));
        store.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));

        store.save_dir(&tmp).expect("save_dir");

        assert!(tmp.join("EX12-035.json").exists());
        assert!(tmp.join("BT8-084.json").exists());
        let files: Vec<_> = std::fs::read_dir(&tmp).unwrap().filter_map(|e| e.ok()).collect();
        assert_eq!(files.len(), 2, "one file per card, not per clause");
    }

    #[test]
    fn load_dir_round_trips_every_row() {
        let tmp = std::env::temp_dir().join("exam_verdicts_round_trip");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.record(v("BT8-084#effect#0", Verdict::Diverged, "sha-bt8-084"));
        store.save_dir(&tmp).expect("save_dir");

        let back = VerdictStore::load_dir(&tmp).expect("load_dir");
        assert_eq!(back.len(), 2);
        assert_eq!(back.get("EX12-035#effect#0").unwrap().verdict, Verdict::Confirmed);
        assert_eq!(back.get("BT8-084#effect#0").unwrap().verdict, Verdict::Diverged);
    }

    #[test]
    fn load_dir_missing_directory_is_empty_not_an_error() {
        let tmp = std::env::temp_dir().join("exam_verdicts_absent_dir");
        let _ = std::fs::remove_dir_all(&tmp);
        let store = VerdictStore::load_dir(&tmp).expect("missing dir is a fresh checkout");
        assert!(store.is_empty());
    }

    #[test]
    fn load_dir_rejects_a_row_filed_under_the_wrong_card() {
        let tmp = std::env::temp_dir().join("exam_verdicts_misfiled");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        // BT8-084's verdict written into EX12-035.json: a hand-edit or a bad
        // merge. Silently accepting it would file a verdict under a card that
        // never earned it.
        let mut store = VerdictStore::default();
        store.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));
        std::fs::write(tmp.join("EX12-035.json"), store.to_json().unwrap()).unwrap();

        let err = VerdictStore::load_dir(&tmp).expect_err("must reject a misfiled row");
        assert!(err.contains("EX12-035"), "error names the file: {err}");
        assert!(err.contains("BT8-084"), "error names the offending card: {err}");
    }

    #[test]
    fn save_dir_removes_a_card_file_that_no_longer_has_verdicts() {
        let tmp = std::env::temp_dir().join("exam_verdicts_pruned");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));
        store.save_dir(&tmp).unwrap();

        // A re-extraction dropped BT8-084 entirely.
        let mut store2 = VerdictStore::default();
        store2.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store2.save_dir(&tmp).unwrap();

        assert!(tmp.join("EX12-035.json").exists());
        assert!(!tmp.join("BT8-084.json").exists(), "stale card file must be pruned");
    }

    #[test]
    fn card_file_name_is_the_card_id_plus_json() {
        assert_eq!(card_file_name("EX12-035"), "EX12-035.json");
        assert_eq!(card_file_name("P-130"), "P-130.json");
    }
}
