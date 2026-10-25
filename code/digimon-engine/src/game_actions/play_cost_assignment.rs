//! Declaration-scoped assignment; reductions remain additive and separately typed.
use super::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayCostAdjustments {
    pub assigned_base: Option<u16>,
    pub effect_cost: CostDelta,
    pub reduction: i32,
    pub pending_payment: Option<u64>,
    /// Fixed credit from only the currently parked, not yet completed payment.
    pub pending_reduction: Option<i32>,
}
// Historical verification protocols used these four Debug fields. New pending
// credit is separately fingerprinted by the current protocol, never injected
// into archival digest bytes.
impl std::fmt::Debug for PlayCostAdjustments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayCostAdjustments")
            .field("assigned_base", &self.assigned_base)
            .field("effect_cost", &self.effect_cost)
            .field("reduction", &self.reduction)
            .field("pending_payment", &self.pending_payment).finish()
    }
}
impl Default for PlayCostAdjustments {
    fn default() -> Self {
        Self::new(CostDelta::Reduce(0))
    }
}
impl PlayCostAdjustments {
    pub fn new(effect_cost: CostDelta) -> Self {
        Self {
            assigned_base: None,
            effect_cost,
            reduction: 0,
            pending_payment: None,
            pending_reduction: None,
        }
    }
    pub fn final_cost(self, original: i32, material_delta: i32) -> u16 {
        let Some(base) = self.assigned_base else {
            return (original - self.reduction).max(0) as u16;
        };
        let assigned = match self.effect_cost {
            CostDelta::Free => return 0,
            // Fixed supplied the declaration base; the later setter replaces it.
            CostDelta::Fixed(_) => base as i32,
            CostDelta::Reduce(n) => base as i32 - n as i32,
        };
        (assigned + material_delta - self.reduction).max(0) as u16
    }
}
impl std::ops::AddAssign<i32> for PlayCostAdjustments {
    fn add_assign(&mut self, n: i32) {
        self.reduction += n;
    }
}
impl std::ops::Add<i32> for PlayCostAdjustments {
    type Output = Self;
    fn add(mut self, n: i32) -> Self {
        self += n;
        self
    }
}
#[derive(Debug, Clone)]
pub(crate) struct CostPaymentReceipt {
    pub id: u64,
    pub target: CardHandle,
    pub key: CostReductionKey,
    pub assignment: u16,
    pub completed: bool,
    pub record_activation: bool,
    pub source_identity: Option<u64>,
}
impl Game {
    /// Assignment eligibility shares execution guards and never pays a cost.
    /// Any eligible assignment can be chosen last; mandatory assignments exclude
    /// the unassigned base, while optional/self-gated payments may all be declined.
    pub(crate) fn available_play_assignment_base(
        &self,
        player: PlayerId,
        card: CardHandle,
        from_hand: bool,
        printed: u16,
    ) -> Option<u16> {
        let target = CostTargetContext {
            card,
            from_hand,
            is_digivolve: false,
            target_permanents: [None, None],
        };
        let mut minimum = None;
        let mut mandatory = false;
        for info in self.before_pay_cost_source_infos(player, Some(card)) {
            let key = CostReductionKey {
                source_card: info.source_card,
                source_permanent: info.source_permanent,
                controller: info.controller,
                card_id: info.card_id,
                effect_slot: info.effect_slot,
                is_under: info.is_under,
            };
            let Some(effects) = self.effects_for_card(&key.card_id, key.source_card) else {
                continue;
            };
            let Some(effect) = effects.get(key.effect_slot as usize) else {
                continue;
            };
            let Some(value) = effect.cost_assignment else {
                continue;
            };
            if self
                .inspect_cost_reduction_candidate(&key, Some(target))
                .is_none()
            {
                continue;
            }
            minimum = Some(minimum.map_or(value, |n: u16| n.min(value)));
            mandatory |= !effect.optional && !effect.pay_cost_self_gated;
        }
        minimum.map(|n| if mandatory { n } else { printed.min(n) })
    }
    pub(crate) fn apply_play_cost_candidate(
        &mut self,
        key: &CostReductionKey,
        target: CostTargetContext,
        total: &mut PlayCostAdjustments,
    ) {
        let Some(effects) = self.effects_for_card(&key.card_id, key.source_card) else {
            return;
        };
        let Some(effect) = effects.get(key.effect_slot as usize) else {
            return;
        };
        let Some(value) = effect.cost_assignment else {
            if let Some(amount) = self.apply_cost_reduction_candidate(key, target, true) {
                *total += amount;
                if self.pending_selection.is_some() {
                    total.pending_reduction = Some(amount);
                }
            }
            return;
        };
        if self
            .inspect_cost_reduction_candidate(key, Some(target))
            .is_none()
        {
            return;
        }
        let Some(pay) = &effect.pay_cost_fn else {
            total.assigned_base = Some(value);
            if effect.max_per_turn > 0 {
                self.record_cost_reducer_activation(key);
            }
            return;
        };
        let id = self.next_cost_payment_receipt;
        self.next_cost_payment_receipt = id.checked_add(1).expect("cost payment receipt exhausted");
        let source_identity = key
            .source_permanent
            .filter(|h| h.index != crate::action::space::BREEDING_TARGET as u8)
            .map(|h| self.capture_digivolve_host_identity(h));
        self.cost_payment_receipts.push(CostPaymentReceipt {
            id,
            target: target.card,
            key: key.clone(),
            assignment: value,
            completed: false,
            record_activation: effect.max_per_turn > 0,
            source_identity,
        });
        let previous = self.active_cost_payment_receipt.replace(id);
        let mut ctx = EffectContext::new_with_cost_target(
            self,
            key.source_card,
            key.source_permanent,
            key.controller,
            target.card,
            target.from_hand,
        );
        ctx.cost_target_permanents = target.target_permanents_vec();
        let success = pay(&mut ctx);
        self.active_cost_payment_receipt = previous;
        if success && !self.dsl_clause_aborted {
            if let Some(r) = self.cost_payment_receipts.iter_mut().find(|r| r.id == id) {
                r.completed = true;
            }
        }
        total.pending_payment = Some(id);
        if self.pending_selection.is_none() {
            self.finish_play_cost_payment(target, total);
        }
    }
    /// Fail only the current payment represented by this continuation. Prior
    /// discounts/assignments and unrelated nested receipts remain intact.
    pub(crate) fn fail_owned_choice_payment_hook(
        &mut self, hook: &mut crate::resume::AfterSelectionHook,
    ) {
        use crate::resume::AfterSelectionHook as Hook;
        match hook {
            Hook::DigivolveReducerChain { paid, .. } => {
                *paid = false;
                self.pending_cost_reduction_amount_override = None;
            }
            Hook::InteractiveDigivolveReducer { amount, .. }
            | Hook::InteractiveOptionUseReducer { amount, .. } => {
                *amount = 0;
                self.pending_cost_reduction_amount_override = None;
            }
            Hook::PlayCostContinuation { accumulated_reduction, .. } => {
                if let Some(current) = accumulated_reduction.pending_reduction.take() {
                    accumulated_reduction.reduction -= current;
                }
                if let Some(id) = accumulated_reduction.pending_payment {
                    if let Some(receipt) = self.cost_payment_receipts.iter_mut().find(|r| r.id == id) {
                        receipt.completed = false;
                    }
                }
                self.pending_cost_reduction_amount_override = None;
            }
            Hook::DeletionWindowContinue { .. } | Hook::DigiXrosLeaveContinuation { .. }
            | Hook::PartitionSecondPlay { .. } => {}
        }
    }

