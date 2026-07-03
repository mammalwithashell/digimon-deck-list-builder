//! PUCT-MCTS over cloneable `Game` states (task 2.1).
//!
//! Tree shape (design D3): a node is a DECISION POINT — the player the
//! state is waiting on (mulligan decider, `pending_selection.selecting_player`,
//! or the turn player) plus that player's action mask — and an edge is a
//! masked `action_id`. Mid-effect selections are ordinary nodes; there is no
//! turn-level abstraction and no board-symmetry augmentation.
//!
//! **Search granularity (design Open Question, resolved here): forced nodes
//! are auto-advanced.** When applying an edge's action lands in a state with
//! exactly ONE legal action (a PASS-only interrupt window, a forced breeding
//! pass, a single-candidate selection, ...), the expansion applies that
//! action immediately and keeps going instead of materializing a node for
//! it. Forced decision points therefore never consume a simulation and never
//! appear as tree nodes; the tree only branches where a real choice exists.
//! The chain is bounded by `SearchConfig::max_decisions` as a defensive cap
//! (a mis-masked no-op action could otherwise loop forever).
//!
//! **Value sign convention (the subtle part):** every value is stored from
//! the perspective of a specific PLAYER, never "the side to move at some
//! depth". A leaf evaluation (or terminal outcome) is produced for the leaf
//! node's mover; during backup each edge accumulates that value converted to
//! the edge's PARENT-node mover via [`signed_value`] — negated only when the
//! two movers are different players (two-player zero-sum). Consecutive
//! decisions by the SAME player — ubiquitous here via pending selections and
//! phase chains (breeding → main) — therefore back up WITHOUT a sign flip.
//! A depth-alternating negation would be wrong; see
//! `tests/search/perfect_information.rs`.
//!
//! The evaluator is called with batch = 1 for now; the batch-shaped
//! [`PolicyValueFn`] API is deliberate so Phase 4 leaf batching (+ virtual
//! loss) is an internal change.

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::action::build_action_mask;
use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::observation::{
    build_observation_tensor, default_observation_profile, ObservationProfileId,
};

use super::dirichlet::sample_dirichlet;
use super::evaluator::{EvalRequest, PolicyValueFn};

/// Tunables for one search. `Default` is a sensible AlphaZero-style setting
/// for interactive budgets.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Number of simulations (tree traversals). Each simulation expands at
    /// most one new node; forced (single-action) chains are free.
    pub simulations: u32,
    /// PUCT exploration constant.
    pub c_puct: f32,
    /// Dirichlet concentration for root exploration noise.
    pub dirichlet_alpha: f32,
    /// Mixing weight of the root noise: `(1-eps)*prior + eps*noise`.
    /// Set to 0.0 to disable noise entirely (pure network priors).
    pub dirichlet_epsilon: f32,
    /// Seed for ALL search randomness (root noise). Same seed + same root
    /// state + same evaluator => bit-identical `SearchResult`.
    pub seed: u64,
    /// Observation profile for the tensors fed to the evaluator. MUST match
    /// the profile the evaluator's network was trained/exported with — an
    /// exported model's `.meta.json` records it as `observation_profile`
    /// (parse with `observation::parse_observation_profile`). Defaults to
    /// the engine default (`standard_lite_deck_v2`).
    pub observation_profile: ObservationProfileId,
    /// Safety cap on decision depth: bounds both the forced-chain
    /// auto-advance loop and the selection path length. Rollouts truncated
    /// by the cap are scored by the evaluator (not as terminals).
    pub max_decisions: u32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            simulations: 400,
            c_puct: 1.5,
            dirichlet_alpha: 0.3,
            dirichlet_epsilon: 0.25,
            seed: 0,
            observation_profile: default_observation_profile(),
            max_decisions: 512,
        }
    }
}

