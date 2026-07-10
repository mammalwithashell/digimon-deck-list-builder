//! Determinized aggregation over hidden information (task 2.3) + the
//! no-X-ray guard (task 2.5).
//!
//! Two modes behind one config knob (spec: "PIMC and IS-MCTS aggregation
//! modes"):
//!
//! - **PIMC** (design D1, the default): sample K concrete worlds via the
//!   Phase 1 materializer, run K INDEPENDENT [`Mcts`] searches (per-world
//!   budget = `simulations / worlds`, remainder dropped), and SUM root
//!   visit counts across worlds. `root_q` is the visit-weighted average of
//!   per-world Q per action; `root_value` is the plain mean of the K world
//!   root values. Known limitation (design D6): strategy fusion — each
//!   world is searched as if it will be revealed.
//! - **IS-MCTS** (single-observer): ONE tree keyed by the CHOSEN-action
//!   path, re-determinized every iteration. Each simulation materializes a
//!   fresh world, replays the descent's actions onto it, restricts PUCT
//!   selection to the actions legal IN THAT WORLD (subset-legality; edges
//!   for late-appearing legal actions are created lazily), expands and
//!   evaluates on the world, and backs up along the path. Statistics
//!   aggregate over worlds, which reduces strategy fusion at higher cost.
//!
//! **The no-X-ray invariant, strong form.** `determinized_search` is a
//! function of the viewer's INFOSET (+ seeds), never of the true hidden
//! arrangement. The Phase 1 materializer guarantees this in distribution
//! but not per seed: it re-deals the pooled hidden card INSTANCES in their
//! source order, so a hidden-to-hidden permutation of the true state would
//! permute the pool and change the sampled world for a fixed seed. We
//! therefore CANONICALIZE first: clone the root and sort each player's
//! pooled hidden collections (opponent hand ∪ deck ∪ face-down security;
//! viewer deck ∪ face-down security; both digitama decks) by
//! `(data_index, card_index)`, then materialize from the canonical clone.
//! A hidden-to-hidden permutation moves the same instances between hidden
//! zones, so the sorted union — and hence every sampled world — is
//! BITWISE IDENTICAL. Canonicalization is itself a hidden-to-hidden
//! rearrangement (infoset-preserving), and the materializer re-deals
//! everything pooled anyway, so it changes nothing about the sampling
//! distribution. Guarded by `tests/search/determinized.rs`.
//!
//! **Root sanity contracts** (assertions, loud on purpose):
//! - the root must be the VIEWER's decision point (materializing across an
//!   opponent-owned pending selection is meaningless — its valid indices
//!   referenced the re-dealt hand; Phase 1 module docs);
//! - the viewer's legal-action set at the root must be IDENTICAL in every
//!   materialized world (it is infoset-determined; a divergence is a
//!   determinization bug worth failing loudly on).
//!
//! **IS-MCTS corners cut (documented honestly — it is the configurable
//! alternative, not the default):**
//! - a node's priors come from the FIRST world that expanded it; revisits
//!   do not re-evaluate, and actions that only become legal in later worlds
//!   get a uniform `1/|legal-now|` default prior;
//! - each edge's Q is stored in the perspective of the mover who first
//!   created it and is converted (two-player zero-sum negation) when the
//!   current world's mover differs;
//! - the PUCT parent-visit term counts visits over the currently-legal
//!   subset only;
//! - the first simulation is spent evaluating the root (so root visit
//!   totals are `simulations - 1`).

use std::collections::BTreeMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::action::build_action_mask;
use crate::card_registry::CardRegistry;
use crate::card_source::CardSource;
use crate::determinize::materialize;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::observation::build_observation_tensor;
use crate::player::Player;

use super::dirichlet::sample_dirichlet;
use super::evaluator::{EvalRequest, PolicyValueFn};
use super::mcts::{
    advance_forced_chain, decision_player, puct_argmax_q, signed_value, terminal_value, Mcts,
    SearchConfig, SearchResult,
};

/// How hidden-information uncertainty is aggregated (design D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterminizationMode {
    /// K independent world searches; root visits summed (the default).
    Pimc,
    /// One infoset-keyed tree, re-determinized every iteration.
    IsMcts,
}

