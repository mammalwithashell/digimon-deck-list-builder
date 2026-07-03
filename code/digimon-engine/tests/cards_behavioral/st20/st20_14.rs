//! ST20-14 Our Courage United — Option, Black, Cost 3. Traits: ADVENTURE.
//! Full annotated header (card text, DCGO crosscheck, pattern tags, known
//! gaps, and the subject-handle-index-shift DEBUG NOTE) is kept verbatim on
//! disk at code/digimon-engine/tests/cards_behavioral/st20/st20_14.rs.
//! Summary: card-level use_requirement color bypass; [Main] mandatory
//! Draw 2 + place_self_as_delay_option; [All Turns] kind:replacement
//! Delay leave-watcher (RK-G003 delete_permanent{target:source} cost +
//! RK-G004 non-cancelling optional select_hand/play_from_hand_free reward);
//! inherited [Security] place_self_as_delay_option. See gaps field for the
//! one engine issue discovered (G-REPLACEMENT-SUBJECT-STALE-AFTER-SOURCE-DELETE),
//! which does not block this card and is pinned by a dedicated regression
//! test (st20_14_engine_gap_subject_handle_stale_when_source_deleted_at_lower_index).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, TRASH_EFFECT_START,
};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_adventure_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "AdventureMon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn make_adventure_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "AdventureTamer");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 2;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn make_non_adventure_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "PlainMon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn make_l5_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Black];
    c
}

fn make_l4_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "L4Mon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Black];
    c
}

fn make_adventure_reward_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, "AdventureReward");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn make_adventure_l6_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "AdventureTooHigh");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 10;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn make_non_adventure_reward_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "NonAdventureReward");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 must be in embedded DSL pack")
        .memory(10)
        .start()
}

fn field_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == card_id)
    })
}

fn seat_as_delay_option(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    use digimon_engine::permanent::OptionState;
    let turn = runner.game.turn_count;
    let perm =
        &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

#[test]
fn st20_14_identity_matches_printed_card() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    assert_eq!(card.name, "Our Courage United");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(
        card.traits.iter().any(|t| t == "ADVENTURE"),
        "must carry the ADVENTURE trait, got {:?}",
        card.traits
    );
}

#[test]
fn st20_14_has_three_clauses() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    assert_eq!(
        card.effects.len(),
        3,
        "expected 3 compiled clauses (Main, replacement, Security); got {}",
        card.effects.len()
    );
}

#[test]
fn st20_14_has_card_level_use_requirement() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    assert!(
        card.use_requirement.is_some(),
        "ST20-14 must declare a card-level use_requirement for its color bypass"
    );
}

#[test]
fn st20_14_main_clause_is_mandatory_face_up() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    let main_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause with MainFromHand timing must exist");

    assert!(
        !main_clause.optional,
        "ST20-14 [Main] must be mandatory once played (printed text has no 'you may')"
    );
    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "ST20-14 Main clause must have FaceUp scope (Option played from hand)"
    );
}

#[test]
fn st20_14_main_clause_draws_two_and_places_self_as_delay_option() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    let main_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause exists");

    assert!(
        main_clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::Draw { count: 2, .. })),
        "Main clause must draw 2 cards"
    );
    assert!(
        main_clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)),
        "Main clause must place self as a Delay-Option permanent"
    );
}

#[test]
fn st20_14_has_replacement_clause_for_when_would_leave_battle_area() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    let has_leave_replacement = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                ..
            }) if trigger == "when_would_leave_battle_area"
        )
    });
    assert!(
        has_leave_replacement,
        "ST20-14 must have a declarative when_would_leave_battle_area replacement clause"
    );
}

#[test]
fn st20_14_replacement_clause_has_no_opt_flag() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    let once_per_turn = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            trigger,
            once_per_turn,
            ..
        }) if trigger == "when_would_leave_battle_area" => Some(*once_per_turn),
        _ => None,
    });
    assert_eq!(
        once_per_turn,
        Some(false),
        "ST20-14's [All Turns] Delay leave-watcher has no [OPT] tag in printed text"
    );
}

#[test]
fn st20_14_security_clause_is_inherited_and_mandatory() {
    let r = runner();
    let card = r.compiled_card("ST20-14").expect("ST20-14 present");
    let sec_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("Security clause with OnSecurity timing must exist");

    assert_eq!(
        sec_clause.scope,
        CompiledScope::Inherited,
        "ST20-14 Security clause must have Inherited scope"
    );
    assert!(
        !sec_clause.optional,
        "printed '[Security] Place this card in the battle area' has no 'you may'"
    );
    assert!(
        sec_clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)),
        "Security clause must place self as a Delay-Option permanent"
    );
}

