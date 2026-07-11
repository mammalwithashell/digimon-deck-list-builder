//! BT25-052 Logimon — Digimon, Lv.4, Green/Red dual, DP 6000, Cost 5.
//! Trait line (card image): Sup./Appmon | Social | Login.
//!
//! # Printed text (official bundle data/card_bundles/BT25-052.md + card image)
//! Digivolve circles: Green Lv.3 / 3 AND Red Lv.3 / 3 (dual-ring circle) plus
//!   the rainbow "Stnd." circle / cost 2 (any colour, NO level — DCGO
//!   `AddSelfDigivolutionRequirementStaticEffect(HasStandardAppTraits, 2)`).
//! [App Fusion] [Onmon] & [Gatchmon]: Cost 0.
//! [Main][Once Per Turn] You may link 1 [Social], [Tool] or [Game] trait
//!   Digimon card from your hand or this Digimon's digivolution cards to this
//!   Digimon with the cost reduced by 1.
//! [Your Turn][Once Per Turn] When this Digimon gets linked, if you have 1 or
//!   fewer Tamers, you may play 1 [Kazuki & Itsuki] from your hand without
//!   paying the cost.
//! Link box: [Appmon] trait: Cost 2; +DP 3000.
//! [When Linking] Suspend 1 of your opponent's Digimon or Tamers.
//!   (link-source effect; cards.json mis-slots it as inherited.)
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_052.cs
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3; BT24-067 is the near-twin)
//! - Standard-circle + trait-gated ("Stnd.") + App Fusion alt-path registration
//! - DigiLink Shape-B self link-condition + linked +3000 DP aura
//! - [Main] OPT activated self-link (hand/sources zone choice)
//! - B3 when_card_linked_to_this host-side triggered OPT with your_turn gate
//!   and Tamer-count condition (positive + negative + OPT lockout)
//! - Linked-scope [When Linking] suspend (Digimon or Tamer)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-052";

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

/// Kazuki & Itsuki — a Tamer card named "Kazuki & Itsuki" (the [Your Turn]
/// payoff plays it from hand by NAME).
fn make_kazuki(id: &str) -> CardData {
    let mut card = make_test_card(id, "Kazuki & Itsuki");
    card.card_kind = CardKind::Tamer;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-052 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-HAND", 3, 2000, 3, &["Tool"]))
        .add_card(make_digimon("APPMON-HOST", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("OPP-DIGI", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("LINK-FODDER", 3, 1000, 2, &["Game"]))
        .add_card(make_kazuki("KAZ-IN-HAND"))
        .add_card(make_tamer("TAMER-A"))
        .add_card(make_tamer("TAMER-B"))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Fire the on-field [Main] activated effect of the permanent at `field_index`.
fn fire_main(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner
        .game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

/// Fire Logimon's host-side when_card_linked_to_this trigger by pushing a
/// plain fodder card as a linked card onto `host` (the Logimon permanent)
/// and dispatching OnLink, then draining. (BT24-067 idiom.)
fn fire_link_onto_host(runner: &mut DebugRunner, host: PermanentHandle) {
    let linked_handle = runner.push_linked_owned(host, "LINK-FODDER", 0);
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

fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

// ── Section 1: structural assertions ─────────────────────────────────────────

#[test]
fn bt25_052_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Logimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.cost, Some(5));
    assert_eq!(
        card.color,
        vec![CompiledColor::Green, CompiledColor::Red],
        "Logimon is a Green/Red dual (both printed colours)"
    );
}

/// Trait line (card image): Sup./Appmon | Social | Login — every segment must
/// be in `traits` (predicate `trait_has` consults only the traits list; other
/// cards gate on [Appmon] hosts and [Sup.] grade bases).
#[test]
fn bt25_052_traits_contain_sup_appmon_social_login() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t: Vec<&str> = card.traits.iter().map(String::as_str).collect();
    for expected in &["Sup.", "Appmon", "Social", "Login"] {
        assert!(
            t.contains(expected),
            "trait '{}' not found in traits {:?}",
            expected,
            t
        );
    }
}

/// Printed standard circles: Green Lv.3 / 3 AND Red Lv.3 / 3 (dual-ring
/// circle; official Bandai DB), authored as bare {level_eq, color_is}
/// alt-paths per the printed-circle convention
/// (tests/alt_path_printed_cost_guard.rs).
#[test]
fn bt25_052_has_both_standard_lv3_cost3_circles() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    for color in [CompiledColor::Green, CompiledColor::Red] {
        let has = card.alt_paths.iter().any(|p| {
            matches!(p.kind, CompiledAltPathKind::Digivolve)
                && p.cost == Some(CompiledCost::Literal(3))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(3)
                        && f.color_is == Some(color)
                        && f.trait_has.is_none()
                })
        });
        assert!(
            has,
            "BT25-052 prints a standard {:?} Lv.3 / cost 3 circle — it must be authored",
            color
        );
    }
}

