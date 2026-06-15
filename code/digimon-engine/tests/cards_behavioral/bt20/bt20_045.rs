//! BT20-045 Examon

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};

#[test]
fn bt20_045_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt20_045_has_blast_dna_digivolve_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT20-045").expect("compiled card");

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BlastDnaDigivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some("Breakdramon"))
            && path
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some("Slayerdramon"))
    }));
}

#[test]
fn bt20_045_counter_blast_dna_uses_breakdramon_and_slayerdramon() {
    let mut breakdramon = make_test_card_with_level("TEST-BREAKDRAMON", "Breakdramon", 6);
    breakdramon.colors = vec![CardColor::Green, CardColor::Red];
    breakdramon.dp = Some(12000);

    let mut slayerdramon = make_test_card_with_level("TEST-SLAYERDRAMON", "Slayerdramon", 6);
    slayerdramon.colors = vec![CardColor::Blue];
    slayerdramon.dp = Some(12000);

    let mut attacker = make_test_card_with_level("TEST-ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(17000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(breakdramon)
        .add_card(slayerdramon)
        .add_card(attacker)
        .hand(1, &["BT20-045", "TEST-SLAYERDRAMON"])
        .start();

    let attacking = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    let breakdramon = runner.place_on_field(1, "TEST-BREAKDRAMON", Some(0));

    let result = runner.attack_digimon(attacking, breakdramon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let counter_prompt = runner
        .pending_selection()
        .expect("Counter window should offer Examon Blast DNA");
    assert!(
        counter_prompt
            .valid_action_ids
            .contains(&DNA_DIGIVOLVE_START),
        "BT20-045 in hand slot 0 should be a Counter Blast DNA candidate: {:?}",
        counter_prompt.valid_action_ids
    );
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-045 for Counter Blast DNA");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    assert_eq!(
        runner
            .pending_selection()
            .expect("field material prompt")
            .valid_action_ids,
        vec![0]
    );

    runner
        .execute_action(1, 0)
        .expect("choose Breakdramon as the field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );

    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Slayerdramon as the hand material");

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BT20-045"
    );
    assert!(evolved
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "TEST-SLAYERDRAMON"));
    assert_eq!(runner.hand_size(1), 0);
}

// ─── [When Digivolving] if DNA digivolving, return ALL opponent Digimon with ──
//     the highest DP to the bottom of the deck (G-HIGHEST-DP-SWEEP).
//
// Printed text (cards.json effect_description_eng):
//   "[When Digivolving] If DNA digivolving, return all of your opponent's
//    Digimon with the highest DP to the bottom of the deck."
//
// The clause is DNA-origin-gated and returns EVERY opponent Digimon tied for
// the highest DP (highest_dp aggregate). Driven through the same Counter Blast
// DNA route the keyword-slice test uses (Blast DNA sets current_dna_origin), so
// the WhenDigivolving body sees dna_origin == true.

/// Helper: build a leveled Digimon with explicit DP/colors for the opponent
/// field. Kept LOCAL to this test file (no shared-support edits).
fn dp_digimon(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card_with_level(id, name, 6);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Purple];
    c
}