/// Configuration for a determinized search.
#[derive(Debug, Clone)]
pub struct DeterminizedConfig {
    /// Number of sampled worlds K (PIMC only; IS-MCTS re-determinizes every
    /// iteration and ignores this).
    pub worlds: u32,
    pub mode: DeterminizationMode,
    /// The inner search settings. `simulations` is the TOTAL budget:
    /// PIMC gives each world `simulations / worlds` (remainder dropped);
    /// IS-MCTS runs `simulations` iterations on its single tree.
    pub search: SearchConfig,
}

impl Default for DeterminizedConfig {
    fn default() -> Self {
        Self {
            worlds: 8,
            mode: DeterminizationMode::Pimc,
            search: SearchConfig::default(),
        }
    }
}

/// Search `root` from `viewer`'s perspective without X-raying hidden
/// information: worlds are sampled from the viewer's infoset (canonicalized
/// — see module docs) and all rollout knowledge comes from those worlds'
/// committed piles. Deterministic per `config.search.seed`.
///
/// Panics (loudly, by contract) when the root is not the viewer's decision
/// point, on non-2-player/opaque games (Phase 1 scope), or when a
/// determinization bug makes the viewer's root action surface differ
/// between worlds.
pub fn determinized_search(
    registry: &CardRegistry,
    root: &Game,
    viewer: PlayerId,
    evaluator: &mut dyn PolicyValueFn,
    config: &DeterminizedConfig,
) -> SearchResult {
    assert_eq!(
        root.players.len(),
        2,
        "determinized search is 2-player only"
    );
    assert!((viewer as usize) < 2, "viewer out of range");
    if root.game_over {
        return SearchResult {
            root_visits: Vec::new(),
            root_q: Vec::new(),
            root_value: terminal_value(root, viewer),
            simulations_run: 0,
        };
    }
    assert_eq!(
        decision_player(root),
        viewer,
        "determinized search must root at the VIEWER's decision point \
         (an opponent-owned pending selection references their re-dealt hand)"
    );

    let canon = canonicalize_hidden(root, viewer);
    match config.mode {
        DeterminizationMode::Pimc => pimc_search(registry, &canon, viewer, evaluator, config),
        DeterminizationMode::IsMcts => ismcts_search(registry, &canon, viewer, evaluator, config),
    }
}

// ───────────────────────────── PIMC ─────────────────────────────

fn pimc_search(
    registry: &CardRegistry,
    canon: &Game,
    viewer: PlayerId,
    evaluator: &mut dyn PolicyValueFn,
    config: &DeterminizedConfig,
) -> SearchResult {
    // Respect the TOTAL simulation budget. A naive `(sims / worlds).max(1)`
    // per-world floor would overshoot (sims=1, worlds=8 → 8 sims), so clamp
    // the world count to the available budget instead; the integer-division
    // remainder is dropped as documented. `simulations == 0` runs a single
    // 0-sim world so the result still enumerates the root's legal actions
    // with zero visits (same semantic as the core searcher at 0 sims).
    let (worlds, per_world) = if config.search.simulations == 0 {
        (1, 0)
    } else {
        let w = config.worlds.max(1).min(config.search.simulations);
        (w, config.search.simulations / w)
    };
    let mcts = Mcts::new(registry);

    let mut ref_legal: Option<Vec<u16>> = None;
    let mut visits: BTreeMap<u16, u32> = BTreeMap::new();
    let mut weighted_q: BTreeMap<u16, f64> = BTreeMap::new();
    let mut value_sum = 0.0f64;
    let mut sims_total = 0u32;

    for world_idx in 0..worlds {
        let world_seed = derive_seed(config.search.seed, STREAM_WORLD, world_idx as u64);
        let world = sample_world(canon, viewer, world_seed);
        check_root_surface(&world, viewer, &mut ref_legal, world_idx);

        let world_config = SearchConfig {
            simulations: per_world,
            seed: derive_seed(world_seed, STREAM_SEARCH, world_idx as u64),
            ..config.search.clone()
        };
        let result = mcts.search(&world, evaluator, &world_config);
        sims_total += result.simulations_run;
        value_sum += result.root_value as f64;
        for ((action, v), (action_q, q)) in result.root_visits.iter().zip(result.root_q.iter()) {
            debug_assert_eq!(action, action_q, "visits/q must be parallel");
            *visits.entry(*action).or_insert(0) += v;
            *weighted_q.entry(*action).or_insert(0.0) += (*q as f64) * (*v as f64);
        }
    }

    let root_visits: Vec<(u16, u32)> = visits.iter().map(|(&a, &v)| (a, v)).collect();
    let root_q: Vec<(u16, f32)> = visits
        .iter()
        .map(|(&a, &v)| {
            let q = if v > 0 {
                (weighted_q[&a] / v as f64) as f32
            } else {
                0.0
            };
            (a, q)
        })
        .collect();
    SearchResult {
        root_visits,
        root_q,
        root_value: (value_sum / worlds as f64) as f32,
        simulations_run: sims_total,
    }
}

