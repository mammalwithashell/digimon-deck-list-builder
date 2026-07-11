//! BT25-070 Logamon — Digimon, Lv.4, Black/Purple dual, DP 6000, Cost 5.
//! Trait line (card image): Sup./Appmon | Social | Logoff.
//!
//! # Printed text (official bundle data/card_bundles/BT25-070.md + card image)
//! Digivolve circles: Black Lv.3 / 3 AND Purple Lv.3 / 3 (dual-ring circle)
//!   plus the rainbow "Stnd." circle / cost 2 (any colour, NO level — DCGO
//!   `AddSelfDigivolutionRequirementStaticEffect(HasStandardAppTraits, 2)`).
//! [App Fusion] [Offmon] & [Hackmon]: Cost 0.
//! [Main][Once Per Turn] You may link 1 [Social], [Tool] or [Game] trait
//!   Digimon card from your trash or this Digimon's digivolution cards to this
//!   Digimon with the cost reduced by 1.
//! [Your Turn][Once Per Turn] When this Digimon gets linked, delete 1 of your
//!   opponent's Digimon with a play cost of 4 or less. (MANDATORY — no "you
//!   may"; DCGO canNoSelect: false; skips silently with no legal target.)
//! Link box: <Link> [Appmon] trait: Cost 2; +DP 3000.
//! [When Linking] 1 of your opponent's Digimon or Tamers can't unsuspend
//!   until their turn ends. (link-source effect; DCGO SetIsLinkedEffect(true).)
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_070.cs
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3; BT25-052 is the near-twin)
//! - Standard-circle + trait-gated ("Stnd.") + App Fusion alt-path registration
//! - DigiLink Shape-B self link-condition + linked +3000 DP aura
//! - [Main] OPT activated self-link (trash/sources zone choice + OPT lockout)
//! - B3 when_card_linked_to_this host-side MANDATORY triggered delete with
//!   your_turn gate (positive + negative + OPT lockout + OPT reset)
//! - Linked-scope [When Linking] CannotUnsuspend (Digimon or Tamer)

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
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-070";

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

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-070 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-TRASH", 4, 4000, 4, &["Tool"]))
        .add_card(make_digimon("GAME-FODDER", 3, 1000, 2, &["Game"]))
        .add_card(make_digimon("APPMON-HOST", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("OPP-SMALL", 3, 3000, 3, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 5, 8000, 8, &["Beast"]))
        .add_card(make_tamer("OPP-TAMER"))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
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

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn is_unsuspendable(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has(h, ModifierType::CannotUnsuspend)
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

/// Fire Logamon's host-side when_card_linked_to_this trigger by pushing a
/// plain fodder card as a linked card onto `host` (the Logamon permanent)
/// and dispatching OnLink, then draining. (BT24-067/BT25-052 idiom.)
fn fire_link_onto_host(runner: &mut DebugRunner, host: PermanentHandle) {
    let linked_handle = runner.push_linked_owned(host, "GAME-FODDER", 0);
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
fn bt25_070_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Logamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.cost, Some(5));
    assert_eq!(
        card.color,
        vec![CompiledColor::Black, CompiledColor::Purple],
        "Logamon is a Black/Purple dual (both printed colours; was mis-authored purple-only)"
    );
}

/// Trait line (card image): Sup./Appmon | Social | Logoff — every segment must
/// be in `traits` (predicate `trait_has` consults only the traits list; other
/// cards gate on [Appmon] hosts, [Social] links and [Sup.] grade bases).
#[test]
fn bt25_070_traits_contain_sup_appmon_social_logoff() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t: Vec<&str> = card.traits.iter().map(String::as_str).collect();
    for expected in &["Sup.", "Appmon", "Social", "Logoff"] {
        assert!(
            t.contains(expected),
            "trait '{}' not found in traits {:?}",
            expected,
            t
        );
    }
}

