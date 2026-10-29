//! ST23-05 Habakirimon — Digimon (BEATBREAK), Lv.6, Yellow, DP 12000, Cost 11.
//! Traits: Shaman, Glowing Dawn, BEATBREAK. Attribute: Virus.
//!
//! # Card text (data/cards.json — verbatim)
//! [When Digivolving] [When Attacking] [Once Per Turn] Place 1 of your
//! opponent's lowest DP Digimon as the top security card. Then, by trashing the
//! top security card of 1 player with the most security cards, ＜Recovery +1＞.
//! [All Turns] [Once Per Turn] When any of your [Glowing Dawn] trait Digimon
//! would leave the battle area, by trashing your top security card, they don't
//! leave.
//!
//! Printed digivolve box: [Digivolve] Lv.5 w/[Glowing Dawn] trait: Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Yellow/ST23_05.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - extreme-metric place-on-security (lowest_dp; multi-target tie via selector)
//! - E1 3-way EffectChoice gated by symmetric "most security" formula + Recovery
//! - F3 leave-prevention replacement (trash top security → cancel)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST23-05";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A vanilla opponent Digimon with a specified DP (for the lowest-DP pick).
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 5;
    c
}

/// Non-Digimon security filler (avoids a security battle when broken).
fn sec_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn is_security_top_is(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .security
        .first()
        .map(|s| s.card_id(&runner.game.card_data) == card_id)
        .unwrap_or(false)
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-05 YAML parses and compiles")
        .add_card(opp_digimon("OPP-LOW", 2000))
        .add_card(opp_digimon("OPP-HIGH", 9000))
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .memory(8)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st23_05_metadata_and_alt_paths() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Habakirimon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    let gd = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)
    });
    assert!(gd, "Lv.5 [Glowing Dawn] alt-path present");
}

#[test]
fn st23_05_has_wd_wa_clause_and_leave_replacement() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let has_wd_wa = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && t.once_per_turn
        )
    });
    assert!(has_wd_wa, "[WD][WA][OPT] clause present");
    let has_repl = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(has_repl, "leave-prevention replacement present");
}

// ─── Section 2 — place opponent lowest-DP Digimon as top security ────────────

/// POSITIVE: with two opponent Digimon (DP 2000 / 9000), the lowest-DP pick
/// surfaces exactly one legal target and places it as the opponent's top
/// security card. (Then the most-security 3-way choice follows; decline it.)
#[test]
fn st23_05_places_opponent_lowest_dp_as_top_security() {
    let mut runner = base();
    runner.set_first_player(0);

    let hab = runner.place_stack(0, &["ST23-05"]);
    let opp_low = runner.place_on_field(1, "OPP-LOW", Some(0));
    let opp_high = runner.place_on_field(1, "OPP-HIGH", Some(0));
    let opp_field_before = runner.battle_area_size(1);
    let opp_sec_before = runner.security_count(1);

    // Attack the HIGH-DP defender (not the player) so the just-placed security
    // card is not immediately broken by this attack's security check — the WD/WA
    // effect still fires on attack declaration.
    runner.attack_digimon(hab, opp_high, false);

    // Step A: the lowest-DP pick — only OPP-LOW (2000) is a legal target.
    let view = runner
        .pending_selection_view()
        .expect("lowest-DP place-on-security pick installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-DP (2000) opponent Digimon is a legal target; got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .unwrap();

    // Step B: the most-security 3-way choice — decline it for this test.
    if let Some(SelectionKind::EffectChoice) = runner.pending_kind() {
        let v = runner.pending_selection_view().unwrap();
        let last = v.effect_choices.as_ref().unwrap().len() - 1;
        runner
            .execute_action(v.selecting_player, v.valid_action_ids[last])
            .expect("decline the trash choice");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        opp_sec_before + 1,
        "the lowest-DP opponent Digimon became the opponent's top security card"
    );
    assert!(
        is_security_top_is(&runner, 1, "OPP-LOW"),
        "OPP-LOW (the lowest-DP Digimon) is the opponent's new top security card"
    );
}

/// Multi-target tie: two opponent Digimon tied for lowest DP surface a 2-way
/// pick (no auto-selection — no-approximations).
#[test]
fn st23_05_lowest_dp_tie_surfaces_multi_target_pick() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .add_card(opp_digimon("OPP-A", 3000))
        .add_card(opp_digimon("OPP-B", 3000))
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .memory(8)
        .start();
    runner.set_first_player(0);

    let hab = runner.place_stack(0, &["ST23-05"]);
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));

    runner.attack_player(hab, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("place-on-security pick installs");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "two Digimon tied for lowest DP → a 2-way pick (no auto-select)"
    );
}

// ─── Section 3 — most-security trash → Recovery +1 ──────────────────────────

