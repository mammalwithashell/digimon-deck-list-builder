use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, ATTACK_START, DNA_DIGIVOLVE_START, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
const ID: &str = "EX3-074";
fn setup(hand: &[&str]) -> DebugRunner {
    setup_extra(hand, Vec::new())
}
fn setup_extra(hand: &[&str], extra: Vec<digimon_engine::card_data::CardData>) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/ex3/EX3-074.yaml"))
        .unwrap();
    for c in extra {
        b = b.add_card(c);
    }
    for (id, name, lv, color, dp) in [
        ("G", "Groundramon", 6, CardColor::Green, 12000),
        ("B", "Wingdramon", 6, CardColor::Blue, 11000),
        ("HIGH", "HighDramon", 6, CardColor::Blue, 13000),
        ("RED", "RedDramon", 6, CardColor::Red, 10000),
        ("BAD", "Bad", 6, CardColor::Green, 10000),
        ("LOW", "LowDramon", 5, CardColor::Green, 7000),
        ("FILL", "Fill", 3, CardColor::White, 1000),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(lv);
        c.colors = vec![color];
        c.dp = Some(dp);
        b = b.add_card(c);
    }
    let mut t = make_test_card("T", "TamerDramon");
    t.card_kind = CardKind::Tamer;
    t.colors = vec![CardColor::Blue];
    t.dp = Some(1000);
    b = b.add_card(t);
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn choose(r: &mut DebugRunner, a: u16) {
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.execute_action(0, a).unwrap();
}
fn evolve(r: &mut DebugRunner, dna: bool) {
    r.place_on_field(0, "G", None);
    if dna {
        r.place_on_field(0, "B", None);
        assert!(build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize] > 0.0);
        r.game.decode_action(DNA_DIGIVOLVE_START, 0);
        let first = r.pending_selection_view().unwrap().valid_action_ids[0];
        choose(r, first);
        let second = r.pending_selection_view().unwrap().valid_action_ids[0];
        choose(r, second);
    } else {
        let a = encode_digivolve(0, 0);
        assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
        r.game.decode_action(a, 0);
    }
}
#[test]
fn native_tuck_filters_and_bottom_placement() {
    let mut r = setup(&[ID, "HIGH", "G", "RED", "BAD", "T"]);
    evolve(&mut r, false);
    let p = r.pending_selection_view().unwrap();
    assert!(p.is_optional);
    assert_eq!(p.valid_action_ids, vec![0, 1]);
    choose(&mut r, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources[0].card_id(&r.game.card_data),
        "HIGH"
    );
    assert_eq!(r.memory(), 5);
}
#[test]
fn dna_independent_options_clone_and_free_play() {
    for tuck in [true, false] {
        let mut r = setup(&[ID, "HIGH", "G", "B", "RED", "BAD", "T"]);
        evolve(&mut r, true);
        assert_eq!(r.memory(), 10);
        if tuck {
            choose(&mut r, 0);
        } else {
            choose(&mut r, PASS);
        }
        let p = r.pending_selection_view().unwrap();
        assert!(p.is_optional);
        assert_eq!(
            p.valid_action_ids,
            if tuck { vec![0, 1] } else { vec![1, 2] }
        );
        let mut c = setup(&[]);
        c.game = r.game.clone();
        let a = p.valid_action_ids[0];
        choose(&mut r, a);
        choose(&mut c, PASS);
        assert_eq!(r.game.players[0].battle_area.len(), 2);
        assert_eq!(c.game.players[0].battle_area.len(), 1);
        assert_eq!(r.memory(), 10);
    }
}
#[test]
fn native_decline_and_empty_hand_finish() {
    for hand in [vec![ID, "G"], vec![ID]] {
        let mut r = setup(&hand);
        evolve(&mut r, false);
        if r.pending_selection().is_some() {
            choose(&mut r, PASS);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}
#[test]
fn suspension_mandatory_tail_opt_subject_and_both_turns() {
    for turn in [0, 1] {
        let mut r = setup(&[]);
        let h = r.place_on_field(0, ID, None);
        let own = r.place_on_field(0, "G", None);
        r.place_on_field(1, "G", None);
        r.place_on_field(1, "B", None);
        r.place_on_field(1, "T", None);
        r.game.turn_player_idx = turn;
        r.game.suspend(own);
        assert!(r.pending_selection().is_none());
        r.game.suspend(h);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        let p = r.pending_selection_view().unwrap();
        assert!(!p.is_optional);
        assert_eq!(p.valid_action_ids, vec![ATTACK_START, ATTACK_START + 1]);
        let mut clone = setup(&[]);
        clone.game = r.game.clone();
        choose(&mut r, ATTACK_START);
        choose(&mut clone, ATTACK_START + 1);
        assert!(r.game.players[1].battle_area[0].is_suspended);
        assert!(clone.game.players[1].battle_area[1].is_suspended);
        r.game.suspend(h);
        assert!(r.game.players[0].battle_area[0].is_suspended);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn routes_are_exact() {
    for (base, legal) in [("G", true), ("B", true), ("RED", false), ("LOW", false)] {
        let mut r = setup(&[ID]);
        r.place_on_field(0, base, None);
        let a = encode_digivolve(0, 0);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize] > 0.0, legal);
        if legal {
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 5);
        }
    }
    for (a, b, legal) in [
        ("G", "B", true),
        ("G", "G", false),
        ("B", "B", false),
        ("G", "LOW", false),
        ("RED", "B", false),
    ] {
        let mut r = setup(&[ID]);
        r.place_on_field(0, a, None);
        r.place_on_field(0, b, None);
        assert_eq!(
            build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize] > 0.0,
            legal
        );
    }
}

