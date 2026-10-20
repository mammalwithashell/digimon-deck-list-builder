//! BT6-068 Impmon: official four-printing observations + actual images; Q1462.
//! Rules manual pp22/24: 15-1-2/5, 15-7-1/2/5. Discard is optional;
//! returning an exact-trait Digimon is mandatory after a successful discard.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::{
    build_action_mask, encode_digivolve, BREEDING_TARGET, PASS, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::SelectionKind;

const CID: &str = "BT6-068";
const F: &str = "TEST-IMPMON-FILLER";
const LORD: &str = "TEST-IMPMON-LORD";
const MUSK: &str = "TEST-IMPMON-MUSK";
const DUAL: &str = "TEST-IMPMON-DUAL";
const OPT: &str = "TEST-IMPMON-OPTION";
const TAMER: &str = "TEST-IMPMON-TAMER";
const NEAR: &str = "TEST-IMPMON-NEAR";
const EGG: &str = "TEST-IMPMON-EGG";
const WRONG: &str = "TEST-IMPMON-WRONG-EGG";

fn fixture(id: &str, kind: CardKind, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![CardColor::Purple];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn setup(player: u8) -> DebugRunner {
    let rows = CardData::load_from_str(include_str!(
        "../../fixtures/focused_remaining_effects_r4.json"
    ))
    .unwrap();
    let mut egg = fixture(EGG, CardKind::DigiEgg, &[]);
    egg.level = Some(2);
    let mut wrong = egg.clone();
    wrong.card_id = WRONG.to_string();
    wrong.colors = vec![CardColor::Blue];
    let mut r = DebugRunner::builder()
        .dsl_card(CID)
        .expect("BT6-068 must exist in embedded native pack")
        .with_card_data(rows)
        .add_card(fixture(F, CardKind::Digimon, &[]))
        .add_card(fixture(
            LORD,
            CardKind::Digimon,
            &["Seven Great Demon Lords"],
        ))
        .add_card(fixture(MUSK, CardKind::Digimon, &["Three Musketeers"]))
        .add_card(fixture(DUAL, CardKind::Dual, &["Three Musketeers"]))
        .add_card(fixture(OPT, CardKind::Option, &["Seven Great Demon Lords"]))
        .add_card(fixture(TAMER, CardKind::Tamer, &["Three Musketeers"]))
        .add_card(fixture(
            NEAR,
            CardKind::Digimon,
            &["Seven Great Demon Lords X", "Three Musketeers X"],
        ))
        .add_card(egg)
        .add_card(wrong)
        .deck(0, &[F; 12])
        .deck(1, &[F; 12])
        .security(0, &[F; 3])
        .security(1, &[F; 3])
        .build();
    r.set_first_player(player);
    r.set_turn(2);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(10);
    r
}

fn act(r: &mut DebugRunner, player: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
}

fn play(r: &mut DebugRunner, player: u8) {
    let idx = r.add_to_hand(player, CID);
    act(r, player, idx as u16);
    assert_eq!(r.memory(), 7);
}

fn ids(r: &DebugRunner, player: u8, trash: bool) -> Vec<String> {
    let p = &r.game.players[player as usize];
    let cards = if trash { &p.trash } else { &p.hand };
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn prompt(r: &DebugRunner, player: u8, kind: SelectionKind, optional: bool, actions: &[u16]) {
    let v = r
        .pending_selection_view()
        .expect("player-visible choice required even for one target");
    assert_eq!(v.selecting_player, player);
    assert_eq!(v.kind, kind);
    assert_eq!(v.is_optional, optional);
    let mut actual = v.valid_action_ids.clone();
    actual.sort();
    let mut expected = actions.to_vec();
    expected.sort();
    assert_eq!(actual, expected);
    let mask = build_action_mask(&r.game, player);
    for a in &expected {
        assert_eq!(mask[*a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], if optional { 1.0 } else { 0.0 });
    assert!(build_action_mask(&r.game, 1 - player)
        .iter()
        .all(|&x| x == 0.0));
}

#[test]
fn bt6_068_native_definition_has_exact_printed_metadata_and_only_on_play() {
    let r = setup(0);
    let c = r.compiled_card(CID).unwrap();
    assert_eq!(c.name, "Impmon");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.color, [CompiledColor::Purple]);
    assert_eq!(c.level, Some(3));
    assert_eq!(c.cost, Some(3));
    assert_eq!(c.dp, Some(2000));
    assert_eq!(c.all_traits(), ["Rookie", "Virus", "Evil"]);
    assert!(c.alt_paths.is_empty());
    assert_eq!(c.effects.len(), 1);
    match &c.effects[0] {
        CompiledClause::Triggered(t) => {
            assert_eq!(t.when, [CompiledTiming::OnPlay]);
            assert_eq!(t.scope, CompiledScope::FaceUp);
            assert!(!t.once_per_turn);
            assert!(
                !t.optional,
                "the optional discard owns the single decline prompt"
            );
        }
        _ => panic!("On Play triggered clause required"),
    }
    let d = r.game.card_data.iter().find(|x| x.card_id == CID).unwrap();
    assert_eq!(
        d.evo_costs,
        [EvoCost {
            card_color: 6,
            level: 2,
            memory_cost: 0
        }]
    );
    assert!(d.inherited_text.is_empty() && d.security_text.is_empty());
}

#[test]
fn bt6_068_real_play_exposes_every_hand_card_and_optional_decline_both_seats() {
    for p in 0..2 {
        let mut r = setup(p);
        for id in [F, LORD, OPT, TAMER] {
            r.add_to_hand(p, id);
        }
        r.inject_trash(p, MUSK);
        play(&mut r, p);
        prompt(&r, p, SelectionKind::Hand, true, &[0, 1, 2, 3]);
        act(&mut r, p, PASS);
        assert_eq!(ids(&r, p, false), [F, LORD, OPT, TAMER]);
        assert_eq!(ids(&r, p, true), [MUSK]);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn bt6_068_empty_hand_cannot_return_existing_eligible_trash() {
    for p in 0..2 {
        let mut r = setup(p);
        r.inject_trash(p, LORD);
        play(&mut r, p);
        assert!(r.pending_selection().is_none());
        assert!(ids(&r, p, false).is_empty());
        assert_eq!(ids(&r, p, true), [LORD]);
    }
}

#[test]
fn bt6_068_discard_allowed_without_return_target_and_returns_nothing() {
    for p in 0..2 {
        let mut r = setup(p);
        r.add_to_hand(p, F);
        r.inject_trash(p, NEAR);
        play(&mut r, p);
        prompt(&r, p, SelectionKind::Hand, true, &[0]);
        act(&mut r, p, 0);
        assert!(r.pending_selection().is_none());
        assert_eq!(ids(&r, p, true), [NEAR, F]);
        assert!(ids(&r, p, false).is_empty());
    }
}

#[test]
fn bt6_068_return_is_mandatory_and_filters_exact_trait_kind_and_own_trash() {
    for p in 0..2 {
        for (chosen, id) in [(0, LORD), (1, MUSK), (2, DUAL)] {
            let mut r = setup(p);
            r.add_to_hand(p, F);
            for id in [LORD, MUSK, DUAL, OPT, TAMER, NEAR, F] {
                r.inject_trash(p, id);
            }
            r.inject_trash(1 - p, LORD);
            r.add_to_hand(p, MUSK);
            r.place_on_field(p, LORD, Some(0));
            r.game.stage_inject_card(p, LORD, "security_top").unwrap();
            play(&mut r, p);
            prompt(&r, p, SelectionKind::Hand, true, &[0, 1]);
            act(&mut r, p, 0);
            prompt(
                &r,
                p,
                SelectionKind::Trash,
                false,
                &[
                    TRASH_EFFECT_START,
                    TRASH_EFFECT_START + 1,
                    TRASH_EFFECT_START + 2,
                ],
            );
            let before = ids(&r, p, true);
            assert!(r.execute_action(p, PASS).is_err());
            assert_eq!(ids(&r, p, true), before);
            act(&mut r, p, TRASH_EFFECT_START + chosen);
            assert_eq!(ids(&r, p, false), [MUSK, id]);
            assert_eq!(r.trash_size(p), 7);
            assert_eq!(ids(&r, 1 - p, true), [LORD]);
            assert_eq!(r.battle_area_size(p), 2);
            assert_eq!(r.security_count(p), 4);
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt6_068_newly_discarded_matching_digimon_can_return_itself_both_traits() {
    for p in 0..2 {
        for id in [LORD, MUSK] {
            let mut r = setup(p);
            let idx = r.add_to_hand(p, id);
            let identity = r.game.players[p as usize].hand[idx].handle();
            play(&mut r, p);
            prompt(&r, p, SelectionKind::Hand, true, &[0]);
            act(&mut r, p, 0);
            prompt(&r, p, SelectionKind::Trash, false, &[TRASH_EFFECT_START]);
            assert_eq!(r.game.players[p as usize].trash[0].handle(), identity);
            act(&mut r, p, TRASH_EFFECT_START);
            assert_eq!(r.game.players[p as usize].hand[0].handle(), identity);
            assert_eq!(ids(&r, p, false), [id]);
            assert_eq!(r.trash_size(p), 0);
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt6_068_cloned_discard_and_return_prompts_resume_independently_once() {
    for p in 0..2 {
        let mut original = setup(p);
        original.add_to_hand(p, F);
        original.add_to_hand(p, MUSK);
        original.inject_trash(p, LORD);
        play(&mut original, p);
        let mut declined = setup(p);
        declined.game = original.game.clone();
        prompt(&declined, p, SelectionKind::Hand, true, &[0, 1]);
        act(&mut declined, p, PASS);
        assert_eq!(ids(&declined, p, false), [F, MUSK]);
        assert_eq!(ids(&declined, p, true), [LORD]);
        prompt(&original, p, SelectionKind::Hand, true, &[0, 1]);
        act(&mut original, p, 1);
        prompt(
            &original,
            p,
            SelectionKind::Trash,
            false,
            &[TRASH_EFFECT_START, TRASH_EFFECT_START + 1],
        );
        let mut fork = setup(p);
        fork.game = original.game.clone();
        act(&mut fork, p, TRASH_EFFECT_START + 1);
        assert_eq!(ids(&fork, p, false), [F, MUSK]);
        assert_eq!(ids(&fork, p, true), [LORD]);
        prompt(
            &original,
            p,
            SelectionKind::Trash,
            false,
            &[TRASH_EFFECT_START, TRASH_EFFECT_START + 1],
        );
        act(&mut original, p, TRASH_EFFECT_START);
        assert_eq!(ids(&original, p, false), [F, LORD]);
        assert_eq!(ids(&original, p, true), [MUSK]);
        assert!(original.pending_selection().is_none() && fork.pending_selection().is_none());
        assert_eq!(original.battle_area_size(p), 1);
        assert_eq!(fork.battle_area_size(p), 1);
    }
}

#[test]
fn bt6_068_ordinary_purple_egg_evolution_cost_zero_draws_without_on_play() {
    for p in 0..2 {
        for target in [0, BREEDING_TARGET] {
            for (base, legal) in [(EGG, true), (WRONG, false), (F, false)] {
                let mut r = setup(p);
                let idx = r.add_to_hand(p, CID);
                r.add_to_hand(p, F);
                r.inject_trash(p, LORD);
                if target == BREEDING_TARGET {
                    r.place_in_breeding(p, base);
                } else {
                    r.place_on_field(p, base, Some(0));
                }
                let action = encode_digivolve(idx as u16, target);
                assert_eq!(
                    build_action_mask(&r.game, p)[action as usize],
                    if legal { 1.0 } else { 0.0 }
                );
                if !legal {
                    continue;
                }
                let deck = r.deck_size(p);
                act(&mut r, p, action);
                assert_eq!(r.memory(), 10);
                assert_eq!(r.deck_size(p), deck - 1);
                assert_eq!(r.hand_size(p), 2);
                assert_eq!(ids(&r, p, true), [LORD]);
                assert!(r.pending_selection().is_none());
            }
        }
    }
}