/// "Stnd." circle / cost 2: trait gate ONLY. DCGO
/// AddSelfDigivolutionRequirementStaticEffect(HasStandardAppTraits, 2) has no
/// level parameter and HasStandardAppTraits == EqualsTraits("Stnd.").
/// Pre-fix (2026-07-10) this was authored as { level_eq: 3, trait_has:
/// "Standard App" } — a trait string no card carries, so the path was dead.
#[test]
fn bt25_052_has_stnd_trait_gate_cost2_no_level() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from.as_ref().is_some_and(|f| {
                f.trait_has.as_deref() == Some("Stnd.")
                    && f.level_eq.is_none()
                    && f.color_is.is_none()
            })
    });
    assert!(
        has,
        "BT25-052's rainbow 'Stnd.' circle must be a trait-only gate at cost 2 \
         (no level, no colour — DCGO HasStandardAppTraits)"
    );
}

/// [App Fusion] [Onmon] & [Gatchmon]: Cost 0 (DCGO AddAppfuseMethodByName).
/// Pre-fix this was OMITTED as BLOCKED; the app_fusion alt-path primitive has
/// since landed (BT25-036/BT25-060 precedent).
#[test]
fn bt25_052_registers_app_fusion_cost0() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        has,
        "BT25-052 must register the cost-0 [App Fusion] [Onmon] & [Gatchmon] alt-path"
    );
}

#[test]
fn bt25_052_has_link_condition_appmon_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 2
        )
    });
    assert!(has, "BT25-052 declares a self link-condition with cost 2");
}

/// Link box "+DP 3000": scope-linked aura applying +3000 DP to the host.
/// Pre-fix (2026-07-10) the aura was missing entirely.
#[test]
fn bt25_052_has_linked_dp_aura_3000() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(3000),
                ..
            })
        )
    });
    assert!(
        has,
        "BT25-052 declares a scope:linked +3000 DP aura (printed link-box DP bonus)"
    );
}

/// [Main][Once Per Turn] activated self-link clause.
#[test]
fn bt25_052_has_main_once_per_turn_link_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::MainOnField) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main] activated clause present");
    assert!(t.once_per_turn, "[Once Per Turn] flag set");
    assert!(
        t.process
            .iter()
            .any(|s| matches!(s, CompiledStep::LinkCards { .. })),
        "[Main] clause links a card to this Digimon"
    );
}

/// Host-side when_card_linked_to_this triggered clause: OPT, optional,
/// face-up scope (printed [Your Turn] [Once Per Turn] ... you may).
#[test]
fn bt25_052_has_when_card_linked_to_this_once_per_turn() {
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

/// [When Linking] suspend clause: linked scope (the card image's lower box is
/// the LINK effect; cards.json mis-slots it as "inherited" — DCGO models it
/// as WhenLinked + SetIsLinkedEffect(true)).
#[test]
fn bt25_052_has_linked_when_linking_suspend() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let when_linked = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenLinked)
                    && matches!(t.scope, CompiledScope::Linked)
        )
    });
    assert!(
        when_linked,
        "must have a linked [When Linking] suspend clause"
    );
}

// ── Section 2: [Main] activated self-link ─────────────────────────────────────

#[test]
fn bt25_052_main_links_tool_from_hand_to_self() {
    let mut r = base()
        .hand(0, &["TOOL-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    // [Main] activated link from hand installs a selection over the Tool card.
    assert!(
        fire_main(&mut r, 0, logi.index as usize),
        "[Main] self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    assert_eq!(
        r.game.player(0).battle_area[logi.index as usize]
            .linked_cards
            .len(),
        1,
        "the Tool card from hand attached to Logimon"
    );
    assert_eq!(r.hand_size(0), 0, "Tool card left the hand");
}

/// [Once Per Turn]: a second [Main] activation in the same turn is locked out
/// even though another eligible card remains in hand.
#[test]
fn bt25_052_main_opt_locks_out_second_activation_same_turn() {
    let mut r = base()
        .hand(0, &["TOOL-IN-HAND", "LINK-FODDER"]) // two eligible cards
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    // First activation: link one card.
    assert!(
        fire_main(&mut r, 0, logi.index as usize),
        "first [Main] activation installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    r.auto_resolve().ok();

    // Second activation same turn: OPT must block despite the remaining card.
    assert!(
        !fire_main(&mut r, 0, logi.index as usize),
        "[Once Per Turn]: second [Main] activation in the same turn must not prompt"
    );
    assert_eq!(
        r.game.player(0).battle_area[logi.index as usize]
            .linked_cards
            .len(),
        1,
        "only the first activation linked a card"
    );
}

/// No eligible card anywhere (hand empty, no digivolution cards): the [Main]
/// activation has no candidates and installs no selection.
#[test]
fn bt25_052_main_no_candidates_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    assert!(
        !fire_main(&mut r, 0, logi.index as usize),
        "no [Social]/[Tool]/[Game] Digimon card available: no selection"
    );
}

// ── Section 3: linked DP aura (+3000 to host) ────────────────────────────────

#[test]
fn bt25_052_linked_host_gains_3000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let dp_before = r.effective_dp(host).expect("host on field");

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 3000),
        "host effective DP +3000 while Logimon is linked"
    );
}

