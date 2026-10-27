//! Gumbel root selection (Danihelka, Guez, Schrittwieser & Silver, "Policy
//! improvement by planning with Gumbel", ICLR 2022) for the search-budget
//! experiments.
//!
//! The root replaces PUCT + Dirichlet noise with:
//!
//! 1. one Gumbel sample `g(a)` per legal root action;
//! 2. the `m` candidates with the highest `g(a) + logit(a)` (Gumbel-top-k,
//!    i.e. sampling without replacement from the prior);
//! 3. **sequential halving** over the simulation budget: every surviving
//!    candidate is visited the same number of times per phase, then the
//!    worse half by `g(a) + logit(a) + σ(q̂(a))` is dropped, until one
//!    candidate remains and absorbs the rest of the budget;
//! 4. the action to play is the survivor (`argmax g + logit + σ(q̂)`), and the
//!    policy target is the **completed-Q improved policy**
//!    `softmax(logit(a) + σ(completed_q(a)))` over every legal action, where
//!    unvisited actions take the mixed value estimate instead of `q̂`.
//!
//! Only the root changes; selection below the root stays PUCT, so this is the
//! "Gumbel root, PUCT interior" variant rather than full Gumbel MuZero. The
//! point is to make a small budget (tens of simulations) yield a usable policy
//! improvement, which plain visit counts do not.
//!
//! Everything here is pure arithmetic over per-edge `(visits, mean Q)` stats
//! in the root mover's perspective, so it is unit-tested without a `Game`.

use std::collections::VecDeque;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// `n` standard Gumbel(0, 1) samples from a seeded stream.
pub(super) fn gumbel_noise(seed: u64, n: usize) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n)
        .map(|_| {
            // u ∈ (0, 1): both bounds excluded so the double log is finite.
            let u: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
            (-(-u.ln()).ln()) as f32
        })
        .collect()
}

/// The paper's monotone transform of a (completed) Q value:
/// `σ(q) = (c_visit + max_visits) · c_scale · q`.
pub(super) fn sigma(q: f32, max_visits: u32, c_visit: f32, c_scale: f32) -> f32 {
    (c_visit + max_visits as f32) * c_scale * q
}

/// Root-level sequential-halving scheduler plus the completed-Q policy
/// improvement. Indices are root EDGE indices (mask order).
pub(super) struct GumbelRoot {
    gumbel: Vec<f32>,
    /// `ln prior(a)` — the un-noised network prior over legal actions.
    logits: Vec<f32>,
    /// Surviving candidate edges, ascending index order.
    remaining: Vec<usize>,
    /// Edge indices scheduled for the current phase.
    queue: VecDeque<usize>,
    /// Simulations not yet handed out.
    budget_left: u32,
    phases_left: u32,
    phase_started: bool,
    c_visit: f32,
    c_scale: f32,
    /// Network value of the root for the root mover (mixed-value anchor).
    root_value: f32,
}

