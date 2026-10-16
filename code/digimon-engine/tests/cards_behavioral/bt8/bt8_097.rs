//! BT8-097 Crimson Blaze — Option, Cost 6, Red.
//!
//! # Card text (official observation 4f4e0ee3a00e2bb6df7d0bdb47d1bcce3423e22727d5f3f28311125a7963cf31 and actual image)
//!
//! ```text
//! When you would play this card, reduce its memory cost by 1 for each
//! Digimon your opponent has in play.
//!
//! [Main] Your opponent can't play Digimon by effects until the end of their
//! turn. Delete all of your opponent's Digimon with 6000 DP or less.
//!
//! Security Effect [Security] Activate this card's [Main] effects.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Red/BT8_097.cs
//!
//! # Patterns this test covers
//! - D2: Cost reduction with formula (1 × opponent battle-area Digimon count)
//! - D6: Flood gate / opponent restriction (CannotPlayDigimonByEffect with EndOfOpponentsTurn)
//! - simultaneous batch deletion of opponent Digimon ≤6000 DP (automated sweep, no player choice)
//! - Option card [Main] from hand (when: main_from_hand)
//! - [Security] activate Main (when: on_security mirrors main_from_hand body)
//! - Filtered card_count_in_zone formula for per-Digimon cost reduction
//! - dp_lte filter on delete_all_permanents
//! - Player-level floodgate DSL via add_player_modifier

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::TriggerSource;

fn audit_runner() -> DebugRunner {
    use digimon_engine::enums::{CardColor, GamePhase};
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../../../cards/ex5/EX5-060.yaml"))
        .unwrap();
    for (id, kind, dp, color) in [
        ("A-RED", CardKind::Digimon, 7000, CardColor::Red),
        ("A-BLUE", CardKind::Digimon, 7000, CardColor::Blue),
        ("A-LOW", CardKind::Digimon, 6000, CardColor::Red),
        ("A-TAMER", CardKind::Tamer, 1000, CardColor::Blue),
        ("A-OPTION", CardKind::Option, 1000, CardColor::Blue),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.dp = Some(dp);
        c.level = Some(3);
        c.colors = vec![color];
        c.play_cost = 1;
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &["BT8-097", "EX5-060", "A-LOW"])
        .hand(1, &["A-LOW", "A-TAMER"])
        .deck(0, &["A-RED"; 12])
        .deck(1, &["A-RED"; 12])
        .security(0, &["BT8-097"])
        .security(1, &["A-RED"; 4])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.inject_trash(1, "A-LOW");
    r.game.current_phase = GamePhase::Main;
    r
}

#[test]
fn bt8_097_legal_option_mask_color_cost_filter_and_zero_floor() {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    for count in [0, 2, 7] {
        let mut r = audit_runner();
        r.place_on_field(0, "A-BLUE", None);
        for _ in 0..count {
            r.place_on_field(1, "A-BLUE", None);
        }
        r.place_on_field(1, "A-TAMER", None);
        r.place_on_field(1, "A-OPTION", None);
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
            0.0,
            "even zero-cost Crimson Blaze needs a red color source"
        );
        r.place_on_field(0, "A-RED", None);
        assert!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0);
        let before = r.memory();
        r.game.decode_action(PLAY_HAND_START, 0);
        assert_eq!(before - r.memory(), (6 - count).max(0));
        assert!(r
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect));
    }
}

#[test]
fn bt8_097_sweep_filters_and_immunity_on_both_activation_paths() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::ModifierEntry;
    for security in [false, true] {
        let mut r = audit_runner();
        r.place_on_field(0, "A-LOW", None);
        r.place_on_field(1, "A-LOW", None);
        r.place_on_field(1, "A-BLUE", None);
        r.place_on_field(1, "A-TAMER", None);
        r.place_on_field(1, "A-OPTION", None);
        let immune = r.place_on_field(1, "A-LOW", None);
        r.game.modifiers.add(
            immune,
            ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 1),
        );
        let protected = r.place_on_field(1, "A-LOW", None);
        r.game.modifiers.add(
            protected,
            ModifierEntry::simple(
                ModifierType::CannotBeDestroyedByEffect,
                0,
                Expiry::Permanent,
                1,
            ),
        );
        if security {
            let h = prepare_security_option(&mut r, "BT8-097");
            r.game.enqueue_triggered(
                EffectTiming::SecuritySkill,
                TriggerSource::SecurityRevealed {
                    defender: 0,
                    card: h,
                },
            );
            r.game.drain_effect_queue();
        } else {
            assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
            r.game.decode_action(0, 0);
        }
        assert_eq!(r.battle_area_size(1), 5);
        assert!(r.pending_selection().is_none());
        assert!(r
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect));
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "A-LOW"
        );
    }
}

