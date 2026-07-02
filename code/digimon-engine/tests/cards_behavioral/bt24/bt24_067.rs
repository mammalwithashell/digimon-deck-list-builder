//! BT24-067 Hackmon — Digimon, Lv.3, Purple, DP 1000, Cost 3.
//! Traits: Stnd./Appmon | System | Hacking. Attribute: System.
//!
//! # Card text (transcribed from card image + DCGO BT24_067.cs, authoritative)
//! Digivolve box: "Digivolve: Lv.2 w/[Appmon] trait: Cost 0"
//!
//! Main effect:
//! [Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1 or
//!   fewer Tamers, you may play 1 [Rei Katsura] from your hand without paying
//!   the cost.
//!
//! Link box:
//! Link [Appmon] trait: Cost 1
//! <Retaliation> (granted to the host while this card is linked)
//!
//! Link box also prints +2000 DP (linked aura to host; data-driven LinkDP in
//! DCGO Permanent.cs, not a CardEffect).
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_067.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B3  when_card_linked_to_this host-side triggered OPT with your_turn gate
//! - C1  link_condition + linked keyword grant (Retaliation)
//! - E2  OPT + optional decline
//! - H9  Retaliation keyword grant via linked scope
//!
//! Near-twin of BT21-009 Gatchmon; differs: Purple, Rei Katsura, Retaliation,
//! single Appmon alt-digi, no DP aura.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT24-067";

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card
}

fn make_rei(id: &str) -> CardData {
    // Rei Katsura — a Tamer card named "Rei Katsura"
    let mut card = make_test_card(id, "Rei Katsura");
    card.card_kind = CardKind::Tamer;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-067 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon(
            "APPMON-HOST",
            4,
            4000,
            4,
            &["Appmon", "Social"],
        ))
        .add_card(make_rei("REI-IN-HAND"))
        .add_card(make_tamer("TAMER-A"))
        .add_card(make_tamer("TAMER-B"))
        .deck(1, &["DECK-PAD"; 12])
}

/// Fire the when_card_linked_to_this trigger on `host` by pushing BT24-067
/// as a linked card and dispatching OnLink to all players, then draining.
fn fire_link_onto_host(runner: &mut DebugRunner, host: PermanentHandle) {
    use digimon_engine::enums::EffectTiming;
    // Attach the card as linked (does NOT fire triggers by itself).
    let linked_handle = runner.push_linked_owned(host, CARD_ID, 0);
    // Enqueue OnLink globally to fire both the linked card's WhenLinked ESS
    // and the host's when_card_linked_to_this handler.
    for pid in 0..2usize {
        runner.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: linked_handle,
            },
        );
    }
    runner.game.drain_effect_queue();
}

// ── Section 1: structural assertions ─────────────────────────────────────────

#[test]
fn bt24_067_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Hackmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.cost, Some(3));
}

#[test]
fn bt24_067_traits_contain_appmon_system_hacking_stnd() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t: Vec<&str> = card.traits.iter().map(String::as_str).collect();
    for expected in &["Appmon", "System", "Hacking", "Stnd."] {
        assert!(
            t.contains(expected),
            "trait '{}' not found in traits {:?}",
            expected,
            t
        );
    }
}

/// Alt-path: digivolve from Lv.2 with [Appmon] trait for cost 0.
#[test]
fn bt24_067_has_alt_digivolve_from_lv2_appmon_cost0() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve) && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        has,
        "BT24-067 has a Lv.2 [Appmon] alt-digivolve path with cost 0"
    );
}

/// Link condition: this card links onto [Appmon] hosts for cost 1.
#[test]
fn bt24_067_has_link_condition_appmon_cost_1() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 1
        )
    });
    assert!(has, "BT24-067 declares a self link-condition with cost 1");
}

/// Linked Retaliation grant: host gains Retaliation while this card is linked.
#[test]
fn bt24_067_has_linked_retaliation_grant() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Retaliation" && *scope == CompiledScope::Linked
        )
    });
    assert!(
        has,
        "BT24-067 declares a scope:linked Retaliation grant (link-box keyword)"
    );
}

/// Host-side when_card_linked_to_this triggered clause: exactly one, OPT, own scope.
#[test]
fn bt24_067_has_when_card_linked_to_this_once_per_turn() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::WhenCardLinkedToThis] =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "exactly one when_card_linked_to_this clause"
    );
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(clause.once_per_turn, "OPT flag set");
    assert!(clause.optional, "optional (you may)");
}

/// Linked DP aura: +2000 DP to the host while this card is linked (link box).
#[test]
fn bt24_067_has_linked_dp_aura_2000() {
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
        "BT24-067 declares a scope:linked +2000 DP aura (link-box DP bonus)"
    );
}

// ── Section 2: linked ESS behavioral ──────────────────────────────────────────

/// [Link] +2000 DP: host effective DP rises by 2000 while BT24-067 is linked.
#[test]
fn bt24_067_linked_host_gains_2000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let dp_before = r.effective_dp(host).expect("host on field");

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 2000),
        "host effective DP +2000 while BT24-067 is linked"
    );
}

/// [Link] Retaliation grant: host gains Retaliation keyword while BT24-067 is linked.
#[test]
fn bt24_067_linked_retaliation_reaches_host() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));

    assert!(
        !r.game.has_keyword(host, Keyword::Retaliation),
        "baseline: host has no Retaliation before linking"
    );

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(host, Keyword::Retaliation),
        "host gains <Retaliation> while BT24-067 is linked"
    );
}

