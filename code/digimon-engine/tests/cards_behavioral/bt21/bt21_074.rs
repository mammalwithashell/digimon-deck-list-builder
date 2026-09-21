//! BT21-074 Satellamon — Digimon (Appmon), Lv.5, Purple/Black, DP 7000, Cost 7.
//! Traits: Ult. / Appmon / GPS. Attribute: Navi.
//!
//! # Card text (data/card_bundles/BT21-074.md — official Bandai DB, confirmed
//! against the card image BT21-074.webp, which also prints the "Sup. 4"
//! digivolve circle the DB bundle omits)
//!
//! Digivolve: Purple Lv.4 / cost 4; Black Lv.4 / cost 4; [Sup.] trait / cost 4
//! (image + DCGO); and [Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3.
//!
//! [On Play] [When Digivolving] By placing 1 [Appmon] or [Three Musketeers]
//! trait card from your hand or trash as any of your Digimon's bottom
//! digivolution card, until your opponent's turn ends, their effects can't
//! return that Digimon to hands or decks or affect it with ＜De-Digivolve＞
//! effects. [When Digivolving] [When Attacking] [Once Per Turn] By trashing
//! 1 card with the [Appmon] or [Three Musketeers] trait from your Digimon's
//! digivolution cards, ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//! Special Rule: +4000 DP (link box)
//! Inherited: ＜Link＞ [Appmon] trait: Cost 3  [When Linking] Delete 1 of your
//! opponent's level 4 or lower Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_074.cs
//! - Alt digivolution: Level == 4 && HasText("Three Musketeers") cost 3;
//!   EqualsTraits("Sup.") cost 4. Link condition: HasAppmonTraits, cost 3.
//! - On Play / When Digivolving (`-1, false` = mandatory clause): hand-or-
//!   trash bool select, then SelectHandEffect / SelectCardEffect(Root.Trash)
//!   (canNoSelect: true) over EqualsTraits("Appmon") || HasThreeMusketeersTraits;
//!   then SelectPermanentEffect over own non-token Digimon (canNoSelect: true)
//!   → AddDigivolutionCardsBottom; then GainCanNotReturnToHand /
//!   GainCanNotReturnToDeck (IsOpponentEffect, UntilOpponentTurnEnd) +
//!   ImmuneFromDeDigivolveClass (UntilOpponentTurnEndEffects).
//! - When Digivolving / When Attacking (`1, true` = OPT + optional, shared
//!   hash "BT21_074De-digivolve"): SelectTrashDigivolutionCards over any own
//!   Digimon (canNoTrash: false) filtered Appmon-or-TM trait; if trashed →
//!   mandatory SelectPermanentEffect over opponent Digimon → IDegeneration(1).
//! - WhenLinked: mandatory Destroy select over opponent Digimon with level ≤ 4.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H alt paths (two circles + trait gate + in_text_contains special)
//! - A6 union-zone (hand|trash) place-as-bottom-source + opponent-scoped
//!   return/De-Digivolve protection modifiers with end_of_opponents_turn expiry
//! - E2 OPT optional [WD][WA] union-material trash cost → De-Digivolve 1
//! - DigiLink self link-condition + linked DP aura + when_linked delete

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};

const CARD_ID: &str = "BT21-074";

fn digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-074 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "DECK-PAD"))
        .add_card(digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(digimon("HOST-PLAIN", 4, 4000, 4, &["Beast"]))
        .add_card(digimon("APP-CARD", 3, 2000, 3, &["Appmon"]))
        .add_card(digimon("TM-CARD", 3, 2000, 3, &["Three Musketeers"]))
        .add_card(digimon("PLAIN-CARD", 3, 2000, 3, &["Beast"]))
        .add_card(digimon("OPP-BASE", 3, 2000, 3, &["Beast"]))
        .add_card(digimon("OPP-TOP", 4, 4000, 4, &["Beast"]))
        .add_card(digimon("OPP-LV4", 4, 4000, 4, &["Beast"]))
        .add_card(digimon("OPP-LV5", 5, 6000, 6, &["Beast"]))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn resolve_trigger_order_if_present(runner: &mut DebugRunner) {
    while matches!(runner.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        let act = runner.pending_selection().unwrap().valid_action_ids[0];
        runner.execute_action(0, act).expect("TriggerOrder");
    }
}

fn fire_link_onto_host(runner: &mut DebugRunner, host: PermanentHandle) {
    let linked = runner.push_linked_owned(host, CARD_ID, 0);
    for pid in 0..2u8 {
        runner.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: linked,
            },
        );
    }
    runner.game.drain_effect_queue();
}