#[test]
fn bt8_097_effect_play_gate_persists_with_empty_board_and_allows_tamers() {
    use digimon_engine::enums::{CostDelta, PlaySource};
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
    r.game.decode_action(0, 0);
    assert!(r
        .game
        .play_from_hand_with_cost(1, 0, CostDelta::Free, PlaySource::ByEffect)
        .is_none());
    assert_eq!(r.game.players[1].hand.len(), 2);
    assert!(r
        .game
        .play_from_hand_with_cost(1, 1, CostDelta::Free, PlaySource::ByEffect)
        .is_some());
    r.end_turn();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    assert!(digimon_engine::action::build_action_mask(&r.game, 1)[0] > 0.0);
    r.game.decode_action(0, 1);
    assert_eq!(r.battle_area_size(1), 2, "ordinary play remains legal");
}

#[test]
fn bt8_097_q4663_own_effect_can_play_opponents_digimon() {
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
    r.game.decode_action(0, 0);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
    r.game.decode_action(0, 0); // My Dragomon, opponent's trash.
    let v = r
        .pending_selection_view()
        .expect("opponent chooses trash Digimon");
    assert_eq!(v.selecting_player, 1);
    let a = v.valid_action_ids[0];
    assert!(digimon_engine::action::build_action_mask(&r.game, 1)[a as usize] > 0.0);
    r.game.decode_action(a, 1);
    assert_eq!(
        r.battle_area_size(1),
        1,
        "Q4663: Crimson Blaze blocks the opponent's effects, not my Dragomon playing their Digimon"
    );
}

#[test]
fn bt8_097_q6238_prevents_effect_play_into_breeding() {
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
    r.game.decode_action(0, 0);
    assert!(
        !r.game.play_to_breeding_from_hand(1, 0),
        "Q6238: opponent cannot play a Digimon into breeding by effect"
    );
    assert!(r.game.players[1].breeding_area.is_none());
    assert_eq!(r.game.players[1].hand.len(), 2);
}

// October 2026 audit: actual card image and official observation; DCGO
// DestroyPermanentsClass treats the printed "all" as one deletion batch.
#[test]
fn bt8_097_sweep_is_one_batch_for_multi_target_replacement() {
    use digimon_engine::action::{build_action_mask, space::PASS};
    use digimon_engine::enums::{CardColor, GamePhase};
    for security in [false, true] {
        let mut carrier = make_test_card("AUDIT-CARRIER", "Carrier");
        carrier.dp = Some(9000);
        let mut ally = make_test_card("AUDIT-ALLY", "Dracomon");
        ally.dp = Some(3000);
        let mut red = make_test_card("AUDIT-RED", "Red");
        red.colors = vec![CardColor::Red];
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
            .unwrap()
            .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-027.yaml"))
            .unwrap()
            .add_card(carrier)
            .add_card(ally)
            .add_card(red)
            .hand(0, &["BT8-097"])
            .deck(0, &["AUDIT-RED"; 8])
            .deck(1, &["AUDIT-RED"; 8])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        r.place_on_field(0, "AUDIT-RED", None);
        r.place_stack(1, &["BT20-027", "AUDIT-CARRIER"]);
        r.place_on_field(1, "AUDIT-ALLY", None);
        r.place_on_field(1, "AUDIT-ALLY", None);
        if security {
            let h = prepare_security_option(&mut r, "BT8-097");
            r.game.enqueue_triggered(
                EffectTiming::SecuritySkill,
                TriggerSource::SecurityRevealed {
                    defender: 0,
                    card: h,
                },
            );
            r.game.drain_effect_queue();
        } else {
            assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
            r.game.decode_action(0, 0);
        }
        let v = r.pending_selection_view().expect("paid replacement choice");
        assert_eq!(v.selecting_player, 1);
        assert!(v.is_optional);
        let a = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        assert!(build_action_mask(&r.game, 1)[a as usize] > 0.0);
        let mut declined = r.game.clone();
        assert!(build_action_mask(&declined, 1)[PASS as usize] > 0.0);
        declined.decode_action(PASS, 1);
        declined.drain_effect_queue();
        if declined.pending_selection.is_some() {
            assert!(build_action_mask(&declined, 1)[PASS as usize] > 0.0);
            declined.decode_action(PASS, 1);
            declined.drain_effect_queue();
        }
        assert_eq!(
            declined.players[1].battle_area.len(),
            1,
            "declining all replacement offers must delete both eligible Digimon"
        );
        r.execute_action(1, a).unwrap();
        r.game.drain_effect_queue();
        assert_eq!(
            r.battle_area_size(1),
            3,
            "one suspension protects both Dracomon in the same simultaneous sweep (BT20-027 Q4319)"
        );
        assert!(r.game.players[1].battle_area[0].is_suspended);
    }
}

