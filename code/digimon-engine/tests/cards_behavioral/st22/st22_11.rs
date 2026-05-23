//! ST22-11 Defense Plug-In F
//!
//! # Clauses under test
//!
//! 0. **Color bypass** — `kind: flood_gate` + `IgnoreColorRequirement` gated by
//!    "While you have a Tamer". Tests structural shape and mask behavior.
//!
//! 1. **[Security] De-Digivolve 2** — pre-existing clause. Test retained plus
//!    the `add_this_option_to_hand` tail added in this pass.
//!
//! 2. **[Main] optional free link + buff** — `link_to_own_digimon` (optional,
//!    free) followed by buff selection: Reboot + +3000 DP until EoOT.
//!
//! 3. **[Link] requirement** — `kind: link_requirement, scope: inherited,
//!    cost: 2, filter: level_gte:3`. Structural check only.

#![allow(unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

// ─── Card-data factories ─────────────────────────────────────────────────────

/// A filler card with no special behavior — black Digimon for P0's board.
fn make_black_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c
}

/// A black Tamer — used to activate the color-bypass gate.
fn make_black_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c
}

/// A red Digimon — wrong color for ST22-11 (black option); used to confirm
/// color-bypass is NOT active when no Tamer is present.
fn make_red_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A strong attacker for P0 used to trigger P1's security.
fn make_strong_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 10;
    c.colors = vec![CardColor::Red];
    c
}

/// A level-3 Digimon for P1's battle area — De-Digivolve 2 target.
fn make_lv3_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

// ─── Dual-mode helpers ───────────────────────────────────────────────────────

/// True when a dual-mode mode-select prompt (EffectChoice) is currently
/// installed. ST22-11 carries both a Standard `[Main]` body and a
/// `link_requirement` clause, so playing it from hand surfaces a mode-select
/// before either body runs — identical to ST22-08's dual-mode behaviour.
fn mode_select_pending(runner: &DebugRunner) -> bool {
    matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    )
}

/// Play ST22-11 from hand index 0 in Standard `[Main]` mode.
///
/// The dual-mode mode-select prompt is resolved automatically by choosing the
/// first choice (`HAND_EFFECT_START + 0` = Standard `[Main]`). Returns the
/// `OptionPlayResult` of the initial `play_option_from_hand` call; after this
/// function returns the next `pending_selection` (if any) is the first prompt
/// from the Standard `[Main]` body (the optional link-host selection).
fn play_st22_11_standard(runner: &mut DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if mode_select_pending(runner) {
        // Dual-mode: pick Standard `[Main]` Option (choice index 0).
        let ids = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .clone();
        runner
            .game
            .resolve_selection(0, ids[0])
            .expect("resolve mode-select: choose Standard [Main]");
    }
    result
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═════════════════════════════════════════════════════════════════════════════

/// YAML parses and compiles without errors.
#[test]
fn st22_11_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("ST22-11 YAML must parse and compile")
        .build();
}

/// Clause 0 must be a FloodGate declarative carrying IgnoreColorRequirement.
#[test]
fn st22_11_clause_0_is_flood_gate_ignore_color() {
    let runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .build();
    let card = runner.compiled_card("ST22-11").expect("compiled");

    let is_flood_gate = match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. }) => {
            modifier.eq_ignore_ascii_case("IgnoreColorRequirement")
        }
        _ => false,
    };
    assert!(
        is_flood_gate,
        "clause 0 must be FloodGate(IgnoreColorRequirement); got {:?}",
        card.effects[0]
    );
}

/// Clause 1 must have an on_security triggered clause with DeDigivolve(2).
#[test]
fn st22_11_has_security_dedigivolve_two() {
    let runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("ST22-11 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("ST22-11").expect("ST22-11 compiled");
    let security = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let security = security.expect("ST22-11 must have security clause");
    assert!(security.process.iter().any(|step| matches!(
        step,
        CompiledStep::DeDigivolve {
            amount: Some(2),
            ..
        }
    )));
}

/// Clause 2 must be a main_from_hand triggered clause.
#[test]
fn st22_11_has_main_from_hand_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .build();
    let card = runner.compiled_card("ST22-11").expect("compiled");

    let has_main = card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::MainFromHand),
        _ => false,
    });
    assert!(has_main, "ST22-11 must have a main_from_hand clause");
}

/// Clause 3 must be a LinkRequirement declarative with scope: inherited.
#[test]
fn st22_11_has_link_requirement_inherited() {
    let runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .build();
    let card = runner.compiled_card("ST22-11").expect("compiled");

    let has_link_req = card.effects.iter().any(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement {
            scope,
            cost,
            ..
        }) => *scope == CompiledScope::Inherited && *cost == 2,
        _ => false,
    });
    assert!(
        has_link_req,
        "ST22-11 must have an inherited LinkRequirement with cost 2"
    );
}

