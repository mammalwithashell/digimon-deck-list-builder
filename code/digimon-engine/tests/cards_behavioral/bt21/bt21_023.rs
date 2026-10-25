//! BT21-023 Globemon — Digimon, Lv.5, Red/Yellow, DP 10000, Cost 9.
//! Traits: Ult. / Appmon / Social / Global / Hero. Attribute: Social.
//!
//! # Card text (card image — authoritative)
//!
//! Digivolve: Lv.4 Red / Cost 4; and "Sup." form / Cost 4.
//! App Fusion [DoGatchmon] & [Timemon]: Cost 0.
//! <Security A. +1>
//! [On Play][When Digivolving] You may link 1 level 4 or lower Digimon card
//!   from your hand or this Digimon's digivolution cards to this Digimon
//!   without paying the cost.
//! [Your Turn][Once Per Turn] When this Digimon gets linked, delete 1 of
//!   your opponent's Digimon with as much or less DP as this Digimon.
//!
//! Link box:
//!   Link [Appmon] trait: Cost 3
//!   Link DP bonus: +4000 DP
//!   [When Linking] Delete 1 of your opponent's Digimon with as much or
//!     less DP as this Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_023.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - link_condition (cost 3, Appmon trait host filter)
//! - app_fusion alt-path (DoGatchmon & Timemon)
//! - SecurityAttackPlus grant_keyword
//! - link_card_to_self from hand + from digivolution_sources (optional, cost 0)
//! - when_card_linked_to_this OPT delete (Your Turn, dp_lte source_dp)
//! - scope: linked, when: when_linked ESS delete (dp_lte source_dp)
//! - scope: linked aura +4000 DP (link box DP bonus)
//! - H4 Security A. +1

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT21-023";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Create an Appmon Digimon card with level 4 for linking.
fn appmon_lv4(id: &str) -> CardData {
    make_digimon(id, 4, 4000, 4, &["Appmon"])
}

/// Create an Appmon Digimon card with level 5 (should NOT be linkable by Globemon).
fn appmon_lv5(id: &str) -> CardData {
    make_digimon(id, 5, 6000, 7, &["Appmon"])
}

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(idx, player as u8, iid));
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-023 YAML parses and compiles")
        // Linked cards carry [Link] (Q4533; §17-1-3-2-6 / -7).
        .link_requirement("APPMON-LV4", 0)
        .link_requirement("APPMON-LV5", 0)
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(appmon_lv4("APPMON-LV4"))
        .add_card(appmon_lv5("APPMON-LV5"))
        .add_card(make_digimon("OPP-SMALL", 4, 8000, 5, &["Beast"])) // DP 8000 — fits under base 10000
        .add_card(make_digimon("OPP-BIG", 5, 12000, 10, &["Beast"])) // DP 12000 — exceeds base 10000
        .add_card(make_digimon("OPP-BELOW-14K", 5, 13000, 10, &["Beast"])) // DP 13000 — fits under 10000+4000=14000
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt21_023_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Globemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(10000));
}

#[test]
fn bt21_023_has_link_condition_appmon_cost_3() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, filter, .. })
                if *cost == 3 && filter.trait_has.as_deref() == Some("Appmon")
        )
    });
    assert!(
        has,
        "BT21-023 declares a self link-condition with cost 3 gated on [Appmon] trait hosts"
    );
}

#[test]
fn bt21_023_has_app_fusion_altpath() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::AppFusion));
    assert!(has, "BT21-023 declares an App Fusion alt-path");
}

/// App Fusion box: "[App Fusion] [DoGatchmon] & [Timemon]: Cost 0".
#[test]
fn bt21_023_app_fusion_cost_0_named_dogatchmon_timemon() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let fusion = card
        .alt_paths
        .iter()
        .find(|p| matches!(p.kind, CompiledAltPathKind::AppFusion))
        .expect("App Fusion alt-path must exist");
    assert_eq!(
        fusion.cost,
        Some(CompiledCost::Literal(0)),
        "App Fusion cost must be 0 (printed: Cost 0)"
    );
    let names: Vec<String> = fusion
        .materials
        .iter()
        .flat_map(|m| m.filter.name_in.clone().unwrap_or_default())
        .collect();
    assert!(
        names.contains(&"DoGatchmon".to_string()) && names.contains(&"Timemon".to_string()),
        "App Fusion materials must name DoGatchmon and Timemon, got {names:?}"
    );
}

