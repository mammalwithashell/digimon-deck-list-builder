//! EX8-060 Myotismon — Digimon, Purple, Lv.5, cost 7, 7000 DP. Traits: Undead,
//! NSo. Attribute: Virus.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-060.md)
//! [Digivolve] Purple Lv.4: Cost 3
//! [Digivolve] Lv.4 w/[NSo] trait: Cost 3
//! [When Attacking] You may play 1 [NSo] trait Digimon card with a play cost of
//! 3 or less from your trash without paying the cost.
//! [Your Turn] [Once Per Turn] When your Digimon are played or digivolve, if any
//! of them have the [NSo] trait, 2 of your Digimon may DNA digivolve into a
//! Digimon card with the [NSo] trait in the hand. Then, that DNA digivolved
//! Digimon may attack.
//! Inherited Effect
//! [When Attacking] [Once Per Turn] By deleting 1 of your other Digimon, this
//! Digimon unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Purple/EX8_060.cs — see the YAML header.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;

use super::nso_support::*;

const CARD_ID: &str = "EX8-060";

fn builder() -> DebugRunnerBuilder {
    let mut dna_result = make_test_dna_card("NSO-DNA", "Nso Dna Result", 4, 3, 0);
    dna_result.traits = vec!["NSo".to_string()];
    dna_result.colors = vec![CardColor::Purple];
    dna_result.level = Some(6);
    dna_result.dp = Some(9000);
    let mut plain_dna = make_test_dna_card("PLAIN-DNA", "Plain Dna Result", 4, 3, 0);
    plain_dna.colors = vec![CardColor::Purple];
    plain_dna.level = Some(6);
    plain_dna.dp = Some(9000);
    let mut plain_base4 = plain("X4-DIGI", 4, 4000, CardColor::Purple);
    plain_base4.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 6, // Purple
        level: 3,
        memory_cost: 1,
    }];
    let mut base4 = nso("N4-DIGI", 4, 4000, 3, CardColor::Purple);
    base4.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 6, // Purple
        level: 3,
        memory_cost: 1,
    }];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-060 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N4-PURPLE", 4, 4000, 4, CardColor::Purple))
        .add_card(nso("N4-BLUE", 4, 4000, 4, CardColor::Blue))
        .add_card(plain("X4-PURPLE", 4, 4000, CardColor::Purple))
        .add_card(plain("X4-BLUE", 4, 4000, CardColor::Blue))
        .add_card(nso("N3-CHEAP", 3, 3000, 3, CardColor::Purple))
        .add_card(nso("N3-COSTLY", 3, 3000, 4, CardColor::Purple))
        .add_card(plain("X3-CHEAP", 3, 3000, CardColor::Purple))
        .add_card(nso("N3-A", 3, 3000, 3, CardColor::Purple))
        .add_card(plain("X3-B", 3, 3000, CardColor::Red))
        .add_card(nso("N3-BASE", 3, 3000, 3, CardColor::Purple))
        .add_card(base4)
        .add_card(plain_base4)
        .add_card(dna_result)
        .add_card(plain_dna)
        .add_card(plain("OPP-A", 4, 9000, CardColor::Blue))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(20)
}

fn digivolve_over(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn ex8_060_digivolves_from_a_purple_level_4_for_3() {
    let (mut r, slot) = digivolve_over("X4-PURPLE");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

#[test]
fn ex8_060_digivolves_from_any_colour_nso_level_4_for_3() {
    let (mut r, slot) = digivolve_over("N4-BLUE");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

#[test]
fn ex8_060_rejects_an_off_colour_level_4_without_the_nso_trait() {
    let (r, slot) = digivolve_over("X4-BLUE");
    assert!(!offered(&r, slot));
}

#[test]
fn ex8_060_compiles_the_printed_clauses() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope != CompiledScope::Inherited && t.when == vec![CompiledTiming::WhenAttacking])));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when == vec![CompiledTiming::WhenAttacking]
                && t.once_per_turn)));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.once_per_turn
                && t.when.contains(&CompiledTiming::OnAllyPlayed)
                && t.when.contains(&CompiledTiming::OnDigivolve))));
}