// ── Section 3: condition gating ───────────────────────────────────────────────

/// Positive: 0 Tamers — when-linked trigger fires and offers Rei Katsura.
#[test]
fn bt24_067_when_linked_0_tamers_fires() {
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    // No tamers on field: condition passes.
    fire_link_onto_host(&mut r, host);

    assert!(
        r.game.pending_selection.is_some(),
        "0 tamers: when-linked trigger should surface a selection for Rei Katsura"
    );
}

/// Positive: 1 Tamer — trigger still fires (≤1 is the boundary).
#[test]
fn bt24_067_when_linked_1_tamer_fires() {
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TAMER-A", Some(0));

    fire_link_onto_host(&mut r, host);

    assert!(
        r.game.pending_selection.is_some(),
        "1 tamer: when-linked trigger should still fire"
    );
}

/// Negative: 2 Tamers — trigger condition fails, no selection.
#[test]
fn bt24_067_when_linked_2_tamers_no_fire() {
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TAMER-A", Some(0));
    r.place_on_field(0, "TAMER-B", Some(0));

    fire_link_onto_host(&mut r, host);

    assert!(
        r.game.pending_selection.is_none(),
        "2 tamers: when-linked trigger should be suppressed"
    );
}

/// Negative: opponent's turn — [Your Turn] gate blocks the trigger.
#[test]
fn bt24_067_when_linked_opponents_turn_no_fire() {
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    // Advance to player 1's turn.
    r.end_turn();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    fire_link_onto_host(&mut r, host);

    assert!(
        r.game.pending_selection.is_none(),
        "opponent's turn: [Your Turn] gate must block the trigger"
    );
}

// ── Section 4: behavioral outcome ─────────────────────────────────────────────

/// When triggered: select Rei → play free from hand → Tamer on field.
#[test]
fn bt24_067_when_linked_plays_rei_katsura_free() {
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    let field_before = r.battle_area_size(0);
    let hand_before = r.hand_size(0);

    fire_link_onto_host(&mut r, host);

    // Select the Rei Katsura card.
    assert!(
        r.game.pending_selection.is_some(),
        "when-linked fires and offers Rei Katsura selection"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    // Drain any follow-up prompts.
    r.game.drain_effect_queue();

    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "Rei Katsura was played to the field (free)"
    );
    assert_eq!(r.hand_size(0), hand_before - 1, "Rei Katsura left the hand");
}

/// Decline (PASS) → nothing happens.
#[test]
fn bt24_067_when_linked_decline_no_play() {
    use digimon_engine::action::space::PASS;
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    let field_before = r.battle_area_size(0);
    let hand_before = r.hand_size(0);

    fire_link_onto_host(&mut r, host);

    assert!(r.game.pending_selection.is_some(), "when-linked fires");
    assert!(r.pending_is_optional(), "prompt must be optional (you may)");
    let _ = r.game.resolve_selection(0, PASS);

    assert_eq!(r.battle_area_size(0), field_before, "PASS: no card played");
    assert_eq!(r.hand_size(0), hand_before, "PASS: hand unchanged");
}

/// No Rei Katsura in hand → no selection offered (condition gates out).
#[test]
fn bt24_067_when_linked_no_rei_in_hand_no_prompt() {
    let mut r = base()
        .hand(0, &[]) // empty hand
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    fire_link_onto_host(&mut r, host);

    // The DCGO CanActivateCondition checks HasMatchConditionOwnersHand before
    // activating — so no selection when there's no Rei Katsura.
    assert!(
        r.game.pending_selection.is_none(),
        "no Rei Katsura in hand: no prompt should be installed"
    );
}

// ── Section 5: OPT enforcement ────────────────────────────────────────────────

/// Second link same turn → OPT lockout prevents a second prompt.
#[test]
fn bt24_067_when_linked_opt_blocks_second_link_same_turn() {
    use digimon_engine::action::space::PASS;
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    // First link — fires; decline via PASS.
    fire_link_onto_host(&mut r, host);
    if r.game.pending_selection.is_some() {
        let _ = r.game.resolve_selection(0, PASS);
    }

    // Second link same turn — OPT must block.
    use digimon_engine::enums::EffectTiming;
    let dummy_card = r.push_linked_owned(host, "APPMON-HOST", 0);
    for pid in 0..2usize {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: dummy_card,
            },
        );
    }
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "OPT: second link in same turn must be locked out"
    );
}

/// OPT clears after end_turn cycle.
#[test]
fn bt24_067_when_linked_opt_resets_after_end_turn() {
    use digimon_engine::action::space::PASS;
    let mut r = base()
        .hand(0, &["REI-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    // First link — fires; decline.
    fire_link_onto_host(&mut r, host);
    if r.game.pending_selection.is_some() {
        let _ = r.game.resolve_selection(0, PASS);
    }

    // End-turn cycle: player 0 → player 1 → player 0 again.
    r.end_turn();
    r.end_turn();
    r.game.enter_main_phase();

    // Fire link again — should be allowed (OPT cleared).
    fire_link_onto_host(&mut r, host);

    assert!(
        r.game.pending_selection.is_some(),
        "OPT must clear after end_turn cycle — second link should fire again"
    );
}
