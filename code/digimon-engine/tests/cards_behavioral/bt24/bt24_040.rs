//! BT24-040 Venusmon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_040.cs
//! Covers Lv5 [TS] evolution, gated play-cost reduction, shared source-trash
//! plus lock effect, and [All Turns][OPT] TS leave replacement.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledDpConstraint,
};
use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::TriggerSource;

#[test]
fn bt24_040_has_printed_stats_lv5_ts_alt_path_and_security_gated_cost_reduction() {
    let compiled = compiled_bt24_040();

    assert_eq!(compiled.card, "BT24-040");
    assert_eq!(compiled.name, "Venusmon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Yellow, CompiledColor::Blue]
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                        && from
                            .all_of
                            .iter()
                            .any(|pred| pred.trait_has.as_deref() == Some("TS"))
                })
        }),
        "Venusmon must digivolve from Lv5 [TS] for cost 3"
    );

    let cost_reduction = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this,
                condition,
                amount,
                ..
            }) => Some((*when_playing_this, condition, amount)),
            _ => None,
        })
        .expect("Venusmon play-cost reduction clause");
    assert!(
        cost_reduction.0,
        "cost reduction must only apply to this card"
    );
    assert_eq!(cost_reduction.2, &Some(5));
    assert_eq!(
        cost_reduction
            .1
            .as_ref()
            .and_then(|pred| pred.security_count_lte.clone()),
        Some(CompiledDpConstraint::Literal(3)),
        "cost reduction must require your security count <= 3"
    );
}

#[test]
fn bt24_040_on_play_trashes_selected_sources_and_locks_two_specific_permanents() {
    let mut runner = shared_body_runner()
        .hand(0, &["BT24-040"])
        .memory(20)
        .start();
    let source_stack = runner.place_stack(1, &["SRC-A", "SRC-B", "STACK-TOP"]);
    let lock_digimon = runner.place_on_field(1, "LOCK-DIGIMON", Some(0));
    let lock_tamer = runner.place_on_field(1, "LOCK-TAMER", Some(0));

    runner.play(0, 0).expect("play Venusmon");
    choose_permanent(&mut runner, source_stack, "trash selected source stack");
    choose_permanent(&mut runner, lock_tamer, "lock selected Tamer");
    let second_lock_view = runner
        .pending_selection_view()
        .expect("second lock selection");
    assert!(
        !second_lock_view
            .valid_action_ids
            .contains(&encode_permanent(lock_tamer)),
        "second lock prompt must exclude the first lock target"
    );
    choose_permanent(&mut runner, lock_digimon, "lock selected Digimon");
    runner.auto_resolve().expect("finish Venusmon On Play");

    assert_eq!(stack_ids(&runner, source_stack), vec!["STACK-TOP"]);
    assert!(trash_contains(&runner, 1, "SRC-A"));
    assert!(trash_contains(&runner, 1, "SRC-B"));
    assert_locked(&runner, lock_tamer);
    assert_locked(&runner, lock_digimon);
    assert!(
        !runner
            .modifiers()
            .has(source_stack, ModifierType::CannotSuspend),
        "source-stack target was not one of the chosen lock targets"
    );
}

#[test]
fn bt24_040_when_digivolving_uses_same_source_trash_and_lock_body() {
    let mut runner = shared_body_runner()
        .add_card(make_trait_digimon("LV5-TS", 5, 7000, &["TS"]))
        .hand(0, &["BT24-040"])
        .memory(20)
        .start();
    let base = runner.place_on_field(0, "LV5-TS", Some(0));
    let source_stack = runner.place_stack(1, &["SRC-A", "SRC-B", "STACK-TOP"]);
    let lock_digimon = runner.place_on_field(1, "LOCK-DIGIMON", Some(0));
    let lock_tamer = runner.place_on_field(1, "LOCK-TAMER", Some(0));
    let evo_card = runner.game.players[0].hand.remove(0);
    assert!(runner.game.digivolve_onto(0, base.index as usize, evo_card));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base),
    );
    runner.game.drain_effect_queue();
    choose_permanent(&mut runner, source_stack, "trash selected source stack");
    choose_permanent(&mut runner, lock_digimon, "lock selected Digimon");
    choose_permanent(&mut runner, lock_tamer, "lock selected Tamer");
    runner
        .auto_resolve()
        .expect("finish Venusmon When Digivolving");

    assert_eq!(stack_ids(&runner, source_stack), vec!["STACK-TOP"]);
    assert_locked(&runner, lock_digimon);
    assert_locked(&runner, lock_tamer);
}