/// All four clauses compile (flood_gate + security + main + link_requirement).
#[test]
fn st22_11_has_four_compiled_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .build();
    let card = runner.compiled_card("ST22-11").expect("compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "expected 4 clauses (flood_gate, on_security, main_from_hand, link_requirement); got {}",
        card.effects.len()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0: Color bypass behavioral
// ═════════════════════════════════════════════════════════════════════════════

/// Without any Tamer on the board, a red Digimon does NOT satisfy the color
/// requirement for the black Option ST22-11 — the hand-play bit must be 0.
#[test]
fn st22_11_color_bypass_inactive_without_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_red_digimon("RED-DIGI"))
        .hand(0, &["ST22-11"])
        .memory(10)
        .start();

    runner.place_on_field(0, "RED-DIGI", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "ST22-11 (black) must not be playable from hand with only a red Digimon on board"
    );
}

/// With a Tamer on the board the flood_gate activates IgnoreColorRequirement
/// and the black Option becomes playable even without a black board source.
#[test]
fn st22_11_color_bypass_active_with_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_red_digimon("RED-DIGI"))
        .add_card(make_black_tamer("TAMER"))
        .hand(0, &["ST22-11"])
        .memory(10)
        .start();

    // Put only a tamer (not a black Digimon) on the field.
    runner.place_on_field(0, "TAMER", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "ST22-11 must be playable with a Tamer on the board (color bypass active)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 1: [Security] behavioral
// ═════════════════════════════════════════════════════════════════════════════

/// Full security flow: P0 attacks P1's player; ST22-11 fires; select the
/// opponent Digimon; De-Digivolve 2 runs; add_this_option_to_hand adds the
/// card back to P1's hand.
#[test]
fn st22_11_security_de_digivolves_and_adds_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_strong_attacker("ATK"))
        .add_card(make_lv3_digimon("OPP-DIGI"))
        .security(1, &["ST22-11"])
        .memory(10)
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    let _opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));

    assert_eq!(runner.security_count(1), 1, "precondition: ST22-11 in P1 security");
    assert_eq!(runner.hand_size(1), 0, "precondition: P1 hand empty");

    let result = runner.attack_player(atk, 1, false);

    // The security clause must park on the select_opponent_permanent prompt.
    assert_eq!(
        result,
        AttackResult::InProgress,
        "security clause must install a pending selection"
    );

    // Select the opponent Digimon.
    let view = runner
        .pending_selection_view()
        .expect("select_opponent_permanent pending after security");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("valid action");

    runner.auto_resolve().expect("security pipeline drains");

    // ST22-11 left security and went to P1's hand.
    assert_eq!(runner.security_count(1), 0, "ST22-11 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "add_this_option_to_hand must add ST22-11 to P1's hand"
    );
}

/// When P1 has NO Digimon in the battle area, the condition gate on the
/// security clause prevents De-Digivolve from firing, but the card still
/// goes to P1's hand via the baseline security-option behavior.
#[test]
fn st22_11_security_adds_to_hand_even_without_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_strong_attacker("ATK"))
        .security(1, &["ST22-11"])
        .memory(10)
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));

    assert_eq!(runner.security_count(1), 1, "precondition: ST22-11 in P1 security");
    assert_eq!(runner.battle_area_size(1), 0, "precondition: P1 has no Digimon");
    assert_eq!(runner.hand_size(1), 0, "precondition: P1 hand empty");

    let _result = runner.attack_player(atk, 1, false);
    runner.auto_resolve().expect("pipeline drains");

    assert_eq!(runner.security_count(1), 0, "ST22-11 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "ST22-11 must go to P1's hand even when condition gate prevents De-Digivolve"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 2: [Main] optional link + buff behavioral
// ═════════════════════════════════════════════════════════════════════════════

/// Playing ST22-11 from hand installs a pending selection (the optional
/// link_to_own_digimon prompt). PASS is a valid action (link is optional).
///
/// ST22-11 is dual-mode (Standard [Main] + Link Requirements), so playing it
/// surfaces a mode-select prompt first. We resolve that by choosing Standard,
/// then the link-host selection appears with is_optional == true.
#[test]
fn st22_11_main_link_selection_is_optional_with_pass() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_black_digimon("DIGI"))
        .hand(0, &["ST22-11"])
        .memory(10)
        .start();

    runner.place_on_field(0, "DIGI", Some(0));

    // ST22-11 is dual-mode — play_option_from_hand returns Pending on the
    // mode-select prompt. play_st22_11_standard resolves that automatically.
    let result = play_st22_11_standard(&mut runner);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "play must park (mode-select or link-host selection)"
    );

    // After the mode-select resolves, the next pending is the optional
    // link-host selection from the Standard [Main] body.
    let _view = runner
        .pending_selection_view()
        .expect("link-host selection must be installed after mode-select resolves");
    // For optional link selections, is_optional == true allows PASS via
    // resolve_selection — PASS is NOT in valid_action_ids (that list holds only
    // host-candidate actions), but is_optional gates a PASS accept in the engine.
    assert!(
        runner.pending_is_optional(),
        "link is optional — is_optional must be true (allows PASS decline)"
    );
}

