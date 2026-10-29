use digimon_engine::action::space::{
    decode_breeding_source_select, decode_source_select, encode_breeding_source_select,
    encode_source_select, ACTION_SPACE_SIZE, BREEDING_SOURCE_SELECT_END,
    BREEDING_SOURCE_SELECT_START, BREEDING_TARGET, SOURCES_PER_FIELD, SOURCE_SELECT_END,
    SOURCE_SELECT_START,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::{CountCappedZone, EffectContext};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::{action::mask::build_action_mask, action::space::PASS};

#[test]
fn source_select_encoder_round_trips_existing_range() {
    let action = encode_source_select(3, 5).expect("field 3 source 5 fits");
    assert_eq!(action, SOURCE_SELECT_START + 3 * SOURCES_PER_FIELD + 5);
    assert_eq!(decode_source_select(action), (3, 5));
}

#[test]
fn source_select_encoder_rejects_values_outside_existing_range() {
    assert_eq!(encode_source_select(14, 0), None);
    assert_eq!(encode_source_select(0, SOURCES_PER_FIELD), None);
    // Battle-area source-select abuts the breeding-source sub-range, which
    // is now the LAST range and abuts ACTION_SPACE_SIZE (Task S1.3).
    assert_eq!(SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START);
    assert_eq!(BREEDING_SOURCE_SELECT_END as usize, ACTION_SPACE_SIZE);
}

#[test]
fn breeding_source_select_encoder_round_trips() {
    let action = encode_breeding_source_select(1, 4).expect("player 1 source 4 fits");
    assert_eq!(action, BREEDING_SOURCE_SELECT_START + SOURCES_PER_FIELD + 4);
    assert_eq!(decode_breeding_source_select(action), (1, 4));
}

/// A breeding-carrier source selection installs `valid_action_ids` in the
/// `BREEDING_SOURCE_SELECT` range; the mask exposes them to the selecting
/// player and they survive the `(aid as usize) < ACTION_SPACE_SIZE` guard.
#[test]
fn breeding_source_select_ids_survive_the_mask() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("BSRC", "Breeding Source"))
        .add_card(make_test_card("BMID", "Breeding Mid"))
        .add_card(make_test_card("BTOP", "Breeding Top"))
        .start();
    let p0 = 0;
    let p1 = 1;
    // Build a 3-card breeding stack on P0.
    let handle = r.place_stack(p0, &["BSRC", "BMID", "BTOP"]);
    let perm = r.game.players[p0 as usize]
        .battle_area
        .remove(handle.index as usize);
    r.game.players[p0 as usize].breeding_area = Some(perm);
    let carrier = PermanentHandle {
        player: p0,
        index: BREEDING_TARGET as u8,
    };

    let carrier_top = r.game.players[p0 as usize]
        .breeding_area
        .as_ref()
        .unwrap()
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, carrier_top, None, p0);
        ctx.select_count_capped_multi(
            p0,
            CountCappedZone::Material(carrier),
            2,
            "pick breeding sources",
            false,
            None,
            |_, _| true,
            |_, _| {},
        );
    }

    // Owned pages retain breeding provenance independently of action IDs.
    let sel = r.game.pending_selection.as_ref().expect("pending installed");
    let rows = sel.card_choice.as_ref().expect("physical material owner")
        .view(&r.game, p0).rows;
    assert_eq!(rows.len(), 2, "both physical breeding sources are offered");
    let mut positions = Vec::new();
    for row in rows {
        assert!(matches!(row.operation, digimon_engine::owned_card_choice::CardChoiceOperation::CommitCard));
        assert!(matches!(row.zone, Some(digimon_engine::owned_card_choice::CardChoiceZone::BreedingSource)));
        assert_eq!(row.owner, Some(p0));
        assert!(sel.valid_action_ids.contains(&row.action_id));
        assert!((row.action_id as usize) < ACTION_SPACE_SIZE);
        positions.push(row.position.expect("source position"));
    }
    positions.sort();
    assert_eq!(positions, vec!["0", "1"]);

    let p0_mask = build_action_mask(&r.game, p0);
    let p1_mask = build_action_mask(&r.game, p1);
    assert_eq!(p0_mask.len(), ACTION_SPACE_SIZE);
    for &aid in &sel.valid_action_ids {
        assert_eq!(
            p0_mask[aid as usize], 1.0,
            "the mask exposes breeding-source ID {aid} to the selecting player"
        );
    }
    assert!(
        p1_mask.iter().all(|v| *v == 0.0),
        "non-selecting player sees an empty mask"
    );
}

#[test]
fn source_multi_mask_only_exposes_selecting_players_pending_actions() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let stack = r.place_stack(p0, &["SRC-A", "TOP-A"]);
    let stack_top = r.top_card(stack);
    {
        let mut ctx = EffectContext::new(&mut r.game, stack_top, Some(stack), p0);
        ctx.select_own_sources(
            "pick one source",
            1,
            1,
            move |_, source| source.card != stack_top,
            |_, _| {},
        );
    }

    let p0_mask = build_action_mask(&r.game, p0);
    let p1_mask = build_action_mask(&r.game, p1);
    assert!(
        p0_mask.iter().any(|v| *v > 0.5),
        "selecting player sees source action"
    );
    assert!(
        p1_mask.iter().all(|v| *v == 0.0),
        "non-selecting player sees empty mask"
    );
    assert_eq!(
        p0_mask[PASS as usize], 0.0,
        "exact one source cannot PASS before picking"
    );
}
