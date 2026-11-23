pub(super) use super::common::{action, candidates, ids, next_turn, start};
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

pub(super) const TAMER: &str = "TEXT-KURATA-TAMER";
pub(super) const BOUNDARY: &str = "TEXT-KURATA-11000";
pub(super) const HIGH: &str = "TEXT-KURATA-12000";
pub(super) const KURATA: &str = "TEXT-KURATA-NAME";
pub(super) const EVOLUTION: &str = "TEXT-KURATA-EVOLUTION";
pub(super) const GIZMON: &str = "TEXT-KURATA-GIZMON";
pub(super) const XT: &str = "TEXT-KURATA-XT";
const FILLER: &str = "TEXT-KURATA-FILLER";

pub(super) fn builder(card: &str) -> DebugRunnerBuilder {
    let mut tamer = make_test_card(TAMER, "Test Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Purple];
    tamer.traits = vec!["Kurata's Army".into()];
    let mut kurata = tamer.clone();
    kurata.card_id = KURATA.into();
    kurata.card_name = "Akihiro Kurata".into();
    let mut boundary = make_test_card(BOUNDARY, "Boundary");
    boundary.dp = Some(11000);
    let mut high = make_test_card(HIGH, "High");
    high.dp = Some(12000);
    let mut evolution = make_test_card(EVOLUTION, "Eligible evolution");
    evolution.level = Some(4);
    evolution.evo_costs = vec![EvoCost {
        card_color: 6,
        level: 3,
        memory_cost: 1,
    }];
    DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(tamer)
        .add_card(kurata)
        .add_card(boundary)
        .add_card(high)
        .add_card(evolution)
        .add_card(make_test_card(GIZMON, "Gizmon: AT"))
        .add_card(make_test_card(XT, "Gizmon: XT"))
        .add_card(make_test_card(FILLER, "Filler"))
        .deck(0, &[FILLER; 30])
        .deck(1, &[FILLER; 30])
        .security(0, &[FILLER; 5])
        .security(1, &[FILLER; 5])
        .memory(10)
}

pub(super) fn delete_in_battle(r: &mut DebugRunner, card: &str) {
    let target = r.place_on_field(0, card, Some(0));
    delete_suspended_target_in_battle(r, target);
}

fn delete_suspended_target_in_battle(
    r: &mut DebugRunner,
    target: digimon_engine::permanent::PermanentHandle,
) {
    let attacker = r.place_on_field(1, HIGH, Some(0));
    next_turn(r);
    if r.pending_selection()
        .is_some_and(|pending| pending.prompt.starts_with("<Engage>"))
    {
        r.assert_prompt(SelectionKind::Target, true, 0);
        r.decline_prompt().unwrap();
    }
    r.game.players[0].battle_area[target.index as usize].is_suspended = true;
    action(
        r,
        1,
        encode_attack(attacker.index as u16, target.index as u16),
    );
}

pub(super) fn armor_purge_in_battle(card: &str) {
    for accept in [false, true] {
        let mut r = start(builder(card));
        let host = r.place_stack(0, &[BOUNDARY, card]);
        delete_suspended_target_in_battle(&mut r, host);
        if card == "BT27-055" {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            assert!(r
                .pending_selection()
                .unwrap()
                .prompt
                .contains("triggered effect"));
            r.decline_optional_trigger().unwrap();
        }
        if r.current_phase() == digimon_engine::enums::GamePhase::CounterTiming {
            r.assert_prompt(SelectionKind::Hand, true, 0);
            r.decline_prompt().unwrap();
        }
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if accept {
            r.accept_optional_trigger().unwrap();
            assert_eq!(
                ids(&r, &r.game.players[0].battle_area[0].card_sources),
                [BOUNDARY]
            );
            assert_eq!(ids(&r, &r.game.players[0].trash), [card]);
        } else {
            r.decline_optional_trigger().unwrap();
            assert!(r.game.players[0].battle_area.is_empty());
            let mut trashed = ids(&r, &r.game.players[0].trash);
            trashed.sort();
            assert_eq!(trashed, [card, BOUNDARY]);
        }
        assert!(r.pending_selection().is_none());
    }
}

pub(super) fn gizmon_recycles_two_to_play_from_trash(card: &str, reward: &str) {
    let payments = ["TEXT-PAY-GIZMON-A", "TEXT-PAY-GIZMON-B"];
    for (accept, play) in [(false, false), (true, false), (true, true)] {
        let mut r = start(
            builder(card)
                .add_card(make_test_card(payments[0], "ProtoGizmon"))
                .add_card(make_test_card(payments[1], "Gizmon Prototype")),
        );
        for entry in [payments[0], payments[1], reward, BOUNDARY] {
            r.inject_trash(0, entry);
        }
        delete_in_battle(&mut r, card);
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if !accept {
            r.decline_optional_trigger().unwrap();
            assert_eq!(r.trash_size(0), 5);
            assert!(r.pending_selection().is_none());
            continue;
        }
        r.accept_optional_trigger().unwrap();
        for payment in payments {
            let kind = r.pending_selection().unwrap().kind.clone();
            assert!(matches!(kind, SelectionKind::CountCappedMultiSelect { .. }));
            r.assert_prompt(kind, false, 0);
            assert!(!candidates(&r).contains(&BOUNDARY.to_string()));
            r.choose_prompt_card(payment, None).unwrap();
        }
        let mut returned = ids(&r, &r.game.players[0].deck[..2]);
        returned.sort();
        assert_eq!(returned, payments);
        r.assert_prompt(SelectionKind::Trash, true, 0);
        assert_eq!(candidates(&r), [reward]);
        if play {
            r.choose_prompt_card(reward, None).unwrap();
            assert_eq!(
                ids(&r, &r.game.players[0].battle_area[0].card_sources),
                [reward]
            );
        } else {
            r.decline_prompt().unwrap();
            assert!(r.game.players[0].battle_area.is_empty());
        }
        assert!(r.pending_selection().is_none());
    }
}

pub(super) fn fixed_play_cost_payment(card: &str, level: u8, full: i16, reduction: i16) {
    use digimon_engine::enums::Expiry;

    const COST: &str = "TEXT-KURATA-COST";
    for (accept, evade) in [(false, false), (true, false), (true, true)] {
        let mut payment = make_test_card(COST, "Cost candidate");
        payment.level = Some(level);
        payment.play_cost = 1;
        let mut decoy = make_test_card("TEXT-COST-DECOY", "Wrong level");
        decoy.level = Some(level + 1);
        decoy.play_cost = 1;
        let mut r = start(
            builder(card)
                .add_card(payment)
                .add_card(decoy)
                .hand(0, &[card, "TEXT-COST-DECOY"]),
        );
        let payer = r.place_on_field(0, COST, Some(0));
        r.place_on_field(0, "TEXT-COST-DECOY", Some(0));
        r.place_on_field(1, COST, Some(0));
        if evade {
            r.game
                .modifiers
                .grant_keyword(payer, Keyword::Evade, Expiry::Permanent, 0);
            r.game.tick_declarative_effects();
        }
        let before = r.memory();
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::EffectChoice, true, 0);
        assert_eq!(
            r.pending_selection()
                .unwrap()
                .effect_choices
                .as_ref()
                .unwrap()[0]
                .label,
            format!("Cost reduction (-{reduction})")
        );
        if accept {
            r.execute_branch(0).unwrap();
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            assert_eq!(candidates(&r), [COST]);
            r.choose_prompt_card(COST, None).unwrap();
            if evade {
                r.assert_prompt(SelectionKind::Replacement, true, 0);
                r.accept_optional_trigger().unwrap();
                assert!(r.game.players[0].battle_area[payer.index as usize].is_suspended);
            }
        } else {
            r.decline_prompt().unwrap();
        }
        let paid = accept && !evade;
        let trash = ids(&r, &r.game.players[0].trash);
        assert_eq!(trash, if paid { &[COST][..] } else { &[] });
        assert_eq!(
            before - r.memory(),
            if paid { full - reduction } else { full }
        );
        assert_eq!(r.game.players[1].battle_area.len(), 1);
        assert!(r.pending_selection().is_none());
        let before_next_play = r.memory();
        action(&mut r, 0, PLAY_HAND_START);
        assert_eq!(before_next_play - r.memory(), 1);
    }
}

