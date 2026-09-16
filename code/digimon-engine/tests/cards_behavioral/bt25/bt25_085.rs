//! BT25-085 BeelStarmon / Fly Bullet — DUAL (Digimon Lv.6 Purple/Black DP
//! 12000 · Option use cost 6). Traits: Wizard / Three Musketeers / Iliad / TS.
//!
//! # Card text (data/card_bundles/BT25-085.md — official Bandai DB, confirmed
//! against the card image BT25-085.webp)
//!
//! Digivolve: Purple Lv.5 / cost 4; Black Lv.5 / cost 4; and
//! [Digivolve] Lv.5 w/[Three Musketeers] in text or w/[TS] trait: Cost 3.
//! "Can't play to the field" (DUAL — the Digimon face is only reached by
//! digivolving / Arts Digivolve).
//!
//! ＜Blocker＞ [When Digivolving] [When Attacking] [Once Per Turn] You may use
//! 1 [Three Musketeers] or [TS] trait Option card from your hand or this
//! Digimon's digivolution cards without paying the cost. [When Digivolving]
//! [When Attacking] [Counter] [Once Per Turn] By trashing 1 Option card from
//! any of your Digimon's digivolution cards or link cards, this Digimon
//! unsuspends.
//!
//! DUAL Effect: ＜Use Req. ([Three Musketeers] in text)＞ [Main] Delete 1 of
//! your opponent's highest level Digimon. Then, you may place 1 [Three
//! Musketeers] trait card from your hand or trash as any of your Digimon's
//! bottom digivolution card.  DUAL Rule: ＜Arts Digivolve＞
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_085.cs
//! - Alt Digivolution: HasText("Three Musketeers") || EqualsTraits("TS"),
//!   cost 3, level 5, ignoreDigivolutionRequirement: TRUE.
//! - Blocker: BlockerSelfStaticEffect.
//! - Shared WD/WA use-Option (maxCountPerTurn 1, isSkippable, hash
//!   "BT25_085_WD_WA"): SetIntSelection "Use from Hand / Use from
//!   Digivolution Cards / Don't use" → SelectHandEffect or SelectCardEffect
//!   (Root.DigivolutionCards, THIS permanent) over IsOption && (TM || TS
//!   trait) && !CanNotPlayThisOption → PlayOptionCards(payCost: false);
//!   RemoveUse when nothing was used.
//! - Shared WD/WA/Counter unsuspend (hash "BT25_085_WD_WA_Counter"):
//!   SelectPermanentEffect over own Digimon whose DigivolutionOrLinkCards
//!   contain an Option → SelectCardEffect(Mode.Discard) over that union →
//!   IUnsuspendPermanents(this); RemoveUse when nothing was trashed.
//! - Option face: UseRequirements(HasText("Three Musketeers")); OptionSkill:
//!   mandatory Destroy select over IsMaxLevel opponent Digimon; then (if you
//!   have a Digimon) "From hand / From trash / Do not place" → pick a
//!   [Three Musketeers]-trait card → pick any own Digimon →
//!   AddDigivolutionCardsBottom. ArtsDigivolveEffect.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - DUAL card (digimon + option faces) + Arts Digivolve + Use Req.
//! - E2 OPT optional union-zone (hand|material, material_of: source) Option use
//!   (G-DSL-USE-OPTION-FROM-SOURCES via use_option_bound Material origin)
//! - E2 OPT optional trash-Option-from-own-stacks cost → unsuspend self,
//!   on [WD][WA][Counter]
//! - [Main] highest-level delete + optional union-zone place-as-bottom-source

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::OptionPlayResult;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "BT25-085";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

struct OptionMainDraw;

impl CardEffect for OptionMainDraw {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain draw 1")
            .process(|ctx: &mut EffectContext| {
                let owner = ctx.player;
                ctx.draw(owner, 1);
            })
            .build()]
    }
}

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