// ── Section 4: [When Linking] suspend (linked scope, mandatory) ──────────────

/// Link Logimon onto an [Appmon] host via the link action (pays the printed
/// link cost 2). [When Linking] then suspends an opponent Digimon.
#[test]
fn bt25_052_when_linking_suspends_opp_digimon() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    assert!(
        !r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "opp Digimon starts unsuspended"
    );

    r.game.decode_action(link_bit(logi) as u16, 0);
    // Host select (if prompted), then the suspend target select.
    if r.game.pending_selection.is_some() {
        let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
    }
    if r.game.pending_selection.is_some() {
        let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
    }
    r.auto_resolve().ok();

    assert!(
        r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "[When Linking] suspended the opponent Digimon"
    );
}

/// The printed target set is "Digimon or Tamers": an opponent TAMER is a
/// legal suspend target too.
#[test]
fn bt25_052_when_linking_can_suspend_opp_tamer() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = r.place_on_field(1, "TAMER-A", Some(0));
    advance_to_main(&mut r);

    r.game.decode_action(link_bit(logi) as u16, 0);
    if r.game.pending_selection.is_some() {
        let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
    }
    if r.game.pending_selection.is_some() {
        let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
    }
    r.auto_resolve().ok();

    assert!(
        r.game.player(1).battle_area[opp_tamer.index as usize].is_suspended,
        "[When Linking] can suspend an opponent Tamer"
    );
}

// ── Section 5: [Your Turn][OPT] when-linked → play Kazuki & Itsuki free ───────

/// Positive: 0 Tamers, Kazuki in hand, your turn — the trigger fires and the
/// selected Kazuki & Itsuki is played from hand for free.
#[test]
fn bt25_052_when_linked_plays_kazuki_free() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));

    let field_before = r.battle_area_size(0);
    let hand_before = r.hand_size(0);
    let mem_before = r.game.memory;

    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_some(),
        "when-linked fires and offers the Kazuki & Itsuki selection"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "Kazuki & Itsuki was played to the field"
    );
    assert_eq!(r.hand_size(0), hand_before - 1, "Kazuki left the hand");
    assert_eq!(
        r.game.memory, mem_before,
        "played without paying the cost — memory unchanged"
    );
}

/// Positive boundary: exactly 1 Tamer still satisfies "1 or fewer".
#[test]
fn bt25_052_when_linked_1_tamer_fires() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TAMER-A", Some(0));

    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_some(),
        "1 tamer: the trigger still fires (<=1 boundary)"
    );
}

/// Negative: 2 Tamers — the condition fails, no prompt.
#[test]
fn bt25_052_when_linked_2_tamers_no_fire() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TAMER-A", Some(0));
    r.place_on_field(0, "TAMER-B", Some(0));

    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_none(),
        "2 tamers: the trigger must be suppressed"
    );
    assert_eq!(r.hand_size(0), 1, "Kazuki stays in hand");
}

/// Negative: opponent's turn — the [Your Turn] gate blocks the trigger.
/// Pre-fix (2026-07-10) the YAML had no your_turn gate at all.
#[test]
fn bt25_052_when_linked_opponents_turn_no_fire() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    // Advance to player 1's turn.
    r.end_turn();
    let logi = r.place_on_field(0, CARD_ID, Some(0));

    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_none(),
        "opponent's turn: [Your Turn] gate must block the trigger"
    );
}

/// Decline (PASS): "you may" — nothing is played.
#[test]
fn bt25_052_when_linked_decline_no_play() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));

    let field_before = r.battle_area_size(0);
    let hand_before = r.hand_size(0);

    fire_link_onto_host(&mut r, logi);

    assert!(r.game.pending_selection.is_some(), "when-linked fires");
    assert!(r.pending_is_optional(), "prompt must be optional (you may)");
    let _ = r.game.resolve_selection(0, PASS);

    assert_eq!(r.battle_area_size(0), field_before, "PASS: no card played");
    assert_eq!(r.hand_size(0), hand_before, "PASS: hand unchanged");
}