/// Outcome of one search from a root state.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Per root edge: `(action_id, visit count)`, in mask order. Visit
    /// counts are the search policy (normalize / temperature-sample them).
    pub root_visits: Vec<(u16, u32)>,
    /// Per root edge: `(action_id, mean action-value Q)` from the ROOT
    /// mover's perspective (0.0 for unvisited edges). Diagnostic + review
    /// grading companion to `root_visits`.
    pub root_q: Vec<(u16, f32)>,
    /// Visit-weighted mean value of the root from the root mover's
    /// perspective (terminal outcome if the root is already terminal).
    pub root_value: f32,
    /// Simulations actually run (0 if the root was terminal / choiceless).
    pub simulations_run: u32,
}

/// The player whose decision a state is waiting on: the mulligan decider,
/// else the pending selection's `selecting_player`, else the turn player.
/// (Same resolution rule as the clone-fuzz spike and the RL runners.)
pub fn decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Convert a value expressed for `value_player` into `perspective`'s
/// perspective under the two-player zero-sum assumption.
///
/// THE sign rule: flip on PLAYER IDENTITY, never on tree depth. Consecutive
/// decisions by the same player (pending selections, breeding → main) keep
/// the sign; only an actual change of mover negates.
#[inline]
pub(super) fn signed_value(value: f32, value_player: PlayerId, perspective: PlayerId) -> f32 {
    if value_player == perspective {
        value
    } else {
        -value
    }
}

/// Terminal outcome of a finished game from `perspective`'s side.
pub(super) fn terminal_value(game: &Game, perspective: PlayerId) -> f32 {
    match game.winner {
        Some(w) if w == perspective => 1.0,
        Some(_) => -1.0,
        None => 0.0, // draw / no winner recorded
    }
}

struct Edge {
    action: u16,
    prior: f32,
    child: Option<usize>,
    visits: u32,
    /// Total backed-up value from the perspective of THIS node's mover
    /// (i.e. the player choosing among these edges).
    w: f32,
}

struct Node {
    game: Game,
    /// Decision player at this state (perspective for `Edge::w` and for the
    /// node's leaf evaluation).
    mover: PlayerId,
    /// `Some(v)` when the state is terminal; `v` is for `mover`.
    terminal_value: Option<f32>,
    edges: Vec<Edge>,
}

/// PUCT-MCTS searcher. Holds the card registry needed to build observation
/// tensors for the evaluator; one instance is reusable across searches.
pub struct Mcts<'a> {
    registry: &'a CardRegistry,
}

impl<'a> Mcts<'a> {
    pub fn new(registry: &'a CardRegistry) -> Self {
        Self { registry }
    }