fn has_protection(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner
        .game
        .modifiers
        .has(h, ModifierType::CannotBeReturnedToHand)
        && runner
            .game
            .modifiers
            .has(h, ModifierType::CannotBeReturnedToDeck)
        && runner
            .game
            .modifiers
            .has(h, ModifierType::CannotBeDeDigivolved)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt21_074_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT21-074 in embedded pack");
    assert_eq!(card.name, "Satellamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in ["Ult.", "Appmon", "GPS"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
}

#[test]
fn bt21_074_alt_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let circles = card
        .alt_paths
        .iter()
        .filter(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(4))
                && p.from
                    .as_ref()
                    .is_some_and(|f| f.level_eq == Some(4) && f.color_is.is_some())
        })
        .count();
    assert_eq!(circles, 2, "Purple Lv.4 / 4 and Black Lv.4 / 4");
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(4))
                && p.from
                    .as_ref()
                    .is_some_and(|f| f.trait_has.as_deref() == Some("Sup."))
        }),
        "[Sup.] trait / cost 4 (image + DCGO)"
    );
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(3))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(4)
                        && f.in_text_contains.as_deref() == Some("Three Musketeers")
                })
        }),
        "[Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3"
    );
}

#[test]
fn bt21_074_link_aura_and_clause_shapes() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 3
    )), "<Link> [Appmon] trait: Cost 3");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. })
                if *scope == CompiledScope::Linked && *dp_modifier == Some(4000)
        )),
        "link box +4000 DP aura"
    );
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let tuck = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] tuck clause");
    assert!(tuck.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(
        !tuck.optional,
        "DCGO isOptional: false (the picks themselves are declinable)"
    );
    let dd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("[When Digivolving][When Attacking] De-Digivolve clause");
    assert!(dd.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(dd.once_per_turn);
    assert!(dd.optional);
    let wl = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenLinked])
        .expect("[When Linking]");
    assert_eq!(wl.scope, CompiledScope::Linked);
}

// ─── Section 2 — [On Play][WD] tuck from hand/trash → protection ─────────────

#[test]
fn bt21_074_on_play_tucks_from_hand_and_protects_chosen_digimon_until_opponents_turn_ends() {
    let mut runner = base()
        .hand(0, &["APP-CARD", "PLAIN-CARD"])
        .memory(5)
        .start();
    let sat = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "HOST-PLAIN", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, sat);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        })
    );
    assert!(runner.pending_is_optional(), "DCGO canNoSelect: true");
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the [Appmon] card qualifies (PLAIN-CARD does not)"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick APP-CARD");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner
        .execute_action(0, encode_attack(0, host.index as u16))
        .expect("tuck under HOST-PLAIN");
    runner.auto_resolve().ok();

    let stack = &runner.game.players[0].battle_area[host.index as usize].card_sources;
    assert_eq!(stack.len(), 2);
    assert_eq!(
        stack[0].card_id(&runner.game.card_data),
        "APP-CARD",
        "bottom source"
    );
    assert_eq!(runner.hand_size(0), 1);
    assert!(
        has_protection(&runner, host),
        "return-to-hand/deck + De-Digivolve protection"
    );
    assert!(
        !has_protection(&runner, sat),
        "only THAT Digimon is protected"
    );

    runner.game.end_turn(); // P0 → P1
    runner.auto_resolve().ok();
    assert!(
        has_protection(&runner, host),
        "still protected during the opponent's turn"
    );
    runner.game.end_turn(); // P1 → P0
    runner.auto_resolve().ok();
    assert!(
        !has_protection(&runner, host),
        "expires when the opponent's turn ends"
    );
}

#[test]
fn bt21_074_on_play_tucks_from_trash() {
    let mut runner = base().memory(5).start();
    runner.inject_trash(0, "TM-CARD");
    runner.inject_trash(0, "PLAIN-CARD");
    let sat = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "HOST-PLAIN", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, sat);
    let view = runner.pending_selection_view().expect("union pick");
    let idx = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "TM-CARD")
        .unwrap();
    let pick = TRASH_EFFECT_START + idx as u16;
    assert!(view.valid_action_ids.contains(&pick));
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1
    );
    runner
        .execute_action(0, pick)
        .expect("pick TM-CARD from trash");
    runner
        .execute_action(0, encode_attack(0, host.index as u16))
        .expect("tuck");
    runner.auto_resolve().ok();

    let stack = &runner.game.players[0].battle_area[host.index as usize].card_sources;
    assert_eq!(stack[0].card_id(&runner.game.card_data), "TM-CARD");
    assert!(has_protection(&runner, host));
}

