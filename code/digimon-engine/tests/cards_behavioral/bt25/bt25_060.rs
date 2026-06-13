//! BT25-060 Rebootmon — Digimon, Lv.6, Green, DP 12000, Play Cost 12.
//! Trait line (card image): God/Appmon | God | Reboot →
//!   traits: [God, Appmon, Reboot], attribute: God.
//!
//! # Corrected printed card text (card image — authoritative)
//! Digivolve boxes: Lv.5 / Cost 4 (standard, evo_costs) + Ult. / Cost 4 (alt).
//! App Fusion [Bootmon] & [Shutmon]: Cost 0.
//!   "If 2 such cards are linked together, stack the link card on top and digivolve."
//! ＜Security A. +1＞ ＜Reboot＞ ＜Link +1＞ (Add 1 to this Digimon's maximum links.)
//! [When Digivolving] [When Attacking] [Once Per Turn] By linking 1 [Appmon] trait
//!   Digimon card from your hand or this Digimon's digivolution cards to this Digimon
//!   without paying the cost, 1 of your Digimon may unsuspend.
//! [All Turns] [Once Per Turn] When this Digimon gets linked or unsuspends, until
//!   your turn ends, this Digimon gains ＜Piercing＞ and ＜Blocker＞, and your
//!   opponent's Digimon effects don't affect it.
//! (No printed Link box — Rebootmon is NOT a link card; no link_condition.)
//!
//! # DCGO C# reference (authoritative — READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_060.cs
//!   - AddSelfDigivolutionRequirementStaticEffect(HasUltimateAppTraits, cost 4) — alt-digivolve.
//!   - AddAppfuseMethodByName(["Bootmon","Shutmon"]) — App Fusion alt-play.
//!   - ChangeSelfSAttackStaticEffect(+1), RebootSelfStaticEffect, ChangeSelfLinkMaxStaticEffect(1).
//!   - Shared WD/WA (maxCountPerTurn 1, shared hash): link 1 [Appmon] from hand/
//!     digivolution cards to self (free); if linked, 1 of your Digimon MAY unsuspend
//!     (canNoSelect: true). If NOT linked, RemoveUse() (OPT refunded).
//!   - Shared All-Turns (WhenLinked + OnUnTappedAnyone, shared hash, maxCount 1):
//!     gain <Piercing> + <Blocker> + DigimonEffectImmunity UntilOwnerTurnEnd.
//!
//! Pattern tags: Appmon link box (no link_condition — receives links), Ult.
//!   alt-digivolve trait gate, App Fusion (Gap 4), <Link +1> aura (ChangeLinkMax),
//!   link_card_to_self self-host, dual-timing OPT grant (when_card_linked_to_this
//!   + on_unsuspend), temp keyword grant + effect-immunity (end_of_your_turn).
//! Sister card precedent: BT21-101 Gaiamon (same God/Appmon Lv.6 Ult.+App Fusion
//!   +Link+1 shell); AD1-009 (grant_effect_immunity self, end-of-turn keyword grant).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectSourceKind, EffectTiming, Keyword, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-060";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Named Digimon for App-Fusion condition matching (Bootmon / Shutmon).
fn named_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(7000);
    card.play_cost = 7;
    card.traits = vec!["Appmon".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-060 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // [Appmon] Digimon card that can be linked from hand / sources.
        .add_card(make_digimon("APPMON-IN-HAND", 4, 4000, 4, &["Appmon", "Social"]))
        // A non-Appmon Digimon — must NOT be linkable.
        .add_card(make_digimon("NON-APPMON", 4, 4000, 4, &["Beast"]))
        // A second own Digimon to be the unsuspend target.
        .add_card(make_digimon("OWN-ALLY", 4, 5000, 4, &["Beast"]))
        // App-Fusion named materials.
        .add_card(named_digimon("BOOT", "Bootmon"))
        .add_card(named_digimon("SHUT", "Shutmon"))
        // An opponent Digimon (effect-immunity probe target).
        .add_card(make_digimon("OPP-DIGI", 5, 8000, 8, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Fire the [When Digivolving] activated effect of the permanent at `field_index`.
fn fire_when_digivolving(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

/// Fire the [When Attacking] activated effect of the permanent at `field_index`.
fn fire_when_attacking(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

/// Drain every queued selection by always taking the FIRST valid action (or
/// PASS when none). Used to clear follow-up prompts whose specific branch is not
/// the subject of the test.
fn resolve_all_first(r: &mut DebugRunner, player: PlayerId) {
    while r.game.pending_selection.is_some() {
        let act = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .first()
            .copied();
        match act {
            Some(a) => {
                let _ = r.game.resolve_selection(player, a);
            }
            None => {
                let _ = r.game.resolve_selection(player, digimon_engine::action::space::PASS);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// STRUCTURAL TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt25_060_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Rebootmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    for t in ["God", "Appmon", "Reboot"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "trait {t} present in merged trait line"
        );
    }
}

#[test]
fn bt25_060_has_no_link_condition() {
    // Rebootmon prints NO "Link [Appmon]: Cost N" box — it is a finisher, not a
    // link card. It still RECEIVES links (host-side) but cannot link onto a host.
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has_lc = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { .. })
        )
    });
    assert!(!has_lc, "Rebootmon declares NO link_condition (no printed link box)");
}

#[test]
fn bt25_060_grants_security_attack_plus_one() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, value, .. })
            if keyword == "SecurityAttackPlus" && *value == Some(1)
    ));
    assert!(has, "<Security A. +1> declared as grant_keyword SecurityAttackPlus value 1");
}

