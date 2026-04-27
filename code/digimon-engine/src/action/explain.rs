use serde::{Deserialize, Serialize};

use crate::action::space::{
    decode_attack, decode_digivolve, decode_field_effect, decode_source_select, ACTION_SPACE_SIZE,
    ATTACK_END, ATTACK_START, BREEDING_TARGET, DIGIVOLVE_END, DIGIVOLVE_START, DNA_DIGIVOLVE_END,
    DNA_DIGIVOLVE_START, FIELD_EFFECT_END, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    HAND_EFFECT_END, HAND_EFFECT_START, HATCH, MOVE_FROM_BREEDING, PASS, PLAY_HAND_END,
    PLAY_HAND_START, SECURITY_TARGET, SOURCE_SELECT_END, SOURCE_SELECT_START, TRASH_EFFECT_END,
    TRASH_EFFECT_START,
};
use crate::card_source::CardSource;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Play,
    HandEffect,
    Hatch,
    Move,
    Pass,
    DnaDigivolve,
    Attack,
    Digivolve,
    FieldEffect,
    TrashEffect,
    SourceSelect,
    Selection,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionZone {
    Hand,
    Battle,
    Breeding,
    Security,
    Trash,
    Source,
    Revealed,
    EffectChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionExplanation {
    pub action_id: u16,
    pub player_id: PlayerId,
    pub phase: String,
    pub kind: ActionKind,
    pub label: String,
    pub source_zone: Option<ActionZone>,
    pub source_index: Option<u16>,
    pub target_zone: Option<ActionZone>,
    pub target_index: Option<u16>,
    pub card_id: Option<String>,
    pub card_name: Option<String>,
}

pub fn explain_action(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if action_id as usize >= ACTION_SPACE_SIZE {
        return base(
            game,
            player_id,
            action_id,
            ActionKind::Unknown,
            format!("Unknown action {action_id}"),
        );
    }

    match game.current_phase {
        GamePhase::Mulligan => explain_mulligan(game, player_id, action_id),
        GamePhase::Breeding => explain_breeding(game, player_id, action_id),
        GamePhase::Main => explain_main(game, player_id, action_id),
        GamePhase::EndOfTurnAction => explain_end_of_turn(game, player_id, action_id),
        GamePhase::SelectTarget
        | GamePhase::SelectMaterial
        | GamePhase::SelectTrash
        | GamePhase::SelectSource
        | GamePhase::SelectHand
        | GamePhase::SelectReveal
        | GamePhase::SelectSecurity
        | GamePhase::EffectChoice
        | GamePhase::BlockTiming
        | GamePhase::CounterTiming
        | GamePhase::AllianceTiming
        | GamePhase::SelectUnion
        | GamePhase::SelectPermutation
        | GamePhase::SelectBudgeted => explain_selection(game, player_id, action_id),
        GamePhase::Unsuspend | GamePhase::Draw | GamePhase::EndTurn | GamePhase::GameOver => base(
            game,
            player_id,
            action_id,
            ActionKind::Unknown,
            format!("No action in {:?}", game.current_phase),
        ),
    }
}

fn base(
    game: &Game,
    player_id: PlayerId,
    action_id: u16,
    kind: ActionKind,
    label: String,
) -> ActionExplanation {
    ActionExplanation {
        action_id,
        player_id,
        phase: format!("{:?}", game.current_phase),
        kind,
        label,
        source_zone: None,
        source_index: None,
        target_zone: None,
        target_index: None,
        card_id: None,
        card_name: None,
    }
}

fn apply_card_context(
    mut explanation: ActionExplanation,
    game: &Game,
    card: &CardSource,
) -> ActionExplanation {
    explanation.card_id = Some(card.card_id(&game.card_data).to_string());
    explanation.card_name = Some(card.card_name(&game.card_data).to_string());
    explanation
}

fn with_hand_card(
    explanation: ActionExplanation,
    game: &Game,
    player_id: PlayerId,
    hand_idx: usize,
) -> ActionExplanation {
    if let Some(card) = game.player(player_id).hand.get(hand_idx) {
        apply_card_context(explanation, game, card)
    } else {
        explanation
    }
}

fn explain_mulligan(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    match action_id {
        0 => base(
            game,
            player_id,
            action_id,
            ActionKind::Selection,
            "Keep opening hand".to_string(),
        ),
        1 => base(
            game,
            player_id,
            action_id,
            ActionKind::Selection,
            "Mulligan opening hand".to_string(),
        ),
        _ => base(
            game,
            player_id,
            action_id,
            ActionKind::Unknown,
            format!("Unknown mulligan action {action_id}"),
        ),
    }
}

fn explain_breeding(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    match action_id {
        HATCH => base(
            game,
            player_id,
            action_id,
            ActionKind::Hatch,
            "Hatch from egg deck".to_string(),
        ),
        MOVE_FROM_BREEDING => {
            let mut e = base(
                game,
                player_id,
                action_id,
                ActionKind::Move,
                "Move from breeding area".to_string(),
            );
            e.source_zone = Some(ActionZone::Breeding);
            e.target_zone = Some(ActionZone::Battle);
            e
        }
        PASS => base(
            game,
            player_id,
            action_id,
            ActionKind::Pass,
            "Pass / decline".to_string(),
        ),
        _ => base(
            game,
            player_id,
            action_id,
            ActionKind::Unknown,
            format!("Unknown breeding action {action_id}"),
        ),
    }
}

fn explain_main(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
        let hand_idx = action_id as usize;
        let mut e = base(
            game,
            player_id,
            action_id,
            ActionKind::Play,
            hand_card_label(game, player_id, hand_idx, "Play", "hand card"),
        );
        e.source_zone = Some(ActionZone::Hand);
        e.source_index = Some(hand_idx as u16);
        return with_hand_card(e, game, player_id, hand_idx);
    }

    if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&action_id) {
        let hand_idx = (action_id - HAND_EFFECT_START) as usize;
        let mut e = base(
            game,
            player_id,
            action_id,
            ActionKind::HandEffect,
            hand_card_label(game, player_id, hand_idx, "Activate", "hand effect"),
        );
        e.source_zone = Some(ActionZone::Hand);
        e.source_index = Some(hand_idx as u16);
        return with_hand_card(e, game, player_id, hand_idx);
    }

    if action_id == PASS {
        return base(
            game,
            player_id,
            action_id,
            ActionKind::Pass,
            "Pass / decline".to_string(),
        );
    }

    if (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&action_id) {
        let hand_idx = action_id - DNA_DIGIVOLVE_START;
        let mut e = base(
            game,
            player_id,
            action_id,
            ActionKind::DnaDigivolve,
            hand_card_label(
                game,
                player_id,
                hand_idx as usize,
                "DNA digivolve with",
                "hand card",
            ),
        );
        e.source_zone = Some(ActionZone::Hand);
        e.source_index = Some(hand_idx);
        return with_hand_card(e, game, player_id, hand_idx as usize);
    }

    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        return explain_attack(game, player_id, action_id);
    }

    if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
        return explain_digivolve(game, player_id, action_id);
    }

    if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        let (perm, effect) = decode_field_effect(action_id);
        let e = base(
            game,
            player_id,
            action_id,
            ActionKind::FieldEffect,
            field_effect_label("Activate field effect", perm, effect),
        );
        return apply_field_effect_context(e, game, player_id, perm, effect);
    }

    if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action_id) {
        let trash_idx = action_id - TRASH_EFFECT_START;
        let mut e = base(
            game,
            player_id,
            action_id,
            ActionKind::TrashEffect,
            format!("Activate trash effect {trash_idx}"),
        );
        e.source_zone = Some(ActionZone::Trash);
        e.source_index = Some(trash_idx);
        return with_trash_card(e, game, player_id, trash_idx);
    }

    base(
        game,
        player_id,
        action_id,
        ActionKind::Unknown,
        format!("Unknown action {action_id}"),
    )
}