pub(super) fn tamer_evolution(card: &str, cost: i16, keywords: &[Keyword]) {
    let mut r = start(builder(card).hand(0, &[card]));
    let base = r.place_on_field(0, TAMER, Some(0));
    r.place_on_field(1, BOUNDARY, Some(0));
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(before - r.memory(), cost);
    let stack = &r.game.players[0].battle_area[base.index as usize];
    assert_eq!(stack.top_card().card_id(&r.game.card_data), card);
    assert_eq!(stack.card_sources[0].card_id(&r.game.card_data), TAMER);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(r.pending_selection().is_none());
    for &keyword in keywords {
        assert!(
            r.game.has_keyword(base, keyword),
            "{card} lacks {keyword:?}"
        );
    }
}

pub(super) fn security_plays_self(card: &str) {
    let mut r = start(builder(card).security(1, &[card]));
    let attacker = r.place_on_field(0, HIGH, Some(0));
    let before = r.memory();
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert_eq!(r.security_count(1), 0);
    assert_eq!(before, r.memory());
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        card
    );
    assert!(r.pending_selection().is_none());
}

pub(super) fn evolving(
    card: &str,
    base: digimon_engine::card_data::CardData,
    cost: i16,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let base_id = base.card_id.clone();
    let mut r = start(builder(card).add_card(base).hand(0, &[card]));
    let host = r.place_on_field(0, &base_id, Some(0));
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, host.index as u16));
    assert_eq!(before - r.memory(), cost);
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[0].card_sources),
        [base_id.as_str(), card]
    );
    (r, host)
}