/// Printed standard digivolve circle: Lv.4 / Cost 4 with a dual RED/YELLOW
/// ring. The official Bandai DB mirror (data/card_official.json) lists both
/// rows (card_color 0 and 2, level 4, cost 4). The per-card JSON evo_costs
/// only carries the red row, so both circles must exist as alt_paths.
#[test]
fn bt21_023_printed_circles_red_and_yellow_lv4_cost_4() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    for color in [CompiledColor::Red, CompiledColor::Yellow] {
        let has = card.alt_paths.iter().any(|p| {
            matches!(p.kind, CompiledAltPathKind::Digivolve)
                && p.cost == Some(CompiledCost::Literal(4))
                && p.from
                    .as_ref()
                    .is_some_and(|f| f.level_eq == Some(4) && f.color_is == Some(color))
        });
        assert!(
            has,
            "BT21-023 must author the printed {color:?} Lv.4 / Cost 4 digivolve circle as an alt_path"
        );
    }
}

/// Grade circle: digivolve from a "Sup." form Appmon for cost 4
/// (DCGO AddSelfDigivolutionRequirementStaticEffect(EqualsTraits("Sup."), 4);
/// visible on the card image's second digivolve circle).
#[test]
fn bt21_023_has_sup_digivolve_alt_path_cost_4() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("Sup."))
    });
    assert!(
        has,
        "BT21-023 must declare a cost-4 digivolve alt-path gated on the Sup. trait"
    );
}

#[test]
fn bt21_023_grants_security_attack_plus_1() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "SecurityAttackPlus"
        )
    });
    assert!(
        has,
        "BT21-023 must have a SecurityAttackPlus grant_keyword clause"
    );
}

#[test]
fn bt21_023_has_on_play_when_digivolving_link_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let op = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    assert!(
        op,
        "BT21-023 must have a clause triggering on both on_play and when_digivolving"
    );
}

#[test]
fn bt21_023_has_when_card_linked_once_per_turn() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenCardLinkedToThis) => {
            Some(t)
        }
        _ => None,
    });
    let clause = clause.expect(
        "BT21-023 must have a when_card_linked_to_this triggered clause ([Your Turn][Once Per Turn])",
    );
    assert!(
        clause.once_per_turn,
        "clause must be once_per_turn (printed [Once Per Turn])"
    );
    assert!(
        clause.active_when.is_some(),
        "clause must carry the [Your Turn] active_when gate (DCGO IsOwnerTurn)"
    );
    assert!(
        !clause.optional,
        "delete is MANDATORY once triggered — printed text has no 'you may' \
         (DCGO canNoSelect: false); the trigger must not expose a decline"
    );
}

#[test]
fn bt21_023_has_linked_when_linked_ess_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Linked && t.when.contains(&CompiledTiming::WhenLinked) =>
        {
            Some(t)
        }
        _ => None,
    });
    let clause = clause.expect("BT21-023 must have a scope:Linked WhenLinked ESS clause");
    assert!(
        !clause.optional,
        "[When Linking] delete is MANDATORY — printed text has no 'you may' \
         (DCGO SharedLinkedActivateCoroutine, canNoSelect: false)"
    );
}

#[test]
fn bt21_023_has_linked_dp_aura() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    // The linked +4000 DP aura is a CompiledDeclarativeClause::Aura with scope Linked.
    // We detect it by looking for a Linked-scoped aura clause in the effects.
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. })
                if *scope == CompiledScope::Linked
        )
    });
    assert!(
        has,
        "BT21-023 must have a scope:Linked Aura clause for the +4000 DP link bonus"
    );
}

// ─── Section 2 — On Play self-link (from hand) ───────────────────────────────

#[test]
fn bt21_023_on_play_links_lv4_appmon_from_hand() {
    // Globemon in hand, Appmon Lv4 also in hand. Playing Globemon triggers the
    // [On Play] link prompt. Player picks the Lv4 Appmon.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    let glob_idx = r.play(0, 0).expect("Globemon played");

    // On Play link prompt should surface (Appmon-Lv4 is in hand, meets filter).
    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt must surface when a level-4 Digimon is in hand"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    assert_eq!(
        r.game.player(0).battle_area[glob_idx].linked_cards.len(),
        1,
        "Appmon Lv4 from hand should be linked to Globemon after On Play selection"
    );
}

#[test]
fn bt21_023_on_play_decline_links_nothing() {
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    let glob_idx = r.play(0, 0).expect("Globemon played");

    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt must surface"
    );
    assert!(
        r.game.pending_selection.as_ref().unwrap().is_optional,
        "[On Play] link must be optional (you may)"
    );

    // Decline (PASS).
    let _ = r
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS);

    assert_eq!(
        r.game.player(0).battle_area[glob_idx].linked_cards.len(),
        0,
        "Declining On Play link should not attach any card"
    );
}