fn option(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-085 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "DECK-PAD"))
        .add_card(option("TM-OPT", 6, &["Three Musketeers"]))
        .add_card(option("TS-OPT", 5, &["TS"]))
        .add_card(option("PLAIN-OPT", 4, &[]))
        .add_card(digimon("TM-DIGI", 5, 7000, 7, &["Three Musketeers"]))
        .add_card(digimon("PLAIN-DIGI", 4, 5000, 5, &["Beast"]))
        .add_card(digimon("TM-CARD", 3, 2000, 3, &["Three Musketeers"]))
        .add_card(digimon("OPP-LV4", 4, 4000, 4, &["Beast"]))
        .add_card(digimon("OPP-LV6", 6, 11000, 11, &["Beast"]))
        .deck(0, &["DECK-PAD"; 8])
        .deck(1, &["DECK-PAD"; 8])
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

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_085_dual_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-085 in embedded pack");
    assert_eq!(card.name, "BeelStarmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    for t in ["Wizard", "Three Musketeers", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
    let dual = card.dual.as_ref().expect("dual block");
    assert_eq!(dual.digimon.level, 6);
    assert_eq!(dual.option.use_cost, 6);
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "DUAL Rule <Arts Digivolve>"
    );
    assert!(
        dual.option.use_requirement.is_some(),
        "<Use Req. ([Three Musketeers] in text)>"
    );
}

#[test]
fn bt25_085_alt_paths() {
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
                    .is_some_and(|f| f.level_eq == Some(5) && f.color_is.is_some())
        })
        .count();
    assert_eq!(circles, 2, "Purple Lv.5 / 4 and Black Lv.5 / 4");
    let special = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        })
        .expect("[Digivolve] Lv.5 w/[Three Musketeers] in text or w/[TS] trait: Cost 3");
    let from = special.from.as_ref().unwrap();
    assert_eq!(from.level_eq, Some(5));
    assert!(from
        .any_of
        .iter()
        .any(|q| q.in_text_contains.as_deref() == Some("Three Musketeers")));
    assert!(from
        .any_of
        .iter()
        .any(|q| q.trait_has.as_deref() == Some("TS")));
    assert!(
        special.ignore_requirements,
        "DCGO ignoreDigivolutionRequirement: true"
    );
}

#[test]
fn bt25_085_digimon_face_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let dual = card.dual.as_ref().expect("dual");
    assert!(
        dual.digimon.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Blocker"
        )),
        "<Blocker>"
    );
    let triggered: Vec<_> = dual
        .digimon
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let use_clause = triggered
        .iter()
        .find(|t| !t.when.contains(&CompiledTiming::Counter))
        .expect("[WD][WA][OPT] use-Option clause");
    assert!(use_clause.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(use_clause.when.contains(&CompiledTiming::WhenAttacking));
    assert!(use_clause.once_per_turn);
    assert!(
        !use_clause.optional,
        "DCGO optional: false — the union pick's PASS is the 'Don't use' branch"
    );
    let unsus = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::Counter))
        .expect("[WD][WA][Counter][OPT] unsuspend clause");
    assert!(unsus.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(unsus.when.contains(&CompiledTiming::WhenAttacking));
    assert!(unsus.once_per_turn);
    assert!(
        unsus.optional,
        "DCGO isSkippable + RemoveUse → clause-level optional gate"
    );
    assert_eq!(dual.option.effects.len(), 1, "Option face [Main]");
}

// ─── Section 2 — [WD][WA][OPT] use a TM/TS Option from hand or own sources ───

