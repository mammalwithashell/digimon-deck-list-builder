//! BT21-009 Gatchmon — Digimon, Lv.3, Red, DP 2000, Cost 3.
//! Traits: Stnd./Appmon | Social | Search/Hero
//!
//! # Card text (CORRECTED — transcribed from card image, authoritative)
//! Digivolve box: "Digivolve: Lv.2 w/[Appmon]/[Hero] trait: Cost 0"
//!
//! Main effect:
//! [Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1 or
//!   fewer Tamers, you may play 1 [Haru Shinkai] from your hand without paying
//!   the cost.
//!
//! Link box:
//! Link [Appmon] trait: Cost 1
//! Link DP bonus: +2000 DP
//! Raid (granted to the host while this card is linked)
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_009.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B3  when_card_linked_to_this host-side triggered OPT with your_turn gate
//! - C1  link_condition + linked DP aura + linked keyword grant (Raid)
//! - E2  OPT + optional decline
//! - H9  Raid keyword grant via linked scope

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-009";

// ── Helpers ──────────────────────────────────────────────────────────────────

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

fn make_haru(id: &str) -> CardData {
    // Haru Shinkai — a Tamer card named "Haru Shinkai"
    let mut card = make_test_card(id, "Haru Shinkai");
    card.card_kind = CardKind::Tamer;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-009 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon(
            "APPMON-HOST",
            4,
            4000,
            4,
            &["Appmon", "Social"],
        ))
        .add_card(make_haru("HARU-IN-HAND"))
        .add_card(make_tamer("TAMER-A"))
        .add_card(make_tamer("TAMER-B"))
        .deck(1, &["DECK-PAD"; 12])
}

/// Fire the when_card_linked_to_this trigger on `host` by pushing BT21-009
/// as a linked card and dispatching OnLink to all players, then draining.
/// Returns true if a pending_selection installed (matching BT25-052/BT25-070 recipe).
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

// ── Section 1: structural assertions ────────────────────────────────────────

#[test]
fn bt21_009_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Gatchmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.cost, Some(3));
}

#[test]
fn bt21_009_traits_contain_appmon_social_search_hero_stnd() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t: Vec<&str> = card.traits.iter().map(String::as_str).collect();
    for expected in &["Appmon", "Social", "Search", "Hero", "Stnd."] {
        assert!(
            t.contains(expected),
            "trait '{}' not found in traits {:?}",
            expected,
            t
        );
    }
}

/// Special condition (black text): "[Digivolve] Lv.2 w/[Appmon]/[Hero] trait:
/// Cost 0" — a trait-gated Lv.2 alt-digivolve path at cost 0.
#[test]
fn bt21_009_has_alt_digivolve_from_lv2_appmon_or_hero_cost0() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(2)
                    && f.any_of
                        .iter()
                        .any(|g| g.trait_has.as_deref() == Some("Appmon"))
                    && f.any_of
                        .iter()
                        .any(|g| g.trait_has.as_deref() == Some("Hero"))
            })
    });
    assert!(
        has,
        "BT21-009 has the trait-gated Lv.2 w/[Appmon]/[Hero] alt-digivolve path at cost 0"
    );
}

/// Printed standard circle: "Red Lv.2 / cost 0" (card image digivolve circle +
/// official Bandai DB). Authored as a bare {level_eq, color_is} alt-path per the
/// printed-circle convention (tests/alt_path_printed_cost_guard.rs); it also
/// backfills the DebugRunner fixture's `evo_costs`.
#[test]
fn bt21_009_has_standard_red_lv2_cost0_circle() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(2)
                    && f.color_is == Some(CompiledColor::Red)
                    && f.any_of.is_empty()
                    && f.trait_has.is_none()
            })
    });
    assert!(
        has,
        "BT21-009 prints a standard Red Lv.2 / cost 0 digivolve circle — it must be \
         authored alongside the trait-gated special condition"
    );
}