#[test]
fn bt24_040_protects_ts_digimon_from_non_own_effect_by_placing_other_sourceless_digimon_in_security(
) {
    let mut runner = protection_runner().start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    let cost_body = runner.place_on_field(0, "COST-BODY", Some(0));
    let stacked_body = runner.place_stack(0, &["STACKED-SOURCE", "STACKED-BODY"]);

    runner.game.return_to_hand_from_effect(target, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    let view = runner
        .pending_selection_view()
        .expect("choose replacement cost body");
    assert!(
        !view.valid_action_ids.contains(&encode_permanent(target)),
        "replacement subject cannot be selected as the other Digimon"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_permanent(stacked_body)),
        "Digimon with digivolution cards cannot pay the cost"
    );
    choose_permanent(&mut runner, cost_body, "pay replacement cost");

    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(bottom_security_id(&runner, 0), Some("COST-BODY"));
    assert_eq!(
        runner.security_count(1),
        0,
        "your own Digimon goes to YOUR (its owner's) security, never the opponent's"
    );
}

/// OWNER RULING (2026-10-20, binding): "placing 1 other Digimon ... as the
/// bottom security card" names no stack, so the chosen Digimon goes to the
/// bottom of ITS OWNER's security stack. "1 other Digimon" is unqualified
/// (DCGO offers either player's; Q5781 defines "other" as other than the
/// Digimon that would leave); choosing the opponent's sourceless Digimon sends
/// it to the bottom of the OPPONENT's security (their count +1, yours
/// unchanged) while your [TS] Digimon still stays.
#[test]
fn bt24_040_protection_paid_with_opponent_digimon_places_it_in_opponent_security() {
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .add_card(make_test_card("OWN-SEC", "Own Security"))
        .add_card(make_test_card("OPP-SEC", "Opponent Security"))
        .security(0, &["OWN-SEC"])
        .security(1, &["OPP-SEC"])
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    let own_cost_body = runner.place_on_field(0, "COST-BODY", Some(0));
    let opponent_cost_body = runner.place_on_field(1, "OPP-COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    let view = runner
        .pending_selection_view()
        .expect("choose replacement cost body");
    assert!(
        view.valid_action_ids
            .contains(&encode_any_permanent(own_cost_body)),
        "own sourceless Digimon remains a legal cost body"
    );
    assert!(
        view.valid_action_ids
            .contains(&encode_any_permanent(opponent_cost_body)),
        "opponent sourceless Digimon must be a legal cost body (unqualified \"other Digimon\")"
    );
    choose_any_permanent(
        &mut runner,
        opponent_cost_body,
        "pay with opponent cost body",
    );

    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(!hand_contains(&runner, 0, "TS-TARGET"));
    assert!(permanent_exists(&runner, 0, "COST-BODY"));
    assert!(!permanent_exists(&runner, 1, "OPP-COST-BODY"));
    assert_eq!(
        security_ids(&runner, 1),
        vec!["OPP-COST-BODY", "OPP-SEC"],
        "opponent's Digimon goes to the BOTTOM of its owner's (the opponent's) security"
    );
    assert_eq!(
        security_ids(&runner, 0),
        vec!["OWN-SEC"],
        "your security is unchanged when the opponent's Digimon pays the cost"
    );
}

/// Follow-through of the owner ruling: the opponent's Digimon placed in the
/// opponent's security is an ordinary security card of THAT player — a later
/// security check reveals it from their stack and it is trashed to its owner's
/// trash, never yours.
#[test]
fn bt24_040_opponent_cost_body_is_checked_from_opponent_security_into_owner_trash() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.dp = Some(9000);
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .add_card(attacker)
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    let opponent_cost_body = runner.place_on_field(1, "OPP-COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    choose_any_permanent(
        &mut runner,
        opponent_cost_body,
        "pay with opponent cost body",
    );
    assert_eq!(security_ids(&runner, 1), vec!["OPP-COST-BODY"]);
    assert_eq!(runner.security_count(0), 0);

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve the security check");

    assert_eq!(
        runner.security_count(1),
        0,
        "the security check consumed the opponent's only security card"
    );
    assert!(
        trash_contains(&runner, 1, "OPP-COST-BODY"),
        "the checked security Digimon goes to its owner's (the opponent's) trash"
    );
    assert!(
        !trash_contains(&runner, 0, "OPP-COST-BODY"),
        "the opponent's card never reaches your trash"
    );
}