fn explain_attack(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    let (attacker, target) = decode_attack(action_id);
    let mut e = base(
        game,
        player_id,
        action_id,
        ActionKind::Attack,
        String::new(),
    );
    e.source_zone = Some(ActionZone::Battle);
    e.source_index = Some(attacker);

    if target == SECURITY_TARGET {
        e.target_zone = Some(ActionZone::Security);
        e.label = format!("Slot {attacker} attacks security");
    } else {
        e.target_zone = Some(ActionZone::Battle);
        e.target_index = Some(target);
        e.label = format!("Slot {attacker} attacks opponent slot {target}");
    }

    with_battle_card(e, game, player_id, attacker)
}

fn explain_digivolve(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    let (hand, field) = decode_digivolve(action_id);
    let mut e = base(
        game,
        player_id,
        action_id,
        ActionKind::Digivolve,
        String::new(),
    );
    e.source_zone = Some(ActionZone::Hand);
    e.source_index = Some(hand);

    if field == BREEDING_TARGET {
        e.target_zone = Some(ActionZone::Breeding);
        e.label = format!("Digivolve hand {hand} onto breeding area");
    } else {
        e.target_zone = Some(ActionZone::Battle);
        e.target_index = Some(field);
        e.label = format!("Digivolve hand {hand} onto slot {field}");
    }

    with_hand_card(e, game, player_id, hand as usize)
}