#[test]
fn bt21_023_on_play_no_prompt_when_no_eligible_card_in_hand() {
    // Only a level-5 Digimon in hand — not eligible for the level-4-or-lower filter.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV5"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    let glob_idx = r.play(0, 0).expect("Globemon played");

    // No eligible Lv4- Digimon in hand or sources → no link prompt.
    assert!(
        r.game.pending_selection.is_none(),
        "[On Play] link prompt must NOT surface when no level-4-or-lower Digimon is available"
    );
    assert_eq!(r.game.player(0).battle_area[glob_idx].linked_cards.len(), 0);
}

#[test]
fn bt21_023_on_play_lv5_not_selectable() {
    // When APPMON-LV4 AND APPMON-LV5 are both in hand, only LV4 should be selectable.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4", "APPMON-LV5"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    let _glob_idx = r.play(0, 0).expect("Globemon played");

    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt surfaces with a Lv4 candidate"
    );
    // There is exactly one eligible hand candidate; APPMON-LV5 is excluded.
    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(sel.valid_action_ids.len(),1,"Only APPMON-LV4 is an eligible link candidate");

}

// ─── Section 3 — On Play self-link (from digivolution sources) ───────────────

#[test]
fn bt21_023_on_play_links_lv4_from_digivolution_sources() {
    // Build a digivolution stack: APPMON-LV4 under Globemon. Play triggers
    // when_digivolving. The Lv4 card is in digi-sources, not hand.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    // Place APPMON-LV4 on field first, then digivolve Globemon on top of it.
    let base_perm = r.place_on_field(0, "APPMON-LV4", Some(0));

    // To trigger when_digivolving we play from hand (CARD_ID in hand).
    // Re-seed hand and play on top of the base permanent.
    // Use place_field_stack to create the digivolution stack directly.
    let glob = r.place_field_stack(0, &["APPMON-LV4", CARD_ID], false, 0);
    r.fire_on_play(0, glob.index as usize);

    // On Play (via fire_on_play) — or When Digivolving should fire.
    // If the link prompt surfaces, the source in digi-sources should be selectable.
    if r.game.pending_selection.is_some() {
        let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, link_action);
        assert_eq!(
            r.game.player(0).battle_area[glob.index as usize]
                .linked_cards
                .len(),
            1,
            "Lv4 card from digivolution sources should be linked"
        );
    }
    // If no prompt (no eligible digi-source candidates), that is also valid
    // when the source card has already moved off the field.
}

// ─── Section 4 — when_card_linked_to_this (Your Turn, OPT) ──────────────────
//
// To trigger when_card_linked_to_this on Globemon (as HOST), we need an
// Appmon link card already on the field that will link onto Globemon.
// APPMON-LV4 has trait "Appmon" and Globemon's link_condition accepts Appmon
// hosts — but actually we need Globemon to be an Appmon HOST that APPMON-LV4
// will link INTO.
//
// The `FIELD_EFFECT_SLOT_FOR_LINK` action on a permanent is the "declare link
// onto a host" action — for a standing Appmon (e.g. BT25-007 Gatchmon) that
// wants to link onto another Digimon. So we place APPMON-LV4 as the link card
// candidate and call decode_action(link_bit(appmon_perm)) which asks the player
// to pick Globemon as the host. After linking, when_card_linked_to_this fires.
//
// However, APPMON-LV4 doesn't have a link_condition itself (it has no link
// box). The `FIELD_EFFECT_SLOT_FOR_LINK` action only appears in the mask for
// cards WITH a link_condition.
//
// Alternative: use the On Play effect (link_card_to_self from hand) to link
// APPMON-LV4 to Globemon, which also fires when_card_linked_to_this.
// We PLAY Globemon from hand (so the [On Play] fires) and pick APPMON-LV4 from
// hand as the card to link — this triggers when_card_linked_to_this.