/// The cost body's own would-leave shield. Venusmon's controller pays with the
/// opponent's sourceless Digimon; leaving by Venusmon's effect is leaving "by
/// an opponent's effect" from the OPPONENT's side, so the opponent's <Guard>
/// (§16-45-1/-2, optional §16-45-3) may protect it. The payment parks behind
/// the opponent's prompt and RESUMES after it:
/// - declined: the placement completes, so the optional processing condition
///   was executed and "they don't leave" follows (§15-7-1);
/// - accepted: the chosen Digimon never leaves, the condition was not executed,
///   so the processing after it can't be (§15-7-2) — the [TS] Digimon leaves.
fn guard_board() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut guard = make_trait_digimon("OPP-GUARD", 4, 5000, &[]);
    guard.keywords = vec![Keyword::Guard];
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .add_card(guard)
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(1, "OPP-GUARD", Some(0));
    let opponent_cost_body = runner.place_on_field(1, "OPP-COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Venusmon's replacement");
    choose_any_permanent(
        &mut runner,
        opponent_cost_body,
        "pay with the opponent's sourceless Digimon",
    );
    let view = runner
        .pending_selection_view()
        .expect("the opponent is offered <Guard> for the chosen Digimon");
    assert_eq!(view.selecting_player, 1, "<Guard> is the opponent's choice");
    assert!(
        runner.pending_is_optional(),
        "<Guard> is optional (§16-45-3)"
    );
    assert!(
        permanent_exists(&runner, 1, "OPP-COST-BODY"),
        "the chosen Digimon has not left while <Guard> is being decided"
    );
    (runner, target, opponent_cost_body)
}

fn assert_guard_declined_outcome(runner: &DebugRunner) {
    assert!(
        permanent_exists(runner, 0, "TS-TARGET"),
        "the cost was paid, so the [TS] Digimon doesn't leave (§15-7-1)"
    );
    assert!(!hand_contains(runner, 0, "TS-TARGET"));
    assert!(!permanent_exists(runner, 1, "OPP-COST-BODY"));
    assert_eq!(
        security_ids(runner, 1),
        vec!["OPP-COST-BODY"],
        "the declined shield lets the chosen Digimon reach its owner's security"
    );
    assert_eq!(runner.security_count(0), 0);
    assert!(
        permanent_exists(runner, 1, "OPP-GUARD"),
        "<Guard> was not paid"
    );
}

