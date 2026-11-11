//! Rule guards found by the league game audit (2026-11-11).
//!
//! - A DUAL card prints "Can't play to the field." (CR 4-6-2: it is digivolved
//!   with its Digimon information or used with its Option information). DCGO
//!   `CanPlayAsNewPermanent` refuses it, and as a Digimon it has no play cost.
//! - CR 11-1-2: only the turn player can attack, including effect attacks.
use digimon_engine::{
    card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, EffectTiming},
    selection::TriggerSource,
};

const PLAYER_YAML: &str = r#"
card: PLAY-PICKER
name: Play picker
kind: tamer
color: [white]
cost: 1
effects:
  - when: main_on_field
    summary: "Play 1 play cost 5 or lower Digimon card from hand without paying the cost"
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter:
            kind: digimon
            play_cost_lte: 5
          prompt: "Play 1 play cost 5 or lower Digimon card"
      - play_from_hand_free: { of: you, hand_index: pick }
"#;

fn dual_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Dual;
    c.level = Some(6);
    c.dp = Some(13000);
    // cards.json stores the Option face's use cost in `play_cost`.
    c.play_cost = 2;
    c.colors = vec![CardColor::Green];
    c.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 13000,
            colors: vec![CardColor::Green],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: Vec::new(),
        },
        option: DualOptionFace {
            use_cost: 2,
            colors: vec![CardColor::Green],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
        },
    });
    c
}

fn plain(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.play_cost = 3;
    c
}

fn runner(hand: &[&str]) -> DebugRunner {
    let mut source = make_test_card("PLAY-PICKER", "Play picker");
    source.card_kind = CardKind::Tamer;
    source.level = None;
    source.dp = None;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PLAYER_YAML)
        .unwrap()
        .add_card(source)
        .add_card(dual_card("DUAL"))
        .add_card(plain("PLAIN"))
        .hand(0, hand)
        .deck(0, &["PLAIN"; 8])
        .deck(1, &["PLAIN"; 8])
        .memory(5)
        .start();
    r.skip_mulligan();
    r
}

#[test]
fn a_dual_card_is_not_a_play_cost_candidate_for_an_effect_play() {
    let mut r = runner(&["DUAL", "PLAIN"]);
    let source = r.place_on_field(0, "PLAY-PICKER", None);
    r.game.enter_main_phase();
    r.game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("hand pick");
    assert_eq!(
        view.valid_action_ids,
        vec![1],
        "only PLAIN (hand slot 1): a DUAL card has no play cost"
    );
}

#[test]
fn a_dual_card_cannot_be_played_to_the_field() {
    let mut r = runner(&["DUAL"]);
    r.game.enter_main_phase();
    let card = r.game.players[0].hand[0].handle();
    assert!(
        !r.game.can_play_card_from_effect_without_cost(0, card, 1),
        "effects can't play a DUAL card"
    );
    assert!(r.game.play_from_hand(0, 0).is_none(), "nor can a hand play");
    assert!(r.game.players[0].battle_area.is_empty());
}

/// CR 7-1-1: playing places "a new card with a play cost" onto the field.
/// A Digi-Egg and an Option have none (DCGO `HasPlayCost`), so no effect can
/// play them. Rose Rave audit 2026-11-11 (Rosemon BT26-049, Crowmon BT26-076,
/// Pinamon BT26-005 offered them).
#[test]
fn digi_eggs_and_options_cannot_be_played_by_effects() {
    let mut egg = make_test_card("EGG", "Egg");
    egg.card_kind = CardKind::DigiEgg;
    egg.level = Some(2);
    egg.play_cost = 0;
    let mut opt = make_test_card("OPT", "Opt");
    opt.card_kind = CardKind::Option;
    opt.level = None;
    opt.dp = None;
    opt.play_cost = 2;
    let mut source = make_test_card("PLAY-PICKER", "Play picker");
    source.card_kind = CardKind::Tamer;
    source.level = None;
    source.dp = None;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PLAYER_YAML)
        .unwrap()
        .add_card(source)
        .add_card(egg)
        .add_card(opt)
        .add_card(plain("PLAIN"))
        .hand(0, &["EGG", "OPT", "PLAIN"])
        .deck(0, &["PLAIN"; 8])
        .deck(1, &["PLAIN"; 8])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    for i in 0..2 {
        let card = r.game.players[0].hand[i].handle();
        assert!(
            !r.game.can_play_card_from_effect_without_cost(0, card, 1),
            "hand slot {i} has no play cost"
        );
    }
    let plain_card = r.game.players[0].hand[2].handle();
    assert!(r.game.can_play_card_from_effect_without_cost(0, plain_card, 1));
}