#[test]
fn prevented_unsuspend_still_suspends_opponent_and_no_target_completes() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    for opponent in [false, true] {
        let mut r = setup(&[]);
        let h = r.place_on_field(0, ID, None);
        if opponent {
            r.place_on_field(1, "G", None);
        }
        r.game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::CannotUnsuspend, 0, Expiry::Permanent, 1),
        );
        r.game.suspend(h);
        assert!(r.game.players[0].battle_area[0].is_suspended);
        if opponent {
            choose(&mut r, ATTACK_START);
            assert!(r.game.players[1].battle_area[0].is_suspended);
        }
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn opt_resets_and_inherited_copy_does_not_trigger() {
    let mut r = setup(&[]);
    let h = r.place_on_field(0, ID, None);
    r.game.suspend(h);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    r.game.suspend(h);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    r.end_turn();
    r.auto_resolve().unwrap();
    r.game.unsuspend(h);
    r.game.suspend(h);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    let other = r.place_stack(0, &[ID, "G"]);
    r.game.suspend(other);
    assert!(r.game.players[0].battle_area[other.index as usize].is_suspended);
}

// ─── DNA free-play candidates must be playable (G-DSL-PLAY-CANDIDATE-LEGALITY) ──
// DCGO EX3_074.cs:137-162 `CanSelectCardCondition1` adds
// `CardEffectCommons.CanPlayAsNewPermanent(cardSource, payCost: false, ...)`
// (:151) to the name/colour/DP gates: a card the play would refuse is never
// offered. CR 4.3 7-1-1 (playing places a card with a play cost onto the
// field), 4-6-1/4-6-2 (a DUAL card is digivolved or used, never played) and
// 1-3-2 (an impossible action is not carried out).

/// A green DUAL card with [Dramon] in its name whose Digimon face has 7000 DP.
fn dual_dramon(id: &str) -> digimon_engine::card_data::CardData {
    use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
    let mut c = make_test_card(id, "DualDramon");
    c.card_kind = CardKind::Dual;
    c.colors = vec![CardColor::Green];
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 4;
    c.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 5,
            dp: 7000,
            colors: vec![CardColor::Green],
            traits: vec![],
            evo_costs: vec![],
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost: 4,
            colors: vec![CardColor::Green],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    c
}

/// DNA digivolve, PASS the optional tuck, and return the card ids the DNA
/// free-play pick offers (empty when no pick installs).
fn dna_free_play_offer(r: &mut DebugRunner) -> Vec<String> {
    dna_free_play_offer_after(r, |_| {})
}
fn dna_free_play_offer_after(r: &mut DebugRunner, after_evolve: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    evolve(r, true);
    after_evolve(r);
    // The tuck pick (optional) comes first; decline it.
    if r.pending_selection().is_some() {
        choose(r, PASS);
    }
    let Some(v) = r.pending_selection_view() else {
        return Vec::new();
    };
    let hand: Vec<String> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| hand[a as usize].clone())
        .collect()
}

/// A DUAL card can't be played (CR 4.3 4-6-2; DCGO `CanPlayAsNewPermanent`
/// requires `!IsOption`): only Wingdramon is offered.
#[test]
fn dna_free_play_excludes_dual_card() {
    let mut r = setup_extra(&[ID, "DUAL", "B"], vec![dual_dramon("DUAL")]);
    assert_eq!(dna_free_play_offer(&mut r), vec!["B".to_string()]);
}

/// "You can't play Digimon by effects": no free-play pick installs.
#[test]
fn dna_free_play_offers_nothing_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup(&[ID, "B"]);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    assert!(dna_free_play_offer(&mut r).is_empty());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

/// A full battle area (no empty frame, DCGO `CanPlayAsNewPermanent`): no pick.
#[test]
fn dna_free_play_offers_nothing_when_battle_area_full() {
    let mut r = setup(&[ID, "B"]);
    // After the DNA digivolve only Examon remains; cap the area at 1 slot
    // before the free-play pick builds its candidates.
    let offered = dna_free_play_offer_after(&mut r, |r| r.game.rules.field_slots = 1);
    assert!(offered.is_empty(), "{offered:?}");
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}