fn assert_guard_accepted_outcome(runner: &DebugRunner) {
    assert!(
        trash_contains(runner, 1, "OPP-GUARD"),
        "<Guard> deletes its own Digimon to keep the chosen Digimon"
    );
    assert!(
        permanent_exists(runner, 1, "OPP-COST-BODY"),
        "the chosen Digimon doesn't leave"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "nothing reached the opponent's security"
    );
    assert_eq!(runner.security_count(0), 0);
    assert!(
        !permanent_exists(runner, 0, "TS-TARGET") && hand_contains(runner, 0, "TS-TARGET"),
        "the optional processing condition was not executed, so the [TS] Digimon \
         leaves (§15-7-2)"
    );
}

#[test]
fn bt24_040_opponent_declines_guard_on_cost_body_so_payment_completes_and_ts_stays() {
    let (mut runner, _, _) = guard_board();
    runner.execute_action(1, PASS).expect("decline <Guard>");
    runner.auto_resolve().expect("finish the replacement");
    assert!(runner.pending_selection().is_none());
    assert_guard_declined_outcome(&runner);
}

#[test]
fn bt24_040_opponent_accepts_guard_on_cost_body_so_cost_is_unpaid_and_ts_leaves() {
    let (mut runner, _, _) = guard_board();
    runner
        .execute_action(1, REPLACEMENT_ACCEPT)
        .expect("accept <Guard>");
    runner.auto_resolve().expect("finish the replacement");
    assert!(runner.pending_selection().is_none());
    assert_guard_accepted_outcome(&runner);

    // Venusmon was activated (the cost body was chosen), so its [Once Per
    // Turn] is spent even though the payment failed.
    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));
    runner.game.return_to_hand_from_effect(target_b, 1);
    assert!(
        runner.pending_selection().is_none(),
        "the [Once Per Turn] was spent by the activation"
    );
    assert!(hand_contains(&runner, 0, "TS-TARGET-B"));
}

#[test]
fn bt24_040_clone_parked_at_the_cost_body_guard_prompt_resolves_both_branches() {
    let (mut runner, _, _) = guard_board();
    let mut declined = DebugRunner::wrap(runner.game.clone());
    let mut accepted = DebugRunner::wrap(runner.game.clone());

    declined
        .execute_action(1, PASS)
        .expect("decline <Guard> on the clone");
    declined.auto_resolve().expect("finish the declined clone");
    assert_guard_declined_outcome(&declined);

    accepted
        .execute_action(1, REPLACEMENT_ACCEPT)
        .expect("accept <Guard> on the clone");
    accepted.auto_resolve().expect("finish the accepted clone");
    assert_guard_accepted_outcome(&accepted);

    runner
        .execute_action(1, PASS)
        .expect("decline <Guard> on the original");
    runner.auto_resolve().expect("finish the original");
    assert_guard_declined_outcome(&runner);
}

/// DCGO `IPutSecurityPermanent` also requires the DESTINATION owner to be
/// able to add security (`TopCard.Owner.CanAddSecurity`). When the opponent
/// can't add security by effects, placing their Digimon fails, the optional
/// processing condition is not executed, and the [TS] Digimon leaves
/// (§15-7-2).
#[test]
fn bt24_040_opponent_cost_body_cannot_be_placed_when_opponent_cannot_add_security() {
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    let opponent_cost_body = runner.place_on_field(1, "OPP-COST-BODY", Some(0));
    runner.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            1,
            Expiry::EndOfTurn,
            None,
            1,
        ),
    );

    runner.game.return_to_hand_from_effect(target, 1);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Venusmon's replacement");
    choose_any_permanent(
        &mut runner,
        opponent_cost_body,
        "pick the opponent's sourceless Digimon",
    );
    runner.auto_resolve().expect("finish the replacement");

    assert!(permanent_exists(&runner, 1, "OPP-COST-BODY"));
    assert_eq!(runner.security_count(1), 0);
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "the placement failed, so the [TS] Digimon leaves (§15-7-2)"
    );
}

