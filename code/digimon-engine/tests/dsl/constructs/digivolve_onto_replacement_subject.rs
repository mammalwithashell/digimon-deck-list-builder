//! `can_digivolve_onto: replacement_subject` inside a replacement's
//! `choose.card_filter` (G-DSL-CAN-DIGIVOLVE-ONTO-REPLACEMENT-SUBJECT).
//!
//! Return to the Primogenitor (BT17-097) <Delay>: "By digivolving that Digimon
//! into a Digimon card with [Imperialdramon] in its name in your hand without
//! paying the cost, prevent that deletion." DCGO offers only hand cards that
//! can digivolve onto the would-be-deleted Digimon (BT17_097.cs:137
//! `CanPlayCardTargetFrame(permanent.PermanentFrame, false, activateClass)`,
//! and `DigivolveIntoHandOrTrashCard` with
//! `ignoreDigivolutionRequirementFixedCost: -1`, i.e. requirements apply).
//! The Delay hand-digivolve flow now evaluates the choose filter with the
//! replacement's bindings, so the filter can name the subject. A card that
//! can't digivolve onto it would be a choice with no legal follow-up (CR 4.3
//! 1-3-2).
use digimon_engine::{
    action::space::HAND_EFFECT_START,
    card_data::{CardData, EvoCost},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, DelayTrigger},
    permanent::{OptionState, PermanentHandle},
    replacement::ReplacementCause,
};

const DELAY: &str = r#"
card: DELAY-DIGI-SOURCE
name: Delay digivolve source
kind: option
color: [blue]
cost: 2
effects:
  - kind: replacement
    timing: when_would_be_deleted
    active_when:
      all_of:
        - all_turns: true
        - trait_has: "Free"
        - replacement_subject_is_mine: true
    summary: "Delay: digivolve the would-be-deleted Digimon into an [Imperialdramon] card in hand to prevent the deletion"
    cost:
      delay_self: true
    choose:
      from: hand
      card_filter:
        all_of:
          - name_contains: "Imperialdramon"
          - can_digivolve_onto: replacement_subject
      min: 1
      max: 1
    outcome: prevent
    then:
      - digivolve_without_cost:
          target: replacement_subject
          card: chosen
"#;

fn digimon(id: &str, name: &str, level: u8, from_level: Option<u8>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Free".into()];
    if let Some(from) = from_level {
        c.evo_costs = vec![EvoCost { card_color: CardColor::Red as u8, level: from, memory_cost: 3 }];
    }
    c
}

/// Field: the Lv.4 [Free] target and the Delay Option. Hand: `hand`.
fn fixture(yaml: &str, hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon("FREE-TARGET", "Free Target", 4, None))
        // Lv.5 with a Lv.4 red digivolution requirement: can digivolve onto it.
        .add_card(digimon("IMP-OK", "Imperialdramon OK", 5, Some(4)))
        // Lv.7 needing a Lv.6: can't.
        .add_card(digimon("IMP-BAD", "Imperialdramon Bad", 7, Some(6)))
        .hand(0, hand)
        .memory(0)
        .start();
    let target = r.place_on_field(0, "FREE-TARGET", Some(0));
    let delay = r.place_on_field(0, "DELAY-DIGI-SOURCE", Some(0));
    r.game.player_mut(0).battle_area[delay.index as usize].option_state = OptionState::Delayed {
        owner: 0,
        trash_on_turn: r.game.turn_count + 1,
        trigger: DelayTrigger::EndOfYourNextTurn,
        // CR 4.3 16-17-3 (<Delay>): placed on an earlier turn, so usable now.
        placed_on_turn: r.game.turn_count.saturating_sub(1),
    };
    (r, target)
}

fn offered_ids(r: &DebugRunner) -> Vec<String> {
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    let mut ids: Vec<String> = pending
        .effect_choices
        .as_ref()
        .expect("hand choice rows")
        .iter()
        .filter_map(|row| row.source_card)
        .filter_map(|card| r.game.card_data_for_handle(card).map(|d| d.card_id.clone()))
        .collect();
    ids.sort();
    ids
}

fn top_of_target(r: &DebugRunner) -> Option<String> {
    r.game.players[0].battle_area.iter().find_map(|p| {
        p.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "FREE-TARGET")
            .then(|| p.top_card().card_id(&r.game.card_data).to_string())
    })
}

#[test]
fn choose_offers_only_cards_that_can_digivolve_onto_the_subject() {
    let (mut r, target) = fixture(DELAY, &["IMP-BAD", "IMP-OK"]);
    r.game.delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert_eq!(offered_ids(&r), vec!["IMP-OK".to_string()]);
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert_eq!(top_of_target(&r).as_deref(), Some("IMP-OK"), "digivolved, deletion prevented");
}

#[test]
fn without_the_leaf_the_unroutable_card_is_offered() {
    // Control: the name filter alone offers the Lv.7 that can't digivolve
    // onto a Lv.4 (the compiled replacement digivolve ignores requirements,
    // so only the filter can keep it out).
    let yaml = DELAY.replace("          - can_digivolve_onto: replacement_subject\n", "");
    let (mut r, target) = fixture(&yaml, &["IMP-BAD", "IMP-OK"]);
    r.game.delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert_eq!(offered_ids(&r), vec!["IMP-BAD".to_string(), "IMP-OK".to_string()]);
}

#[test]
fn no_choice_and_the_deletion_stands_when_no_card_can_digivolve_onto_it() {
    let (mut r, target) = fixture(DELAY, &["IMP-BAD"]);
    r.game.delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_none());
    assert!(top_of_target(&r).is_none(), "the target was deleted");
}
