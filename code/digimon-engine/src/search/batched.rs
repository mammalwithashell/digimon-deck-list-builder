//! Cooperative independent-tree scheduler. Each tree has at most one pending
//! evaluation and backs it up before selecting again: no virtual loss and no
//! speculative same-tree leaves. Evaluation must be batch-independent.
use super::*;
use crate::search::profiling::{measure, span};

enum Pending {
    Root(PreparedNode),
    Expand {
        prepared: PreparedNode,
        parent: usize,
        edge: usize,
        path: Vec<(usize, usize)>,
    },
    Depth {
        mover: PlayerId,
        path: Vec<(usize, usize)>,
    },
}

struct Tree {
    arena: Vec<Node>,
    config: SearchConfig,
    rng: StdRng,
    completed: u32,
    pending: Option<Pending>,
    /// Gumbel root scheduler (`RootPolicy::Gumbel`), created once the root
    /// node exists; `None` under PUCT.
    gumbel: Option<GumbelRoot>,
}

impl Tree {
    fn new(mcts: &Mcts<'_>, root: &Game, config: &SearchConfig) -> Self {
        debug_assert_eq!(root.rules.player_count, 2);
        let mut tree = Self {
            arena: Vec::new(),
            config: config.clone(),
            rng: StdRng::seed_from_u64(config.seed),
            completed: 0,
            pending: None,
            gumbel: None,
        };
        match mcts.prepare_node(measure("clone", || root.clone()), false, config) {
            PreparedNode::Ready(node, value) => {
                tree.gumbel = gumbel_root_for(&node, value, config);
                tree.arena.push(node);
            }
            prepared => tree.pending = Some(Pending::Root(prepared)),
        }
        tree
    }

    fn backup(&mut self, path: &[(usize, usize)], value: f32, mover: PlayerId) {
        for &(node_idx, edge_idx) in path {
            let node = &mut self.arena[node_idx];
            let v = signed_value(value, mover, node.mover);
            let edge = &mut node.edges[edge_idx];
            edge.visits += 1;
            edge.w += v;
        }
        self.completed += 1;
    }

    fn attach(&mut self, parent: usize, edge: usize, node: Node) {
        let child = self.arena.len();
        self.arena.push(node);
        self.arena[parent].edges[edge].child = Some(child);
    }