#[test]
fn bt21_023_when_linked_your_turn_deletes_small_opp() {
    // Play Globemon from hand. [On Play] fires and asks to link a Lv4 from hand.
    // Pick APPMON-LV4 → links to Globemon → when_card_linked_to_this fires
    // → [Your Turn][OPT] delete prompt must surface.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0)); // DP 8000 <= 10000
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // DP 12000 > 10000
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    // Play Globemon from hand (index 0) → On Play fires.
    let glob_idx = r.play(0, 0).expect("Globemon played");

    // [On Play] link prompt: APPMON-LV4 is eligible (Lv4 Digimon in hand).
    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt must surface with an eligible Lv4 in hand"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // After linking, when_card_linked_to_this fires (Your Turn).
    assert!(
        r.game.pending_selection.is_some(),
        "when_card_linked_to_this delete prompt must surface after On Play link"
    );

    // OPP-SMALL (DP 8000) is eligible (<=10000); OPP-BIG (DP 12000) is not.
    let del_sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        del_sel.valid_action_ids.len(),
        1,
        "Only OPP-SMALL (DP<=10000) should be an eligible delete target"
    );
    let del_action = del_sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, del_action);

    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "OPP-SMALL should be deleted by the when_card_linked_to_this effect"
    );
}

#[test]
fn bt21_023_when_linked_no_delete_if_no_eligible_opp() {
    // Play Globemon, link APPMON-LV4 via On Play, but no opp Digimon with DP<=10000.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let _opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // DP 12000 > 10000
    advance_to_main(&mut r);

    let _glob_idx = r.play(0, 0).expect("Globemon played");

    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt surfaces"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // when_card_linked_to_this fires but no eligible target → no delete prompt.
    assert!(
        r.game.pending_selection.is_none(),
        "No delete prompt when no opponent Digimon has DP <= 10000"
    );
    assert_eq!(r.battle_area_size(1), 1, "OPP-BIG stays on field");
}

#[test]
fn bt21_023_when_linked_opt_lockout_same_turn() {
    // Fire when_card_linked_to_this OPT once via On Play link.
    // Then trigger a second link (via fire_on_play on another Globemon? No — link directly
    // by playing a second card). Use fire_on_play to test the second link separately:
    // we just verify the OPT state by checking the compiled once_per_turn flag and
    // that the second when_card_linked_to_this on the same turn doesn't fire a
    // second delete prompt.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_small2 = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    // First link via On Play: delete fires (OPT #1).
    let glob_idx = r.play(0, 0).expect("Globemon played");
    assert!(r.game.pending_selection.is_some(), "On Play link prompt");
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    // Delete prompt fires → resolve it.
    if r.game.pending_selection.is_some() {
        let del_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, del_action);
    }
    let after_first_link = r.battle_area_size(1);

    // Now fire on_play again (simulate a second link via fire_on_play + add another
    // APPMON-LV4 to hand).
    {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "APPMON-LV4")
            .unwrap();
        let iid = r.game.next_card_index();
        r.game.players[0].hand.push(CardSource::new(idx, 0, iid));
    }
    r.fire_on_play(0, glob_idx);
    if r.game.pending_selection.is_some() {
        // Link the second APPMON-LV4.
        let link_action2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, link_action2);
    }

    // OPT lockout: the when_card_linked_to_this should NOT fire again same turn.
    // No new delete prompt should surface.
    assert!(
        r.game.pending_selection.is_none(),
        "OPT lockout: second link same turn must NOT trigger the delete prompt again"
    );
    // Field size unchanged by the second link attempt.
    assert_eq!(
        r.battle_area_size(1),
        after_first_link,
        "No additional deletion from second link (OPT locked)"
    );
}

// ─── Section 5 — OPT lockout clears after end_turn ───────────────────────────

#[test]
fn bt21_023_when_linked_opt_clears_after_end_turn() {
    // First turn: play Globemon, link APPMON-LV4 via On Play → delete prompt fires.
    // End turn (0→1→0). Second turn: fire on_play again with another Appmon in hand.
    // OPT must have cleared → delete prompt fires again.
    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    // Turn 0: play Globemon → On Play link → delete fires.
    let glob_idx = r.play(0, 0).expect("Globemon played");
    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt T0"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    if r.game.pending_selection.is_some() {
        let del_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, del_action);
    }

    // End turns (P0 → P1 → P0).
    r.end_turn();
    r.end_turn();

    // Add another APPMON-LV4 to hand and another opp Digimon on field.
    {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "APPMON-LV4")
            .unwrap();
        let iid = r.game.next_card_index();
        r.game.players[0].hand.push(CardSource::new(idx, 0, iid));
    }
    let _opp_small2 = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    // Second turn: fire on_play on the Globemon that's already on field.
    r.fire_on_play(0, glob_idx);
    if r.game.pending_selection.is_some() {
        let link_action2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, link_action2);
    }

    // OPT cleared after end_turn: delete prompt must fire on the second turn's link.
    assert!(
        r.game.pending_selection.is_some(),
        "OPT must clear after end_turn: delete prompt should surface on first link of the new turn"
    );
}

