use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

fn digimon(id: &str, traits: &[&str]) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Yellow],
        traits: traits.iter().map(|s| s.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn overclock_digimon(id: &str, trait_name: &str) -> CardData {
    let mut card = digimon(id, &[trait_name]);
    card.effect_text = format!(
        "＜Overclock ([{trait_name}] Trait)＞ (At the end of your turn, by deleting 1 of your Tokens or other [{trait_name}] trait Digimon, this Digimon attacks a player without suspending.)"
    );
    card.keywords = vec![Keyword::Overclock];
    card
}

fn token(id: &str) -> CardData {
    let mut card = digimon(id, &[]);
    card.card_kind = CardKind::Token;
    card.level = None;
    card
}

fn encode_field_effect(field_index: u16, effect_slot: u16) -> u16 {
    FIELD_EFFECT_START + field_index * EFFECTS_PER_PERMANENT + effect_slot
}

fn setup_puppet_overclock() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(overclock_digimon("PUPPET-OVERCLOCK", "Puppet"))
        .add_card(token("PUPPET-TOKEN"))
        .add_card(digimon("BT5-008", &["Reptile"]))
        .add_card(digimon("PUPPET-ALLY", &["Puppet"]))
        .start();

    r.place_on_field(0, "PUPPET-OVERCLOCK", Some(0));
    r.place_on_field(0, "PUPPET-TOKEN", Some(0));
    r.place_on_field(0, "BT5-008", Some(0));
    r.place_on_field(0, "PUPPET-ALLY", Some(0));
    r.game.current_phase = GamePhase::EndOfTurnAction;
    r
}

#[test]
fn overclock_only_exposes_token_or_matching_other_digimon_as_cost() {
    let mut r = setup_puppet_overclock();

    let effect_bit = encode_field_effect(0, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[effect_bit as usize], 1.0,
        "Overclock activation should be visible"
    );

    r.game.decode_action(effect_bit, 0);
    let cost_mask = build_action_mask(&r.game, 0);
    assert_eq!(
        cost_mask[encode_attack(0, 1) as usize],
        1.0,
        "token is legal cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 2) as usize],
        0.0,
        "unmatched non-token is not legal cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 3) as usize],
        1.0,
        "matching trait Digimon is legal cost"
    );
    assert_eq!(cost_mask[PASS as usize], 1.0, "Overclock is optional");
}

#[test]
fn overclock_decode_rejects_non_candidate_cost_target() {
    let mut r = setup_puppet_overclock();

    r.game
        .decode_action(encode_field_effect(0, FIELD_EFFECT_SLOT_FOR_OVERCLOCK), 0);
    r.game.decode_action(encode_attack(0, 2), 0);

    assert!(
        r.game.pending_selection.is_some(),
        "invalid cost target must leave the pending choice unresolved"
    );
    assert_eq!(
        r.game.player(0).battle_area.len(),
        4,
        "invalid cost target must not delete a permanent"
    );
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn overclock_attack_survives_cost_deleting_lower_slot() {
    let mut r = DebugRunner::builder()
        .add_card(overclock_digimon("PUPPET-OVERCLOCK", "Puppet"))
        .add_card(digimon("PUPPET-ALLY", &["Puppet"]))
        .start();

    r.place_on_field(0, "PUPPET-ALLY", Some(0));
    r.place_on_field(0, "PUPPET-OVERCLOCK", Some(0));
    r.game.current_phase = GamePhase::EndOfTurnAction;

    r.game
        .decode_action(encode_field_effect(1, FIELD_EFFECT_SLOT_FOR_OVERCLOCK), 0);
    r.game.decode_action(encode_attack(0, 0), 0);

    assert_eq!(
        r.game.player(0).battle_area.len(),
        1,
        "selected lower-index cost should be deleted"
    );
    assert!(
        r.game.game_over,
        "Overclock source should be re-resolved after the cost shifts its slot and attack directly"
    );
    assert_eq!(r.game.winner, Some(0));
}

#[test]
fn overclock_decline_does_not_delete_or_attack() {
    let mut r = DebugRunner::builder()
        .add_card(overclock_digimon("PUPPET-OVERCLOCK", "Puppet"))
        .add_card(token("PUPPET-TOKEN"))
        .start();

    r.place_on_field(0, "PUPPET-OVERCLOCK", Some(0));
    r.place_on_field(0, "PUPPET-TOKEN", Some(0));
    r.game.current_phase = GamePhase::EndOfTurnAction;
    r.game
        .decode_action(encode_field_effect(0, FIELD_EFFECT_SLOT_FOR_OVERCLOCK), 0);
    r.game.decode_action(PASS, 0);

    assert_eq!(r.game.player(0).battle_area.len(), 2);
    assert!(r.game.pending_attack.is_none());
}
