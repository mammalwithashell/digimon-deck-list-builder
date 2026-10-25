//! Effect-owned DNA declaration. Both physical materials stay in their original
//! areas until the selected recipe has finished its player-visible payments.
use super::*;
use crate::resume::{OuterContinuation, ResumeFrame, ResumeStack};
use crate::trigger_context::TriggerContext;

/// Recipe identity includes orientation: equal memory costs do not erase a
/// player's choice of which material will be above the other (CR 8-2-2-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EffectDnaHandRoute {
    pub requirement_index: Option<usize>,
    pub field_is_upper: bool,
    pub memory_cost: i16,
}

/// DNA's new resolver state is hashed explicitly; normal replay projections do
/// not acquire new rows when no DNA declaration is parked.
pub(crate) fn verification_rows(game: &Game, include_link_pins: bool) -> Vec<(String, String)> {
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::Object(payment)) = map.get_mut("CompleteCostPayment") {
                    if let Some(receipt) = payment.get_mut("receipt") { *receipt = serde_json::Value::from(0); }
                }
                for value in map.values_mut() { normalize(value); }
            }
            serde_json::Value::Array(values) => {
                // Runtime receipt operations are hashed as correlated V4 refs.
                values.retain(|value|value.get("SecurityPlayCaptured").is_none() && value.get("SecurityPlayFinalize").is_none());
                for value in values { normalize(value); }
            },
            _ => {}
        }
    }
    fn push(path: String, state: &EffectDnaHandDeclaration, rows: &mut Vec<(String, String)>, include_link_pins: bool) {
        let mut declaration = state.clone();
        declaration.outer_conts.clear();
        rows.push((path.clone(), format!("{declaration:?}")));
        for (index, tail) in state.outer_conts.iter().enumerate() {
            let mut steps = serde_json::to_value(tail.tail.as_ref()).expect("compiled DNA tail serializes");
            normalize(&mut steps);
            rows.push((format!("{path}/tail/{index}"), format!("{:?}", (
                steps.to_string(), tail.source_card, tail.source_permanent, tail.player,
                &tail.trigger_context, &tail.runtime, if include_link_pins {tail.bindings.canonical_verification_debug()} else {tail.bindings.legacy_canonical_verification_debug()},
            ))));
        }
    }
    fn frames(path: &str, stack: &ResumeStack, rows: &mut Vec<(String, String)>, include_link_pins: bool) {
        for (index, frame) in stack.frames.iter().enumerate() {
            let path = format!("{path}/{index}");
            match frame {
                ResumeFrame::EffectDnaHandRoute(s) => push(path, s, rows, include_link_pins),
                ResumeFrame::DigivolveReducerPrompt(s) | ResumeFrame::DigivolveReducerSuspend(s) => {
                    if let Some(dna) = &s.dna_effect {
                        push(path.clone(), dna, rows, include_link_pins);
                        let mut chain = s.clone(); chain.dna_effect = None;
                        rows.push((format!("{path}/chain"), format!("{chain:?}")));
                    }
                }
                ResumeFrame::ReplacementBody { inner, .. } | ResumeFrame::FieldTopSourceBatch { inner, .. }
                | ResumeFrame::DelayCancelAfterSelection { inner, .. } | ResumeFrame::DelayHandDigivolveAfterSelection { inner, .. }
                | ResumeFrame::DelayDnaAfterSelection { inner, .. } | ResumeFrame::DelayFieldDnaAfterPayment { inner, .. }
                | ResumeFrame::DelayPlayFromHandAfterSelection { inner, .. } | ResumeFrame::DelayPlayFromUnionAfterSelection { inner, .. }
                | ResumeFrame::DelayBodyAfterCost { inner, .. } => frames(&path, inner, rows, include_link_pins),
                _ => {}
            }
        }
    }
    let mut rows = Vec::new();
    if let Some(stack) = &game.pending_selection_resume { frames("pending", stack, &mut rows, include_link_pins); }
    for (index, hook) in game.after_selection_resume_hooks.0.iter().enumerate() {
        if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
            if let Some(dna) = &state.dna_effect {
                push(format!("hook/{index}"), dna, &mut rows, include_link_pins);
                let mut chain = state.clone(); chain.dna_effect = None;
                rows.push((format!("hook/{index}/chain"), format!("{chain:?}")));
            }
        }
    }
    for (index, scope) in game.effect_dna_would_scopes.iter().enumerate() {
        push(format!("would/{index}"), &scope.declaration, &mut rows, include_link_pins);
        rows.push((format!("would/{index}/cost"), scope.effective_cost.to_string()));
    }
    rows
}