/// Link condition: this card links onto [Appmon] hosts for cost 1.
#[test]
fn bt21_009_has_link_condition_appmon_cost_1() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 1
        )
    });
    assert!(has, "BT21-009 declares a self link-condition with cost 1");
}

/// Linked DP aura: +2000 DP to the host while this card is linked.
#[test]
fn bt21_009_has_linked_dp_aura_2000() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. })
                if *scope == CompiledScope::Linked && *dp_modifier == Some(2000)
        )
    });
    assert!(
        has,
        "BT21-009 declares a scope:linked DP aura of +2000 (link-box DP bonus)"
    );
}

/// Linked Raid grant: host gains Raid while this card is linked.
#[test]
fn bt21_009_has_linked_raid_grant() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Raid" && *scope == CompiledScope::Linked
        )
    });
    assert!(
        has,
        "BT21-009 declares a scope:linked Raid grant (link-box keyword)"
    );
}

/// Host-side when_card_linked_to_this triggered clause: exactly one, OPT, own scope.
#[test]
fn bt21_009_has_when_card_linked_to_this_once_per_turn() {
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

// ── Section 2: linked ESS behavioral ────────────────────────────────────────

/// [Link] +2000 DP reaches the host while BT21-009 is linked.
#[test]
fn bt21_009_linked_dp_bonus_2000_reaches_host() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let base_dp = r.game.effective_dp(host).expect("host has DP");

    r.push_linked_owned(host, CARD_ID, 0);
    // tick so the static DP aura is registered
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(host),
        Some(base_dp + 2000),
        "linked BT21-009 contributes +2000 DP to the host"
    );
}

/// [Link] Raid grant: host gains Raid keyword while BT21-009 is linked.
#[test]
fn bt21_009_linked_raid_reaches_host() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));

    assert!(
        !r.game.has_keyword(host, Keyword::Raid),
        "baseline: host has no Raid before linking"
    );

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(host, Keyword::Raid),
        "host gains <Raid> while BT21-009 is linked"
    );
}

// ── Section 3: condition gating ─────────────────────────────────────────────

/// Positive: 0 Tamers — when-linked trigger fires and offers Haru Shinkai.
#[test]
fn bt21_009_when_linked_0_tamers_fires() {
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    // No tamers on field: condition passes.
    fire_link_onto_host(&mut r, host);

    assert!(
        r.game.pending_selection.is_some(),
        "0 tamers: when-linked trigger should surface a selection for Haru Shinkai"
    );
}

