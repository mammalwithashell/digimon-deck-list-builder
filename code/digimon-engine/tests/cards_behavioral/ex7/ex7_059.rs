//! EX7-059 BeelStarmon ACE — Digimon, Lv.6, Purple, DP 11000, Cost 6.
//! Traits: Wizard / Three Musketeers. Attribute: Virus. ACE (Overflow -4).
//!
//! # Card text (data/card_bundles/EX7-059.md — official Bandai DB, confirmed
//! against the card image EX7-059.webp)
//!
//! Digivolve: Purple Lv.5 / cost 3 (standard circle) and
//! [Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 3.
//!
//! [Hand] [Counter] ＜Blast Digivolve＞
//! [On Play] [When Digivolving] Return 1 Option card from your trash to the
//! hand. Then, you may use 1 [Three Musketeers] trait Option card from your
//! hand without paying the cost.
//! [When Attacking] [Once Per Turn] By trashing 1 Option card from this
//! Digimon's digivolution cards, you may use 1 [Three Musketeers] trait
//! Option card from your hand without paying the cost.
//! ＜Overflow (-4)＞
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_059.cs
//! - Digivolution Condition: TopCard.HasText("Three Musketeers"), level 5, cost 3.
//! - Blast Digivolve: BlastDigivolveEffect at OnCounterTiming.
//! - On Play / When Digivolving (`-1, false`): mandatory SelectCardEffect
//!   (Mode.AddHand, Root.Trash, canNoSelect: false) over IsOption; then
//!   SelectHandEffect (canNoSelect: true) over IsOption && TM trait &&
//!   HasUseCost && !CanNotPlayThisOption → PlayOptionCards(payCost: false).
//!   The hand-use half runs even when the trash half found nothing.
//! - When Attacking (`1, true` = OPT + optional): SelectCardEffect over THIS
//!   permanent's DigivolutionCards filtered IsOption, canNoSelect: true →
//!   ITrashDigivolutionCards; only if trashed: the same optional hand use.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H alt digivolution path (level_eq + in_text_contains)
//! - Blast Digivolve marker (grant_keyword + burst_digivolve alt-path) + ACE Overflow
//! - A3 return-Option-from-trash (mandatory) then optional free use from hand
//! - E2 OPT + optional cost (trash own Option source) → optional free use

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "EX7-059";

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

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 5;
    card.colors = vec![CardColor::Purple];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn make_option(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.colors = vec![CardColor::Purple];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-059 YAML parses and compiles")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_option("TM-OPT", 6, &["Three Musketeers"]))
        .add_card(make_option("PLAIN-OPT", 4, &[]))
        .add_card(make_digimon("PURPLE-LV5", 5, 7000, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", 5, 6000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ex7_059_yaml_has_printed_metadata_and_ace_overflow() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX7-059 in embedded pack");
    assert_eq!(card.name, "BeelStarmon ACE");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.ace_overflow, Some(-4), "＜Overflow (-4)＞");
    for t in ["Wizard", "Three Musketeers"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
}

#[test]
fn ex7_059_has_blast_digivolve_marker_and_alt_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "BlastDigivolve"
        )),
        "[Hand][Counter] <Blast Digivolve> grant_keyword"
    );
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::BurstDigivolve),
        "Blast Digivolve registers the burst_digivolve marker alt-path"
    );
    let special = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from.as_ref().is_some_and(|f| {
                f.all_of
                    .iter()
                    .any(|q| q.in_text_contains.as_deref() == Some("Three Musketeers"))
                    && f.all_of.iter().any(|q| q.level_eq == Some(5))
            })
    });
    assert!(
        special,
        "[Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 3"
    );
}

#[test]
fn ex7_059_clause_shapes() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let op = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving]");
    assert!(op.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!op.optional, "return-1-Option is mandatory");
    assert!(!op.once_per_turn);
    let wa = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenAttacking])
        .expect("[When Attacking]");
    assert!(wa.optional, "DCGO isOptional: true");
    assert!(wa.once_per_turn, "printed [Once Per Turn]");
}

// ─── Section 2 — [On Play][When Digivolving] ─────────────────────────────────

/// Positive: mandatory trash pick returns an Option to hand; then the TM
/// Option may be used free (its body draws 1).
#[test]
fn ex7_059_op_returns_option_from_trash_then_uses_tm_option_free() {
    let mut runner = base().memory(3).start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    runner.inject_trash(0, "TM-OPT");
    runner.inject_trash(0, "PLAIN-OPT");
    runner.inject_trash(0, "DECK-PAD"); // not an Option → never a candidate
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);

    fire(&mut runner, EffectTiming::OnPlay, beel);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    assert!(
        !runner.pending_is_optional(),
        "'Return 1 Option card' is mandatory"
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "both Options (and only Options) are candidates"
    );
    let tm_idx = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "TM-OPT")
        .unwrap();
    let chosen = digimon_engine::action::space::TRASH_EFFECT_START + tm_idx as u16;
    assert!(
        view.valid_action_ids.contains(&chosen),
        "TM-OPT (trash index {tm_idx}) must be encoded as a valid trash pick"
    );
    runner
        .execute_action(0, chosen)
        .expect("return TM-OPT to hand");
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the Option returned to hand"
    );

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "then: may use a [Three Musketeers] Option from hand"
    );
    assert!(runner.pending_is_optional());
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).expect("use it free");
    runner.auto_resolve().ok();

    assert_eq!(runner.memory(), mem_before, "used without paying the cost");
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the Option left the hand (used) and its body drew 1"
    );
}

