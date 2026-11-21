//! EX11-018 Ryugumon — Lv.6 Blue, Mollusk / LIBERATOR / Aquatic ((Rule) trait).
//!
//! <Evade> <Decode (Lv.5 or lower w/[Aqua]/[Sea Animal] in any trait)>
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By placing 1
//! Digimon card with [Aqua] or [Sea Animal] in any of its traits from your hand
//! as this Digimon's bottom digivolution card, 1 of your Digimon unsuspends.
//! [All Turns] [Once Per Turn] When effects add to this Digimon's digivolution
//! cards, return 1 of your opponent's Digimon with as many or fewer
//! digivolution cards as this Digimon to the bottom of the deck.
//!
//! DCGO: EX11/Blue/EX11_018.cs. Synthetic boards only (no recorded games).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "EX11-018";

fn setup() -> DebugRunner {
    let mut tamer = tamer("AQUA-TAMER", "Aqua Tamer", CardColor::Blue, &["Aqua"]);
    tamer.play_cost = 2;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-018 in embedded DSL pack")
        .add_card(filler("FILL"))
        .add_card(digimon("AQUA4", "Aqua Four", CardColor::Blue, 4, 5, &["Aqua"]))
        .add_card(digimon("AQUATIC3", "Aquatic Three", CardColor::Blue, 3, 3, &["Aquatic"]))
        .add_card(digimon("SEA5", "Sea Five", CardColor::Blue, 5, 7, &["Sea Animal"]))
        .add_card(digimon("SEA6", "Sea Six", CardColor::Blue, 6, 9, &["Sea Animal"]))
        .add_card(digimon("SEANOSPACE3", "Sea No Space", CardColor::Blue, 3, 3, &["SeaAnimal"]))
        .add_card(digimon("BEAST4", "Beast Four", CardColor::Blue, 4, 5, &["Beast"]))
        .add_card(digimon("MYMON", "My Mon", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon("OPP-A", "Opp A", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP-B", "Opp B", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP-C", "Opp C", CardColor::Red, 3, 3, &[]))
        .add_card(tamer)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 4])
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

fn hand_pick_ready(r: &mut DebugRunner, h: PermanentHandle, timing: EffectTiming) {
    fire(r, timing, h);
}

/// Drive the pending selections of an OP/WD/WA use + its All Turns follow-up,
/// answering by prompt text. `unsuspend` is the own Digimon to unsuspend;
/// `bounce` the opponent Digimon (by index on their field) to return.
fn drive(r: &mut DebugRunner, unsuspend: Option<PermanentHandle>, bounce: Option<PermanentHandle>) {
    for _ in 0..8 {
        let Some(v) = r.pending_selection_view() else { return };
        if v.prompt.contains("unsuspends") {
            pick_side_field(r, 0, unsuspend.expect("an unsuspend pick was offered"));
        } else if v.prompt.contains("bottom of the deck") {
            pick_side_field(r, 0, bounce.expect("a bounce pick was offered"));
        } else {
            panic!("unexpected prompt {:?}", v.prompt);
        }
    }
}

// ─── Structural / identity ───────────────────────────────────────────────────

#[test]
fn ex11_018_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Ryugumon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(12000));
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
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD/WA clause");
    assert_eq!(
        shared.when,
        vec![
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving,
            CompiledTiming::WhenAttacking
        ]
    );
    assert!(shared.once_per_turn, "ONE shared once-per-turn counter (DCGO shared hash)");
    assert!(shared.optional && shared.outer_prompt, "outer yes/no (DCGO isOptional)");
    let all_turns = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAddDigivolutionCards))
        .expect("All Turns clause");
    assert!(all_turns.once_per_turn);
    assert!(!all_turns.optional, "the bounce is mandatory (DCGO isOptional false)");
}

#[test]
fn ex11_018_face_up_has_evade() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(me, Keyword::Evade));
}

// ─── [On Play] [When Digivolving] [When Attacking] [Once Per Turn] ───────────

