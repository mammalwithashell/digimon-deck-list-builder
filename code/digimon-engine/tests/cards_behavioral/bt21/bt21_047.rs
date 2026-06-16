//! BT21-047 Navimon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Stnd./Appmon | Navi. Attribute: Navi.
//!
//! # Card text (card image — authoritative)
//!
//! Digivolve: Lv.2 w/[Appmon] trait: Cost 0
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Appmon]
//! trait and 1 card with the [App Driver] trait among them to the hand. Return
//! the rest to the bottom of the deck.
//!
//! Link box:
//!   Link [Appmon] trait: Cost 1
//!   +2000 DP
//!   <Piercing> (When this Digimon attacks and deletes your opponent's Digimon
//!   in battle, it checks security before the attack ends.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Green/BT21_047.cs
//! - AddSelfLinkConditionStaticEffect(HasAppmonTraits, linkCost 1)
//! - AddSelfDigivolutionRequirementStaticEffect(HasAppmonTraits, 0, lvl 2)
//! - OnEnterFieldAnyone: SimplifiedRevealDeckTopCardsAndSelect (2 buckets)
//! - OnDetermineDoSecurityCheck: PierceSelfEffect(isLinkedEffect: true)
//!
//! # Patterns this test covers
//! - DigiLink Shape-B self link-condition (cost 1, trait_has Appmon)
//! - Alt-digivolve registration (Lv.2 + Appmon trait, cost 0)
//! - [On Play] reveal-3 two-bucket add-to-hand (trait_has Appmon + trait_has "App Driver")
//! - scope: linked dp aura (+2000 DP to host)
//! - scope: linked grant_keyword Piercing (host gains Piercing while linked)
//!
//! # Clause summary
//!
//! | # | Kind      | Scope  | Timing       | Details                               |
//! |---|-----------|--------|--------------|---------------------------------------|
//! | 0 | link_cond | FaceUp | static       | cost 1, host filter trait_has Appmon  |
//! | 1 | triggered | FaceUp | on_play      | reveal 3 → add Appmon+AppDriver; rest bottom |
//! | 2 | aura      | Linked | static       | dp_modifier +2000 (to host)           |
//! | 3 | grant_kwd | Linked | static       | keyword: Piercing (to host)           |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT21-047";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Returns the action ID for the on-field "Link" action for `perm`.
fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-047 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Cards for reveal buckets.
        .add_card(make_digimon("APPMON-CARD", 3, 2000, 3, &["Search", "Appmon"]))
        .add_card(make_digimon("DRIVER-CARD", 3, 2000, 3, &["App Driver"]))
        .add_card(make_digimon("PLAIN-CARD", 3, 2000, 3, &["Beast"]))
        // Host for link absorption.
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

/// Metadata must be read correctly from the YAML.
#[test]
fn bt21_047_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("BT21-047 in compiled pack");
    assert_eq!(card.name, "Navimon", "name");
    assert_eq!(card.level, Some(3), "level");
    assert_eq!(card.dp, Some(1000), "dp");
}

/// The card must have a self link-condition with cost 1 and host filter [Appmon].
#[test]
fn bt21_047_has_link_condition_appmon_cost_1() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 1
        )
    });
    assert!(has, "BT21-047 must declare a self link-condition with cost 1");
}

/// The alt-path must be a cost-0 digivolve over an Appmon Lv.2.
#[test]
fn bt21_047_registers_appmon_alt_digivolve_cost_0() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        has,
        "BT21-047 must register a cost-0 alt-digivolve over Appmon Lv.2"
    );
}

/// The card must have an [On Play] triggered clause.
#[test]
fn bt21_047_has_on_play_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let on_play = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )
    });
    assert!(on_play, "BT21-047 must have an [On Play] clause");
}

/// The card must have a linked DP aura (+2000, scope Linked).
#[test]
fn bt21_047_has_linked_dp_aura_2000() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(2000),
                ..
            })
        )
    });
    assert!(
        has,
        "BT21-047 must declare a scope:Linked aura with dp_modifier +2000"
    );
}

/// The card must have a scope:Linked GrantKeyword clause for Piercing.
#[test]
fn bt21_047_has_linked_piercing_grant() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                scope: CompiledScope::Linked,
                ..
            })
        )
    });
    assert!(
        has,
        "BT21-047 must declare a scope:Linked GrantKeyword clause (for Piercing)"
    );
}

// ─── Section 2 — On Play: reveal-3 two-bucket ────────────────────────────────