/// Declining the link (choosing PASS) still proceeds to the buff selection.
/// After resolving the buff selection, the Digimon gains Reboot and +3000 DP.
///
/// ST22-11 is dual-mode, so we first resolve the mode-select (Standard), then
/// PASS the optional link-host selection, then pick the buff target.
#[test]
fn st22_11_main_declining_link_grants_reboot_and_dp_buff() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_black_digimon("DIGI"))
        .hand(0, &["ST22-11"])
        .memory(10)
        .start();

    let digi = runner.place_on_field(0, "DIGI", Some(0));

    // Resolve mode-select (dual-mode) → Standard [Main].
    let result = play_st22_11_standard(&mut runner);
    assert_eq!(result, OptionPlayResult::Pending);

    // After mode-select resolves, the optional link-host selection is pending.
    // Decline the link — select PASS.
    runner
        .execute_action(0, PASS)
        .expect("PASS on the link prompt must be accepted");

    // Next pending: select_own_permanent (buff target). The buff runs
    // unconditionally — "Then, 1 of your Digimon gains Reboot and +3000 DP."
    let view = runner
        .pending_selection_view()
        .expect("buff target selection must follow link decline");

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("buff target selection");

    runner.auto_resolve().expect("pipeline drains");

    // The Digimon now has Reboot.
    assert!(
        runner.game.has_keyword(digi, Keyword::Reboot),
        "Digimon must have Reboot after the [Main] buff"
    );

    // DP increased by 3000 (base 4000 + 3000 = 7000).
    let effective = runner.effective_dp(digi).expect("must have a DP value");
    assert_eq!(
        effective, 7000,
        "Digimon must have +3000 DP from the [Main] buff (4000 base + 3000 buff = 7000)"
    );
}

/// Accepting the link attaches ST22-11 to the host, then the buff selection
/// fires and Reboot + +3000 DP are granted to the selected Digimon.
#[test]
fn st22_11_main_accepting_link_attaches_card_and_grants_buff() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_black_digimon("DIGI"))
        .hand(0, &["ST22-11"])
        .memory(10)
        .start();

    let digi = runner.place_on_field(0, "DIGI", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);

    // Choose to link — select the first non-PASS action.
    let view = runner.pending_selection_view().expect("pending");
    let link_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("must have at least one non-PASS (the host Digimon)");
    runner
        .execute_action(view.selecting_player, link_action)
        .expect("link action");

    // Next pending: buff target selection.
    let view2 = runner
        .pending_selection_view()
        .expect("buff target selection must follow link");
    runner
        .execute_action(view2.selecting_player, view2.valid_action_ids[0])
        .expect("buff target");

    runner.auto_resolve().expect("pipeline drains");

    // ST22-11 must be attached to the Digimon.
    let linked = &runner.game.player(0).battle_area[digi.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "ST22-11 must be attached to the Digimon");

    // Reboot granted.
    assert!(
        runner.game.has_keyword(digi, Keyword::Reboot),
        "Digimon must have Reboot after the [Main] buff"
    );
}

/// Reboot and +3000 DP expire at end of the opponent's turn (EoOT).
/// After P0's end-of-turn (which is P0's turn end) and P1's full turn cycle,
/// the modifier must be gone.
///
/// Two `end_turn` calls: first ends P0's turn (enters P1's turn), second
/// ends P1's turn (returns to P0). Modifiers with `end_of_opponents_turn`
/// are removed at the *end of the opponent's turn* — i.e., when `end_turn`
/// fires on the player who is NOT the controller. So: P0 plays ST22-11 →
/// P0 ends turn → P1 starts/ends turn → modifier should be gone.
#[test]
fn st22_11_main_buff_expires_after_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("parses")
        .add_card(make_black_digimon("DIGI"))
        .add_card(make_black_digimon("FILLER"))
        .hand(0, &["ST22-11"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    let digi = runner.place_on_field(0, "DIGI", Some(0));

    // Play ST22-11. Resolve mode-select (Standard [Main]) then decline link.
    let result = play_st22_11_standard(&mut runner);
    assert_eq!(result, OptionPlayResult::Pending);

    // After mode-select resolves, the optional link-host selection is pending.
    // Decline the link.
    runner.execute_action(0, PASS).expect("pass link");

    // Select buff target.
    let view = runner.pending_selection_view().expect("buff prompt");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("buff target");
    runner.auto_resolve().expect("drains");

    // Reboot is active right after.
    assert!(runner.game.has_keyword(digi, Keyword::Reboot), "Reboot active");

    // End P0's turn.
    runner.game.end_turn();
    // Reboot should still be active (P1's turn hasn't ended yet).
    assert!(
        runner.game.has_keyword(digi, Keyword::Reboot),
        "Reboot must still be active during P1's turn"
    );

    // End P1's turn → triggers end_of_opponents_turn expiry.
    runner.game.end_turn();

    // Reboot must be gone.
    assert!(
        !runner.game.has_keyword(digi, Keyword::Reboot),
        "Reboot must expire at end of opponent's (P1's) turn"
    );
}