#[test]
fn bt25_060_grants_reboot() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "Reboot"
    ));
    assert!(has, "<Reboot> declared as grant_keyword Reboot");
}

#[test]
fn bt25_060_has_link_plus_one_aura() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura { modifier, modifier_value, .. })
            if modifier.as_deref() == Some("ChangeLinkMax") && *modifier_value == Some(1)
    ));
    assert!(has, "<Link +1> declared as aura ChangeLinkMax modifier_value 1");
}

#[test]
fn bt25_060_has_ult_alt_digivolve() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let count = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert!(count >= 1, "at least one digivolve alt-path (Ult. / Cost 4) declared");
}

#[test]
fn bt25_060_has_app_fusion_bootmon_shutmon() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::AppFusion));
    assert!(has, "App Fusion [Bootmon] & [Shutmon] declared as app_fusion alt-path");
}

#[test]
fn bt25_060_has_wd_wa_link_then_unsuspend_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        let CompiledClause::Triggered(t) = c else { return false };
        let wd = t.when.contains(&CompiledTiming::WhenDigivolving);
        let wa = t.when.contains(&CompiledTiming::WhenAttacking);
        let links = t
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::LinkCardToSelf { .. }));
        let unsuspends = step_tree_has_unsuspend(&t.process);
        wd && wa && t.once_per_turn && links && unsuspends
    });
    assert!(
        has,
        "[WD][WA][OPT] clause links an Appmon to self then unsuspends an own Digimon"
    );
}

#[test]
fn bt25_060_has_all_turns_dual_timing_grant_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        let CompiledClause::Triggered(t) = c else { return false };
        let linked = t.when.contains(&CompiledTiming::WhenCardLinkedToThis);
        let unsusp = t.when.contains(&CompiledTiming::OnUnsuspend);
        let grants_kw = t.process.iter().any(|s| matches!(s, CompiledStep::GrantKeyword { .. }));
        let grants_immunity = t
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::GrantEffectImmunity { .. }));
        linked && unsusp && t.once_per_turn && grants_kw && grants_immunity
    });
    assert!(
        has,
        "[All Turns][OPT] clause fires on link-to-this OR unsuspend, grants keywords + immunity"
    );
}

