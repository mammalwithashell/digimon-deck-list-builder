//! Behavioral tests for the quiz-derived rule-concept test cards
//! (`TEST-Q04-SECATK`, `TEST-Q10-CAPMEM`, `TEST-Q13-NEGDP`,
//! `TEST-Q16-ONDEL`, `TEST-Q17-EOTDEL`).
//!
//! Each block maps one or more ZXavier's Digi Rulings quiz questions
//! to the rule they hinge on, then drives the engine through the
//! minimal scenario that exercises that rule. The cards themselves
//! are synthetic — they're named and structured to make the rule
//! under test the only moving part.
//!
//! Source: https://docs.google.com/forms/d/e/1FAIpQLScqEKQIXYxaXZM-4YM4-94Zgkva2aTCDCK1H-RJcTM_uOKMKA/viewform

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};

// ─────────────────────────────────────────────────────────────────
// Local card factories (the default `make_test_card` produces a cost-3
// Lv.3 2000-DP Digimon, which is fine for OnPlay tests but too weak
// for combat scenarios).
// ─────────────────────────────────────────────────────────────────

fn dgmn(card_id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─────────────────────────────────────────────────────────────────
// Quiz Q4 — Atomic Inferno / Holy Flame: does the attacker do another
// security check after the first one resolves?
// Rule 16-3: [Security Attack +X] adds X to the number of checks an
// attack performs. A Digimon with SA+1 attacking a player consumes
// two security cards per declaration.
// ─────────────────────────────────────────────────────────────────

#[test]
fn q04_security_attack_plus_one_consumes_two_security_cards() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = dgmn("ATK", 8000);
            c.card_id = "TEST-Q04-SECATK".to_string();
            c.effect_class_name = "TEST_Q04_SECATK".to_string();
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-Q04-SECATK"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // Play the attacker so its OnPlay grants SecurityAttack +1 to itself.
    r.play(0, 0);
    let atk = r.perm_handle(0, 0);

    // Confirm the modifier landed as a permanent SecurityAttackChange +1.
    assert_eq!(
        r.game.modifiers.sum(atk, ModifierType::SecurityAttackChange),
        1,
        "OnPlay should grant SecurityAttackChange +1"
    );

    // Bypass summoning sickness by backdating the permanent's turn_played.
    r.game.players[0].battle_area[0].turn_played = 0;

    let sec_before = r.security_count(1);
    let _ = r.attack_player(atk, 1, false);

    assert_eq!(
        r.security_count(1),
        sec_before - 2,
        "Security Attack +1 should consume 2 security cards, not 1"
    );
}

#[test]
fn q04_baseline_no_security_attack_plus_consumes_one_card() {
    // Control: a vanilla attacker (no OnPlay SA+1 buff) consumes only
    // one security card per attack declaration.
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 8000))
        .add_card(make_test_card("FILLER", "Filler"))
        .security(1, &["FILLER", "FILLER", "FILLER"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let sec_before = r.security_count(1);
    let _ = r.attack_player(atk, 1, false);
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "baseline attacker should consume exactly 1 security card"
    );
}

// ─────────────────────────────────────────────────────────────────
// Quiz Q5 — Assembly at 0 memory: is playing a Digimon legal when
// memory is exactly at 0?
// Rule (4-1-2, 7-1): memory may go to the opponent's side (negative,
// down to -10). Paying a cost from 0 memory is legal as long as the
// resulting memory doesn't exceed the -10 floor. TEST-001 costs 3
// and gains 1 memory on play, so starting at 0 the play must resolve
// with memory at -2.
// ─────────────────────────────────────────────────────────────────

#[test]
fn q05_play_at_zero_memory_is_legal_and_memory_goes_negative() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(0)
        .start();

    let result = r.play(0, 0);
    assert_eq!(result, Some(0), "play at memory=0 should succeed");
    assert_eq!(r.battle_area_size(0), 1, "card should hit the field");
    assert_eq!(
        r.memory(),
        -2,
        "0 − 3 (cost) + 1 (OnPlay) = -2; memory crosses to opp's side"
    );
}

// ─────────────────────────────────────────────────────────────────
// Quiz Q10/Q11 — Mental Training: memory effects that would push
// memory above 10 are capped.
// Rule 4-1-2-2: "memory can't exceed 10 on your side".
// TEST-Q10-CAPMEM gains 20 memory on play; the resulting value is
// clamped to 10 regardless of the starting memory.
// ─────────────────────────────────────────────────────────────────

#[test]
fn q10_memory_gain_is_clamped_to_10() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q10-CAPMEM", "CapMem");
            c.play_cost = 0; // don't make cost part of the signal
            c
        })
        .hand(0, &["TEST-Q10-CAPMEM"])
        .memory(3)
        .start();

    r.play(0, 0);
    assert_eq!(
        r.memory(),
        10,
        "3 + 20 should clamp to the 10-memory ceiling, not 23"
    );
}

#[test]
fn q10_clamp_holds_even_when_starting_from_zero() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q10-CAPMEM", "CapMem");
            c.play_cost = 0;
            c
        })
        .hand(0, &["TEST-Q10-CAPMEM"])
        .memory(0)
        .start();
    r.play(0, 0);
    assert_eq!(r.memory(), 10, "gain 20 from 0 should clamp to 10");
}

