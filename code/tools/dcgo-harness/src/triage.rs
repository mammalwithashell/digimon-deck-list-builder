//! Corpus triage: collapse many recordings' divergences into a ranked list of
//! distinct defects, each with a concrete repro.

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::card_data::CardData;

use crate::queue::QueueStatus;

/// One divergence from one recording.
#[derive(Debug, Clone)]
pub struct Finding {
    pub game_id: String,
    pub step: u32,
    pub kind: String,
    pub action_id: u16,
    /// Card occupying the board slot the action referenced, when the failure
    /// is board-addressed. This is what makes two bugs on different cards
    /// distinguishable.
    pub card_at_slot: Option<String>,
    pub recording_path: String,
}

/// What makes two findings "the same bug":
/// (failure kind, action-space range, card at the referenced slot).
///
/// Coarse enough that fifty recordings hitting one card's bug collapse into a
/// single ranked entry; specific enough that a field-effect bug and an attack
/// bug on the same card stay apart.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    pub kind: String,
    pub range: &'static str,
    pub card: String,
}

fn action_range(action_id: u16) -> &'static str {
    match action_id {
        0..=29 => "play_hand",
        30..=59 => "hand_effect_or_selection",
        60 => "hatch",
        61 => "move_from_breeding",
        62 => "pass",
        63..=92 => "dna_digivolve",
        93 => "concede",
        100..=399 => "attack",
        400..=999 => "digivolve",
        1000..=1149 => "field_effect",
        1150..=1194 => "trash_effect",
        2000..=2191 => "source_select",
        _ => "other",
    }
}

pub fn signature_of(kind: &str, action_id: u16, card_at_slot: Option<&str>) -> Signature {
    Signature {
        kind: kind.to_string(),
        range: action_range(action_id),
        card: card_at_slot.unwrap_or("-").to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub signature: Signature,
    pub count: usize,
    pub example_recording: String,
    pub example_step: u32,
}

/// Group findings by signature and rank most-frequent first.
pub fn cluster(findings: &[Finding]) -> Vec<Cluster> {
    let mut by_sig: HashMap<Signature, Cluster> = HashMap::new();
    for f in findings {
        let sig = signature_of(&f.kind, f.action_id, f.card_at_slot.as_deref());
        by_sig
            .entry(sig.clone())
            .and_modify(|c| c.count += 1)
            .or_insert(Cluster {
                signature: sig,
                count: 1,
                example_recording: f.recording_path.clone(),
                example_step: f.step,
            });
    }
    let mut out: Vec<Cluster> = by_sig.into_values().collect();
    // Ties broken by signature so output is stable run to run. `kind` must
    // be part of the chain too: two clusters that land on equal count, card,
    // AND range but differ only in `kind` (the first field of `Signature`)
    // would otherwise fall back to `HashMap::into_values()` order, which is
    // not guaranteed stable across process runs.
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.signature.card.cmp(&b.signature.card))
            .then_with(|| a.signature.range.cmp(b.signature.range))
            .then_with(|| a.signature.kind.cmp(&b.signature.kind))
    });
    out
}

