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
//!
//! **Version 2** (card-loop design D8) adds an `interactions` map beside
//! `clauses`, one [`InteractionVerdict`] per interaction id (`qa:<Q>`,
//! `probe:<clause>:<family>[:neg]`, `combo:<slug>`). A file is written as v2
//! ONLY when its card carries an interaction verdict, so the existing v1 files
//! are never rewritten just because the reader learned v2. An interaction that
//! counts for several cards (one official ruling printed on two cards) is
//! written, identically, into EVERY one of those cards' files: a card stays the
//! unit of a file, which is what keeps disjoint fleet writers merge-clean, and
//! the loader refuses two copies of one interaction that disagree.

use std::collections::{BTreeMap, BTreeSet};
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

/// Whose bug a `diverged` clause is. `general_rule.pdf` outranks DCGO, so a
/// divergence is not automatically ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Triage {
    /// Our engine is wrong; the rules or DCGO's correct behavior is cited.
    OursWrong,
    /// DCGO is wrong or differs only in a rules-neutral way; cited.
    DcgoQuirk,
    /// Not yet decided. Blocks readiness.
    Undetermined,
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
    /// Whose bug a `diverged` clause is, once triaged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triage: Option<Triage>,
    /// Evidence for the triage class (rules section, DCGO file:line, or
    /// gap-tracker id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    /// RFC 3339 timestamp of the recording.
    pub recorded_at: String,
}

/// One interaction's exam record (verdict-store v2): the [`ClauseVerdict`]
/// shape, keyed by interaction id instead of clause id, and naming every card
/// it counts for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionVerdict {
    /// `qa:<Q>` / `probe:<clause>:<family>[:neg]` / `combo:<slug>` -- from the
    /// interaction denominator (`data/interaction_denominator.json`), never
    /// invented here (combo ids excepted: they are never gating).
    pub interaction_id: String,
    /// Every card this verdict counts for; the record is filed under each.
    pub card_ids: Vec<String>,
    /// `qa` | `probe` | `combo`.
    pub source: String,
    /// `positive` | `negative`.
    pub kind: String,
    pub verdict: Verdict,
    /// The denominator's drift fingerprint for this interaction (the ruling's
    /// question + answer, or the probed clause's text). A mismatch reports the
    /// verdict `unmeasured`, exactly like a clause.
    pub text_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dcgo_build: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub recorded_at: String,
    /// Fields this reader does not model (a triage block, a citation,
    /// `produced_by` provenance) are carried through a load/save round trip
    /// rather than silently dropped.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
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

/// The verdict-file versions this reader understands.
pub const SUPPORTED_VERSIONS: &[u32] = &[1, 2];

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
    /// v2 only; omitted from a file that has none, which is then written as v1.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    interactions: BTreeMap<String, InteractionVerdict>,
    #[serde(skip)]
    current_text_shas: BTreeMap<String, String>,
    /// Card ids whose rows changed since load; `save_dir` writes only these.
    #[serde(skip)]
    dirty: BTreeSet<String>,
}