#[test]
fn st20_14_color_bypass_active_with_own_adventure_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(make_adventure_digimon("ADV-DIGI"))
        .memory(10)
        .hand(0, &["ST20-14"])
        .start();

    r.place_on_field(0, "ADV-DIGI", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "ST20-14 (Black) must be playable while an own Red ADVENTURE Digimon is on field \
         (color bypass, not a natural color match)"
    );
}

#[test]
fn st20_14_color_bypass_active_with_own_adventure_tamer() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(make_adventure_tamer("ADV-TAMER"))
        .memory(10)
        .hand(0, &["ST20-14"])
        .start();

    r.place_on_field(0, "ADV-TAMER", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "ST20-14 (Black) must be playable while an own Red ADVENTURE Tamer is on field"
    );
}

#[test]
fn st20_14_color_bypass_inactive_without_adventure_permanent() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(make_non_adventure_digimon("PLAIN-DIGI"))
        .memory(10)
        .hand(0, &["ST20-14"])
        .start();

    r.place_on_field(0, "PLAIN-DIGI", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "ST20-14 must NOT be playable without an own ADVENTURE Digimon/Tamer or a color match"
    );
}

#[test]
fn st20_14_color_bypass_inactive_for_opponent_adventure_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(make_adventure_digimon("OPP-ADV-DIGI"))
        .memory(10)
        .hand(0, &["ST20-14"])
        .start();

    r.place_on_field(1, "OPP-ADV-DIGI", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "opponent's ADVENTURE Digimon must not satisfy ST20-14's own-field color-bypass gate"
    );
}

#[test]
fn st20_14_main_draws_two_and_places_self_in_battle_area() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["ST20-14"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let hand_before = r.hand_size(0);
    let deck_before = r.game.players[0].deck.len();
    let battle_before = r.battle_area_size(0);

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must fire ST20-14's [Main]");
    r.game.drain_effect_queue();

    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 2,
        "2 cards must be drawn from the deck"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "net hand size: -1 (ST20-14 played) + 2 (drawn) = +1"
    );
    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "ST20-14 must be placed on the battle area, not trashed"
    );
    assert!(
        field_contains(&r, 0, "ST20-14"),
        "ST20-14 itself must be the permanent placed on the battle area"
    );
    assert_eq!(
        r.trash_size(0),
        0,
        "ST20-14 must NOT be trashed by the standard Option-disposal path"
    );
}

#[test]
fn st20_14_replacement_installs_for_own_level5_digimon_leaving() {
    let l5 = make_l5_digimon("L5-SUBJECT", "L5Subject");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .memory(10)
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let subject = r.place_on_field(0, "L5-SUBJECT", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("ST20-14's optional Delay response must install");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(
        pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "replacement prompt must offer ACCEPT"
    );
    assert!(
        pending.is_optional,
        "replacement prompt must be optional (PASS legal via is_optional, \
         not listed in valid_action_ids for this selection kind)"
    );
}

#[test]
fn st20_14_replacement_does_not_fire_for_own_level4_digimon() {
    let l4 = make_l4_digimon("L4-SUBJECT");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l4)
        .memory(10)
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let subject = r.place_on_field(0, "L4-SUBJECT", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "level-4 leaving Digimon must NOT trigger ST20-14's level-5+ gate"
    );
    assert!(
        field_contains(&r, 0, "ST20-14"),
        "ST20-14 must remain on field (Delay cost not paid)"
    );
}

#[test]
fn st20_14_replacement_does_not_fire_for_opponent_level5_digimon() {
    let opp_l5 = make_l5_digimon("OPP-L5", "OppL5");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(opp_l5)
        .memory(10)
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let opp_subject = r.place_on_field(1, "OPP-L5", None);

    r.game
        .delete_permanent_with_cause(opp_subject, ReplacementCause::OwnEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "opponent's level-5+ Digimon leaving must NOT trigger ST20-14 (subject must be mine)"
    );
    assert!(
        field_contains(&r, 0, "ST20-14"),
        "ST20-14 must remain on field (Delay cost not paid)"
    );
}