/// The cost body's leave cause is decided from its CONTROLLER's side, not
/// from the destination stack's. Here the chosen Digimon is owned by the
/// opponent (so it goes to the opponent's security, §3-1-3-8) but controlled
/// by Venusmon's player: it leaves by its controller's own effect, so its
/// controller's <Guard> ("by an opponent's effect", §16-45-1) is not offered.
#[test]
fn bt24_040_cost_body_leave_cause_is_keyed_on_its_controller_not_its_owner() {
    let mut guard = make_trait_digimon("OWN-GUARD", 4, 5000, &[]);
    guard.keywords = vec![Keyword::Guard];
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .add_card(guard)
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "OWN-GUARD", Some(0));
    let borrowed = runner.place_on_field(1, "OPP-COST-BODY", Some(0));
    let borrowed = runner.transfer_control(borrowed, 0);

    // Battle deletion: "other than by your effects" for Venusmon, but not "by
    // an opponent's effect", so <Guard> stays out of the [TS] Digimon's window.
    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::Battle);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Venusmon's replacement");
    choose_any_permanent(&mut runner, borrowed, "pay with the borrowed Digimon");

    assert!(
        runner.pending_selection().is_none(),
        "the borrowed Digimon leaves by its controller's own effect: <Guard> is not offered"
    );
    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(permanent_exists(&runner, 0, "OWN-GUARD"));
    assert_eq!(
        security_ids(&runner, 1),
        vec!["OPP-COST-BODY"],
        "the borrowed Digimon goes to its owner's security"
    );
    assert_eq!(runner.security_count(0), 0);
}

