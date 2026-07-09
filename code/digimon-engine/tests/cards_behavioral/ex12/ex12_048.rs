use crate::dsl_card_data::{card_data_from_compiled, compiled};
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::enums::{CardColor, Keyword, ModifierType};

use super::support::{fire_when_digivolving, plain_digimon, select_first_non_pass, DebugRunner};

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