// ─────────────────────────────────────────────────────────────────
// Quiz Q13/Q14 — Nyabootmon stacking -DP onto Rapidmon (X Antibody)
// / ShineGreymon: Ruin Mode: how much -DP is applied if Nyabootmon's
// effect fires three times?
// Rule 1-3-8: "Multiple DP modifications: calculate total modifier
// first, then apply to original". Three separate -3000 modifiers
// accumulate to -9000 on the same target.
// ─────────────────────────────────────────────────────────────────

#[test]
fn q13_three_negative_dp_modifiers_stack_additively() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q13-NEGDP", "NegDP");
            c.play_cost = 0;
            c
        })
        .add_card(dgmn("TARGET", 10000))
        .hand(0, &["TEST-Q13-NEGDP", "TEST-Q13-NEGDP", "TEST-Q13-NEGDP"])
        .memory(5)
        .start();

    // Seed opponent's field with a 10000-DP target at index 0.
    let target = r.place_on_field(1, "TARGET", Some(0));
    assert_eq!(r.dp_of(target), Some(10000));

    // Three separate plays each stamp a fresh -3000 modifier.
    r.play(0, 0);
    r.play(0, 0);
    r.play(0, 0);

    assert_eq!(
        r.dp_of(target),
        Some(1000),
        "three -3000 modifiers should total -9000 (10000 − 9000 = 1000)"
    );
}

#[test]
fn q13_single_negative_dp_modifier_baseline() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q13-NEGDP", "NegDP");
            c.play_cost = 0;
            c
        })
        .add_card(dgmn("TARGET", 10000))
        .hand(0, &["TEST-Q13-NEGDP"])
        .start();
    let target = r.place_on_field(1, "TARGET", Some(0));
    r.play(0, 0);
    assert_eq!(
        r.dp_of(target),
        Some(7000),
        "one -3000 modifier should leave target at 7000"
    );
}

// ─────────────────────────────────────────────────────────────────
// Quiz Q16 — Paildramon / Lilithmon: does [Partition]'s trigger fire
// when the Digimon is removed by an opponent's effect rather than in
// battle?
// Rule 15-16-4: "[On Deletion] effects trigger when the card with the
// effect is deleted." The rule does not distinguish between deletion
// by battle and deletion by effect. The
// `Game::delete_permanent_with_effects` entry point (used by
// effect-driven deletion) must fire OnDeletion triggers.
// ─────────────────────────────────────────────────────────────────

#[test]
fn q16_on_deletion_fires_when_deleted_by_effect() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q16-ONDEL", "OnDel");
            c.play_cost = 0;
            c
        })
        .hand(0, &["TEST-Q16-ONDEL"])
        .start();
    r.play(0, 0);
    let victim = r.perm_handle(0, 0);

    let mem_before = r.memory();
    r.game.delete_permanent_with_effects(victim);

    assert_eq!(r.battle_area_size(0), 0, "victim should be gone");
    assert_eq!(r.trash_size(0), 1, "victim should be in trash");
    assert_eq!(
        r.memory(),
        mem_before + 3,
        "OnDeletion must fire when removal is driven by an effect, not just by battle"
    );
}

// ─────────────────────────────────────────────────────────────────
// Quiz Q17 — Magnamon (X Antibody): does "[End of Your Turn] Delete
// this Digimon" trigger on the turn the Digimon is digivolved into?
// Rule 15-16-12-1: [End of Your Turn] fires "when your turn ends,"
// with no same-turn suppression. Ending the turn the card was played
// on must still fire the trigger and delete the permanent.
// ─────────────────────────────────────────────────────────────────

#[test]
fn q17_end_of_your_turn_self_delete_fires_same_turn() {
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q17-EOTDEL", "EotDel");
            c.play_cost = 0;
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-Q17-EOTDEL"])
        .security(1, &["FILLER"]) // keep P1 alive so turn rotation is allowed
        .start();
    r.play(0, 0);
    assert_eq!(r.battle_area_size(0), 1, "card is on field after play");

    r.end_turn();

    assert_eq!(
        r.battle_area_size(0),
        0,
        "EndOfYourTurn self-delete must remove the permanent"
    );
    assert_eq!(
        r.trash_size(0),
        1,
        "deleted permanent should land in trash"
    );
}

#[test]
fn q17_end_of_your_turn_only_fires_on_your_own_turn_end() {
    // Opposite-turn sanity check: ending the opponent's turn must NOT
    // delete a TEST-Q17-EOTDEL on P0's field (the trigger scope is
    // strictly your own turn ending).
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-Q17-EOTDEL", "EotDel");
            c.play_cost = 0;
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-Q17-EOTDEL"])
        .security(0, &["FILLER"])
        .security(1, &["FILLER"])
        .start();
    r.play(0, 0);
    assert_eq!(r.battle_area_size(0), 1);

    // End P0's turn first — this SHOULD delete it on P0's own EOT.
    // To check the opposite branch we skip that by re-placing the
    // permanent with a fresh turn baseline after rotation.
    r.end_turn();
    assert_eq!(r.battle_area_size(0), 0, "P0's EOT deletes the card");

    // Now we're on P1's turn. Put a fresh copy on P0's field and
    // end P1's turn — P0's [End of Your Turn] trigger must NOT fire
    // because it's not P0's turn ending.
    r.place_on_field(0, "TEST-Q17-EOTDEL", Some(0));
    assert_eq!(r.battle_area_size(0), 1);
    r.end_turn();
    assert_eq!(
        r.battle_area_size(0),
        1,
        "P0's [End of Your Turn] must not fire at the end of P1's turn"
    );
}