// ─── Helper fixtures ─────────────────────────────────────────────────────────

/// Base runner with Crimson Blaze registered.
fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .memory(8)
        .start()
}

/// Runner with BT8-097 in player 0's hand plus 3-card decks for multi-turn tests.
fn crimson_blaze_in_hand() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .hand(0, &["BT8-097"])
        .deck(0, &["BT8-097", "BT8-097", "BT8-097"])
        .deck(1, &["BT8-097", "BT8-097", "BT8-097"])
        .memory(8)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions on compiled card
// ═══════════════════════════════════════════════════════════════════════════════

/// BT8-097 must be present in the embedded DSL pack (YAML compiles cleanly).
#[test]
fn bt8_097_yaml_compiles_and_is_in_pack() {
    let runner = runner();
    assert!(
        runner.compiled_card("BT8-097").is_some(),
        "BT8-097 must be present in the embedded DSL pack"
    );
}

/// Exactly one CostReduction declarative clause (Clause A: BeforePayCost).
#[test]
fn bt8_097_has_one_cost_reduction_declarative() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    let cr_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();

    assert_eq!(
        cr_count, 1,
        "BT8-097 must have exactly 1 CostReduction declarative clause (BeforePayCost)"
    );
}

/// Exactly two triggered clauses: [Main] (main_from_hand) + [Security] (on_security).
#[test]
fn bt8_097_has_two_triggered_clauses() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT8-097 must have exactly 2 triggered clauses: main_from_hand + on_security"
    );
}

/// The [Main] clause fires on MainFromHand timing.
#[test]
fn bt8_097_main_clause_fires_on_main_from_hand() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    let has_main = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::MainFromHand),
        _ => false,
    });

    assert!(
        has_main,
        "BT8-097 must have a triggered clause with MainFromHand timing (the [Main] effect)"
    );
}

/// The [Security] clause fires on OnSecurity timing.
#[test]
fn bt8_097_security_clause_fires_on_on_security() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    let has_security = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnSecurity),
        _ => false,
    });

    assert!(
        has_security,
        "BT8-097 must have a triggered clause with OnSecurity timing ([Security] effect)"
    );
}

/// Neither the [Main] nor [Security] clause is [Once Per Turn] or optional.
/// Option cards have no [Once Per Turn] restriction on [Main].
#[test]
fn bt8_097_triggered_clauses_not_once_per_turn_and_not_optional() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    for clause in &card.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert!(
                !t.once_per_turn,
                "BT8-097 has no [Once Per Turn] text — once_per_turn must be false"
            );
            assert!(
                !t.optional,
                "BT8-097 has no 'you may' in [Main] or [Security] — optional must be false"
            );
        }
    }
}

/// All triggered clauses are own-scope (FaceUp), not inherited or security-scoped.
#[test]
fn bt8_097_all_triggered_clauses_are_face_up_scope() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    for clause in &card.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "All BT8-097 triggered clauses must be own-scope (FaceUp)"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Floodgate: positive condition (modifier installed on play)
// ═══════════════════════════════════════════════════════════════════════════════