/// `with_stack_options`: also seat a [TS] Option + a plain Option under
/// BeelStarmon (the "this Digimon's digivolution cards" origin). Hand always
/// holds TM-OPT + PLAIN-OPT.
fn use_runner(with_stack_options: bool) -> (DebugRunner, PermanentHandle) {
    let mut runner = base().hand(0, &["TM-OPT", "PLAIN-OPT"]).memory(3).start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    runner.register_effect("TS-OPT", Arc::new(OptionMainDraw));
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    if with_stack_options {
        runner.push_source(beel, "TS-OPT");
        runner.push_source(beel, "PLAIN-OPT");
    } else {
        runner.push_source(beel, "PLAIN-DIGI");
    }
    (runner, beel)
}

/// Drive to the use-Option clause's union pick when WA fires (only the
/// use clause is live: no Option elsewhere → the unsuspend clause cannot
/// activate, so no TriggerOrder competes). DCGO `optional: false` → no outer
/// yes/no gate; the union pick itself is the (declinable) first prompt.
fn open_use_pick(runner: &mut DebugRunner, beel: PermanentHandle) {
    fire(runner, EffectTiming::WhenAttacking, beel);
    resolve_trigger_order_if_present(runner);
}

#[test]
fn bt25_085_use_tm_option_from_hand_free() {
    let (mut runner, beel) = use_runner(false);
    let mem_before = runner.memory();
    open_use_pick(&mut runner, beel);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::MATERIAL
        }),
        "view = {:?}",
        runner.pending_selection_view()
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "TM-OPT in hand; PLAIN-OPT is not a candidate"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("use from hand");
    runner.auto_resolve().ok();

    assert_eq!(runner.memory(), mem_before, "without paying the cost");
    assert_eq!(
        runner.hand_size(0),
        2,
        "TM-OPT left the hand; its body drew 1 (PLAIN-OPT + drawn)"
    );
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"));
}

#[test]
fn bt25_085_use_ts_option_from_own_digivolution_cards_free() {
    let (mut runner, beel) = use_runner(true);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    resolve_trigger_order_if_present(&mut runner);
    // Two clauses are live (TS-OPT in the stack also satisfies the unsuspend
    // cost). Accept gates until the hand|material union pick appears.
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::UnionZone { zones })
                if zones == UnionZoneSet::HAND | UnionZoneSet::MATERIAL =>
            {
                break
            }
            Some(SelectionKind::TriggerOrder) => resolve_trigger_order_if_present(&mut runner),
            // The unsuspend clause's own picks (which Digimon / which Option)
            // may come first — decline them so the use clause is isolated.
            Some(_) => {
                runner
                    .execute_action(0, PASS)
                    .expect("decline the unsuspend pick");
            }
            None => panic!("the use-Option union pick never installed"),
        }
    }
    let view = runner.pending_selection_view().unwrap();
    let candidates: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        candidates.len(),
        2,
        "TM-OPT (hand) + TS-OPT (stack); PLAIN-OPTs excluded"
    );
    let material_pick = candidates
        .iter()
        .copied()
        .find(|&a| a >= digimon_engine::action::space::SOURCE_SELECT_START)
        .expect("the stack Option is encoded as a source-select action");
    runner
        .execute_action(0, material_pick)
        .expect("use from digivolution cards");
    runner.auto_resolve().ok();

    assert_eq!(runner.memory(), mem_before);
    let stack = &runner.game.players[0].battle_area[beel.index as usize].card_sources;
    assert!(
        !stack
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-OPT"),
        "TS-OPT left the digivolution cards (used)"
    );
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TS-OPT"));
    assert_eq!(runner.hand_size(0), hand_before + 1, "its body drew 1");
}

#[test]
fn bt25_085_use_decline_uses_nothing() {
    let (mut runner, beel) = use_runner(false);
    open_use_pick(&mut runner, beel);
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.hand_size(0), 2);
}

#[test]
fn bt25_085_use_no_candidate_no_prompt() {
    let mut runner = base().hand(0, &["PLAIN-OPT"]).memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(beel, "PLAIN-DIGI");
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(
        runner.pending_selection().is_none(),
        "unexpected prompt: {:?}",
        runner.pending_selection_view()
    );
}