/// POSITIVE: trashing the chosen player's top security recovers +1 to the
/// OWNER. With own security strictly greater than the opponent's, "trash own"
/// qualifies; net own security = +1 (Recovery) -1 (trash) = unchanged.
#[test]
fn st23_05_trash_own_security_recovers_to_owner() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .add_card(opp_digimon("OPP-LOW", 2000))
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .security(0, &["SEC"; 5]) // own (5) > opp (1) → "most" qualifies for own
        .security(1, &["SEC"; 1])
        .memory(8)
        .start();
    runner.set_first_player(0);

    let hab = runner.place_stack(0, &["ST23-05"]);
    runner.place_on_field(1, "OPP-LOW", Some(0));
    let own_sec_before = runner.security_count(0);

    runner.attack_player(hab, 1, false);
    // Step A: place the opponent's lowest-DP Digimon on security.
    let v = runner
        .pending_selection_view()
        .expect("place pick installs");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    // Step B: choose "trash your own top security card" (branch 0).
    let choice = runner
        .pending_selection_view()
        .expect("3-way choice installs");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner.execute_branch(0).expect("trash own");
    let _ = runner.auto_resolve();

    // Recovery +1 then trash -1 = net unchanged own security.
    assert_eq!(
        runner.security_count(0),
        own_sec_before,
        "Recovery +1 to the owner offsets the -1 trash of own security"
    );
}

/// NEGATIVE (most-security gate): when own security < opponent's, the "trash
/// your own" branch must not be a legal action.
#[test]
fn st23_05_own_trash_gated_when_not_most_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .add_card(opp_digimon("OPP-LOW", 2000))
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .security(0, &["SEC"; 1]) // own (1) < opp (5) → own does NOT have "most"
        .security(1, &["SEC"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);

    let hab = runner.place_stack(0, &["ST23-05"]);
    runner.place_on_field(1, "OPP-LOW", Some(0));
    let own_sec_before = runner.security_count(0);

    runner.attack_player(hab, 1, false);
    let v = runner
        .pending_selection_view()
        .expect("place pick installs");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    assert_most_security_choice(&runner, false, true);
    assert!(
        runner.execute_action(0, HAND_EFFECT_START).is_err(),
        "a player with fewer security cards is not a legal payment choice"
    );
    runner
        .execute_action(0, HAND_EFFECT_START + 2)
        .expect("decline the legal payment choice");
    runner.auto_resolve().expect("complete the attack");

    assert_eq!(
        runner.security_count(0),
        own_sec_before,
        "declining pays no security and recovers no card"
    );
}

fn most_security_choice_runner(own_count: usize, opponent_count: usize) -> DebugRunner {
    let own_security = vec!["SEC"; own_count];
    let opponent_security = vec!["SEC"; opponent_count];
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("Habakirimon compiles")
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .security(0, &own_security)
        .security(1, &opponent_security)
        .memory(8)
        .start();
    runner.set_first_player(0);
    let hab = runner.place_stack(0, &[CARD_ID]);
    // No opponent Digimon can be placed, but "Then" still reaches the
    // most-security choice via Habakirimon's real When Attacking trigger.
    runner.attack_player(hab, 1, false);
    runner
}