// ──────────────────────────── IS-MCTS ────────────────────────────

struct IsEdge {
    prior: f32,
    visits: u32,
    /// Total backed-up value in `perspective`'s frame.
    w: f32,
    /// The mover who created this edge (fixed; later worlds convert).
    perspective: PlayerId,
    child: Option<usize>,
}

#[derive(Default)]
struct IsNode {
    /// Whether some world has evaluated this node (set its priors).
    evaluated: bool,
    /// BTreeMap for deterministic iteration order.
    edges: BTreeMap<u16, IsEdge>,
}

fn ismcts_search(
    registry: &CardRegistry,
    canon: &Game,
    viewer: PlayerId,
    evaluator: &mut dyn PolicyValueFn,
    config: &DeterminizedConfig,
) -> SearchResult {
    let search = &config.search;
    let mut noise_rng = StdRng::seed_from_u64(derive_seed(search.seed, STREAM_NOISE, 0));
    let mut arena: Vec<IsNode> = vec![IsNode::default()];
    let mut ref_legal: Option<Vec<u16>> = None;

    for iteration in 0..search.simulations {
        let world_seed = derive_seed(search.seed, STREAM_WORLD, iteration as u64);
        let mut world = sample_world(canon, viewer, world_seed);
        check_root_surface(&world, viewer, &mut ref_legal, iteration);

        // ── Descend on THIS world ────────────────────────────────────
        let mut cur = 0usize;
        let mut path: Vec<(usize, u16)> = Vec::new();
        let (leaf_value, leaf_mover) = loop {
            if world.game_over {
                break (terminal_value(&world, viewer), viewer);
            }
            let pid = decision_player(&world);
            let mask = build_action_mask(&world, pid);
            let legal = legal_set(&mask);
            if legal.is_empty() {
                debug_assert!(false, "non-terminal state with empty action mask");
                break (0.0, pid);
            }

            if !arena[cur].evaluated {
                // Expansion: this world sets the node's priors.
                let (policy, value) = evaluate(registry, evaluator, &world, pid, &mask, search);
                let legal_mass: f32 = legal.iter().map(|&a| policy[a as usize]).sum();
                let uniform = 1.0 / legal.len() as f32;
                let node = &mut arena[cur];
                for &a in &legal {
                    let prior = if legal_mass > 0.0 {
                        policy[a as usize] / legal_mass
                    } else {
                        uniform
                    };
                    node.edges.insert(
                        a,
                        IsEdge {
                            prior,
                            visits: 0,
                            w: 0.0,
                            perspective: pid,
                            child: None,
                        },
                    );
                }
                node.evaluated = true;
                // Root Dirichlet noise, once, at root expansion.
                if cur == 0 && search.dirichlet_epsilon > 0.0 && node.edges.len() > 1 {
                    let noise = sample_dirichlet(
                        &mut noise_rng,
                        search.dirichlet_alpha as f64,
                        node.edges.len(),
                    );
                    let eps = search.dirichlet_epsilon;
                    for (edge, n) in node.edges.values_mut().zip(noise) {
                        edge.prior = (1.0 - eps) * edge.prior + eps * n;
                    }
                }
                break (value, pid);
            }

            if path.len() as u32 >= search.max_decisions {
                // Depth cap: score the current world here as a leaf.
                let (_, value) = evaluate(registry, evaluator, &world, pid, &mask, search);
                break (value, pid);
            }

            // Subset-legality: create lazy edges for actions this world
            // legalizes that earlier worlds never saw (uniform default
            // prior — documented corner).
            {
                let uniform = 1.0 / legal.len() as f32;
                let node = &mut arena[cur];
                for &a in &legal {
                    node.edges.entry(a).or_insert(IsEdge {
                        prior: uniform,
                        visits: 0,
                        w: 0.0,
                        perspective: pid,
                        child: None,
                    });
                }
            }

            // PUCT over the CURRENT world's legal subset, Q converted into
            // the current mover's frame.
            let chosen = {
                let node = &arena[cur];
                let stats: Vec<(f32, u32, f32)> = legal
                    .iter()
                    .map(|a| {
                        let e = &node.edges[a];
                        let q = if e.visits > 0 {
                            signed_value(e.w / e.visits as f32, e.perspective, pid)
                        } else {
                            0.0
                        };
                        (e.prior, e.visits, q)
                    })
                    .collect();
                legal[puct_argmax_q(&stats, search.c_puct)]
            };

            path.push((cur, chosen));
            world.decode_action(chosen, pid);
            // Same granularity rule as the core: forced chains collapse.
            let _ = advance_forced_chain(&mut world, search.max_decisions);

            cur = match arena[cur].edges[&chosen].child {
                Some(child) => child,
                None => {
                    arena.push(IsNode::default());
                    let child = arena.len() - 1;
                    arena[cur]
                        .edges
                        .get_mut(&chosen)
                        .expect("edge exists")
                        .child = Some(child);
                    child
                }
            };
        };

        // ── Backup (identity-keyed sign, per stored edge perspective) ──
        for &(node_idx, action) in &path {
            let edge = arena[node_idx]
                .edges
                .get_mut(&action)
                .expect("path edge exists");
            edge.visits += 1;
            edge.w += signed_value(leaf_value, leaf_mover, edge.perspective);
        }
    }

    // ── Result extraction (root edges' perspective is always the viewer) ──
    let root = &arena[0];
    let root_visits: Vec<(u16, u32)> = root.edges.iter().map(|(&a, e)| (a, e.visits)).collect();
    let root_q: Vec<(u16, f32)> = root
        .edges
        .iter()
        .map(|(&a, e)| {
            debug_assert_eq!(e.perspective, viewer, "root edges are viewer-owned");
            let q = if e.visits > 0 {
                e.w / e.visits as f32
            } else {
                0.0
            };
            (a, q)
        })
        .collect();
    let total_visits: u32 = root.edges.values().map(|e| e.visits).sum();
    let total_w: f32 = root.edges.values().map(|e| e.w).sum();
    SearchResult {
        root_visits,
        root_q,
        root_value: if total_visits > 0 {
            total_w / total_visits as f32
        } else {
            0.0
        },
        simulations_run: search.simulations,
    }
}