impl Default for VerdictStore {
    fn default() -> Self {
        VerdictStore {
            version: 1,
            last_updated: String::new(),
            clauses: BTreeMap::new(),
            interactions: BTreeMap::new(),
            current_text_shas: BTreeMap::new(),
            dirty: BTreeSet::new(),
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
        // Keep the file's existing line endings: core.autocrlf checkouts hold
        // CRLF, and an LF rewrite shows up as a modified file for no reason.
        let existing_crlf = std::fs::read(path)
            .map(|b| b.windows(2).any(|w| w == b"\r\n"))
            .unwrap_or(false);
        if existing_crlf {
            text = text.replace('\n', "\r\n");
        }
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

        // Which file each merged interaction came from, for the disagreement
        // message below.
        let mut interaction_origin: BTreeMap<String, PathBuf> = BTreeMap::new();
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
            for (id, iv) in one.interactions.into_iter() {
                // Same misfiling rule as clauses: a card's file only holds
                // interactions that count for that card.
                if !iv.card_ids.iter().any(|c| *c == expected_card) {
                    return Err(format!(
                        "verdict file {} holds interaction {id:?}, which counts for {:?} \
                         but not for {expected_card:?}; each file holds exactly one card's \
                         verdicts",
                        path.display(),
                        iv.card_ids
                    ));
                }
                // A shared ruling is written into every card's file in one
                // commit, so its copies must agree; two that differ mean a
                // partial write or a bad merge, and picking one would hide it.
                if let Some(existing) = merged.interactions.get(&id) {
                    if *existing != iv {
                        return Err(format!(
                            "interaction {id:?} has disagreeing copies in {} and {}; a shared \
                             verdict must be written identically into every listed card's file",
                            interaction_origin[&id].display(),
                            path.display()
                        ));
                    }
                    continue;
                }
                interaction_origin.insert(id.clone(), path.clone());
                merged.interactions.insert(id, iv);
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
        // A shared interaction lands in EVERY card file it counts for.
        for (id, iv) in self.interactions.iter() {
            for card_id in &iv.card_ids {
                let per = by_card.entry(card_id.clone()).or_default();
                per.interactions.insert(id.clone(), iv.clone());
                if iv.recorded_at > per.last_updated {
                    per.last_updated = iv.recorded_at.clone();
                }
            }
        }

        for (card_id, per) in by_card.iter() {
            let file = dir.join(card_file_name(card_id));
            // Only cards whose rows changed this run are rewritten; a file that
            // does not exist yet is always written.
            if self.dirty.contains(card_id) || !file.exists() {
                per.save(&file)?;
            }
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
            // Guard the only unbounded delete in this module: only ever prune
            // a file whose stem is shaped like a card id. A `*.json` file
            // that doesn't match (a note, a README export, anything a human
            // dropped into the directory) is left alone rather than deleted,
            // even though it isn't in `by_card` either.
            if !by_card.contains_key(&stem) && looks_like_card_id(&stem) {
                std::fs::remove_file(&path).map_err(|e| {
                    format!("failed to prune stale verdict file {}: {e}", path.display())
                })?;
            }
        }
        Ok(())
    }

    /// Serialize to the `{version, last_updated, clauses[, interactions]}`
    /// JSON shape: version 1 when there are no interaction verdicts (so a
    /// clause-only file reads exactly as before), version 2 otherwise.
    pub fn to_json(&self) -> Result<String, String> {
        let mut snapshot = self.clone();
        if snapshot.last_updated.is_empty() {
            snapshot.last_updated = chrono::Utc::now().to_rfc3339();
        }
        snapshot.version = if snapshot.interactions.is_empty() { 1 } else { 2 };
        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("failed to serialize verdict store: {e}"))
    }

    /// Parse the v1 `{version, last_updated, clauses}` or v2 `{..., interactions}`
    /// JSON shape.
    pub fn from_json(text: &str) -> Result<VerdictStore, String> {
        let mut store: VerdictStore =
            serde_json::from_str(text).map_err(|e| format!("invalid verdict store JSON: {e}"))?;
        if !SUPPORTED_VERSIONS.contains(&store.version) {
            return Err(format!(
                "verdict store version {} is not one this reader understands ({SUPPORTED_VERSIONS:?})",
                store.version
            ));
        }
        if store.version == 1 && !store.interactions.is_empty() {
            return Err("a version-1 verdict store cannot carry `interactions` (that is v2)"
                .to_string());
        }
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
        for (key, iv) in store.interactions.iter_mut() {
            if iv.interaction_id.is_empty() {
                iv.interaction_id = key.clone();
            } else if &iv.interaction_id != key {
                return Err(format!(
                    "verdict store key {key:?} disagrees with its interaction_id {:?}",
                    iv.interaction_id
                ));
            }
            if iv.card_ids.is_empty() {
                return Err(format!(
                    "interaction verdict {key:?} names no card: it would count for nothing"
                ));
            }
        }
        Ok(store)
    }

    /// Insert or replace one clause's verdict, marking its card file dirty.
    pub fn record(&mut self, v: ClauseVerdict) {
        self.last_updated = v.recorded_at.clone();
        self.dirty.insert(v.card_id.clone());
        self.clauses.insert(v.clause_id.clone(), v);
    }

    /// Classify a `diverged` row. `ours_wrong` and `dcgo_quirk` need a citation
    /// (a `general_rule.pdf` section, a DCGO `file:line`, or a gap-tracker entry
    /// that carries one); `undetermined` does not. Marks the card dirty.
    pub fn set_triage(
        &mut self,
        clause_id: &str,
        triage: Triage,
        citation: Option<String>,
    ) -> Result<(), String> {
        let citation = citation.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
        if matches!(triage, Triage::OursWrong | Triage::DcgoQuirk) && citation.is_none() {
            return Err(format!(
                "triage {triage:?} for `{clause_id}` needs --citation (rules section, \
                 DCGO file:line, or gap-tracker id)"
            ));
        }
        let row = self
            .clauses
            .get_mut(clause_id)
            .ok_or_else(|| format!("no stored verdict for `{clause_id}`"))?;
        if row.verdict != Verdict::Diverged {
            return Err(format!(
                "`{clause_id}` is {} -- not diverged; only a diverged row is triaged",
                row.verdict
            ));
        }
        row.triage = Some(triage);
        row.citation = citation;
        self.dirty.insert(row.card_id.clone());
        Ok(())
    }

    /// Stamp the oracle provenance (which harness job ran it, which DCGO build
    /// answered) on a recorded clause or interaction row, and mark its card(s)
    /// dirty so the stamp is written.
    pub fn set_provenance(
        &mut self,
        id: &str,
        job_id: Option<String>,
        dcgo_build: Option<String>,
    ) -> Result<(), String> {
        if let Some(row) = self.clauses.get_mut(id) {
            row.job_id = job_id;
            row.dcgo_build = dcgo_build;
            self.dirty.insert(row.card_id.clone());
            return Ok(());
        }
        let row = self
            .interactions
            .get_mut(id)
            .ok_or_else(|| format!("no stored verdict for `{id}`"))?;
        row.job_id = job_id;
        row.dcgo_build = dcgo_build;
        self.dirty.extend(row.card_ids.iter().cloned());
        Ok(())
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
    ///
    /// Also answers for interaction ids: the two id spaces cannot collide
    /// (`qa:` / `probe:` / `combo:` vs `card#zone#idx`), so one scratch map
    /// serves both.
    pub fn is_invalidated(&self, clause_id: &str) -> bool {
        let Some(current) = self.current_text_shas.get(clause_id) else {
            return false;
        };
        if let Some(cv) = self.clauses.get(clause_id) {
            return &cv.text_sha256 != current;
        }
        match self.interactions.get(clause_id) {
            Some(iv) => &iv.text_sha256 != current,
            None => false,
        }
    }

    /// Insert or replace one interaction's verdict, marking every card file it
    /// is filed under dirty (`save_dir` writes only dirty cards).
    pub fn record_interaction(&mut self, v: InteractionVerdict) {
        self.last_updated = v.recorded_at.clone();
        self.dirty.extend(v.card_ids.iter().cloned());
        self.interactions.insert(v.interaction_id.clone(), v);
    }

    /// The stored interaction verdict, drift or no drift.
    pub fn get_interaction(&self, interaction_id: &str) -> Option<&InteractionVerdict> {
        self.interactions.get(interaction_id)
    }

    /// Every stored interaction verdict, in id order.
    pub fn iter_interactions(&self) -> impl Iterator<Item = (&String, &InteractionVerdict)> {
        self.interactions.iter()
    }

    /// How many interaction verdicts are stored (not a denominator either).
    pub fn interaction_count(&self) -> usize {
        self.interactions.len()
    }

    /// Count the five classes over `all_interaction_ids` -- the interaction
    /// denominator -- with the same unmeasured / invalidated rules as
    /// [`Self::summary`].
    pub fn interaction_summary(&self, all_interaction_ids: &[String]) -> VerdictSummary {
        let mut sum = VerdictSummary {
            total: all_interaction_ids.len(),
            ..Default::default()
        };
        for id in all_interaction_ids {
            let verdict = match self.interactions.get(id) {
                None => Verdict::Unmeasured,
                Some(_) if self.is_invalidated(id) => {
                    sum.invalidated += 1;
                    Verdict::Unmeasured
                }
                Some(iv) => iv.verdict,
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
    /// Cards the extract covered -- including those with zero clauses, which
    /// the `clauses` list alone cannot show.
    #[serde(default)]
    cards: Vec<String>,
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
    /// Every card the book covers: the file's `cards` list plus the card
    /// prefix of every clause id.
    cards: std::collections::BTreeSet<String>,
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
        let mut cards: std::collections::BTreeSet<String> = file.cards.into_iter().collect();
        for c in file.clauses {
            if let Some(card) = c.id.split('#').next() {
                cards.insert(card.to_string());
            }
            by_id.insert(c.id.clone(), c);
        }
        Ok(ClauseTextBook {
            by_id,
            cards,
            source: source.to_string(),
        })
    }

    /// True when the book's extract covered this card (even with zero clauses).
    pub fn has_card(&self, card_id: &str) -> bool {
        self.cards.contains(card_id)
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

/// Whether `stem` has the shape of a card id: uppercase ASCII letters/digits,
/// then a `-`, then one or more digits -- e.g. `EX12-004`, `BT8-084`,
/// `P-130`, `ST1-15`.
///
/// `VerdictStore::save_dir`'s prune step is the one unbounded delete in this
/// module (it removes any `*.json` in the directory that isn't a currently-
/// tracked card). This check is its guard rail: a file whose stem doesn't
/// look like a card id is skipped rather than deleted, so an unrelated file
/// dropped into the ledger directory can't be silently destroyed by a save.
fn looks_like_card_id(stem: &str) -> bool {
    let Some(dash) = stem.find('-') else {
        return false;
    };
    let (head, rest) = stem.split_at(dash);
    let tail = &rest[1..]; // drop the '-' itself
    if head.is_empty() || tail.is_empty() {
        return false;
    }
    let head_ok = head
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let tail_ok = tail.chars().all(|c| c.is_ascii_digit());
    head_ok && tail_ok
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
        triage: None,
        citation: None,
        recorded_at,
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Interaction denominator -- the bridge between a scenario's `interaction.id`
// and the cards / fingerprint its verdict must carry.
// ---------------------------------------------------------------------------

/// The committed interaction denominator, as
/// `python -m tools.card_loop interactions build` writes it.
pub const DEFAULT_INTERACTION_DENOMINATOR: &str = "data/interaction_denominator.json";

/// One denominator row. Only the fields the exam needs; the file carries more.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InteractionEntry {
    pub source: String,
    pub card_ids: Vec<String>,
    pub kind: String,
    #[serde(default)]
    pub gating: bool,
    #[serde(default)]
    pub text_sha256: String,
    #[serde(default)]
    pub clause_id: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub family_version: Option<String>,
    #[serde(default)]
    pub q_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InteractionFile {
    version: u32,
    interactions: BTreeMap<String, InteractionEntry>,
}

/// The interaction denominator, loaded. The orphan rule for interactions: a
/// `qa:` / `probe:` id absent from this book can neither validate nor be given
/// a verdict, for the same reason as an unknown clause id.
#[derive(Debug, Clone)]
pub struct InteractionBook {
    by_id: BTreeMap<String, InteractionEntry>,
    source: String,
}

impl InteractionBook {
    /// Load the denominator. A missing file is an error that says the
    /// denominator was never generated -- interaction scenarios are refused
    /// rather than examined against nothing; legacy clause scenarios never
    /// need this book.
    pub fn load(path: &Path) -> Result<InteractionBook, String> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            format!(
                "interaction denominator not generated: cannot read {} ({e}). Build it with \
                 `python -m tools.card_loop interactions build`",
                path.display()
            )
        })?;
        Self::from_json(&text, &path.display().to_string())
    }

    pub fn from_json(text: &str, source: &str) -> Result<InteractionBook, String> {
        let file: InteractionFile = serde_json::from_str(text)
            .map_err(|e| format!("invalid interaction denominator JSON ({source}): {e}"))?;
        if file.version != 1 {
            return Err(format!(
                "interaction denominator {source} is version {}, this reader knows 1",
                file.version
            ));
        }
        Ok(InteractionBook {
            by_id: file.interactions,
            source: source.to_string(),
        })
    }

    pub fn get(&self, interaction_id: &str) -> Option<&InteractionEntry> {
        self.by_id.get(interaction_id)
    }

    /// Every interaction id, sorted.
    pub fn ids(&self) -> Vec<String> {
        self.by_id.keys().cloned().collect()
    }

    /// The GATING interaction ids that count for `card_id`, sorted.
    pub fn gating_ids_for_card(&self, card_id: &str) -> Vec<String> {
        self.by_id
            .iter()
            .filter(|(_, e)| e.gating && e.card_ids.iter().any(|c| c == card_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Record one interaction scenario run's verdict, or refuse (the orphan rule).
///
/// A `qa:` / `probe:` id must be in `book`; the record takes its cards, kind and
/// fingerprint from there, so it is filed under every card the interaction
/// counts for. A `combo:` id is accepted without the book (combos never gate),
/// filed under `combo_cards`. The store is untouched on refusal.
#[allow(clippy::too_many_arguments)]
pub fn record_interaction_verdict(
    store: &mut VerdictStore,
    book: &InteractionBook,
    interaction_id: &str,
    combo_cards: &[String],
    verdict: Verdict,
    scenario_path: Option<String>,
    reason: Option<String>,
    recorded_at: String,
) -> Result<(), String> {
    let (card_ids, source, kind, text_sha256) = if interaction_id.starts_with("combo:") {
        if combo_cards.is_empty() {
            return Err(format!(
                "refusing to record combo interaction `{interaction_id}` under no card"
            ));
        }
        (
            combo_cards.to_vec(),
            "combo".to_string(),
            "positive".to_string(),
            String::new(),
        )
    } else {
        let e = book.get(interaction_id).ok_or_else(|| {
            format!(
                "refusing to record a verdict for interaction `{interaction_id}`: it is not in \
                 the interaction denominator ({}). A verdict keyed outside it would count toward \
                 nothing (the orphan rule) -- rebuild the denominator, or fix the scenario's \
                 `interaction.id`.",
                book.source()
            )
        })?;
        (
            e.card_ids.clone(),
            e.source.clone(),
            e.kind.clone(),
            e.text_sha256.clone(),
        )
    };
    store.record_interaction(InteractionVerdict {
        interaction_id: interaction_id.to_string(),
        card_ids,
        source,
        kind,
        verdict,
        text_sha256,
        scenario_path,
        reason,
        dcgo_build: None,
        job_id: None,
        recorded_at,
        extra: BTreeMap::new(),
    });
    Ok(())
}

/// Write one (possibly shared) interaction verdict into EVERY listed card's
/// file under `dir`, leaving each file's other verdicts as they were.
///
/// Returns the files written, so the caller commits them together (design D8:
/// "a shared Q&A verdict is written into every card file that prints it in the
/// same commit"). Each card's file is loaded first (or started empty), so this
/// never prunes or rewrites another card's verdicts the way a whole-store
/// [`VerdictStore::save_dir`] from a partial store would.
pub fn write_interaction_to_card_files(
    dir: &Path,
    v: &InteractionVerdict,
) -> Result<Vec<PathBuf>, String> {
    if v.card_ids.is_empty() {
        return Err(format!(
            "interaction verdict {:?} names no card to file it under",
            v.interaction_id
        ));
    }
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("failed to create verdict directory {}: {e}", dir.display()))?;
    let mut written = Vec::new();
    for card_id in &v.card_ids {
        let path = dir.join(card_file_name(card_id));
        let mut one = if path.exists() {
            VerdictStore::load(&path)?
        } else {
            VerdictStore::default()
        };
        one.record_interaction(v.clone());
        // `record_interaction` stamps last_updated from this verdict; keep the
        // file's newest timestamp instead when it already had a later one.
        let newest = one
            .clauses
            .values()
            .map(|c| c.recorded_at.as_str())
            .chain(one.interactions.values().map(|i| i.recorded_at.as_str()))
            .max()
            .unwrap_or_default()
            .to_string();
        one.last_updated = newest;
        one.save(&path)?;
        written.push(path);
    }
    Ok(written)
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
            triage: None,
            citation: None,
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
    fn book_knows_zero_clause_cards() {
        let book = ClauseTextBook::from_json(
            r#"{"cards":["BT1-009","ST1-07"],"clauses":[{"id":"ST1-07#inherited#0","label":"Inherited Effect","text":"x"}]}"#,
            "t",
        )
        .unwrap();
        assert!(book.has_card("BT1-009"));
        assert!(book.has_card("ST1-07"));
        assert!(!book.has_card("EX10-025"));
    }

    #[test]
    fn book_without_a_cards_array_still_knows_the_cards_of_its_clauses() {
        let book = ClauseTextBook::from_json(EXTRACT_JSON, "t").unwrap();
        assert!(book.has_card("ST1-12"));
        assert!(!book.has_card("ST1-13"));
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
    fn save_dir_never_deletes_a_file_that_doesnt_look_like_a_card_id() {
        let tmp = std::env::temp_dir().join("exam_verdicts_unrelated_file_survives");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        // An unrelated file a human dropped into the ledger directory --
        // its stem is not shaped like a card id, so the prune step (which
        // is otherwise an unbounded delete of anything untracked) must
        // leave it alone.
        std::fs::write(tmp.join("notes.json"), "{}").unwrap();

        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.save_dir(&tmp).unwrap();

        assert!(
            tmp.join("notes.json").exists(),
            "an unrelated *.json file must survive save_dir's prune step"
        );
        assert!(tmp.join("EX12-035.json").exists());
    }

    #[test]
    fn card_file_name_is_the_card_id_plus_json() {
        assert_eq!(card_file_name("EX12-035"), "EX12-035.json");
        assert_eq!(card_file_name("P-130"), "P-130.json");
    }

    #[test]
    fn migration_from_the_blob_preserves_every_row() {
        let tmp = std::env::temp_dir().join("exam_verdicts_migration");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let mut blob = VerdictStore::default();
        blob.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        blob.record(v("EX12-035#security#0", Verdict::Unreachable, "sha-ex12-035"));
        blob.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));
        let blob_path = tmp.join("dcgo_exam_verdicts.json");
        blob.save(&blob_path).unwrap();

        let loaded = VerdictStore::load(&blob_path).unwrap();
        let dir = tmp.join("exam-verdicts");
        loaded.save_dir(&dir).unwrap();
        let back = VerdictStore::load_dir(&dir).unwrap();

        assert_eq!(back.len(), blob.len(), "no row may be lost in migration");
        for (clause_id, before) in blob.iter() {
            let after = back.get(clause_id).expect("every clause survives");
            assert_eq!(after.verdict, before.verdict);
            assert_eq!(after.text_sha256, before.text_sha256);
            assert_eq!(after.recorded_at, before.recorded_at);
        }
    }

    #[test]
    fn save_dir_rewrites_only_touched_cards_and_keeps_crlf() {
        let dir = std::env::temp_dir().join("verdict-dirty-only-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let row = |card: &str| {
            format!(
                "{{\r\n  \"version\": 1,\r\n  \"last_updated\": \"2026-01-01T00:00:00Z\",\r\n  \
                 \"clauses\": {{\r\n    \"{card}#effect#0\": {{\r\n      \"clause_id\": \"{card}#effect#0\",\r\n      \
                 \"card_id\": \"{card}\",\r\n      \"verdict\": \"confirmed\",\r\n      \"label\": \"Effect\",\r\n      \
                 \"text_sha256\": \"abc\",\r\n      \"recorded_at\": \"2026-01-01T00:00:00Z\"\r\n    }}\r\n  }}\r\n}}\r\n"
            )
        };
        std::fs::write(dir.join("BT1-001.json"), row("BT1-001")).unwrap();
        std::fs::write(dir.join("BT1-002.json"), row("BT1-002")).unwrap();
        let untouched_before = std::fs::read(dir.join("BT1-002.json")).unwrap();

        let mut store = VerdictStore::load_dir(&dir).unwrap();
        store.record(ClauseVerdict {
            clause_id: "BT1-001#effect#0".into(),
            card_id: "BT1-001".into(),
            verdict: Verdict::Diverged,
            label: "Effect".into(),
            text_sha256: "abc".into(),
            scenario_path: None,
            reason: Some("DIVERGED at step 1".into()),
            dcgo_build: None,
            job_id: None,
            triage: None,
            citation: None,
            recorded_at: "2026-02-01T00:00:00Z".into(),
        });
        store.save_dir(&dir).unwrap();

        assert_eq!(std::fs::read(dir.join("BT1-002.json")).unwrap(), untouched_before);
        let touched = std::fs::read_to_string(dir.join("BT1-001.json")).unwrap();
        assert!(touched.contains("\"diverged\""));
        assert!(touched.contains("\r\n"), "an existing CRLF file must stay CRLF");
        assert!(!touched.replace("\r\n", "").contains('\n'), "no bare LF lines");
    }

    #[test]
    fn set_provenance_stamps_job_and_build_and_the_row_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = VerdictStore::default();
        store.record(diverged_row("BT1-001"));
        store.save_dir(dir.path()).unwrap();

        let mut store = VerdictStore::load_dir(dir.path()).unwrap();
        store
            .set_provenance("BT1-001#effect#0", Some("exam-BT1-001-effect0".into()), Some("8b6c39ea1".into()))
            .unwrap();
        store.save_dir(dir.path()).unwrap();

        let back = VerdictStore::load_dir(dir.path()).unwrap();
        let row = back.get("BT1-001#effect#0").unwrap();
        assert_eq!(row.job_id.as_deref(), Some("exam-BT1-001-effect0"));
        assert_eq!(row.dcgo_build.as_deref(), Some("8b6c39ea1"));
        assert!(store.set_provenance("BT9-999#effect#0", None, None).is_err());
    }

    pub(super) fn diverged_row_for(card: &str) -> ClauseVerdict {
        diverged_row(card)
    }

    fn diverged_row(card: &str) -> ClauseVerdict {
        ClauseVerdict {
            clause_id: format!("{card}#effect#0"),
            card_id: card.into(),
            verdict: Verdict::Diverged,
            label: "Effect".into(),
            text_sha256: "abc".into(),
            scenario_path: None,
            reason: Some("DIVERGED at step 3".into()),
            dcgo_build: None,
            job_id: None,
            triage: None,
            citation: None,
            recorded_at: "2026-02-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn set_triage_records_class_and_citation_on_a_diverged_row() {
        let mut store = VerdictStore::default();
        store.record(diverged_row("BT6-060"));
        store
            .set_triage("BT6-060#effect#0", Triage::DcgoQuirk, Some("G-EXAM-REVEAL-BUCKET-ADD-TIMING".into()))
            .unwrap();
        let row = store.get("BT6-060#effect#0").unwrap();
        assert_eq!(row.triage, Some(Triage::DcgoQuirk));
        assert_eq!(row.citation.as_deref(), Some("G-EXAM-REVEAL-BUCKET-ADD-TIMING"));
        let json = store.to_json().unwrap();
        assert!(json.contains("\"triage\": \"dcgo_quirk\""));
    }

    #[test]
    fn set_triage_refuses_quirk_or_ours_wrong_without_citation() {
        let mut store = VerdictStore::default();
        store.record(diverged_row("BT6-060"));
        assert!(store.set_triage("BT6-060#effect#0", Triage::DcgoQuirk, None).is_err());
        assert!(store.set_triage("BT6-060#effect#0", Triage::OursWrong, Some("  ".into())).is_err());
        assert!(store.set_triage("BT6-060#effect#0", Triage::Undetermined, None).is_ok());
    }

    #[test]
    fn set_triage_refuses_a_non_diverged_row() {
        let mut store = VerdictStore::default();
        let mut row = diverged_row("BT1-001");
        row.verdict = Verdict::Confirmed;
        store.record(row);
        let err = store
            .set_triage("BT1-001#effect#0", Triage::DcgoQuirk, Some("x".into()))
            .unwrap_err();
        assert!(err.contains("not diverged"), "got: {err}");
    }

    #[test]
    fn old_rows_without_triage_still_parse() {
        let text = r#"{"version":1,"last_updated":"x","clauses":{"BT1-001#effect#0":{
            "clause_id":"BT1-001#effect#0","card_id":"BT1-001","verdict":"diverged",
            "label":"Effect","text_sha256":"abc","recorded_at":"x"}}}"#;
        let store = VerdictStore::from_json(text).unwrap();
        assert_eq!(store.get("BT1-001#effect#0").unwrap().triage, None);
    }
}

#[cfg(test)]
mod interaction_store_tests {
    //! Verdict-store v2 (card-loop design D8): `interactions` beside `clauses`.
    use super::*;

    fn clause_row(clause: &str) -> ClauseVerdict {
        ClauseVerdict {
            clause_id: clause.to_string(),
            card_id: clause.split('#').next().unwrap().to_string(),
            verdict: Verdict::Confirmed,
            label: "Effect".to_string(),
            text_sha256: "c-sha".to_string(),
            scenario_path: None,
            reason: None,
            dcgo_build: None,
            job_id: None,
            recorded_at: "2026-10-01T00:00:00Z".to_string(),
            triage: None,
            citation: None,
        }
    }

    #[test]
    fn set_provenance_also_stamps_an_interaction_row() {
        let mut store = VerdictStore::default();
        store.record_interaction(iv("qa:Q1", &["BT1-001"], Verdict::Confirmed));
        store.set_provenance("qa:Q1", Some("exam-BT1-001-qa1".into()), None).unwrap();
        assert_eq!(store.get_interaction("qa:Q1").unwrap().job_id.as_deref(), Some("exam-BT1-001-qa1"));
    }

    #[test]
    fn an_interaction_verdict_for_a_card_that_already_has_a_file_is_saved() {
        // record_interaction must mark its cards dirty: save_dir rewrites only
        // dirty cards, so an interaction filed under a card that already has a
        // verdict file was reported "recorded" and silently never written.
        let dir = tempfile::tempdir().unwrap();
        let mut store = VerdictStore::default();
        store.record(super::tests::diverged_row_for("BT1-001"));
        store.save_dir(dir.path()).unwrap();

        let mut store = VerdictStore::load_dir(dir.path()).unwrap();
        store.record_interaction(iv("qa:Q7", &["BT1-001"], Verdict::Confirmed));
        store.save_dir(dir.path()).unwrap();

        let back = VerdictStore::load_dir(dir.path()).unwrap();
        assert!(back.get_interaction("qa:Q7").is_some(), "the interaction verdict was not written");
    }

    fn iv(id: &str, cards: &[&str], verdict: Verdict) -> InteractionVerdict {
        InteractionVerdict {
            interaction_id: id.to_string(),
            card_ids: cards.iter().map(|c| c.to_string()).collect(),
            source: if id.starts_with("qa:") { "qa" } else { "probe" }.to_string(),
            kind: "positive".to_string(),
            verdict,
            text_sha256: "i-sha".to_string(),
            scenario_path: Some("qa/dcgo-exams/BT7/BT7-056-qa-Q1601.yaml".to_string()),
            reason: None,
            dcgo_build: None,
            job_id: None,
            recorded_at: "2026-10-02T00:00:00Z".to_string(),
            extra: BTreeMap::new(),
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("exam_verdicts_v2_{name}"));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    const BOOK: &str = r#"{"version": 1, "interactions": {
        "qa:Q1601": {"source": "qa", "q_id": "Q1601", "card_ids": ["BT7-056", "EX4-030"],
                     "kind": "positive", "gating": true, "text_sha256": "ruling-sha"},
        "probe:BT7-056#effect#0:scope:neg": {"source": "probe", "card_ids": ["BT7-056"],
                     "clause_id": "BT7-056#effect#0", "family": "scope",
                     "family_version": "families@1", "kind": "negative", "gating": true,
                     "text_sha256": "clause-sha", "why": "text:this digimon"}}}"#;

    #[test]
    fn a_clause_only_store_still_writes_version_1_without_an_interactions_key() {
        let mut s = VerdictStore::default();
        s.record(clause_row("BT7-056#effect#0"));
        let json = s.to_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["version"], 1);
        assert!(value.get("interactions").is_none(), "{json}");
    }

    #[test]
    fn a_store_with_an_interaction_writes_version_2_and_round_trips() {
        let mut s = VerdictStore::default();
        s.record(clause_row("BT7-056#effect#0"));
        s.record_interaction(iv("qa:Q1601", &["BT7-056"], Verdict::Confirmed));
        let json = s.to_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["version"], 2);
        let back = VerdictStore::from_json(&json).unwrap();
        assert_eq!(back.get_interaction("qa:Q1601").unwrap().verdict, Verdict::Confirmed);
        assert_eq!(back.get("BT7-056#effect#0").unwrap().verdict, Verdict::Confirmed);
    }

