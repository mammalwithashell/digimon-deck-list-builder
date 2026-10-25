//! Versioned bootstrap teacher. Scores only legal actions and entitled information.
//! Scores are preferences, never rules, rewards, or predictions of hidden outcomes.
use super::{current_decision_player, greedy_action, Policy};
use crate::action::{
    explain::{explain_action, ActionKind},
    main_effect_select,
    space::*,
};
use crate::card_data::CardData;
use crate::enums::{CardKind, EffectTiming, GamePhase, PlayerId};
use crate::game::Game;
use crate::owned_card_choice::{CardChoiceOperation, CardChoiceRowView};
use crate::permanent::PermanentHandle;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const DEVELOPMENT_VERSION: &str = "development_v1";

/// A deliberately narrow static description. Only literal, unconditional own
/// draws are fully known. Conditional/search/payment bodies stay unknown; this
/// must not turn broad `resource_flow` text into a promised hand benefit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandBenefit {
    pub literal_draw: Option<u16>,
}

impl HandBenefit {
    pub fn from_steps(steps: &[digimon_dsl::compiled::CompiledStep]) -> Self {
        use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};
        let mut total = 0u16;
        for step in steps {
            match step {
                CompiledStep::Draw {
                    of: CompiledPlayerRef::You,
                    count,
                } => {
                    total = total.saturating_add(*count as u16);
                }
                _ => return Self::default(),
            }
        }
        Self {
            literal_draw: Some(total),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DevelopmentCandidate {
    pub action: u16,
    pub route: String,
    pub score: f32,
    /// Engine route cost BEFORE optional/triggered reducers, never final payment.
    pub base_memory_cost: Option<u16>,
    pub net_hand_gain: Option<i16>,
    pub replacement: bool,
    pub level_gain: u8,
    pub bodies_consumed: u8,
    pub unknown: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DevelopmentDecision {
    pub version: &'static str,
    pub player: PlayerId,
    pub action: u16,
    pub candidates: Vec<DevelopmentCandidate>,
}

#[derive(Debug, Clone, Default)]
struct PageSearch {
    prompt: String,
    seen: BTreeSet<usize>,
    best: Option<(f32, usize, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct DevelopmentPolicy {
    turn: Option<u32>,
    tried: BTreeMap<String, BTreeSet<u16>>,
    page: PageSearch,
    dna_first: Option<(crate::card_source::CardHandle, usize)>,
    /// Visible trigger-order choice, not a monotonically increasing prompt ID.
    resolution_context: String,
}

impl DevelopmentPolicy {
    pub fn decide(&mut self, game: &Game, mask: &[f32]) -> Result<DevelopmentDecision, String> {
        let player = current_decision_player(game);
        let turn = game.turn_count as u32;
        if self.turn != Some(turn) {
            self.reset();
            self.turn = Some(turn);
        }
        let valid: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (*v > 0.0 && i != CONCEDE_GAME as usize).then_some(i as u16))
            .collect();
        if valid.is_empty() {
            return Err("development_v1: no non-concede legal action".into());
        }
        let mut candidates: Vec<_> = valid
            .iter()
            .map(|a| score_action(game, player, *a))
            .collect();
        if let Some(selection) = &game.pending_selection {
            if selection.prompt == crate::selection::DNA_SECOND_MATERIAL_PROMPT {
                if let Some((source, first)) = self
                    .dna_first
                    .filter(|(source, _)| *source == selection.source_card)
                {
                    if let Some(hand) = game
                        .player(player)
                        .hand
                        .iter()
                        .position(|c| c.handle() == source)
                    {
                        for c in &mut candidates {
                            if let Some((score, _, _)) = dna_pair_score(
                                game,
                                player,
                                hand,
                                first,
                                c.action as usize,
                                selection.previous_phase,
                            ) {
                                c.score = score;
                            }
                        }
                    }
                }
            }
        }
        // Opening/raising decisions retain the established heuristic.
        if matches!(
            game.current_phase,
            GamePhase::Mulligan | GamePhase::Breeding | GamePhase::SelectPlayOrder
        ) {
            let a = greedy_action(game, mask);
            if let Some(c) = candidates.iter_mut().find(|c| c.action == a) {
                c.score += 1000.0;
            }
        }
        let paged = self.page_action(game, player, &valid)?;
        if let Some(a) = paged {
            if let Some(c) = candidates.iter_mut().find(|c| c.action == a) {
                c.score += 2000.0;
            }
        } else if game
            .pending_selection
            .as_ref()
            .is_none_or(|s| s.card_choice.is_none())
        {
            self.page = PageSearch::default();
        }
        if game.pending_selection.is_none() {
            self.resolution_context.clear();
        }
        let signature = format!("{}/{}", semantic_signature(game, player, &valid), self.resolution_context);
        let tried = self.tried.entry(signature).or_default();
        let chosen = candidates
            .iter()
            .filter(|c| !tried.contains(&c.action))
            .max_by(|a, b| {
                a.score
                    .total_cmp(&b.score)
                    .then_with(|| {
                        b.base_memory_cost
                            .unwrap_or(u16::MAX)
                            .cmp(&a.base_memory_cost.unwrap_or(u16::MAX))
                    })
                    .then_with(|| b.action.cmp(&a.action))
            })
            .ok_or_else(|| {
                "development_v1: repeated visible state exhausted legal progress choices"
                    .to_string()
            })?;
        let action = chosen.action;
        tried.insert(action);
        if let Some(selection) = &game.pending_selection {
            if selection.kind == crate::selection::SelectionKind::TriggerOrder {
                self.resolution_context = selection.effect_choices.as_ref()
                    .and_then(|choices| choices.iter().find(|choice| choice.action_id == action))
                    .map(|choice| format!("{}/{:?}/{:?}", choice.label, choice.source_kind, choice.timing))
                    .unwrap_or_default();
            }
            if selection.prompt == crate::selection::DNA_FIRST_MATERIAL_PROMPT {
                self.dna_first = Some((selection.source_card, action as usize));
            }
        }
        Ok(DevelopmentDecision {
            version: DEVELOPMENT_VERSION,
            player,
            action,
            candidates,
        })
    }

    fn page_action(
        &mut self,
        game: &Game,
        player: PlayerId,
        valid: &[u16],
    ) -> Result<Option<u16>, String> {
        let Some(owner) = game
            .pending_selection
            .as_ref()
            .and_then(|s| s.card_choice.as_ref())
        else {
            return Ok(None);
        };
        let view = owner.view(game, player);
        let page = view
            .page_index
            .parse::<usize>()
            .map_err(|_| "invalid page index")?;
        if self.page.prompt != view.prompt_id {
            self.page = PageSearch {
                prompt: view.prompt_id.clone(),
                ..Default::default()
            };
        }
        for row in &view.rows {
            if valid.contains(&row.action_id)
                && matches!(
                    row.operation,
                    CardChoiceOperation::RefreshPage | CardChoiceOperation::FinishImpossible
                )
            {
                self.page = PageSearch::default();
                return Ok(Some(row.action_id));
            }
        }
        if self.page.seen.insert(page) {
            for row in &view.rows {
                if !valid.contains(&row.action_id)
                    || !matches!(row.operation, CardChoiceOperation::CommitCard)
                {
                    continue;
                }
                let value = row
                    .card_registry_index
                    .filter(|_| row.identity_visible)
                    .and_then(|i| game.card_data.get(i))
                    .map_or(0.0, |c| card_utility(game, player, c));
                // Sources/materials are costs; prefer sacrificing less value.
                let value = if matches!(
                    row.zone,
                    Some(
                        crate::owned_card_choice::CardChoiceZone::BattleSource
                            | crate::owned_card_choice::CardChoiceZone::BreedingSource
                            | crate::owned_card_choice::CardChoiceZone::BattleTop
                    )
                ) {
                    -value
                } else {
                    value
                };
                if self
                    .page
                    .best
                    .as_ref()
                    .is_none_or(|(best, _, _)| value > *best)
                {
                    self.page.best = Some((value, page, row_key(row)));
                }
            }
        }
        let next = view.rows.iter().find(|r| {
            valid.contains(&r.action_id) && matches!(r.operation, CardChoiceOperation::NextPage)
        });
        if let Some(next) = next.filter(|_| !self.page.seen.contains(&(page + 1))) {
            return Ok(Some(next.action_id));
        }
        if let Some((_, best_page, key)) = &self.page.best {
            if *best_page == page {
                let action = view
                    .rows
                    .iter()
                    .find(|r| {
                        matches!(r.operation, CardChoiceOperation::CommitCard)
                            && row_key(r) == *key
                            && valid.contains(&r.action_id)
                    })
                    .map(|r| r.action_id);
                if action.is_some() {
                    self.page = PageSearch::default();
                    return Ok(action);
                }
                return Err(
                    "development_v1: chosen paged candidate invalidated without refresh".into(),
                );
            }
            let target = view.rows.iter().find(|r| {
                valid.contains(&r.action_id)
                    && if *best_page < page {
                        matches!(r.operation, CardChoiceOperation::PreviousPage)
                    } else {
                        matches!(r.operation, CardChoiceOperation::NextPage)
                    }
            });
            return target
                .map(|r| Some(r.action_id))
                .ok_or_else(|| "development_v1: no route back to selected page".into());
        }
        // Inspection rows have no commit semantics; complete/decline via an actual legal control.
        Ok(view
            .rows
            .iter()
            .find(|r| {
                valid.contains(&r.action_id) && matches!(r.operation, CardChoiceOperation::Decline)
            })
            .map(|r| r.action_id))
    }
}

impl Policy for DevelopmentPolicy {
    fn select(&mut self, game: &Game, mask: &[f32]) -> u16 {
        self.decide(game, mask)
            .expect("development teacher failed; quarantine this game")
            .action
    }
    fn reset(&mut self) {
        *self = Self::default();
    }
}

fn row_key(row: &CardChoiceRowView) -> String {
    format!(
        "{:?}/{:?}/{:?}/{:?}/{:?}",
        row.zone, row.owner, row.carrier_identity, row.position, row.card_id
    )
}

fn semantic_signature(game: &Game, player: PlayerId, valid: &[u16]) -> String {
    // No private opponent identities, security contents or future deck order.
    let mut key = format!(
        "{player}/{:?}/{}/{:?}",
        game.current_phase, game.memory, valid
    );
    for p in &game.players {
        key.push_str(&format!(
            "/counts:{},{},{},{}",
            p.hand.len(),
            p.deck.len(),
            p.security.len(),
            p.trash.len()
        ));
        for permanent in p.battle_area.iter().chain(p.breeding_area.iter()) {
            key.push_str(&format!(
                "/{}:{}:{}",
                permanent.top_card().card_id(&game.card_data),
                permanent.is_suspended,
                permanent.card_sources.len()
            ));
        }
    }
    for card in &game.player(player).hand {
        key.push_str(card.card_id(&game.card_data));
        key.push('/');
    }
    for card in &game.revealed_cards {
        key.push_str(card.card_id(&game.card_data));
        key.push('/');
    }
    if let Some(selection) = &game.pending_selection {
        key.push_str(&format!("/{:?}/{}/{:?}/{:?}", selection.kind, selection.prompt,
            selection.source_permanent, selection.source_kind));
        if let Some(choices) = &selection.effect_choices {
            for choice in choices {
                key.push_str(&format!("/{}/{}/{:?}", choice.action_id, choice.label, choice.timing));
            }
        }
        if let Some(owner) = &selection.card_choice {
            let view = owner.view(game, player);
            key.push_str(&view.page_index);
            for row in view.rows {
                key.push_str(&row_key(&row));
            }
        }
    }
    key
}

fn card_utility(game: &Game, player: PlayerId, card: &CardData) -> f32 {
    let own = game.player(player);
    let copies = own
        .hand
        .iter()
        .filter(|c| c.card_id(&game.card_data) == card.card_id)
        .count();
    let bridge = own
        .battle_area
        .iter()
        .chain(own.breeding_area.iter())
        .any(|base| {
            card.evo_costs.iter().any(|cost| {
                base.level(&game.card_data) == Some(cost.level)
                    && crate::action::mask::evo_color(cost.card_color).is_some_and(|color| {
                        base.top_card()
                            .digimon_colors(&game.card_data)
                            .contains(&color)
                    })
            })
        });
    let body = if card.level == Some(3) && own.battle_area.is_empty() {
        3.0
    } else {
        0.0
    };
    1.0 + if bridge { 4.0 } else { 0.0 } + body - copies.saturating_sub(1).min(3) as f32
}

fn score_action(game: &Game, player: PlayerId, action: u16) -> DevelopmentCandidate {
    let explanation = explain_action(game, player, action);
    let mut c = DevelopmentCandidate {
        action,
        route: format!("{:?}", explanation.kind),
        score: -1.0,
        base_memory_cost: None,
        net_hand_gain: None,
        replacement: false,
        level_gain: 0,
        bodies_consumed: 0,
        unknown: vec![],
    };
    if let Some(selection) = &game.pending_selection {
        if action == PASS {
            c.score = -0.5;
            return c;
        }
        if selection.prompt == crate::selection::DNA_FIRST_MATERIAL_PROMPT {
            if let Some(hand) = game
                .player(player)
                .hand
                .iter()
                .position(|c| c.handle() == selection.source_card)
            {
                if let Some((score, cost, gain)) = best_dna_pair(
                    game,
                    player,
                    hand,
                    Some(action as usize),
                    selection.previous_phase,
                ) {
                    c.score = score;
                    c.base_memory_cost = Some(cost);
                    c.level_gain = gain;
                    c.bodies_consumed = 1;
                    c.route = "dna_material".into();
                    return c;
                }
            }
        }
        if let Some((card, discard, draw_after)) = pending_resource_card(game, player, action) {
            c.score = if discard {
                -card_utility(game, player, card)
            } else {
                card_utility(game, player, card)
            };
            c.route = if discard {
                "hand_payment"
            } else {
                "resource_selection"
            }
            .into();
            c.net_hand_gain = if discard {
                draw_after.map(|draw| draw as i16 - 1)
            } else {
                Some(1)
            };
            if discard {
                if let Some(net) = c.net_hand_gain {
                    c.score += 2.0 * net.clamp(-3, 3) as f32;
                } else {
                    c.unknown.push("payment_downstream_benefit".into());
                }
            }
            return c;
        }
        if let Some(choice) = selection
            .effect_choices
            .as_ref()
            .and_then(|cs| cs.iter().find(|x| x.action_id == action))
        {
            if let Some(count) = choice.count_choice {
                c.score = match count.operation {
                    crate::count_choice::CountChoiceOperation::Commit(value) => {
                        1.0 - value.payable as f32 * 0.01
                    }
                    _ => -100.0,
                };
            } else {
                let flags = &choice.observation_metadata.categories;
                c.score = 0.0
                    + if flags.draw_search { 2.0 } else { 0.0 }
                    + if flags.digivolve { 3.0 } else { 0.0 };
                c.unknown.push("pending_effect_net_resources".into());
            }
        } else {
            c.score = 0.0;
            c.unknown.push("pending_operation_benefit".into());
        }
        return c;
    }
    match explanation.kind {
        ActionKind::Pass => {
            c.score = -6.0;
        }
        ActionKind::Digivolve => {
            let (hand, field) = decode_digivolve(action);
            let host = PermanentHandle {
                player,
                index: field as u8,
            };
            if let (Some(route), Some(base), Some(card)) = (
                game.normal_digivolve_route_for_hand_card(player, hand as usize, host),
                game.digivolve_base_permanent(host),
                game.player(player).hand.get(hand as usize),
            ) {
                c.route = if field == BREEDING_TARGET {
                    "raising_evolution"
                } else {
                    "battle_evolution"
                }
                .into();
                c.base_memory_cost = Some(route.memory_cost);
                c.level_gain = card
                    .level(&game.card_data)
                    .unwrap_or(0)
                    .saturating_sub(base.level(&game.card_data).unwrap_or(0))
                    .min(2);
                c.net_hand_gain = Some(if game.player(player).deck.is_empty() {
                    -1
                } else {
                    0
                });
                c.replacement = c.net_hand_gain == Some(0);
                c.score = 4.0 * c.level_gain as f32;
                let data = &game.card_data[card.data_index];
                if game.player(player).hand.iter().any(|next| {
                    next.handle() != card.handle()
                        && game.card_data[next.data_index]
                            .evo_costs
                            .iter()
                            .any(|cost| {
                                Some(cost.level) == data.level
                                    && crate::action::mask::evo_color(cost.card_color)
                                        .is_some_and(|color| data.colors.contains(&color))
                            })
                }) {
                    c.score += 2.0;
                }
                c.unknown
                    .push("optional_cost_reducers_and_triggered_benefits".into());
            }
        }
        ActionKind::DnaDigivolve => {
            let hand = (action - DNA_DIGIVOLVE_START) as usize;
            if let Some((score, cost, gain)) =
                best_dna_pair(game, player, hand, None, game.current_phase)
            {
                c.score = score;
                c.base_memory_cost = Some(cost);
                c.level_gain = gain;
                c.bodies_consumed = 1;
                c.route = "dna_evolution".into();
                c.net_hand_gain = Some(if game.player(player).deck.is_empty() {
                    -1
                } else {
                    0
                });
                c.replacement = c.net_hand_gain == Some(0);
                // Pair score already includes memory and body costs.
                return c;
            }
        }
        ActionKind::Play => {
            if let Some(card) = game.player(player).hand.get(action as usize) {
                let data = &game.card_data[card.data_index];
                c.base_memory_cost = Some(data.play_cost);
                c.score = if card.is_digimon(&game.card_data) {
                    if data.level == Some(3) {
                        7.0
                    } else {
                        2.0
                    }
                } else if data.card_kind == CardKind::Tamer {
                    2.0
                } else {
                    0.0
                };
                c.net_hand_gain = Some(-1);
                if let Some(effects) =
                    game.effects_for_card(card.card_id(&game.card_data), card.handle())
                {
                    let timing = if data.card_kind == CardKind::Option {
                        EffectTiming::OptionMain
                    } else {
                        EffectTiming::OnPlay
                    };
                    for effect in effects
                        .iter()
                        .filter(|e| e.timing == timing && !e.inherited)
                    {
                        if effect.condition.is_none() && effect.activation_cost_fn.is_none() {
                            if let Some(draw) = effect.development_hand.literal_draw {
                                c.net_hand_gain = c.net_hand_gain.map(|n| {
                                    n + draw.min(game.player(player).deck.len() as u16) as i16
                                });
                            } else {
                                c.net_hand_gain = None;
                            }
                        } else {
                            c.net_hand_gain = None;
                        }
                    }
                }
                c.replacement = c.net_hand_gain == Some(0);
                c.unknown
                    .push("play_cost_modifiers_and_conditional_benefits".into());
            }
        }
        ActionKind::Attack => {
            let (attacker, target) = decode_attack(action);
            let opponent = game.player(1 - player);
            let dp = game
                .effective_dp(PermanentHandle {
                    player,
                    index: attacker as u8,
                })
                .unwrap_or(0);
            if target == SECURITY_TARGET {
                // Never advertise guaranteed lethal through visible defenses.
                c.score = if opponent.security.is_empty() && opponent.battle_area.is_empty() {
                    100.0
                } else {
                    2.0
                };
            } else {
                let enemy_dp = game
                    .effective_dp(PermanentHandle {
                        player: 1 - player,
                        index: target as u8,
                    })
                    .unwrap_or(0);
                c.score = if dp > enemy_dp { 3.0 } else { -4.0 };
            }
        }
        ActionKind::HandEffect | ActionKind::FieldEffect | ActionKind::TrashEffect => {
            let matched = match explanation.kind {
                ActionKind::HandEffect => main_effect_select::hand_main_match(
                    game,
                    player,
                    (action - HAND_EFFECT_START) as usize,
                ),
                ActionKind::TrashEffect => main_effect_select::trash_main_match(
                    game,
                    player,
                    (action - TRASH_EFFECT_START) as usize,
                ),
                _ => {
                    let (field, _) = decode_field_effect(action);
                    main_effect_select::field_main_match(game, player, field as usize)
                }
            };
            if let Some(effect) = matched {
                c.net_hand_gain = effect
                    .development_hand
                    .literal_draw
                    .map(|n| n.min(game.player(player).deck.len() as u16) as i16);
                c.score = 0.0;
            }
            c.unknown
                .push("activated_effect_cost_and_conditional_benefit".into());
        }
        _ => {
            c.unknown.push("unsupported_action_benefit".into());
        }
    }
    if let Some(net) = c.net_hand_gain {
        c.score += 2.0 * net.clamp(-3, 3) as f32;
    }
    if c.replacement {
        c.score += 0.5;
    }
    if let Some(cost) = c.base_memory_cost {
        let memory = if player == 0 {
            game.memory
        } else {
            -game.memory
        };
        c.score -= 0.75 * cost as f32 + 1.5 * (cost as i32 - memory as i32).max(0) as f32;
    }
    c
}

fn dna_pair_score(
    game: &Game,
    player: PlayerId,
    hand: usize,
    first: usize,
    second: usize,
    phase: GamePhase,
) -> Option<(f32, u16, u8)> {
    use crate::dna_digivolve::DnaRouteWindow;
    if first == second {
        return None;
    }
    let window = if phase == GamePhase::EndOfTurnAction {
        DnaRouteWindow::EndOfTurnAction
    } else {
        DnaRouteWindow::Main
    };
    let route = game.dna_route_for_hand_card(player, hand, first, second, window)?;
    let own = game.player(player);
    let base = own.battle_area[first]
        .level(&game.card_data)
        .unwrap_or(0)
        .max(own.battle_area[second].level(&game.card_data).unwrap_or(0));
    let gain = own.hand[hand]
        .level(&game.card_data)
        .unwrap_or(0)
        .saturating_sub(base)
        .min(2);
    let cost = route.memory_cost.max(0) as u16;
    let memory = if player == 0 {
        game.memory
    } else {
        -game.memory
    };
    let score = 4.0 * gain as f32 + 0.5
        - 2.0
        - 0.75 * cost as f32
        - 1.5 * (cost as i32 - memory as i32).max(0) as f32;
    Some((score, cost, gain))
}

fn best_dna_pair(
    game: &Game,
    player: PlayerId,
    hand: usize,
    first: Option<usize>,
    phase: GamePhase,
) -> Option<(f32, u16, u8)> {
    let mut best: Option<(f32, u16, u8)> = None;
    for a in 0..game.player(player).battle_area.len() {
        if first.is_some_and(|f| f != a) {
            continue;
        }
        for b in 0..game.player(player).battle_area.len() {
            if let Some(candidate) = dna_pair_score(game, player, hand, a, b, phase) {
                if best.is_none_or(|old| candidate.0 > old.0) {
                    best = Some(candidate);
                }
            }
        }
    }
    best
}

fn pending_resource_card(
    game: &Game,
    player: PlayerId,
    action: u16,
) -> Option<(&CardData, bool, Option<u16>)> {
    use crate::resume::{ResumeFrame, ResumeSelectKind};
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledStep};
    // Inspect the typed operation only, never captured bindings, future draws,
    // candidate pools or hidden values. Unrecognized continuations stay unknown.
    let frame = game.pending_selection_resume.as_ref()?.frames.last()?;
    let ResumeFrame::RunTail {
        prov,
        select_kind,
        bind_as: Some(binding),
        inner_tail,
        ..
    } = frame
    else {
        return None;
    };
    if prov.controller != player {
        return None;
    }
    let bound = |reference: &CompiledBindingRef| {
        matches!(reference,
        CompiledBindingRef::Named(name) | CompiledBindingRef::Binding(name) if name == binding)
    };
    let (card, discard) = match (select_kind, inner_tail.first()?) {
        (
            ResumeSelectKind::Hand { of_player },
            CompiledStep::TrashFromHandByIndex {
                of: CompiledPlayerRef::You,
                hand_index,
            },
        ) if *of_player == player && bound(hand_index) => {
            (game.player(player).hand.get(action as usize)?, true)
        }
        (
            ResumeSelectKind::Trash { of_player },
            CompiledStep::AddToHandFromTrash {
                of: CompiledPlayerRef::You,
                card,
            },
        ) if *of_player == player && bound(card) => (
            game.player(player)
                .trash
                .get(action.checked_sub(TRASH_EFFECT_START)? as usize)?,
            false,
        ),
        (
            _,
            CompiledStep::AddToHandFromReveal {
                of: CompiledPlayerRef::You,
                card,
            },
        ) if bound(card) => (
            game.revealed_cards
                .get(action.checked_sub(SEL_REVEAL_START)? as usize)?,
            false,
        ),
        _ => return None,
    };
    let draw_after = HandBenefit::from_steps(&inner_tail[1..])
        .literal_draw
        .map(|n| n.min(game.player(player).deck.len() as u16));
    Some((&game.card_data[card.data_index], discard, draw_after))
}