/// Negative (decline the use): the returned Option stays in hand.
#[test]
fn ex7_059_op_declining_use_keeps_option_in_hand() {
    let mut runner = base().memory(3).start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    runner.inject_trash(0, "TM-OPT");
    let beel = runner.place_on_field(0, CARD_ID, Some(0));

    fire(&mut runner, EffectTiming::OnPlay, beel);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).expect("return");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner.execute_action(0, PASS).expect("decline");

    assert!(runner.pending_selection().is_none());
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"));
}

/// Empty trash but a TM Option already in hand: the trash half self-skips
/// and the hand-use half is still offered (DCGO runs both halves
/// independently).
#[test]
fn ex7_059_op_empty_trash_still_offers_hand_use() {
    let mut runner = base().hand(0, &["TM-OPT"]).memory(3).start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    let beel = runner.place_on_field(0, CARD_ID, Some(0));

    fire(&mut runner, EffectTiming::OnPlay, beel);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "no Option in trash → straight to the optional hand use"
    );
}

/// Negative: nothing in trash and no TM Option in hand → no prompt at all.
#[test]
fn ex7_059_op_nothing_eligible_no_prompt() {
    let mut runner = base().hand(0, &["PLAIN-OPT"]).memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::OnPlay, beel);
    assert!(runner.pending_selection().is_none());
}

// ─── Section 3 — [When Attacking][OPT] trash Option source → free use ────────

fn wa_runner() -> (DebugRunner, PermanentHandle) {
    let mut runner = base().hand(0, &["TM-OPT"]).memory(3).start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(beel, "PLAIN-OPT");
    (runner, beel)
}

#[test]
fn ex7_059_wa_trashes_option_source_then_uses_tm_option_free() {
    let (mut runner, beel) = wa_runner();
    let mem_before = runner.memory();
    let cp = runner.event_checkpoint();

    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    // The optional clause's first step is itself an optional (declinable)
    // cost pick, so the engine offers it directly — no separate outer gate.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::MATERIAL
        }),
        "cost pick over THIS Digimon's digivolution cards"
    );
    assert!(
        runner.pending_is_optional(),
        "DCGO canNoSelect: true — the activation is declinable here"
    );
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, act)
        .expect("trash the Option source");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "view = {:?}",
        runner.pending_selection_view()
    );
    assert!(runner.pending_is_optional());
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).expect("use TM-OPT");
    runner.auto_resolve().ok();

    assert_eq!(runner.memory(), mem_before);
    let stack = &runner.game.players[0].battle_area[beel.index as usize].card_sources;
    assert_eq!(stack.len(), 1, "Option source trashed as the cost");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PLAIN-OPT"),
        "the trashed Option source lands in its owner's trash"
    );
    let _ = cp;
    assert_eq!(
        runner.hand_size(0),
        1,
        "TM-OPT left the hand; its body drew 1"
    );
}

/// Negative (decline the cost pick): nothing is trashed and no use is offered.
#[test]
fn ex7_059_wa_declining_cost_trashes_nothing_and_offers_no_use() {
    let (mut runner, beel) = wa_runner();
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::MATERIAL
        })
    );
    assert!(
        runner.pending_is_optional(),
        "DCGO canNoSelect: true on the trash pick"
    );
    runner.execute_action(0, PASS).expect("decline the cost");

    assert!(
        runner.pending_selection().is_none(),
        "no use without paying the cost"
    );
    let stack = &runner.game.players[0].battle_area[beel.index as usize].card_sources;
    assert_eq!(stack.len(), 2);
}

/// Negative (no Option source): nothing is offered.
#[test]
fn ex7_059_wa_no_option_source_no_prompt() {
    let mut runner = base().hand(0, &["TM-OPT"]).memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(beel, "PURPLE-LV5");
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex7_059_wa_opt_locks_second_activation_same_turn_and_clears_next_turn() {
    let (mut runner, beel) = wa_runner();
    runner.push_source(beel, "PLAIN-OPT");
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    runner.auto_resolve().expect("first activation");

    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert!(runner.pending_selection().is_none(), "OPT lock");

    runner.game.end_turn();
    runner.auto_resolve().ok();
    runner.game.end_turn();
    runner.auto_resolve().ok();
    fire(&mut runner, EffectTiming::WhenAttacking, beel);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::MATERIAL
        }),
        "lock cleared"
    );
}

// ─── Section 4 — Blast Digivolve + Overflow ──────────────────────────────────

#[test]
fn ex7_059_blast_digivolve_offered_in_counter_window() {
    let mut runner = base().hand(1, &[CARD_ID]).start();
    let attacker = runner.place_on_field(0, "OPP-DIGI", Some(0));
    let defender = runner.place_on_field(1, "PURPLE-LV5", Some(0));

    let result = runner.attack_digimon(attacker, defender, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);
    let prompt = runner.pending_selection().expect("counter window");
    assert_eq!(prompt.selecting_player, 1);
    assert!(
        prompt.valid_action_ids.contains(&encode_digivolve(0, 0)),
        "EX7-059 in hand is offered as a Blast Digivolve onto the purple Lv.5"
    );
}

#[test]
fn ex7_059_overflow_minus_4_fires_on_leave_field() {
    let mut runner = base().memory(3).start();
    let beel = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.game.memory;
    runner
        .game
        .delete_permanent_with_cause(beel, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();
    assert_eq!(runner.game.memory, mem_before - 4, "＜Overflow (-4)＞");
}