/// Positive: 1 Tamer — trigger still fires (≤1 is the boundary).
#[test]
fn bt21_009_when_linked_1_tamer_fires() {
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
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
fn bt21_009_when_linked_2_tamers_no_fire() {
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
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
fn bt21_009_when_linked_opponents_turn_no_fire() {
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
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

// ── Section 4: behavioral outcome ───────────────────────────────────────────

/// When triggered: select Haru → play free from hand → Tamer on field.
#[test]
fn bt21_009_when_linked_plays_haru_shinkai_free() {
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    let field_before = r.battle_area_size(0);
    let hand_before = r.hand_size(0);

    fire_link_onto_host(&mut r, host);

    // Select the Haru Shinkai card.
    assert!(
        r.game.pending_selection.is_some(),
        "when-linked fires and offers Haru Shinkai selection"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    // Drain any follow-up prompts (e.g. play confirmation).
    r.game.drain_effect_queue();

    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "Haru Shinkai was played to the field (free)"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before - 1,
        "Haru Shinkai left the hand"
    );
}

/// Decline (PASS) → nothing happens.
#[test]
fn bt21_009_when_linked_decline_no_play() {
    use digimon_engine::action::space::PASS;
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
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

/// No Haru Shinkai in hand → no selection offered (condition gates out).
#[test]
fn bt21_009_when_linked_no_haru_in_hand_no_prompt() {
    let mut r = base()
        .hand(0, &[]) // empty hand
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, CARD_ID, Some(0));

    fire_link_onto_host(&mut r, host);

    // The DCGO CanActivateCondition checks HasMatchConditionOwnersHand before
    // activating — so no selection when there's no Haru.
    assert!(
        r.game.pending_selection.is_none(),
        "no Haru Shinkai in hand: no prompt should be installed"
    );
}

// ── Section 5: OPT enforcement ───────────────────────────────────────────────

/// Second link same turn → OPT lockout prevents a second prompt.
#[test]
fn bt21_009_when_linked_opt_blocks_second_link_same_turn() {
    use digimon_engine::action::space::PASS;
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
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
    // Fire OnLink again (simulates a second card being linked onto host).
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
fn bt21_009_when_linked_opt_resets_after_end_turn() {
    use digimon_engine::action::space::PASS;
    let mut r = base()
        .hand(0, &["HARU-IN-HAND"])
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

// ── Section 6: digivolve-route fidelity ──────────────────────────────────────
// The card prints BOTH requirements (card image + official DB bundle):
//   (a) standard circle  "Red Lv.2 / cost 0"
//   (b) special condition "[Digivolve] Lv.2 w/[Appmon]/[Hero] trait: Cost 0"
// Pre-fix (2026-07-10) only (b) was authored, so a plain red Lv.2 without
// [Appmon]/[Hero] had NO digivolve route into BT21-009.

/// A Lv.2 base Digimon with explicit colors/traits for digivolve-route tests.
fn make_lv2_base(
    id: &str,
    color: digimon_engine::enums::CardColor,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 1;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Digivolve BT21-009 from hand over the given base; returns (proceeded, memory delta).
fn try_digivolve_over(base_card: CardData) -> (bool, i16) {
    use digimon_engine::enums::{GamePhase, PlaySource};
    let base_id = base_card.card_id.clone();
    let mut r = base()
        .add_card(base_card)
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, &base_id, Some(0));

    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    r.game.drain_effect_queue();
    (proceeded, r.game.memory - mem_before)
}

/// Standard circle (a): a plain red Lv.2 with NO [Appmon]/[Hero] trait
/// digivolves into BT21-009 for the printed circle cost (0).
/// Pre-fix this failed: the YAML only authored the trait-gated condition.
#[test]
fn bt21_009_digivolves_from_plain_red_lv2_for_0() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) = try_digivolve_over(make_lv2_base(
        "RED-LV2-PLAIN",
        CardColor::Red,
        &[],
    ));
    assert!(
        proceeded,
        "printed standard circle Red Lv.2 / cost 0: a plain red Lv.2 must be a legal base"
    );
    assert_eq!(delta, 0, "the printed circle cost is 0 — no memory paid");
}

/// Special condition (b): an off-colour (blue) Lv.2 with the [Hero] trait
/// digivolves into BT21-009 for 0 via the trait-gated path.
#[test]
fn bt21_009_digivolves_from_offcolor_hero_lv2_for_0() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) = try_digivolve_over(make_lv2_base(
        "BLUE-LV2-HERO",
        CardColor::Blue,
        &["Hero"],
    ));
    assert!(
        proceeded,
        "special condition Lv.2 w/[Appmon]/[Hero]: an off-colour Hero Lv.2 must be a legal base"
    );
    assert_eq!(delta, 0, "the special-condition cost is 0 — no memory paid");
}

/// Negative: an off-colour Lv.2 with NO [Appmon]/[Hero] trait matches neither
/// printed requirement — no digivolve route may exist.
#[test]
fn bt21_009_no_route_from_offcolor_traitless_lv2() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) = try_digivolve_over(make_lv2_base(
        "BLUE-LV2-PLAIN",
        CardColor::Blue,
        &[],
    ));
    assert!(
        !proceeded,
        "a blue traitless Lv.2 matches neither the red circle nor the trait gate — \
         digivolve must be refused"
    );
    assert_eq!(delta, 0, "refused digivolve must not spend memory");
}
