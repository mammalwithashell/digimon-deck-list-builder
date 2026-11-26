use crate::dsl_card_data::{card_data_from_compiled, compiled};
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;

use super::support::{
    blue_sw, decline_all_selections, digimon, field_contains, fire_when_digivolving,
    hand_contains, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-048";

#[test]
fn ex12_048_printed_metadata_and_special_digivolve_path() {
    let data = card_data_from_compiled(CARD_ID);
    assert_eq!(data.play_cost, 13);
    assert_eq!(data.dp, Some(13000));

    let card = compiled(CARD_ID);
    let has_special_lv5_path = card.alt_paths.iter().any(|path| {
        if path.kind != CompiledAltPathKind::Digivolve
            || path.cost != Some(CompiledCost::Literal(4))
        {
            return false;
        }
        let Some(from) = path.from.as_ref() else {
            return false;
        };
        from.level_eq == Some(5)
            && from.any_of.iter().any(|p| {
                p.in_text_contains.as_deref() == Some("Gokuumon")
                    || p.trait_has.as_deref() == Some("Shambala")
            })
    });
    assert!(
        has_special_lv5_path,
        "printed Lv.5 w/[Gokuumon] text or [Shambala] trait cost-4 path must be present"
    );
}

#[test]
fn ex12_048_has_printed_keywords() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-048 YAML loads")
        .start();
    let gokuumon = runner.place_on_field(0, CARD_ID, Some(0));

    for keyword in [
        Keyword::Rush,
        Keyword::Raid,
        Keyword::Piercing,
        Keyword::SecurityAttackPlus(1),
    ] {
        assert!(runner.game.has_keyword(gokuumon, keyword), "{keyword:?}");
    }
}

#[test]
fn ex12_048_on_play_dp_minus_scales_by_level5_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-048 YAML loads")
        .add_card(plain_digimon("LV5-A", CardColor::Red, 5, 7000))
        .add_card(plain_digimon("LV5-B", CardColor::Yellow, 5, 7000))
        .add_card(plain_digimon("OPP", CardColor::Red, 6, 15000))
        .start();
    let gokuumon = runner.place_stack(0, &["LV5-A", "LV5-B", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, gokuumon);
    select_first_non_pass(&mut runner);

    assert_eq!(
        runner.game.modifiers.sum(opp, ModifierType::ChangeDp),
        -14000
    );
}

/// EX12-048's `[All Turns]` replacement is SELF-scoped, and the card face says
/// so twice: "When **this Digimon** would leave the battle area other than by
/// your effects, you may play 2 level 5 cards ... from **its** digivolution
/// cards" (`data/card_bundles/EX12-048.md`, the official Bandai text). Both
/// halves are self-referential -- whose leave opens the window, and whose stack
/// supplies the payload.
///
/// It was authored with `replacement_subject_is_mine: true` in `active_when`,
/// which is a CROSS-permanent gate: `predicate_reads_replacement_subject`
/// (dsl_cards/lower_replacement.rs:406) lists that key, so the clause skipped
/// `replacement_subject_is_source` and `collect_candidates` offered it for ANY
/// of your Digimon leaving -- then played level 5 sources out of that other
/// Digimon's stack. Same defect family as the four `<Decode>` cards fixed
/// alongside this, but this one is a card-specific clause rather than the
/// keyword, which is why a Decode-shaped search missed it.
///
/// Note the cause exclusion here is `own_effect`, NOT battle: "other than by
/// your effects" is this card's own wording and must survive the fix.
#[test]
fn ex12_048_all_turns_window_does_not_open_for_another_permanents_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-048 YAML loads")
        .add_card(digimon("SW-SRC", CardColor::Red, 5, 7, 6000, &["SW"]))
        .add_card(plain_digimon("OTHER-TOP", CardColor::Red, 6, 9000))
        .memory(10)
        .start();
    // SeitenGokuumon is on the field but is NOT the permanent that leaves.
    let _seiten = runner.place_on_field(0, CARD_ID, Some(0));
    // A SECOND P0 Digimon stacked over a Lv.5 [SW] source -- exactly the
    // payload this clause names, so a wrongly cross-permanent window would
    // have a legal candidate and really park.
    let other = runner.place_stack(0, &["SW-SRC", "OTHER-TOP"]);

    // A non-own-effect leave of the OTHER permanent (opponent-effect bounce).
    runner.game.return_to_hand_from_effect(other, 1);

    assert!(
        runner.game.pending_selection.is_none(),
        "the [All Turns] window belongs to THIS Digimon: SeitenGokuumon is \
         still on the field and the leaving Digimon carries no such clause \
         (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        !field_contains(&runner, 0, "SW-SRC"),
        "the payload comes from ITS OWN digivolution cards -- this clause may \
         not reach into another permanent's stack"
    );
}