/// Drive BT20-045 into play via the Counter Blast DNA route (Breakdramon field
/// material + Slayerdramon hand material), with `extra_opp` opponent Digimon
/// already on player 0's field. Returns the runner positioned just AFTER the
/// Blast DNA resolves (and thus after the WhenDigivolving sweep fires).
fn counter_blast_dna_into_examon_with_opp_field(
    opp_field: &[digimon_engine::card_data::CardData],
) -> DebugRunner {
    let mut breakdramon = make_test_card_with_level("TEST-BREAKDRAMON", "Breakdramon", 6);
    breakdramon.colors = vec![CardColor::Green, CardColor::Red];
    breakdramon.dp = Some(12000);

    let mut slayerdramon = make_test_card_with_level("TEST-SLAYERDRAMON", "Slayerdramon", 6);
    slayerdramon.colors = vec![CardColor::Blue];
    slayerdramon.dp = Some(12000);

    let mut attacker = make_test_card_with_level("TEST-ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(17000);

    let mut builder = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(breakdramon)
        .add_card(slayerdramon)
        .add_card(attacker);
    for c in opp_field {
        builder = builder.add_card(c.clone());
    }
    let mut runner = builder
        .hand(1, &["BT20-045", "TEST-SLAYERDRAMON"])
        .start();

    // Player 0 (the opponent of the Blast-DNA-ing player 1) gets the field.
    let attacking = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    for c in opp_field {
        runner.place_on_field(0, &c.card_id, Some(0));
    }
    let breakdramon = runner.place_on_field(1, "TEST-BREAKDRAMON", Some(0));

    let result = runner.attack_digimon(attacking, breakdramon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-045 for Counter Blast DNA");
    runner
        .execute_action(1, 0)
        .expect("choose Breakdramon as field material");
    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Slayerdramon as hand material");
    runner
}

#[test]
fn bt20_045_dna_digivolve_bottom_decks_all_highest_dp_opp_digimon() {
    // Opponent field (player 0): two Digimon tied at the highest DP (12000) and
    // one lower (9000). Note the attacker (TEST-ATTACKER, DP 17000) is consumed
    // as the Counter target's irrelevant — it remains on field and is the
    // genuine highest unless deleted. To make the test deterministic, all
    // opponent field DP must be BELOW the surviving attacker so the attacker
    // would dominate; instead we give the high pair 18000 so THEY are highest.
    let high_a = dp_digimon("OPP-HIGH-A", "OppHighA", 18000);
    let high_b = dp_digimon("OPP-HIGH-B", "OppHighB", 18000);
    let low = dp_digimon("OPP-LOW", "OppLow", 9000);

    let mut runner =
        counter_blast_dna_into_examon_with_opp_field(&[high_a, high_b, low]);
    let _ = runner.auto_resolve();

    // The two 18000 Digimon are the highest → both returned to deck bottom.
    // Remaining on player 0's field: TEST-ATTACKER (17000) + OPP-LOW (9000) = 2.
    // (The attacker survived its own attack against Breakdramon, which was
    //  consumed as Blast DNA material, so the attack fizzled with no clash.)
    let p0_field: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !p0_field.iter().any(|id| id == "OPP-HIGH-A"),
        "highest-DP OppHighA must be bottom-decked: field = {p0_field:?}"
    );
    assert!(
        !p0_field.iter().any(|id| id == "OPP-HIGH-B"),
        "highest-DP OppHighB must be bottom-decked (all ties returned): field = {p0_field:?}"
    );
    assert!(
        p0_field.iter().any(|id| id == "OPP-LOW"),
        "the lower-DP OppLow must survive the sweep: field = {p0_field:?}"
    );
}

#[test]
fn bt20_045_dna_sweep_returns_to_bottom_of_deck() {
    // The single highest-DP opponent Digimon must land at the BOTTOM of player
    // 0's deck (the sweep fires synchronously during the Blast DNA resolution
    // inside the helper, so it is already off-field by the time we inspect).
    let high = dp_digimon("OPP-HIGH", "OppHigh", 18000);

    let mut runner = counter_blast_dna_into_examon_with_opp_field(&[high]);
    let _ = runner.auto_resolve();

    // It must have LEFT player 0's battle area...
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH"),
        "the highest-DP opponent Digimon must be returned (off the field)"
    );
    // ...and sit at the BOTTOM of the deck (index 0 per move_card_to_deck:
    // to_bottom inserts at index 0).
    let bottom_id = runner.game.players[0]
        .deck
        .first()
        .map(|cs| cs.card_id(&runner.game.card_data).to_string());
    assert_eq!(
        bottom_id.as_deref(),
        Some("OPP-HIGH"),
        "the returned Digimon must sit at the BOTTOM of the opponent's deck"
    );
}

#[test]
fn bt20_045_non_dna_digivolve_does_not_sweep() {
    // Negative: a STANDARD (non-DNA) digivolve into BT20-045 leaves all
    // opponent Digimon on the field — the dna_origin gate blocks the sweep.
    let mut base = make_test_card_with_level("TEST-GREEN-LV6", "GreenBase", 6);
    base.colors = vec![CardColor::Green];
    base.dp = Some(11000);

    let mut opp = dp_digimon("OPP-HIGH", "OppHigh", 18000);
    opp.colors = vec![CardColor::Purple];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(base)
        .add_card(opp)
        .hand(0, &["BT20-045"])
        .memory(10)
        .start();

    let opp_perm = runner.place_on_field(1, "OPP-HIGH", Some(0));
    let base_perm = runner.place_on_field(0, "TEST-GREEN-LV6", None);
    let opp_field_before = runner.battle_area_size(1);

    // Standard digivolve BT20-045 (Lv.6 green, cost 5) onto the green base.
    let hand_idx = 0usize;
    let succeeded = {
        let hand_card = runner.game.players[0].hand[hand_idx].handle();
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            hand_card,
            None,
            0,
        );
        ctx.effect_initiated_digivolve_ignore_requirements(
            0,
            hand_idx,
            base_perm,
            digimon_engine::enums::CostDelta::Free,
        )
    };
    assert!(succeeded, "standard digivolve into BT20-045 should succeed");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before,
        "non-DNA digivolve must NOT trigger the highest-DP sweep (dna_origin gate)"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH"),
        "opponent's highest-DP Digimon must remain after a non-DNA digivolve"
    );
    let _ = opp_perm;
}

