//! BT16-055 Namakemon — Digimon, Lv.4, Black, Puppet.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] [When Digivolving] If you have 3 or more security cards, until
//! the end of your opponent's turn, 1 of your Digimon can't have its DP
//! reduced by your opponent's effects and isn't affected by <De-Digivolve>
//! effects. If you have 3 or fewer, 1 of your Digimon gains <Blocker> and
//! <Reboot> until the end of your opponent's turn.
//!
//! Inherited: [All Turns] While this Digimon has [Pulsemon] in its text, it
//! gets +1000 DP.
//! ```
//!
//! # Legacy reference
//!
//! DCGO reference unavailable in this checkout.
//!
//! # Coverage
//!
//! - Metadata and normal/[Pulsemon] digivolution paths.
//! - Faithful active low-security branch: security <= 3 selects 1 own Digimon
//!   and grants <Blocker> + <Reboot> until the opponent's turn ends.
//! - High-security branch (>=3 security): selects 1 own Digimon, grants ImmuneFromDPMinus + CannotBeDeDigivolved until opponent's turn ends (PUPPETS-G024).
//! - Inherited text-gated DP aura.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDpConstraint,
    CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt16_055_metadata_paths_and_supported_clause_shape_match_printed_text() {
    let runner = namakemon_runner().start();
    let compiled = runner.compiled_card("BT16-055").expect("BT16-055 compiles");

    assert_eq!(compiled.name, "Namakemon");
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(compiled.dp, Some(4000));
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "Puppet"));

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(2))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(3) && from.color_is == Some(CompiledColor::Black)
                })
        }),
        "normal black Lv.3 digivolve path must cost 2; paths={digivolve_paths:?}"
    );
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(2))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|from| from.name_is.as_deref() == Some("Pulsemon"))
        }),
        "[Pulsemon] digivolve path must cost 2; paths={digivolve_paths:?}"
    );

    let on_play_when_digivolving_clauses = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .filter(|triggered| {
            triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        on_play_when_digivolving_clauses.len(),
        2,
        "both the <=3 keyword-grant slice and the >=3 protection slice must be present; got={on_play_when_digivolving_clauses:?}"
    );

    let low_security_clause = on_play_when_digivolving_clauses.iter().find(|triggered| {
        triggered
            .condition
            .as_ref()
            .and_then(|p| p.security_count_lte.clone())
            == Some(CompiledDpConstraint::Literal(3))
    });
    assert!(
        low_security_clause.is_some(),
        "<=3 security Blocker+Reboot clause must be present"
    );

    let high_security_clause = on_play_when_digivolving_clauses.iter().find(|triggered| {
        triggered
            .condition
            .as_ref()
            .and_then(|p| p.security_count_gte.clone())
            == Some(CompiledDpConstraint::Literal(3))
    });
    assert!(
        high_security_clause.is_some(),
        ">=3 security DP-reduction/De-Digivolve protection clause must be present"
    );
}

#[test]
fn bt16_055_on_play_low_security_grants_blocker_and_reboot_to_selected_digimon() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");

    // Both effects are enqueued regardless of conditions; TriggerOrder fires
    // so the controller can sequence them. Drain it (high-security fires first
    // but its >=3 condition fails at 2 security, so it silently skips and the
    // drainer immediately presents the low-security OwnField selection).
    drain_trigger_order_if_any(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("low-security own Digimon selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.valid_action_ids.contains(&encode_permanent(ally)),
        "ally must be a legal recipient"
    );
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert_has_low_security_keywords(&runner, ally);
}

