//! BT19-024 MarineBullmon — Lv.5 Blue, Mollusk / LIBERATOR / Aquatic ((Rule) trait).
//!
//! <Decode (Blue Lv.4)>
//! [On Play] [When Digivolving] You may place 1 Digimon card with [Aqua] or
//! [Sea Animal] in any of its traits from your hand as 1 of your Digimon's
//! bottom digivolution card.
//! Inherited: [End of Attack] [Once Per Turn] You may play 1 level 4 or lower
//! Digimon card with [Aqua] or [Sea Animal] in any of its traits from this
//! Digimon's digivolution cards without paying the cost.
//!
//! DCGO: BT19/Blue/BT19_024.cs. Synthetic boards only (no recorded games).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT19-024";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-024 in embedded DSL pack")
        .add_card(filler("FILL"))
        .add_card(digimon("BLUE4", "Blue Four", CardColor::Blue, 4, 5, &[]))
        .add_card(digimon("RED4", "Red Four", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("AQUA3", "Aqua Three", CardColor::Blue, 3, 3, &["Aqua"]))
        .add_card(digimon("AQUATIC4", "Aquatic Four", CardColor::Blue, 4, 5, &["Aquatic"]))
        .add_card(digimon("SEA4", "Sea Four", CardColor::Green, 4, 5, &["Sea Animal"]))
        .add_card(digimon("SEA5", "Sea Five", CardColor::Blue, 5, 7, &["Sea Animal"]))
        .add_card(digimon("BEAST4", "Beast Four", CardColor::Blue, 4, 5, &["Beast"]))
        .add_card(digimon("GREEN4", "Green Four", CardColor::Green, 4, 5, &[]))
        .add_card(digimon("MYMON", "My Mon", CardColor::Blue, 3, 3, &[]))
        .add_card(tamer("AQUA-TAMER", "Aqua Tamer", CardColor::Blue, &["Aqua"]))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(1, &["FILL"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 3;
    r
}

fn accept(r: &mut DebugRunner) {
    let a = non_pass(r)[0];
    let p = r.pending_selection().expect("pending").selecting_player;
    r.execute_action(p, a).unwrap();
}

// ─── Structural / identity ───────────────────────────────────────────────────

#[test]
fn bt19_024_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "MarineBullmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in ["Mollusk", "LIBERATOR", "Aquatic"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let op_wd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD clause");
    assert_eq!(op_wd.when.len(), 2);
    assert!(op_wd.optional && op_wd.outer_prompt && !op_wd.once_per_turn);
    let eoa = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfAttack))
        .expect("inherited End of Attack");
    assert!(eoa.optional && eoa.outer_prompt && eoa.once_per_turn);
}

#[test]
fn bt19_024_digivolves_from_blue_lv4_for_3_only() {
    for (base_id, ok) in [("BLUE4", true), ("RED4", false)] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        let idx = r.add_to_hand(0, CARD_ID);
        let done = r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve);
        assert_eq!(done, ok, "{base_id}");
        if ok {
            assert_eq!(top_id(&r, base), CARD_ID);
            assert_eq!(r.memory(), 10 - 3);
        }
    }
}

// ─── [On Play] [When Digivolving] ────────────────────────────────────────────

#[test]
fn bt19_024_not_offered_without_an_aqua_or_sea_animal_digimon_in_hand() {
    let mut r = setup();
    push_hand(&mut r, 0, "BEAST4");
    push_hand(&mut r, 0, "AQUA-TAMER");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt19_024_places_a_hand_card_as_bottom_source_of_a_chosen_own_digimon() {
    let mut r = setup();
    for id in ["AQUA3", "AQUATIC4", "SEA4", "BEAST4", "AQUA-TAMER"] {
        push_hand(&mut r, 0, id);
    }
    let mon = r.place_stack(0, &["FILL", "MYMON"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    assert!(r.pending_is_optional(), "outer 'you may' prompt");
    accept(&mut r);
    assert!(r.pending_is_optional(), "the hand pick is declinable (canNoSelect)");
    // [Aqua] / [Aquatic] / [Sea Animal] Digimon cards only.
    assert_eq!(non_pass(&r).len(), 3);
    pick_hand(&mut r, 0, "AQUATIC4");
    let v = r.pending_selection_view().expect("own Digimon pick");
    assert!(!v.is_optional, "the Digimon pick is mandatory (canNoSelect false)");
    assert_eq!(non_pass(&r).len(), 2, "MYMON and MarineBullmon itself");
    pick_side_field(&mut r, 0, mon);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, mon), vec!["AQUATIC4".to_string(), "FILL".to_string()]);
    assert!(!hand_ids(&r, 0).contains(&"AQUATIC4".to_string()));
}

#[test]
fn bt19_024_can_place_under_itself_and_is_declinable_at_both_prompts() {
    let mut r = setup();
    push_hand(&mut r, 0, "AQUA3");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    accept(&mut r);
    pick_hand(&mut r, 0, "AQUA3");
    pick_side_field(&mut r, 0, me);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, me), vec!["AQUA3".to_string()]);

    for decline_outer in [true, false] {
        let mut r = setup();
        push_hand(&mut r, 0, "AQUA3");
        let me = r.place_on_field(0, CARD_ID, Some(0));
        fire(&mut r, EffectTiming::WhenDigivolving, me);
        if !decline_outer {
            accept(&mut r);
        }
        pass(&mut r, 0);
        let _ = r.auto_resolve();
        assert!(hand_ids(&r, 0).contains(&"AQUA3".to_string()));
        assert_eq!(sources(&r, me), 0);
    }
}