// ─── [All Turns][Once Per Turn] when any Digimon suspend, this Digimon may ─────
//     unsuspend (G-SUSPEND-OBSERVER-UNSUSPEND).
//
// Printed text: "[All Turns] [Once Per Turn] When any Digimon suspend, this
// Digimon may unsuspend." — ANY Digimon (either player), no owner gate. Mirrors
// AD1-014's clause but WITHOUT the `event_target_owner: you` restriction, so an
// OPPONENT Digimon suspending also offers the unsuspend (DCGO
// IsPermanentExistsOnBattleAreaDigimon — any owner).

/// Place BT20-045 on a player's field and pre-suspend it, then return its
/// handle.
fn placed_suspended_examon(runner: &mut DebugRunner, player: u8) -> digimon_engine::permanent::PermanentHandle {
    let perm = runner.place_on_field(player, "BT20-045", Some(0));
    runner.game.players[player as usize].battle_area[perm.index as usize].is_suspended = true;
    perm
}

#[test]
fn bt20_045_unsuspends_when_own_digimon_suspends() {
    let mut ally = make_test_card_with_level("TEST-ALLY", "Ally", 4);
    ally.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(ally)
        .memory(20)
        .start();

    let examon = placed_suspended_examon(&mut runner, 0);
    let ally = runner.place_on_field(0, "TEST-ALLY", Some(0));

    // Suspending an OWN Digimon fires on_suspend → Examon may unsuspend.
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[examon.index as usize].is_suspended,
        "BT20-045 should unsuspend when any (own) Digimon suspends"
    );
}

#[test]
fn bt20_045_unsuspends_when_opponent_digimon_suspends() {
    // The defining difference from AD1-014: BT20-045 has NO owner gate, so an
    // OPPONENT Digimon suspending ALSO offers the unsuspend.
    let mut opp = make_test_card_with_level("TEST-OPP", "Opp", 4);
    opp.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(opp)
        .memory(20)
        .start();

    let examon = placed_suspended_examon(&mut runner, 0);
    let opp = runner.place_on_field(1, "TEST-OPP", Some(0));

    runner.game.suspend(opp);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[examon.index as usize].is_suspended,
        "BT20-045 should unsuspend when ANY Digimon (incl. opponent's) suspends \
         — no owner gate on the printed 'any Digimon'"
    );
}

#[test]
fn bt20_045_suspend_observer_is_once_per_turn() {
    // Second qualifying suspend in the same turn must NOT re-arm: re-suspend
    // Examon, suspend a second ally, and assert Examon stays suspended.
    let mut ally1 = make_test_card_with_level("TEST-ALLY1", "Ally1", 4);
    ally1.dp = Some(5000);
    let mut ally2 = make_test_card_with_level("TEST-ALLY2", "Ally2", 4);
    ally2.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(ally1)
        .add_card(ally2)
        .memory(20)
        .start();

    let examon = placed_suspended_examon(&mut runner, 0);
    let ally1 = runner.place_on_field(0, "TEST-ALLY1", Some(0));
    let ally2 = runner.place_on_field(0, "TEST-ALLY2", Some(0));

    // First suspend consumes the OPT and unsuspends Examon.
    runner.game.suspend(ally1);
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.players[0].battle_area[examon.index as usize].is_suspended,
        "first suspend should unsuspend Examon"
    );

    // Re-suspend Examon directly, then suspend a 2nd ally in the SAME turn.
    runner.game.players[0].battle_area[examon.index as usize].is_suspended = true;
    runner.game.suspend(ally2);
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0].battle_area[examon.index as usize].is_suspended,
        "Once Per Turn: the second suspend in the same turn must NOT re-arm the unsuspend"
    );
}
