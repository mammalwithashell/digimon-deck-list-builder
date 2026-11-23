use super::common::{action, ids, start};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, BREEDING_TARGET, PLAY_HAND_START, SECURITY_TARGET,
    TRASH_EFFECT_START,
};
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

pub(super) fn fixture(cards: &[&str]) -> DebugRunnerBuilder {
    let mut red_tamer = make_test_card("RED-TAMER", "Red Tamer");
    red_tamer.card_kind = CardKind::Tamer;
    red_tamer.colors = vec![CardColor::Red];
    let mut blue_tamer = red_tamer.clone();
    blue_tamer.card_id = "BLUE-TAMER".into();
    blue_tamer.colors = vec![CardColor::Blue];
    let mut alysion = make_test_card("ALYSION", "ALYSION Digimon");
    alysion.traits = vec!["ALYSION".into()];
    alysion.dp = Some(5000);
    let mut ice = make_test_card("ICE", "Ice-Snow Digimon");
    ice.traits = vec!["Ice-Snow".into()];
    ice.dp = Some(5000);
    let mut builder = DebugRunner::builder()
        .add_card(red_tamer)
        .add_card(blue_tamer)
        .add_card(alysion)
        .add_card(ice)
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(10);
    for card in cards {
        builder = builder.dsl_card(card).unwrap();
    }
    builder
}

pub(super) fn assemble(builder: DebugRunnerBuilder, card: &str, materials: [&str; 3]) {
    let mut r = start(builder.hand(0, &[card]));
    for material in materials {
        r.inject_trash(0, material);
    }
    action(&mut r, 0, PLAY_HAND_START);
    for material in materials {
        let trash = ids(&r, &r.game.players[0].trash);
        let index = trash.iter().position(|id| id == material).unwrap();
        action(&mut r, 0, TRASH_EFFECT_START + index as u16);
    }
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 4);
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.memory(), 3);
}

pub(super) fn gemmon_text_digivolution(card: &str, level: u8, cost: i16) {
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
        assert_eq!(
            build_action_mask(&r.game, 0)[evolve as usize] > 0.5,
            matches
        );
        if !matches {
            continue;
        }
        action(&mut r, 0, evolve);
        if matches!(card, "BT27-011" | "BT27-014") {
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            r.choose_prompt_card(card, None).unwrap();
        }
        assert!(r.pending_selection().is_none());
        let player = &r.game.players[0];
        let evolved = if level == 2 {
            player.breeding_area.as_ref().unwrap()
        } else {
            &player.battle_area[target as usize]
        };
        assert_eq!(ids(&r, &evolved.card_sources), ["BLUE-BASE", card]);
        assert_eq!(r.memory(), 10 - cost);
    }
}

pub(super) fn play_from_security(card: &str) {
    let mut r = start(fixture(&[card]).security(1, &[card]));
    let attacker = r.place_stack(0, &["ICE"]);
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert_eq!(
        ids(&r, &r.game.players[1].battle_area[0].card_sources),
        [card]
    );
    assert_eq!(r.memory(), 10);
    assert_eq!(r.security_count(1), 0);
}