#[test]
fn ex12_048_all_turns_replacement_plays_two_level5_sw_sources_simultaneously() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-048 YAML loads")
        .add_card(digimon("SW-LV5-A", CardColor::Red, 5, 7, 7000, &["SW"]))
        .add_card(digimon("SW-LV5-B", CardColor::Yellow, 5, 7, 7000, &["SW"]))
        .memory(10)
        .start();
    let seiten = runner.place_stack(0, &["SW-LV5-A", "SW-LV5-B", CARD_ID]);

    runner.game.return_to_hand_from_effect(seiten, 1);

    // Accept the optional [All Turns] replacement.
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept EX12-048 replacement");
    // Pick both Lv.5 [SW] sources from the single multi-select before either plays.
    select_first_non_pass(&mut runner);
    assert!(
        !field_contains(&runner, 0, "SW-LV5-A") && !field_contains(&runner, 0, "SW-LV5-B"),
        "neither source should enter the field before the second material pick finishes"
    );
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-048 simultaneous play");

    assert!(field_contains(&runner, 0, "SW-LV5-A"), "first Lv.5 SW source played");
    assert!(field_contains(&runner, 0, "SW-LV5-B"), "second Lv.5 SW source played");
}

/// CR 4.3 §16-14 / DCGO `CardController.cs:1475-1481` (`Nov24#g15` step 70):
/// when `EX12-048` plays `EX12-029` and `EX12-056` from its digivolution cards
/// via `play_from_materials`, each entrant with `<DigiXros>` and an eligible
/// material must offer an optional DigiXros material prompt before simultaneous
/// entry — including on the opponent's turn and regardless of printed cost when
/// the source play is free (`CostDelta::Free`).
#[test]
fn ex12_048_source_play_offers_optional_digixros_for_both_entrants_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-048 YAML loads")
        .dsl_card("EX12-029")
        .expect("EX12-029 YAML loads")
        .dsl_card("EX12-056")
        .expect("EX12-056 YAML loads")
        .add_card(blue_sw("SW-MAT", 4))
        .hand(0, &["SW-MAT"])
        .start();
    runner.game.turn_player_idx = 1;
    // Printed cost (7) would underflow memory_range.0 (-10) under the turn-player
    // formula `-8 - 7 = -15`, verifying `can_stop` respects `CostDelta::Free` and
    // opponent-turn memory bounds.
    runner.game.set_memory(-8);

    let seiten = runner.place_stack(0, &["EX12-029", "EX12-056", CARD_ID]);
    runner.game.return_to_hand_from_effect(seiten, 1);

    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept EX12-048 replacement");
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);

    for label in ["first entrant", "second entrant"] {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("expected DigiXros material prompt for {label}"));
        assert_eq!(view.kind, SelectionKind::Material, "{label}: {view:?}");
        assert_eq!(view.prompt, "Select DigiXros material", "{label}: {view:?}");
        assert_eq!(view.selecting_player, 0, "{label}: {view:?}");
        assert!(
            view.is_optional,
            "free opponent-turn DigiXros prompt for {label} must allow PASS"
        );
        assert!(
            !field_contains(&runner, 0, "EX12-029") && !field_contains(&runner, 0, "EX12-056"),
            "neither simultaneous entrant may enter before DigiXros preparation completes ({label})"
        );
        runner
            .execute_action(0, PASS)
            .unwrap_or_else(|e| panic!("decline DigiXros for {label}: {e:?}"));
    }

    assert!(field_contains(&runner, 0, "EX12-029"), "EX12-029 entered play");
    assert!(field_contains(&runner, 0, "EX12-056"), "EX12-056 entered play");
    assert_eq!(
        runner.game.memory, -8,
        "free source plays must not charge memory"
    );
    assert!(
        hand_contains(&runner, 0, "SW-MAT"),
        "declining DigiXros must leave the hand material unconsumed"
    );

    decline_all_selections(&mut runner);
    for card_id in ["EX12-029", "EX12-056"] {
        let perm = runner.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
            .unwrap_or_else(|| panic!("{card_id} must remain on battle area"));
        assert_eq!(
            perm.card_sources.len(),
            1,
            "{card_id} should have 0 DigiXros sources after declining"
        );
    }
}