/// [Main] play: CannotPlayDigimonByEffect modifier is installed on the opponent.
///
/// NOTE: Option card [Main] effects fire via `activate_hand_main(player, hand_index)`,
/// NOT via `runner.play()` (which fires OnPlay timing, different from MainFromHand).
/// `activate_hand_main` directly dispatches `EffectTiming::MainFromHand`.
#[test]
fn bt8_097_main_installs_cannot_play_digimon_by_effect_on_opponent() {
    let mut runner = crimson_blaze_in_hand();

    // No modifier before activation.
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "opponent must NOT have CannotPlayDigimonByEffect before [Main] activates"
    );

    // Activate [Main] effect from hand (fires MainFromHand timing).
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for BT8-097 at hand index 0"
    );
    runner.auto_resolve().unwrap();

    // Floodgate modifier must be installed on opponent (player 1).
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "CannotPlayDigimonByEffect must be installed on opponent after [Main] fires"
    );
}

/// Unlike BT23-014, Crimson Blaze must NOT install CannotPlayTamerByEffect
/// (printed text only blocks Digimon, not Tamers).
#[test]
fn bt8_097_main_does_not_install_cannot_play_tamer_by_effect() {
    let mut runner = crimson_blaze_in_hand();

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayTamerByEffect),
        "BT8-097 printed text restricts Digimon only, not Tamers — \
         CannotPlayTamerByEffect must NOT be installed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Floodgate: negative condition (modifier expires after opponent's turn)
// ═══════════════════════════════════════════════════════════════════════════════

/// The floodgate modifier is still active DURING the opponent's turn (expires at end).
#[test]
fn bt8_097_floodgate_modifier_active_during_opponents_turn() {
    let mut runner = crimson_blaze_in_hand();

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "modifier must be installed after [Main]"
    );

    // End player 0's turn → player 1's turn starts.
    runner.end_turn();

    // Still active DURING player 1's turn.
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "CannotPlayDigimonByEffect must remain active DURING opponent's turn \
         (expires only when their turn ENDS)"
    );
}

/// The floodgate modifier expires AFTER the opponent's turn ends.
#[test]
fn bt8_097_floodgate_modifier_expires_after_opponents_turn_ends() {
    let mut runner = crimson_blaze_in_hand();

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    // End P0's turn → P1 plays → end P1's turn → P0's turn.
    runner.end_turn();
    runner.end_turn();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "CannotPlayDigimonByEffect must expire after opponent's turn fully ends"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Main] delete clause: for_each sweep
// ═══════════════════════════════════════════════════════════════════════════════

/// [Main]: no crash when opponent has no Digimon (empty field).
#[test]
fn bt8_097_main_no_crash_when_opponent_has_no_digimon() {
    let mut runner = crimson_blaze_in_hand();

    // No opponent permanents.
    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 0);

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    // No selection, no crash, no field change.
    assert!(
        runner.game.pending_selection.is_none(),
        "no pending selection expected when opponent has no Digimon"
    );
    assert_eq!(runner.battle_area_size(1), 0);
}

/// [Main]: opponent Digimon with DP ≤ 6000 is deleted (positive — eligible target).
#[test]
fn bt8_097_main_deletes_opp_digimon_with_dp_lte_6000() {
    let mut low_dp = make_test_card("OPP-LOWDP", "OppLowDp");
    low_dp.card_kind = CardKind::Digimon;
    low_dp.dp = Some(5000);
    low_dp.level = Some(4);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(low_dp)
        .hand(0, &["BT8-097"])
        .deck(0, &["BT8-097", "BT8-097"])
        .deck(1, &["BT8-097", "BT8-097"])
        .memory(8)
        .start();

    runner.place_on_field(1, "OPP-LOWDP", Some(0));
    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 1);

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert!(
        runner.game.pending_selection.is_none(),
        "for_each delete sweep must not install a player selection"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent Digimon with ≤6000 DP must be deleted by [Main]"
    );
}