/// Tally of what happened to every file discovered under `--corpus`. This is
/// triage's OWN denominator, and it is deliberately a *different* count from
/// `queue::QueueStatus`: `QueueStatus` counts job files under `--root` (the
/// harness's submit/claim/done/failed queue), which has nothing to do with
/// the recordings replayed under `--corpus`. Before this struct existed,
/// `TriageReport::render` printed `QueueStatus::summary()` under the
/// `corpus:` label and gated the inconclusive verdict on `QueueStatus`'s
/// `completed` field — so a run that replayed 200 recordings and turned up
/// real `IllegalAction` divergences could still print "inconclusive" purely
/// because the *queue* directory had zero completed jobs (e.g. a corpus
/// copied in from elsewhere, or `--root` left at its default). See F1 in
/// the triage review that added this struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct CorpusStats {
    /// `.jsonl` files discovered under `--corpus`.
    pub files_seen: usize,
    /// Discovered but unreadable (permissions, I/O error, etc).
    pub read_failed: usize,
    /// Read successfully but not valid recording JSON.
    pub parse_failed: usize,
    /// Successfully parsed and pushed through `replay_recording`. This is
    /// the true "how many games did triage actually check" denominator —
    /// the inconclusive gate is keyed off this field, not `QueueStatus`.
    pub replayed: usize,
    /// Of `replayed`: reached `game_end` with a matching winner.
    pub passed: usize,
    /// Of `replayed`: halted cleanly before `game_end` without a
    /// disagreement (`ReplayOutcome::PartialPass` — e.g. an unencoded
    /// selection). Not a failure; see that variant's doc comment.
    pub partial: usize,
    /// Of `replayed`: a genuine parity disagreement — ANY `ReplayFail`
    /// variant (`IllegalAction`, `ActorMismatch`, `WinnerMismatch`,
    /// `EngineError`, `OpaqueRevealError`), not just the `IllegalAction`
    /// subset that `cluster()` groups into `Finding`s. Clustering is scoped
    /// to illegal actions on purpose, but every failure still has to land
    /// somewhere countable or a corpus that dies entirely on e.g.
    /// `EngineError` would report zero findings and read as clean.
    pub failed: usize,
}

impl CorpusStats {
    pub fn summary(&self) -> String {
        format!(
            "files_seen={} read_failed={} parse_failed={} replayed={} passed={} partial={} failed={}",
            self.files_seen,
            self.read_failed,
            self.parse_failed,
            self.replayed,
            self.passed,
            self.partial,
            self.failed
        )
    }
}

/// Read, parse, and replay every recording in `paths`, tallying the result
/// into a [`CorpusStats`] and collecting `IllegalAction` failures as
/// clusterable [`Finding`]s.
///
/// G3 (second triage review pass): this loop used to live only inside
/// `main.rs`'s `Triage` handler, exercised solely by a manual corpus run —
/// and that handler is exactly where the earlier Critical review finding
/// (F1: the wrong denominator reaching `TriageReport`) actually lived.
/// Pulling it out into a plain function over `&[PathBuf]` + `&HashMap<...>`
/// makes the `ReplayOutcome -> CorpusStats` mapping unit-testable against a
/// small on-disk fixture corpus instead of only being checkable by eyeballing
/// a real run's stdout. `main.rs` now just calls this and prints the report.
pub fn scan_corpus(
    paths: &[PathBuf],
    card_data: &HashMap<String, CardData>,
) -> (CorpusStats, Vec<Finding>) {
    let mut stats = CorpusStats {
        files_seen: paths.len(),
        ..Default::default()
    };
    let mut findings: Vec<Finding> = Vec::new();

    for path in paths {
        let text = match std::fs::read_to_string(path) {
            // Recordings written before the recorder's UTF8Encoding(false)
            // fix start with a BOM, which serde_json rejects as invalid
            // JSON.
            Ok(t) => t.trim_start_matches('\u{feff}').to_string(),
            Err(e) => {
                eprintln!("warn: skipping {}: read failed: {}", path.display(), e);
                stats.read_failed += 1;
                continue;
            }
        };
        let recording = match dcgo_replay::parse_jsonl(&text) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warn: skipping {}: parse failed: {}", path.display(), e);
                stats.parse_failed += 1;
                continue;
            }
        };
        let outcome = dcgo_replay::replay_recording(
            &recording,
            card_data,
            &dcgo_replay::ReplayConfig::default(),
        );
        stats.replayed += 1;
        match &outcome {
            dcgo_replay::ReplayOutcome::Pass { .. } => stats.passed += 1,
            dcgo_replay::ReplayOutcome::PartialPass { .. } => stats.partial += 1,
            dcgo_replay::ReplayOutcome::Fail(fail) => {
                // Every failure counts toward the denominator (F6); only
                // `IllegalAction` also becomes a clusterable `Finding` —
                // `cluster()`'s signature scheme (action-range + card-at-slot)
                // is specific to board-addressed illegal actions and doesn't
                // apply to actor/winner mismatches or engine errors.
                stats.failed += 1;
                if let dcgo_replay::ReplayFail::IllegalAction(ia) = fail {
                    findings.push(Finding {
                        game_id: recording.start.game_id.clone(),
                        step: ia.step,
                        kind: "illegal_action".to_string(),
                        action_id: ia.action_id,
                        card_at_slot: None,
                        recording_path: path.display().to_string(),
                    });
                }
            }
        }
    }

    (stats, findings)
}