fn assert_most_security_choice(runner: &DebugRunner, own_legal: bool, opponent_legal: bool) {
    let view = runner
        .pending_selection_view()
        .expect("payable recovery choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    assert_eq!(view.selecting_player, 0);
    let expected_actions: Vec<_> = [own_legal, opponent_legal, true]
        .iter()
        .enumerate()
        .filter_map(|(index, &legal)| legal.then_some(HAND_EFFECT_START + index as u16))
        .collect();
    assert_eq!(view.valid_action_ids, expected_actions);
    let choices = view.effect_choices.as_ref().expect("named choices");
    assert_eq!(choices.len(), expected_actions.len());
    let mask = build_action_mask(&runner.game, 0);
    for (index, expected_legal) in [own_legal, opponent_legal, true].into_iter().enumerate() {
        assert_eq!(
            mask[HAND_EFFECT_START as usize + index] > 0.5,
            expected_legal,
            "choice {index} must match the payment's current legality"
        );
    }
    assert_eq!(
        mask.iter().filter(|&&legal| legal > 0.5).count(),
        expected_actions.len()
    );
    assert!(build_action_mask(&runner.game, 1)
        .iter()
        .all(|&legal| legal == 0.0));
}

#[test]
fn st23_05_most_security_choice_masks_the_lower_stack() {
    for (own, opponent) in [(4, 2), (1, 3), (1, 0), (0, 1)] {
        let mut runner = most_security_choice_runner(own, opponent);
        assert_most_security_choice(&runner, own > opponent, opponent > own);
        let illegal_action = HAND_EFFECT_START + u16::from(own > opponent);
        assert!(runner.execute_action(0, illegal_action).is_err());
        assert_eq!(
            runner.security_count(0),
            own,
            "no cost before a legal choice"
        );
        assert_eq!(runner.security_count(1), opponent);
        runner
            .execute_action(0, HAND_EFFECT_START + 2)
            .expect("decline stays legal regardless of which player has most");
        runner.auto_resolve().expect("complete the attack");
        assert_eq!(runner.security_count(0), own, "decline never recovers");
    }
}

#[test]
fn st23_05_most_security_choice_tie_offers_both_players() {
    let runner = most_security_choice_runner(2, 2);
    assert_most_security_choice(&runner, true, true);
}

#[test]
fn st23_05_most_security_choice_empty_stacks_skip_unpayable_menu() {
    let runner = most_security_choice_runner(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "only decline remains, so no choice is needed"
    );
    assert_eq!(runner.security_count(0), 0);
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(
        runner.deck_size(0),
        10,
        "unpayable recovery must not draw from deck"
    );
    assert!(
        runner.game_over(),
        "the attack proceeds after skipping the unpayable menu"
    );
    assert_eq!(runner.winner(), Some(0));
}

// ─── Section 4 — leave-prevention replacement (Glowing Dawn) ────────────────

fn leave_prevention_runner(security: &[&str]) -> DebugRunner {
    let mut glowing_dawn_tamer = sec_filler("GD-TAMER");
    glowing_dawn_tamer.traits = vec!["Glowing Dawn".to_string()];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("Habakirimon compiles")
        .dsl_card("BT25-032")
        .expect("Glowing Dawn ally compiles")
        .add_card(opp_digimon("NON-GD", 4000))
        .add_card(glowing_dawn_tamer)
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .security(0, security)
        .security(1, &["SEC"; 3])
        .memory(8)
        .start()
}

fn assert_leave_prevention_choice(runner: &DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("payable Glowing Dawn leave prevention must be offered");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert_eq!(view.selecting_player, 0, "Habakirimon's owner decides");
    assert!(runner.pending_is_optional(), "by trashing is optional");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[REPLACEMENT_ACCEPT as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
    assert_eq!(mask.iter().filter(|&&legal| legal > 0.5).count(), 2);
    assert!(
        build_action_mask(&runner.game, 1)
            .iter()
            .all(|&legal| legal == 0.0),
        "only the effect's controller may accept or decline"
    );
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_installs() {
    let mut runner = leave_prevention_runner(&["SEC", "NON-GD"]);
    let hab = runner.place_stack(0, &[CARD_ID]);
    let hab_card = runner.top_card(hab);
    let top_security = runner.game.player(0).security.last().unwrap().handle();

    runner
        .game
        .delete_permanents_batch(vec![hab], ReplacementCause::OpponentEffect);

    assert_leave_prevention_choice(&runner);
    assert_eq!(runner.security_count(0), 2, "no payment before consent");
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept prevention");
    runner.auto_resolve().expect("complete prevention");
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(runner.security_count(1), 3);
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(runner.game.player(0).trash[0].handle(), top_security);
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.top_card(hab), hab_card, "the same Digimon survives");
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_empty_security_is_not_offered() {
    // CR 15-7-4: an unpayable optional processing condition cannot be chosen.
    let mut runner = leave_prevention_runner(&[]);
    let hab = runner.place_stack(0, &[CARD_ID]);
    runner
        .game
        .delete_permanents_batch(vec![hab], ReplacementCause::OpponentEffect);

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(0), 0, "deletion proceeds");
    assert_eq!(runner.security_count(0), 0);
    assert_eq!(
        runner.security_count(1),
        3,
        "opponent cannot fund this cost"
    );
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(
        runner.game.player(0).trash[0].card_id(&runner.game.card_data),
        CARD_ID
    );
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_decline_leaves_opt_available() {
    let mut runner = leave_prevention_runner(&["SEC"; 2]);
    let hab = runner.place_stack(0, &[CARD_ID]);
    let hab_card = runner.top_card(hab);
    let ally = runner.place_stack(0, &["BT25-032"]);
    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OpponentEffect);
    assert_leave_prevention_choice(&runner);
    runner.execute_action(0, PASS).expect("decline prevention");
    runner.auto_resolve().expect("complete deletion");

    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.top_card(hab), hab_card);
    assert_eq!(runner.security_count(0), 2, "declining pays no security");
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(
        runner.game.player(0).trash[0].card_id(&runner.game.card_data),
        "BT25-032"
    );

    let later_ally = runner.place_stack(0, &["BT25-032"]);
    let later_card = runner.top_card(later_ally);
    runner
        .game
        .delete_permanents_batch(vec![later_ally], ReplacementCause::OpponentEffect);
    assert_leave_prevention_choice(&runner);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("unused OPT remains available after declining");
    runner.auto_resolve().expect("complete prevention");
    assert_eq!(runner.battle_area_size(0), 2);
    assert_eq!(runner.top_card(later_ally), later_card);
    assert_eq!(runner.security_count(0), 1);
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_payment_consumes_opt() {
    let mut runner = leave_prevention_runner(&["SEC"; 2]);
    runner.place_stack(0, &[CARD_ID]);
    let ally = runner.place_stack(0, &["BT25-032"]);
    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OpponentEffect);
    assert_leave_prevention_choice(&runner);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("pay prevention cost");
    runner.auto_resolve().expect("complete prevention");
    assert_eq!(runner.battle_area_size(0), 2);
    assert_eq!(
        runner.security_count(0),
        1,
        "another payment is still possible"
    );

    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection().is_none(),
        "OPT prevents another offer"
    );
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.security_count(0), 1, "no second cost");
    assert_eq!(runner.trash_size(0), 2);
    assert_eq!(
        runner.game.player(0).trash[1].card_id(&runner.game.card_data),
        "BT25-032"
    );
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_rejects_wrong_trait_kind_and_owner() {
    for (owner, target_id) in [(0, "NON-GD"), (0, "GD-TAMER"), (1, "BT25-032")] {
        let mut runner = leave_prevention_runner(&["SEC"; 2]);
        runner.place_stack(0, &[CARD_ID]);
        let target = runner.place_stack(owner, &[target_id]);
        runner
            .game
            .delete_permanents_batch(vec![target], ReplacementCause::OpponentEffect);

        assert!(
            runner.pending_selection().is_none(),
            "invalid target: {target_id}"
        );
        assert_eq!(runner.battle_area_size(0), 1, "Habakirimon remains alone");
        assert_eq!(runner.battle_area_size(1), 0);
        assert_eq!(
            runner.security_count(0),
            2,
            "invalid target cannot spend security"
        );
        assert_eq!(runner.trash_size(owner), 1);
        assert_eq!(
            runner.game.player(owner).trash[0].card_id(&runner.game.card_data),
            target_id
        );
    }
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_accepts_own_effect_on_opponents_turn() {
    let mut runner = leave_prevention_runner(&["SEC"]);
    runner.set_first_player(1);
    runner.place_stack(0, &[CARD_ID]);
    let ally = runner.place_stack(0, &["BT25-032"]);
    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OwnEffect);
    assert_leave_prevention_choice(&runner);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("all turns protection also applies to own effects");
    runner.auto_resolve().expect("complete prevention");
    assert_eq!(runner.battle_area_size(0), 2);
    assert_eq!(runner.security_count(0), 0);
}