/// Recursive scan for an `Unsuspend` step inside a body.
fn step_tree_has_unsuspend(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::Unsuspend { .. } => true,
        CompiledStep::Optional(body) => step_tree_has_unsuspend(body),
        CompiledStep::If { then, else_branch, .. } => {
            step_tree_has_unsuspend(then) || step_tree_has_unsuspend(else_branch)
        }
        CompiledStep::ForEach { body, .. } => step_tree_has_unsuspend(body),
        _ => false,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIORAL TESTS
// ─────────────────────────────────────────────────────────────────────────────

/// [WD] By linking 1 [Appmon] from hand, then unsuspend a chosen own Digimon.
#[test]
fn bt25_060_wd_links_appmon_from_hand_then_unsuspends_chosen_ally() {
    let mut r = base()
        .hand(0, &["APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    // Suspend the ally so the unsuspend is observable.
    r.game.suspend(ally);
    assert!(
        r.game.player(0).battle_area[ally.index as usize].is_suspended,
        "ally is suspended pre-effect"
    );
    advance_to_main(&mut r);

    // [When Digivolving] installs the link selection over the Appmon in hand.
    assert!(
        fire_when_digivolving(&mut r, 0, reboot.index as usize),
        "[WD] self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    assert_eq!(
        r.game.player(0).battle_area[reboot.index as usize].linked_cards.len(),
        1,
        "the Appmon from hand linked onto Rebootmon"
    );

    // After linking, the unsuspend selection ("1 of your Digimon may unsuspend")
    // surfaces — choose the suspended ally specifically.
    assert!(
        r.game.pending_selection.is_some(),
        "unsuspend selection surfaces after the link"
    );
    let ally_action = digimon_engine::action::space::encode_attack(0, ally.index as u16);
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .contains(&ally_action),
        "the suspended ally is offered as an unsuspend target"
    );
    let _ = r.game.resolve_selection(0, ally_action);

    assert!(
        !r.game.player(0).battle_area[ally.index as usize].is_suspended,
        "the chosen own Digimon unsuspended"
    );
}

/// Negative: a NON-Appmon Digimon in hand is not a legal link candidate, so no
/// link occurs (and therefore no unsuspend follows).
#[test]
fn bt25_060_wd_does_not_link_non_appmon() {
    let mut r = base()
        .hand(0, &["NON-APPMON"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let _ = fire_when_digivolving(&mut r, 0, reboot.index as usize);
    resolve_all_first(&mut r, 0);
    assert_eq!(
        r.game.player(0).battle_area[reboot.index as usize].linked_cards.len(),
        0,
        "a non-Appmon card is never linked"
    );
}

/// Declining the (optional) link skips the unsuspend — faithful to DCGO's
/// `if (linked)` gate: the unsuspend only happens when a card was linked.
#[test]
fn bt25_060_wd_declining_link_skips_unsuspend() {
    let mut r = base()
        .hand(0, &["APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "OWN-ALLY", Some(1));
    r.game.suspend(ally);
    advance_to_main(&mut r);

    // WD surfaces the link selection (the Appmon in hand is a candidate).
    assert!(fire_when_digivolving(&mut r, 0, reboot.index as usize));
    assert!(
        r.game.pending_selection.as_ref().unwrap().is_optional,
        "the link selection is optional (declinable)"
    );
    // Decline the link (PASS).
    let _ = r.game.resolve_selection(0, digimon_engine::action::space::PASS);

    assert_eq!(
        r.game.player(0).battle_area[reboot.index as usize].linked_cards.len(),
        0,
        "no card linked after declining"
    );
    // No unsuspend prompt should follow, and the ally stays suspended.
    assert!(
        r.game.pending_selection.is_none(),
        "declining the link does NOT surface the unsuspend selection"
    );
    assert!(
        r.game.player(0).battle_area[ally.index as usize].is_suspended,
        "the ally stays suspended (no unsuspend when no link occurred)"
    );
}

/// OPT lockout is shared across [When Digivolving] and [When Attacking]:
/// firing WD consumes the once-per-turn so WA cannot re-fire the same turn.
#[test]
fn bt25_060_wd_wa_share_once_per_turn_lockout() {
    let mut r = base()
        .hand(0, &["APPMON-IN-HAND", "APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    // Fire WD and complete the link.
    assert!(
        fire_when_digivolving(&mut r, 0, reboot.index as usize),
        "[WD] installs the link selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    resolve_all_first(&mut r, 0);
    assert_eq!(
        r.game.player(0).battle_area[reboot.index as usize].linked_cards.len(),
        1,
        "WD linked 1 Appmon"
    );

    // WA must NOT re-offer the clause (shared once-per-turn lockout).
    let wa_surfaced = fire_when_attacking(&mut r, 0, reboot.index as usize);
    assert!(
        !wa_surfaced,
        "[WA] is locked out after [WD] fired the shared once-per-turn"
    );
    assert_eq!(
        r.game.player(0).battle_area[reboot.index as usize].linked_cards.len(),
        1,
        "no second link via WA after WD consumed the OPT"
    );

    // Lockout CLEARS after the turn cycle returns to the controller — the clause
    // can fire again and link the second Appmon still in hand.
    r.end_turn(); // opponent's turn
    r.end_turn(); // back to controller
    advance_to_main(&mut r);
    let reboot2 = r.perm_handle(0, reboot.index as usize);
    assert!(
        fire_when_digivolving(&mut r, 0, reboot2.index as usize),
        "[WD] re-offers the link next turn (OPT reset)"
    );
    let link2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link2);
    resolve_all_first(&mut r, 0);
    assert_eq!(
        r.game.player(0).battle_area[reboot2.index as usize].linked_cards.len(),
        2,
        "second Appmon linked next turn after the OPT reset"
    );
}

/// [All Turns] grant fires when a card gets linked TO Rebootmon (host-side):
/// it gains <Piercing> + <Blocker> + opponent-Digimon-effect immunity.
#[test]
fn bt25_060_grant_fires_on_link_to_host() {
    let mut r = base()
        .hand(0, &["APPMON-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    assert!(!r.game.has_keyword(reboot, Keyword::Piercing), "no Piercing pre-link");
    assert!(!r.game.has_keyword(reboot, Keyword::Blocker), "no Blocker pre-link");

    // Drive a real link onto Rebootmon via its own [WD] link step so OnLink
    // fires the host-side when_card_linked_to_this trigger.
    assert!(fire_when_digivolving(&mut r, 0, reboot.index as usize));
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    resolve_all_first(&mut r, 0);

    assert!(
        r.game.has_keyword(reboot, Keyword::Piercing),
        "Rebootmon gained <Piercing> when a card linked to it"
    );
    assert!(
        r.game.has_keyword(reboot, Keyword::Blocker),
        "Rebootmon gained <Blocker> when a card linked to it"
    );
    assert!(
        r.game.permanent_is_unaffected_by_effect(reboot, 1, EffectSourceKind::Digimon),
        "Rebootmon is immune to opponent's Digimon effects after the link"
    );
}

/// [All Turns] grant fires when Rebootmon UNSUSPENDS (the OR branch).
#[test]
fn bt25_060_grant_fires_on_unsuspend() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);
    r.game.suspend(reboot);
    assert!(!r.game.has_keyword(reboot, Keyword::Piercing), "no Piercing before unsuspend");

    r.game.unsuspend(reboot);
    resolve_all_first(&mut r, 0);

    assert!(
        r.game.has_keyword(reboot, Keyword::Piercing),
        "Rebootmon gained <Piercing> on unsuspend"
    );
    assert!(
        r.game.has_keyword(reboot, Keyword::Blocker),
        "Rebootmon gained <Blocker> on unsuspend"
    );
    assert!(
        r.game.permanent_is_unaffected_by_effect(reboot, 1, EffectSourceKind::Digimon),
        "Rebootmon immune to opponent Digimon effects on unsuspend"
    );
}

/// The granted Piercing/Blocker/immunity expire at the end of your turn.
#[test]
fn bt25_060_grant_expires_end_of_your_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);
    r.game.suspend(reboot);
    r.game.unsuspend(reboot);
    resolve_all_first(&mut r, 0);
    assert!(r.game.has_keyword(reboot, Keyword::Piercing), "Piercing present this turn");

    // End the turn → the until-your-turn-end grants must expire.
    r.end_turn();
    assert!(
        !r.game.has_keyword(reboot, Keyword::Piercing),
        "Piercing expired at end of your turn"
    );
    assert!(
        !r.game.has_keyword(reboot, Keyword::Blocker),
        "Blocker expired at end of your turn"
    );
    assert!(
        !r.game.permanent_is_unaffected_by_effect(reboot, 1, EffectSourceKind::Digimon),
        "effect-immunity expired at end of your turn"
    );
}

/// <Link +1> raises the max-link cap by 1 while Rebootmon is on the field.
#[test]
fn bt25_060_link_plus_one_raises_link_max() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).start();
    let reboot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);
    r.game.tick_declarative_effects();
    let delta = r.game.modifiers.link_max_delta(reboot);
    assert_eq!(delta, 1, "<Link +1> grants a +1 ChangeLinkMax delta while on field");
}

/// App Fusion behavioral: top Bootmon + linked Shutmon → Rebootmon's app-fusion
/// play is offered and resolves (stacks on top, consumes the link).
#[test]
fn bt25_060_app_fusion_stacks_bootmon_and_shutmon() {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::encode_digivolve;

    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    // Host: top = Bootmon, linked = Shutmon (two distinct named cards linked).
    let host = r.place_on_field(0, "BOOT", Some(0));
    let linked = r.push_linked_owned(host, "SHUT", 0);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "App Fusion offered as a legal digivolve onto Bootmon+Shutmon host"
    );

    r.game.decode_action(action, 0);
    let perm = &r.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        CARD_ID,
        "Rebootmon stacked on top of the host via App Fusion"
    );
    assert!(
        perm.card_sources.iter().any(|c| c.handle() == linked),
        "the linked Shutmon was consumed under the new top as a digivolution source"
    );
    assert!(perm.linked_cards.is_empty(), "consumed link removed from linked_cards");
}