/// Printed standard circles: Black Lv.3 / 3 AND Purple Lv.3 / 3 (dual-ring
/// circle; official Bandai DB), authored as bare {level_eq, color_is}
/// alt-paths per the printed-circle convention.
#[test]
fn bt25_070_has_both_standard_lv3_cost3_circles() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    for color in [CompiledColor::Black, CompiledColor::Purple] {
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
            "BT25-070 prints a standard {:?} Lv.3 / cost 3 circle — it must be authored",
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
fn bt25_070_has_stnd_trait_gate_cost2_no_level() {
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
        "BT25-070's rainbow 'Stnd.' circle must be a trait-only gate at cost 2 \
         (no level, no colour — DCGO HasStandardAppTraits)"
    );
}

/// [App Fusion] [Offmon] & [Hackmon]: Cost 0 (DCGO AddAppfuseMethodByName).
/// Pre-fix this was OMITTED as BLOCKED; the app_fusion alt-path primitive has
/// since landed (BT25-036/BT25-052/BT25-060 precedent).
#[test]
fn bt25_070_registers_app_fusion_cost0() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        has,
        "BT25-070 must register the cost-0 [App Fusion] [Offmon] & [Hackmon] alt-path"
    );
}

#[test]
fn bt25_070_has_link_condition_appmon_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 2
        )
    });
    assert!(has, "BT25-070 declares a self link-condition with cost 2");
}

/// Link box "+DP 3000": scope-linked aura applying +3000 DP to the host.
/// Pre-fix (2026-07-10) the aura was missing entirely.
#[test]
fn bt25_070_has_linked_dp_aura_3000() {
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
        "BT25-070 declares a scope:linked +3000 DP aura (printed link-box DP bonus)"
    );
}

/// [Main][Once Per Turn] activated self-link clause.
#[test]
fn bt25_070_has_main_once_per_turn_link_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
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

/// Host-side when_card_linked_to_this triggered clause: OPT, MANDATORY (the
/// printed delete has no "you may"), face-up scope.
#[test]
fn bt25_070_has_when_card_linked_to_this_once_per_turn_mandatory() {
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
    assert!(
        !clause.optional,
        "the printed delete is mandatory — no 'you may' (DCGO canNoSelect: false)"
    );
}

/// [When Linking] can't-unsuspend clause: linked scope (the card image's lower
/// box is the LINK effect — DCGO WhenLinked + SetIsLinkedEffect(true)).
#[test]
fn bt25_070_has_linked_when_linking_clause() {
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
        "must have a linked [When Linking] can't-unsuspend clause"
    );
}

// ── Section 2: [Main] activated self-link + chained when-linked delete ───────

/// [Main] links a [Tool] card from the TRASH to Logamon; the resulting link
/// then fires the [Your Turn] when-linked trigger, which MUST offer only the
/// cost-<=4 opponent Digimon (OPP-SMALL, cost 3 — not OPP-BIG, cost 8) as a
/// MANDATORY delete.
#[test]
fn bt25_070_main_links_from_trash_then_when_linked_deletes_small_opp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0)); // cost 3 — deletable
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // cost 8 — safe
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    // [Main] activated self-link from trash.
    assert!(
        fire_main(&mut r, 0, loga.index as usize),
        "[Main] self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    assert_eq!(
        r.game.player(0).battle_area[loga.index as usize]
            .linked_cards
            .len(),
        1,
        "Tool card linked from trash"
    );

    // When-linked: delete 1 opp Digimon cost <=4 (only OPP-SMALL eligible).
    assert!(
        r.game.pending_selection.is_some(),
        "When-linked delete prompt surfaces"
    );
    assert!(
        !r.pending_is_optional(),
        "the delete is mandatory — no 'you may' printed"
    );
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .len(),
        1,
        "only the play-cost-<=4 Digimon is a legal target (OPP-BIG cost 8 excluded)"
    );
    let del_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, del_action);

    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "the cost-<=4 opponent Digimon was deleted"
    );
}