/// Selecting a hand DigiXros material for the first simultaneous prepared source
/// play must tuck that card underneath the first entrant at commit AND exclude
/// the already-claimed physical card from the second entrant's DigiXros prompt.
#[test]
fn ex12_048_source_play_digixros_tucks_material_and_prevents_double_pick() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-048 YAML loads")
        .dsl_card("EX12-029")
        .expect("EX12-029 YAML loads")
        .dsl_card("EX12-056")
        .expect("EX12-056 YAML loads")
        .add_card(blue_sw("SW-MAT", 4))
        .hand(0, &["SW-MAT"])
        .start();
    runner.game.turn_player_idx = 1;
    runner.game.set_memory(0);

    let seiten = runner.place_stack(0, &["EX12-029", "EX12-056", CARD_ID]);
    runner.game.return_to_hand_from_effect(seiten, 1);

    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept EX12-048 replacement");
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);

    let dx1 = runner
        .pending_selection_view()
        .expect("first entrant DigiXros prompt");
    assert_eq!(dx1.kind, SelectionKind::Material);
    assert_eq!(dx1.prompt, "Select DigiXros material");
    assert!(dx1.is_optional);
    select_first_non_pass(&mut runner);

    // SW-MAT was the sole eligible material and is now claimed by the first
    // prepared play, so the second entrant must NOT be offered SW-MAT again.
    if let Some(next_view) = runner.pending_selection_view() {
        assert_ne!(
            next_view.prompt, "Select DigiXros material",
            "claimed material must not be re-offered to the second simultaneous entrant: {next_view:?}"
        );
    }

    assert!(
        !hand_contains(&runner, 0, "SW-MAT"),
        "selected DigiXros material must be consumed from hand at simultaneous commit"
    );
    assert!(field_contains(&runner, 0, "EX12-029"), "EX12-029 entered play");
    assert!(field_contains(&runner, 0, "EX12-056"), "EX12-056 entered play");

    decline_all_selections(&mut runner);
    let mut source_counts = Vec::new();
    let mut tucked_sw_mat = 0usize;
    for card_id in ["EX12-029", "EX12-056"] {
        let perm = runner.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
            .unwrap_or_else(|| panic!("{card_id} must be on P0 battle area"));
        source_counts.push(perm.card_sources.len());
        if perm.card_sources.len() == 2
            && perm.card_sources[0].card_id(&runner.game.card_data) == "SW-MAT"
        {
            tucked_sw_mat += 1;
        }
    }
    source_counts.sort();
    assert_eq!(
        source_counts,
        vec![1, 2],
        "exactly one simultaneous entrant should tuck the single SW-MAT source"
    );
    assert_eq!(tucked_sw_mat, 1, "SW-MAT must be tucked under the DigiXros entrant");
}