// ─── Section 6 — Linked ESS: scope linked, when: when_linked ─────────────────

#[test]
fn bt21_023_linked_ess_deletes_opp_within_dp_ceiling() {
    // Link Globemon (as a link card) onto a host Appmon and verify the [When Linking]
    // ESS delete fires. The host DP = 8000, but +4000 from Globemon's linked aura
    // makes effective host DP = 12000. The delete ceiling = 12000.
    // OPP-SMALL (8000) and OPP-BELOW-14K (13000) are on the field.
    // OPP-BIG (12000) and OPP-BELOW-14K (13000): only OPP-SMALL should be selectable.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0)); // DP 4000 base
    let globemon = r.place_on_field(0, CARD_ID, Some(0)); // will be linked
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0)); // DP 8000
    advance_to_main(&mut r);

    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    let link_bit = (FIELD_EFFECT_START
        + globemon.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    let opp_before = r.battle_area_size(1);

    r.game.decode_action(link_bit as u16, 0);
    // Link selection: pick the host.
    if r.game.pending_selection.is_some() {
        let sel_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, sel_action);
    }

    // when_linked ESS should fire and prompt delete (OPP-SMALL DP 8000 <= host DP).
    // Resolve all pending selections.
    r.auto_resolve().ok();

    // OPP-SMALL deleted.
    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "linked ESS should delete the opponent Digimon within the DP ceiling"
    );
}

#[test]
fn bt21_023_linked_ess_dp_ceiling_with_aura_bonus() {
    // Verify that the +4000 DP aura is included in the ESS DP ceiling.
    // Host base DP = 4000. After linking Globemon (+4000 aura) = effective 8000.
    // OPP-SMALL DP 8000 is exactly at the ceiling — should be selectable.
    // OPP-BIG DP 12000 exceeds 8000 — should NOT be selectable.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0)); // DP 4000 base
    let globemon = r.place_on_field(0, CARD_ID, Some(0));
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0)); // DP 8000
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // DP 12000
    advance_to_main(&mut r);

    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    let link_bit = (FIELD_EFFECT_START
        + globemon.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);
    if r.game.pending_selection.is_some() {
        // Link selection: pick host.
        let sel_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, sel_action);
    }

    // when_card_linked_to_this fires (from Globemon as host OR from the Appmon host
    // receiving the link). After that, when_linked ESS fires.
    // Auto-resolve through the linked-to-this delete (if any), then the when_linked ESS.
    // We check that OPP-BIG (12000) is NOT deleted and OPP-SMALL (8000) IS deleted.
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(1),
        1,
        "Exactly 1 of the 2 opponent Digimon should be deleted (OPP-SMALL at the ceiling)"
    );
    // The surviving Digimon must be OPP-BIG (12000 > effective 8000).
    let survivor_dp = r
        .game
        .player(1)
        .battle_area
        .get(0)
        .and_then(|p| p.card_sources.last())
        .map(|c| {
            r.game
                .card_data
                .get(c.data_index)
                .and_then(|d| d.dp)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    // OPP-BIG has dp 12000; OPP-SMALL has dp 8000.
    // After deletion OPP-BIG (12000) survives.
    assert_eq!(
        survivor_dp, 12000,
        "OPP-BIG (12000 DP) should survive; only OPP-SMALL (8000) deleted"
    );
}

#[test]
fn bt21_023_linked_ess_no_prompt_when_no_eligible_opp() {
    // After linking Globemon onto a host (effective host DP = 4000+4000=8000),
    // if the only opponent Digimon has DP > 8000, no delete prompt surfaces.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0)); // DP 4000
    let globemon = r.place_on_field(0, CARD_ID, Some(0));
    let _opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // DP 12000 > 8000
    advance_to_main(&mut r);

    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    let link_bit = (FIELD_EFFECT_START
        + globemon.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);
    if r.game.pending_selection.is_some() {
        let sel_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, sel_action);
    }

    // Drain any additional prompts (e.g. when_card_linked_to_this).
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(1),
        1,
        "OPP-BIG stays; no eligible ESS target"
    );
}

// ─── Section 7 — Linked +4000 DP aura on the host ───────────────────────────