    pub(crate) fn finish_play_cost_payment(
        &mut self,
        target: CostTargetContext,
        total: &mut PlayCostAdjustments,
    ) {
        // Successful continuation settles this provisional credit. Failed
        // completion removes it before this normal finalizer runs.
        total.pending_reduction = None;
        let Some(id) = total.pending_payment.take() else {
            return;
        };
        let Some(index) = self
            .cost_payment_receipts
            .iter()
            .position(|r| r.id == id && r.target == target.card)
        else {
            return;
        };
        let receipt = self.cost_payment_receipts.remove(index);
        if receipt.completed {
            total.assigned_base = Some(receipt.assignment);
            if receipt.record_activation {
                let mut key = receipt.key;
                if let (Some(original), Some(identity)) =
                    (key.source_permanent, receipt.source_identity)
                {
                    let Some(current) =
                        self.resolve_digivolve_host_identity(original.player, identity)
                    else {
                        return;
                    };
                    if !self.player(current.player).battle_area[current.index as usize]
                        .card_sources
                        .iter()
                        .any(|c| c.handle() == key.source_card)
                    {
                        return;
                    }
                    key.source_permanent = Some(current);
                }
                self.record_cost_reducer_activation(&key);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayCostOrder {
    pub choices: Vec<CostReductionCandidate>,
    pub page: usize,
}
impl Game {
    pub(crate) fn install_play_cost_order(&mut self, state: PlayFromHandCostReductionPromptState) {
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: state.player_id,
            previous_phase,
            valid_action_ids: Vec::new(),
            is_optional: false,
            prompt: "Choose which play-cost effect to resolve next".into(),
            effect_choices: None,
            source_card: state.target.card,
            source_permanent: None,
            source_kind: self.effect_source_kind_for_handle(state.target.card),
            callback: Box::new(|_, _| panic!("play cost ordering requires typed resume")),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::PlayFromHandCostReductionPrompt(
                state,
            )],
        });
        self.refresh_play_cost_order();
    }
    fn refresh_play_cost_order(&mut self) {
        let Some(stack) = self.pending_selection_resume.as_ref() else {
            return;
        };
        let Some(state) = active_play_cost_order(stack) else {
            return;
        };
        let order = state.order.as_ref().expect("ordering frame");
        let mut rows: Vec<_> = order
            .choices
            .iter()
            .skip(order.page * 28)
            .take(28)
            .enumerate()
            .map(|(i, c)| crate::selection::EffectChoiceEntry {
                count_choice: None,
                label: c.label.clone(),
                action_id: 30 + i as u16,
                source_card: Some(c.key.source_card),
                source_kind: Some(self.effect_source_kind_for_handle(c.key.source_card)),
                timing: Some(EffectTiming::BeforePayCost),
                is_optional: false,
                keyword: None,
                observation_metadata: Default::default(),
            })
            .collect();
        for (show, id, label) in [
            (order.page > 0, 58, "Previous page"),
            ((order.page + 1) * 28 < order.choices.len(), 59, "Next page"),
        ] {
            if show {
                rows.push(crate::selection::EffectChoiceEntry {
                    count_choice: None,
                    label: label.into(),
                    action_id: id,
                    source_card: None,
                    source_kind: None,
                    timing: None,
                    is_optional: false,
                    keyword: None,
                    observation_metadata: Default::default(),
                });
            }
        }
        if let Some(p) = self.pending_selection.as_mut() {
            p.valid_action_ids = rows.iter().map(|r| r.action_id).collect();
            p.effect_choices = Some(rows);
        }
    }
    pub(crate) fn navigate_play_cost_order(&mut self, action: u16) -> bool {
        if action != 58 && action != 59 {
            return false;
        }
        let Some(stack) = self.pending_selection_resume.as_mut() else {
            return false;
        };
        let Some(state) = active_play_cost_order_mut(stack) else {
            return false;
        };
        let order = state.order.as_mut().expect("ordering frame");
        if action == 58 {
            if order.page == 0 {
                return false;
            }
            order.page -= 1;
        } else {
            if (order.page + 1) * 28 >= order.choices.len() {
                return false;
            }
            order.page += 1;
        }
        self.refresh_play_cost_order();
        true
    }
}
pub(crate) fn active_play_cost_order(
    stack: &crate::resume::ResumeStack,
) -> Option<&PlayFromHandCostReductionPromptState> {
    use crate::resume::ResumeFrame as F;
    match stack.frames.last()? {
        F::PlayFromHandCostReductionPrompt(s) if s.order.is_some() => Some(s),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_play_cost_order(inner),
        _ => None,
    }
}
fn active_play_cost_order_mut(
    stack: &mut crate::resume::ResumeStack,
) -> Option<&mut PlayFromHandCostReductionPromptState> {
    use crate::resume::ResumeFrame as F;
    match stack.frames.last_mut()? {
        F::PlayFromHandCostReductionPrompt(s) if s.order.is_some() => Some(s),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_play_cost_order_mut(inner),
        _ => None,
    }
}

#[cfg(test)]
mod owned_choice_payment_tests {
    use super::*;
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::resume::AfterSelectionHook;

