//! P-170 AvengeKidmon — Digimon, Lv.6, Red/Purple, DP 13000, Cost 13.
//! Traits: Dragonkin (+ (Rule) Trait: Has [Three Musketeers]). Attribute: Virus.
//!
//! # Card text (data/card_bundles/P-170.md — official Bandai DB)
//!
//! Digivolve: Red Lv.5 / cost 5; Purple Lv.5 / cost 5; and
//! [Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4.
//!
//! When this card would be played, by returning 3 cards with [Three
//! Musketeers] in their texts from your trash to the bottom of the deck,
//! reduce the play cost by 6. ＜Raid＞＜Blocker＞＜Retaliation＞ [On Deletion]
//! You may play 1 [Three Musketeers] trait Digimon card with a play cost of
//! 12 or less from your hand or trash without paying the cost. (Rule) Trait:
//! Has [Three Musketeers].
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_170.cs
//! - Alternate Digivolution: TopCard.HasText("Three Musketeers") && Level 5, cost 4.
//! - BeforePayCost (CanActivate: ≥3 HasText("Three Musketeers") cards in
//!   trash): SelectCardEffect over trash, maxCount 3, canNoSelect only when
//!   the full cost is affordable (DCGO UI convenience — the reduction is a
//!   player choice per the printed "by returning"); on exactly 3 selected →
//!   AddLibraryBottomCards; ChangeCostClass(-6). The "Not Shown" static
//!   ChangeCostClass is DCGO's affordability-preview mirror of the same -6.
//! - Raid / Retaliation / Blocker: self static keyword effects.
//! - On Deletion (`-1, true` = optional): hand-or-trash bool selection, then
//!   SelectHandEffect / SelectCardEffect(Root.Trash) over
//!   CanPlayAsNewPermanent && EqualsTraits("Three Musketeers") && HasPlayCost
//!   && GetCostItself ≤ 12 → PlayPermanentCards(payCost: false).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H alt digivolution paths (standard circles + in_text_contains special)
//! - C when_playing_this cost_reduction with an interactive pay_cost (return 3 from trash)
//! - Declarative keyword grants (Raid / Blocker / Retaliation)
//! - E [On Deletion] optional union-zone (hand|trash) free play with a play-cost gate
//! - (Rule) Trait grant folded into `traits:`

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};

const CARD_ID: &str = "P-170";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, level: u8, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(6000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A card whose printed text contains "Three Musketeers" (in-text, not trait).
fn tm_text_card(id: &str) -> CardData {
    let mut c = digimon(id, 3, 3, &["Beast"]);
    c.effect_text = "[On Play] Three Musketeers shenanigans.".to_string();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-170 YAML parses and compiles")
        .add_card(filler("DECK-PAD"))
        .add_card(tm_text_card("TM-TEXT-1"))
        .add_card(tm_text_card("TM-TEXT-2"))
        .add_card(tm_text_card("TM-TEXT-3"))
        .add_card(digimon("TM-COST12", 6, 12, &["Three Musketeers"]))
        .add_card(digimon("TM-COST13", 6, 13, &["Three Musketeers"]))
        .add_card(digimon("TM-COST5", 4, 5, &["Three Musketeers"]))
        .add_card(digimon("PLAIN-COST5", 4, 5, &["Beast"]))
        .deck(0, &["DECK-PAD"; 5])
        .deck(1, &["DECK-PAD"; 5])
}

fn drive_accept(runner: &mut DebugRunner) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("execute");
    }
}

fn drive_decline(runner: &mut DebugRunner) {
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional || view.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        runner
            .execute_action(view.selecting_player, action)
            .expect("execute");
    }
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn p_170_yaml_has_printed_metadata_including_rule_trait() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("P-170 in embedded pack");
    assert_eq!(card.name, "AvengeKidmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
    assert_eq!(card.dp, Some(13000));
    assert!(card.traits.contains(&"Dragonkin".to_string()));
    assert!(
        card.traits.contains(&"Three Musketeers".to_string()),
        "(Rule) Trait: Has [Three Musketeers] folds into the trait list"
    );
}

#[test]
fn p_170_alt_paths_two_circles_cost5_and_tm_text_cost4() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let circles = card
        .alt_paths
        .iter()
        .filter(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(5))
                && p.from.as_ref().is_some_and(|f| {
                    f.all_of.iter().any(|q| q.level_eq == Some(5))
                        && f.all_of.iter().any(|q| q.color_is.is_some())
                })
        })
        .count();
    assert_eq!(circles, 2, "Red Lv.5 / 5 and Purple Lv.5 / 5");
    assert!(card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.all_of
                    .iter()
                    .any(|q| q.in_text_contains.as_deref() == Some("Three Musketeers"))
            })
    }));
}

#[test]
fn p_170_declares_keywords_cost_reducer_and_optional_on_deletion() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    for kw in ["Raid", "Blocker", "Retaliation"] {
        assert!(
            card.effects.iter().any(|c| matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                    if keyword == kw
            )),
            "missing <{kw}> grant"
        );
    }
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )),
        "when-played cost reducer"
    );
    let od = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnDeletion] => Some(t),
            _ => None,
        })
        .expect("[On Deletion] clause");
    assert!(od.optional, "'you may play'");
    assert_eq!(od.scope, CompiledScope::FaceUp);
}