#[test]
fn bt16_055_when_digivolving_uses_same_low_security_keyword_grant() {
    // At exactly 3 security, BOTH conditions pass (>=3 AND <=3). Both effects
    // fire. TriggerOrder fires first so the controller can sequence them.
    // After both resolve, Namakemon's stack has both Blocker+Reboot (low branch)
    // and ImmuneFromDPMinus+CannotBeDeDigivolved (high branch).
    let mut runner = namakemon_runner()
        .add_card(make_digimon("BASE", 3, 1000))
        .add_card(make_test_card("FILLER", "Filler"))
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let namakemon = runner.place_stack(0, &["BASE", "BT16-055"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(namakemon),
    );
    runner.game.drain_effect_queue();

    // Resolve first TriggerOrder — picks the first queued effect (high-security
    // >=3 branch; its condition passes at exactly 3 security).
    drain_trigger_order_if_any(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("selection for first effect (OwnField)");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.valid_action_ids.contains(&encode_permanent(namakemon)),
        "Namakemon must be a valid target for first effect"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose Namakemon stack for first effect");

    // After first effect resolves, the second queued effect (low-security <=3
    // branch) may also present a TriggerOrder or directly an OwnField.
    // Drain any TriggerOrder then pick the same target.
    drain_trigger_order_if_any(&mut runner);

    if let Some(view2) = runner.pending_selection_view() {
        assert_eq!(view2.kind, SelectionKind::OwnField);
        runner
            .execute_action(0, view2.valid_action_ids[0])
            .expect("choose target for second effect");
    }
    runner.auto_resolve().expect("finish all effects");

    // At 3 security, both branches fired: Blocker+Reboot from low branch.
    assert_has_low_security_keywords(&runner, namakemon);
    // And ImmuneFromDPMinus+CannotBeDeDigivolved from high branch.
    assert!(
        runner.game.modifiers.has(
            namakemon,
            digimon_engine::enums::ModifierType::ImmuneFromDPMinus
        ),
        "at exactly 3 security, high branch also fires — ImmuneFromDPMinus expected"
    );
}

#[test]
fn bt16_055_low_security_keywords_expire_after_opponents_turn_ends() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER"])
        .deck(0, &["FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");
    // At 1 security, TriggerOrder fires; high-security effect goes first but
    // its >=3 condition fails → skips → low-security fires → OwnField shown.
    drain_trigger_order_if_any(&mut runner);
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert_has_low_security_keywords(&runner, ally);
    runner.end_turn();
    assert_has_low_security_keywords(&runner, ally);
    runner.end_turn();
    assert!(
        !runner.game.has_keyword(ally, Keyword::Blocker),
        "Blocker expires when the opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(ally, Keyword::Reboot),
        "Reboot expires when the opponent's turn ends"
    );
}

#[test]
fn bt16_055_at_four_security_fires_high_security_branch_not_low_security() {
    // At 4 security: >=3 branch fires (DP-reduction/De-Digivolve protection).
    // <=3 branch does NOT fire (4 > 3, condition fails). TriggerOrder still
    // fires because both effects are enqueued before condition checks.
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");

    // Drain TriggerOrder (high-security picked first → its condition passes).
    drain_trigger_order_if_any(&mut runner);

    // The high-security branch prompts selection (target for protection).
    let view = runner
        .pending_selection_view()
        .expect("high-security branch must prompt a selection at 4 security");
    assert_eq!(
        view.kind,
        SelectionKind::OwnField,
        "selection must be from own field"
    );
    assert!(
        view.valid_action_ids.contains(&encode_permanent(ally)),
        "ally must be a valid protection target"
    );

    // After resolving the selection, confirm the protective modifiers land
    // and the <=3 Blocker/Reboot keywords do NOT.
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally for protection");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert!(
        runner
            .game
            .modifiers
            .has(ally, digimon_engine::enums::ModifierType::ImmuneFromDPMinus),
        "ally must have ImmuneFromDPMinus after high-security protection"
    );
    assert!(
        runner.game.modifiers.has(
            ally,
            digimon_engine::enums::ModifierType::CannotBeDeDigivolved
        ),
        "ally must have CannotBeDeDigivolved after high-security protection"
    );
    assert!(
        !runner.game.has_keyword(ally, Keyword::Blocker),
        "Blocker must NOT be granted at 4 security (<=3 condition fails)"
    );
    assert!(
        !runner.game.has_keyword(ally, Keyword::Reboot),
        "Reboot must NOT be granted at 4 security (<=3 condition fails)"
    );
}

#[test]
fn bt16_055_high_security_selects_one_digimon_for_dp_reduction_and_de_digivolve_protection() {
    // PUPPETS-G024 — narrow opponent-effect protection at >=3 security.
    // Verifies:
    //   (a) selection prompts for own Digimon target
    //   (b) ImmuneFromDPMinus is installed on the target
    //   (c) CannotBeDeDigivolved is installed on the target
    //   (d) Narrowness: CannotBeAffected (blanket) is NOT installed
    //   (e) Narrowness: Blocker/Reboot are NOT installed (those are the <=3 branch)
    //   (f) At 2 security (< 3), neither protective modifier fires
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");
    // TriggerOrder fires; pick high-security effect first (its condition passes at 4 security).
    drain_trigger_order_if_any(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("high-security protection target selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.valid_action_ids.contains(&encode_permanent(ally)),
        "ally must be a valid protection target"
    );

    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally for protection");
    runner.auto_resolve().expect("finish Namakemon effect");

    // (b) ImmuneFromDPMinus installed.
    assert!(
        runner
            .game
            .modifiers
            .has(ally, digimon_engine::enums::ModifierType::ImmuneFromDPMinus),
        "ally must have ImmuneFromDPMinus"
    );
    // (c) CannotBeDeDigivolved installed.
    assert!(
        runner.game.modifiers.has(
            ally,
            digimon_engine::enums::ModifierType::CannotBeDeDigivolved
        ),
        "ally must have CannotBeDeDigivolved"
    );
    // (d) Narrowness: blanket CannotBeAffected must NOT be installed.
    assert!(
        !runner
            .game
            .modifiers
            .has(ally, digimon_engine::enums::ModifierType::CannotBeAffected),
        "protection must be narrow — CannotBeAffected (blanket immunity) must NOT be granted"
    );
    // (e) Narrowness: Blocker/Reboot must NOT be installed (those are the <=3 branch).
    assert!(
        !runner.game.has_keyword(ally, Keyword::Blocker),
        "Blocker must NOT be granted by the >=3 security branch"
    );
    assert!(
        !runner.game.has_keyword(ally, Keyword::Reboot),
        "Reboot must NOT be granted by the >=3 security branch"
    );
}

#[test]
fn bt16_055_high_security_protection_absent_below_three_security() {
    // At 2 security, the >=3 branch does NOT fire (condition fails); only the
    // low-security Blocker+Reboot branch fires. TriggerOrder still prompts
    // because both effects are enqueued before conditions are checked.
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");

    // Drain TriggerOrder (high-security picked first; its condition fails →
    // skips; drainer immediately fires low-security → OwnField shown).
    drain_trigger_order_if_any(&mut runner);

    // Low-security Blocker+Reboot selection.
    let view = runner
        .pending_selection_view()
        .expect("low-security branch must prompt at 2 security");
    assert_eq!(view.kind, SelectionKind::OwnField);
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally");
    runner.auto_resolve().expect("finish effect");

    // Low-security keywords were granted.
    assert_has_low_security_keywords(&runner, ally);

    // Protective modifiers were NOT installed (>=3 condition failed).
    assert!(
        !runner
            .game
            .modifiers
            .has(ally, digimon_engine::enums::ModifierType::ImmuneFromDPMinus),
        "ImmuneFromDPMinus must NOT be granted at 2 security"
    );
    assert!(
        !runner.game.modifiers.has(
            ally,
            digimon_engine::enums::ModifierType::CannotBeDeDigivolved
        ),
        "CannotBeDeDigivolved must NOT be granted at 2 security"
    );
}

/// PUPPETS-G024 — opponent-effect-scoping boundary. Drives the BT16-055
/// high-security branch, then exercises the protected Digimon with BOTH an
/// opponent-controlled and a controller-owned DP-reduction. The opponent's
/// reduction must be suppressed; the controller's own reduction must STILL
/// apply (printed text: "can't have its DP reduced **by your opponent's
/// effects**" — own-side reduction is unaffected).
#[test]
fn bt16_055_high_security_dp_protection_is_opponent_effect_scoped() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 5000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");
    drain_trigger_order_if_any(&mut runner);
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally for protection");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "baseline DP before any reduction"
    );

    // OPPONENT (player 1) effect tries to reduce ally's DP by 3000.
    {
        let opp_card = runner.game.players[1].battle_area.first();
        // Player 1 has no field card; build the EffectContext with the
        // protected ally's own top card as a stand-in source_card — the
        // `player` field (1) is what tags `source_player` on the modifier.
        let _ = opp_card;
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        ctx.add_dp_modifier(ally, -3000, Expiry::EndOfTurn);
    }
    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "OPPONENT DP-reduction must be blocked by ImmuneFromDPMinus"
    );

    // CONTROLLER (player 0) own effect reduces ally's DP by 2000 — this must
    // STILL APPLY. The printed protection is narrow to opponent's effects.
    {
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        ctx.add_dp_modifier(ally, -2000, Expiry::EndOfTurn);
    }
    assert_eq!(
        runner.effective_dp(ally),
        Some(3000),
        "controller's OWN DP-reduction must STILL apply to the protected Digimon"
    );
}

