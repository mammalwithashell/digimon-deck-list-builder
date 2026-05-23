//! BT13-112 Omnimon

use std::sync::Arc;

use digimon_engine::action::space::{
    encode_attack, encode_breeding_select, encode_breeding_source_select,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ON_PLAY_GAIN: i16 = 1;

struct OnPlayGainMemory;

impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("On Play: gain memory")
            .process(|ctx| ctx.gain_memory(ON_PLAY_GAIN))
            .build()]
    }
}

fn test_digimon(card_id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(6);
    card.dp = Some(dp);
    card.play_cost = 10;
    card
}

fn royal_knight(card_id: &str, name: &str) -> CardData {
    let mut card = test_digimon(card_id, name, 11000);
    card.traits = vec!["Royal Knight".to_string()];
    card
}

fn runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-112")
        .expect("BT13-112 must load from embedded DSL pack")
        .add_card(test_digimon("KING", "Breeding Host", 2000))
        .add_card(royal_knight("RK-ALPHA", "Alphamon"))
        .add_card(royal_knight("RK-OMEGA-A", "Omnimon"))
        .add_card(royal_knight("RK-OMEGA-B", "Omnimon"))
        .add_card(test_digimon("NON-RK", "Non Royal", 5000))
        .add_card(test_digimon("OPP-DGM", "Opponent", 7000))
        .memory(0)
        .start();
    runner.register_effect("RK-ALPHA", Arc::new(OnPlayGainMemory));
    runner.register_effect("RK-OMEGA-A", Arc::new(OnPlayGainMemory));
    runner.register_effect("RK-OMEGA-B", Arc::new(OnPlayGainMemory));
    runner
}

fn install_breeding_stack(runner: &mut DebugRunner) {
    let handle = runner.place_stack(
        0,
        &["RK-ALPHA", "RK-OMEGA-A", "RK-OMEGA-B", "NON-RK", "KING"],
    );
    let perm = runner.game.players[0]
        .battle_area
        .remove(handle.index as usize);
    runner.game.players[0].breeding_area = Some(perm);
}

fn source_action(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding stack")
        .card_sources
        .iter()
        .position(|source| runner.game.card_data[source.data_index].card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} source in breeding"));
    encode_breeding_source_select(0, idx as u16).expect("breeding source action")
}

fn find_field_card(runner: &DebugRunner, card_id: &str) -> PermanentHandle {
    let index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} on field"));
    PermanentHandle {
        player: 0,
        index: index as u8,
    }
}

#[test]
fn bt13_112_loads_from_dsl() {
    DebugRunner::builder()
        .dsl_card("BT13-112")
        .expect("BT13-112 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt13_112_delete_branch_deletes_one_opponent_digimon() {
    let mut runner = runner();
    let opp = runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));

    runner.execute_branch(0).expect("choose delete branch");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "delete branch removes the selected opponent Digimon"
    );
}

#[test]
fn bt13_112_source_play_branch_plays_distinct_royal_knights_trashes_breeding_and_grants_rush() {
    let mut runner = runner();
    install_breeding_stack(&mut runner);
    runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);
    let memory_before = runner.game.memory;

    runner.fire_on_play(0, omnimon.index as usize);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner
        .execute_branch(1)
        .expect("choose breeding-source play branch");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent)
    );
    runner
        .execute_action(0, encode_breeding_select(0).unwrap())
        .expect("choose breeding Digimon");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { picked: 0, .. })
        ),
        "source selection should open as a count-capped multi-pick"
    );
    let alpha_action = source_action(&runner, "RK-ALPHA");
    runner
        .execute_action(0, alpha_action)
        .expect("choose Alphamon source");

    let omega_a_action = source_action(&runner, "RK-OMEGA-A");
    let omega_b_action = source_action(&runner, "RK-OMEGA-B");
    let pending = runner
        .pending_selection_view()
        .expect("source selection re-arms after first pick");
    assert!(pending.valid_action_ids.contains(&omega_a_action));
    assert!(pending.valid_action_ids.contains(&omega_b_action));

    runner
        .execute_action(0, omega_a_action)
        .expect("choose one Omnimon source");
    assert!(
        runner.pending_selection().is_none(),
        "duplicate-name Omnimon source is masked, so the batch commits"
    );

    assert!(
        runner.game.players[0].breeding_area.is_none(),
        "a successful source play trashes the breeding Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        3,
        "Omnimon plus two different-name Royal Knight sources are in battle"
    );
    assert_eq!(
        runner.trash_size(0),
        3,
        "remaining duplicate source, non-Royal-Knight source, and breeding top card are trashed"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "played sources' own On Play effects are suppressed"
    );

    for card_id in ["BT13-112", "RK-ALPHA", "RK-OMEGA-A"] {
        let handle = find_field_card(&runner, card_id);
        assert!(
            runner.game.has_keyword(handle, Keyword::Rush),
            "{card_id} should have Rush for the turn"
        );
    }
}