pub struct TriageReport {
    /// The harness's job-queue status (`--root`: pending/claimed/done/
    /// failed job files). Printed for operational visibility but is NOT the
    /// triage denominator — see `corpus_stats`.
    pub status: QueueStatus,
    /// What actually happened to the recordings under `--corpus`. This IS
    /// the triage denominator: the inconclusive gate and the exit-code
    /// verdict are both keyed off this, not `status`.
    pub corpus_stats: CorpusStats,
    pub clusters: Vec<Cluster>,
}

impl TriageReport {
    /// True once at least one recording was actually replayed. A run that
    /// discovered zero `.jsonl` files, or discovered files that all failed
    /// to read/parse, checked nothing — no verdict about divergences can be
    /// drawn from it, regardless of what the job queue says.
    pub fn is_conclusive(&self) -> bool {
        self.corpus_stats.replayed > 0
    }

    /// True when the run was conclusive and found nothing wrong: at least
    /// one recording fully replayed to a matching game-end (`passed > 0`),
    /// zero illegal-action clusters, AND zero replay failures of any other
    /// kind (see `CorpusStats::failed`'s doc comment for why the third
    /// clause matters — clustering alone can't tell you a corpus is clean).
    ///
    /// G1/G2 (second triage review pass): the previous version checked only
    /// `failed == 0 && clusters.is_empty()`, which is `true` for
    /// `CorpusStats::default()` (`replayed == 0`) — it was only correct in
    /// practice because `main.rs` separately ANDed it with
    /// `is_conclusive()`. That left an identical hole open for a corpus
    /// where every recording halts early as `PartialPass`: zero failures,
    /// zero clusters, but ALSO zero full passes — nothing was actually
    /// verified, yet this returned `true`. This is not hypothetical: the
    /// real corpus (see `docs`/the triage report) reports `passed=0
    /// partial=3`. Requiring `passed > 0` closes both holes at once and
    /// makes the method self-sufficient: since `passed <= replayed`, a
    /// `true` result already implies `is_conclusive()`, so a future caller
    /// who calls `is_clean()` alone — without first ANDing it with
    /// `is_conclusive()` the way `main.rs` does — cannot silently
    /// reintroduce this bug. Mirrors `queue::QueueStatus::is_clean()`'s
    /// `completed > 0 && failed == 0` convention.
    pub fn is_clean(&self) -> bool {
        self.corpus_stats.passed > 0 && self.corpus_stats.failed == 0 && self.clusters.is_empty()
    }

    /// Render the report. Both denominators are unconditional and clearly
    /// separated: `corpus:` is what triage itself replayed under
    /// `--corpus`, `queue:` is the harness job-queue status under `--root`.
    /// A batch where 180 of 200 corpus recordings died on deck-import
    /// errors must never read as a pass, and it must never be confused with
    /// an unrelated job queue that happens to say something rosier.
    ///
    /// G1: every branch below also discloses the partial count in the
    /// `VERDICT:` line itself, not just in the `corpus:` summary above it —
    /// a corpus that is mostly `PartialPass` must read as visibly less than
    /// a full pass even to someone who only glances at the one-line
    /// verdict.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("corpus: {}\n", self.corpus_stats.summary()));
        out.push_str(&format!("queue: {}\n", self.status.summary()));

