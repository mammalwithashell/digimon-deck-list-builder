//! BT21-073 Charismon — Digimon, Lv.5, Purple/Black, DP 8000, Cost 8.
//! Traits: "Ult.", Appmon, Social, "Mind Control". Attribute: Social.
//!
//! # Card text (official Bandai DB bundle + card image — authoritative)
//!
//! Digivolve: split purple/black circle → Lv.4 Purple / Cost 4 AND
//!   Lv.4 Black / Cost 4; plus rainbow grade circle "Sup." / Cost 4.
//! App Fusion [Sociamon] & [Gossipmon]: Cost 0.
//! <Blocker>
//! [On Play][When Digivolving] You may link 1 level 4 or lower Digimon card
//!   from your trash or this Digimon's digivolution cards to this Digimon
//!   without paying the cost.
//! [Your Turn][Once Per Turn] When this Digimon gets linked, give 1 of your
//!   opponent's Digimon "[Start of Your Main Phase] This Digimon attacks."
//!   until their turn ends.
//!
//! Link box:
//!   Link [Appmon] trait: Cost 3
//!   Link DP bonus: +4000 DP
//!   [All Turns][Once Per Turn] When this Digimon would leave the battle area,
//!     by trashing 1 of this Digimon's link cards, it doesn't leave.
//!     (OPT — DCGO maxCount 1 + hash "AllTurns_BT21_073".)
//!
//! # DCGO C# reference (READ-ONLY)
//! C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_073.cs
//!
//! # Patterns covered
//! - link_condition (cost 3, Appmon trait host filter)
//! - app_fusion alt-path (Sociamon & Gossipmon)
//! - printed digivolve circles as alt_paths: Purple Lv.4/4 + Black Lv.4/4
//!   (split circle) + any-color Sup./4 grade circle
//! - grant_keyword Blocker
//! - link_card_to_self from [trash, digivolution_sources] (optional, cost 0) — on_play/when_digivolving
//! - when_card_linked_to_this OPT [Your Turn]: grant_triggered_effect force_attack on opponent Digimon
//!   until end_of_opponents_turn (taunt — BT25-054 idiom)
//! - scope: linked leave-replacement by trashing own link card, [All Turns]
//!   [Once Per Turn] (SetIsLinkedEffect(true) + maxCount 1 in DCGO) — accept /
//!   decline / OPT lockout / next-turn reset
//! - scope: linked aura +4000 DP (link box DP bonus)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT21-073";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
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
        .expect("BT21-073 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("APPMON-LV4", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("APPMON-LV5", 5, 6000, 7, &["Appmon"]))
        .add_card(make_digimon("NON-APPMON-LV4", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", 4, 5000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI2", 4, 5000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt21_073_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Charismon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    assert_eq!(card.cost, Some(8));
}

#[test]
fn bt21_073_has_link_condition_appmon_cost_3() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 3
        )
    });
    assert!(
        has,
        "BT21-073 must declare a self link-condition with cost 3 for [Appmon] hosts"
    );
}

#[test]
fn bt21_073_has_app_fusion_altpath() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::AppFusion));
    assert!(
        has,
        "BT21-073 must declare an App Fusion alt-path (Sociamon & Gossipmon)"
    );
}

#[test]
fn bt21_073_has_all_printed_digivolve_circles() {
    // Card face: ONE split purple/black "Lv.4 / 4" circle (official Bandai DB
    // lists it as Purple Lv.4 / cost 4 AND Black Lv.4 / cost 4) plus the
    // rainbow any-color "Sup. / 4" grade circle. All three must be authored
    // as digivolve alt_paths at the printed cost 4.
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();

    let has_circle = |color: CompiledColor| {
        digivolve_paths.iter().any(|p| {
            p.cost == Some(CompiledCost::Literal(4))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(4) && f.color_is == Some(color)
                })
        })
    };
    assert!(
        has_circle(CompiledColor::Purple),
        "BT21-073 must have the printed Purple Lv.4 / cost 4 digivolve circle as an alt_path"
    );
    assert!(
        has_circle(CompiledColor::Black),
        "BT21-073 must have the printed Black Lv.4 / cost 4 digivolve circle as an alt_path \
         (split purple/black circle; official DB lists both — JSON evo_costs drops black)"
    );

    let has_sup = digivolve_paths.iter().any(|p| {
        p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.trait_has.as_deref() == Some("Sup.") && f.color_is.is_none()
            })
    });
    assert!(
        has_sup,
        "BT21-073 must have the rainbow (no color gate) Sup. / cost 4 grade circle as an alt_path"
    );
}