/// [Main]: multiple opponent Digimon present — all are swept (no player choice).
/// Verifies the for_each is an automated sweep, not a targeted selection.
///
/// G-FOR-EACH-DELETE-INDEX-SHIFT (closed): `for_each` now snapshots each
/// matched permanent's STABLE top-card identity (`CardHandle`) and re-resolves
/// the positional handle at the start of every iteration. When a Digimon at
/// index 0 is deleted, the second permanent's iteration re-resolves to its new
/// (shifted-down) index instead of going stale, so all eligible Digimon are
/// swept. The fix is body-agnostic — correct for delete, return-to-hand, etc.
#[test]
fn bt8_097_main_deletes_multiple_opp_digimon_with_no_player_choice() {
    let mut d1 = make_test_card("OPP-D1", "OppD1");
    d1.card_kind = CardKind::Digimon;
    d1.dp = Some(3000);
    d1.level = Some(3);
    let mut d2 = make_test_card("OPP-D2", "OppD2");
    d2.card_kind = CardKind::Digimon;
    d2.dp = Some(6000);
    d2.level = Some(4);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(d1)
        .add_card(d2)
        .hand(0, &["BT8-097"])
        .deck(0, &["BT8-097", "BT8-097"])
        .deck(1, &["BT8-097", "BT8-097"])
        .memory(8)
        .start();

    runner.place_on_field(1, "OPP-D1", Some(0));
    runner.place_on_field(1, "OPP-D2", Some(0));
    assert_eq!(runner.battle_area_size(1), 2);

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert!(
        runner.game.pending_selection.is_none(),
        "for_each delete must not install a player selection"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "all opponent Digimon with ≤6000 DP must be deleted — no player choice needed"
    );
}

/// [Main]: opponent with DP > 6000 is NOT deleted.
#[test]
fn bt8_097_main_spares_opp_digimon_with_dp_above_6000() {
    let mut high_dp = make_test_card("OPP-HIGHDP", "OppHighDp");
    high_dp.card_kind = CardKind::Digimon;
    high_dp.dp = Some(7000); // above 6000 — must survive
    high_dp.level = Some(5);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(high_dp)
        .hand(0, &["BT8-097"])
        .deck(0, &["BT8-097", "BT8-097"])
        .deck(1, &["BT8-097", "BT8-097"])
        .memory(8)
        .start();

    runner.place_on_field(1, "OPP-HIGHDP", Some(0));

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Digimon with 7000 DP must NOT be deleted (above 6000 DP cap)"
    );
}

/// [Main] DP boundary: exactly 6000 DP must be deleted.
/// Boundary-inclusion test for dp_lte: 6000 (≤ 6000 means exactly 6000 is eligible).
#[test]
fn bt8_097_main_deletes_opp_digimon_with_exactly_6000_dp() {
    let mut exactly_6000 = make_test_card("OPP-6000DP", "Opp6000Dp");
    exactly_6000.card_kind = CardKind::Digimon;
    exactly_6000.dp = Some(6000); // exactly 6000 — must be deleted (≤ 6000)
    exactly_6000.level = Some(4);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(exactly_6000)
        .hand(0, &["BT8-097"])
        .deck(0, &["BT8-097", "BT8-097"])
        .deck(1, &["BT8-097", "BT8-097"])
        .memory(8)
        .start();

    runner.place_on_field(1, "OPP-6000DP", Some(0));

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "Digimon with exactly 6000 DP must be deleted (dp_lte: 6000 means ≤ 6000)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [Security] clause mirrors [Main] effects
// ═══════════════════════════════════════════════════════════════════════════════

/// [Security] timing fires the floodgate modifier on the opponent.
#[test]
fn bt8_097_security_installs_cannot_play_digimon_by_effect_modifier() {
    let mut runner = runner();

    // Seat an actual revealed-security source before firing its security timing.
    let handle = prepare_security_option(&mut runner, "BT8-097");

    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "modifier must not be present before security effect fires"
    );

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::SecurityRevealed {
            defender: 0,
            card: handle,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "CannotPlayDigimonByEffect must be installed on opponent after [Security] fires"
    );
}

#[test]
fn bt8_097_triggered_clauses_use_native_add_player_modifier_step() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered.iter().all(|clause| {
            clause.process.iter().any(|step| {
                matches!(
                    step,
                    CompiledStep::AddPlayerModifier {
                        modifier,
                        ..
                    } if modifier == "CannotPlayDigimonByEffect"
                )
            })
        }),
        "BT8-097 main and security clauses should use add_player_modifier for the floodgate"
    );
    assert!(
        triggered.iter().all(|clause| !clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. }))),
        "BT8-097 should not need raw_rust for its player-level floodgate"
    );
}

