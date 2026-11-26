//! Shared fixtures and helpers for ALYSION behavioral tests (ST25, ST26, ED01, SD01, SD02, BD01, BD02).
#![allow(dead_code)]

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, BREEDING_TARGET, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

pub fn test_digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card_with_level(id, name, level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c.traits = traits.iter().map(|s| (*s).to_string()).collect();
    c
}

pub fn test_tamer(id: &str, name: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = traits.iter().map(|s| (*s).to_string()).collect();
    c
}

pub fn fixture(cards: &[&str]) -> DebugRunnerBuilder {
    let mut builder = DebugRunner::builder()
        .add_card(test_digimon("RED-ALYSION", "Red Alysion", CardColor::Red, 4, 5000, &["ALYSION"]))
        .add_card(test_digimon("BLUE-ALYSION", "Blue Alysion", CardColor::Blue, 4, 5000, &["ALYSION"]))
        .add_card(test_digimon("ICE-MON", "Ice Mon", CardColor::Blue, 4, 5000, &["Ice-Snow"]))
        .add_card(test_digimon("PLAIN-2K", "Plain 2k", CardColor::Green, 3, 2000, &[]))
        .add_card(test_digimon("PLAIN-3K", "Plain 3k", CardColor::Green, 3, 3000, &[]))
        .add_card(test_digimon("PLAIN-4K", "Plain 4k", CardColor::Green, 4, 4000, &[]))
        .add_card(test_digimon("PLAIN-5K", "Plain 5k", CardColor::Green, 4, 5000, &[]))
        .add_card(test_digimon("PLAIN-6K", "Plain 6k", CardColor::Green, 5, 6000, &[]))
        .add_card(test_digimon("PLAIN-7K", "Plain 7k", CardColor::Green, 5, 7000, &[]))
        .add_card(test_digimon("PLAIN-8K", "Plain 8k", CardColor::Green, 5, 8000, &[]))
        .add_card(test_digimon("PLAIN-12K", "Plain 12k", CardColor::Green, 6, 12000, &[]))
        .add_card(test_digimon("RED-LV3", "Red Lv3", CardColor::Red, 3, 2000, &[]))
        .add_card(test_digimon("RED-LV4", "Red Lv4", CardColor::Red, 4, 5000, &[]))
        .add_card(test_digimon("RED-LV5", "Red Lv5", CardColor::Red, 5, 7000, &[]))
        .add_card(test_digimon("BLUE-LV3", "Blue Lv3", CardColor::Blue, 3, 2000, &[]))
        .add_card(test_digimon("BLUE-LV4", "Blue Lv4", CardColor::Blue, 4, 5000, &[]))
        .add_card(test_digimon("BLUE-LV5", "Blue Lv5", CardColor::Blue, 5, 7000, &[]))
        .add_card(test_tamer("RED-TAMER", "Red Tamer", CardColor::Red, &[]))
        .add_card(test_tamer("BLUE-TAMER", "Blue Tamer", CardColor::Blue, &[]))
        .add_card(test_tamer("KANATA", "Kanata Hondo", CardColor::Red, &["ALYSION"]))
        .add_card(test_tamer("VALNER", "Valner Dragnogh", CardColor::Blue, &["ALYSION"]))
        .add_card(test_digimon("GEMMON-RED", "Gemmon", CardColor::Red, 3, 1000, &["ALYSION"]))
        .add_card(test_digimon("RENAMON-BLUE", "Renamon", CardColor::Blue, 3, 1000, &["ALYSION"]))
        .add_card(test_digimon("RENAMON-YEL", "Renamon", CardColor::Yellow, 3, 1000, &[]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 15])
        .deck(1, &["FILLER"; 15])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(10);
    for card in cards {
        builder = builder.dsl_card(card).unwrap();
    }
    builder
}

pub fn start(builder: DebugRunnerBuilder) -> DebugRunner {
    let mut r = builder.start();
    r.game.enter_main_phase();
    r
}

pub fn action(r: &mut DebugRunner, player: u8, act: u16) {
    assert!(
        build_action_mask(&r.game, player)[act as usize] > 0.5,
        "illegal action {act}, phase {:?}, prompt {:?}",
        r.current_phase(),
        r.pending_selection_view()
    );
    r.game.decode_action(act, player);
}

pub fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn candidates(r: &DebugRunner) -> Vec<String> {
    let mut cards: Vec<_> = r
        .prompt_card_candidates()
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    cards.sort();
    cards
}

pub fn advance_turn(r: &mut DebugRunner) {
    let p = r.game.turn_player();
    action(r, p, PASS);
    if r.current_phase() == GamePhase::Breeding {
        action(r, 1 - p, PASS);
    }
}

pub fn delete_perm(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.delete_permanents_batch(
        vec![h],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    while let Some((player, act)) = r.pending_selection().map(|p| {
        (
            p.selecting_player,
            p.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(PASS),
        )
    }) {
        action(r, player, act);
    }
}

pub fn assert_memory_setter(card: &str) {
    for (pass_val, expected) in [(1, 3), (2, 3), (4, 4)] {
        let mut r = start(fixture(&[card]));
        r.place_on_field(1, card, Some(0));
        r.game.set_memory(-pass_val);
        r.game.end_turn();
        assert_eq!(r.game.turn_player(), 1);
        assert_eq!(r.memory(), expected);
    }
}

pub fn assert_security_play(card: &str) {
    let mut r = start(fixture(&[card]).security(1, &[card]));
    let attacker = r.place_stack(0, &["PLAIN-12K"]);
    action(&mut r, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    assert_eq!(ids(&r, &r.game.players[1].battle_area[0].card_sources), [card]);
    assert_eq!(r.security_count(1), 0);
}

pub fn assert_gemmon_text_digivolve(card: &str, level: u8, cost: i16) {
    for matches in [false, true] {
        let mut base = make_test_card_with_level("BLUE-BASE", "Blue Base", level);
        base.colors = vec![CardColor::Blue];
        if level == 2 {
            base.card_kind = CardKind::DigiEgg;
            base.dp = None;
        }
        if matches {
            base.requirement_text = "[Digivolve] w/[Gemmon] in text".into();
        }
        let mut r = start(fixture(&[card]).add_card(base).hand(0, &[card]));
        let target = if level == 2 {
            r.place_in_breeding(0, "BLUE-BASE");
            BREEDING_TARGET
        } else {
            u16::from(r.place_stack(0, &["BLUE-BASE"]).index)
        };
        let evolve = encode_digivolve(0, target);
        assert_eq!(build_action_mask(&r.game, 0)[evolve as usize] > 0.5, matches);
        if matches {
            action(&mut r, 0, evolve);
            while let Some((player, act)) = r.pending_selection().map(|p| {
                (
                    p.selecting_player,
                    p.valid_action_ids
                        .iter()
                        .copied()
                        .find(|&a| a != PASS)
                        .unwrap_or(PASS),
                )
            }) {
                action(&mut r, player, act);
            }
            assert_eq!(r.memory(), 10 - cost);
        }
    }
}