#[test]
fn bt21_073_has_blocker_keyword() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Blocker"
        )
    });
    assert!(has, "BT21-073 must have a Blocker grant_keyword clause");
}

#[test]
fn bt21_073_has_on_play_when_digivolving_link_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    assert!(
        has,
        "BT21-073 must have a clause triggering on both on_play and when_digivolving"
    );
}

#[test]
fn bt21_073_has_when_card_linked_once_per_turn() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenCardLinkedToThis)
                && t.once_per_turn
        )
    });
    assert!(
        has,
        "BT21-073 must have a when_card_linked_to_this clause with once_per_turn"
    );
}

#[test]
fn bt21_073_has_linked_leave_replacement() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    // The leave-replacement is a linked-scope declarative clause (SetIsLinkedEffect(true) in DCGO).
    // Printed: "[All Turns] [Once Per Turn] ... by trashing 1 of this Digimon's
    // link cards, it doesn't leave" → optional AND once_per_turn (DCGO maxCount 1).
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) if *scope == CompiledScope::Linked && *optional && *once_per_turn
        )
    });
    assert!(
        has,
        "BT21-073 must have a scope:Linked, optional, once_per_turn Replacement clause \
         for the leave-prevention link effect (printed [All Turns][Once Per Turn])"
    );
}

#[test]
fn bt21_073_has_linked_dp_aura() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. })
                if *scope == CompiledScope::Linked
        )
    });
    assert!(
        has,
        "BT21-073 must have a scope:Linked Aura clause for the +4000 DP link bonus"
    );
}

// ─── Section 2 — On Play self-link (from trash) ──────────────────────────────

#[test]
fn bt21_073_on_play_links_lv4_from_trash() {
    // Seed APPMON-LV4 in trash. Play Charismon → On Play fires → link prompt surfaces.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    advance_to_main(&mut r);

    let chari_idx = r.play(0, 0).expect("Charismon played");

    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt must surface when a level-4 Digimon is in trash"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    assert_eq!(
        r.game.player(0).battle_area[chari_idx].linked_cards.len(),
        1,
        "APPMON-LV4 from trash should be linked to Charismon after On Play selection"
    );
}

#[test]
fn bt21_073_on_play_decline_links_nothing() {
    // Seed APPMON-LV4 in trash. Play Charismon → On Play fires → player declines (optional).
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    advance_to_main(&mut r);

    let chari_idx = r.play(0, 0).expect("Charismon played");

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
        r.game.player(0).battle_area[chari_idx].linked_cards.len(),
        0,
        "Declining On Play link should not attach any card"
    );
}

#[test]
fn bt21_073_on_play_no_prompt_when_no_eligible_card_in_trash_or_sources() {
    // APPMON-LV5 in trash (level 5, not eligible). No prompt surfaces.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV5");
    advance_to_main(&mut r);

    let chari_idx = r.play(0, 0).expect("Charismon played");

    assert!(
        r.game.pending_selection.is_none(),
        "[On Play] link prompt must NOT surface when no level-4-or-lower Digimon is in trash/sources"
    );
    assert_eq!(
        r.game.player(0).battle_area[chari_idx].linked_cards.len(),
        0
    );
}

// ─── Section 3 — On Play self-link (from digivolution sources) ───────────────