#[test]
fn an_effect_cannot_make_the_non_turn_player_attack() {
    use digimon_engine::combat::{AttackInitiator, AttackOpen, AttackResult, TargetConstraint};
    use digimon_engine::selection::AttackTarget;
    let mut r = runner(&[]);
    r.set_first_player(0);
    let theirs = r.place_field_stack(1, &["PLAIN"], false, 0);
    assert_eq!(r.turn_player(), 0);
    let result = r.game.begin_attack_open(AttackOpen {
        attacker: theirs,
        initiator: AttackInitiator::Effect { source: None, optional: false },
        suspend_attacker: true,
        ignore_summoning_sickness: true,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    assert_eq!(result, AttackResult::Invalid, "CR 11-1-2: only the turn player can attack");
    assert!(r.game.pending_attack.is_none());
}

/// An Option's [Main] body must not also be an [On Play]. `timing_flag_matches`
/// dispatches OnPlay on the `on_play` flag, and Option bodies used to be built
/// from `Effect::on_play(..)`, so a played DUAL card also ran its Option
/// [Main] as an [On Play] (game audit 2026-11-11). Checks every lowered card.
#[test]
fn no_lowered_option_main_body_carries_the_on_play_flag() {
    use digimon_engine::{card_source::CardHandle, dsl_cards::DslCardEffect, effect::CardEffect};
    let pack = digimon_engine::dsl_registry::from_embedded_cached().expect("embedded pack loads");
    let mut checked = 0;
    let mut offenders = Vec::new();
    for (id, card) in pack.iter() {
        let effects = DslCardEffect::new(std::sync::Arc::new(card.clone())).effects(CardHandle(0));
        for e in effects.iter().filter(|e| e.timing == EffectTiming::OptionMain) {
            checked += 1;
            if e.on_play {
                offenders.push(id.clone());
            }
        }
    }
    assert!(checked > 100, "expected many Option [Main] bodies, saw {checked}");
    assert!(offenders.is_empty(), "Option [Main] bodies flagged on_play: {offenders:?}");
}

/// A printed [Burst Digivolve] is a Main-phase digivolve, not <Blast Digivolve>
/// ([Hand] [Counter]), so it must not carry the counter-window marker. Rose
/// Rave audit 2026-11-11: Rosemon: Burst Mode (BT26-050) was digivolved into
/// for free during the opponent's attack 13 times in 68 games.
#[test]
fn burst_digivolve_cards_are_not_blast_digivolve_counters() {
    use digimon_engine::{card_source::CardHandle, dsl_cards::DslCardEffect, effect::CardEffect};
    let pack = digimon_engine::dsl_registry::from_embedded_cached().expect("embedded pack loads");
    let has_marker = |id: &str| {
        let card = pack.lookup(id).unwrap_or_else(|| panic!("{id} in pack"));
        DslCardEffect::new(std::sync::Arc::new(card.clone()))
            .effects(CardHandle(0))
            .iter()
            .any(|e| e.blast_digivolve)
    };
    for id in ["BT13-020", "BT13-060", "BT25-104", "BT26-050"] {
        assert!(!has_marker(id), "{id} prints [Burst Digivolve], not a [Counter]");
    }
    assert!(has_marker("AD1-005"), "AD1-005 prints <Blast Digivolve>");
}