#[test]
fn bt25_085_use_opt_locks_second_activation_same_turn() {
    let (mut runner, beel) = use_runner(false);
    // Two TM Options in hand so a second activation would have a candidate.
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TM-OPT")
        .unwrap();
    let ci = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(idx, 0, ci));

    open_use_pick(&mut runner, beel);
    let a = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, a).expect("use");
    runner.auto_resolve().ok();

    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(runner.pending_selection().is_none(), "OPT lock");
    runner.game.end_turn();
    runner.auto_resolve().ok();
    runner.game.end_turn();
    runner.auto_resolve().ok();
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
        "lock cleared"
    );
}

/// DCGO `RemoveUse`: declining ("Don't use an Option") does NOT consume the
/// [Once Per Turn] — the effect can still be used later this turn.
#[test]
fn bt25_085_use_decline_does_not_consume_opt() {
    let (mut runner, beel) = use_runner(false);
    open_use_pick(&mut runner, beel);
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
        "declining must not spend the OPT (DCGO activateClass.RemoveUse)"
    );
}

// ─── Section 3 — [WD][WA][Counter][OPT] trash Option from any stack → unsuspend ─

fn unsuspend_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = base().memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    runner.push_source(other, "PLAIN-OPT");
    runner.game.players[0].battle_area[beel.index as usize].is_suspended = true;
    (runner, beel, other)
}

#[test]
fn bt25_085_wa_trash_option_from_another_digimons_stack_unsuspends_self() {
    let (mut runner, beel, other) = unsuspend_runner();
    assert!(is_suspended(&runner, beel));
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    resolve_trigger_order_if_present(&mut runner);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "outer gate"
    );
    runner.accept_optional_trigger().expect("accept");
    // Which Digimon → which Option (mandatory once accepted).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let view = runner.pending_selection_view().unwrap();
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, other.index as u16)));
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, beel.index as u16)),
        "BeelStarmon has no Option in its own stack"
    );
    runner
        .execute_action(0, encode_attack(0, other.index as u16))
        .expect("pick the Digimon");
    let a = runner
        .pending_selection()
        .expect("which Option")
        .valid_action_ids[0];
    runner.execute_action(0, a).expect("trash it");
    runner.auto_resolve().ok();

    assert!(!is_suspended(&runner, beel), "this Digimon unsuspends");
    assert_eq!(
        runner.game.players[0].battle_area[other.index as usize]
            .card_sources
            .len(),
        1,
        "the Option was trashed as the cost"
    );
}

#[test]
fn bt25_085_counter_timing_also_unsuspends() {
    let (mut runner, beel, other) = unsuspend_runner();
    fire(&mut runner, EffectTiming::CounterEffect, beel);
    resolve_trigger_order_if_present(&mut runner);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "[Counter] timing"
    );
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner.auto_resolve().ok();
    assert!(!is_suspended(&runner, beel));
}

