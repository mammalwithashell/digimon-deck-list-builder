use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::enums::{CardColor, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::support::{
    blue_sw, hand_index, is_suspended, plain_digimon, play_declining_optional_materials,
    select_first_non_pass, select_hand_card, top_card_id, DebugRunner,
};

const CARD_ID: &str = "EX12-029";

#[test]
fn ex12_029_inherited_when_attacking_is_not_optional_after_trigger() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-029");
    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed inherited [When Attacking] clause");

    assert!(
        !inherited.optional,
        "printed inherited body has no 'you may' after the trigger resolves"
    );
}

#[test]
fn ex12_029_has_blocker_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .start();

    let sagomon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(sagomon, Keyword::Blocker),
        "Sagomon has printed <Blocker>"
    );
}

#[test]
fn ex12_029_on_play_locks_opponent_then_grants_alliance_and_attack_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .add_card(blue_sw("SW-ALLY", 4))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let ally = runner.place_on_field(0, "SW-ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    play_declining_optional_materials(&mut runner, 0, play_slot);
    select_first_non_pass(&mut runner);
    assert!(
        runner.modifiers().has(opp, ModifierType::CannotSuspend),
        "target can't suspend until their turn ends"
    );

    select_first_non_pass(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("Alliance rider should expose optional attack targets");
    assert!(view.is_optional, "the immediate attack rider is optional");
    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the selected other [SW] Digimon gains Alliance"
    );
}

