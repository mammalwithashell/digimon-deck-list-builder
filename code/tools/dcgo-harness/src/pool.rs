//! Deck-pool sampling. The CLI owns deck *selection* (which matchups the
//! corpus exercises); DCGO owns deck *encoding*.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::job::{JobDecks, JobLimits, JobSpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PooledDeck {
    pub name: String,
    /// Main deck, 50 card IDs.
    pub cards: Vec<String>,
    /// Digitama deck, up to 5 card IDs.
    #[serde(default)]
    pub eggs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckPool {
    pub decks: Vec<PooledDeck>,
}

pub fn load_pool(path: &Path) -> Result<DeckPool, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("reading deck pool {}: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing deck pool: {}", e))
}

/// Build `count` jobs by walking deck pairs deterministically.
///
/// Determinism matters twice over: the per-job `seed` makes the *game*
/// reproducible, and a deterministic pairing walk makes the *batch*
/// reproducible, so "rerun batch 42" produces the same matchups.
///
/// Pairing walks p0 through the pool in order and offsets p1 by a stride that
/// is coprime-ish with the pool size, which spreads matchups without ever
/// pairing a deck against itself.
pub fn build_jobs(
    pool: &DeckPool,
    count: u32,
    base_seed: u64,
    limits: &JobLimits,
) -> Result<Vec<JobSpec>, String> {
    let n = pool.decks.len();
    if n < 2 {
        return Err(format!(
            "deck pool needs at least 2 decks to form a matchup, found {}",
            n
        ));
    }

    let mut jobs = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let a = i % n;
        // Offset by at least 1 so p0 != p1, and vary it so matchups spread.
        let b = (a + 1 + (i / n) % (n - 1)) % n;
        let p0 = &pool.decks[a];
        let p1 = &pool.decks[b];

        // Eggs ride the same list; DCGO's card-kind routing separates them.
        let mut p0_cards = p0.cards.clone();
        p0_cards.extend(p0.eggs.iter().cloned());
        let mut p1_cards = p1.cards.clone();
        p1_cards.extend(p1.eggs.iter().cloned());

        jobs.push(JobSpec {
            job_id: format!("vol-{:05}", i),
            policy: "ai".to_string(),
            decks: JobDecks {
                p0: p0_cards,
                p1: p1_cards,
            },
            // Alternate the opening seat so first-player advantage does not
            // bias which lines the corpus covers.
            first_player: (i % 2) as u8,
            seed: base_seed.wrapping_add(i as u64),
            limits: limits.clone(),
        });
    }
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobLimits;

    fn pool() -> DeckPool {
        DeckPool {
            decks: vec![
                PooledDeck { name: "a".into(), cards: vec!["EX12-035".into()], eggs: vec!["EX12-001".into()] },
                PooledDeck { name: "b".into(), cards: vec!["BT16-082".into()], eggs: vec!["BT14-001".into()] },
                PooledDeck { name: "c".into(), cards: vec!["BT17-102".into()], eggs: vec!["BT14-001".into()] },
            ],
        }
    }

    #[test]
    fn build_jobs_emits_requested_count_with_unique_ids() {
        let jobs = build_jobs(&pool(), 5, 100, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect("build");
        assert_eq!(jobs.len(), 5);
        let mut ids: Vec<&str> = jobs.iter().map(|j| j.job_id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5, "job ids must be unique");
    }

    #[test]
    fn build_jobs_is_deterministic_for_a_given_base_seed() {
        let limits = JobLimits { max_turns: 40, timeout_seconds: 180 };
        let a = build_jobs(&pool(), 6, 42, &limits).expect("build");
        let b = build_jobs(&pool(), 6, 42, &limits).expect("build");
        let seeds_a: Vec<u64> = a.iter().map(|j| j.seed).collect();
        let seeds_b: Vec<u64> = b.iter().map(|j| j.seed).collect();
        assert_eq!(seeds_a, seeds_b);
        assert_eq!(a[3].decks.p0, b[3].decks.p0);
    }

    #[test]
    fn build_jobs_never_mirrors_a_deck_against_itself() {
        let jobs = build_jobs(&pool(), 12, 1, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect("build");
        for j in &jobs {
            assert_ne!(j.decks.p0, j.decks.p1, "mirror matches waste corpus slots");
        }
    }

    #[test]
    fn build_jobs_alternates_first_player() {
        let jobs = build_jobs(&pool(), 4, 1, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect("build");
        let firsts: Vec<u8> = jobs.iter().map(|j| j.first_player).collect();
        assert!(firsts.contains(&0) && firsts.contains(&1), "both seats must go first");
    }

    #[test]
    fn build_jobs_rejects_a_pool_too_small_to_pair() {
        let small = DeckPool { decks: vec![PooledDeck { name: "solo".into(), cards: vec![], eggs: vec![] }] };
        let err = build_jobs(&small, 1, 1, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect_err("one deck cannot form a pair");
        assert!(err.contains("at least 2"), "error should say what is wrong: {}", err);
    }
}