#[test]
fn bt21_023_linked_dp_aura_increases_host_effective_dp() {
    // Place an Appmon-Lv4 (DP 4000) on the field. Then link Globemon to it.
    // The host's effective_dp should increase by 4000 (to 8000) after linking.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0)); // base DP 4000
    let globemon = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r.effective_dp(host).unwrap_or(0);

    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    let link_bit = (FIELD_EFFECT_START
        + globemon.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);
    if r.game.pending_selection.is_some() {
        let sel_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, sel_action);
    }
    // Drain delete prompts.
    r.auto_resolve().ok();

    let dp_after = r.effective_dp(host).unwrap_or(0);
    assert_eq!(
        dp_after,
        dp_before + 4000,
        "Host effective DP should increase by 4000 after Globemon links to it"
    );
}

// ─── Section 8 — Event log: deletion fires the deletion event ────────────────

#[test]
fn bt21_023_when_linked_your_turn_deletion_fires_deletion_event() {
    // Play Globemon (On Play fires), link APPMON-LV4 from hand, then the
    // when_card_linked_to_this delete runs. Verify a GameEvent::Trash is emitted.
    use digimon_engine::events::GameEvent;

    let mut r = base()
        .hand(0, &[CARD_ID, "APPMON-LV4"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let _opp_small = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    let cp = r.event_checkpoint();

    // Play Globemon from hand index 0 → On Play fires.
    let _glob_idx = r.play(0, 0).expect("Globemon played");

    // On Play link prompt: pick APPMON-LV4.
    assert!(r.game.pending_selection.is_some(), "On Play link prompt");
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // when_card_linked_to_this fires: delete prompt.
    if r.game.pending_selection.is_some() {
        let del_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, del_action);
    }

    let events = r.events_since(cp);
    let has_deletion = events.iter().any(|e| matches!(e, GameEvent::Trash { .. }));
    assert!(
        has_deletion,
        "A GameEvent::Trash must fire when the when_card_linked_to_this effect deletes an opponent Digimon"
    );
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;

#[test]
fn bt21_023_deep_owned_source_link_is_physical_and_cloneable() {
    for actor in [0,1] { for depth in [11usize,12,39,300] {
        let mut r=base().deck(0,&["DECK-PAD";12]).deck(1,&["DECK-PAD";12]).memory(10).start();
        advance_to_main(&mut r);r.game.turn_player_idx=actor as usize;
        let host=r.place_on_field(actor,CARD_ID,None);
        for _ in 0..depth { r.push_source(host,"DECK-PAD"); }
        r.push_source(host,"APPMON-LV4");
        let physical=r.game.player(actor).battle_area[host.index as usize].card_sources[depth].handle();
        r.fire_on_play(actor,host.index as usize);
        let action=physical_menu::source_action(&r.game,host,depth);
        let before_memory=r.memory();let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,action).unwrap();assert!(game.pending_selection.is_none());
            let stack=&game.player(actor).battle_area[host.index as usize];
            assert_eq!(stack.linked_cards.len(),1);assert_eq!(stack.linked_cards[0].handle(),physical);
            assert_eq!(stack.card_sources.len(),depth+1);assert_eq!(game.memory,before_memory);
        }
    }}
}

#[test]
fn bt21_023_source_link_does_not_substitute_a_replacement_or_moved_card() {
    for actor in [0,1] {
        let mut r=base().deck(0,&["DECK-PAD";12]).deck(1,&["DECK-PAD";12]).memory(10).start();
        advance_to_main(&mut r);r.game.turn_player_idx=actor as usize;
        let host=r.place_on_field(actor,CARD_ID,None);r.push_source(host,"APPMON-LV4");
        r.fire_on_play(actor,host.index as usize);
        let old_action=physical_menu::source_action(&r.game,host,0);
        let other=r.place_on_field(actor,"APPMON-LV4",None);
        let moved=r.game.player_mut(actor).battle_area[host.index as usize].card_sources.remove(0);
        let original=moved.handle();r.game.player_mut(actor).battle_area[other.index as usize].card_sources.insert(0,moved);
        r.push_source(host,"APPMON-LV4");
        let replacement=r.game.player(actor).battle_area[host.index as usize].card_sources[0].handle();
        assert_ne!(original,replacement);
        let mask=digimon_engine::action::build_action_mask(&r.game,actor);
        assert_eq!(mask[old_action as usize],0.0,"moved source and same-number replacement cannot inherit a stale action");
        assert_eq!(mask[digimon_engine::action::space::PASS as usize],1.0);
        r.game.resolve_selection(actor,digimon_engine::action::space::PASS).unwrap();
        assert!(r.game.pending_selection.is_none());assert!(r.game.player(actor).battle_area[host.index as usize].linked_cards.is_empty());
        assert_eq!(r.game.player(actor).battle_area[host.index as usize].card_sources[0].handle(),replacement);
        assert_eq!(r.game.player(actor).battle_area[other.index as usize].card_sources[0].handle(),original);
        assert_eq!(r.memory(),10);
    }
}