#[derive(Debug, Clone)]
pub(crate) struct EffectDnaHandDeclaration {
    pub player: PlayerId,
    pub field_identity: u64,
    pub field_top: CardHandle,
    pub partner: CardHandle,
    pub result: CardHandle,
    pub delta: CostDelta,
    pub ignore_requirements: bool,
    pub routes: Vec<EffectDnaHandRoute>,
    pub chosen: Option<EffectDnaHandRoute>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub source_kind: EffectSourceKind,
    pub source_controller: PlayerId,
    pub trigger_context: Option<TriggerContext>,
    pub result_clause: Option<(CardHandle, PlayerId)>,
    pub outer_conts: Vec<OuterContinuation>,
}

/// An immediate would-digivolve window retains the declaration before either
/// hand card moves. A resumed replacement revalidates the same physical cards.
#[derive(Debug, Clone)]
pub(crate) struct EffectDnaWouldScope {
    pub declaration: EffectDnaHandDeclaration,
    pub effective_cost: u16,
}

/// Complete a child payment before the declaration whose field payment opened
/// it. Ordinary scope ordering is already applied by the caller. When DNA
/// participates, retain the whole ancestry so moving a DNA child cannot let
/// an ordinary grandparent overtake its still-pending ordinary child.
pub(crate) fn order_nested_dna_payment_hooks(
    indexed: &mut [(usize, crate::resume::AfterSelectionHook)],
) {
    use crate::resume::AfterSelectionHook as Hook;
    use super::digivolve::DigivolveReducerChoice;
    let slots: Vec<_> = indexed.iter().enumerate().filter_map(|(slot, (_, hook))|
        matches!(hook, Hook::DigivolveReducerChain { .. }).then_some(slot)).collect();
    let state_at = |slot: usize| match &indexed[slot].1 {
        Hook::DigivolveReducerChain { state, .. } => state,
        _ => unreachable!("reducer slot"),
    };
    if !slots.iter().any(|&slot| state_at(slot).dna_effect.is_some()) { return; }
    let mut edges: Vec<(CardHandle, CardHandle)> = Vec::new();
    for &slot in &slots {
        let child = state_at(slot);
        let clause = child.dna_effect.as_ref().and_then(|s| s.result_clause)
            .or_else(|| child.effect.as_ref().and_then(|s| s.result_clause));
        let Some(clause) = clause else { continue; };
        let parents: Vec<_> = slots.iter().filter_map(|&other| {
            let parent = state_at(other);
            if parent.result_card() == child.result_card() { return None; }
            let Some(DigivolveReducerChoice::Field(candidate)) = &parent.current else { return None; };
            ((candidate.key.source_card, candidate.key.controller) == clause).then_some(parent.result_card())
        }).collect();
        // A physical clause must identify one enclosing payment. Never invent
        // an edge from self-recursion, ambiguous reused clauses, or a cycle.
        let [parent] = parents.as_slice() else { continue; };
        let mut ancestor = *parent;
        let mut cyclic = ancestor == child.result_card();
        for _ in 0..edges.len() {
            let Some((_, next)) = edges.iter().find(|(card, _)| *card == ancestor) else { break; };
            ancestor = *next;
            cyclic |= ancestor == child.result_card();
        }
        if !cyclic { edges.push((child.result_card(), *parent)); }
    }
    if edges.is_empty() { return; }
    let hooks: Vec<_> = slots.iter().map(|&slot| indexed[slot].clone()).collect();
    let cards: Vec<_> = slots.iter().map(|&slot| state_at(slot).result_card()).collect();
    let mut remaining: Vec<_> = (0..slots.len()).collect();
    let mut order = Vec::new();
    while !remaining.is_empty() {
        let next = remaining.iter().position(|&candidate| !edges.iter().any(|(child, parent)|
            *parent == cards[candidate] && remaining.iter().any(|&i| cards[i] == *child)))
            .expect("typed payment parent graph is acyclic");
        order.push(remaining.remove(next));
    }
    for (&slot, source) in slots.iter().zip(order) { indexed[slot] = hooks[source].clone(); }
}

