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
    // Ties broken by signature so output is stable run to run.
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.signature.card.cmp(&b.signature.card))
            .then_with(|| a.signature.range.cmp(b.signature.range))
    });
    out
}

pub struct TriageReport {
    pub status: QueueStatus,
    pub clusters: Vec<Cluster>,
}

impl TriageReport {
    /// Render the report. The denominator line is unconditional: a batch where
    /// 180 of 200 jobs died on deck-import errors must never read as a pass.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("corpus: {}\n", self.status.summary()));

        if self.status.completed == 0 {
            out.push_str(
                "VERDICT: inconclusive — no games completed, so nothing was actually checked.\n",
            );
            return out;
        }

        if self.clusters.is_empty() {
            out.push_str(&format!(
                "VERDICT: no divergences across {} completed game(s).\n",
                self.status.completed
            ));
            return out;
        }

        out.push_str(&format!(
            "VERDICT: {} distinct divergence(s) across {} completed game(s).\n\n",
            self.clusters.len(),
            self.status.completed
        ));
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
    fn report_refuses_a_clean_verdict_without_completed_games() {
        let report = TriageReport {
            status: crate::queue::QueueStatus { pending: 0, claimed: 0, completed: 0, partial: 0, failed: 200 },
            clusters: Vec::new(),
        };
        let text = report.render();
        assert!(text.contains("failed=200"), "denominator must appear: {}", text);
        assert!(
            !text.to_lowercase().contains("no divergences found"),
            "a batch with zero completed games must not read as a pass: {}",
            text
        );
    }
}