#[test]
fn bt24_040_replacement_is_not_offered_without_legal_cost_body_and_opt_remains_available() {
    let mut runner = protection_runner()
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .start();
    runner.place_stack(0, &["STACKED-SOURCE", "BT24-040"]);
    let target_a = runner.place_on_field(0, "TS-TARGET", Some(0));

    runner.game.return_to_hand_from_effect(target_a, 1);

    assert!(
        runner.pending_selection().is_none(),
        "replacement accept must not be offered when no legal cost body exists"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "unpayable replacement should let the original leave event resolve"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "unpayable replacement must not place a cost body in security"
    );

    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    let cost_body = runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target_b, 1);

    let view = runner
        .pending_selection_view()
        .expect("later payable replacement should still be offered");
    assert!(
        view.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "OPT must remain available after the earlier unpayable event"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept later payable replacement");
    choose_any_permanent(&mut runner, cost_body, "pay later replacement cost");

    assert!(permanent_exists(&runner, 0, "TS-TARGET-B"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(bottom_security_id(&runner, 0), Some("COST-BODY"));
}

#[test]
fn bt24_040_replacement_is_not_offered_when_security_placement_cost_is_floodgated() {
    let mut runner = protection_runner()
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .add_card(make_test_card("COST-BODY-B", "Cost Body B"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target_a = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::EndOfTurn,
            None,
            1,
        ),
    );

    runner.game.return_to_hand_from_effect(target_a, 1);

    assert!(
        runner.pending_selection().is_none(),
        "replacement accept must not be offered when security placement costs are floodgated"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "floodgated replacement should let the original leave event resolve"
    );
    assert!(permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(
        runner.security_count(0),
        0,
        "blocked security-placement cost must leave security unchanged"
    );

    runner.game.modifiers.expire_player_end_of_turn(0);
    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    let cost_body_b = runner.place_on_field(0, "COST-BODY-B", Some(0));

    runner.game.return_to_hand_from_effect(target_b, 1);

    let view = runner
        .pending_selection_view()
        .expect("later payable replacement should still be offered after the floodgate expires");
    assert!(
        view.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "OPT must remain available after the earlier floodgated event"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept later payable replacement");
    choose_any_permanent(
        &mut runner,
        cost_body_b,
        "pay later replacement cost after floodgate expires",
    );

    assert!(permanent_exists(&runner, 0, "TS-TARGET-B"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY-B"));
    assert_eq!(bottom_security_id(&runner, 0), Some("COST-BODY-B"));
}

#[test]
fn bt24_040_protection_does_not_fire_for_your_own_effects() {
    let mut runner = protection_runner().start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 0);

    assert!(
        runner.pending_selection().is_none(),
        "own effects must not install the replacement cost selection"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert_eq!(runner.security_count(0), 0);
}

#[test]
fn bt24_040_replacement_is_once_per_turn_even_with_another_cost_body_available() {
    let mut runner = protection_runner()
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .add_card(make_test_card("COST-BODY-B", "Cost Body B"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target_a = runner.place_on_field(0, "TS-TARGET", Some(0));
    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    let cost_a = runner.place_on_field(0, "COST-BODY", Some(0));
    runner.place_on_field(0, "COST-BODY-B", Some(0));

    runner.game.return_to_hand_from_effect(target_a, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    choose_permanent(&mut runner, cost_a, "pay first replacement cost");
    assert!(permanent_exists(&runner, 0, "TS-TARGET"));

    runner.game.return_to_hand_from_effect(target_b, 1);

    assert!(
        runner.pending_selection().is_none(),
        "OPT replacement must not prompt for the second matching leave event"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET-B"));
    assert!(permanent_exists(&runner, 0, "COST-BODY-B"));
    assert_eq!(runner.security_count(0), 1);
}

#[test]
fn bt24_040_declining_replacement_lets_ts_digimon_leave_without_paying_cost_body() {
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));
    runner.place_on_field(1, "OPP-COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement prompt should allow PASS before choosing a cost body"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline replacement protection");

    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "decline should commit the original return-to-hand event"
    );
    assert!(
        permanent_exists(&runner, 0, "COST-BODY"),
        "decline must not place any cost body in security"
    );
    assert!(
        permanent_exists(&runner, 1, "OPP-COST-BODY"),
        "decline must not move the opponent's sourceless Digimon either"
    );
    assert_eq!(runner.security_count(0), 0);
    assert_eq!(runner.security_count(1), 0);
}

fn venusmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT24-040")
        .expect("BT24-040 YAML loads")
}

fn shared_body_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    venusmon_runner()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("STACK-TOP", "Stack Top"))
        .add_card(make_test_card("LOCK-DIGIMON", "Lock Digimon"))
        .add_card(make_tamer("LOCK-TAMER"))
}

fn protection_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    venusmon_runner()
        .add_card(make_trait_digimon("TS-TARGET", 5, 7000, &["TS"]))
        .add_card(make_test_card("COST-BODY", "Cost Body"))
        .add_card(make_test_card("STACKED-SOURCE", "Stacked Source"))
        .add_card(make_test_card("STACKED-BODY", "Stacked Body"))
}

fn choose_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_permanent(handle);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}

fn choose_any_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_any_permanent(handle);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn encode_any_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn assert_locked(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(
        runner.modifiers().has(handle, ModifierType::CannotSuspend),
        "expected CannotSuspend on {handle:?}"
    );
    assert!(
        runner
            .modifiers()
            .has(handle, ModifierType::CannotActivateWhenDigivolvingEffects),
        "expected CannotActivateWhenDigivolvingEffects on {handle:?}"
    );
}

fn make_trait_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn stack_ids(runner: &DebugRunner, handle: PermanentHandle) -> Vec<&str> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
}

fn trash_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn hand_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

/// Security stack bottom-first (index 0 = bottom card).
fn security_ids(runner: &DebugRunner, player: usize) -> Vec<&str> {
    runner.game.players[player]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
}

fn bottom_security_id(runner: &DebugRunner, player: usize) -> Option<&str> {
    runner.game.players[player]
        .security
        .first()
        .map(|card| card.card_id(&runner.game.card_data))
}

fn compiled_bt24_040() -> digimon_dsl::compiled::CompiledCard {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt24")
        .join("BT24-040.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT24-040 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT24-040 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-040 YAML compiles");
    registry
        .lookup("BT24-040")
        .expect("BT24-040 in registry")
        .clone()
}