/// PUPPETS-G024 — De-Digivolve opponent-effect-scoping boundary. An opponent
/// De-Digivolve on the protected Digimon must be blocked; the controller's
/// OWN De-Digivolve must STILL apply.
#[test]
fn bt16_055_high_security_de_digivolve_protection_is_opponent_effect_scoped() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("BASE3", 3, 3000))
        .add_card(make_digimon("BASE4", 4, 4000))
        .add_card(make_digimon("TOP5", 5, 5000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    // A 3-card stack (Lv.3 → Lv.4 → Lv.5) so De-Digivolve has cards to pop.
    let ally = runner.place_stack(0, &["BASE3", "BASE4", "TOP5"]);

    runner.play(0, 0).expect("play Namakemon");
    drain_trigger_order_if_any(&mut runner);
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally for protection");
    runner.auto_resolve().expect("finish Namakemon effect");

    let depth_before = runner.game.players[0].battle_area[ally.index as usize]
        .card_sources
        .len();
    assert_eq!(depth_before, 3, "ally starts as a 3-card stack");

    // OPPONENT (player 1) effect tries to De-Digivolve 1 — must be blocked.
    runner.game.set_effect_source_player_for_test(Some(1));
    {
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let popped = ctx.de_digivolve(ally, Some(3), Some(1));
        assert_eq!(popped, 0, "opponent De-Digivolve must be blocked → 0 pops");
    }
    runner.game.set_effect_source_player_for_test(None);
    assert_eq!(
        runner.game.players[0].battle_area[ally.index as usize]
            .card_sources
            .len(),
        3,
        "opponent De-Digivolve blocked — stack depth unchanged"
    );

    // CONTROLLER (player 0) own De-Digivolve 1 — must STILL apply.
    runner.game.set_effect_source_player_for_test(Some(0));
    {
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        let popped = ctx.de_digivolve(ally, Some(3), Some(1));
        assert_eq!(
            popped, 1,
            "controller's OWN De-Digivolve must STILL apply → 1 pop"
        );
    }
    runner.game.set_effect_source_player_for_test(None);
    assert_eq!(
        runner.game.players[0].battle_area[ally.index as usize]
            .card_sources
            .len(),
        2,
        "controller's own De-Digivolve popped one card from the stack"
    );
}

#[test]
fn bt16_055_inherited_gives_1000_dp_only_when_carrier_text_contains_pulsemon() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon_with_effect_text(
            "PULSEMON-TEXT-CARRIER",
            4,
            4000,
            "This card mentions [Pulsemon].",
        ))
        .add_card(make_digimon_with_effect_text(
            "NO-PULSEMON-TEXT-CARRIER",
            4,
            4000,
            "This card does not mention the required name.",
        ))
        .start();
    let pulsemon_text_carrier = runner.place_stack(0, &["BT16-055", "PULSEMON-TEXT-CARRIER"]);
    let non_pulsemon_text_carrier =
        runner.place_stack(0, &["BT16-055", "NO-PULSEMON-TEXT-CARRIER"]);

    assert_eq!(runner.effective_dp(pulsemon_text_carrier), Some(5000));
    assert_eq!(runner.effective_dp(non_pulsemon_text_carrier), Some(4000));
}

