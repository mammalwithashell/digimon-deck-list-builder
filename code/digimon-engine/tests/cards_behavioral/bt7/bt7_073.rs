//! BT7-073 KaiserLeomon — Digimon, Lv.4, Purple, DP 6000, Cost 6.
//! Traits: Hybrid / Variable / Cyborg.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT7-073.md`;
//! card image confirms)
//!
//! You may digivolve this card from your hand onto one of your purple Tamers
//! as if the Tamer is a level 3 purple Digimon for a memory cost of 2.
//! [When Digivolving] If a card with [Hybrid] in its traits or [Koichi Kimura]
//! is in this Digimon's digivolution cards, this Digimon gains ＜Retaliation＞
//! until the end of your opponent's next turn. (When this Digimon is deleted
//! after losing a battle, delete the Digimon it was battling.)
//!
//! Digivolve: Purple Lv.3 / cost 3; Purple Lv.4 / cost 1 (card image +
//! official DB; `card_overrides.json` carries both rows).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT7/Purple/BT7_073.cs
//! - EffectTiming.None → `AddSelfDigivolutionRequirementStaticEffect`
//!   (TopCard Purple && IsTamer, digivolutionCost 2, ignore false).
//! - OnEnterFieldAnyone / CanTriggerWhenDigivolving → NOT optional
//!   (`SetUpActivateClass(..., -1, false, ...)`); `CanActivateCondition` =
//!   ≥1 digivolution card with trait "Hybrid" OR name "Koichi Kimura";
//!   body = `GainRetaliation(self, EffectDuration.UntilOpponentTurnEnd)`.
//!
//! # Patterns this test covers
//! - Hybrid Tamer digivolve (`source_treated_as`, BT7-071 sister).
//! - Conditional keyword grant with a scheduled expiry
//!   (`grant_keyword` step, `expiry: end_of_opponents_next_turn`).
//! - Condition gating on own digivolution sources
//!   (`self_digivolution_sources_trait_has` / `_contain_name`).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT7-073";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.colors = vec![color];
    c
}

fn digimon(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(3000);
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT7-073 YAML parses, compiles and is in the embedded pack")
        .dsl_card("BT7-071")
        .expect("BT7-071 Loweemon (a [Hybrid] Lv.4) is in the embedded pack")
        .add_card(tamer("KOICHI", "Koichi Kimura", CardColor::Purple))
        .add_card(tamer("TAMER-PU", "Some Purple Tamer", CardColor::Purple))
        .add_card(digimon("ROOKIE-PU", 3, CardColor::Purple, &["Beast"]))
        .add_card(digimon("HYBRID-L3", 3, CardColor::Purple, &["Hybrid"]))
        .add_card(digimon("CHAMP-PU", 4, CardColor::Purple, &["Beast"]))
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Digivolve KaiserLeomon (hand index 0) onto `base` and assert it landed.
fn digivolve_onto(runner: &mut DebugRunner, base: PermanentHandle) -> i16 {
    runner.game.current_phase = GamePhase::Main;
    let before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "KaiserLeomon must digivolve onto the base"
    );
    let _ = runner.auto_resolve();
    assert_eq!(
        stack_ids(runner, base).last().map(String::as_str),
        Some(CARD_ID),
        "KaiserLeomon is on top"
    );
    before - runner.game.memory
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn bt7_073_metadata_matches_printed_card() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "KaiserLeomon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    for t in ["Hybrid", "Variable", "Cyborg"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t} printed");
    }
}

#[test]
fn bt7_073_has_two_printed_circles_and_the_purple_tamer_route() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.alt_paths.len(), 3, "Lv.3/3 + Lv.4/1 circles + Tamer route");

    let lv3 = &card.alt_paths[0];
    assert_eq!(lv3.from.as_ref().and_then(|f| f.level_eq), Some(3));
    assert_eq!(lv3.cost, Some(CompiledCost::Literal(3)));
    let lv4 = &card.alt_paths[1];
    assert_eq!(lv4.from.as_ref().and_then(|f| f.level_eq), Some(4));
    assert_eq!(lv4.cost, Some(CompiledCost::Literal(1)));

    let hybrid = &card.alt_paths[2];
    assert_eq!(hybrid.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(hybrid.cost, Some(CompiledCost::Literal(2)), "for a memory cost of 2");
    assert_eq!(hybrid.source_treated_as.as_deref(), Some("level_3_purple_digimon"));
    assert_eq!(
        hybrid.from.as_ref().and_then(|f| f.kind),
        Some(CompiledCardKind::Tamer)
    );
}

