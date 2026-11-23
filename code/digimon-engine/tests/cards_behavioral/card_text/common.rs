use digimon_engine::action::space::{
    encode_attack, encode_digivolve, HAND_EFFECT_START, PASS, SECURITY_TARGET,
};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

pub fn digimon(id: &str, level: u8, dp: i32, glowing: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![
        CardColor::Green,
        CardColor::Yellow,
        CardColor::Black,
        CardColor::Purple,
    ];
    if glowing {
        c.traits = vec!["Glowing Dawn".into(), "BEATBREAK".into()];
    }
    c.evo_costs = [2, 3, 5, 6]
        .into_iter()
        .map(|card_color| EvoCost {
            card_color,
            level: level.saturating_sub(1),
            memory_cost: 4,
        })
        .collect();
    c
}

pub fn digivolve_paths(card: &str, level: u8, standard: &[(CardColor, i16)], alternate: i16) {
    use digimon_engine::action::space::BREEDING_TARGET;
    for (color, glowing, cost) in standard
        .iter()
        .map(|&(color, cost)| (color, false, cost))
        .chain([(CardColor::Blue, true, alternate)])
    {
        let mut base = digimon("TEXT-BASE", level, 3000, glowing);
        base.colors = vec![color];
        if level == 2 {
            base.card_kind = CardKind::DigiEgg;
            base.dp = None;
        }
        let mut r = start(builder(card).add_card(base).hand(0, &[card]));
        let field = if level == 2 {
            r.place_in_breeding(0, "TEXT-BASE");
            BREEDING_TARGET
        } else {
            r.place_on_field(0, "TEXT-BASE", Some(0)).index as u16
        };
        let before = r.memory();
        action(&mut r, 0, encode_digivolve(0, field));
        assert_eq!(
            before - r.memory(),
            cost,
            "{card}: {color:?}, Glowing Dawn: {glowing}"
        );
        assert!(!ids(&r, &r.game.players[0].hand).contains(&card.to_owned()));
    }
}