fn explain_end_of_turn(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if action_id == PASS {
        return base(
            game,
            player_id,
            action_id,
            ActionKind::Pass,
            "Pass / decline".to_string(),
        );
    }

    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        return explain_attack(game, player_id, action_id);
    }

    if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        let (perm, effect) = decode_field_effect(action_id);
        let e = base(
            game,
            player_id,
            action_id,
            ActionKind::FieldEffect,
            field_effect_label("Resolve end-of-turn effect", perm, effect),
        );
        return apply_field_effect_context(e, game, player_id, perm, effect);
    }

    base(
        game,
        player_id,
        action_id,
        ActionKind::Unknown,
        format!("Unknown end-of-turn action {action_id}"),
    )
}

fn explain_selection(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) {
        let (field, source) = decode_source_select(action_id);
        let mut e = base(
            game,
            player_id,
            action_id,
            ActionKind::SourceSelect,
            format!("Select source {source} on slot {field}"),
        );
        e.source_zone = Some(ActionZone::Source);
        e.source_index = Some(source);
        e.target_zone = Some(ActionZone::Battle);
        e.target_index = Some(field);
        return e;
    }

    if action_id == PASS {
        return base(
            game,
            player_id,
            action_id,
            ActionKind::Pass,
            "Pass / decline".to_string(),
        );
    }

    let mut e = base(
        game,
        player_id,
        action_id,
        ActionKind::Selection,
        format!("Select option {action_id}"),
    );

    if let Some(sel) = game.pending_selection.as_ref() {
        e.label = sel
            .effect_choices
            .as_ref()
            .and_then(|choices| choices.iter().find(|choice| choice.action_id == action_id))
            .map(|choice| choice.label.clone())
            .unwrap_or_else(|| format!("{}: select {action_id}", sel.prompt));
        if game.current_phase == GamePhase::EffectChoice {
            e.source_zone = Some(ActionZone::EffectChoice);
        }
    }

    e
}

fn hand_card_label(
    game: &Game,
    player_id: PlayerId,
    hand_idx: usize,
    verb: &str,
    fallback: &str,
) -> String {
    game.player(player_id)
        .hand
        .get(hand_idx)
        .map(|card| {
            format!(
                "{verb} {} from hand {hand_idx}",
                card.card_name(&game.card_data)
            )
        })
        .unwrap_or_else(|| format!("{verb} {fallback} {hand_idx}"))
}

fn with_battle_card(
    explanation: ActionExplanation,
    game: &Game,
    player_id: PlayerId,
    battle_idx: u16,
) -> ActionExplanation {
    if let Some(perm) = game.player(player_id).battle_area.get(battle_idx as usize) {
        apply_card_context(explanation, game, perm.top_card())
    } else {
        explanation
    }
}

fn with_breeding_card(
    explanation: ActionExplanation,
    game: &Game,
    player_id: PlayerId,
) -> ActionExplanation {
    if let Some(perm) = game.player(player_id).breeding_area.as_ref() {
        apply_card_context(explanation, game, perm.top_card())
    } else {
        explanation
    }
}

fn field_effect_label(prefix: &str, perm: u16, effect: u16) -> String {
    if perm == BREEDING_TARGET && effect == FIELD_EFFECT_SLOT_FOR_MAIN {
        format!("{prefix} {effect} in breeding area")
    } else {
        format!("{prefix} {effect} on slot {perm}")
    }
}

fn apply_field_effect_context(
    mut explanation: ActionExplanation,
    game: &Game,
    player_id: PlayerId,
    perm: u16,
    effect: u16,
) -> ActionExplanation {
    if perm == BREEDING_TARGET && effect == FIELD_EFFECT_SLOT_FOR_MAIN {
        explanation.source_zone = Some(ActionZone::Breeding);
        explanation.source_index = None;
        with_breeding_card(explanation, game, player_id)
    } else {
        explanation.source_zone = Some(ActionZone::Battle);
        explanation.source_index = Some(perm);
        with_battle_card(explanation, game, player_id, perm)
    }
}

fn with_trash_card(
    explanation: ActionExplanation,
    game: &Game,
    player_id: PlayerId,
    trash_idx: u16,
) -> ActionExplanation {
    if let Some(card) = game.player(player_id).trash.get(trash_idx as usize) {
        apply_card_context(explanation, game, card)
    } else {
        explanation
    }
}