fn namakemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-055")
        .expect("BT16-055 YAML loads")
}

/// Resolve any pending TriggerOrder selection by picking the first valid
/// action. When the drainer presents two triggered effects to order (because
/// both effects are enqueued even when only one condition passes), this
/// helper picks the first one so the caller can then handle the actual
/// selection. Returns true if a TriggerOrder was resolved, false if the
/// current selection is not a TriggerOrder (or there is no selection).
fn drain_trigger_order_if_any(runner: &mut DebugRunner) -> bool {
    if let Some(view) = runner.pending_selection_view() {
        if view.kind == SelectionKind::TriggerOrder {
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("resolve TriggerOrder");
            return true;
        }
    }
    false
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn assert_has_low_security_keywords(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(
        runner.game.has_keyword(handle, Keyword::Blocker),
        "selected Digimon gains Blocker"
    );
    assert!(
        runner.game.has_keyword(handle, Keyword::Reboot),
        "selected Digimon gains Reboot"
    );
}

fn make_digimon(id: &str, level: u8, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_digimon_with_effect_text(
    id: &str,
    level: u8,
    dp: i32,
    effect_text: &str,
) -> digimon_engine::card_data::CardData {
    let mut card = make_digimon(id, level, dp);
    card.effect_text = effect_text.to_string();
    card
}
