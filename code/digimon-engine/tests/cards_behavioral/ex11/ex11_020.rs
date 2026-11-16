//! EX11-020 Hanimon.
//! Printed text covered here:
//! - [On Deletion] If deleted other than in battle, you may play 1 [Shoemon]
//!   from your hand without paying the cost.
//!
//! [Shoemon] is a card NAME (card image + official Bandai DB bundle
//! data/card_bundles/EX11-020.md); cards.json's "[Shoemon] trait" is an API
//! ingest error. DCGO EX11_020.cs:62-65 `EqualsCardName("Shoemon")`. Only a
//! [Shoemon] this effect can actually play is offered (CR 4.3 1-3-2, 7-1-1;
//! G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY).
//! - Inherited: [Opponent's Turn] [Once Per Turn] when one of your opponent's
//!   Digimon attacks, by deleting 1 of your other Digimon, end that attack.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn ex11_020_on_deletion_may_play_shoemon_from_hand_if_not_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon("SHOEMON"))
        .add_card(make_test_card("NOT-SHOEMON", "Not Shoemon"))
        .hand(0, &["SHOEMON", "NOT-SHOEMON"])
        .memory(0)
        .start();
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));

    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_is_optional(),
        "play-from-hand prompt must be optional"
    );
    let view = runner
        .pending_selection_view()
        .expect("[Shoemon] hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [Shoemon]-named card is eligible"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose [Shoemon]");
    runner.auto_resolve().expect("finish free play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SHOEMON"),
        "selected [Shoemon] enters the battle area"
    );
    assert_eq!(runner.hand_size(0), 1, "non-Shoemon card remains in hand");
}

#[test]
fn ex11_020_on_deletion_can_decline_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon("SHOEMON"))
        .hand(0, &["SHOEMON"])
        .memory(0)
        .start();
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));

    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::OwnEffect);
    runner.execute_action(0, PASS).expect("decline free play");

    assert_eq!(runner.hand_size(0), 1, "declining leaves the card in hand");
    assert!(
        runner.pending_selection_view().is_none(),
        "decline resolves without hidden auto-play"
    );
}

#[test]
fn ex11_020_on_deletion_does_not_fire_when_deleted_in_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon("SHOEMON"))
        .hand(0, &["SHOEMON"])
        .memory(0)
        .start();
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));

    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::Battle);

    assert!(
        runner.pending_selection_view().is_none(),
        "battle deletion must not offer the free-play prompt"
    );
    assert_eq!(runner.hand_size(0), 1, "hand card was not played");
}

#[test]
fn ex11_020_inherited_may_delete_other_digimon_to_end_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .deck(0, &["SECURITY"; 8])
        .deck(1, &["SECURITY"; 8])
        .start();
    runner.place_stack(0, &["EX11-020", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("other-Digimon cost selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "attack cancel must be declinable and cannot auto-delete"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the other Digimon, not the carrier, is eligible"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete other Digimon and end attack");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(runner.security_count(0), 1, "security was not checked");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER"),
        "carrier remains because the printed cost says other Digimon"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "OTHER"),
        "selected other Digimon was deleted as the cost"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "attack state is fully cleared"
    );
}

#[test]
fn ex11_020_inherited_decline_does_not_delete_or_end_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .deck(0, &["SECURITY"; 8])
        .deck(1, &["SECURITY"; 8])
        .start();
    runner.place_stack(0, &["EX11-020", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);
    runner
        .execute_action(0, PASS)
        .expect("decline attack cancel");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "declining does not delete either Digimon"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "declining does not end the attack, so the security check happens"
    );
}

/// Regression: a lone carrier stack (no other Digimon) must NOT end the
/// opponent attack for free. The printed cost is "by deleting 1 of your other
/// Digimon" — with nothing else on the field the cost is unpayable, so the
/// `select_own_permanent` step finds zero candidates and short-circuits. The
/// `delete_permanent` + `end_attack` payoff is gated behind
/// `binding_present: sacrifice`, so neither runs and the attack proceeds.
#[test]
fn ex11_020_inherited_lone_carrier_cannot_end_attack_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .deck(0, &["SECURITY"; 8])
        .deck(1, &["SECURITY"; 8])
        .start();
    runner.place_stack(0, &["EX11-020", "CARRIER"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection_view().is_none(),
        "no other Digimon exists to pay the cost — no prompt must be parked"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "the unpayable cost means the attack is NOT ended — security is checked"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the lone carrier survives the opponent attack"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER"),
        "the carrier stack remains on the field"
    );
}

/// Regression for PUPPETS-G012 review Issue 2 — the deferred-replacement
/// decline path must preserve the deletion cause.
///
/// `delete_permanent_with_cause` sets `current_deletion_cause` only around its
/// *synchronous* `commit_permanent_deletion`. When an optional
/// `WhenWouldBeDeleted` replacement (here `<Barrier>`) parks a selection, the
/// fire-site early-returns and restores `current_deletion_cause` to `None`.
/// Declining the replacement later runs `commit_permanent_deletion_no_replace`
/// from the selection callback — which must re-establish the original cause so
/// the OnDeletion `event_cause` predicate still sees `battle_deletion`.
///
/// This card mirrors EX11-020's On Deletion clause (`not battle_deletion`) and
/// additionally grants itself `<Barrier>`. Deleted in battle, Barrier offers an
/// optional replacement; declining it deletes the card in battle. The
/// `not battle_deletion` clause must therefore NOT fire.
const BARRIER_ON_DELETION_YAML: &str = r#"
card: TST-BARRIER-ONDEL
name: Barrier Hanimon
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
traits: [Puppet]
effects:
  - kind: grant_keyword
    keyword: Barrier
  - when: on_deletion
    optional: true
    condition:
      not:
        event_cause: battle_deletion
    summary: "[On Deletion] If deleted other than in battle, you may play 1 [Shoemon] from hand free"
    process:
      - select_hand:
          of: you
          bind_as: shoemon
          optional: true
          filter:
            all_of:
              - name_is: Shoemon
              - can_play_from_effect_without_cost: true
          prompt: "Play 1 [Shoemon] from your hand without paying the cost"
      - play_from_hand_free: { of: you, hand_index: shoemon }