#[test]
fn bt7_073_has_one_mandatory_conditional_when_digivolving_clause() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let wd = triggered[0];
    assert_eq!(wd.when, vec![CompiledTiming::WhenDigivolving]);
    assert_eq!(wd.scope, CompiledScope::FaceUp);
    assert!(!wd.optional, "'gains Retaliation' — no 'you may'");
    assert!(!wd.once_per_turn);
    assert!(
        wd.condition.is_some(),
        "the 'If a [Hybrid] card or [Koichi Kimura] is in its digivolution cards' gate"
    );
}

// ─── Section 2 — Condition gating (positive / negative split) ────────────────

#[test]
fn bt7_073_digivolving_onto_koichi_kimura_grants_retaliation() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let koichi = runner.place_on_field(0, "KOICHI", Some(0));

    let paid = digivolve_onto(&mut runner, koichi);

    assert_eq!(paid, 2, "Tamer route costs 2");
    assert_eq!(stack_ids(&runner, koichi).first().map(String::as_str), Some("KOICHI"));
    assert!(
        runner.game.has_keyword(koichi, Keyword::Retaliation),
        "[Koichi Kimura] beneath → gains <Retaliation>"
    );
}

#[test]
fn bt7_073_digivolving_onto_a_hybrid_lv4_grants_retaliation() {
    // Loweemon (BT7-071, [Hybrid]) on a purple rookie; KaiserLeomon uses the
    // Purple Lv.4 / cost 1 circle. Loweemon becomes a [Hybrid] source.
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let base = runner.place_stack(0, &["ROOKIE-PU", "BT7-071"]);

    let paid = digivolve_onto(&mut runner, base);

    assert_eq!(paid, 1, "Purple Lv.4 circle costs 1");
    assert!(runner.game.has_keyword(base, Keyword::Retaliation));
}

#[test]
fn bt7_073_digivolving_onto_a_hybrid_lv3_grants_retaliation() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let base = runner.place_on_field(0, "HYBRID-L3", Some(0));

    let paid = digivolve_onto(&mut runner, base);

    assert_eq!(paid, 3, "Purple Lv.3 circle costs 3");
    assert!(runner.game.has_keyword(base, Keyword::Retaliation));
}

#[test]
fn bt7_073_digivolving_onto_a_non_koichi_purple_tamer_grants_nothing() {
    // NEGATIVE: a purple Tamer that is neither [Hybrid] nor [Koichi Kimura].
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let tamer = runner.place_on_field(0, "TAMER-PU", Some(0));

    let paid = digivolve_onto(&mut runner, tamer);

    assert_eq!(paid, 2, "the Tamer route itself still works");
    assert!(
        !runner.game.has_keyword(tamer, Keyword::Retaliation),
        "no [Hybrid]/[Koichi Kimura] source → no Retaliation"
    );
    assert!(runner.pending_selection().is_none());
}

#[test]
fn bt7_073_digivolving_onto_a_plain_purple_digimon_grants_nothing() {
    // NEGATIVE: plain Purple Lv.4 base — no qualifying source.
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    let base = runner.place_on_field(0, "CHAMP-PU", Some(0));

    let paid = digivolve_onto(&mut runner, base);

    assert_eq!(paid, 1);
    assert!(!runner.game.has_keyword(base, Keyword::Retaliation));
}

// ─── Section 3 — Expiry: "until the end of your opponent's next turn" ────────

#[test]
fn bt7_073_retaliation_lasts_through_opponents_next_turn_then_expires() {
    // Both players need a deck so the turn-start draws don't deck anyone out.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["ROOKIE-PU", "ROOKIE-PU", "ROOKIE-PU"])
        .deck(1, &["ROOKIE-PU", "ROOKIE-PU", "ROOKIE-PU"])
        .memory(5)
        .start();
    let koichi = runner.place_on_field(0, "KOICHI", Some(0));
    digivolve_onto(&mut runner, koichi);
    assert!(runner.game.has_keyword(koichi, Keyword::Retaliation));

    // End our turn → opponent's turn: still granted.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    assert!(
        runner.game.has_keyword(koichi, Keyword::Retaliation),
        "Retaliation persists during the opponent's next turn"
    );

    // End the opponent's turn → back to us: expired.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 0);
    assert!(
        !runner.game.has_keyword(koichi, Keyword::Retaliation),
        "Retaliation expires at the end of the opponent's next turn"
    );
}