    /// Run a full search from `root`. The root state itself is NEVER
    /// auto-advanced (the caller wants a policy over exactly this state's
    /// legal actions), even if it has a single legal action.
    pub fn search(
        &self,
        root: &Game,
        evaluator: &mut dyn PolicyValueFn,
        config: &SearchConfig,
    ) -> SearchResult {
        debug_assert!(
            root.rules.player_count == 2,
            "search assumes two-player zero-sum backup"
        );
        let mut rng = StdRng::seed_from_u64(config.seed);
        let mut arena: Vec<Node> = Vec::new();

        let (root_node, _root_eval) = self.make_node(root.clone(), false, evaluator, config);
        if root_node.terminal_value.is_some() || root_node.edges.is_empty() {
            return SearchResult {
                root_visits: Vec::new(),
                root_q: Vec::new(),
                root_value: root_node.terminal_value.unwrap_or(0.0),
                simulations_run: 0,
            };
        }
        arena.push(root_node);

        // Root Dirichlet exploration noise (root ONLY): mix
        // (1-eps)*prior + eps*Dirichlet(alpha) over the legal edges.
        if config.dirichlet_epsilon > 0.0 && arena[0].edges.len() > 1 {
            let noise = sample_dirichlet(
                &mut rng,
                config.dirichlet_alpha as f64,
                arena[0].edges.len(),
            );
            let eps = config.dirichlet_epsilon;
            for (edge, n) in arena[0].edges.iter_mut().zip(noise) {
                edge.prior = (1.0 - eps) * edge.prior + eps * n;
            }
        }

        for _ in 0..config.simulations {
            // ── Selection ────────────────────────────────────────────────
            // Descend by PUCT until we hit a terminal node, an unexpanded
            // edge, or the depth cap. `path` holds (node, chosen edge).
            let mut path: Vec<(usize, usize)> = Vec::new();
            let mut cur = 0usize;
            let (leaf_value, leaf_mover) = loop {
                if let Some(tv) = arena[cur].terminal_value {
                    break (tv, arena[cur].mover);
                }
                if path.len() as u32 >= config.max_decisions {
                    // Depth cap: score this node as a leaf via the
                    // evaluator (its edges/priors already exist; we just
                    // refuse to descend further).
                    let node = &arena[cur];
                    let mask = build_action_mask(&node.game, node.mover);
                    let obs = build_observation_tensor(
                        &node.game,
                        node.mover,
                        self.registry,
                        config.observation_profile,
                    );
                    let eval = evaluator.evaluate_batch(&[EvalRequest { obs, mask }]);
                    break (eval[0].value, node.mover);
                }
                let edge_idx = select_edge(&arena[cur], config.c_puct);
                path.push((cur, edge_idx));
                match arena[cur].edges[edge_idx].child {
                    Some(child) => cur = child,
                    None => {
                        // ── Expansion + evaluation ───────────────────────
                        let parent = &arena[cur];
                        let mut game = parent.game.clone();
                        game.decode_action(parent.edges[edge_idx].action, parent.mover);
                        let (node, value) = self.make_node(game, true, evaluator, config);
                        let mover = node.mover;
                        let child_idx = arena.len();
                        arena.push(node);
                        arena[cur].edges[edge_idx].child = Some(child_idx);
                        break (value, mover);
                    }
                }
            };

            // ── Backup ───────────────────────────────────────────────────
            // Convert the leaf value (expressed for `leaf_mover`) to each
            // ancestor edge's parent-mover perspective by PLAYER IDENTITY.
            for &(node_idx, edge_idx) in &path {
                let node_mover = arena[node_idx].mover;
                let v = signed_value(leaf_value, leaf_mover, node_mover);
                let edge = &mut arena[node_idx].edges[edge_idx];
                edge.visits += 1;
                edge.w += v;
            }
        }

        // ── Result extraction ───────────────────────────────────────────
        let root = &arena[0];
        let root_visits: Vec<(u16, u32)> =
            root.edges.iter().map(|e| (e.action, e.visits)).collect();
        let root_q: Vec<(u16, f32)> = root
            .edges
            .iter()
            .map(|e| {
                let q = if e.visits > 0 {
                    e.w / e.visits as f32
                } else {
                    0.0
                };
                (e.action, q)
            })
            .collect();
        let total_visits: u32 = root.edges.iter().map(|e| e.visits).sum();
        let total_w: f32 = root.edges.iter().map(|e| e.w).sum();
        let root_value = if total_visits > 0 {
            total_w / total_visits as f32
        } else {
            0.0
        };
        SearchResult {
            root_visits,
            root_q,
            root_value,
            simulations_run: config.simulations,
        }
    }

