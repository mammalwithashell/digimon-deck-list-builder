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
                    pred_any(from, |p| p.name_is.as_deref() == Some("Tsunomon"))
                        && pred_any(from, |p| p.trait_has.as_deref() == Some("VB"))
                })
        })
        .expect("Lv.2 Tsunomon/VB digivolve path");
    let from = special.from.as_ref().expect("special path has from filter");
    assert!(pred_any(from, |p| p.level_eq == Some(2)));
    assert!(pred_any(from, |p| p.name_is.as_deref() == Some("Tsunomon")));
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

/// DCGO `EX12_021.cs`: the inherited [When Attacking] is listed by
/// `CanUseCondition` (an own attack) and only `ActivateCoroutine` tests the
/// hand count (`RemoveUse()` when it fails), so with 8+ cards in hand it is
/// STILL an orderable trigger next to a sibling [When Attacking], and
/// resolving it does nothing and leaves the [Once Per Turn] unspent
/// (CR 4.3 15-8-3-9). G-ENGINE-TRIGGER-ORDER-NO-TARGET-ENTRY.
#[test]
fn ex12_021_inherited_attack_hand_test_is_a_processing_condition_and_stays_orderable() {
    use digimon_engine::selection::SelectionKind;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-021 YAML loads")
        .dsl_card("EX12-024")
        .expect("EX12-024 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("DRAW", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("FILL", CardColor::Blue, 3, 2000))
        .hand(0, &["FILL"; 8])
        .deck(0, &["DRAW"; 4])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "EX12-024", "CARRIER"]);
    runner.attack_player(carrier, 1, false);

    let pending = runner
        .pending_selection()
        .expect("both inherited [When Attacking] effects are orderable");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    let labels: Vec<String> = pending
        .effect_choices
        .as_ref()
        .map(|choices| choices.iter().map(|c| c.label.clone()).collect())
        .unwrap_or_default();
    assert!(
        labels.iter().any(|l| l.starts_with(CARD_ID)),
        "EX12-021 (hand 8) stays in the order list: {labels:?}"
    );
    assert_eq!(pending.valid_action_ids.len(), 2, "{labels:?}");

    // Resolve EX12-021 first: with 8 cards in hand it draws nothing.
    let ex12_021_action = pending
        .effect_choices
        .as_ref()
        .and_then(|choices| choices.iter().find(|c| c.label.starts_with(CARD_ID)))
        .map(|c| c.action_id)
        .expect("EX12-021 entry");
    let hand_before = hand_size(&runner, 0);
    runner
        .execute_action(0, ex12_021_action)
        .expect("choose EX12-021");
    // EX12-021 drew nothing (8 cards in hand); the only draw is the sibling
    // EX12-024's own "draw 1, then trash 1", which is now waiting for its trash.
    assert_eq!(
        hand_size(&runner, 0),
        hand_before + 1,
        "8 cards in hand: EX12-021 draws nothing (only the sibling's draw)"
    );
    // The unmet hand test left EX12-021's [Once Per Turn] unspent (DCGO
    // `RemoveUse()`); only the sibling's OPT is used.
    assert_eq!(runner.game.opt_used(carrier), 1, "EX12-021's OPT stays unspent");
}