        // Gate on the corpus denominator (what triage itself replayed), NOT
        // the queue's `completed` bucket — see `CorpusStats`'s doc comment.
        if !self.is_conclusive() {
            out.push_str(
                "VERDICT: inconclusive — no recordings were replayed, so nothing was actually checked.\n",
            );
            if !self.clusters.is_empty() {
                // Defensive and currently unreachable via `main.rs` (a
                // `Cluster` can only exist once at least one recording
                // replayed all the way to an `IllegalAction` `Finding`,
                // which implies `replayed > 0`) — but a hand-built
                // `TriageReport` (tests today, any future caller tomorrow)
                // could disagree with itself here. Silently dropping a
                // non-empty `clusters` in the "nothing was checked" branch
                // is exactly the discard shape the original bug had —
                // surface the inconsistency instead of hiding it.
                out.push_str(&format!(
                    "  (inconsistent report: {} cluster(s) present despite corpus_stats.replayed == 0)\n",
                    self.clusters.len()
                ));
            }
            return out;
        }

        let clustered: usize = self.clusters.iter().map(|c| c.count).sum();
        // Clustering only ever groups `IllegalAction` failures (see
        // `cluster()`'s doc comment and F6). Any other `ReplayFail` kind
        // still counts toward `corpus_stats.failed` but never becomes a
        // `Cluster` — surface the gap explicitly so it can't silently
        // disappear from the report.
        let unclustered_failures = self.corpus_stats.failed.saturating_sub(clustered);
        let partial = self.corpus_stats.partial;

        // G1: a corpus where every replayed recording halted early
        // (`PartialPass`) and none produced a genuine failure must NOT read
        // as a clean pass — nothing was actually verified end-to-end, it
        // merely didn't crash or diverge. `is_clean()` requires `passed >
        // 0` for exactly this reason; branch on it explicitly here so the
        // partial-only case gets its own honest verdict instead of falling
        // through to "no divergences" below, which is precisely what used
        // to happen.
        if self.clusters.is_empty() && unclustered_failures == 0 && self.corpus_stats.passed == 0 {
            // Deliberately does NOT contain the substring "no divergences" —
            // that phrase is reserved for the genuinely-clean branch below,
            // and this branch exists specifically so a partial-only corpus
            // can never be mistaken for it.
            out.push_str(&format!(
                "VERDICT: not verified — {} of {} replayed game(s) halted early (partial pass; \
                 e.g. an unencoded selection) and 0 fully reached a matching game-end. Nothing \
                 diverged that we could detect, but \"didn't crash\" is not \"verified clean\" — \
                 nothing was actually checked end to end.\n",
                partial, self.corpus_stats.replayed
            ));
            return out;
        }

        if self.clusters.is_empty() && unclustered_failures == 0 {
            // `is_clean()` is true here: passed > 0, failed == 0, no
            // clusters.
            out.push_str(&format!(
                "VERDICT: no divergences across {} replayed game(s) ({} passed, {} partial).\n",
                self.corpus_stats.replayed, self.corpus_stats.passed, partial
            ));
            return out;
        }

        if self.clusters.is_empty() {
            // Every failure was a non-IllegalAction kind — nothing to
            // cluster, but it is emphatically not a clean run.
            out.push_str(&format!(
                "VERDICT: {} failed replay(s), none clustered as illegal-action divergences \
                 (see corpus: failed= above; {} partial) — re-run with `dcgo-replay --input <file> \
                 --verbose` to inspect a specific recording.\n",
                unclustered_failures, partial
            ));
            return out;
        }