    #[test]
    fn unknown_fields_on_an_interaction_survive_a_round_trip() {
        // A triage block / citation / produced_by written by a newer tool must
        // not be dropped by an older reader's load-then-save.
        let mut row = iv("qa:Q1601", &["BT7-056"], Verdict::Diverged);
        row.extra.insert("citation".to_string(), serde_json::json!("qa:Q1601"));
        let mut s = VerdictStore::default();
        s.record_interaction(row);
        let back = VerdictStore::from_json(&s.to_json().unwrap()).unwrap();
        assert_eq!(
            back.get_interaction("qa:Q1601").unwrap().extra["citation"],
            serde_json::json!("qa:Q1601")
        );
    }

    #[test]
    fn from_json_refuses_unknown_versions_and_v1_interactions_and_key_mismatches() {
        let err = VerdictStore::from_json(r#"{"version": 3, "clauses": {}}"#).unwrap_err();
        assert!(err.contains("version 3"), "{err}");

        let mut s = VerdictStore::default();
        s.record_interaction(iv("qa:Q1601", &["BT7-056"], Verdict::Confirmed));
        let v2 = s.to_json().unwrap();
        let v1 = v2.replace("\"version\": 2", "\"version\": 1");
        assert!(VerdictStore::from_json(&v1).unwrap_err().contains("version-1"));

        let mismatched =
            v2.replace("\"interaction_id\": \"qa:Q1601\"", "\"interaction_id\": \"qa:Q9\"");
        assert!(VerdictStore::from_json(&mismatched).unwrap_err().contains("qa:Q9"));
    }

    #[test]
    fn save_dir_files_a_shared_ruling_under_every_card_and_load_dir_merges_it_once() {
        let dir = tmp("shared");
        let mut s = VerdictStore::default();
        s.record(clause_row("BT7-056#effect#0"));
        s.record_interaction(iv("qa:Q1601", &["BT7-056", "EX4-030"], Verdict::Confirmed));
        s.save_dir(&dir).unwrap();

        for card in ["BT7-056", "EX4-030"] {
            let text = std::fs::read_to_string(dir.join(format!("{card}.json"))).unwrap();
            assert!(text.contains("qa:Q1601"), "{card} file lacks the shared ruling");
            assert!(text.contains("\"version\": 2"));
        }
        let back = VerdictStore::load_dir(&dir).unwrap();
        assert_eq!(back.interaction_count(), 1, "one interaction, two copies");
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn load_dir_refuses_disagreeing_copies_of_one_interaction() {
        let dir = tmp("disagree");
        let mut s = VerdictStore::default();
        s.record_interaction(iv("qa:Q1601", &["BT7-056", "EX4-030"], Verdict::Confirmed));
        s.save_dir(&dir).unwrap();
        // A partial rewrite: only EX4-030's copy moves to diverged.
        let mut other = VerdictStore::default();
        other.record_interaction(iv("qa:Q1601", &["BT7-056", "EX4-030"], Verdict::Diverged));
        other.save(&dir.join("EX4-030.json")).unwrap();

        let err = VerdictStore::load_dir(&dir).unwrap_err();
        assert!(err.contains("disagreeing copies"), "{err}");
        assert!(err.contains("BT7-056.json") && err.contains("EX4-030.json"), "{err}");
    }

    #[test]
    fn load_dir_refuses_an_interaction_misfiled_under_a_card_it_does_not_count_for() {
        let dir = tmp("misfiled");
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = VerdictStore::default();
        s.record_interaction(iv("qa:Q1601", &["BT7-056"], Verdict::Confirmed));
        std::fs::write(dir.join("ST1-12.json"), s.to_json().unwrap()).unwrap();
        let err = VerdictStore::load_dir(&dir).unwrap_err();
        assert!(err.contains("ST1-12"), "{err}");
    }

    #[test]
    fn the_committed_v1_ledger_still_loads_and_would_be_written_back_as_v1() {
        let root = std::env::var("DIGIMON_REPO_ROOT")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_string());
        let dir = Path::new(&root).join("qa/qa-reports/exam-verdicts");
        let store = VerdictStore::load_dir(&dir).expect("committed ledger loads");
        assert!(!store.is_empty(), "expected committed verdicts under {}", dir.display());
        assert_eq!(store.interaction_count(), 0);
        let value: serde_json::Value = serde_json::from_str(&store.to_json().unwrap()).unwrap();
        assert_eq!(value["version"], 1, "a clause-only ledger must not be upgraded to v2");
    }

    /// The Python writer (`exam_binding.write_interaction_verdict`, the one the
    /// card-loop driver uses) and this one must agree byte for byte: the
    /// fixture was written by Python, is read here, and re-saved identically.
    #[test]
    fn the_python_writers_v2_files_round_trip_byte_identically() {
        let root = std::env::var("DIGIMON_REPO_ROOT")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_string());
        let fixture =
            Path::new(&root).join("code/tests/tools/fixtures/card_loop/interactions/verdicts_v2");
        let store = VerdictStore::load_dir(&fixture).expect("python-written v2 files load");
        assert_eq!(store.interaction_count(), 2);
        let shared = store.get_interaction("qa:Q2002").unwrap();
        assert_eq!(shared.card_ids, vec!["BT7-056".to_string(), "EX4-030".to_string()]);
        assert_eq!(shared.extra["produced_by"], serde_json::json!("att-0001"));

        let out = tmp("python_round_trip");
        store.save_dir(&out).unwrap();
        for card in ["BT7-056", "EX4-030"] {
            let name = format!("{card}.json");
            let ours = std::fs::read_to_string(out.join(&name)).unwrap();
            let theirs = std::fs::read_to_string(fixture.join(&name)).unwrap().replace("\r\n", "\n");
            assert_eq!(ours, theirs, "{name} differs between the Rust and Python writers");
        }
    }