impl Game {
    pub(crate) fn resolve_effect_dna_hand(
        &self, state: &EffectDnaHandDeclaration,
    ) -> Option<(PermanentHandle, usize, usize)> {
        let field = self.resolve_digivolve_host_identity(state.player, state.field_identity)?;
        if self.player(state.player).battle_area.get(field.index as usize)?.top_card().handle()
            != state.field_top { return None; }
        if self.modifiers.has(field, ModifierType::CannotDigivolve)
            || self.modifiers.player_has(state.player, ModifierType::CannotDigivolveDigimonByEffect)
            || self.permanent_is_unaffected_by_effect(field, state.source_controller, state.source_kind)
        { return None; }
        let hand = &self.player(state.player).hand;
        let partner = hand.iter().position(|c| c.handle() == state.partner)?;
        let result = hand.iter().position(|c| c.handle() == state.result)?;
        if partner == result { return None; }
        let routes = self.effect_dna_hand_recipe_routes(state.player, field, state.partner,
            state.result, state.ignore_requirements);
        if let Some(chosen) = state.chosen {
            if !routes.contains(&chosen) { return None; }
        } else if routes.is_empty() { return None; }
        Some((field, partner, result))
    }

    fn effect_dna_delta(&self, player: PlayerId, delta: CostDelta) -> CostDelta {
        match delta {
            CostDelta::Reduce(n) if n > 0 && (
                self.modifiers.player_has(player, ModifierType::CannotReduceCost)
                || self.modifiers.player_has(player, ModifierType::CannotReduceDigivolveCost)
                || self.modifiers.any_other_player_has(player, ModifierType::OpponentCannotReduceDigivolveCost)
            ) => CostDelta::Reduce(0),
            other => other,
        }
    }

    fn effect_dna_target(&self, state: &EffectDnaHandDeclaration) -> Option<CostTargetContext> {
        let (field, _, _) = self.resolve_effect_dna_hand(state)?;
        // The second material is a card in hand, not a temporary battle-area
        // object. Its field auras and inherited effects must not become active.
        Some(CostTargetContext { card: state.result, from_hand: true,
            is_digivolve: true, target_permanents: [Some(field), None] })
    }