#[test]
fn ex11_018_not_offered_without_an_aqua_or_sea_animal_digimon_in_hand() {
    let mut r = setup();
    // A Beast, and a Tamer that has [Aqua] (not a Digimon card).
    push_hand(&mut r, 0, "BEAST4");
    push_hand(&mut r, 0, "AQUA-TAMER");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    hand_pick_ready(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex11_018_hand_pick_offers_exactly_the_aqua_or_sea_animal_digimon_cards() {
    let mut r = setup();
    for id in ["AQUA4", "AQUATIC3", "SEA5", "SEANOSPACE3", "BEAST4", "AQUA-TAMER", "MYMON"] {
        push_hand(&mut r, 0, id);
    }
    let h = r.place_on_field(0, CARD_ID, Some(0));
    hand_pick_ready(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_is_optional(), "outer 'by placing' yes/no");
    accept(&mut r);
    // [Aqua] substring: [Aqua], [Aquatic]; [Sea Animal] / [SeaAnimal] spellings.
    // Not [Beast], not the [Aqua] Tamer (not a Digimon card), not a trait-less Digimon.
    assert_eq!(non_pass(&r).len(), 4);
    assert!(!r.pending_is_optional(), "once accepted the pick is mandatory (canNoSelect false)");
}

#[test]
fn ex11_018_places_the_card_as_bottom_source_then_unsuspends_one_digimon() {
    let mut r = setup();
    push_hand(&mut r, 0, "AQUA4");
    let mon = r.place_on_field(0, "MYMON", Some(0));
    r.game.players[0].battle_area[mon.index as usize].is_suspended = true;
    let h = r.place_stack(0, &["FILL", CARD_ID]);
    hand_pick_ready(&mut r, h, EffectTiming::OnPlay);
    accept(&mut r);
    pick_hand(&mut r, 0, "AQUA4");
    assert_eq!(source_ids(&r, h), vec!["AQUA4".to_string(), "FILL".to_string()],
        "the placed card is the NEW bottom digivolution card");
    // Both Digimon are legal picks (DCGO UnTap has no suspended-only filter).
    assert_eq!(non_pass(&r).len(), 2);
    drive(&mut r, Some(mon), None);
    assert!(!r.game.players[0].battle_area[mon.index as usize].is_suspended);
    assert!(!hand_ids(&r, 0).contains(&"AQUA4".to_string()));
}

#[test]
fn ex11_018_decline_the_outer_prompt_keeps_the_use_and_changes_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, "AQUA4");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    hand_pick_ready(&mut r, h, EffectTiming::OnPlay);
    pass(&mut r, 0);
    assert!(hand_ids(&r, 0).contains(&"AQUA4".to_string()));
    assert_eq!(sources(&r, h), 0);
    // The use was not spent: the same effect is offered again.
    hand_pick_ready(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection().is_some(), "declining preserved the once-per-turn use");
}

#[test]
fn ex11_018_the_three_timings_share_one_use_per_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "AQUA4");
    push_hand(&mut r, 0, "AQUATIC3");
    let h = r.place_stack(0, &["FILL", CARD_ID]);
    hand_pick_ready(&mut r, h, EffectTiming::OnPlay);
    accept(&mut r);
    pick_hand(&mut r, 0, "AQUA4");
    drive(&mut r, Some(h), None);
    hand_pick_ready(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection().is_none(), "OP already used the shared [Once Per Turn]");
    hand_pick_ready(&mut r, h, EffectTiming::WhenAttacking);
    assert!(r.pending_selection().is_none());
}

// ─── [All Turns] [Once Per Turn] ─────────────────────────────────────────────

#[test]
fn ex11_018_own_placement_returns_an_opponent_digimon_with_as_many_or_fewer_digivolution_cards() {
    let mut r = setup();
    push_hand(&mut r, 0, "AQUA4");
    let h = r.place_stack(0, &["FILL", CARD_ID]); // 1 source
    let opp0 = r.place_on_field(1, "OPP-A", Some(0)); // 0 sources
    let opp1 = r.place_stack(1, &["FILL", "OPP-B"]); // 1 source
    let _opp3 = r.place_stack(1, &["FILL", "FILL", "FILL", "OPP-C"]); // 3 sources
    hand_pick_ready(&mut r, h, EffectTiming::OnPlay);
    accept(&mut r);
    pick_hand(&mut r, 0, "AQUA4");
    // Ryugumon now has 2 digivolution cards (FILL + AQUA4): OPP-A (0) and
    // OPP-B (1) qualify; OPP-C (3) does not. Resolve the unsuspend first.
    let mut saw_bounce = false;
    for _ in 0..4 {
        let Some(v) = r.pending_selection_view() else { break };
        if v.prompt.contains("unsuspends") {
            pick_side_field(&mut r, 0, h);
        } else {
            assert!(v.prompt.contains("bottom of the deck"), "{:?}", v.prompt);
            assert!(!v.is_optional, "mandatory (canNoSelect false)");
            assert_eq!(non_pass(&r).len(), 2, "only the <= 2-source opponents");
            pick_side_field(&mut r, 0, opp1);
            saw_bounce = true;
        }
    }
    assert!(saw_bounce, "the placement triggered the All Turns bounce");
    assert!(!field_ids(&r, 1).contains(&"OPP-B".to_string()));
    assert!(field_ids(&r, 1).contains(&"OPP-A".to_string()));
    assert!(field_ids(&r, 1).contains(&"OPP-C".to_string()));
    assert!(deck_ids(&r, 1).contains(&"OPP-B".to_string()), "returned to the deck");
    let _ = opp0;
}

// ─── <Decode> ────────────────────────────────────────────────────────────────

#[test]
fn ex11_018_decode_plays_a_lv5_or_lower_aqua_source_when_leaving_outside_battle() {
    let mut r = setup();
    // Sources: Lv.6 Sea Animal (too high), Lv.5 Sea Animal, Lv.4 Aqua, a Beast.
    let h = r.place_stack(0, &["SEA6", "SEA5", "AQUA4", "BEAST4", CARD_ID]);
    r.game.return_to_hand_from_effect(h, 1);
    let accept = r.pending_selection_view().expect("Decode outer prompt");
    assert_eq!(accept.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    let pick = r.pending_selection_view().expect("Decode source pick");
    assert_eq!(pick.valid_action_ids.len(), 2, "SEA5 and AQUA4 only");
    r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    let on_field = field_ids(&r, 0);
    assert_eq!(on_field.len(), 1);
    assert!(on_field[0] == "SEA5" || on_field[0] == "AQUA4", "{on_field:?}");
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()), "the leave still happened");
}