    #[test]
    fn record_interaction_verdict_takes_cards_and_fingerprint_from_the_denominator() {
        let book = InteractionBook::from_json(BOOK, "test").unwrap();
        let mut store = VerdictStore::default();
        record_interaction_verdict(
            &mut store,
            &book,
            "qa:Q1601",
            &[],
            Verdict::Confirmed,
            Some("x.yaml".to_string()),
            None,
            "2026-10-04T00:00:00Z".to_string(),
        )
        .unwrap();
        let row = store.get_interaction("qa:Q1601").unwrap();
        assert_eq!(row.card_ids, vec!["BT7-056".to_string(), "EX4-030".to_string()]);
        assert_eq!(row.text_sha256, "ruling-sha");
        assert_eq!(row.source, "qa");
    }

    #[test]
    fn an_orphan_interaction_is_refused_and_the_store_is_untouched() {
        let book = InteractionBook::from_json(BOOK, "test").unwrap();
        let mut store = VerdictStore::default();
        let err = record_interaction_verdict(
            &mut store,
            &book,
            "qa:Q9999",
            &[],
            Verdict::Confirmed,
            None,
            None,
            "2026-10-04T00:00:00Z".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("qa:Q9999"), "{err}");
        assert_eq!(store.interaction_count(), 0);
    }