#[test]
fn bt25_085_trash_option_link_card_unsuspends() {
    let mut runner = base().memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    runner.push_linked_owned(other, "PLAIN-OPT", 0);
    runner.game.players[0].battle_area[beel.index as usize].is_suspended = true;
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    resolve_trigger_order_if_present(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner.auto_resolve().ok();
    assert!(
        !is_suspended(&runner, beel),
        "an Option LINK card is a legal cost too"
    );
    assert!(runner.game.players[0].battle_area[other.index as usize]
        .linked_cards
        .is_empty());
}

#[test]
fn bt25_085_unsuspend_decline_stays_suspended() {
    let (mut runner, beel, other) = unsuspend_runner();
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    resolve_trigger_order_if_present(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    runner
        .decline_optional_trigger()
        .expect("decline at the gate");
    assert!(runner.pending_selection().is_none());
    assert!(is_suspended(&runner, beel));
    // DCGO RemoveUse: declining does not spend the OPT.
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    resolve_trigger_order_if_present(&mut runner);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "declining the gate must not spend the [Once Per Turn]"
    );
    assert_eq!(
        runner.game.players[0].battle_area[other.index as usize]
            .card_sources
            .len(),
        2
    );
}

#[test]
fn bt25_085_unsuspend_no_option_anywhere_no_prompt() {
    let mut runner = base().memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    runner.push_source(other, "TM-CARD");
    runner.game.players[0].battle_area[beel.index as usize].is_suspended = true;
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(runner.pending_selection().is_none());
}

// ─── Section 4 — Option face: [Main] + Arts Digivolve ────────────────────────

#[test]
fn bt25_085_option_main_deletes_highest_level_then_may_place_tm_card_under_own_digimon() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-CARD"]).memory(8).start();
    let tm = runner.place_on_field(0, "TM-DIGI", Some(0)); // satisfies Use Req.
    let lv4 = runner.place_on_field(1, "OPP-LV4", Some(0));
    let lv6 = runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();
    let mem_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner.pending_selection_view().unwrap();
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, lv6.index as u16)));
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, lv4.index as u16)),
        "only the highest-level Digimon is a candidate"
    );
    runner
        .execute_action(0, encode_attack(0, lv6.index as u16))
        .expect("delete");
    assert_eq!(runner.battle_area_size(1), 1);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        }),
        "then: may place a [Three Musketeers] card from hand or trash"
    );
    assert!(runner.pending_is_optional());
    let pick = runner.pending_selection().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("pick TM-CARD from hand");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner
        .execute_action(0, encode_attack(0, tm.index as u16))
        .expect("place under TM-DIGI");
    runner.auto_resolve().ok();

    let stack = &runner.game.players[0].battle_area[tm.index as usize].card_sources;
    assert_eq!(
        stack[0].card_id(&runner.game.card_data),
        "TM-CARD",
        "bottom source"
    );
    assert_eq!(mem_before - runner.memory(), 6, "printed use cost 6 paid");
}

#[test]
fn bt25_085_option_main_declining_place_leaves_hand_untouched() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-CARD"]).memory(8).start();
    runner.place_on_field(0, "TM-DIGI", Some(0));
    runner.place_on_field(1, "OPP-LV6", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let a = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, a).expect("delete");
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    runner.execute_action(0, PASS).expect("decline the place");
    // Decline the trailing (optional) Arts Digivolve prompt as well —
    // accepting it would digivolve TM-DIGI and draw a card.
    while let Some(view) = runner.pending_selection_view() {
        let a = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        runner
            .execute_action(view.selecting_player, a)
            .expect("drive");
    }
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        runner.hand_size(0),
        1,
        "TM-CARD stays in hand; hand = {hand:?}"
    );
}

#[test]
fn bt25_085_arts_digivolve_stacks_onto_a_tm_lv5_base() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(8).start();
    let base_perm = runner.place_on_field(0, "TM-DIGI", Some(0)); // Lv.5 [Three Musketeers]
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    loop {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("selection parked before Arts");
        let is_arts = sel.kind == SelectionKind::OwnField
            && sel.is_optional
            && sel
                .valid_action_ids
                .contains(&encode_attack(0, base_perm.index as u16));
        if is_arts {
            break;
        }
        let action = if sel.is_optional {
            PASS
        } else {
            sel.valid_action_ids[0]
        };
        runner.execute_action(0, action).expect("drive to Arts");
    }
    let trash_before = runner.trash_size(0);
    runner
        .execute_action(0, encode_attack(0, base_perm.index as u16))
        .expect("accept Arts");
    let perm = &runner.game.player(0).battle_area[base_perm.index as usize];
    assert_eq!(perm.stack_size(), 2, "Arts stacked the DUAL onto the base");
    assert_eq!(perm.top_card().card_id(&runner.game.card_data), CARD_ID);
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "Arts prevented the normal Option trash"
    );
}