    /// Correlated candidate oracle. Explore real payment branches only on
    /// clones, retaining the live effect's category and controller for immunity.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn effect_dna_hand_routes(
        &self, player: PlayerId, field: PermanentHandle, partner: CardHandle,
        result: CardHandle, delta: CostDelta, ignore_requirements: bool,
        source_controller: PlayerId, source_kind: EffectSourceKind,
        source_card: CardHandle, source_permanent: Option<PermanentHandle>,
    ) -> Vec<EffectDnaHandRoute> {
        if field.player != player || partner == result { return Vec::new(); }
        let Some(host) = self.players.get(player as usize)
            .and_then(|p| p.battle_area.get(field.index as usize)) else { return Vec::new(); };
        let top = host.top_card().handle();
        let candidates = self.effect_dna_hand_recipe_routes(player, field, partner, result, ignore_requirements);
        if candidates.is_empty() { return candidates; }
        let mut probe = self.clone();
        probe.pending_selection = None;
        probe.pending_selection_resume = None;
        probe.after_selection_resume_hooks.0.clear();
        probe.pending_player_digivolve_reduction = 0;
        probe.pending_interactive_digivolve_reduction = 0;
        probe.pending_cost_reduction_amount_override = None;
        let identity = probe.capture_digivolve_host_identity(field);
        let state = EffectDnaHandDeclaration {
            player, field_identity: identity, field_top: top, partner, result, delta,
            ignore_requirements, routes: Vec::new(), chosen: None,
            source_card, source_permanent,
            source_kind, source_controller, trigger_context: probe.current_trigger_context.clone(),
            result_clause: None, outer_conts: Vec::new(),
        };
        candidates.into_iter().filter(|route| probe.effect_dna_hand_route_payable(&state, *route)).collect()
    }

    fn effect_dna_hand_route_payable(&self, state: &EffectDnaHandDeclaration, route: EffectDnaHandRoute) -> bool {
        let mut initial = self.clone();
        let mut state = state.clone();
        state.chosen = Some(route);
        let Some(target) = initial.effect_dna_target(&state) else { return false; };
        let Some((field, _, _)) = initial.resolve_effect_dna_hand(&state) else { return false; };
        initial.enter_deferred_drain();
        initial.current_dna_origin = Some(true);
        initial.effect_source_player = Some(state.source_controller);
        initial.effect_source_card = Some(state.source_card);
        initial.effect_source_permanent = state.source_permanent;
        initial.current_trigger_context = state.trigger_context.clone();
        initial.try_prompt_dna_digivolve_reducer_chain(state.clone(), field, target, true);
        let mut frontier = vec![initial];
        while let Some(mut branch) = frontier.pop() {
            if let Some(pending) = branch.pending_selection.as_ref() {
                let player = pending.selecting_player;
                let mask = crate::action::mask::build_action_mask(&branch, player);
                for action in (0..mask.len()).rev() {
                    if mask[action] == 0.0 { continue; }
                    let mut next = branch.clone();
                    if next.resolve_payment_probe_selection(player, action as u16).is_ok() {
                        frontier.push(next);
                    }
                }
                continue;
            }
            let Some(target) = branch.effect_dna_target(&state) else { continue; };
            let paid = std::mem::take(&mut branch.pending_player_digivolve_reduction);
            let reduction = branch.scan_before_pay_cost_reduction_with_target(state.player,
                CostReductionKind::Digivolve, Some(target));
            branch.scan_before_pay_cost_observers(state.player, Some(target));
            if branch.pending_selection.is_some() { continue; }
            if branch.resolve_effect_dna_hand(&state).is_none() { continue; }
            let base = branch.effect_dna_delta(state.player, state.delta).resolve(route.memory_cost.max(0) as u16);
            let cost = (i32::from(base) - paid - reduction).max(0) as u16;
            if branch.can_pay_memory_for(state.player, cost) { return true; }
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn begin_effect_dna_hand(
        &mut self, player: PlayerId, field: PermanentHandle, partner: CardHandle,
        result: CardHandle, delta: CostDelta, ignore_requirements: bool,
        source_card: CardHandle, source_permanent: Option<PermanentHandle>,
        source_kind: EffectSourceKind, source_controller: PlayerId,
    ) -> Option<PermanentHandle> {
        let routes = self.effect_dna_hand_routes(player, field, partner, result, delta,
            ignore_requirements, source_controller, source_kind, source_card, source_permanent);
        if routes.is_empty() { return None; }
        let field_top = self.player(player).battle_area[field.index as usize].top_card().handle();
        let field_identity = self.capture_digivolve_host_identity(field);
        let mut state = EffectDnaHandDeclaration { player, field_identity, field_top, partner,
            result, delta, ignore_requirements, routes, chosen: None, source_card,
            source_permanent, source_kind, source_controller,
            trigger_context: self.current_trigger_context.clone(), result_clause: None, outer_conts: Vec::new() };
        if state.routes.len() > 1 {
            self.install_effect_dna_route_choice(state);
            return None;
        }
        state.chosen = state.routes.first().copied();
        self.resume_effect_dna_hand(state, false)
    }

    pub(crate) fn resume_effect_dna_hand(
        &mut self, mut state: EffectDnaHandDeclaration, reducers_resolved: bool,
    ) -> Option<PermanentHandle> {
        let previous = (self.effect_source_player, self.effect_source_card,
            self.effect_source_permanent, self.current_dna_origin, self.current_trigger_context.clone());
        self.effect_source_player = Some(state.source_controller);
        self.effect_source_card = Some(state.source_card);
        self.effect_source_permanent = state.source_permanent;
        self.current_dna_origin = Some(true);
        self.current_trigger_context = state.trigger_context.clone();
        let result = self.resume_effect_dna_hand_inner(&state, reducers_resolved);
        if self.pending_selection.is_some() {
            if let Some(parked) = self.effect_dna_declaration_for_result_mut(state.result) {
                parked.result_clause = state.result_clause;
                parked.outer_conts.append(&mut state.outer_conts);
            } else {
                // The merge can pend its When Digivolving effects. DSL tails
                // are attached by the ordinary VM after the synchronous result.
                self.finish_effect_dna_tails(state, result);
            }
        } else { self.finish_effect_dna_tails(state, result); }
        (self.effect_source_player, self.effect_source_card, self.effect_source_permanent,
            self.current_dna_origin, self.current_trigger_context) = previous;
        result
    }

    fn resume_effect_dna_hand_inner(&mut self, state: &EffectDnaHandDeclaration, reducers_resolved: bool) -> Option<PermanentHandle> {
        let paid = if reducers_resolved { std::mem::take(&mut self.pending_player_digivolve_reduction) } else { 0 };
        let (field, _, _) = self.resolve_effect_dna_hand(state)?;
        let route = state.chosen?;
        let target = self.effect_dna_target(state)?;
        if !reducers_resolved {
            let mut payment_state = state.clone();
            payment_state.outer_conts.clear();
            if self.try_prompt_dna_digivolve_reducer_chain(payment_state, field, target, false).is_some() {
                return self.find_battle_permanent_with_top(state.result);
            }
        }
        let reduction = self.scan_before_pay_cost_reduction_with_target(state.player,
            CostReductionKind::Digivolve, Some(target));
        self.scan_before_pay_cost_observers(state.player, Some(target));
        let (field, _, _) = self.resolve_effect_dna_hand(state)?;
        let base = self.effect_dna_delta(state.player, state.delta).resolve(route.memory_cost.max(0) as u16);
        let cost = (i32::from(base) - paid - reduction).max(0) as u16;
        if !self.can_pay_memory_for(state.player, cost) { return None; }
        let mut declaration = state.clone();
        declaration.outer_conts.clear();
        self.effect_dna_would_scopes.push(EffectDnaWouldScope {
            declaration, effective_cost: cost,
        });
        // The immediate window and any parked replacement body carry the
        // DNA event even after this method restores its temporary scope.
        let mut context = self.current_trigger_context.clone().unwrap_or_default();
        context.dna_origin = true;
        self.current_trigger_context = Some(context);
        let outcome = self.try_replace(EffectTiming::WhenPermanentWouldDigivolve,
            crate::replacement::ReplacementSubject::Permanent(field),
            self.infer_effect_cause(state.player), Some(Zone::BattleArea));
        if self.pending_selection.is_some() { return None; }
        self.commit_effect_dna_would_digivolve(state.result, outcome, false)
    }

    pub(crate) fn effect_dna_window_card(
        &self, subject: crate::replacement::ReplacementSubject,
    ) -> Option<CardHandle> {
        let crate::replacement::ReplacementSubject::Permanent(field) = subject else { return None; };
        self.effect_dna_would_scopes.iter().rev().find(|scope| {
            self.resolve_digivolve_host_identity(scope.declaration.player,
                scope.declaration.field_identity) == Some(field)
        }).map(|scope| scope.declaration.result)
    }

    pub(crate) fn commit_effect_dna_would_digivolve(
        &mut self, card: CardHandle, outcome: crate::replacement::ReplacementOutcome,
        run_continuations: bool,
    ) -> Option<PermanentHandle> {
        let index = self.effect_dna_would_scopes.iter()
            .rposition(|scope| scope.declaration.result == card)?;
        let scope = self.effect_dna_would_scopes.remove(index);
        let state = &scope.declaration;
        let previous = (self.effect_source_player, self.effect_source_card,
            self.effect_source_permanent, self.current_dna_origin, self.current_trigger_context.clone());
        self.effect_source_player = Some(state.source_controller);
        self.effect_source_card = Some(state.source_card);
        self.effect_source_permanent = state.source_permanent;
        self.current_dna_origin = Some(true);
        self.current_trigger_context = state.trigger_context.clone();
        let result = if outcome == crate::replacement::ReplacementOutcome::None && !self.game_over {
            self.resolve_effect_dna_hand(state).and_then(|(field, partner, result)| {
                self.dna_digivolve_hand_partner_with_order(field, state.player, partner, result,
                    scope.effective_cost, true, state.chosen.expect("DNA route was selected").field_is_upper)
            })
        } else { None };
        if run_continuations { self.finish_effect_dna_tails(scope.declaration, result); }
        (self.effect_source_player, self.effect_source_card, self.effect_source_permanent,
            self.current_dna_origin, self.current_trigger_context) = previous;
        result
    }

    fn find_battle_permanent_with_top(&self, card: CardHandle) -> Option<PermanentHandle> {
        self.players.iter().enumerate().find_map(|(player, state)| state.battle_area.iter()
            .position(|p| p.top_card().handle() == card)
            .map(|index| PermanentHandle { player: player as PlayerId, index: index as u8 }))
    }

    fn finish_effect_dna_tails(&mut self, mut state: EffectDnaHandDeclaration, result: Option<PermanentHandle>) {
        if let Some(result) = result {
            for tail in &mut state.outer_conts {
                if state.result_clause == Some((tail.source_card, tail.player)) {
                    tail.bindings.record_digivolved(result);
                }
            }
        }
        crate::dsl_cards::step::selections::run_outer_conts(self, state.outer_conts);
    }

    fn install_effect_dna_route_choice(&mut self, state: EffectDnaHandDeclaration) {
        let entries: Vec<_> = state.routes.iter().enumerate().map(|(i, route)| crate::selection::EffectChoiceEntry {
            action_id: crate::action::space::HAND_EFFECT_START + i as u16,
            label: format!("DNA requirement {}: cost {}, {} material on top",
                route.requirement_index.map_or(0, |i| i + 1),
                self.effect_dna_delta(state.player, state.delta).resolve(route.memory_cost.max(0) as u16),
                if route.field_is_upper { "battle-area" } else { "hand" }),
            source_card: Some(state.result), source_kind: Some(EffectSourceKind::Digimon),
            timing: None, is_optional: false, keyword: None, count_choice: None,
            observation_metadata: Default::default(),
        }).collect();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection { card_choice: None, zone_owner: Some(state.player),
            kind: crate::selection::SelectionKind::EffectChoice, selecting_player: state.player,
            previous_phase, valid_action_ids: entries.iter().map(|e| e.action_id).collect(),
            is_optional: false, prompt: "Choose a DNA requirement and material order".into(),
            effect_choices: Some(entries), source_card: state.source_card,
            source_permanent: state.source_permanent, source_kind: state.source_kind,
            callback: Box::new(|_, _| panic!("DNA route choice requires its typed resume frame")), on_decline: None });
        self.pending_selection_resume = Some(ResumeStack { frames: vec![ResumeFrame::EffectDnaHandRoute(state)] });
    }

    pub(crate) fn run_effect_dna_hand_route(&mut self, mut state: EffectDnaHandDeclaration, action: u16) {
        state.chosen = action.checked_sub(crate::action::space::HAND_EFFECT_START)
            .and_then(|i| state.routes.get(i as usize)).copied();
        self.resume_effect_dna_hand(state, false);
    }

    pub(crate) fn effect_dna_declaration_for_result_mut(&mut self, result: CardHandle) -> Option<&mut EffectDnaHandDeclaration> {
        fn find(stack: &mut ResumeStack, result: CardHandle) -> Option<&mut EffectDnaHandDeclaration> {
            for frame in stack.frames.iter_mut().rev() {
                let state = match frame {
                    ResumeFrame::EffectDnaHandRoute(s) if s.result == result => Some(s),
                    ResumeFrame::DigivolveReducerPrompt(s) | ResumeFrame::DigivolveReducerSuspend(s) =>
                        s.dna_effect.as_mut().filter(|s| s.result == result),
                    ResumeFrame::ReplacementBody { inner, .. } | ResumeFrame::FieldTopSourceBatch { inner, .. }
                    | ResumeFrame::DelayCancelAfterSelection { inner, .. } | ResumeFrame::DelayHandDigivolveAfterSelection { inner, .. }
                    | ResumeFrame::DelayDnaAfterSelection { inner, .. } | ResumeFrame::DelayFieldDnaAfterPayment { inner, .. }
                    | ResumeFrame::DelayPlayFromHandAfterSelection { inner, .. } | ResumeFrame::DelayPlayFromUnionAfterSelection { inner, .. }
                    | ResumeFrame::DelayBodyAfterCost { inner, .. } => find(inner, result),
                    _ => None,
                };
                if state.is_some() { return state; }
            }
            None
        }
        if let Some(state) = self.pending_selection_resume.as_mut().and_then(|s| find(s, result)) { return Some(state); }
        for hook in self.after_selection_resume_hooks.0.iter_mut().rev() {
            if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
                if state.dna_effect.as_ref().is_some_and(|s| s.result == result) { return state.dna_effect.as_mut(); }
            }
        }
        self.effect_dna_would_scopes.iter_mut().rev()
            .find(|scope| scope.declaration.result == result).map(|scope| &mut scope.declaration)
    }

    pub(crate) fn bind_effect_dna_clause_owner(&mut self, result: CardHandle, clause: CardHandle, player: PlayerId) -> bool {
        let Some(state) = self.effect_dna_declaration_for_result_mut(result) else { return false; };
        state.result_clause = Some((clause, player)); true
    }

    pub(crate) fn effect_dna_declaration_for_clause_mut(&mut self, clause: CardHandle, player: PlayerId) -> Option<&mut EffectDnaHandDeclaration> {
        fn find(stack: &ResumeStack, clause: CardHandle, player: PlayerId) -> Option<CardHandle> {
            for frame in stack.frames.iter().rev() {
                let found = match frame {
                    ResumeFrame::EffectDnaHandRoute(s) => (s.result_clause == Some((clause, player))).then_some(s.result),
                    ResumeFrame::DigivolveReducerPrompt(s) | ResumeFrame::DigivolveReducerSuspend(s) =>
                        s.dna_effect.as_ref().filter(|s| s.result_clause == Some((clause, player))).map(|s| s.result),
                    ResumeFrame::ReplacementBody { inner, .. } | ResumeFrame::FieldTopSourceBatch { inner, .. }
                    | ResumeFrame::DelayCancelAfterSelection { inner, .. } | ResumeFrame::DelayHandDigivolveAfterSelection { inner, .. }
                    | ResumeFrame::DelayDnaAfterSelection { inner, .. } | ResumeFrame::DelayFieldDnaAfterPayment { inner, .. }
                    | ResumeFrame::DelayPlayFromHandAfterSelection { inner, .. } | ResumeFrame::DelayPlayFromUnionAfterSelection { inner, .. }
                    | ResumeFrame::DelayBodyAfterCost { inner, .. } => find(inner, clause, player),
                    _ => None,
                };
                if found.is_some() { return found; }
            }
            None
        }
        let result = self.pending_selection_resume.as_ref().and_then(|s| find(s, clause, player))
            .or_else(|| self.after_selection_resume_hooks.0.iter().rev().find_map(|hook| {
                if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
                    state.dna_effect.as_ref().filter(|s| s.result_clause == Some((clause, player))).map(|s| s.result)
                } else { None }
            }))
            .or_else(|| self.effect_dna_would_scopes.iter().rev()
                .find(|scope| scope.declaration.result_clause == Some((clause, player)))
                .map(|scope| scope.declaration.result))?;
        self.effect_dna_declaration_for_result_mut(result)
    }
}
