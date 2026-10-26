//! Private evaluation diagnostics. Never serialized into a policy observation,
//! action mask, public UI state, or reward event. Queries cannot mutate Game.
use crate::{
    card_source::CardHandle, enums::PlayerId, game::Game, permanent::PermanentHandle,
    resume::ResumeFrame,
};
use serde_json::{json, Value};

/// Check Nyaromon's printed VB restriction using the same pair legality as the
/// effect's actual partner picker. A trigger-order row alone is not evidence
/// that its optional DNA process can complete.
fn nyaromon_has_target(game: &Game, player: PlayerId, anchor: Option<PermanentHandle>) -> bool {
    let Some(anchor) = anchor else {
        return false;
    };
    let Some(perm) = game
        .player(anchor.player)
        .battle_area
        .get(anchor.index as usize)
    else {
        return false;
    };
    if !perm.has_trait_for_rules("VB", &game.card_data, &game.modifiers, anchor) {
        return false;
    }
    (0..game.player(player).battle_area.len()).any(|i| {
        let partner = PermanentHandle {
            player,
            index: i as u8,
        };
        partner != anchor
            && game
                .player(player)
                .hand
                .iter()
                .enumerate()
                .any(|(j, card)| {
                    game.card_data[card.data_index]
                        .traits
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case("VB"))
                        && crate::effect_context::dna_pair_can_reach_hand_card(
                            game, player, anchor, partner, j,
                        )
                })
    })
}

fn descriptor(
    game: &Game,
    card: CardHandle,
    player: PlayerId,
    stage: &str,
    actions: &[u16],
) -> Value {
    json!({"card_id": "EX12-001", "source_handle": card.0, "player": player+1,
           "sequence": format!("{}:{}", game.turn_count, card.0),
           "stage": stage, "accept_actions": actions})
}

/// Schema 1 covers EX12-001's sequence and distinguishes physical eggs. One
/// inherited end-of-turn activation per egg/global turn is the counting unit.
pub fn decision_context(game: &Game) -> Value {
    let mut effects = Vec::new();
    if let (Some(pending), Some(frame)) = (
        game.pending_selection.as_ref(),
        game.pending_selection_resume
            .as_ref()
            .and_then(|s| s.frames.last()),
    ) {
        if let ResumeFrame::TriggerOrderSelection(state) = frame {
            for (offset, index) in state.bundle.iter().enumerate() {
                let Some(qe) = game.effect_queue.get(*index) else {
                    continue;
                };
                if qe.card_id == "EX12-001"
                    && nyaromon_has_target(game, qe.controller, qe.source_permanent)
                {
                    effects.push(descriptor(
                        game,
                        qe.source_card,
                        qe.controller,
                        "trigger_order",
                        &[crate::action::space::HAND_EFFECT_START + offset as u16],
                    ));
                }
            }
        } else if game
            .card_data_for_handle(pending.source_card)
            .is_some_and(|c| c.card_id == "EX12-001")
        {
            match frame {
                ResumeFrame::MayDnaPartnerSelection(state) => {
                    if !pending.valid_action_ids.is_empty() {
                        effects.push(descriptor(
                            game,
                            state.prov.source_card,
                            state.controller,
                            "dna_partner",
                            &pending.valid_action_ids,
                        ));
                    }
                }
                ResumeFrame::MayDnaResultSelection(state) => {
                    if !state.valid_action_ids.is_empty() {
                        let mut item = descriptor(
                            game,
                            state.prov.source_card,
                            state.controller,
                            "dna_result",
                            &state.valid_action_ids,
                        );
                        let targets: serde_json::Map<String, Value> = state
                            .valid_action_ids
                            .iter()
                            .filter_map(|&action| {
                                let index = action
                                    .checked_sub(crate::action::space::PLAY_HAND_START)?
                                    as usize;
                                let card = game.player(state.controller).hand.get(index)?;
                                Some((action.to_string(), json!(card.card_id(&game.card_data))))
                            })
                            .collect();
                        item["result_cards"] = Value::Object(targets);
                        effects.push(item);
                    }
                }
                ResumeFrame::AttackDeclarationDecision(state) => {
                    if !state.valid_action_ids.is_empty() {
                        let mut item = descriptor(
                            game,
                            state.source_card,
                            state.attacker.player,
                            "followup_attack",
                            &state.valid_action_ids,
                        );
                        item["attacker_slot"] = json!(state.attacker.index);
                        effects.push(item);
                    }
                }
                ResumeFrame::RunTail {
                    prov,
                    select_kind: crate::resume::ResumeSelectKind::BeginAttack { attacker, .. },
                    ..
                } => {
                    if !pending.valid_action_ids.is_empty() {
                        let mut item = descriptor(
                            game,
                            prov.source_card,
                            prov.controller,
                            "followup_attack",
                            &pending.valid_action_ids,
                        );
                        item["attacker_slot"] = json!(attacker.index);
                        effects.push(item);
                    }
                }
                _ => {}
            }
        }
    }
    json!({"schema_version": 1, "effects": effects})
}
