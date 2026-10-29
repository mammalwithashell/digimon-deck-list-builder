//! Presentation paging is a route to a decision, not a game-tree decision.
//! Enumerate every legal commitment on cloned pages, then search those complete
//! routes. No candidate is capped, selected automatically, or exposed to a
//! different observer. Public engine masks and observation contracts are intact.
use super::evaluator::{EvalRequest, Evaluation, PolicyValueFn};
use crate::{
    action::build_action_mask,
    card_registry::CardRegistry,
    enums::PlayerId,
    game::Game,
    observation::{build_observation_tensor_with_mask, ObservationProfileId},
    owned_card_choice::CardChoiceOperation,
};

pub(crate) struct Surface {
    pub plans: Vec<Vec<u16>>,
    pub requests: Vec<EvalRequest>,
    origins: Vec<(usize, u16)>,
    paged: bool,
    request_count: usize,
}

impl Surface {
    pub fn new(
        game: &Game,
        mover: PlayerId,
        registry: &CardRegistry,
        profile: ObservationProfileId,
        mask: Vec<f32>,
    ) -> Self {
        let owned = game
            .pending_selection
            .as_ref()
            .and_then(|s| s.card_choice.as_ref());
        let pages = owned.and_then(|o| o.search_page_shape(game));
        let mut surface = Self {
            plans: Vec::new(),
            requests: Vec::new(),
            origins: Vec::new(),
            paged: pages.is_some(),
            request_count: 0,
        };
        if let Some((start, count)) = pages {
            let prompt_id = owned.unwrap().token().prompt_id.to_string();
            for page in 0..count {
                let mut copy = game.clone();
                let direction = if page < start { 58 } else { 59 };
                let route = vec![direction; page.abs_diff(start)];
                apply_plan(&mut copy, mover, &route);
                let current = copy
                    .pending_selection
                    .as_ref()
                    .unwrap()
                    .card_choice
                    .as_ref()
                    .unwrap()
                    .view(&copy, mover);
                assert_eq!(
                    current.prompt_id, prompt_id,
                    "navigation changed the prompt"
                );
                assert_eq!(current.page_index.parse::<usize>().unwrap(), page);
                let page_mask = build_action_mask(&copy, mover);
                let choices: Vec<u16> = current
                    .rows
                    .iter()
                    .filter(|row| {
                        matches!(row.operation, CardChoiceOperation::CommitCard)
                            || (page == start
                                && matches!(row.operation, CardChoiceOperation::Decline))
                    })
                    .map(|row| row.action_id)
                    .filter(|&a| page_mask[a as usize] > 0.5)
                    .collect();
                surface.add_page(&copy, mover, registry, profile, page_mask, &route, choices);
            }
        } else {
            let choices = mask
                .iter()
                .enumerate()
                .filter(|(_, m)| **m > 0.5)
                .map(|(i, _)| i as u16)
                .collect();
            surface.add_page(game, mover, registry, profile, mask, &[], choices);
        }
        // Stable route order is also the tie-break and PIMC aggregation key.
        let mut order: Vec<usize> = (0..surface.plans.len()).collect();
        order.sort_by(|&a, &b| surface.plans[a].cmp(&surface.plans[b]));
        surface.plans = order.iter().map(|&i| surface.plans[i].clone()).collect();
        surface.origins = order.iter().map(|&i| surface.origins[i]).collect();
        assert!(
            !surface.plans.is_empty(),
            "nonterminal decision has no complete legal route"
        );
        surface
    }

    fn add_page(
        &mut self,
        game: &Game,
        mover: PlayerId,
        registry: &CardRegistry,
        profile: ObservationProfileId,
        mask: Vec<f32>,
        route: &[u16],
        choices: Vec<u16>,
    ) {
        let page = self.requests.len();
        self.requests.push(EvalRequest {
            obs: build_observation_tensor_with_mask(game, mover, registry, profile, &mask),
            mask,
        });
        self.request_count += 1;
        for action in choices {
            let mut plan = route.to_vec();
            plan.push(action);
            self.plans.push(plan);
            self.origins.push((page, action));
        }
    }

    pub fn evaluate(&self, evaluations: &[Evaluation]) -> (Vec<f32>, f32) {
        assert_eq!(evaluations.len(), self.request_count);
        let mut mass = vec![0.; evaluations.len()];
        let mut counts = vec![0usize; evaluations.len()];
        for &(page, action) in &self.origins {
            mass[page] += evaluations[page].policy[action as usize];
            counts[page] += 1;
        }
        let mut priors: Vec<f32> = self
            .origins
            .iter()
            .map(|&(page, action)| {
                let p = evaluations[page].policy[action as usize];
                if self.paged {
                    // Remove navigation mass and avoid favoring a sparsely filled
                    // page merely because its softmax has fewer alternatives.
                    if mass[page] > 0. {
                        p / mass[page] * counts[page] as f32
                    } else {
                        1.
                    }
                } else {
                    p
                }
            })
            .collect();
        let total: f32 = priors.iter().sum();
        let uniform = 1. / priors.len() as f32;
        for p in &mut priors {
            *p = if total > 0. { *p / total } else { uniform };
        }
        let value = evaluations.iter().map(|e| e.value).sum::<f32>() / evaluations.len() as f32;
        (priors, value)
    }
}

pub(crate) fn apply_plan(game: &mut Game, mover: PlayerId, plan: &[u16]) {
    for &action in plan {
        if plan.len() > 1 {
            assert!(
                build_action_mask(game, mover)[action as usize] > 0.5,
                "stale search route"
            );
        }
        game.decode_action(action, mover);
    }
}

/// Extra presentation pages must not exceed the caller's inference batch cap.
pub(crate) fn evaluate_requests(
    requests: &[EvalRequest],
    evaluator: &mut dyn PolicyValueFn,
    cap: usize,
) -> Vec<Evaluation> {
    let mut result = Vec::with_capacity(requests.len());
    for batch in requests.chunks(cap.max(1)) {
        let evaluated = evaluator.evaluate_batch(batch);
        assert_eq!(
            evaluated.len(),
            batch.len(),
            "evaluator returned wrong batch size"
        );
        for (request, evaluation) in batch.iter().zip(&evaluated) {
            assert_eq!(request.mask.len(), evaluation.policy.len());
            debug_assert!(
                request
                    .mask
                    .iter()
                    .zip(&evaluation.policy)
                    .filter(|(m, _)| **m <= 0.5)
                    .map(|(_, p)| p.abs())
                    .sum::<f32>()
                    < 1e-5,
                "evaluator leaked probability onto illegal actions"
            );
        }
        result.extend(evaluated);
    }
    result
}