pub fn security_tamer_play(card: &str) {
    let mut r = start(builder(card).security(1, &[card]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert_eq!(
        ids(&r, &r.game.players[1].battle_area[0].card_sources),
        [card]
    );
}

pub fn memory_at_start_of_turn(card: &str) {
    for (pass_value, expected) in [(1, 3), (2, 3), (4, 4)] {
        let mut r = start(builder(card));
        r.place_on_field(1, card, Some(0));
        r.game.set_memory(-pass_value);
        r.game.end_turn();
        assert_eq!(r.game.turn_player(), 1);
        assert_eq!(r.memory(), expected);
    }
}

pub fn champion_security_precedes_battle(card: &str, name: &str) {
    for accept in [false, true] {
        let mut tamer = make_test_card("TEXT-MATCH", &format!("{name} & Partner"));
        tamer.card_kind = CardKind::Tamer;
        tamer.level = None;
        tamer.dp = None;
        let mut r = start(
            builder(card)
                .add_card(tamer)
                .hand(1, &["TEXT-MATCH", "TEXT-TAMER"])
                .security(1, &[card]),
        );
        let attacker = r.place_on_field(0, "TEXT-3", Some(0));
        action(
            &mut r,
            0,
            encode_attack(attacker.index as u16, SECURITY_TARGET),
        );
        r.assert_prompt(SelectionKind::Hand, true, 1);
        assert_eq!(candidates(&r), ["TEXT-MATCH"]);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        if accept {
            r.choose_prompt_card("TEXT-MATCH", None).unwrap();
            assert_eq!(
                ids(&r, &r.game.players[1].battle_area[0].card_sources),
                ["TEXT-MATCH"]
            );
        } else {
            r.decline_prompt().unwrap();
            assert!(r.game.players[1].battle_area.is_empty());
        }
        assert!(r.game.players[0].battle_area.is_empty());
    }
}

pub fn pay_tamer_sources(r: &mut DebugRunner, player: u8, id: &str, count: usize) {
    for _ in 0..count {
        r.assert_prompt(SelectionKind::OwnField, false, player);
        assert_eq!(candidates(r), [id]);
        r.choose_prompt_card(id, None).unwrap();
    }
}

pub fn tamer_option_payment(card: &str, own_turn: bool, color: &str) {
    let option = |id: &str, col: &str, traits: &str| {
        format!(
            r#"
card: {id}
name: {id}
kind: option
cost: 5
color: [{col}]
traits: [{traits}]
effects:
  - when: main
    process:
      - draw: {{ of: you, count: 1 }}
"#
        )
    };
    for branch in 0..3 {
        let fixture = builder(card)
            .from_dsl_yaml(&option("TEXT-PAID-OPTION", color, "BEATBREAK"))
            .unwrap()
            .from_dsl_yaml(&option("TEXT-WRONG-COLOR", "green", "BEATBREAK"))
            .unwrap()
            .from_dsl_yaml(&option("TEXT-WRONG-TRAIT", color, "Other"))
            .unwrap();
        let options = ["TEXT-PAID-OPTION", "TEXT-WRONG-COLOR", "TEXT-WRONG-TRAIT"];
        let mut r = start(if own_turn {
            fixture
        } else {
            fixture.hand(0, &options)
        });
        if own_turn {
            for id in options {
                r.inject_trash(0, id);
            }
        }
        r.place_on_field(0, card, Some(0));
        let stock = r.place_stack(
            0,
            &[
                "TEXT-BOTTOM",
                "TEXT-BOTTOM",
                "TEXT-TOP",
                "TEXT-TOP",
                "TEXT-TAMER",
            ],
        );
        for source in &mut r.game.players[0].battle_area[stock.index as usize].card_sources[..4] {
            source.face_down = true;
        }
        if !own_turn {
            next_turn(&mut r);
        }
        let attacker_player = u8::from(!own_turn);
        let attacker = r.place_on_field(attacker_player, "TEXT-6", Some(0));
        let memory = r.memory();
        let deck = r.game.players[0].deck.len();
        action(
            &mut r,
            attacker_player,
            encode_attack(attacker.index as u16, SECURITY_TARGET),
        );
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(if branch == 0 { 2 } else { 1 }).unwrap();
        if branch == 0 {
            assert!(r.pending_selection().is_none());
            assert_eq!(
                r.game.players[0].battle_area[stock.index as usize]
                    .face_down_source_indices()
                    .count(),
                4
            );
            continue;
        }
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(if branch == 1 { 1 } else { 0 }).unwrap();
        if branch == 1 {
            assert!(r.pending_selection().is_none());
            assert_eq!(
                r.game.players[0].battle_area[stock.index as usize]
                    .face_down_source_indices()
                    .count(),
                4
            );
            continue;
        }
        pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 4);
        r.assert_prompt(
            if own_turn {
                SelectionKind::Trash
            } else {
                SelectionKind::Hand
            },
            true,
            0,
        );
        assert_eq!(candidates(&r), ["TEXT-PAID-OPTION"]);
        r.choose_prompt_card("TEXT-PAID-OPTION", None).unwrap();
        assert_eq!(r.game.players[0].deck.len(), deck - 1);
        assert_eq!(r.memory(), memory - if own_turn { 2 } else { 0 });
        assert_eq!(
            r.game.players[0].battle_area[stock.index as usize]
                .face_down_source_indices()
                .count(),
            0
        );
    }
}

pub fn builder(card: &str) -> DebugRunnerBuilder {
    let mut tamer = make_test_card("TEXT-TAMER", "Fixture Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.traits = vec!["Glowing Dawn".into(), "BEATBREAK".into()];
    DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(tamer)
        .add_card(digimon("TEXT-3", 3, 2000, true))
        .add_card(digimon("TEXT-4", 4, 4000, true))
        .add_card(digimon("TEXT-5", 5, 7000, true))
        .add_card(digimon("TEXT-6", 6, 12000, true))
        .add_card(digimon("TEXT-OTHER", 5, 7000, false))
        .add_card(make_test_card("TEXT-BOTTOM", "Bottom face-down card"))
        .add_card(make_test_card("TEXT-TOP", "Top face-down card"))
        .add_card(make_test_card("TEXT-FILLER", "Filler"))
        .deck(0, &["TEXT-FILLER"; 30])
        .deck(1, &["TEXT-FILLER"; 30])
        .security(0, &["TEXT-FILLER"; 5])
        .security(1, &["TEXT-FILLER"; 5])
        .memory(10)
}

pub fn start(builder: DebugRunnerBuilder) -> DebugRunner {
    let mut r = builder.start();
    r.game.enter_main_phase();
    r
}

pub fn action(r: &mut DebugRunner, player: u8, action: u16) {
    assert!(
        build_action_mask(&r.game, player)[action as usize] > 0.5,
        "illegal action {action}, phase {:?}, prompt {:?}",
        r.current_phase(),
        r.pending_selection_view()
    );
    r.game.decode_action(action, player);
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

pub fn stocked_tamer(r: &mut DebugRunner, id: &str) -> PermanentHandle {
    let h = r.place_stack(0, &["TEXT-BOTTOM", "TEXT-TOP", id]);
    for c in &mut r.game.players[0].battle_area[h.index as usize].card_sources[..2] {
        c.face_down = true;
    }
    h
}

pub fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn next_turn(r: &mut DebugRunner) {
    let player = r.game.turn_player();
    action(r, player, PASS);
    if r.current_phase() == digimon_engine::enums::GamePhase::Breeding {
        action(r, 1 - player, PASS);
    }
}

pub fn rookies_reduce_matching_beatbreak_digivolutions_in_the_battle_area(
    rookie: &str,
    color: CardColor,
) {
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

pub fn champions_offer_only_the_matching_tamer_on_digivolve_and_can_decline(
    card: &str,
    name: &str,
) {
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

pub fn hand_main_stacks_the_named_material_and_ignores_the_rookies_level(
    card: &str,
    tamer: &str,
    rookie: &str,
    material: &str,
) {
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

pub fn reina_and_makoto_can_stash_two_or_suspend_without_stashing(card: &str) {
    let deck_cards = ["TEXT-3", "TEXT-4", "TEXT-5"];
    for (branch, deck_size) in [(0, 3), (0, 1), (0, 0), (1, 3), (2, 3)] {
        let mut r = start(builder(card).deck(0, &deck_cards[..deck_size]));
        let tamer = r.place_stack(0, &["TEXT-BOTTOM", card]);
        let attacker = r.place_on_field(0, "TEXT-3", Some(0));
        let deck = r.game.players[0].deck.len();
        assert_eq!(deck, deck_size);
        let deck_before: Vec<_> = r.game.players[0].deck.iter().map(|c| c.handle()).collect();
        action(
            &mut r,
            0,
            encode_attack(attacker.index as u16, SECURITY_TARGET),
        );
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(branch).unwrap();
        let permanent = &r.game.players[0].battle_area[tamer.index as usize];
        assert_eq!(permanent.is_suspended, branch != 2);
        let stashed = if branch == 0 { deck_size.min(2) } else { 0 };
        assert_eq!(permanent.face_down_source_indices().count(), stashed);
        assert_eq!(r.game.players[0].deck.len(), deck - stashed);
        let (remaining, placed) = deck_before.split_at(deck - stashed);
        assert!(permanent.card_sources[..stashed]
            .iter()
            .all(|c| c.face_down));
        assert_eq!(
            permanent.card_sources[..stashed]
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            placed
        );
        assert_eq!(
            ids(&r, &permanent.card_sources[stashed..]),
            ["TEXT-BOTTOM", card]
        );
        assert_eq!(
            r.game.players[0]
                .deck
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            remaining
        );
    }
}