#[test]
fn bt21_074_on_play_decline_tucks_nothing_and_protects_nothing() {
    let mut runner = base().hand(0, &["APP-CARD"]).memory(5).start();
    let sat = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "HOST-PLAIN", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, sat);
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.hand_size(0), 1);
    assert!(!has_protection(&runner, host));
}

#[test]
fn bt21_074_on_play_no_eligible_card_no_prompt() {
    let mut runner = base().hand(0, &["PLAIN-CARD"]).memory(5).start();
    let sat = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "HOST-PLAIN", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, sat);
    assert!(
        runner.pending_selection().is_none(),
        "unexpected prompt: {:?}",
        runner.pending_selection_view()
    );
}

// ─── Section 3 — [WD][WA][OPT] trash Appmon/TM source → De-Digivolve 1 ──────

fn dd_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = base().memory(5).start();
    let sat = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    runner.push_source(host, "TM-CARD");
    runner.push_source(host, "PLAIN-CARD");
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    (runner, sat, opp)
}

#[test]
fn bt21_074_wa_trashes_source_then_de_digivolves() {
    let (mut runner, sat, opp) = dd_runner();
    fire(&mut runner, EffectTiming::WhenAttacking, sat);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::MATERIAL
        })
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only TM-CARD"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner.auto_resolve().ok();
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-BASE"
    );
}

#[test]
fn bt21_074_wa_decline_leaves_everything_untouched() {
    let (mut runner, sat, opp) = dd_runner();
    fire(&mut runner, EffectTiming::WhenAttacking, sat);
    runner.decline_optional_trigger().expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        2
    );
}

#[test]
fn bt21_074_wa_opt_locks_second_activation_same_turn() {
    let (mut runner, sat, opp) = dd_runner();
    runner.push_source(sat, "APP-CARD"); // second eligible source
    fire(&mut runner, EffectTiming::WhenAttacking, sat);
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("first activation");
    fire(&mut runner, EffectTiming::WhenAttacking, sat);
    assert!(runner.pending_selection().is_none(), "OPT lock");
    runner.game.end_turn();
    runner.auto_resolve().ok();
    runner.game.end_turn();
    runner.auto_resolve().ok();
    fire(&mut runner, EffectTiming::WhenAttacking, sat);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "lock cleared"
    );
}

/// On [When Digivolving] both clauses fire: the tuck clause AND the OPT
/// De-Digivolve clause share the timing (a TriggerOrder pick may surface).
#[test]
fn bt21_074_wd_fires_both_clauses() {
    let mut runner = base().hand(0, &["APP-CARD"]).memory(5).start();
    let sat = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    runner.push_source(host, "TM-CARD");
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    fire(&mut runner, EffectTiming::WhenDigivolving, sat);
    resolve_trigger_order_if_present(&mut runner);
    // Drive every prompt by accepting / taking the first legal action.
    while let Some(view) = runner.pending_selection_view() {
        let a = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(PASS);
        runner
            .execute_action(view.selecting_player, a)
            .expect("drive");
        resolve_trigger_order_if_present(&mut runner);
    }
    assert_eq!(runner.hand_size(0), 0, "APP-CARD was tucked");
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-BASE",
        "the opponent was De-Digivolved"
    );
}

// ─── Section 4 — Link box + [When Linking] ────────────────────────────────────

#[test]
fn bt21_074_linked_dp_bonus_4000_reaches_host() {
    let mut runner = base().memory(5).start();
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    let dp_before = runner.game.effective_dp(host).unwrap();
    runner.push_linked_owned(host, CARD_ID, 0);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.game.effective_dp(host), Some(dp_before + 4000));
}

#[test]
fn bt21_074_when_linked_deletes_opp_level_4_or_lower_only() {
    let mut runner = base().memory(5).start();
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));
    fire_link_onto_host(&mut runner, host);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1
    );
    runner.auto_resolve().ok();
    assert_eq!(runner.battle_area_size(1), 1);
    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .all(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-LV5"));
}