impl GumbelRoot {
    /// `priors` are the root edges' network priors (renormalized over the
    /// legal actions); `simulations` is the whole root budget.
    pub(super) fn new(
        priors: &[f32],
        root_value: f32,
        simulations: u32,
        max_candidates: u32,
        c_visit: f32,
        c_scale: f32,
        noise_seed: u64,
    ) -> Self {
        let n = priors.len();
        assert!(n > 0, "Gumbel root needs at least one legal action");
        let gumbel = gumbel_noise(noise_seed, n);
        let logits: Vec<f32> = priors.iter().map(|p| p.max(1e-9).ln()).collect();
        let m = (max_candidates.max(1) as usize).min(n);
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            let sa = gumbel[a] + logits[a];
            let sb = gumbel[b] + logits[b];
            sb.partial_cmp(&sa)
                .expect("finite Gumbel scores")
                .then(a.cmp(&b))
        });
        let mut remaining = order[..m].to_vec();
        remaining.sort_unstable();
        let phases = if m <= 1 {
            1
        } else {
            (m as f64).log2().ceil() as u32
        };
        Self {
            gumbel,
            logits,
            remaining,
            queue: VecDeque::new(),
            budget_left: simulations,
            phases_left: phases.max(1),
            phase_started: false,
            c_visit,
            c_scale,
            root_value,
        }
    }

    /// Surviving candidates (ascending edge index).
    pub(super) fn remaining(&self) -> &[usize] {
        &self.remaining
    }

    fn score(&self, i: usize, stats: &[(u32, f32)], max_visits: u32) -> f32 {
        self.gumbel[i] + self.logits[i] + sigma(stats[i].1, max_visits, self.c_visit, self.c_scale)
    }

    fn max_visits(stats: &[(u32, f32)]) -> u32 {
        stats.iter().map(|s| s.0).max().unwrap_or(0)
    }

    /// Hand out the next root edge to simulate, or `None` once the budget is
    /// spent. `stats` are the CURRENT per-edge `(visits, mean Q)` — the
    /// caller must have backed up the previous simulation first, because a
    /// phase boundary halves the candidates by their Q estimates.
    pub(super) fn next_edge(&mut self, stats: &[(u32, f32)]) -> Option<usize> {
        if self.budget_left == 0 {
            return None;
        }
        if self.queue.is_empty() {
            self.refill(stats);
        }
        self.budget_left -= 1;
        self.queue.pop_front()
    }

    fn refill(&mut self, stats: &[(u32, f32)]) {
        if self.phase_started && self.remaining.len() > 1 {
            // Phase boundary: keep the better half (ties → lower index).
            let max_visits = Self::max_visits(stats);
            let keep = self.remaining.len().div_ceil(2);
            let mut ranked = self.remaining.clone();
            ranked.sort_by(|&a, &b| {
                self.score(b, stats, max_visits)
                    .partial_cmp(&self.score(a, stats, max_visits))
                    .expect("finite halving scores")
                    .then(a.cmp(&b))
            });
            ranked.truncate(keep);
            ranked.sort_unstable();
            self.remaining = ranked;
            self.phases_left = self.phases_left.saturating_sub(1).max(1);
        }
        self.phase_started = true;
        let per_candidate = if self.remaining.len() == 1 {
            self.budget_left
        } else {
            (self.budget_left / (self.phases_left * self.remaining.len() as u32)).max(1)
        };
        let mut scheduled = 0u32;
        'outer: for &edge in &self.remaining {
            for _ in 0..per_candidate {
                if scheduled == self.budget_left {
                    break 'outer;
                }
                self.queue.push_back(edge);
                scheduled += 1;
            }
        }
    }

    /// The action to play: the best surviving candidate by
    /// `g + logit + σ(q̂)`.
    pub(super) fn chosen(&self, stats: &[(u32, f32)]) -> usize {
        let max_visits = Self::max_visits(stats);
        let mut best = self.remaining[0];
        let mut best_score = f32::NEG_INFINITY;
        for &i in &self.remaining {
            let s = self.score(i, stats, max_visits);
            if s > best_score {
                best_score = s;
                best = i;
            }
        }
        best
    }

    /// Completed-Q improved policy over EVERY root edge (paper §4):
    /// `softmax(logit(a) + σ(completed_q(a)))`, with unvisited actions taking
    /// the mixed value estimate
    /// `v_mix = (v_root + N · Σ_visited π(a) q̂(a) / Σ_visited π(a)) / (1 + N)`.
    pub(super) fn improved_policy(&self, stats: &[(u32, f32)]) -> Vec<f32> {
        let n = stats.len();
        let total_visits: u32 = stats.iter().map(|s| s.0).sum();
        let max_visits = Self::max_visits(stats);
        let (mut pi_q, mut pi_mass) = (0.0f64, 0.0f64);
        for (i, &(visits, q)) in stats.iter().enumerate() {
            if visits > 0 {
                let prior = (self.logits[i] as f64).exp();
                pi_q += prior * q as f64;
                pi_mass += prior;
            }
        }
        let v_mix = if total_visits == 0 || pi_mass <= 0.0 {
            self.root_value
        } else {
            ((self.root_value as f64 + total_visits as f64 * (pi_q / pi_mass))
                / (1.0 + total_visits as f64)) as f32
        };
        let scores: Vec<f32> = (0..n)
            .map(|i| {
                let completed = if stats[i].0 > 0 { stats[i].1 } else { v_mix };
                self.logits[i] + sigma(completed, max_visits, self.c_visit, self.c_scale)
            })
            .collect();
        let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f64> = scores
            .iter()
            .map(|&s| ((s - max_score) as f64).exp())
            .collect();
        let total: f64 = exps.iter().sum();
        exps.iter().map(|&e| (e / total) as f32).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gumbel_noise_is_seeded_and_finite() {
        let a = gumbel_noise(7, 16);
        let b = gumbel_noise(7, 16);
        let c = gumbel_noise(8, 16);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.iter().all(|g| g.is_finite()));
    }

    /// Sequential halving over 8 candidates and 32 simulations: three
    /// phases, every candidate visited equally within a phase, exactly the
    /// budget spent, one survivor at the end.
    #[test]
    fn sequential_halving_spends_the_budget_and_leaves_one_survivor() {
        let priors = vec![0.125f32; 8];
        let mut root = GumbelRoot::new(&priors, 0.0, 32, 8, 50.0, 1.0, 1);
        // Edge 3 is secretly the best: its Q is 0.9, everything else 0.
        let mut stats: Vec<(u32, f32)> = vec![(0, 0.0); 8];
        let mut handed = 0;
        while let Some(edge) = root.next_edge(&stats) {
            handed += 1;
            let q = if edge == 3 { 0.9 } else { 0.0 };
            let (v, mean) = stats[edge];
            stats[edge] = (v + 1, (mean * v as f32 + q) / (v + 1) as f32);
        }
        assert_eq!(handed, 32, "the whole budget is handed out");
        // Three phases: 8 → 4 → 2 candidates; the last halving IS the final
        // argmax, so at most two candidates remain and the best one is chosen.
        assert!(root.remaining().len() <= 2 && root.remaining().contains(&3),
            "the clearly best candidate survives every halving: {:?}", root.remaining());
        assert_eq!(root.chosen(&stats), 3);
        // Phase one visited every candidate at least once.
        assert!(stats.iter().all(|s| s.0 >= 1));
        // The survivor absorbed the leftover budget.
        assert!(stats[3].0 > stats.iter().filter(|s| s.0 > 0).map(|s| s.0).min().unwrap());
    }

    #[test]
    fn candidate_cap_limits_the_initial_set_and_single_candidate_takes_everything() {
        let priors = vec![0.25f32; 4];
        let root = GumbelRoot::new(&priors, 0.0, 10, 2, 50.0, 1.0, 3);
        assert_eq!(root.remaining().len(), 2);

        let mut single = GumbelRoot::new(&priors, 0.0, 5, 1, 50.0, 1.0, 3);
        let stats = vec![(0u32, 0.0f32); 4];
        let only = single.remaining()[0];
        for _ in 0..5 {
            assert_eq!(single.next_edge(&stats), Some(only));
        }
        assert_eq!(single.next_edge(&stats), None);
    }

    /// Unvisited actions take the mixed value; a visited action with a
    /// higher Q gets more improved-policy mass than its prior alone implies,
    /// and the result is a distribution.
    #[test]
    fn improved_policy_completes_unvisited_q_and_rewards_high_q() {
        let priors = vec![0.4f32, 0.4, 0.2];
        let root = GumbelRoot::new(&priors, 0.1, 8, 3, 50.0, 1.0, 5);
        let stats = vec![(4u32, 0.8f32), (4, -0.8), (0, 0.0)];
        let pi = root.improved_policy(&stats);
        let total: f32 = pi.iter().sum();
        assert!((total - 1.0).abs() < 1e-5, "sums to one: {total}");
        assert!(pi[0] > pi[1], "higher Q wins between equal priors");
        assert!(pi[0] > priors[0], "good action gains mass over its prior");
        // The unvisited action's completed Q is v_mix =
        // (0.1 + 8 · (0.4·0.8 + 0.4·(-0.8)) / 0.8) / 9 = 0.1 / 9, between
        // the two visited Qs, so it lands between them in mass too.
        assert!(pi[2] < pi[0] && pi[2] > pi[1]);

        // No visits at all: the improved policy falls back to the prior shape
        // (σ is a constant shift when every completed Q equals v_root).
        let cold = root.improved_policy(&[(0, 0.0), (0, 0.0), (0, 0.0)]);
        for (p, prior) in cold.iter().zip(&priors) {
            assert!((p - prior).abs() < 1e-5, "{p} vs {prior}");
        }
    }
}