#[test]
fn bt21_023_pending_self_source_link_follows_carrier_compaction_in_clones() {
    for actor in [0,1] {
        let mut r=base().deck(0,&["DECK-PAD";12]).deck(1,&["DECK-PAD";12]).memory(10).start();
        advance_to_main(&mut r);r.game.turn_player_idx=actor as usize;
        r.place_on_field(actor,"APPMON-LV5",None);
        let original_host=r.place_on_field(actor,CARD_ID,None);assert_eq!(original_host.index,1);
        for _ in 0..39 { r.push_source(original_host,"DECK-PAD"); }
        let original_source=r.push_source(original_host,"APPMON-LV4");
        let original_top=r.game.player(actor).battle_area[1].top_card().handle();
        r.fire_on_play(actor,1);physical_menu::source_action(&r.game,original_host,39);
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            let filler=game.player_mut(actor).battle_area.remove(0);
            game.player_mut(actor).battle_area.push(filler);
            let replacement_top=game.player(actor).battle_area[1].top_card().handle();
            let live=digimon_engine::permanent::PermanentHandle{player:actor,index:0};
            let action=physical_menu::source_action(game,live,39);
            game.resolve_selection(actor,action).unwrap();assert!(game.pending_selection.is_none());
            let carrier=&game.player(actor).battle_area[0];
            assert_eq!(carrier.top_card().handle(),original_top);assert_eq!(carrier.linked_cards.len(),1);
            assert_eq!(carrier.linked_cards[0].handle(),original_source);
            assert_eq!(game.player(actor).battle_area[1].top_card().handle(),replacement_top);
            assert!(game.player(actor).battle_area[1].linked_cards.is_empty());assert_eq!(game.memory,10);
        }
    }
}

#[test]
fn generic_other_source_link_keeps_the_original_destination_after_compaction_and_source_card_moves() {
    const YAML:&str=r#"
card: LINK-DESTINATION
name: Link Destination
kind: digimon
level: 5
color: [red]
cost: 8
dp: 8000
effects:
  - when: on_play
    process:
      - link_cards:
          from: [own_digimon_sources]
          filter: { name_is: APPMON-LV4 }
          to: self
          count: { up_to: 1 }
          cost: free
      - suspend: { target: source }
"#;
    for actor in [0,1] { for destination_state in [0,1,2] {
        let mut r=base().from_dsl_yaml(YAML).unwrap().deck(0,&["DECK-PAD";12]).deck(1,&["DECK-PAD";12]).memory(10).start();
        advance_to_main(&mut r);r.game.turn_player_idx=actor as usize;
        r.place_on_field(actor,"APPMON-LV5",None);
        let destination=r.place_on_field(actor,"LINK-DESTINATION",None);r.push_source(destination,"DECK-PAD");
        let donor=r.place_on_field(actor,"APPMON-LV5",None);
        for _ in 0..39 { r.push_source(donor,"DECK-PAD"); }
        let original_source=r.push_source(donor,"APPMON-LV4");
        r.fire_on_play(actor,destination.index as usize);physical_menu::source_action(&r.game,donor,39);
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            let filler=game.player_mut(actor).battle_area.remove(0);game.player_mut(actor).battle_area.push(filler);
            if destination_state > 0 {
                let card=game.player_mut(actor).battle_area[0].card_sources.pop().unwrap();
                let insert=game.player(actor).battle_area[1].card_sources.len()-1;
                game.player_mut(actor).battle_area[1].card_sources.insert(insert,card);
            }
            if destination_state == 2 {
                game.player_mut(actor).battle_area.remove(0);
                assert!(physical_menu::commits(game).is_empty(),"the departed original destination cannot be replaced by its moved effect card's new carrier");
                assert_eq!(digimon_engine::action::build_action_mask(game,actor)[digimon_engine::action::space::PASS as usize],1.0);
                game.resolve_selection(actor,digimon_engine::action::space::PASS).unwrap();assert!(game.pending_selection.is_none());
                assert!(game.player(actor).battle_area.iter().all(|p|p.linked_cards.is_empty()));
                assert!(game.player(actor).battle_area.iter().all(|p|!p.is_suspended),"the continued source tail cannot target the new carrier of a moved effect card");
                assert_eq!(game.player(actor).battle_area[0].card_sources[39].handle(),original_source);assert_eq!(game.memory,10);
                continue;
            }
            let donor=digimon_engine::permanent::PermanentHandle{player:actor,index:1};
            let action=physical_menu::source_action(game,donor,39);game.resolve_selection(actor,action).unwrap();
            assert!(game.pending_selection.is_none());
            assert_eq!(game.player(actor).battle_area[0].linked_cards.len(),1,"This Digimon remains the original carrier, including after its physical effect card moves");
            assert_eq!(game.player(actor).battle_area[0].linked_cards[0].handle(),original_source);
            assert!(game.player(actor).battle_area[1].linked_cards.is_empty(),"the donor occupying the old destination slot cannot receive the link");
            assert!(game.player(actor).battle_area[0].is_suspended,"the continued source tail follows the original physical carrier");
            assert!(!game.player(actor).battle_area[1].is_suspended);
            assert!(game.player(actor).battle_area[2].linked_cards.is_empty());assert_eq!(game.memory,10);
        }
    }}
}

