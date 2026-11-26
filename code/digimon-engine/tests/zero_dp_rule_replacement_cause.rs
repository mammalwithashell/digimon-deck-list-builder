//! 0-DP state-based deletion (CR 4.3 §17-1-3-2-2) passes `ReplacementCause::Rule`
//! instead of falling back to `ReplacementCause::OwnEffect` via
//! `infer_deletion_cause`. Replacements gated on "other than by your effects"
//! (`none_of: [{ replacement_cause: own_effect }]`, e.g. BT18-063 Beetlemon)
//! must be offered when a Digimon hits 0 DP by rules processing, whereas
//! opponent-effect-only protections (`CannotBeDestroyedByEffect` with
//! `cause_filter: Some(ReplacementCause::OpponentEffect)`) must NOT block 0-DP
//! rule deletion.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn zero_dp_rule_deletion_offers_bt18_063_non_own_effect_replacement() {
    let mut tamer = make_test_card("INHERITED-TAMER", "Inherited Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Yellow];
    tamer.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();

    let mut top = make_test_card("TOP-MON", "Top Mon");
    top.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-063")
        .expect("BT18-063 YAML loads")
        .add_card(tamer)
        .add_card(top)
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &["INHERITED-TAMER", "BT18-063", "TOP-MON"]);

    // Reduce DP to 0 outside an active effect (`effect_source_player == None`),
    // then run the state-based rules check via `drain_effect_queue()`.
    runner.game.modifiers.add(
        carrier,
        ModifierEntry::simple(ModifierType::ChangeDp, -5000, Expiry::EndOfTurn, 1),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("BT18-063 inherited replacement must be offered on 0-DP rule deletion");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert_eq!(pending.selecting_player, 0);
    assert!(pending.is_optional);
    let mask = build_action_mask(&runner.game, 0);
    assert!(mask[REPLACEMENT_ACCEPT as usize] > 0.5);
    assert!(mask[PASS as usize] > 0.5);

    // Accept BT18-063's replacement, then pick INHERITED-TAMER from sources.
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept BT18-063 replacement");
    let source_pick = runner
        .pending_selection()
        .expect("BT18-063 parks on Tamer source selection");
    let pick_action = *source_pick
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("legal Tamer source pick");
    runner
        .execute_action(0, pick_action)
        .expect("play INHERITED-TAMER from sources");
    runner.auto_resolve().expect("finish 0-DP deletion");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "INHERITED-TAMER"),
        "INHERITED-TAMER was played onto the battle area by BT18-063"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TOP-MON"),
        "TOP-MON was still deleted by the 0-DP rule check (BT18-063 is non-cancelling)"
    );
}

#[test]
fn zero_dp_rule_deletion_is_not_blocked_by_opponent_effect_deletion_protection() {
    let mut top = make_test_card("PROTECTED-MON", "Protected Mon");
    top.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .add_card(top)
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .memory(5)
        .start();

    let carrier = runner.place_on_field(0, "PROTECTED-MON", Some(0));
    let mut protection =
        ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect, 1, Expiry::EndOfTurn, 0);
    protection.cause_filter = Some(ReplacementCause::OpponentEffect);
    runner.game.modifiers.add(carrier, protection);

    runner.game.modifiers.add(
        carrier,
        ModifierEntry::simple(ModifierType::ChangeDp, -5000, Expiry::EndOfTurn, 1),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "0-DP rule deletion (ReplacementCause::Rule) must not be blocked by OpponentEffect-only deletion protection"
    );
}