    fn next_request(&mut self, mcts: &Mcts<'_>) -> Option<EvalRequest> {
        if let Some(Pending::Root(prepared)) = &self.pending {
            return Some(mcts.node_request(prepared, &self.config));
        }
        assert!(
            self.pending.is_none(),
            "pending evaluation must be backed up before selecting"
        );
        if self.arena[0].terminal_value.is_some() || self.arena[0].edges.is_empty() {
            return None;
        }
        while self.completed < self.config.simulations {
            let mut path = Vec::new();
            let mut cur = 0;
            loop {
                if let Some(value) = self.arena[cur].terminal_value {
                    self.backup(&path, value, self.arena[cur].mover);
                    break;
                }
                if path.len() as u32 >= self.config.max_decisions {
                    let node = &self.arena[cur];
                    let mask = measure("mask", || build_action_mask(&node.game, node.mover));
                    let request = EvalRequest {
                        obs: build_observation_tensor_with_mask(
                            &node.game,
                            node.mover,
                            mcts.registry,
                            self.config.observation_profile,
                            &mask,
                        ),
                        mask,
                    };
                    self.pending = Some(Pending::Depth {
                        mover: node.mover,
                        path,
                    });
                    return Some(request);
                }
                let edge = match (cur, self.gumbel.as_mut()) {
                    (0, Some(root)) => root
                        .next_edge(&root_stats(&self.arena[0]))
                        .expect("Gumbel root budget equals the simulation count"),
                    _ => select_edge(&self.arena[cur], self.config.c_puct),
                };
                path.push((cur, edge));
                match self.arena[cur].edges[edge].child {
                    Some(child) => cur = child,
                    None => {
                        let parent = &self.arena[cur];
                        let mut game = measure("clone", || parent.game.clone());
                        measure("step", || {
                            game.decode_action(parent.edges[edge].action, parent.mover)
                        });
                        match mcts.prepare_node(game, true, &self.config) {
                            PreparedNode::Ready(node, value) => {
                                let mover = node.mover;
                                self.attach(cur, edge, node);
                                self.backup(&path, value, mover);
                                break;
                            }
                            prepared => {
                                let request = mcts.node_request(&prepared, &self.config);
                                self.pending = Some(Pending::Expand {
                                    prepared,
                                    parent: cur,
                                    edge,
                                    path,
                                });
                                return Some(request);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn accept(&mut self, mcts: &Mcts<'_>, evaluation: &Evaluation) {
        match self.pending.take().expect("unsolicited neural evaluation") {
            Pending::Root(prepared) => {
                let (mut node, value) = mcts.finish_node(prepared, evaluation);
                // Same rule as the serial searcher: a Gumbel root supplies
                // its own exploration, so Dirichlet noise is skipped.
                self.gumbel = gumbel_root_for(&node, value, &self.config);
                if self.gumbel.is_none()
                    && self.config.dirichlet_epsilon > 0.0
                    && node.edges.len() > 1
                {
                    let noise = sample_dirichlet(
                        &mut self.rng,
                        self.config.dirichlet_alpha as f64,
                        node.edges.len(),
                    );
                    let eps = self.config.dirichlet_epsilon;
                    for (edge, n) in node.edges.iter_mut().zip(noise) {
                        edge.prior = (1.0 - eps) * edge.prior + eps * n;
                    }
                }
                self.arena.push(node);
            }
            Pending::Expand {
                prepared,
                parent,
                edge,
                path,
            } => {
                let (node, value) = mcts.finish_node(prepared, evaluation);
                let mover = node.mover;
                self.attach(parent, edge, node);
                self.backup(&path, value, mover);
            }
            Pending::Depth { mover, path } => self.backup(&path, evaluation.value, mover),
        }
    }

    fn result(&self) -> SearchResult {
        let root = &self.arena[0];
        if root.terminal_value.is_some() || root.edges.is_empty() {
            return SearchResult {
                root_visits: Vec::new(),
                root_q: Vec::new(),
                root_value: root.terminal_value.unwrap_or(0.),
                simulations_run: 0,
                root_policy: None,
                chosen_action: None,
            };
        }
        let total_visits: u32 = root.edges.iter().map(|e| e.visits).sum();
        let total_w: f32 = root.edges.iter().map(|e| e.w).sum();
        let (root_policy, chosen_action) = gumbel_outputs(root, self.gumbel.as_ref());
        SearchResult {
            root_visits: root.edges.iter().map(|e| (e.action, e.visits)).collect(),
            root_q: root
                .edges
                .iter()
                .map(|e| {
                    (
                        e.action,
                        if e.visits > 0 {
                            e.w / e.visits as f32
                        } else {
                            0.
                        },
                    )
                })
                .collect(),
            root_value: if total_visits > 0 {
                total_w / total_visits as f32
            } else {
                0.
            },
            simulations_run: self.completed,
            root_policy,
            chosen_action,
        }
    }
}

impl Mcts<'_> {
    /// Search independent roots together, preserving the serial algorithm in
    /// each tree. One root/config per slot; results retain input order. No
    /// tree can have more than one outstanding leaf. A deterministic,
    /// batch-independent evaluator gives the exact serial SearchResult.
    pub fn search_batch(
        &self,
        roots: &[Game],
        evaluator: &mut dyn PolicyValueFn,
        configs: &[SearchConfig],
    ) -> Vec<SearchResult> {
        let _profile = span("search_batch");
        assert_eq!(roots.len(), configs.len(), "one configuration per root");
        let mut trees: Vec<_> = roots
            .iter()
            .zip(configs)
            .map(|(r, c)| Tree::new(self, r, c))
            .collect();
        loop {
            let mut requests = Vec::new();
            let mut slots = Vec::new();
            for (slot, tree) in trees.iter_mut().enumerate() {
                if let Some(request) = tree.next_request(self) {
                    slots.push(slot);
                    requests.push(request);
                }
            }
            if requests.is_empty() {
                break;
            }
            let evaluations = evaluator.evaluate_batch(&requests);
            assert_eq!(
                evaluations.len(),
                requests.len(),
                "evaluator returned wrong batch size"
            );
            for (slot, evaluation) in slots.into_iter().zip(&evaluations) {
                trees[slot].accept(self, evaluation);
            }
        }
        trees.iter().map(Tree::result).collect()
    }
}
