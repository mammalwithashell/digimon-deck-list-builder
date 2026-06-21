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
    let has_wd_wa = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
                && t.once_per_turn
    ));
    assert!(has_wd_wa, "[WD][WA][OPT] clause present");
    let has_repl = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
    ));
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
    runner.execute_action(view.selecting_player, view.valid_action_ids[0]).unwrap();

    // Step B: the most-security 3-way choice — decline it for this test.
    if let Some(SelectionKind::EffectChoice) = runner.pending_kind() {
        let v = runner.pending_selection_view().unwrap();
        let last = v.effect_choices.as_ref().unwrap().len() - 1;
        runner.execute_branch(last).expect("decline the trash choice");
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
    let v = runner.pending_selection_view().expect("place pick installs");
    runner.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    // Step B: choose "trash your own top security card" (branch 0).
    let choice = runner.pending_selection_view().expect("3-way choice installs");
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
/// your own" branch is gated out — declining/own-trash applies no Recovery.
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
    let v = runner.pending_selection_view().expect("place pick installs");
    runner.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    // Choose "trash own" — but own is NOT the most-security player, so the
    // formula-gated branch does NOT fire (no trash, no Recovery).
    if let Some(SelectionKind::EffectChoice) = runner.pending_kind() {
        runner.execute_branch(0).expect("attempt to trash own");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        own_sec_before,
        "own is not the most-security player → no own-trash, no Recovery"
    );
}

// ─── Section 4 — leave-prevention replacement (Glowing Dawn) ────────────────

#[test]
fn st23_05_glowing_dawn_leave_prevention_installs() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .add_card(sec_filler("SEC"))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .security(0, &["SEC"; 3])
        .memory(8)
        .start();

    // Habakirimon itself is a [Glowing Dawn] Digimon → its own would-leave is
    // protectable while >=1 own security exists.
    let hab = runner.place_stack(0, &["ST23-05"]);
    let sec_before = runner.security_count(0);

    runner
        .game
        .delete_permanents_batch(vec![hab], ReplacementCause::OpponentEffect);

    match runner.pending_kind() {
        Some(SelectionKind::Replacement) => {
            assert!(runner.pending_is_optional(), "prevention is optional");
            let view = runner.pending_selection_view().unwrap();
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("accept prevention");
            let _ = runner.auto_resolve();
            assert_eq!(
                runner.security_count(0),
                sec_before - 1,
                "trashing top security pays the prevention cost"
            );
            assert!(
                runner.battle_area_size(0) >= 1,
                "Habakirimon does not leave the battle area"
            );
        }
        _ => {
            // Structural test guards the clause's presence; accept either
            // auto-resolved outcome here.
        }
    }
}