    #[test]
    fn a_combo_interaction_needs_no_denominator_entry_but_needs_a_card() {
        let book = InteractionBook::from_json(BOOK, "test").unwrap();
        let mut store = VerdictStore::default();
        assert!(record_interaction_verdict(
            &mut store,
            &book,
            "combo:x",
            &[],
            Verdict::Confirmed,
            None,
            None,
            "t".to_string()
        )
        .is_err());
        record_interaction_verdict(
            &mut store,
            &book,
            "combo:x",
            &["BT7-056".to_string()],
            Verdict::Confirmed,
            None,
            None,
            "t".to_string(),
        )
        .unwrap();
        assert_eq!(store.get_interaction("combo:x").unwrap().source, "combo");
    }

    #[test]
    fn a_missing_denominator_says_it_was_never_generated() {
        let err = InteractionBook::load(&tmp("no-book").join("interaction_denominator.json"))
            .unwrap_err();
        assert!(err.contains("not generated"), "{err}");
    }

    #[test]
    fn the_book_lists_gating_ids_per_card() {
        let book = InteractionBook::from_json(BOOK, "test").unwrap();
        assert_eq!(
            book.gating_ids_for_card("BT7-056"),
            vec![
                "probe:BT7-056#effect#0:scope:neg".to_string(),
                "qa:Q1601".to_string()
            ]
        );
        assert_eq!(book.gating_ids_for_card("EX4-030"), vec!["qa:Q1601".to_string()]);
    }