/// CR 4.3 §16-22: multiple `<Alliance>` grants on the same Digimon stack and
/// each instance triggers independently when attacking (e.g. EX12-056 +
/// EX12-029 both granting `<Alliance>` to an attacker that then attacks).
#[test]
fn multiple_granted_alliance_instances_trigger_independently_on_attack() {
    // Give opponent's 3 security Digimon 11000 DP so only an attacker gaining
    // BOTH allies' DP (5000 + 4000 + 3000 = 12000 > 11000) and BOTH <Alliance>
    // <Security A. +1> bonuses (1 + 2 = 3 checks) survives and clears all 3.
    let mut runner = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", CardColor::Yellow, 5, 5000))
        .add_card(plain_digimon("ALLY-A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("ALLY-B", CardColor::Black, 4, 3000))
        .add_card(plain_digimon("SEC", CardColor::Red, 6, 11000))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let ally_a = runner.place_on_field(0, "ALLY-A", Some(0));
    let ally_b = runner.place_on_field(0, "ALLY-B", Some(0));

    runner
        .game
        .modifiers
        .grant_keyword(attacker, Keyword::Alliance, Expiry::EndOfTurn, 0);
    runner
        .game
        .modifiers
        .grant_keyword(attacker, Keyword::Alliance, Expiry::EndOfTurn, 0);

    runner.attack_player(attacker, 1, false);

    let order = runner
        .pending_selection_view()
        .expect("two granted <Alliance> triggers should open TriggerOrder");
    assert_eq!(order.kind, SelectionKind::TriggerOrder);
    assert_eq!(
        order.valid_action_ids.len(),
        2,
        "both granted <Alliance> instances must trigger independently"
    );

    runner
        .execute_action(order.selecting_player, order.valid_action_ids[0])
        .expect("choose first <Alliance> trigger");
    assert_eq!(runner.game.current_phase, GamePhase::AllianceTiming);
    let first_ally = runner
        .pending_selection_view()
        .expect("first <Alliance> ally selection");
    let first_pick = first_ally
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("first ally candidate");
    runner
        .execute_action(first_ally.selecting_player, first_pick)
        .expect("suspend first ally for Alliance #1");

    assert_eq!(
        runner.game.current_phase,
        GamePhase::AllianceTiming,
        "second <Alliance> instance must auto-fire after the first resolves"
    );
    let mid_dp = runner.effective_dp(attacker).expect("attacker alive mid-attack");
    assert!(
        mid_dp == 9000 || mid_dp == 8000,
        "after Alliance #1, attacker should have base 5000 + first ally DP (got {mid_dp})"
    );
    let second_ally = runner
        .pending_selection_view()
        .expect("second <Alliance> ally selection");
    let second_pick = second_ally
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("second ally candidate");
    runner
        .execute_action(second_ally.selecting_player, second_pick)
        .expect("suspend second ally for Alliance #2");

    runner.auto_resolve().expect("finish attack");
    assert!(runner.game.players[0].battle_area[ally_a.index as usize].is_suspended);
    assert!(runner.game.players[0].battle_area[ally_b.index as usize].is_suspended);
    assert_eq!(
        runner.effective_dp(attacker),
        Some(5000),
        "attacker survived all three 11000 DP security battles (12000 > 11000) and returned to base DP after attack end"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "base 1 check + 2 <Alliance> checks should check all 3 security cards"
    );
}

/// CR 4.3 §16-22: a printed `<Alliance>` plus a runtime-granted `<Alliance>`
/// on the same Digimon must both trigger independently on attack.
#[test]
fn printed_plus_granted_alliance_both_trigger_independently() {
    let mut printed_attacker = plain_digimon("INNATE-ALLIANCE", CardColor::Yellow, 5, 5000);
    printed_attacker.keywords = vec![Keyword::Alliance];
    printed_attacker.effect_text = "＜Alliance＞".to_string();
    let mut runner = DebugRunner::builder()
        .add_card(printed_attacker)
        .add_card(plain_digimon("ALLY-A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("ALLY-B", CardColor::Black, 4, 3000))
        .add_card(plain_digimon("SEC", CardColor::Red, 3, 1000))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = runner.place_on_field(0, "INNATE-ALLIANCE", Some(0));
    let _ally_a = runner.place_on_field(0, "ALLY-A", Some(0));
    let _ally_b = runner.place_on_field(0, "ALLY-B", Some(0));

    runner
        .game
        .modifiers
        .grant_keyword(attacker, Keyword::Alliance, Expiry::EndOfTurn, 0);

    runner.attack_player(attacker, 1, false);

    let order = runner
        .pending_selection_view()
        .expect("printed + granted <Alliance> should open TriggerOrder");
    assert_eq!(order.kind, SelectionKind::TriggerOrder);
    assert_eq!(
        order.valid_action_ids.len(),
        2,
        "printed <Alliance> and granted <Alliance> must both trigger"
    );
}

/// Regression (`raw-20261124T071011Z#g27` Turn 6 EOT): consuming a lower-slot
/// battle-area DigiXros material shifts later permanents down by 1.
/// `take_digixros_material_origin` must unconditionally re-key modifiers,
/// `pending_attack`, and `effect_queue` so a surviving attacking Digimon
/// (EX12-019 Nezhamon) keeps its DP modifiers, its queued
/// `on_own_security_removed` unsuspend trigger, and its live `pending_attack`
/// through `BlockTiming` and `<Piercing>`.
#[test]
fn digixros_battle_area_material_shifts_queued_triggers_pending_attack_and_modifiers() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .dsl_card("EX12-019")
        .expect("EX12-019 YAML loads")
        .dsl_card("EX12-074")
        .expect("EX12-074 YAML loads")
        .dsl_card("BT26-104")
        .expect("BT26-104 YAML loads")
        .add_card(blue_sw("SW-MAT", 4))
        .add_card(plain_digimon("OPP-LOCK", CardColor::Red, 4, 4000))
        .add_card(plain_digimon("OPP-BLOCKER", CardColor::Red, 5, 6000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 2000))
        .hand(0, &["EX12-074", CARD_ID])
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC", "SEC"])
        .deck(0, &["SEC"; 6])
        .deck(1, &["SEC"; 6])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let _mat = runner.place_on_field(0, "SW-MAT", Some(0));
    let nezhamon = runner.place_on_field(0, "EX12-019", Some(0));
    let kunlun = runner.place_on_field(0, "BT26-104", Some(0));
    let _opp_lock = runner.place_on_field(1, "OPP-LOCK", Some(0));
    let _opp_blocker = runner.place_on_field(1, "OPP-BLOCKER", Some(0));

    runner.game.modifiers.add(
        nezhamon,
        ModifierEntry::simple(ModifierType::ChangeDp, 2000, Expiry::EndOfTurn, 0),
    );

    // End P0's turn to open the [End of Your Turn] pool (EX12-019 <Engage> + BT26-104).
    runner.pass_turn();

    let (player, engage_choice) = {
        let view = runner
            .pending_selection_view()
            .expect("End of Your Turn TriggerOrder (EX12-019 <Engage> + BT26-104)");
        assert_eq!(view.kind, SelectionKind::TriggerOrder, "{view:?}");
        let choice = view
            .effect_choices
            .as_ref()
            .expect("TriggerOrder choices")
            .iter()
            .find(|c| c.label.starts_with("EX12-019"))
            .expect("EX12-019 <Engage> branch");
        (view.selecting_player, choice.action_id)
    };
    runner
        .execute_action(player, engage_choice)
        .expect("select EX12-019 <Engage> trigger");

    runner
        .execute_action(0, encode_attack(nezhamon.index as u16, SECURITY_TARGET))
        .expect("declare EX12-019 <Engage> attack on P1 security");
    assert!(is_suspended(&runner, 0, 1), "EX12-019 suspended by attack");
    assert_eq!(
        runner.game.pending_attack.as_ref().map(|pa| pa.attacker),
        Some(nezhamon),
        "pending_attack tracks EX12-019 at slot 1 before DigiXros"
    );

    // Resolve BT26-104 Kunlun [End of Your Turn] -> use EX12-074 -> remove P0 security
    // (queueing EX12-019's unsuspend at slot 1) -> play EX12-029 DigiXrosing slot 0 SW-MAT.
    runner
        .accept_optional_trigger()
        .expect("accept BT26-104 EOT");
    select_hand_card(&mut runner, 0, "EX12-074");
    select_hand_card(&mut runner, 0, CARD_ID);

    let mat_view = runner
        .pending_selection_view()
        .expect("EX12-029 DigiXros material prompt");
    assert_eq!(mat_view.kind, SelectionKind::Material, "{mat_view:?}");
    select_first_non_pass(&mut runner);

    let shifted_nezhamon = PermanentHandle { player: 0, index: 0 };
    assert_eq!(top_card_id(&runner, 0, 0), "EX12-019");
    assert_eq!(top_card_id(&runner, 0, 1), "BT26-104");
    assert_eq!(top_card_id(&runner, 0, 2), CARD_ID);
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(shifted_nezhamon, ModifierType::ChangeDp),
        2000,
        "EX12-019's +2000 DP modifier must shift from slot 1 to slot 0"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(PermanentHandle { player: 0, index: 1 }, ModifierType::ChangeDp),
        0,
        "slot 1 (now BT26-104) must not inherit EX12-019's old +2000 DP modifier"
    );
    assert_eq!(
        runner.game.modifiers.sum(kunlun, ModifierType::ChangeDp),
        0,
        "slot 2 (now EX12-029) must not inherit any shifted modifier"
    );
    assert_eq!(
        runner.game.pending_attack.as_ref().map(|pa| pa.attacker),
        Some(shifted_nezhamon),
        "pending_attack.attacker must shift from slot 1 to slot 0"
    );

    // Both EX12-019's shifted OnOwnSecurityRemoved and EX12-029's [On Play] are
    // live in EX12-074's resolution pool, matching DCGO's 2-entry MultipleSkills
    // (Nov24#g27 v1[125]). Pick EX12-029 [On Play] first.
    let (order_player, ex12_029_choice) = {
        let order_view = runner
            .pending_selection_view()
            .expect("2-entry TriggerOrder (EX12-019 OnOwnSecurityRemoved + EX12-029 OnPlay)");
        assert_eq!(
            order_view.kind,
            SelectionKind::TriggerOrder,
            "{order_view:?}"
        );
        let choices = order_view
            .effect_choices
            .as_ref()
            .expect("TriggerOrder choices");
        assert_eq!(
            choices.len(),
            2,
            "both EX12-019's shifted security-removed trigger and EX12-029's OnPlay must be live"
        );
        assert!(
            choices.iter().any(|c| c.label.starts_with("EX12-019")),
            "EX12-019 trigger must remain live in TriggerOrder: {choices:?}"
        );
        let choice = choices
            .iter()
            .find(|c| c.label.starts_with(CARD_ID))
            .expect("EX12-029 [On Play] branch");
        (order_view.selecting_player, choice.action_id)
    };
    runner
        .execute_action(order_player, ex12_029_choice)
        .expect("select EX12-029 [On Play] first");

    // Resolve EX12-029 [On Play]: lock P1 slot 0 OPP-LOCK (leaving OPP-BLOCKER free to block)
    // and decline the optional <Alliance> rider.
    let lock_view = runner
        .pending_selection_view()
        .expect("EX12-029 [On Play] OppField lock");
    assert_eq!(lock_view.kind, SelectionKind::OppField, "{lock_view:?}");
    select_first_non_pass(&mut runner);

    let alliance_view = runner
        .pending_selection_view()
        .expect("EX12-029 [On Play] optional OwnField Alliance");
    assert_eq!(
        alliance_view.kind,
        SelectionKind::OwnField,
        "{alliance_view:?}"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline EX12-029 optional Alliance rider");

    // EX12-019's queued on_own_security_removed unsuspend must survive compaction and fire.
    let unsuspend_view = runner
        .pending_selection_view()
        .expect("EX12-019's queued on_own_security_removed unsuspend must survive DigiXros slot compaction");
    assert_eq!(
        unsuspend_view.source_permanent,
        Some(shifted_nezhamon),
        "EX12-019 unsuspend prompt must be re-keyed to slot 0"
    );
    assert!(
        is_suspended(&runner, 0, 0),
        "EX12-019 is still suspended before accepting its unsuspend trigger"
    );
    select_first_non_pass(&mut runner);
    assert!(
        !is_suspended(&runner, 0, 0),
        "EX12-019 must unsuspend after accepting its shifted security-removed trigger"
    );

    // EX12-019's <Engage> attack must still enter BlockTiming for P1's OPP-BLOCKER.
    assert_eq!(
        runner.game.current_phase,
        GamePhase::BlockTiming,
        "EX12-019's <Engage> attack must enter BlockTiming after DigiXros shifted its slot"
    );
    let block_view = runner
        .pending_selection_view()
        .expect("BlockTiming selection for P1's OPP-BLOCKER under <Collision>");
    assert_eq!(block_view.selecting_player, 1);

    // Under <Collision>, P1 blocks with OPP-BLOCKER; EX12-019 (12000+2000+4000 DP,
    // <Piercing>) wins the battle, checks 1 P1 security, and rotates Turn 1 to Player 1.
    select_first_non_pass(&mut runner);
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "OPP-BLOCKER was deleted in battle with EX12-019"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "EX12-019's <Piercing> must check 1 opponent security card"
    );
    assert_eq!(
        runner.game.turn_player(),
        1,
        "Turn 1 rotates to Player 1 after the <Engage> attack completes"
    );
}
