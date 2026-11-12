//! `dna_field_pair` (G-DSL-DNA-FIELD-PAIR-CANDIDATE): "2 of your Digimon may DNA
//! digivolve into X in the hand" offers only a result card, a first Digimon and
//! a second Digimon that together satisfy one of X's printed DNA requirements.
//! DCGO `CanPlayJogress(true)` / `CanJogressFromTargetPermanents`; first used by
//! WarGreymon (EX12-017) [Counter] (deck audit 2026-11-10).
use digimon_engine::{
    action::{mask::build_action_mask, space::{encode_attack, PASS}},
    card_data::CardData,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    enums::{CardKind, EffectTiming},
    selection::TriggerSource,
};

const YAML: &str = r#"
card: PAIR-PICKER
name: Pair picker
kind: tamer
color: [white]
cost: 1
effects:
  - when: main_on_field
    summary: "2 of your Digimon may DNA digivolve into a card in hand"
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            kind: digimon
            dna_field_pair: { role: result }
          prompt: "Choose result"
      - if:
          condition: { binding_present: result }
          then:
            - select_dna_pair:
                left_filter:
                  kind: digimon
                  of: you
                  dna_field_pair: { role: first, result: result }
                right_filter:
                  kind: digimon
                  of: you
                  dna_field_pair: { role: second, result: result, first: first }
                bind_left_as: first
                bind_right_as: second
                optional: true
                prompt: "Choose 2 Digimon"
            - effect_initiated_dna_digivolve:
                target_a: first
                target_b: second
                from_hand: result
                cost: printed
                ignore_requirements: false
"#;

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c
}

/// Field: [PAIR-PICKER, L6, L5, L4]. Hand: [DNA-GOOD (Lv.6 + Lv.5, cost 3),
/// DNA-FAR (Lv.7 + Lv.7, no pair), PLAIN (no DNA)].
fn setup() -> (DebugRunner, [u16; 3]) {
    setup_with_hand(&["DNA-GOOD", "DNA-FAR", "PLAIN"])
}

fn setup_with_hand(hand: &[&str]) -> (DebugRunner, [u16; 3]) {
    let mut source = make_test_card("PAIR-PICKER", "Pair picker");
    source.card_kind = CardKind::Tamer;
    source.level = None;
    source.dp = None;
    let mut good = make_test_dna_card("DNA-GOOD", "Good result", 6, 5, 3);
    good.card_kind = CardKind::Digimon;
    good.level = Some(7);
    let mut far = make_test_dna_card("DNA-FAR", "Far result", 7, 7, 0);
    far.card_kind = CardKind::Digimon;
    far.level = Some(8);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(source)
        .add_card(good)
        .add_card(far)
        .add_card(digimon("PLAIN", 7))
        .add_card(digimon("L6", 6))
        .add_card(digimon("L5", 5))
        .add_card(digimon("L4", 4))
        .hand(0, hand)
        .deck(0, &["L4"; 8])
        .memory(5)
        .start();
    r.skip_mulligan();
    let source = r.place_on_field(0, "PAIR-PICKER", None);
    let l6 = r.place_on_field(0, "L6", None);
    let l5 = r.place_on_field(0, "L5", None);
    let l4 = r.place_on_field(0, "L4", None);
    r.game.enter_main_phase();
    r.game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
    let act = |h: digimon_engine::permanent::PermanentHandle| encode_attack(0, h.index as u16);
    (r, [act(l6), act(l5), act(l4)])
}

#[test]
fn result_choice_offers_only_a_card_with_a_legal_field_pair() {
    let (r, _) = setup();
    let view = r.pending_selection_view().expect("result pick");
    assert_eq!(view.valid_action_ids, vec![0], "only DNA-GOOD (hand slot 0)");
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.0, "may decline");
}

#[test]
fn first_and_second_choices_are_correlated_with_the_result_and_each_other() {
    let (mut r, [l6, l5, l4]) = setup();
    r.execute_action(0, 0).unwrap();
    let mut left = r.pending_selection_view().expect("first material").valid_action_ids;
    left.sort();
    let mut want = vec![l6, l5];
    want.sort();
    assert_eq!(left, want, "the Lv.4 Digimon is not a material");
    assert!(!left.contains(&l4));

    r.execute_action(0, l6).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        let right = game.pending_selection.as_ref().unwrap().valid_action_ids.clone();
        assert!(right.contains(&l5), "Lv.5 completes the Lv.6 + Lv.5 recipe");
        assert!(!right.contains(&l4) && !right.contains(&l6));
        game.resolve_selection(0, l5).unwrap();
        assert!(game.pending_selection.is_none());
        assert!(game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&game.card_data) == "DNA-GOOD"));
    }
    assert_eq!(r.game.verification_digest(), cloned.verification_digest());
}

#[test]
fn no_prompt_when_no_hand_card_has_a_legal_pair() {
    let (r, _) = setup_with_hand(&["DNA-FAR", "PLAIN"]);
    assert!(
        r.pending_selection().is_none(),
        "an optional hand pick with no legal candidate opens nothing"
    );
}