#[test]
fn st23_05_glowing_dawn_leave_prevention_one_payment_saves_all_matching_simultaneous_leaves() {
    for security_count in [1, 2] {
        let security = vec!["SEC"; security_count];
        let mut runner = leave_prevention_runner(&security);
        let hab = runner.place_stack(0, &[CARD_ID]);
        let ally = runner.place_stack(0, &["BT25-032"]);
        let non_gd = runner.place_stack(0, &["NON-GD"]);
        let hab_card = runner.top_card(hab);
        let ally_card = runner.top_card(ally);
        let paid_card = runner.game.player(0).security.last().unwrap().handle();

        // "Any ... they don't leave": one payment protects all qualifying
        // Digimon in this simultaneous leave event, including Habakirimon itself.
        // A non-Glowing-Dawn Digimon in the same deletion remains unprotected.
        runner
            .game
            .delete_permanents_batch(vec![hab, ally, non_gd], ReplacementCause::OpponentEffect);
        assert_leave_prevention_choice(&runner);
        runner
            .execute_action(0, REPLACEMENT_ACCEPT)
            .expect("pay once for all matching simultaneous leaves");
        assert!(
            runner.pending_selection().is_none(),
            "one payment covers the whole matching leave group"
        );
        runner
            .auto_resolve()
            .expect("complete the simultaneous deletion");

        assert_eq!(
            runner.security_count(0),
            security_count - 1,
            "exactly one security pays"
        );
        assert_eq!(
            runner.battle_area_size(0),
            2,
            "both Glowing Dawn Digimon survive"
        );
        assert_eq!(runner.top_card(hab), hab_card);
        assert_eq!(runner.top_card(ally), ally_card);
        assert_eq!(
            runner.trash_size(0),
            2,
            "only payment and ineligible target leave"
        );
        assert!(runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|card| card.handle() == paid_card));
        assert!(runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|card| { card.card_id(&runner.game.card_data) == "NON-GD" }));
    }
}