    /// Materialize a node from a game state, optionally auto-advancing
    /// forced (single-legal-action) decision points first, and evaluate it.
    ///
    /// Returns the node plus its leaf value FOR THE NODE'S MOVER: the
    /// terminal outcome if the (possibly advanced) state is game-over —
    /// computed WITHOUT calling the evaluator — else the evaluator's value.
    fn make_node(
        &self,
        mut game: Game,
        auto_advance: bool,
        evaluator: &mut dyn PolicyValueFn,
        config: &SearchConfig,
    ) -> (Node, f32) {
        let mut mask = if auto_advance {
            advance_forced_chain(&mut game, config.max_decisions)
        } else {
            Vec::new()
        };

        let mover = decision_player(&game);
        if game.game_over {
            let tv = terminal_value(&game, mover);
            return (
                Node {
                    game,
                    mover,
                    terminal_value: Some(tv),
                    edges: Vec::new(),
                },
                tv,
            );
        }

        if mask.is_empty() {
            mask = build_action_mask(&game, mover);
        }
        let legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter(|(_, &m)| m > 0.5)
            .map(|(i, _)| i as u16)
            .collect();
        if legal.is_empty() {
            // Defensive: a non-terminal state with an empty mask is
            // unreachable in a well-formed game. Treat as a drawn terminal
            // rather than poisoning the tree.
            debug_assert!(false, "non-terminal state with empty action mask");
            return (
                Node {
                    game,
                    mover,
                    terminal_value: Some(0.0),
                    edges: Vec::new(),
                },
                0.0,
            );
        }

        let obs = build_observation_tensor(&game, mover, self.registry, config.observation_profile);
        let evaluations = evaluator.evaluate_batch(&[EvalRequest {
            obs,
            mask: mask.clone(),
        }]);
        let evaluation = &evaluations[0];
        debug_assert_eq!(
            evaluation.policy.len(),
            mask.len(),
            "evaluator policy length must match the mask"
        );
        // Mask discipline: the evaluator must put zero mass on illegal
        // actions (masked_softmax guarantees this for the ONNX path).
        debug_assert!(
            evaluation
                .policy
                .iter()
                .zip(mask.iter())
                .filter(|(_, &m)| m <= 0.5)
                .map(|(p, _)| p.abs())
                .sum::<f32>()
                < 1e-5,
            "evaluator leaked probability mass onto illegal actions"
        );

        // Priors over the legal edges, renormalized defensively (an
        // all-zero legal slice falls back to uniform).
        let legal_mass: f32 = legal.iter().map(|&a| evaluation.policy[a as usize]).sum();
        let edges: Vec<Edge> = legal
            .iter()
            .map(|&a| {
                let prior = if legal_mass > 0.0 {
                    evaluation.policy[a as usize] / legal_mass
                } else {
                    1.0 / legal.len() as f32
                };
                Edge {
                    action: a,
                    prior,
                    child: None,
                    visits: 0,
                    w: 0.0,
                }
            })
            .collect();

        let value = evaluation.value;
        (
            Node {
                game,
                mover,
                terminal_value: None,
                edges,
            },
            value,
        )
    }
}

/// Apply forced (single-legal-action) decision points until the state
/// presents a real choice, terminates, or the defensive cap is hit — the
/// documented search-granularity decision (module docs). Returns the action
/// mask of the RESULTING state when it was the reason the chain stopped
/// (>= 2 or 0 legal actions), or an empty `Vec` when the state is terminal
/// or the cap fired (callers rebuild in that case).
pub(super) fn advance_forced_chain(game: &mut Game, cap: u32) -> Vec<f32> {
    let mut mask = Vec::new();
    let mut forced_steps = 0u32;
    while !game.game_over && forced_steps < cap {
        let pid = decision_player(game);
        mask = build_action_mask(game, pid);
        let mut only_action = None;
        let mut legal_count = 0u32;
        for (i, &m) in mask.iter().enumerate() {
            if m > 0.5 {
                legal_count += 1;
                if legal_count > 1 {
                    break;
                }
                only_action = Some(i as u16);
            }
        }
        if legal_count != 1 {
            break;
        }
        game.decode_action(only_action.expect("single legal action"), pid);
        mask.clear(); // state changed; recompute (or terminal)
        forced_steps += 1;
    }
    mask
}