// ─────────────────────────── shared plumbing ───────────────────────────

const STREAM_WORLD: u64 = 0xD1B54A32D192ED03;
const STREAM_SEARCH: u64 = 0x2545F4914F6CDD1D;
const STREAM_NOISE: u64 = 0x9E6C63D0876A9A47;

/// splitmix64 — decorrelated deterministic seed derivation.
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

fn derive_seed(base: u64, stream: u64, index: u64) -> u64 {
    splitmix64(base ^ splitmix64(stream.wrapping_add(splitmix64(index))))
}

fn legal_set(mask: &[f32]) -> Vec<u16> {
    mask.iter()
        .enumerate()
        .filter(|(_, &m)| m > 0.5)
        .map(|(i, _)| i as u16)
        .collect()
}

/// Materialize one world with the task-2.5 hygiene guards: no reveal source
/// (D1: chance frozen by world choice) and, in debug builds, the full
/// Phase 1 invariant checker on every sampled world.
fn sample_world(canon: &Game, viewer: PlayerId, seed: u64) -> Game {
    let world = materialize(canon, viewer, seed);
    debug_assert!(
        world.reveal_source.is_none(),
        "materialized world must carry no live reveal source"
    );
    #[cfg(debug_assertions)]
    if let Err(e) = crate::determinize::check_world_invariants(canon, viewer, &world) {
        panic!("world invariants violated (viewer {viewer}, seed {seed}): {e}");
    }
    world
}

/// Root sanity: the world is waiting on the viewer, and the viewer's root
/// legal-action set is identical across worlds (it is infoset-determined).
fn check_root_surface(
    world: &Game,
    viewer: PlayerId,
    ref_legal: &mut Option<Vec<u16>>,
    world_idx: u32,
) {
    assert_eq!(
        decision_player(world),
        viewer,
        "world {world_idx}: decision player diverged from the viewer"
    );
    let legal = legal_set(&build_action_mask(world, viewer));
    match ref_legal {
        None => *ref_legal = Some(legal),
        Some(reference) => assert_eq!(
            reference, &legal,
            "determinization bug: the viewer's root legal-action set must be \
             infoset-determined, but world {world_idx} disagrees"
        ),
    }
}

