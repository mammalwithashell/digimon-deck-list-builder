//! Corpus triage: collapse many recordings' divergences into a ranked list of
//! distinct defects, each with a concrete repro.

use std::collections::HashMap;

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

    /// True when the run was conclusive and found nothing wrong: zero
    /// illegal-action clusters AND zero replay failures of any other kind
    /// (see `CorpusStats::failed`'s doc comment for why the second half
    /// matters — clustering alone can't tell you a corpus is clean).
    pub fn is_clean(&self) -> bool {
        self.corpus_stats.failed == 0 && self.clusters.is_empty()
    }

    /// Render the report. Both denominators are unconditional and clearly
    /// separated: `corpus:` is what triage itself replayed under
    /// `--corpus`, `queue:` is the harness job-queue status under `--root`.
    /// A batch where 180 of 200 corpus recordings died on deck-import
    /// errors must never read as a pass, and it must never be confused with
    /// an unrelated job queue that happens to say something rosier.
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
            return out;
        }

        let clustered: usize = self.clusters.iter().map(|c| c.count).sum();
        // Clustering only ever groups `IllegalAction` failures (see
        // `cluster()`'s doc comment and F6). Any other `ReplayFail` kind
        // still counts toward `corpus_stats.failed` but never becomes a
        // `Cluster` — surface the gap explicitly so it can't silently
        // disappear from the report.
        let unclustered_failures = self.corpus_stats.failed.saturating_sub(clustered);

        if self.clusters.is_empty() && unclustered_failures == 0 {
            out.push_str(&format!(
                "VERDICT: no divergences across {} replayed game(s).\n",
                self.corpus_stats.replayed
            ));
            return out;
        }

        if self.clusters.is_empty() {
            // Every failure was a non-IllegalAction kind — nothing to
            // cluster, but it is emphatically not a clean run.
            out.push_str(&format!(
                "VERDICT: {} failed replay(s), none clustered as illegal-action divergences \
                 (see corpus: failed= above) — re-run with `dcgo-replay --input <file> --verbose` \
                 to inspect a specific recording.\n",
                unclustered_failures
            ));
            return out;
        }

        out.push_str(&format!(
            "VERDICT: {} distinct divergence(s) across {} replayed game(s)",
            self.clusters.len(),
            self.corpus_stats.replayed
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
        // F5a: two clusters with equal count, card, and range must still
        // sort deterministically by `kind` instead of falling back to
        // `HashMap::into_values()` order, which varies per process.
        let findings = vec![
            finding("g1", "winner_mismatch", 1040, Some("EX10-010")),
            finding("g2", "illegal_action", 1040, Some("EX10-010")),
        ];
        let clusters = cluster(&findings);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].signature.kind, "illegal_action", "'illegal_action' < 'winner_mismatch' lexicographically");
        assert_eq!(clusters[1].signature.kind, "winner_mismatch");
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
}