pub(super) fn standard_routes(
    card: &str,
    color: CardColor,
    level: u8,
    cost: i16,
    keywords: &[Keyword],
) {
    for color in [color, CardColor::Purple] {
        let mut base = make_test_card("TEXT-BT27-STD-BASE", "Standard base");
        base.colors = vec![color];
        base.level = Some(level);
        let (mut r, host) = evolving(card, base, cost);
        if card == "BT27-048" {
            r.assert_prompt(SelectionKind::AnyField, true, 0);
            r.decline_prompt().unwrap();
        }
        if level == 3 {
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            r.decline_prompt().unwrap();
        }
        for &kw in keywords {
            assert!(r.game.has_keyword(host, kw), "{card} lacks {kw:?}");
        }
        assert!(r.pending_selection().is_none());
    }
}

pub(super) fn evolve_against_an_enemy(
    card: &str,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut base = make_test_card("TEXT-PURPLE-ROOKIE", "Purple level 3");
    base.colors = vec![CardColor::Purple];
    let mut r = start(builder(card).add_card(base).hand(0, &[card]));
    let host = r.place_on_field(0, "TEXT-PURPLE-ROOKIE", Some(0));
    r.place_on_field(1, HIGH, Some(0));
    action(&mut r, 0, encode_digivolve(0, host.index as u16));
    (r, host)
}

pub(super) fn attack_security(
    r: &mut DebugRunner,
    host: digimon_engine::permanent::PermanentHandle,
) {
    action(
        r,
        host.player,
        encode_attack(host.index as u16, SECURITY_TARGET),
    );
}

pub(super) fn assert_cannot_digivolve(
    r: &DebugRunner,
    host: digimon_engine::permanent::PermanentHandle,
) {
    assert!(r
        .game
        .modifiers
        .has(host, digimon_engine::enums::ModifierType::CannotDigivolve));
}