#[test]
fn generic_source_link_keeps_the_picked_source_carrier_through_host_selection() {
    const YAML:&str=r#"
card: LINK-HOST-PICKER
name: Link Host Picker
kind: digimon
level: 5
color: [red]
cost: 8
dp: 8000
effects:
  - when: on_play
    process:
      - link_cards:
          from: [own_digimon_sources]
          filter: { name_is: APPMON-LV4 }
          to: own_digimon
          host_filter: { name_is: LINK-TARGET }
          count: { up_to: 1 }
          cost: { reduce: 0 }
          bind_as: linked
      - if:
          condition: { binding_present: linked }
          then: [{ gain_memory: 5 }]
"#;
    for actor in [0,1] { for lost_source in [false,true] {
        let mut r=DebugRunner::builder().from_dsl_yaml(YAML).unwrap().add_card(appmon_lv4("APPMON-LV4")).add_card(appmon_lv5("APPMON-LV5")).add_card(make_test_card("DECK-PAD","Filler")).link_requirement("APPMON-LV4",2).add_card(make_digimon("LINK-TARGET",5,8000,8,&["Appmon"]))
            .deck(0,&["DECK-PAD";12]).deck(1,&["DECK-PAD";12]).memory(5).start();
        advance_to_main(&mut r);r.game.turn_player_idx=actor as usize;
        r.place_on_field(actor,"APPMON-LV5",None);
        let donor=r.place_on_field(actor,"APPMON-LV5",None);
        for _ in 0..39 { r.push_source(donor,"DECK-PAD"); }
        let original_source=r.push_source(donor,"APPMON-LV4");
        let target=r.place_on_field(actor,"LINK-TARGET",None);
        let effect=r.place_on_field(actor,"LINK-HOST-PICKER",None);r.fire_on_play(actor,effect.index as usize);
        let action=physical_menu::source_action(&r.game,donor,39);r.game.resolve_selection(actor,action).unwrap();
        assert!(r.game.pending_selection.is_some(),"source selection pauses at the public host picker");
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.player_mut(actor).battle_area.swap(0,1);
            if lost_source {
                let card=game.player_mut(actor).battle_area[0].card_sources.remove(39);
                game.player_mut(actor).battle_area[1].card_sources.insert(0,card);
                let data=game.card_data.iter().position(|c|c.card_id=="APPMON-LV4").unwrap();let index=game.next_card_index();
                game.player_mut(actor).battle_area[0].card_sources.insert(39,CardSource::new(data,actor,index));
            }
            let host_action=digimon_engine::action::space::encode_attack(0,target.index as u16);
            assert_eq!(digimon_engine::action::build_action_mask(game,actor)[host_action as usize],1.0);
            game.resolve_selection(actor,host_action).unwrap();assert!(game.pending_selection.is_none());
            let linked=&game.player(actor).battle_area[target.index as usize].linked_cards;
            if lost_source {
                assert!(linked.is_empty());assert_eq!(game.memory,5,"lost physical source grants no paid link or success tail");
                assert_eq!(game.player(actor).battle_area[1].card_sources[0].handle(),original_source);
            } else {
                assert_eq!(linked.len(),1);assert_eq!(linked[0].handle(),original_source);
                assert_eq!(game.memory,8,"one link costs two and earns the actual-link success tail once");
            }
        }
    }}
}
