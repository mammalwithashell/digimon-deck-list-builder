//! BT25-092 Asuna Shiroki — Tamer, Purple, Cost 4, [TS].
//!
//! # Card text (cards.json)
//! [Start of Your Main Phase] By trashing 1 card with [Three Musketeers] in its
//!   text or the [TS] trait from your hand, <Draw 1> and gain 1 memory.
//! [Main] By suspending this Tamer and trashing 1 Option card from your hand or
//!   your Digimon's digivolution cards, 1 of your Digimon may digivolve into a
//!   Digimon card with [Three Musketeers] in its text or the [TS] trait in the
//!   hand or trash with the cost reduced by 1.
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_092.cs
//!
//! # Patterns this test covers
//! - B1 start-of-main tamer (trash-as-cost → draw + memory)
//! - Tamer [Security] play-self
//! - [Main] digivolve-from-union-with-source-trash-cost is BLOCKED (dsl) —
//!   tracked as G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const YAML: &str = include_str!("../../../cards/bt25/BT25-092.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_092_structure_start_of_main_and_security() {
    let runner = asuna_runner().start();
    let compiled = runner
        .compiled_card("BT25-092")
        .expect("BT25-092 compiled card present");

    assert_eq!(compiled.card, "BT25-092");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let start_main = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("start-of-your-main-phase clause present");
    assert!(
        start_main.optional,
        "the trash-as-cost clause is optional (\"by trashing\")"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );
}

// ── Section 2: [Start of Your Main Phase] trash → draw + memory ──────────

#[test]
fn bt25_092_start_of_main_trashes_ts_card_draws_and_gains_memory() {
    let mut runner = asuna_runner()
        .hand(0, &["TS-CARD"])
        .add_card(make_ts_hand_card("TS-CARD"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(3)
        .start();
    runner.place_on_field(0, "BT25-092", Some(0));

    let trash_before = runner.trash_size(0);
    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();

    runner.game.enter_main_phase();

    // The clause is optional ("by trashing …"), so an accept/decline prompt
    // installs first; accept it.
    let opt = runner
        .pending_selection_view()
        .expect("[Start of Main] optional-activation prompt installs");
    assert_eq!(opt.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional());
    runner
        .execute_action(opt.selecting_player, opt.valid_action_ids[0])
        .expect("accept the optional cost");

    // Then the trash-as-cost select installs over the TS card in hand.
    let view = runner
        .pending_selection_view()
        .expect("trash select installs after accepting");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash the TS card as cost");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "TS card trashed as cost"
    );
    assert_eq!(runner.deck_size(0), deck_before - 1, "Draw 1 fired");
    assert_eq!(runner.memory(), memory_before + 1, "gained 1 memory");
}

#[test]
fn bt25_092_start_of_main_no_cost_card_does_not_fire() {
    // Hand holds only a non-TS, non-Three-Musketeers card → condition fails →
    // no selection installs, no draw, no memory gain.
    let mut runner = asuna_runner()
        .hand(0, &["PLAIN"])
        .add_card(make_plain_hand_card("PLAIN"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(3)
        .start();
    runner.place_on_field(0, "BT25-092", Some(0));

    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no eligible cost card → clause does not install"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no draw");
    assert_eq!(runner.memory(), memory_before, "no memory gain");
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn asuna_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-092 YAML loads")
}

fn make_ts_hand_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["TS".to_string()];
    card
}

fn make_plain_hand_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn make_filler(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card
}