fn evaluate(
    registry: &CardRegistry,
    evaluator: &mut dyn PolicyValueFn,
    world: &Game,
    pid: PlayerId,
    mask: &[f32],
    search: &SearchConfig,
) -> (Vec<f32>, f32) {
    let obs = build_observation_tensor(world, pid, registry, search.observation_profile);
    let mut evaluations = evaluator.evaluate_batch(&[EvalRequest {
        obs,
        mask: mask.to_vec(),
    }]);
    let evaluation = evaluations.pop().expect("one evaluation per request");
    debug_assert_eq!(evaluation.policy.len(), mask.len());
    (evaluation.policy, evaluation.value)
}

// ───────────────────────── canonicalization ─────────────────────────

/// Clone `root` and rearrange each player's HIDDEN collections into a
/// canonical order, so the materializer's per-seed output depends only on
/// the viewer's infoset (see module docs — the strong no-X-ray form). A
/// pure hidden-to-hidden rearrangement: infoset-preserving by definition.
fn canonicalize_hidden(root: &Game, viewer: PlayerId) -> Game {
    let mut canon = root.clone();
    // Viewer: own hand is known (untouched); deck ∪ face-down security is
    // the hidden pool.
    canonicalize_player_hidden(&mut canon.players[viewer as usize], false);
    // Opponent: hand is hidden too.
    canonicalize_player_hidden(&mut canon.players[(1 - viewer) as usize], true);
    canon
}

/// Sort key: `data_index` orders by card id (the shared card store is
/// id-sorted), `card_index` breaks ties among copies deterministically.
fn card_key(c: &CardSource) -> (usize, u16) {
    (c.data_index, c.card_index)
}

/// Pool the player's hidden collections (deck [+ hand] + face-down
/// security), sort by [`card_key`], and redistribute deterministically
/// (hand → face-down security slots → deck). Pinned (face-up) security
/// cards keep their slots. The digitama deck is sorted in place.
fn canonicalize_player_hidden(p: &mut Player, include_hand: bool) {
    let mut pool: Vec<CardSource> = std::mem::take(&mut p.deck);
    let hand_count = p.hand.len();
    if include_hand {
        pool.append(&mut p.hand);
    }
    let face_up = p.face_up_security.clone();
    let old_security = std::mem::take(&mut p.security);
    let mut slots: Vec<Option<CardSource>> = Vec::with_capacity(old_security.len());
    for c in old_security {
        if face_up.contains(&c.card_index) {
            slots.push(Some(c));
        } else {
            pool.push(c);
            slots.push(None);
        }
    }

    pool.sort_by_key(card_key);

    let mut cards = pool.into_iter();
    if include_hand {
        p.hand.extend(cards.by_ref().take(hand_count));
    }
    for slot in slots.iter_mut() {
        if slot.is_none() {
            *slot = Some(cards.next().expect("canonical refill underflow"));
        }
    }
    p.security = slots
        .into_iter()
        .map(|s| s.expect("all security slots filled"))
        .collect();
    p.deck = cards.collect();

    p.digitama_deck.sort_by_key(card_key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_seed_is_deterministic_and_stream_separated() {
        assert_eq!(
            derive_seed(42, STREAM_WORLD, 3),
            derive_seed(42, STREAM_WORLD, 3)
        );
        assert_ne!(
            derive_seed(42, STREAM_WORLD, 3),
            derive_seed(42, STREAM_WORLD, 4)
        );
        assert_ne!(
            derive_seed(42, STREAM_WORLD, 3),
            derive_seed(42, STREAM_SEARCH, 3)
        );
        assert_ne!(
            derive_seed(42, STREAM_WORLD, 3),
            derive_seed(43, STREAM_WORLD, 3)
        );
    }

    #[test]
    fn default_config_is_pimc_k8() {
        let c = DeterminizedConfig::default();
        assert_eq!(c.worlds, 8);
        assert_eq!(c.mode, DeterminizationMode::Pimc);
        assert!(c.search.simulations > 0);
    }
}