/// The [Main] link's second printed source zone: this Digimon's DIGIVOLUTION
/// CARDS. A [Game] source under Logamon can be pulled out and linked.
#[test]
fn bt25_070_main_links_from_digivolution_sources() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    r.push_source(loga, "GAME-FODDER");
    advance_to_main(&mut r);

    let sources_before = r.game.player(0).battle_area[loga.index as usize]
        .digivolution_cards()
        .len();

    assert!(
        fire_main(&mut r, 0, loga.index as usize),
        "[Main] self-link from digivolution cards installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[loga.index as usize]
            .linked_cards
            .len(),
        1,
        "the Game-trait source was linked to Logamon"
    );
    assert_eq!(
        r.game.player(0).battle_area[loga.index as usize]
            .digivolution_cards()
            .len(),
        sources_before - 1,
        "the linked card left the digivolution cards"
    );
}

/// [Once Per Turn]: a second [Main] activation in the same turn is locked out
/// even though another eligible card remains in the trash.
#[test]
fn bt25_070_main_opt_locks_out_second_activation_same_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    seed_trash(&mut r, 0, "GAME-FODDER");
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    // First activation: link one card (and drain the chained when-linked
    // trigger — no opponent Digimon exist so it skips silently).
    assert!(
        fire_main(&mut r, 0, loga.index as usize),
        "first [Main] activation installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    r.auto_resolve().ok();

    // Second activation same turn: OPT must block despite the remaining card.
    assert!(
        !fire_main(&mut r, 0, loga.index as usize),
        "[Once Per Turn]: second [Main] activation in the same turn must not prompt"
    );
    assert_eq!(
        r.game.player(0).battle_area[loga.index as usize]
            .linked_cards
            .len(),
        1,
        "only the first activation linked a card"
    );
}

/// No eligible card anywhere (trash empty, no digivolution cards): the [Main]
/// activation has no candidates and installs no selection.
#[test]
fn bt25_070_main_no_candidates_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    assert!(
        !fire_main(&mut r, 0, loga.index as usize),
        "no [Social]/[Tool]/[Game] Digimon card available: no selection"
    );
}

// ── Section 3: linked DP aura (+3000 to host) ────────────────────────────────

#[test]
fn bt25_070_linked_host_gains_3000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let dp_before = r.effective_dp(host).expect("host on field");

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 3000),
        "host effective DP +3000 while Logamon is linked (printed Link DP +3000)"
    );
}

// ── Section 4: [When Linking] can't unsuspend (linked scope, mandatory) ──────

/// Link Logamon onto an [Appmon] host via the link action (pays the printed
/// link cost 2). [When Linking] then locks an opponent Digimon's unsuspend
/// until their turn ends.
#[test]
fn bt25_070_when_linking_locks_opp_digimon_unsuspend() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    assert!(!is_unsuspendable(&r, opp), "opp not yet locked");

    r.game.decode_action(link_bit(loga) as u16, 0);
    // Host select (if prompted), then the lock-target select.
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
        is_unsuspendable(&r, opp),
        "[When Linking] applied CannotUnsuspend to the chosen opponent Digimon"
    );
}

/// The printed target set is "Digimon or Tamers": an opponent TAMER is a
/// legal lock target too.
#[test]
fn bt25_070_when_linking_can_lock_opp_tamer() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = r.place_on_field(1, "OPP-TAMER", Some(0));
    advance_to_main(&mut r);

    r.game.decode_action(link_bit(loga) as u16, 0);
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
        is_unsuspendable(&r, opp_tamer),
        "[When Linking] can lock an opponent Tamer's unsuspend"
    );
}

// ── Section 5: [Your Turn][Once Per Turn] when-linked delete negatives ───────

/// Negative: opponent's turn — the [Your Turn] gate blocks the trigger.
#[test]
fn bt25_070_when_linked_opponents_turn_no_fire() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    // Advance to player 1's turn.
    r.end_turn();
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = r.battle_area_size(1);

    fire_link_onto_host(&mut r, loga);

    assert!(
        r.game.pending_selection.is_none(),
        "opponent's turn: [Your Turn] gate must block the trigger"
    );
    assert_eq!(r.battle_area_size(1), opp_before, "nothing deleted");
}

