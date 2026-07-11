use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::enums::CardColor;
use digimon_engine::selection::SelectionKind;

use super::support::{
    hand_contains, plain_digimon, select_hand_card, trash_contains, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-021";

fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p) || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().is_some_and(|q| pred_any(q, f))
}

fn hand_size(runner: &DebugRunner, player: u8) -> usize {
    runner.game.players[player as usize].hand.len()
}

#[test]
fn ex12_021_has_tsunomon_or_vb_egg_digivolve_path() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-021 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-021");

    let special = card
        .alt_paths
        .iter()
        .find(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    pred_any(from, |p| p.name_contains.as_deref() == Some("Tsunomon"))
                        && pred_any(from, |p| p.trait_has.as_deref() == Some("VB"))
                })
        })
        .expect("Lv.2 Tsunomon/VB digivolve path");
    let from = special.from.as_ref().expect("special path has from filter");
    assert!(pred_any(from, |p| p.level_eq == Some(2)));
    assert!(pred_any(from, |p| p.name_contains.as_deref() == Some("Tsunomon")));
    assert!(pred_any(from, |p| p.trait_has.as_deref() == Some("VB")));
}

#[test]
fn ex12_021_start_main_trashes_vb_or_garurumon_card_then_draws_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-021 YAML loads")
        .add_card(vb_digimon("VB-COST", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("PLAIN", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("DRAW", CardColor::Blue, 3, 2000))
        .hand(0, &["VB-COST", "PLAIN"])
        .deck(0, &["DRAW"])
        .memory(2)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.game.enter_main_phase();
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "start-main cost should select an eligible hand card"
    );
    select_hand_card(&mut runner, 0, "VB-COST");
    runner.auto_resolve().expect("finish EX12-021 start-main");

    assert!(trash_contains(&runner, 0, "VB-COST"));
    assert!(hand_contains(&runner, 0, "PLAIN"));
    assert!(hand_contains(&runner, 0, "DRAW"));
    assert_eq!(runner.deck_size(0), deck_before - 1);
    assert_eq!(runner.memory(), memory_before + 1);
}

#[test]
fn ex12_021_inherited_when_attacking_draws_if_hand_has_7_or_fewer_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-021 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("DRAW", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 2000))
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = hand_size(&runner, 0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    runner
        .auto_resolve()
        .expect("resolve inherited attack draw");

    assert_eq!(hand_size(&runner, 0), hand_before + 1);
    assert_eq!(runner.deck_size(0), deck_before - 1);
}

#[test]
fn ex12_021_inherited_when_attacking_does_not_draw_if_hand_has_8_or_more_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-021 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("DRAW", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("FILL", CardColor::Blue, 3, 2000))
        .hand(
            0,
            &[
                "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL",
            ],
        )
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = hand_size(&runner, 0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(hand_size(&runner, 0), hand_before);
    assert_eq!(runner.deck_size(0), deck_before);
}