    fn fixture() -> (DebugRunner, CardHandle) {
        let mut card = make_test_card("OWNED-PAY-TARGET", "Payment target");
        card.play_cost = 10;
        let mut r = DebugRunner::builder().add_card(card)
            .hand(0, &["OWNED-PAY-TARGET"]).memory(10).start();
        r.skip_mulligan();
        let target = r.game.player(0).hand[0].handle();
        (r, target)
    }

    fn play_hook(target: CardHandle, total: PlayCostAdjustments) -> AfterSelectionHook {
        AfterSelectionHook::PlayCostContinuation {
            effect_controller: None, player_id: 0, hand_index: 0,
            target: CostTargetContext { card: target, from_hand: true,
                is_digivolve: false, target_permanents: [None, None] },
            cost_delta: CostDelta::Reduce(0), source: PlaySource::ByHand,
            origin: PendingWouldPlayOrigin::Hand, suppress_on_play: false,
            accumulated_reduction: total, processed: Vec::new(),
        }
    }

    #[test]
    fn failed_owned_payment_preserves_prior_discount_and_never_credits_current_or_variable_cost() {
        let (r, target) = fixture();
        let mut total = PlayCostAdjustments::default();
        total += 2; // an earlier, actually paid reducer
        total += 4; // current credit was provisionally added before parking
        total.pending_reduction = Some(4);
        let mut original = play_hook(target, total);
        let mut fork = original.clone();
        let mut game = r.game;
        let mut clone = game.clone();
        for (game, hook) in [(&mut game, &mut original), (&mut clone, &mut fork)] {
            game.pending_cost_reduction_amount_override = Some(9);
            game.fail_owned_choice_payment_hook(hook);
            game.run_after_selection_hook(hook.clone());
            assert_eq!(game.memory, 2, "pay 10 minus only the prior paid discount of 2");
            assert!(game.player(0).hand.is_empty());
            assert_eq!(game.player(0).battle_area[0].top_card().handle(), target);
            assert!(game.pending_cost_reduction_amount_override.is_none());
        }
    }