/// [Security]: deletes opponent Digimon just like [Main] does.
#[test]
fn bt8_097_security_deletes_eligible_opp_digimon() {
    let mut target = make_test_card("SEC-OPP-DIGI", "SecOppDigi");
    target.card_kind = CardKind::Digimon;
    target.dp = Some(4000);
    target.level = Some(4);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(target)
        .memory(8)
        .start();

    let handle = prepare_security_option(&mut runner, "BT8-097");
    runner.place_on_field(1, "SEC-OPP-DIGI", Some(0));
    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 1);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::SecurityRevealed {
            defender: 0,
            card: handle,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "for_each sweep from [Security] must not install a player selection"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "[Security] clause must delete eligible opponent Digimon (same as [Main])"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Cost reduction structural verification
// ═══════════════════════════════════════════════════════════════════════════════

/// The CostReduction declarative clause compiles correctly.
/// It uses when_playing_this: true and an amount_fn with card_count_in_zone.
#[test]
fn bt8_097_cost_reduction_declarative_compiles() {
    let runner = runner();
    let card = runner.compiled_card("BT8-097").expect("BT8-097 in pack");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )),
        "CostReduction declarative must compile and be present in the embedded pack"
    );
}

/// The cost reduction reduces play cost by 1 per opponent battle-area Digimon.
/// Behavioral test: with 2 opponent Digimon and 1 Tamer, cost decreases by 2
/// (from 6 to 4); the Tamer is ignored by the filtered card_count_in_zone formula.
#[test]
fn bt8_097_cost_reduction_reduces_play_cost_by_one_per_opp_digimon() {
    let mut opp_digi = make_test_card("CR-OPP-DIGI", "CrOppDigi");
    opp_digi.card_kind = CardKind::Digimon;
    opp_digi.dp = Some(7000); // high DP — not eligible for the delete sweep
    opp_digi.level = Some(5);
    let mut opp_tamer = make_test_card("CR-OPP-TAMER", "CrOppTamer");
    opp_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(opp_digi.clone())
        .add_card(opp_tamer)
        .hand(0, &["BT8-097"])
        .memory(4) // With 2 opp Digimon, cost becomes 6 - 2 = 4 → playable exactly.
        .start();

    // Place 2 opponent Digimon plus 1 Tamer. Only the Digimon should count.
    runner.place_on_field(1, "CR-OPP-DIGI", None);
    runner.place_on_field(1, "CR-OPP-DIGI", None);
    runner.place_on_field(1, "CR-OPP-TAMER", None);

    let memory_before = runner.memory();

    // With 2 opp Digimon: base cost 6 - 2 = 4. If the Tamer were counted,
    // the cost would incorrectly become 3.
    runner.play(0, 0);
    runner.auto_resolve().unwrap();

    // Verify BT8-097 is no longer in hand (was played).
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "BT8-097 must have been played from hand (cost was affordable)"
    );

    // Memory should have moved by the reduced cost, not the base cost.
    let cost = memory_before - runner.memory();
    assert_eq!(
        cost, 4,
        "BT8-097 must reduce by 2 for opponent Digimon only; Tamer must not count"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Failure-mode audit: floodgate scope (PR #456 cluster 3c)
// ═══════════════════════════════════════════════════════════════════════════════
//
// PR #456 fixed the BT8-097 [Main]+[Security] tests by routing the floodgate
// through `add_player_modifier` and clearing on opponent's EOT. Adjacent
// failure modes that were not directly fixed but ride the same plumbing —
// these guard against regressions in the floodgate scope.

/// **Adjacent edge-case (cluster 3c):** printed text restricts only
/// effect-driven Digimon plays. Natural plays from hand
/// (`PlaySource::ByHand`) MUST continue to succeed under the floodgate.
/// Verifies the gate at `game_actions.rs:380` consults the play-source
/// kind, not just the card kind.
#[test]
fn bt8_097_floodgate_does_not_block_opponent_natural_play_from_hand() {
    use digimon_engine::card_data::CardData;
    use digimon_engine::enums::CardColor;

    fn opp_digi() -> CardData {
        let mut c = make_test_card("OPP-NATURAL", "OppNatural");
        c.card_kind = CardKind::Digimon;
        c.colors = vec![CardColor::Red];
        c.dp = Some(3000);
        c.level = Some(3);
        c.play_cost = 1;
        c
    }

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt8/BT8-097.yaml"))
        .expect("BT8-097 production YAML")
        .add_card(opp_digi())
        .hand(0, &["BT8-097"])
        .hand(1, &["OPP-NATURAL"])
        .deck(0, &["BT8-097", "BT8-097"])
        .deck(1, &["BT8-097", "BT8-097"])
        .memory(8)
        .start();

    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "floodgate must be installed before opponent's turn"
    );

    // P0 ends turn → P1's turn.
    runner.end_turn();

    // P1 plays Digimon naturally from hand (PlaySource::ByHand path).
    let played = runner.game.play_from_hand(1, 0);
    assert!(
        played.is_some(),
        "natural play_from_hand must NOT be blocked by CannotPlayDigimonByEffect — \
         floodgate is scoped to effect-driven plays only"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "opponent's natural Digimon play must succeed under the floodgate"
    );
}

/// **Adjacent edge-case (cluster 3c):** the floodgate is installed via
/// `add_player_modifier`'s `EndOfOpponentsTurn` expiry. Verifies the
/// modifier persists across the boundary where P0's turn ends + P1's
/// turn starts, then clears exactly when P1's turn ENDS — the boundary
/// PR #456 only round-tripped on.
#[test]
fn bt8_097_floodgate_clears_at_exactly_opp_turn_end_boundary() {
    let mut runner = crimson_blaze_in_hand();
    runner.game.activate_hand_main(0, 0);
    runner.auto_resolve().unwrap();

    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotPlayDigimonByEffect));

    runner.end_turn(); // P0 → P1
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "still active during P1's turn (opp's turn hasn't ended yet)"
    );
    runner.end_turn(); // P1 → P0
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "must clear at the boundary where P1's turn ends"
    );
}