// ─── <Decode (Blue Lv.4)> ────────────────────────────────────────────────────

#[test]
fn bt19_024_decode_plays_a_blue_level_4_source_when_leaving_outside_battle() {
    let mut r = setup();
    // Sources: blue Lv.4 (legal), red Lv.4, green Lv.4, blue Lv.5.
    let h = r.place_stack(0, &["SEA5", "GREEN4", "RED4", "BLUE4", CARD_ID]);
    r.game.return_to_hand_from_effect(h, 1);
    let accept_prompt = r.pending_selection_view().expect("Decode outer prompt");
    assert_eq!(accept_prompt.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    let pick = r.pending_selection_view().expect("Decode source pick");
    assert_eq!(pick.valid_action_ids.len(), 1, "only the blue level 4 source");
    r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["BLUE4".to_string()]);
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()), "the leave still happened");
}

// ─── Inherited [End of Attack] [Once Per Turn] ───────────────────────────────

fn carrier_with_sources(sources: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = setup();
    let mut stack: Vec<&str> = sources.to_vec();
    stack.push(CARD_ID);
    stack.push("BLUE4");
    let h = r.place_stack(0, &stack);
    (r, h)
}

#[test]
fn bt19_024_inherited_end_of_attack_plays_a_lv4_or_lower_aqua_source_free() {
    // Sources under the carrier: Lv.3 [Aqua], Lv.4 [Sea Animal], Lv.5 [Sea
    // Animal] (too high), Lv.4 [Beast] (no trait), and MarineBullmon itself
    // (Lv.5 [Aquatic]: too high).
    let (mut r, h) = carrier_with_sources(&["SEA5", "BEAST4", "SEA4", "AQUA3"]);
    fire(&mut r, EffectTiming::EndOfAttack, h);
    assert!(r.pending_is_optional(), "outer 'you may' prompt");
    accept(&mut r);
    assert!(r.pending_is_optional(), "the pick is declinable (canNoSelect)");
    assert_eq!(non_pass(&r).len(), 2, "Lv.3 [Aqua] and Lv.4 [Sea Animal] only");
    let before_memory = r.memory();
    // Pick the first legal source.
    let first = non_pass(&r)[0];
    r.execute_action(0, first).unwrap();
    let _ = r.auto_resolve();
    let on_field = field_ids(&r, 0);
    assert_eq!(on_field.len(), 2, "the carrier plus the played source");
    assert_eq!(r.memory(), before_memory, "without paying the cost");
}

#[test]
fn bt19_024_inherited_end_of_attack_not_offered_without_a_candidate_and_is_opt() {
    let (mut r, h) = carrier_with_sources(&["SEA5", "BEAST4"]);
    fire(&mut r, EffectTiming::EndOfAttack, h);
    assert!(r.pending_selection().is_none(), "no legal source: not activatable");

    let (mut r, h) = carrier_with_sources(&["SEA4", "AQUA3"]);
    fire(&mut r, EffectTiming::EndOfAttack, h);
    accept(&mut r);
    accept(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0).len(), 2);
    fire(&mut r, EffectTiming::EndOfAttack, h);
    assert!(r.pending_selection().is_none(), "[Once Per Turn]: a second candidate remains but the use is spent");
}

#[test]
fn bt19_024_inherited_end_of_attack_declining_the_outer_prompt_keeps_the_use() {
    let (mut r, h) = carrier_with_sources(&["AQUA3"]);
    fire(&mut r, EffectTiming::EndOfAttack, h);
    pass(&mut r, 0);
    assert_eq!(field_ids(&r, 0).len(), 1);
    fire(&mut r, EffectTiming::EndOfAttack, h);
    assert!(r.pending_selection().is_some(), "declining preserved the once-per-turn use");
}

#[test]
fn bt19_024_face_up_has_no_inherited_end_of_attack() {
    let mut r = setup();
    let me = r.place_stack(0, &["AQUA3", CARD_ID]);
    fire(&mut r, EffectTiming::EndOfAttack, me);
    assert!(r.pending_selection().is_none());
}