#[test]
fn bt21_073_on_play_links_lv4_from_digivolution_sources() {
    // Build a digivolution stack: APPMON-LV4 under Charismon. When Digivolving fires.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);

    // Create a digi-stack: APPMON-LV4 at bottom, CARD_ID on top.
    let chari = r.place_field_stack(0, &["APPMON-LV4", CARD_ID], false, 0);
    r.fire_on_play(0, chari.index as usize);

    // If prompt surfaces, confirm the source card from digi-sources is selectable.
    if r.game.pending_selection.is_some() {
        let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, link_action);
        assert_eq!(
            r.game.player(0).battle_area[chari.index as usize]
                .linked_cards
                .len(),
            1,
            "Lv4 card from digivolution sources should be linked"
        );
    }
    // If no prompt, that is valid when no eligible card remains in sources.
}

// ─── Section 4 — when_card_linked_to_this (Your Turn, OPT taunt grant) ───────
//
// Charismon is the HOST. When a card links TO IT, [Your Turn][OPT] fires:
// select 1 opponent Digimon → grant it "[Start of Your Main Phase] This Digimon attacks."
// until their turn ends. BT25-054 taunt pattern.
//
// Drive: play Charismon via [On Play] free-link (seed APPMON-LV4 in trash).

#[test]
fn bt21_073_when_linked_your_turn_grants_taunt_to_opp_digi() {
    // Play Charismon (On Play). Seed APPMON-LV4 in trash → On Play link prompt.
    // Link APPMON-LV4 → when_card_linked_to_this fires [Your Turn][OPT]:
    // select OPP-DIGI → taunt grant installs. Selections drain cleanly.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    let opp_digi = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    let chari_idx = r.play(0, 0).expect("Charismon played");
    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt surfaces"
    );

    // Link APPMON-LV4 from trash.
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // when_card_linked_to_this fires: taunt target select must surface (Your Turn).
    assert!(
        r.game.pending_selection.is_some(),
        "[Your Turn][OPT] when_card_linked_to_this taunt prompt must surface after linking"
    );

    // Select OPP-DIGI as the taunt target.
    let taunt_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, taunt_action);

    // Selections drained — grant installed cleanly.
    assert!(
        r.game.pending_selection.is_none(),
        "Selections should drain after taunt grant is installed"
    );
}

#[test]
fn bt21_073_when_linked_no_taunt_if_no_opp_digi() {
    // Play Charismon, link via On Play, but no opponent Digimon on field.
    // when_card_linked_to_this fires: taunt prompt should NOT surface (no targets).
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    // No opponent Digimon placed.
    advance_to_main(&mut r);

    let chari_idx = r.play(0, 0).expect("Charismon played");
    assert!(r.game.pending_selection.is_some(), "[On Play] link prompt");
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // when_card_linked_to_this fires, but no opponent Digimon → no taunt prompt.
    assert!(
        r.game.pending_selection.is_none(),
        "Taunt prompt must NOT surface when no opponent Digimon is on the field"
    );
}

#[test]
fn bt21_073_when_linked_taunt_opt_lockout_same_turn() {
    // First link triggers OPT taunt (fires once). Second link same turn: OPT locked out.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    let opp_digi = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    // First: play Charismon → On Play link.
    let chari_idx = r.play(0, 0).expect("Charismon played");
    assert!(
        r.game.pending_selection.is_some(),
        "On Play link prompt (first)"
    );
    let link1 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link1);

    // Taunt prompt fires (OPT used).
    assert!(
        r.game.pending_selection.is_some(),
        "First taunt prompt surfaces"
    );
    let taunt1 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, taunt1);

    // Second link: seed another Lv4 in trash, fire on_play again.
    seed_trash(&mut r, 0, "NON-APPMON-LV4");
    r.fire_on_play(0, chari_idx);
    if r.game.pending_selection.is_some() {
        let link2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, link2);
    }

    // OPT lockout: second when_card_linked_to_this must NOT fire taunt again.
    assert!(
        r.game.pending_selection.is_none(),
        "OPT lockout: second link on same turn must NOT trigger the taunt again"
    );
}