        out.push_str(&format!(
            "VERDICT: {} distinct divergence(s) across {} replayed game(s) ({} partial)",
            self.clusters.len(),
            self.corpus_stats.replayed,
            partial
        ));
        if unclustered_failures > 0 {
            out.push_str(&format!(
                " (+{} failed replay(s) of other kinds, not clustered)",
                unclustered_failures
            ));
        }
        out.push_str(".\n\n");
        for (i, c) in self.clusters.iter().enumerate() {
            out.push_str(&format!(
                "{}. [{}x] {} in {} on {}\n   repro: dcgo-replay --input {} --cards-json data/cards.json --verbose   (step {})\n",
                i + 1,
                c.count,
                c.signature.kind,
                c.signature.range,
                c.signature.card,
                c.example_recording,
                c.example_step
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(game: &str, kind: &str, action_id: u16, card: Option<&str>) -> Finding {
        Finding {
            game_id: game.to_string(),
            step: 9,
            kind: kind.to_string(),
            action_id,
            card_at_slot: card.map(|c| c.to_string()),
            recording_path: format!("recordings/{}.jsonl", game),
        }
    }

    #[test]
    fn one_bug_across_many_games_collapses_to_one_cluster() {
        let findings: Vec<Finding> = (0..50)
            .map(|i| finding(&format!("g{}", i), "illegal_action", 1040, Some("EX10-010")))
            .collect();
        let clusters = cluster(&findings);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 50);
    }

    #[test]
    fn two_different_bugs_on_one_card_stay_apart() {
        // Same card, different action ranges — genuinely different defects.
        let findings = vec![
            finding("g1", "illegal_action", 1040, Some("EX10-010")),
            finding("g2", "illegal_action", 114, Some("EX10-010")),
        ];
        let clusters = cluster(&findings);
        assert_eq!(clusters.len(), 2, "field-effect and attack bugs are not one bug");
    }

    #[test]
    fn clusters_are_ranked_most_frequent_first() {
        let mut findings = vec![finding("g1", "illegal_action", 114, Some("A"))];
        for i in 0..5 {
            findings.push(finding(&format!("h{}", i), "illegal_action", 1040, Some("B")));
        }
        let clusters = cluster(&findings);
        assert_eq!(clusters[0].count, 5);
        assert_eq!(clusters[1].count, 1);
    }

    #[test]
    fn each_cluster_names_a_concrete_recording_to_reproduce_from() {
        let findings = vec![finding("g7", "illegal_action", 1040, Some("EX10-010"))];
        let clusters = cluster(&findings);
        assert_eq!(clusters[0].example_recording, "recordings/g7.jsonl");
        assert_eq!(clusters[0].example_step, 9);
    }

    #[test]
    fn kind_breaks_ties_when_count_card_and_range_are_equal() {
        // F5a: clusters with equal count, card, and range must still sort
        // deterministically by `kind` instead of falling back to
        // `HashMap::into_values()` order, which varies per process.
        //
        // Minor (second review pass): the original version of this test
        // used only 2 kinds, so removing the tie-break entirely still had a
        // 1-in-2 chance of passing by luck (`HashMap`'s order over 2
        // buckets can coincidentally match the sorted order). Widened to 5
        // distinct kinds sharing count/card/range — a regression now has
        // only a 1-in-120 (1/5!) chance of slipping through undetected.
        let findings = vec![
            finding("g1", "winner_mismatch", 1040, Some("EX10-010")),
            finding("g2", "illegal_action", 1040, Some("EX10-010")),
            finding("g3", "opaque_reveal_error", 1040, Some("EX10-010")),
            finding("g4", "engine_error", 1040, Some("EX10-010")),
            finding("g5", "actor_mismatch", 1040, Some("EX10-010")),
        ];
        let clusters = cluster(&findings);
        assert_eq!(clusters.len(), 5);
        let observed: Vec<&str> = clusters.iter().map(|c| c.signature.kind.as_str()).collect();
        assert_eq!(
            observed,
            vec![
                "actor_mismatch",
                "engine_error",
                "illegal_action",
                "opaque_reveal_error",
                "winner_mismatch",
            ],
            "kind must be the deterministic final tie-break, sorted ascending"
        );
    }

    /// F4 rewrite: the original test only asserted the ABSENCE of a string
    /// no branch of `render` ever emits ("no divergences found" — the real
    /// text is "no divergences"), so deleting the inconclusive guard
    /// entirely left it green. This version asserts POSITIVELY that the
    /// inconclusive text appears, and was confirmed to fail (by temporarily
    /// deleting the `!self.is_conclusive()` guard in `render`) before being
    /// restored.
    ///
    /// It also doubles as the F1 regression guard: `status.completed=200`
    /// (a queue that looks fully done) must NOT be enough to read as
    /// conclusive — only `corpus_stats.replayed` gates the verdict, because
    /// the queue (`--root`) and the corpus (`--corpus`) are different
    /// things.
    #[test]
    fn report_reads_inconclusive_when_corpus_replayed_nothing() {
        let report = TriageReport {
            status: crate::queue::QueueStatus {
                pending: 0,
                claimed: 0,
                completed: 200,
                partial: 0,
                failed: 0,
            },
            corpus_stats: CorpusStats::default(),
            clusters: Vec::new(),
        };
        assert!(!report.is_conclusive());
        let text = report.render();
        assert!(
            text.to_lowercase().contains("inconclusive"),
            "zero replayed recordings must read as inconclusive even though the queue says completed=200: {}",
            text
        );
        assert!(
            !text.to_lowercase().contains("no divergences"),
            "an inconclusive run must never also read as a clean pass: {}",
            text
        );
    }

    #[test]
    fn conclusive_verdict_with_findings_is_not_clean() {
        let findings = vec![finding("g1", "illegal_action", 1040, Some("EX10-010"))];
        let report = TriageReport {
            status: crate::queue::QueueStatus::default(),
            corpus_stats: CorpusStats {
                files_seen: 1,
                replayed: 1,
                failed: 1,
                ..Default::default()
            },
            clusters: cluster(&findings),
        };
        assert!(report.is_conclusive());
        assert!(!report.is_clean());
        let text = report.render();
        assert!(text.contains("distinct divergence"), "{}", text);
        assert!(!text.to_lowercase().contains("inconclusive"), "{}", text);
    }

    #[test]
    fn conclusive_verdict_with_no_failures_is_clean() {
        let report = TriageReport {
            status: crate::queue::QueueStatus::default(),
            corpus_stats: CorpusStats {
                files_seen: 3,
                replayed: 3,
                passed: 3,
                ..Default::default()
            },
            clusters: Vec::new(),
        };
        assert!(report.is_conclusive());
        assert!(report.is_clean());
        let text = report.render();
        assert!(text.contains("no divergences"), "{}", text);
    }

    #[test]
    fn unclustered_failures_prevent_a_false_clean_verdict() {
        // F6: EngineError/ActorMismatch/WinnerMismatch/OpaqueRevealError
        // never become `Finding`s, so `clusters` stays empty even when
        // every single replay failed. The verdict must still read as
        // non-clean and must not fall through to "no divergences".
        let report = TriageReport {
            status: crate::queue::QueueStatus::default(),
            corpus_stats: CorpusStats {
                files_seen: 5,
                replayed: 5,
                failed: 5,
                ..Default::default()
            },
            clusters: Vec::new(),
        };
        assert!(report.is_conclusive());
        assert!(!report.is_clean());
        let text = report.render();
        assert!(
            !text.to_lowercase().contains("no divergences"),
            "5 unclustered failures must not read as clean: {}",
            text
        );
        assert!(text.contains("failed=5"), "denominator must appear: {}", text);
    }

    // --- G1/G2 regression coverage (second triage review pass) ---

    /// G2: `is_clean()` must be self-sufficient. A caller who calls it
    /// alone — without first ANDing it with `is_conclusive()`, the way
    /// `main.rs` does — must not be told an empty/inconclusive corpus is
    /// clean. This is the direct pin for the doc-vs-implementation
    /// contradiction: the old body (`failed == 0 && clusters.is_empty()`)
    /// returned `true` for `CorpusStats::default()`.
    #[test]
    fn is_clean_is_false_for_a_default_corpus_stats_on_its_own() {
        let report = TriageReport {
            status: crate::queue::QueueStatus::default(),
            corpus_stats: CorpusStats::default(),
            clusters: Vec::new(),
        };
        assert!(
            !report.is_clean(),
            "is_clean() must be false for an empty/inconclusive corpus even without checking is_conclusive() first"
        );
    }

    /// G1: a corpus where every replayed recording halted early
    /// (`PartialPass`) and nothing failed must NOT read as a clean pass —
    /// this is the live bug, not a hypothetical one: the real corpus run
    /// reports `passed=0 partial=3`. Also pins that the `VERDICT:` line
    /// itself (not just the `corpus:` summary line above it) discloses the
    /// partial count.
    #[test]
    fn partial_only_corpus_is_not_clean_and_discloses_partial_in_the_verdict_line() {
        let report = TriageReport {
            status: crate::queue::QueueStatus::default(),
            corpus_stats: CorpusStats {
                files_seen: 3,
                replayed: 3,
                partial: 3,
                ..Default::default()
            },
            clusters: Vec::new(),
        };
        assert!(report.is_conclusive(), "3 recordings were replayed, however inconclusively");
        assert!(
            !report.is_clean(),
            "zero full passes among all-partial recordings must not be a clean verdict"
        );
        let text = report.render();
        assert!(
            !text.to_lowercase().contains("no divergences"),
            "an all-partial, zero-pass corpus must not read as a clean pass: {}",
            text
        );
        let verdict_line = text
            .lines()
            .find(|l| l.starts_with("VERDICT"))
            .expect("render() must always emit a VERDICT line");
        assert!(
            verdict_line.to_lowercase().contains("partial"),
            "the VERDICT line itself must disclose the partial count, not just the corpus: summary above it: {}",
            verdict_line
        );
        // Exit-code contract: is_conclusive() && is_clean() must be false,
        // which is what main.rs's Triage handler gates the exit code on.
        assert!(!(report.is_conclusive() && report.is_clean()));
    }

    /// Minor cleanup (second review pass): `render()`'s inconclusive branch
    /// used to return early and silently drop any non-empty `clusters` —
    /// the same discard shape as the original wrong-denominator bug, just
    /// currently unreachable via `main.rs`. A hand-built, internally
    /// inconsistent `TriageReport` (replayed == 0 but clusters non-empty)
    /// must surface that inconsistency instead of hiding it.
    #[test]
    fn inconclusive_report_still_surfaces_a_nonempty_clusters_inconsistency() {
        let findings = vec![finding("g1", "illegal_action", 1040, Some("EX10-010"))];
        let report = TriageReport {
            status: crate::queue::QueueStatus::default(),
            corpus_stats: CorpusStats::default(), // replayed == 0
            clusters: cluster(&findings),          // but a cluster exists anyway
        };
        assert!(!report.is_conclusive());
        let text = report.render();
        assert!(text.to_lowercase().contains("inconclusive"), "{}", text);
        assert!(
            text.contains("1 cluster"),
            "a non-empty clusters vec on an inconclusive report must be surfaced, not silently dropped: {}",
            text
        );
    }

    // --- G3: `scan_corpus` (extracted from main.rs) over a fixture corpus ---

    /// Minimal card pool matching `dcgo-replay`'s own
    /// `opaque_recording_without_reveals_surfaces_opaque_error` fixture —
    /// just enough for `ReplaySession::from_dcgo` to run far enough to
    /// surface a real `ReplayFail` rather than an unrelated card-lookup
    /// error.
    fn scan_corpus_micro_card_db() -> HashMap<String, CardData> {
        let json = r#"{
            "BT1-010": {
                "card_id": "BT1-010", "card_name_eng": "Agumon",
                "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
                "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    /// A process-unique temp directory under `std::env::temp_dir()` — no
    /// `tempfile` dependency, mirrors `queue::tests::temp_root`'s pattern
    /// (monotonic counter + wall-clock nanos so parallel `cargo test`
    /// threads never collide).
    fn scan_corpus_temp_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::SystemTime;
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("dcgo_harness_triage_test_{tag}_{nanos}_{n}"));
        std::fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    /// G3: exercises `scan_corpus` directly against a small on-disk fixture
    /// corpus covering all three outcomes the earlier Critical finding (F1,
    /// wrong denominator) lived among: a file that fails to *read*, a file
    /// that fails to *parse*, and a file that DOES make it all the way
    /// through `replay_recording` (proving the read -> parse -> replay ->
    /// tally path is actually exercised, not just eyeballed from a manual
    /// run).
    #[test]
    fn scan_corpus_counts_read_parse_and_replay_outcomes_over_a_fixture_corpus() {
        let dir = scan_corpus_temp_dir("mixed");

        // A path that LOOKS like a recording but is actually a directory —
        // std::fs::read_to_string on it fails, exercising the read-failed
        // path. Mirrors queue.rs's directory-shaped-like-a-file trick.
        let unreadable = dir.join("unreadable.jsonl");
        std::fs::create_dir_all(&unreadable).expect("seed unreadable path");

        // Readable but not valid recording JSON.
        let unparseable = dir.join("garbage.jsonl");
        std::fs::write(&unparseable, "not json at all\n").expect("seed garbage file");

        // A real, minimal recording that makes it all the way into
        // `replay_recording`: an opaque-PvP header with no reveal rows,
        // which fails *construction* with `OpaqueRevealError` — same
        // fixture shape as `dcgo-replay`'s
        // `opaque_recording_without_reveals_surfaces_opaque_error` test.
        // It's a `Fail`, not a `Pass`, but that's enough: it proves the
        // file was read, parsed, and pushed through `replay_recording`,
        // landing in `stats.replayed` / `stats.failed` — the exact tally
        // the wrong-denominator bug lived in.
        let replayable = dir.join("opaque.jsonl");
        std::fs::write(
            &replayable,
            "{\"v\":1,\"type\":\"game_start\",\"game_id\":\"x\",\"timestamp\":\"t\",\
             \"my_player_id\":0,\"is_ai\":false,\"my_deck_post_shuffle\":[\"BT1-010\"],\
             \"opp_deck_post_shuffle\":null}\n\
             {\"type\":\"game_end\",\"winner\":0,\"reason\":\"win\",\"total_steps\":0}\n",
        )
        .expect("seed replayable recording");

        let card_data = scan_corpus_micro_card_db();
        let paths = vec![unreadable, unparseable, replayable];
        let (stats, findings) = scan_corpus(&paths, &card_data);

        assert_eq!(stats.files_seen, 3);
        assert_eq!(stats.read_failed, 1, "the directory path must count as a read failure: {:?}", stats);
        assert_eq!(stats.parse_failed, 1, "the garbage file must count as a parse failure: {:?}", stats);
        assert_eq!(stats.replayed, 1, "the opaque recording must be pushed through replay_recording: {:?}", stats);
        assert_eq!(stats.failed, 1, "OpaqueRevealError is a genuine Fail outcome: {:?}", stats);
        assert_eq!(stats.passed, 0);
        assert_eq!(stats.partial, 0);
        assert!(
            findings.is_empty(),
            "OpaqueRevealError is not an IllegalAction and must not become a Finding: {:?}",
            findings
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
