use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::ModifierType;

fn fresh_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build()
}

fn runner_with_deck() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .deck(0, &["F", "F", "F"])
        .build()
}

#[test]
fn add_player_modifier_step_installs_floodgate_on_referenced_player() {
    let mut runner = fresh_runner();
    let card = runner.game.players[0].hand[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut bindings = Bindings::new();
        run_step(
            &CompiledStep::AddPlayerModifier {
                target_player: CompiledPlayerRef::Opponent,
                modifier: "CannotPlayDigimonByEffect".to_string(),
                value: 0,
                expiry: "end_of_opponents_turn".to_string(),
            },
            &mut ctx,
            &mut bindings,
        );
    }

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "opponent should receive CannotPlayDigimonByEffect from add_player_modifier"
    );
    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "controller should not receive opponent-targeted player modifier"
    );
}

// ── Task 4: Memory steps ─────────────────────────────────────────────────────

#[test]
fn gain_memory_step_mutates_game_memory() {
    let mut runner = fresh_runner();
    let before = runner.game.memory;
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(&CompiledStep::GainMemory(2), &mut ctx, &mut b);
    }
    assert_eq!(runner.game.memory, before + 2);
}

#[test]
fn lose_memory_step_mutates_game_memory() {
    let mut runner = fresh_runner();
    let before = runner.game.memory;
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(&CompiledStep::LoseMemory(3), &mut ctx, &mut b);
    }
    assert_eq!(runner.game.memory, before - 3);
}

#[test]
fn set_memory_step_sets_absolute_value() {
    let mut runner = fresh_runner();
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(&CompiledStep::SetMemory(5), &mut ctx, &mut b);
    }
    assert_eq!(runner.game.memory, 5);
}

// ── Phase 2 Track F (G-DSL-GAIN-MEMORY-FN) — formula-valued memory steps ─────

#[test]
fn gain_memory_fn_evaluates_formula_and_adds_to_memory() {
    // Seed P0 with a 4-card hand and a literal-4 gain. The formula evaluator
    // accepts any FormulaSpec — the trivial Literal case proves wiring.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F", "F", "F", "F"])
        .build();
    let before = runner.game.memory;
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::GainMemoryFn {
                formula: digimon_dsl::compiled::CompiledFormula::Literal(4),
            },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(
        runner.game.memory,
        before + 4,
        "gain_memory_fn(literal:4) adds 4 to memory"
    );
}

#[test]
fn gain_memory_fn_with_floor_div_and_card_count_yields_quotient() {
    // EX1-021 shape: gain memory == floor(hand_count / 4). With 5 cards
    // in P0's hand, expect +1; with 8 cards, expect +2.
    use digimon_dsl::compiled::{
        CompiledFormula, CompiledPerSelector, CompiledPlayerRef, CompiledZone,
    };

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F", "F", "F", "F", "F"])
        .build();
    let before = runner.game.memory;

    let formula = CompiledFormula::FloorDiv(vec![
        CompiledFormula::BasePerDelta {
            base: 0,
            per: CompiledPerSelector::CardCountInZoneScoped {
                zone: CompiledZone::Hand,
                of: CompiledPlayerRef::You,
            },
            delta: 1,
        },
        CompiledFormula::Literal(4),
    ]);

    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(&CompiledStep::GainMemoryFn { formula }, &mut ctx, &mut b);
    }
    assert_eq!(
        runner.game.memory,
        before + 1,
        "5 cards / 4 == 1 → +1 memory"
    );
}

#[test]
fn lose_memory_fn_evaluates_formula_and_subtracts_from_memory() {
    let mut runner = fresh_runner();
    let before = runner.game.memory;
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::LoseMemoryFn {
                formula: digimon_dsl::compiled::CompiledFormula::Literal(3),
            },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.memory, before - 3);
}

// ── Task 5: Draw/trash/shuffle steps ─────────────────────────────────────────

#[test]
fn draw_step_pulls_cards_into_hand() {
    let mut runner = runner_with_deck();
    let before_hand = runner.game.players[0].hand.len();
    let before_deck = runner.game.players[0].deck.len();
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::Draw {
                of: CompiledPlayerRef::You,
                count: 2,
            },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.players[0].hand.len(), before_hand + 2);
    assert_eq!(runner.game.players[0].deck.len(), before_deck - 2);
}

#[test]
fn trash_from_top_moves_deck_to_trash() {
    let mut runner = runner_with_deck();
    let before_deck = runner.game.players[0].deck.len();
    let before_trash = runner.game.players[0].trash.len();
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::TrashFromTop {
                of: CompiledPlayerRef::You,
                count: 1,
            },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.players[0].deck.len(), before_deck - 1);
    assert_eq!(runner.game.players[0].trash.len(), before_trash + 1);
}

#[test]
fn shuffle_deck_step_preserves_card_count() {
    let mut runner = runner_with_deck();
    let before = runner.game.players[0].deck.len();
    {
        let card = runner.game.players[0].hand[0].handle();
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_step(
            &CompiledStep::ShuffleDeck {
                of: CompiledPlayerRef::You,
            },
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.players[0].deck.len(), before);
}