/// Standard PUCT selection: `argmax_a Q(s,a) + c * P(s,a) * sqrt(N(s)) / (1 + N(s,a))`
/// with `Q` from the node mover's perspective (0.0 for unvisited edges) and
/// `N(s)` the sum of the node's edge visits (+1 so the very first pick is
/// still prior-driven). Deterministic first-max tie-break.
fn select_edge(node: &Node, c_puct: f32) -> usize {
    let stats: Vec<(f32, u32, f32)> = node
        .edges
        .iter()
        .map(|e| {
            let q = if e.visits > 0 {
                e.w / e.visits as f32
            } else {
                0.0
            };
            (e.prior, e.visits, q)
        })
        .collect();
    puct_argmax_q(&stats, c_puct)
}

/// Pure PUCT argmax over `(prior, visits, Q)` edge stats — Q is the MEAN
/// action value already expressed in the selecting player's perspective
/// (IS-MCTS converts stored perspectives before calling). Split out so the
/// scoring math is unit-testable without constructing `Game`s and shareable
/// with the determinized aggregation layer.
pub(super) fn puct_argmax_q(stats: &[(f32, u32, f32)], c_puct: f32) -> usize {
    let total: u32 = stats.iter().map(|&(_, v, _)| v).sum();
    let sqrt_total = ((total + 1) as f32).sqrt();
    let mut best = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for (i, &(prior, visits, q)) in stats.iter().enumerate() {
        let u = c_puct * prior * sqrt_total / (1.0 + visits as f32);
        let score = q + u;
        if score > best_score {
            best_score = score;
            best = i;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identity-keyed sign rule: no flip for the same player,
    /// negation for the opponent — regardless of any notion of depth.
    #[test]
    fn signed_value_flips_on_player_identity_not_depth() {
        // Same player: value passes through unchanged.
        assert_eq!(signed_value(1.0, 0, 0), 1.0);
        assert_eq!(signed_value(-0.25, 1, 1), -0.25);
        // Different player: negated.
        assert_eq!(signed_value(1.0, 0, 1), -1.0);
        assert_eq!(signed_value(-0.25, 1, 0), 0.25);
    }

    /// Simulated backup along a path with CONSECUTIVE same-player movers
    /// (p0 → p0 → p1): a win for p0 credits BOTH p0 edges positively (no
    /// per-level alternation) and the p1 edge negatively.
    #[test]
    fn backup_contributions_do_not_alternate_for_same_player_runs() {
        let path_movers: [PlayerId; 3] = [0, 0, 1];
        let (leaf_value, leaf_mover) = (1.0f32, 0 as PlayerId);
        let contributions: Vec<f32> = path_movers
            .iter()
            .map(|&m| signed_value(leaf_value, leaf_mover, m))
            .collect();
        assert_eq!(contributions, vec![1.0, 1.0, -1.0]);
    }

    #[test]
    fn puct_argmax_prefers_prior_when_unvisited_and_q_when_visited() {
        // Stats are (prior, visits, mean Q).
        // Unvisited: the higher prior wins.
        assert_eq!(puct_argmax_q(&[(0.7, 0, 0.0), (0.3, 0, 0.0)], 1.5), 0);
        // Once the high-prior edge accumulates terrible Q, the alternative
        // overtakes it.
        assert_eq!(puct_argmax_q(&[(0.7, 10, -1.0), (0.3, 1, 0.5)], 1.5), 1);
        // A clearly winning visited edge stays selected over an unvisited
        // low-prior one at modest c_puct.
        assert_eq!(puct_argmax_q(&[(0.5, 8, 0.94), (0.1, 0, 0.0)], 1.5), 0);
    }

    #[test]
    fn default_config_is_sane() {
        let c = SearchConfig::default();
        assert!(c.simulations > 0);
        assert!(c.c_puct > 0.0);
        assert!(c.dirichlet_alpha > 0.0);
        assert!((0.0..1.0).contains(&c.dirichlet_epsilon));
        assert!(c.max_decisions > 0);
    }
}