"#;

#[test]
fn deferred_barrier_decline_in_battle_does_not_fire_not_battle_on_deletion() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BARRIER_ON_DELETION_YAML)
        .expect("inline Barrier-OnDeletion YAML compiles")
        .add_card(make_shoemon("SHOEMON"))
        .add_card(make_test_card("SEC", "Security Card"))
        .hand(0, &["SHOEMON"])
        // Barrier's gate requires the owner to hold at least one security card.
        .security(0, &["SEC"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TST-BARRIER-ONDEL", Some(0));

    // Battle deletion: try_replace offers the optional <Barrier> replacement,
    // which parks a selection. The fire-site early-returns.
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::Battle);
    assert!(
        runner.pending_is_optional(),
        "Barrier must offer an optional replacement on a battle deletion"
    );

    // Decline Barrier — the deferred-decline path now actually deletes the
    // card in battle via commit_permanent_deletion_no_replace.
    runner.execute_action(0, PASS).expect("decline Barrier");

    // The On Deletion clause is gated on `not battle_deletion`. Because the
    // card was in fact deleted in battle, it must not fire — no hand prompt,
    // no free play.
    assert!(
        runner.pending_selection_view().is_none(),
        "battle deletion through a declined Barrier must not offer the free-play prompt"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "[Shoemon] stays in hand — the not-battle clause did not fire"
    );
}

/// A Digimon card named exactly [Shoemon] (no Shoemon trait).
fn make_shoemon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, "Shoemon");
    card.traits = vec!["Puppet".to_string()];
    card
}

/// A Digimon with a [Shoemon] TRAIT but another name: not a [Shoemon] (the
/// printed reference is a name, not a trait).
fn make_shoemon_trait_only(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, "Shoemon Friend");
    card.traits = vec!["Shoemon".to_string()];
    card
}

/// A Digimon named [Shoemon] that ALSO carries a [Shoemon] trait, so it passes
/// both the printed name filter and the old (wrong) trait filter. The
/// playability tests use it so they isolate the playability check: without
/// that check this card would be offered under either filter.
fn make_shoemon_named_and_traited(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, "Shoemon");
    card.traits = vec!["Puppet".to_string(), "Shoemon".to_string()];
    card
}

// ─── Candidate legality (G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY) ──────────────

fn hand_candidates_after_effect_deletion(runner: &mut DebugRunner, full_after: bool) -> Vec<String> {
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));
    if full_after {
        // Hanimon's own deletion frees its slot: size the area so that it is
        // full once Hanimon has left.
        runner.game.rules.field_slots = (runner.game.players[0].battle_area.len() - 1) as u8;
    }
    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::OpponentEffect);
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    view.valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
        .collect()
}

/// [Shoemon] is a NAME: a card named [Shoemon] is offered, a card that only
/// carries a [Shoemon] trait is not (card image + official DB; DCGO
/// EX11_020.cs:62-65 `EqualsCardName("Shoemon")`).
#[test]
fn ex11_020_shoemon_is_a_name_not_a_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon("SHOEMON"))
        .add_card(make_shoemon_trait_only("SHOE-TRAIT"))
        .hand(0, &["SHOEMON", "SHOE-TRAIT"])
        .memory(0)
        .start();
    let offered = hand_candidates_after_effect_deletion(&mut runner, false);
    assert_eq!(offered, vec!["SHOEMON".to_string()]);
}

/// "You can't play Digimon by effects": the [Shoemon] can't be played, so it
/// is not offered (CR 4.3 1-3-2; DCGO offers it, then the play fails).
#[test]
fn ex11_020_shoemon_not_offered_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon_named_and_traited("SHOEMON"))
        .hand(0, &["SHOEMON"])
        .memory(0)
        .start();
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    let offered = hand_candidates_after_effect_deletion(&mut runner, false);
    assert!(offered.is_empty(), "nothing playable is offered: {offered:?}");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1, "[Shoemon] stays in hand");
}

/// A full battle area (DCGO `CanPlayAsNewPermanent` needs a free frame): the
/// [Shoemon] is not offered. Hanimon's own deletion frees its slot, so the
/// area is made full AFTER that slot is gone.
#[test]
fn ex11_020_shoemon_not_offered_when_battle_area_full() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon_named_and_traited("SHOEMON"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .hand(0, &["SHOEMON"])
        .memory(0)
        .start();
    runner.place_on_field(0, "OTHER", Some(0));
    let offered = hand_candidates_after_effect_deletion(&mut runner, true);
    assert!(offered.is_empty(), "nothing playable is offered: {offered:?}");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1, "[Shoemon] stays in hand");
}