// Tail audit: the sweep uses current DP and survives cloning before activation.
#[test]
fn bt8_097_tail_current_dp_and_clone_use_real_main_mask() {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    let reduced = r.place_on_field(1, "A-BLUE", None);
    let raised = r.place_on_field(1, "A-LOW", None);
    r.game.modifiers.add(
        reduced,
        ModifierEntry::simple(ModifierType::ChangeDp, -1000, Expiry::Permanent, 1),
    );
    r.game.modifiers.add(
        raised,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::Permanent, 1),
    );
    assert_eq!(r.effective_dp(reduced), Some(6000));
    assert_eq!(r.effective_dp(raised), Some(7000));
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        assert!(build_action_mask(game, 0)[PLAY_HAND_START as usize] > 0.0);
        game.decode_action(PLAY_HAND_START, 0);
        assert_eq!(game.players[1].battle_area.len(), 1);
        assert_eq!(
            game.players[1].battle_area[0]
                .top_card()
                .card_id(&game.card_data),
            "A-LOW"
        );
        assert_eq!(game.memory, 6);
        assert!(game.pending_selection.is_none());
        assert!(game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect));
    }
}

#[test]
fn bt8_097_tail_actual_security_check_needs_no_red_color_or_cost() {
    let mut r = audit_runner();
    let attacker = r.place_on_field(1, "A-BLUE", Some(0));
    r.place_on_field(1, "A-LOW", None);
    r.game.turn_player_idx = 1;
    let before = r.memory();
    r.attack_player(attacker, 0, false);
    assert_eq!(r.security_count(0), 0);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.memory(), before);
    assert!(r
        .game
        .modifiers
        .player_has(1, ModifierType::CannotPlayDigimonByEffect));
    assert!(r.pending_selection().is_none());
}

// A SecurityRevealed card is outside the battle area while its security effect resolves.
fn prepare_security_option(
    runner: &mut DebugRunner,
    id: &str,
) -> digimon_engine::card_source::CardHandle {
    runner.add_to_hand(0, id);
    let card = runner.game.players[0].hand.pop().unwrap();
    let handle = card.handle();
    runner.game.pending_security = Some(digimon_engine::selection::PendingSecurity {
        defender: 0,
        card,
        played: false,
    });
    handle
}