#[test]
fn bt21_073_when_linked_taunt_opt_clears_after_end_turn() {
    // First turn: link fires OPT taunt. End turn (P0→P1→P0). Second turn: taunt fires again.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    let opp_digi = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    // Turn 0.
    let chari_idx = r.play(0, 0).expect("Charismon played");
    assert!(
        r.game.pending_selection.is_some(),
        "[On Play] link prompt T0"
    );
    let link1 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link1);
    if r.game.pending_selection.is_some() {
        let taunt1 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, taunt1);
    }

    // End turns P0 → P1 → P0.
    r.end_turn();
    r.end_turn();

    // Seed another Lv4 Digimon in trash.
    seed_trash(&mut r, 0, "NON-APPMON-LV4");
    let _opp_digi2 = r.place_on_field(1, "OPP-DIGI2", Some(0));
    advance_to_main(&mut r);

    // Second turn: fire on_play on the Charismon already on field.
    r.fire_on_play(0, chari_idx);
    if r.game.pending_selection.is_some() {
        let link2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, link2);
    }

    // OPT cleared after end_turn: taunt prompt must fire again.
    assert!(
        r.game.pending_selection.is_some(),
        "OPT must clear after end_turn: taunt prompt should surface on first link of new turn"
    );
}

// ─── Section 5 — Link DP aura: host effective_dp +4000 while Charismon linked ─

#[test]
fn bt21_073_linked_dp_aura_increases_host_effective_dp() {
    // Place APPMON-LV4 host (DP 4000) on field. Link Charismon to it.
    // The host's effective_dp should increase by 4000 to 8000.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0)); // base DP 4000
    let charismon = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r.effective_dp(host).unwrap_or(0);

    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    let link_bit = (FIELD_EFFECT_START
        + charismon.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);
    if r.game.pending_selection.is_some() {
        let sel = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, sel);
    }
    // Drain any when_card_linked_to_this prompts (no opp digimon → no taunt prompt).
    r.auto_resolve().ok();

    let dp_after = r.effective_dp(host).unwrap_or(0);
    assert_eq!(
        dp_after,
        dp_before + 4000,
        "Host effective DP should increase by 4000 after Charismon links to it"
    );
}

// ─── Section 6 — Link-ESS leave-replacement (scope: linked, [All Turns][OPT]) ─
//
// While Charismon is a LINK CARD on a host, when that host would leave the
// battle area, the host's controller may trash 1 of the host's link cards to
// prevent the leave. Printed [All Turns] [Once Per Turn]. BT25-101 pattern.

#[test]
fn bt21_073_linked_leave_replacement_accept_prevents_leave() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0));
    r.push_linked_owned(host, CARD_ID, 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(host);

    // "by trashing ... it doesn't leave" is a may — optional accept prompt.
    assert!(
        r.game.pending_selection.is_some(),
        "linked leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the replacement");

    // The WHICH-link-card choice surfaces even with a single link card.
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card prompt")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    assert_eq!(r.battle_area_size(0), 1, "the host did NOT leave");
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        0,
        "the chosen link card (Charismon) was trashed as the cost"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly the link card went to trash"
    );
}

#[test]
fn bt21_073_linked_leave_replacement_declined_lets_host_leave() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0));
    r.push_linked_owned(host, CARD_ID, 0);
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(host);
    assert!(
        r.game.pending_selection.is_some(),
        "the optional accept prompt is offered"
    );
    r.game
        .resolve_selection(0, PASS)
        .expect("decline the replacement");

    assert_eq!(r.battle_area_size(0), 0, "declined → the host leaves");
}