    #[test]
    fn write_interaction_to_card_files_keeps_each_files_other_verdicts() {
        let dir = tmp("writer");
        let mut s = VerdictStore::default();
        s.record(clause_row("BT7-056#effect#0"));
        s.record(clause_row("ST1-12#effect#0"));
        s.save_dir(&dir).unwrap();
        let st1_before = std::fs::read_to_string(dir.join("ST1-12.json")).unwrap();

        let shared = iv("qa:Q1601", &["BT7-056", "EX4-030"], Verdict::Confirmed);
        let written = write_interaction_to_card_files(&dir, &shared).unwrap();
        assert_eq!(written.len(), 2);
        let back = VerdictStore::load_dir(&dir).unwrap();
        assert_eq!(back.get("BT7-056#effect#0").unwrap().verdict, Verdict::Confirmed);
        assert_eq!(back.interaction_count(), 1);
        assert_eq!(
            std::fs::read_to_string(dir.join("ST1-12.json")).unwrap(),
            st1_before,
            "a card the ruling does not print on is never rewritten"
        );
        let bt7: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("BT7-056.json")).unwrap())
                .unwrap();
        assert_eq!(bt7["version"], 2);
        assert_eq!(bt7["last_updated"], "2026-10-02T00:00:00Z");
    }

    #[test]
    fn interaction_summary_keeps_the_denominator_and_flags_drift() {
        let mut s = VerdictStore::default();
        s.record_interaction(iv("qa:Q1601", &["BT7-056"], Verdict::Confirmed));
        s.record_interaction(iv(
            "probe:BT7-056#effect#0:scope:neg",
            &["BT7-056"],
            Verdict::Diverged,
        ));
        s.set_current_text_sha("qa:Q1601", "edited-answer-sha");
        let ids = vec![
            "qa:Q1601".to_string(),
            "probe:BT7-056#effect#0:scope:neg".to_string(),
            "qa:Q1602".to_string(),
        ];
        let sum = s.interaction_summary(&ids);
        assert_eq!(sum.total, 3);
        assert_eq!(
            (sum.confirmed, sum.diverged, sum.unmeasured, sum.invalidated),
            (0, 1, 2, 1)
        );
    }
}