/// Positive: deck has both an [Appmon] and an [App Driver] card in the top 3.
/// After resolving, hand has +2 net from the play (-1 played + 2 added).
#[test]
fn bt21_047_on_play_adds_both_bucket_cards_to_hand() {
    // Deck (top-to-bottom): APPMON-CARD, DRIVER-CARD, DECK-PAD
    // (last listed = top for DebugRunnerBuilder.deck convention)
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "DRIVER-CARD", "APPMON-CARD"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    let _g = r.play(0, 0).expect("Navimon played");
    r.auto_resolve().ok();

    // Net: -1 (Navimon played) +2 (added from reveal) = +1 from hand_before.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "On Play must add exactly 2 cards to hand (one per bucket)"
    );
    // No revealed card should go to trash; remainder goes to deck bottom.
    assert_eq!(r.trash_size(0), 0, "remainder must be bottomed, not trashed");
}

/// Negative: only [Appmon] matches in the top 3 (no [App Driver]).
/// The Appmon-bucket card is added; the App Driver bucket gets nothing (no eligible target).
#[test]
fn bt21_047_on_play_only_appmon_matched_deck_one_added() {
    // Deck top 3: APPMON-CARD, PLAIN-CARD, DECK-PAD — no App Driver
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "APPMON-CARD"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    let _g = r.play(0, 0).expect("Navimon played");
    r.auto_resolve().ok();

    // Net: -1 (Navimon played) +1 (only Appmon bucket filled) = 0.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "with only Appmon bucket filled, hand size should be unchanged from before play"
    );
}

/// Negative: only [App Driver] matches in the top 3 (no [Appmon]).
/// The App Driver bucket card is added; the Appmon bucket gets nothing.
#[test]
fn bt21_047_on_play_only_app_driver_matched_one_added() {
    // Deck top 3: DRIVER-CARD, PLAIN-CARD, DECK-PAD — no Appmon
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "DRIVER-CARD"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    let _g = r.play(0, 0).expect("Navimon played");
    r.auto_resolve().ok();

    // Net: -1 (Navimon played) +1 (only App Driver bucket filled) = 0.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "with only App Driver bucket filled, hand size should be unchanged from before play"
    );
}

/// Negative: neither bucket matches in the top 3 — no card added.
#[test]
fn bt21_047_on_play_no_match_no_card_added() {
    // Deck top 3: PLAIN-CARD x3 — neither Appmon nor App Driver
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["PLAIN-CARD", "PLAIN-CARD", "PLAIN-CARD"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    let _g = r.play(0, 0).expect("Navimon played");
    r.auto_resolve().ok();

    // Net: -1 (Navimon played) +0 (no buckets filled) = -1 from hand_before.
    assert_eq!(
        r.hand_size(0),
        hand_before - 1,
        "no matching card in top 3; hand shrinks by 1 (only Navimon played, nothing returned)"
    );
    assert_eq!(r.trash_size(0), 0, "remainder bottomed, not trashed");
}

// ─── Section 3 — Linked side: DP aura and Piercing on host ───────────────────

/// When Navimon is linked to a host [Appmon] Digimon, the host's effective DP
/// increases by +2000 (the linked aura).
#[test]
fn bt21_047_linked_host_gains_2000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let navi = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r
        .effective_dp(host)
        .expect("HOST-APP on field, must have DP");

    // Activate the link action to absorb Navimon onto host.
    r.game.decode_action(link_bit(navi) as u16, 0);
    // The link selection: pick the host permanent.
    let action = r
        .game
        .pending_selection
        .as_ref()
        .expect("link selection must install")
        .valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();

    let dp_after = r
        .effective_dp(host)
        .expect("HOST-APP still on field after absorbing Navimon");

    assert_eq!(
        dp_after,
        dp_before + 2000,
        "host DP must increase by 2000 after Navimon links onto it; \
         before={dp_before}, after={dp_after}"
    );
}

/// After Navimon links onto a host, the host gains the Piercing keyword.
#[test]
fn bt21_047_linked_host_gains_piercing() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let navi = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    assert!(
        !r.game.has_keyword(host, Keyword::Piercing),
        "host must NOT have Piercing before Navimon links onto it"
    );

    // Activate the link action.
    r.game.decode_action(link_bit(navi) as u16, 0);
    let action = r
        .game
        .pending_selection
        .as_ref()
        .expect("link selection must install")
        .valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();
    // The linked keyword grant requires a declarative tick to materialise into
    // the modifier registry (same pattern as bt25_101 linked Reboot test).
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(host, Keyword::Piercing),
        "host must gain Piercing after Navimon links onto it (isLinkedEffect:true)"
    );
}