// ─── Section 2 — When played: return 3 TM-text cards → cost -6 ──────────────

#[test]
fn p_170_accept_reducer_returns_three_tm_text_cards_and_pays_seven() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    for id in ["TM-TEXT-1", "TM-TEXT-2", "TM-TEXT-3"] {
        runner.inject_trash(0, id);
    }
    runner.inject_trash(0, "PLAIN-COST5");
    let mem_before = runner.memory();
    let deck_before = runner.deck_size(0);

    assert!(
        runner.play(0, 0).is_none(),
        "the interactive reducer parks the play"
    );
    assert!(
        runner.pending_selection().is_some(),
        "reducer offered with 3 TM-text cards in trash"
    );
    assert!(
        runner.pending_is_optional(),
        "the reduction is a player choice"
    );
    drive_accept(&mut runner);
    runner.auto_resolve().ok();

    assert_eq!(mem_before - runner.memory(), 7, "13 - 6 = 7 paid");
    assert_eq!(
        runner.deck_size(0),
        deck_before + 3,
        "3 cards returned to the deck"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data).starts_with("TM-TEXT")),
        "all three TM-text cards left the trash"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PLAIN-COST5"),
        "a card without [Three Musketeers] in its text is never a candidate"
    );
    assert_eq!(runner.battle_area_size(0), 1, "AvengeKidmon was played");
}

#[test]
fn p_170_decline_reducer_pays_full_thirteen() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    for id in ["TM-TEXT-1", "TM-TEXT-2", "TM-TEXT-3"] {
        runner.inject_trash(0, id);
    }
    let trash_before = runner.trash_size(0);
    let cp = runner.event_checkpoint();
    runner.play(0, 0);
    assert!(runner.pending_selection().is_some());
    drive_decline(&mut runner);
    runner.auto_resolve().ok();
    // Paying 13 from 10 memory crosses the gauge (10 → -3, which also ends
    // the turn), so read the payment off the event log rather than the
    // perspective-flipped gauge.
    let paid: Vec<i16> = runner
        .events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange { delta, .. } if *delta < 0 => Some(-*delta),
            _ => None,
        })
        .collect();
    assert_eq!(paid, vec![13], "declining pays the printed 13");
    assert_eq!(runner.trash_size(0), trash_before, "nothing returned");
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "AvengeKidmon was still played"
    );
}

#[test]
fn p_170_only_two_tm_text_cards_no_reducer_offered() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.inject_trash(0, "TM-TEXT-1");
    runner.inject_trash(0, "TM-TEXT-2");
    let cp = runner.event_checkpoint();
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "fewer than 3 [Three Musketeers]-text cards → the reducer is not offered"
    );
    runner.auto_resolve().ok();
    let paid: Vec<i16> = runner
        .events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange { delta, .. } if *delta < 0 => Some(-*delta),
            _ => None,
        })
        .collect();
    assert_eq!(paid, vec![13], "fewer than 3 → no reduction possible");
}

// ─── Section 3 — [On Deletion] play a TM Digimon (cost ≤ 12) from hand or trash ─

fn deletion_runner(hand: &[&str], trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = base().hand(0, hand).memory(3).start();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let avenge = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, avenge)
}

#[test]
fn p_170_on_deletion_plays_tm_digimon_free_from_trash() {
    let (mut runner, avenge) =
        deletion_runner(&["TM-COST12", "TM-COST13"], &["TM-COST5", "PLAIN-COST5"]);
    let mem_before = runner.memory();

    runner.game.delete_permanent_with_effects(avenge);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "optional gate"
    );
    runner.accept_optional_trigger().expect("accept");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        })
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "TM-COST12 (hand) + TM-COST5 (trash); cost-13 and non-TM cards excluded"
    );
    let trash_idx = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "TM-COST5")
        .unwrap();
    let pick = digimon_engine::action::space::TRASH_EFFECT_START + trash_idx as u16;
    assert!(view.valid_action_ids.contains(&pick));
    runner.execute_action(0, pick).expect("play from trash");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TM-COST5"),
        "played to the battle area"
    );
    assert_eq!(runner.memory(), mem_before, "without paying the cost");
}

#[test]
fn p_170_on_deletion_plays_tm_digimon_free_from_hand() {
    let (mut runner, avenge) = deletion_runner(&["TM-COST12"], &[]);
    runner.game.delete_permanent_with_effects(avenge);
    runner.accept_optional_trigger().expect("accept");
    let pick = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, pick).expect("play from hand");
    runner.auto_resolve().ok();
    assert_eq!(runner.hand_size(0), 0);
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "TM-COST12"));
}

#[test]
fn p_170_on_deletion_decline_plays_nothing() {
    let (mut runner, avenge) = deletion_runner(&["TM-COST12"], &["TM-COST5"]);
    runner.game.delete_permanent_with_effects(avenge);
    runner.decline_optional_trigger().expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(0), 0);
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn p_170_on_deletion_no_eligible_card_no_prompt() {
    let (mut runner, avenge) = deletion_runner(&["TM-COST13"], &["PLAIN-COST5"]);
    runner.game.delete_permanent_with_effects(avenge);
    assert!(
        runner.pending_selection().is_none(),
        "cost-13 TM Digimon and a non-TM Digimon are not eligible"
    );
}