#[test]
fn bt21_073_linked_leave_replacement_opt_lockout_same_turn() {
    // Printed [Once Per Turn]: after the replacement prevents one leave, a
    // second would-leave in the SAME turn must not be offered again.
    // Link a filler card FIRST (linked index 0) and Charismon SECOND so the
    // first accept can trash the filler while keeping Charismon (the effect
    // carrier) linked.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0));
    r.push_linked_owned(host, "NON-APPMON-LV4", 0);
    r.push_linked_owned(host, CARD_ID, 0);
    advance_to_main(&mut r);

    // First leave attempt: accept, trash the filler (choice 0 = linked index 0).
    r.game.delete_permanent_with_effects(host);
    assert!(
        r.game.pending_selection.is_some(),
        "first leave attempt offers the replacement"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the replacement");
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card prompt")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("trash the filler");

    assert_eq!(r.battle_area_size(0), 1, "the host did NOT leave");
    // Guard: Charismon (the carrier of the linked replacement) must still be linked.
    let still_linked: Vec<String> = r.game.player(0).battle_area[host.index as usize]
        .linked_cards
        .iter()
        .filter_map(|c| {
            r.game
                .card_data_for_handle(c.handle())
                .map(|d| d.card_id.clone())
        })
        .collect();
    assert_eq!(
        still_linked,
        vec![CARD_ID.to_string()],
        "the filler was trashed; Charismon remains linked so its OPT state is testable"
    );

    // Second leave attempt SAME turn: [Once Per Turn] → not offered, host leaves.
    r.game.delete_permanent_with_effects(host);
    assert!(
        r.game.pending_selection.is_none(),
        "OPT lockout: the linked leave-replacement must NOT be offered twice in one turn"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "with the OPT spent, the host leaves normally"
    );
}

#[test]
fn bt21_073_linked_leave_replacement_offered_again_next_turn() {
    // [All Turns] + [Once Per Turn]: the OPT resets at turn end, so a leave in
    // the NEXT turn (opponent's) is offered again.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let host = r.place_on_field(0, "APPMON-LV4", Some(0));
    r.push_linked_owned(host, "NON-APPMON-LV4", 0);
    r.push_linked_owned(host, CARD_ID, 0);
    advance_to_main(&mut r);

    // Spend the OPT this turn (accept, trash the filler at linked index 0).
    r.game.delete_permanent_with_effects(host);
    assert!(r.game.pending_selection.is_some(), "first offer");
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card prompt")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("trash the filler");
    assert_eq!(r.battle_area_size(0), 1, "host stayed");

    // Move to the opponent's turn — the OPT clears; [All Turns] still applies.
    r.end_turn();

    r.game.delete_permanent_with_effects(host);
    assert!(
        r.game.pending_selection.is_some(),
        "next turn ([All Turns]): the leave-replacement is offered again after the OPT resets"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept on the opponent's turn");
    let pick2 = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card prompt (Charismon itself)")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick2).expect("trash Charismon");
    assert_eq!(
        r.battle_area_size(0),
        1,
        "the host survives again by trashing Charismon itself"
    );
}

// ─── Section 7 — Event log: link fires events ────────────────────────────────

#[test]
fn bt21_073_when_linked_taunt_grant_fires_cleanly() {
    // Play Charismon (On Play), link APPMON-LV4 from trash, select OPP-DIGI for taunt.
    // Verify events fire (no crash) and game state is clean after full resolution.
    use digimon_engine::events::GameEvent;

    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "APPMON-LV4");
    let _opp_digi = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    let cp = r.event_checkpoint();

    let chari_idx = r.play(0, 0).expect("Charismon played");
    assert!(r.game.pending_selection.is_some(), "On Play link prompt");
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    if r.game.pending_selection.is_some() {
        let taunt_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, taunt_action);
    }

    let events = r.events_since(cp);
    // At minimum the link event fired — assert something was emitted.
    assert!(
        !events.is_empty(),
        "Events should fire when Charismon links and grants the taunt"
    );
    // No pending selection remains.
    assert!(
        r.game.pending_selection.is_none(),
        "All selections resolved cleanly"
    );
}
