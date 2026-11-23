use super::common::{action, builder, candidates, digimon, ids, start};
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, HAND_EFFECT_START, PASS, SECURITY_TARGET,
};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

#[test]
fn rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area() {
    for (rookie, color) in [
        ("BT27-044", CardColor::Green),
        ("BT27-056", CardColor::Black),
        ("BT27-073", CardColor::Purple),
    ] {
        for (target_color, beatbreak, expected) in [
            (color, true, 3),
            (color, false, 4),
            (CardColor::Yellow, true, 4),
        ] {
            let mut target = digimon("TEXT-EVO", 4, 5000, beatbreak);
            target.colors = vec![target_color];
            let mut r = start(builder(rookie).add_card(target).hand(0, &["TEXT-EVO"]));
            let base = r.place_on_field(0, rookie, Some(0));
            let memory = r.memory();
            action(&mut r, 0, encode_digivolve(0, base.index as u16));
            assert_eq!(memory - r.memory(), expected, "{rookie}");
        }
    }
}

#[test]
fn champions_offer_only_the_matching_tamer_on_digivolve_and_can_decline() {
    for (card, name) in [
        ("BT27-046", "Tomoro Tenma"),
        ("BT27-058", "Reina Sakuya"),
        ("BT27-077", "Makoto Kuonji"),
    ] {
        for tamer_count in 0..=2 {
            let mut matching = make_test_card("TEXT-MATCH", &format!("{name} & Partner"));
            matching.card_kind = CardKind::Tamer;
            matching.level = None;
            matching.dp = None;
            let false_match = make_test_card("TEXT-NOT-TAMER", name);
            let mut r = start(
                builder(card)
                    .add_card(matching)
                    .add_card(false_match)
                    .hand(0, &[card, "TEXT-MATCH", "TEXT-TAMER", "TEXT-NOT-TAMER"]),
            );
            let base = r.place_on_field(0, "TEXT-3", Some(0));
            for _ in 0..tamer_count {
                r.place_on_field(0, "TEXT-TAMER", Some(0));
            }
            action(&mut r, 0, encode_digivolve(0, base.index as u16));
            if tamer_count == 2 {
                assert!(r.pending_selection().is_none(), "{card}");
            } else {
                r.assert_prompt(SelectionKind::Hand, true, 0);
                assert_eq!(candidates(&r), ["TEXT-MATCH"]);
                if tamer_count == 0 {
                    let action_id = r.prompt_card_candidates()[0].0;
                    action(&mut r, 0, action_id);
                    assert_eq!(r.game.players[0].battle_area.len(), 2);
                } else {
                    action(&mut r, 0, PASS);
                }
            }
        }
    }
}

#[test]
fn tomoro_stashes_the_decks_top_card_when_a_digimon_suspends() {
    let mut r = start(builder("BT27-091"));
    let tamer = r.place_on_field(0, "BT27-091", Some(0));
    let attacker = r.place_on_field(0, "TEXT-3", Some(0));
    let deck = r.game.players[0].deck.len();
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(0).unwrap();
    let permanent = &r.game.players[0].battle_area[tamer.index as usize];
    assert!(permanent.is_suspended);
    assert_eq!(
        ids(&r, &permanent.card_sources),
        ["TEXT-FILLER", "BT27-091"]
    );
    assert!(permanent.card_sources[0].face_down);
    assert_eq!(r.game.players[0].deck.len(), deck - 1);
}

#[test]
fn hand_main_stacks_the_named_material_and_ignores_the_rookies_level() {
    for (card, tamer, rookie, material) in [
        ("BT27-050", "BT27-091", "BT27-044", "BT27-046"),
        ("BT27-064", "BT27-094", "BT27-056", "BT27-058"),
        ("BT27-080", "BT27-097", "BT27-073", "BT27-077"),
    ] {
        let mut r = start(
            builder(card)
                .dsl_card(tamer)
                .unwrap()
                .dsl_card(rookie)
                .unwrap()
                .dsl_card(material)
                .unwrap()
                .hand(0, &[card]),
        );
        let base = r.place_on_field(0, rookie, Some(0));
        r.place_on_field(0, tamer, Some(0));
        r.inject_trash(0, material);
        let memory = r.memory();
        action(&mut r, 0, HAND_EFFECT_START);
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        assert_eq!(candidates(&r), [rookie]);
        let host = r.prompt_card_candidates()[0].0;
        action(&mut r, 0, host);
        r.assert_prompt(SelectionKind::Trash, false, 0);
        assert_eq!(candidates(&r), [material]);
        let material_action = r.prompt_card_candidates()[0].0;
        action(&mut r, 0, material_action);
        let permanent = &r.game.players[0].battle_area[base.index as usize];
        assert_eq!(ids(&r, &permanent.card_sources), [material, rookie, card]);
        assert_eq!(memory - r.memory(), 2, "{card}");
    }
}

#[test]
fn atratusmon_can_battle_an_unsuspended_digimon_without_attacking() {
    let mut r = start(builder("BT27-052").memory(20).hand(0, &["BT27-052"]));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, 0);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    let chosen = r.prompt_card_candidates()[0].0;
    action(&mut r, 0, chosen);
    assert!(r.game.players[1].battle_area.is_empty());
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.security_count(1), 5);
}

#[test]
fn artiomon_option_main_de_digivolves_then_unsuspends() {
    let mut r = start(builder("BT27-067").hand(0, &["BT27-067"]));
    let ally = r.place_on_field(0, "TEXT-3", Some(0));
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    let foe = r.place_stack(1, &["TEXT-3", "TEXT-4", "TEXT-5"]);
    action(&mut r, 0, 0);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    let enemy = r.prompt_card_candidates()[0].0;
    action(&mut r, 0, enemy);
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    let target = r.prompt_card_candidates()[0].0;
    action(&mut r, 0, target);
    assert_eq!(
        ids(
            &r,
            &r.game.players[1].battle_area[foe.index as usize].card_sources
        ),
        ["TEXT-3"]
    );
    assert!(!r.game.players[0].battle_area[ally.index as usize].is_suspended);
}

#[test]
fn nosferamon_security_deletes_the_lowest_level_digimon() {
    let mut r = start(builder("BT27-082").security(1, &["BT27-082"]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    r.place_on_field(0, "TEXT-3", Some(0));
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OppField, false, 1);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    let target = r.prompt_card_candidates()[0].0;
    action(&mut r, 1, target);
    assert!(ids(&r, &r.game.players[0].trash).contains(&"TEXT-3".to_owned()));
    assert_eq!(r.security_count(0), 5);
}

#[test]
fn reina_and_makoto_can_stash_two_or_suspend_without_stashing() {
    for card in ["BT27-094", "BT27-097"] {
        for branch in 0..3 {
            let mut r = start(builder(card));
            let tamer = r.place_on_field(0, card, Some(0));
            let attacker = r.place_on_field(0, "TEXT-3", Some(0));
            let deck = r.game.players[0].deck.len();
            action(
                &mut r,
                0,
                encode_attack(attacker.index as u16, SECURITY_TARGET),
            );
            r.assert_prompt(SelectionKind::EffectChoice, false, 0);
            r.execute_branch(branch).unwrap();
            let permanent = &r.game.players[0].battle_area[tamer.index as usize];
            assert_eq!(permanent.is_suspended, branch != 2);
            let stashed = if branch == 0 { 2 } else { 0 };
            assert_eq!(permanent.face_down_source_indices().count(), stashed);
            assert_eq!(r.game.players[0].deck.len(), deck - stashed);
        }
    }
}