    #[test]
    fn failed_owned_assignment_invalidates_only_its_receipt_and_preserves_previous_base() {
        let (mut r, target) = fixture();
        let key = CostReductionKey { source_card: target, source_permanent: None,
            controller: 0, card_id: "OWNED-PAY-TARGET".into(), effect_slot: 0, is_under: false };
        r.game.cost_payment_receipts = vec![
            CostPaymentReceipt { id: 7, target, key: key.clone(), assignment: 1,
                completed: true, record_activation: false, source_identity: None },
            CostPaymentReceipt { id: 9, target, key, assignment: 3,
                completed: true, record_activation: false, source_identity: None },
        ];
        let mut total = PlayCostAdjustments::default();
        total.assigned_base = Some(8);
        total += 2;
        total.pending_payment = Some(7);
        let mut hook = play_hook(target, total);
        r.game.fail_owned_choice_payment_hook(&mut hook);
        assert!(!r.game.cost_payment_receipts[0].completed);
        assert!(r.game.cost_payment_receipts[1].completed, "unrelated receipt remains valid");
        r.game.run_after_selection_hook(hook);
        assert_eq!(r.game.memory, 4, "retain prior assigned base 8 minus prior paid discount 2");
        assert_eq!(r.game.player(0).battle_area[0].top_card().handle(), target);
        assert!(r.game.cost_payment_receipts.iter().any(|receipt| receipt.id == 9 && receipt.completed));
    }
}