#[test]
fn st20_14_replacement_fires_even_when_not_seated_as_delay_option_documented_limitation() {
    let l5 = make_l5_digimon("L5-STD", "L5Std");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .memory(10)
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    r.place_on_field(0, "ST20-14", Some(0));
    let subject = r.place_on_field(0, "L5-STD", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    assert!(
        r.game.pending_selection.is_some(),
        "documented current behavior: the generic replacement fallback has no \
         source_is_delayed_option gate, so the clause fires even from a Standard \
         seating — a state ST20-14 cannot actually reach in real play"
    );
}

#[test]
fn st20_14_replacement_accept_plays_adventure_reward_and_pays_delay_cost() {
    let l5 = make_l5_digimon("L5-ACCEPT", "L5Accept");
    let reward = make_adventure_reward_digimon("REWARD-ADV", 4);
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .add_card(reward)
        .memory(10)
        .hand(0, &["REWARD-ADV"])
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let subject = r.place_on_field(0, "L5-ACCEPT", Some(0));
    let st20_14 = r.place_on_field(0, "ST20-14", None);
    seat_as_delay_option(&mut r, st20_14);
    let memory_before = r.memory();

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional replacement prompt installs");
    let action = pending.valid_action_ids[0];
    assert_eq!(action, REPLACEMENT_ACCEPT);
    r.game
        .resolve_selection(0, action)
        .expect("accept the Delay response");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("accepting asks which ADVENTURE Digimon to play");
    assert_eq!(pending.kind, SelectionKind::Hand);
    assert!(
        pending.is_optional,
        "the hand pick is optional ('you may play') via is_optional"
    );
    let hand_action = pending
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a legal hand-card action exists");

    r.game
        .resolve_selection(0, hand_action)
        .expect("play the ADVENTURE reward from hand");
    r.game.drain_effect_queue();

    assert!(
        field_contains(&r, 0, "REWARD-ADV"),
        "the selected ADVENTURE reward Digimon must be played to the battle area"
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "the reward is played WITHOUT paying its cost (memory unchanged by the play)"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ST20-14"),
        "ST20-14 itself must be trashed (Delay cost paid)"
    );
    assert!(
        !field_contains(&r, 0, "L5-ACCEPT"),
        "the leaving level-5 Digimon must still leave the battle area (no cancel)"
    );
}

#[test]
fn st20_14_engine_gap_subject_handle_stale_when_source_deleted_at_lower_index() {
    let l5 = make_l5_digimon("L5-GAPCASE", "L5GapCase");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .memory(10)
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let subject = r.place_on_field(0, "L5-GAPCASE", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("the outer Delay-accept prompt installs");
    let action = pending.valid_action_ids[0];
    r.game
        .resolve_selection(0, action)
        .expect("accept the Delay response");
    r.game.drain_effect_queue();

    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ST20-14"),
        "ST20-14's Delay cost is still paid regardless of the ordering hazard"
    );
    assert!(
        field_contains(&r, 0, "L5-GAPCASE"),
        "KNOWN ENGINE GAP: the leaving subject incorrectly remains on field \
         because its ReplacementSubject handle went stale after ST20-14's \
         lower-index self-delete shifted battle_area indices. See \
         G-REPLACEMENT-SUBJECT-STALE-AFTER-SOURCE-DELETE in \
         docs/RUST_ENGINE_GAPS.md. If this assertion starts failing, the gap \
         has been fixed — update this test and the tracker."
    );
}

#[test]
fn st20_14_replacement_decline_leaves_st20_14_on_field_and_subject_leaves() {
    let l5 = make_l5_digimon("L5-DECLINE", "L5Decline");
    let reward = make_adventure_reward_digimon("REWARD-DECLINE", 4);
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .add_card(reward)
        .memory(10)
        .hand(0, &["REWARD-DECLINE"])
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let subject = r.place_on_field(0, "L5-DECLINE", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    r.game
        .resolve_selection(0, PASS)
        .expect("decline the Delay response");

    assert!(
        !field_contains(&r, 0, "L5-DECLINE"),
        "the leaving level-5 Digimon must still leave the battle area"
    );
    assert!(
        field_contains(&r, 0, "ST20-14"),
        "ST20-14 remains on the field — Delay cost was not paid on decline"
    );
    assert!(
        !field_contains(&r, 0, "REWARD-DECLINE"),
        "declining must not play the reward Digimon"
    );
    assert_eq!(
        r.hand_size(0),
        1,
        "the reward Digimon stays in hand after a decline"
    );
}

#[test]
fn st20_14_replacement_accept_with_no_eligible_reward_still_pays_delay_cost() {
    let l5 = make_l5_digimon("L5-NOREWARD", "L5NoReward");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .memory(10)
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let subject = r.place_on_field(0, "L5-NOREWARD", Some(0));
    let st20_14 = r.place_on_field(0, "ST20-14", None);
    seat_as_delay_option(&mut r, st20_14);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("the outer Delay-accept prompt installs regardless of hand eligibility");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    let action = pending.valid_action_ids[0];
    assert_eq!(action, REPLACEMENT_ACCEPT);

    r.game
        .resolve_selection(0, action)
        .expect("accept the Delay response");
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "with zero eligible reward candidates, select_hand silently no-ops \
         (no further prompt installs)"
    );
    assert!(
        !field_contains(&r, 0, "L5-NOREWARD"),
        "the leaving level-5 Digimon must still leave"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ST20-14"),
        "ST20-14 must still be trashed — the Delay cost is paid on accept \
         even when no reward is available"
    );
}

#[test]
fn st20_14_replacement_accept_does_not_offer_non_adventure_hand_digimon() {
    let l5 = make_l5_digimon("L5-NONADV", "L5NonAdv");
    let non_adv = make_non_adventure_reward_digimon("NON-ADV-REWARD");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .add_card(non_adv)
        .memory(10)
        .hand(0, &["NON-ADV-REWARD"])
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let subject = r.place_on_field(0, "L5-NONADV", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("the outer Delay-accept prompt installs regardless of hand eligibility");
    let action = pending.valid_action_ids[0];
    r.game
        .resolve_selection(0, action)
        .expect("accept the Delay response");
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "a non-ADVENTURE hand Digimon must not be offered — select_hand no-ops with zero matches"
    );
    assert!(
        !field_contains(&r, 0, "NON-ADV-REWARD"),
        "the non-ADVENTURE card must not be played"
    );
    assert_eq!(
        r.hand_size(0),
        1,
        "the non-ADVENTURE card remains in hand (never an eligible candidate)"
    );
}

#[test]
fn st20_14_replacement_accept_does_not_offer_level6_adventure_hand_digimon() {
    let l5 = make_l5_digimon("L5-TOOHIGH", "L5TooHigh");
    let l6_adv = make_adventure_l6_digimon("L6-ADV-REWARD");
    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(l5)
        .add_card(l6_adv)
        .memory(10)
        .hand(0, &["L6-ADV-REWARD"])
        .deck(0, &["ST20-14"; 3])
        .deck(1, &["ST20-14"; 3])
        .start();

    let st20_14 = r.place_on_field(0, "ST20-14", Some(0));
    seat_as_delay_option(&mut r, st20_14);
    let subject = r.place_on_field(0, "L5-TOOHIGH", None);

    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("the outer Delay-accept prompt installs regardless of hand eligibility");
    let action = pending.valid_action_ids[0];
    r.game
        .resolve_selection(0, action)
        .expect("accept the Delay response");
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "a level-6 ADVENTURE hand Digimon exceeds 'level 5 or lower' — select_hand no-ops"
    );
    assert!(
        !field_contains(&r, 0, "L6-ADV-REWARD"),
        "the level-6 card must not be played"
    );
    assert_eq!(
        r.hand_size(0),
        1,
        "the level-6 card remains in hand (never an eligible candidate)"
    );
}

#[test]
fn st20_14_security_places_self_in_battle_area() {
    let mut attacker = make_filler("ST2014-ATK");
    attacker.dp = Some(9000);

    let mut r = DebugRunner::builder()
        .dsl_card("ST20-14")
        .expect("ST20-14 loads")
        .add_card(attacker.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["ST20-14"])
        .start();

    let attacker_handle = r.place_on_field(0, "ST2014-ATK", Some(0));
    assert_eq!(r.security_count(1), 1, "precondition: ST20-14 in security");
    assert_eq!(r.battle_area_size(1), 0, "precondition: empty battle area");

    let _ = r.attack_player(attacker_handle, 1, false);
    r.auto_resolve().expect("security resolution completes");

    assert_eq!(
        r.security_count(1),
        0,
        "ST20-14 must leave the defender's security stack"
    );
    assert!(
        field_contains(&r, 1, "ST20-14"),
        "ST20-14 must be placed on the defender's battle area, not the trash"
    );
    assert_eq!(
        r.trash_size(1),
        0,
        "ST20-14 must NOT be trashed by the security-check disposal path"
    );
}