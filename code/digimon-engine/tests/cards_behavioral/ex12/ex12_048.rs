use crate::dsl_card_data::{card_data_from_compiled, compiled};
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::enums::{CardColor, Keyword, ModifierType};

use super::support::{
    digimon, field_contains, fire_when_digivolving, plain_digimon, select_first_non_pass,
    DebugRunner,
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