/// No Kazuki & Itsuki in hand: no prompt (DCGO CanActivateCondition checks
/// HasMatchConditionOwnersHand before activating).
#[test]
fn bt25_052_when_linked_no_kazuki_in_hand_no_prompt() {
    let mut r = base()
        .hand(0, &[])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));

    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_none(),
        "no Kazuki & Itsuki in hand: no prompt should be installed"
    );
}

/// OPT lockout: a second link in the same turn must not re-fire the trigger.
#[test]
fn bt25_052_when_linked_opt_blocks_second_link_same_turn() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));

    // First link — fires; take the play (consumes the OPT).
    fire_link_onto_host(&mut r, logi);
    assert!(r.game.pending_selection.is_some(), "first link fires");
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    // Refill the hand so a candidate exists again, then link a second time.
    push_to_hand(&mut r, 0, "KAZ-IN-HAND");
    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_none(),
        "OPT: second link in the same turn must be locked out"
    );
}

/// OPT resets across turns: after an end-turn cycle the trigger fires again.
#[test]
fn bt25_052_when_linked_opt_resets_after_end_turn() {
    let mut r = base()
        .hand(0, &["KAZ-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    advance_to_main(&mut r);
    let logi = r.place_on_field(0, CARD_ID, Some(0));

    // First link — fires; decline.
    fire_link_onto_host(&mut r, logi);
    if r.game.pending_selection.is_some() {
        let _ = r.game.resolve_selection(0, PASS);
    }

    // End-turn cycle: player 0 → player 1 → player 0 again.
    r.end_turn();
    r.end_turn();
    r.game.enter_main_phase();

    fire_link_onto_host(&mut r, logi);

    assert!(
        r.game.pending_selection.is_some(),
        "OPT must clear after end_turn cycle — the trigger fires again"
    );
}

// ── Section 6: digivolve-route fidelity ───────────────────────────────────────
// Printed requirements: (a) Green Lv.3 / 3, (b) Red Lv.3 / 3 (dual-ring
// standard circle), (c) rainbow "Stnd." circle / cost 2 (trait gate, any
// colour). Pre-fix only a dead `{ level_eq: 3, trait_has: "Standard App" }`
// path was authored — no card carries a "Standard App" trait.

fn make_lv3_base(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 2;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Digivolve BT25-052 from hand over the given base; returns (proceeded, memory delta).
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

/// Standard circle (a): plain green Lv.3 → legal base at cost 3.
#[test]
fn bt25_052_digivolves_from_plain_green_lv3_for_3() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("GREEN-LV3-PLAIN", CardColor::Green, &[]));
    assert!(
        proceeded,
        "printed circle Green Lv.3 / 3: a plain green Lv.3 must be a legal base"
    );
    assert_eq!(delta, -3, "the printed circle cost is 3");
}

/// Standard circle (b): plain red Lv.3 → legal base at cost 3 (the off-primary
/// circle from the user's original bug report).
#[test]
fn bt25_052_digivolves_from_plain_red_lv3_for_3() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("RED-LV3-PLAIN", CardColor::Red, &[]));
    assert!(
        proceeded,
        "printed circle Red Lv.3 / 3: a plain red Lv.3 must be a legal base"
    );
    assert_eq!(delta, -3, "the printed circle cost is 3");
}

/// "Stnd." circle (c): an OFF-COLOUR (blue) Lv.3 with the [Stnd.] trait is a
/// legal base at cost 2 — the rainbow circle is colour-free and trait-gated.
/// Pre-fix the gate string "Standard App" made this path unreachable.
#[test]
fn bt25_052_digivolves_from_offcolor_stnd_lv3_for_2() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("BLUE-LV3-STND", CardColor::Blue, &["Stnd."]));
    assert!(
        proceeded,
        "rainbow 'Stnd.' circle / cost 2: an off-colour [Stnd.] base must be legal"
    );
    assert_eq!(delta, -2, "the 'Stnd.' circle cost is 2");
}

/// Negative: an off-colour Lv.3 with no [Stnd.] trait matches neither the
/// printed circles nor the trait gate — no digivolve route may exist.
#[test]
fn bt25_052_no_route_from_offcolor_traitless_lv3() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("BLUE-LV3-PLAIN", CardColor::Blue, &[]));
    assert!(
        !proceeded,
        "a blue traitless Lv.3 matches no printed requirement — digivolve must be refused"
    );
    assert_eq!(delta, 0, "refused digivolve must not spend memory");
}