/// Negative: no opponent Digimon with play cost <=4 exists (only OPP-BIG,
/// cost 8) — the mandatory delete simply skips (DCGO
/// HasMatchConditionPermanent gate); nothing is deleted.
#[test]
fn bt25_070_when_linked_no_eligible_target_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    advance_to_main(&mut r);
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // cost 8 — ineligible
    let opp_before = r.battle_area_size(1);

    fire_link_onto_host(&mut r, loga);

    assert!(
        r.game.pending_selection.is_none(),
        "no play-cost-<=4 opponent Digimon: no delete prompt"
    );
    assert_eq!(
        r.battle_area_size(1),
        opp_before,
        "the cost-8 Digimon must survive"
    );
}

/// [Once Per Turn]: a second link in the same turn must not re-fire the
/// delete trigger.
#[test]
fn bt25_070_when_linked_opt_blocks_second_link_same_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    advance_to_main(&mut r);
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-SMALL", Some(0));
    r.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = r.battle_area_size(1);

    // First link — fires; take the delete (consumes the OPT).
    fire_link_onto_host(&mut r, loga);
    assert!(r.game.pending_selection.is_some(), "first link fires");
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();
    assert_eq!(r.battle_area_size(1), opp_before - 1, "first delete landed");

    // Second link the same turn: OPT must block despite a remaining target.
    fire_link_onto_host(&mut r, loga);
    assert!(
        r.game.pending_selection.is_none(),
        "OPT: second link in the same turn must be locked out"
    );
    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "no second deletion in the same turn"
    );
}

/// OPT resets across turns: after an end-turn cycle the trigger fires again.
#[test]
fn bt25_070_when_linked_opt_resets_after_end_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    advance_to_main(&mut r);
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-SMALL", Some(0));
    r.place_on_field(1, "OPP-SMALL", Some(0));

    // First link — fires; take the delete.
    fire_link_onto_host(&mut r, loga);
    assert!(r.game.pending_selection.is_some(), "first link fires");
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    // End-turn cycle: player 0 → player 1 → player 0 again.
    r.end_turn();
    r.end_turn();
    r.game.enter_main_phase();

    fire_link_onto_host(&mut r, loga);

    assert!(
        r.game.pending_selection.is_some(),
        "OPT must clear after end_turn cycle — the trigger fires again"
    );
}

// ── Section 6: digivolve-route fidelity ───────────────────────────────────────
// Printed requirements: (a) Black Lv.3 / 3, (b) Purple Lv.3 / 3 (dual-ring
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

/// Digivolve BT25-070 from hand over the given base; returns (proceeded, memory delta).
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

/// Standard circle (a): plain black Lv.3 → legal base at cost 3.
#[test]
fn bt25_070_digivolves_from_plain_black_lv3_for_3() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("BLACK-LV3-PLAIN", CardColor::Black, &[]));
    assert!(
        proceeded,
        "printed circle Black Lv.3 / 3: a plain black Lv.3 must be a legal base"
    );
    assert_eq!(delta, -3, "the printed circle cost is 3");
}

/// Standard circle (b): plain purple Lv.3 → legal base at cost 3 (the
/// off-primary half of the split circle).
#[test]
fn bt25_070_digivolves_from_plain_purple_lv3_for_3() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("PURPLE-LV3-PLAIN", CardColor::Purple, &[]));
    assert!(
        proceeded,
        "printed circle Purple Lv.3 / 3: a plain purple Lv.3 must be a legal base"
    );
    assert_eq!(delta, -3, "the printed circle cost is 3");
}

/// "Stnd." circle (c): an OFF-COLOUR (blue) Lv.3 with the [Stnd.] trait is a
/// legal base at cost 2 — the rainbow circle is colour-free and trait-gated.
/// Pre-fix the gate string "Standard App" made this path unreachable.
#[test]
fn bt25_070_digivolves_from_offcolor_stnd_lv3_for_2() {
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
fn bt25_070_no_route_from_offcolor_traitless_lv3() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv3_base("BLUE-LV3-PLAIN", CardColor::Blue, &[]));
    assert!(
        !proceeded,
        "a blue traitless Lv.3 matches no printed requirement — digivolve must be refused"
    );
    assert_eq!(delta, 0, "refused digivolve must not spend memory");
}
