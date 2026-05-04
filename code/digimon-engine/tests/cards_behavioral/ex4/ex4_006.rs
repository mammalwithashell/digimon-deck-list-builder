//! EX4-006 Guilmon — Digimon, Lv.3, Red/Purple, 2000 DP, Cost 3.
//!
//! # Card text (cards.json)
//!
//! [On Play] If the total trashes of both players' add up to 20 or more cards,
//! this Digimon gains <Rush> for the turn.
//!
//! Digivolution: [Gigimon]: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Red/EX4_006.cs
//!
//! # Patterns this test covers
//! - H1 Rush, granted until end of turn from a triggered effect.
//! - G-COUNT-GTE-EVAL: aggregate count predicate over both players' trash.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

fn filler_card() -> CardData {
    let mut card = make_test_card("EX4-FILL", "EX4 Fill");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.dp = Some(3000);
    card
}

fn opposing_target() -> CardData {
    let mut card = make_test_card("EX4-TARGET", "EX4 Target");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.dp = Some(3000);
    card
}

fn runner_with_guilmon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX4-006")
        .expect("EX4-006 YAML must be in the embedded DSL pack")
        .add_card(filler_card())
        .add_card(opposing_target())
        .hand(0, &["EX4-006"])
        .deck(0, &["EX4-FILL"; 5])
        .deck(1, &["EX4-FILL"; 5])
        .memory(10)
        .start()
}

fn push_filler_to_trash(runner: &mut DebugRunner, player: usize, count: usize) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "EX4-FILL")
        .expect("EX4-FILL registered");
    for _ in 0..count {
        let handle = runner.game.next_card_index();
        runner.game.players[player]
            .trash
            .push(CardSource::new(data_idx, player as u8, handle));
    }
}

fn play_guilmon_with_total_trash(total_trash: usize) -> DebugRunner {
    let mut runner = runner_with_guilmon();
    let own = total_trash / 2;
    let opp = total_trash - own;
    push_filler_to_trash(&mut runner, 0, own);
    push_filler_to_trash(&mut runner, 1, opp);

    runner.game.enter_main_phase();
    runner.play(0, 0).expect("play EX4-006 from hand");
    runner
}

#[test]
fn ex4_006_yaml_compiles_with_on_play_and_gigimon_alt_path() {
    let runner = runner_with_guilmon();
    let compiled = runner
        .compiled_card("EX4-006")
        .expect("EX4-006 compiled card present");

    let on_play = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )
    });
    assert!(on_play, "EX4-006 must have an OnPlay triggered clause");

    let alt_path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(path.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        alt_path.is_some(),
        "EX4-006 must have a zero-cost Gigimon digivolve alt path"
    );
}

#[test]
fn ex4_006_on_play_grants_rush_when_shared_trash_count_is_twenty() {
    let mut runner = play_guilmon_with_total_trash(20);
    let guilmon = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(
        runner.game.has_keyword(guilmon, Keyword::Rush),
        "EX4-006 should gain Rush when shared trash count is 20"
    );

    let defender = runner.place_on_field(1, "EX4-TARGET", Some(0));
    runner.game.players[1].battle_area[defender.index as usize].is_suspended = true;
    let mask = build_action_mask(&runner.game, 0);
    let attack_action = encode_attack(guilmon.index as u16, defender.index as u16) as usize;
    assert_eq!(
        mask[attack_action], 1.0,
        "Rush must make EX4-006 able to attack the turn it was played"
    );
}

#[test]
fn ex4_006_on_play_does_not_grant_rush_when_shared_trash_count_is_nineteen() {
    let mut runner = play_guilmon_with_total_trash(19);
    let guilmon = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(
        !runner.game.has_keyword(guilmon, Keyword::Rush),
        "EX4-006 must not gain Rush below 20 shared trash cards"
    );

    let defender = runner.place_on_field(1, "EX4-TARGET", Some(0));
    runner.game.players[1].battle_area[defender.index as usize].is_suspended = true;
    let mask = build_action_mask(&runner.game, 0);
    let attack_action = encode_attack(guilmon.index as u16, defender.index as u16) as usize;
    assert_eq!(
        mask[attack_action], 0.0,
        "Without Rush, same-turn EX4-006 attacks must stay masked out"
    );
}