// ─── [When Attacking] ────────────────────────────────────────────────────────

fn attack_board() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    (r, me)
}

#[test]
fn ex8_060_when_attacking_plays_an_nso_digimon_with_play_cost_3_or_less_from_trash() {
    let (mut r, me) = attack_board();
    push_trash(&mut r, 0, "N3-CHEAP");
    push_trash(&mut r, 0, "N3-COSTLY");
    push_trash(&mut r, 0, "X3-CHEAP");
    r.attack_player(me, 1, false);
    let view = r.pending_selection_view().expect("trash play prompt");
    assert!(view.is_optional, "'You may play'");
    let offered = offered_ids(&r);
    assert_eq!(offered, vec!["N3-CHEAP".to_string()], "play cost <= 3 and [NSo] only: {offered:?}");
    let mem = r.memory();
    pick_trash(&mut r, "N3-CHEAP");
    assert!(field_ids(&r, 0).contains(&"N3-CHEAP".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn ex8_060_the_trash_play_is_optional() {
    let (mut r, me) = attack_board();
    push_trash(&mut r, 0, "N3-CHEAP");
    r.attack_player(me, 1, false);
    pass(&mut r);
    assert!(!field_ids(&r, 0).contains(&"N3-CHEAP".to_string()));
}

// ─── Inherited [When Attacking][OPT] delete another Digimon -> unsuspend ──────

fn inherited_board() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let host = r.place_stack(0, &[CARD_ID, "N4-PURPLE"]);
    r.place_on_field(0, "N3-A", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    (r, host)
}

#[test]
fn ex8_060_inherited_deleting_another_digimon_unsuspends_this_digimon() {
    let (mut r, host) = inherited_board();
    r.attack_player(host, 1, false);
    let view = r.pending_selection_view().expect("delete-cost prompt");
    assert!(view.is_optional, "'By deleting' is declinable");
    pick_field(&mut r, 0, "N3-A");
    assert!(trash_ids(&r, 0).contains(&"N3-A".to_string()), "the other Digimon was deleted");
    let host_now = handle_of(&r, 0, "N4-PURPLE");
    assert!(
        !r.game.players[0].battle_area[host_now.index as usize].is_suspended,
        "this Digimon unsuspended"
    );
}

#[test]
fn ex8_060_inherited_may_be_declined_and_then_the_use_is_refunded() {
    let (mut r, host) = inherited_board();
    r.attack_player(host, 1, false);
    pass(&mut r);
    assert!(field_ids(&r, 0).contains(&"N3-A".to_string()), "nothing deleted");
    let host_now = handle_of(&r, 0, "N4-PURPLE");
    assert!(
        r.game.players[0].battle_area[host_now.index as usize].is_suspended,
        "still suspended from attacking"
    );
}

#[test]
fn ex8_060_inherited_cannot_delete_itself() {
    let mut r = builder().start();
    let host = r.place_stack(0, &[CARD_ID, "N4-PURPLE"]);
    r.place_on_field(1, "OPP-A", Some(0));
    r.attack_player(host, 1, false);
    // No OTHER Digimon: the delete cost is never offered.
    if let Some(view) = r.pending_selection_view() {
        assert_ne!(view.kind, SelectionKind::OwnField, "{view:?}");
    }
}

// ─── [Your Turn][OPT] DNA digivolve into an [NSo] Digimon in hand ─────────────

/// Myotismon + an NSo Lv.3 (A) + a plain Lv.3 (B) on P0's field, a Lv.4 NSo
/// digivolution card in hand and the DNA result (L4 + L3 recipe) in hand.
fn dna_board() -> DebugRunner {
    let mut r = builder().start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "N3-BASE", Some(0));
    r.place_on_field(0, "X3-B", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    push_hand(&mut r, 0, "N4-DIGI");
    push_hand(&mut r, 0, "NSO-DNA");
    push_hand(&mut r, 0, "PLAIN-DNA");
    r
}

fn digivolve_n3_base(r: &mut DebugRunner) {
    let slot = slot_of(r, 0, "N3-BASE");
    let hand = hand_ids(r, 0).iter().position(|s| s == "N4-DIGI").unwrap();
    assert!(r.game.digivolve_from_hand(0, hand, slot, PlaySource::ByHand));
}

#[test]
fn ex8_060_your_turn_nso_digivolve_offers_dna_into_an_nso_hand_card_then_attack() {
    let mut r = dna_board();
    digivolve_n3_base(&mut r);
    let view = r.pending_selection_view().expect("DNA result prompt");
    assert!(view.is_optional, "'may DNA digivolve'");
    let offered = offered_ids(&r);
    assert_eq!(offered, vec!["NSO-DNA".to_string()], "only the [NSo] result: {offered:?}");
    pick_hand(&mut r, 0, "NSO-DNA");
    // Pick the two materials: the new level 4 NSo Digimon and the level 3.
    let view = r.pending_selection_view().expect("first material");
    assert_eq!(view.kind, SelectionKind::AnyField, "the DNA pair prompt");
    pick_field(&mut r, 0, "N4-DIGI");
    let _ = r.pending_selection_view().expect("second material");
    pick_field(&mut r, 0, "X3-B");
    assert!(field_ids(&r, 0).contains(&"NSO-DNA".to_string()), "DNA digivolved into the hand card");
    // "Then, that DNA digivolved Digimon may attack."
    let view = r.pending_selection_view().expect("the attack prompt");
    assert!(view.is_optional, "'may attack'");
    assert_eq!(view.kind, SelectionKind::Target);
    let security = r.game.players[1].security.len();
    let first = view.valid_action_ids.iter().copied().find(|a| *a != 62).unwrap();
    r.execute_action(view.selecting_player, first).unwrap();
    drain_declining(&mut r);
    assert!(
        r.game.players[1].security.len() < security || r.game.players[1].battle_area.is_empty(),
        "the DNA digivolved Digimon attacked"
    );
}

#[test]
fn ex8_060_declining_the_dna_offer_does_nothing_and_refunds_nothing_else() {
    let mut r = dna_board();
    digivolve_n3_base(&mut r);
    pass(&mut r);
    assert!(!field_ids(&r, 0).contains(&"NSO-DNA".to_string()));
    assert!(r.pending_selection_view().is_none(), "no attack offer without a DNA digivolve");
}

/// "[Your Turn] [Once Per Turn] ... may DNA digivolve" is a whole-effect "may":
/// declining the offer leaves the once-per-turn use unspent (official Q&A 1818 /
/// the folded-may refund), so a second [NSo] digivolve the same turn is offered
/// the DNA digivolve again; accepting it spends the use.
#[test]
fn ex8_060_declining_the_dna_offer_keeps_the_once_per_turn_use() {
    let mut r = dna_board();
    r.place_on_field(0, "N3-A", Some(0));
    push_hand(&mut r, 0, "N4-DIGI");
    digivolve_n3_base(&mut r);
    pass(&mut r);
    assert!(r.pending_selection_view().is_none(), "the declined offer is closed");
    // A second [NSo] digivolve in the same turn: the use was refunded.
    let slot = slot_of(&r, 0, "N3-A");
    let hand = hand_ids(&r, 0).iter().position(|s| s == "N4-DIGI").unwrap();
    assert!(r.game.digivolve_from_hand(0, hand, slot, PlaySource::ByHand));
    let view = r.pending_selection_view().expect("the offer is made again");
    assert_eq!(offered_ids(&r), vec!["NSO-DNA".to_string()], "{view:?}");
}

#[test]
fn ex8_060_a_non_nso_digivolution_does_not_trigger_the_dna_offer() {
    let mut r = dna_board();
    // Digivolve into a plain (non-[NSo]) Lv.4: no [NSo] Digimon digivolved.
    push_hand(&mut r, 0, "X4-DIGI");
    let slot = slot_of(&r, 0, "N3-BASE");
    let hand = hand_ids(&r, 0).iter().position(|s| s == "X4-DIGI").unwrap();
    assert!(r.game.digivolve_from_hand(0, hand, slot, PlaySource::ByHand));
    assert!(r.pending_selection_view().is_none(), "no [NSo] Digimon digivolved: no DNA offer");
}
