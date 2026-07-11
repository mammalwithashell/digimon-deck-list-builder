use crate::dsl_card_data::{card_data_from_compiled, compiled};

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledColor, CompiledCost, CompiledDistinctBy,
    CompiledDpConstraint, CompiledPredicate, CompiledRepeat, CompiledZone,
};
use digimon_engine::enums::CardColor;

fn pred_any<F>(pred: &CompiledPredicate, f: F) -> bool
where
    F: Fn(&CompiledPredicate) -> bool + Copy,
{
    f(pred)
        || pred.all_of.iter().any(|child| pred_any(child, f))
        || pred.any_of.iter().any(|child| pred_any(child, f))
        || pred.none_of.iter().any(|child| pred_any(child, f))
        || pred.not.as_deref().is_some_and(|child| pred_any(child, f))
}

fn has_path<F>(card_id: &str, kind: CompiledAltPathKind, cost: i32, pred: F) -> bool
where
    F: Fn(&CompiledAltPath) -> bool,
{
    compiled(card_id).alt_paths.iter().any(|path| {
        path.kind == kind && path.cost == Some(CompiledCost::Literal(cost)) && pred(path)
    })
}

fn has_standard_color_path(card_id: &str, level: u8, color: CompiledColor, cost: i32) -> bool {
    has_path(card_id, CompiledAltPathKind::Digivolve, cost, |path| {
        path.from.as_deref().is_some_and(|from| {
            pred_any(from, |pred| pred.level_eq == Some(level))
                && pred_any(from, |pred| pred.color_is == Some(color))
        })
    })
}

#[test]
fn ex12_standard_dual_color_digivolve_paths_match_printed_circles() {
    for (card_id, level, color, cost) in [
        ("EX12-010", 3, CompiledColor::Black, 3),
        ("EX12-015", 4, CompiledColor::Yellow, 4),
        ("EX12-016", 4, CompiledColor::Black, 4),
        ("EX12-017", 5, CompiledColor::Black, 4),
        ("EX12-019", 5, CompiledColor::Black, 4),
        ("EX12-029", 4, CompiledColor::Yellow, 4),
        ("EX12-031", 4, CompiledColor::Yellow, 4),
        ("EX12-034", 5, CompiledColor::Black, 4),
        ("EX12-036", 5, CompiledColor::Yellow, 4),
        ("EX12-046", 4, CompiledColor::Red, 4),
        ("EX12-056", 4, CompiledColor::Yellow, 4),
        ("EX12-057", 5, CompiledColor::Yellow, 4),
        ("EX12-063", 4, CompiledColor::Green, 4),
    ] {
        assert!(
            has_standard_color_path(card_id, level, color, cost),
            "{card_id} missing printed {color:?} Lv.{level} cost-{cost} digivolve circle"
        );
    }

    assert!(
        has_path("EX12-076", CompiledAltPathKind::Digivolve, 6, |path| {
            path.from.as_deref().is_some_and(|from| {
                pred_any(from, |pred| pred.level_eq == Some(6))
                    && pred_any(from, |pred| pred.self_color_count_gte == Some(2))
            })
        }),
        "EX12-076 printed standard circle is multicolor Lv.6 cost 6"
    );
}

#[test]
fn ex12_special_digivolve_paths_match_printed_banners() {
    assert!(
        has_path("EX12-005", CompiledAltPathKind::Digivolve, 0, |path| {
            path.from.as_deref().is_some_and(|from| {
                pred_any(from, |pred| pred.name_is.as_deref() == Some("Koromon"))
            })
        }),
        "EX12-005 must include the printed [Koromon] cost-0 path"
    );

    for (card_id, level, trait_name, cost) in [
        ("EX12-019", 5, "Shambala", 3),
        ("EX12-034", 5, "Shambala", 3),
        ("EX12-057", 5, "Shambala", 3),
    ] {
        assert!(
            has_path(card_id, CompiledAltPathKind::Digivolve, cost, |path| {
                path.from.as_deref().is_some_and(|from| {
                    pred_any(from, |pred| pred.level_eq == Some(level))
                        && pred_any(from, |pred| pred.trait_has.as_deref() == Some(trait_name))
                })
            }),
            "{card_id} missing printed Lv.{level} [{trait_name}] cost-{cost} path"
        );
    }

    for (card_id, level, cost) in [("EX12-036", 5, 3), ("EX12-065", 5, 3)] {
        assert!(
            has_path(card_id, CompiledAltPathKind::Digivolve, cost, |path| {
                path.from.as_deref().is_some_and(|from| {
                    pred_any(from, |pred| pred.level_eq == Some(level))
                        && pred_any(from, |pred| {
                            pred.trait_has.as_deref() == Some("Aquatic")
                                || pred.trait_has.as_deref() == Some("Puppet")
                                || pred.trait_has.as_deref() == Some("Shambala")
                        })
                })
            }),
            "{card_id} missing printed special cost-{cost} digivolve path"
        );
    }

    assert!(
        has_path("EX12-048", CompiledAltPathKind::Digivolve, 4, |path| {
            path.from.as_deref().is_some_and(|from| {
                pred_any(from, |pred| pred.level_eq == Some(5))
                    && pred_any(from, |pred| {
                        pred.in_text_contains.as_deref() == Some("Gokuumon")
                            || pred.trait_has.as_deref() == Some("Shambala")
                    })
            })
        }),
        "EX12-048 printed special digivolve path is cost 4"
    );

    assert!(
        has_path("EX12-076", CompiledAltPathKind::Digivolve, 5, |path| {
            path.from.as_deref().is_some_and(|from| {
                pred_any(from, |pred| pred.level_eq == Some(6))
                    && pred_any(from, |pred| {
                        pred.trait_has.as_deref() == Some("Hybrid")
                            || pred.form_is.as_deref() == Some("Hybrid")
                            || pred.trait_has.as_deref() == Some("Shambala")
                            || pred.trait_has.as_deref() == Some("TS")
                    })
            })
        }),
        "EX12-076 must include the Lv.6 [Hybrid]/[Shambala]/[TS] cost-5 path"
    );
}

#[test]
fn ex12_digixros_play_banners_are_exposed_as_alt_paths() {
    for card_id in ["EX12-015", "EX12-029", "EX12-056"] {
        let card = compiled(card_id);
        let xros = card
            .alt_paths
            .iter()
            .find(|path| path.kind == CompiledAltPathKind::DigiXros)
            .unwrap_or_else(|| panic!("{card_id} prints DigiXros -2"));
        assert_eq!(xros.cost, Some(CompiledCost::Literal(7)));
        assert_eq!(xros.materials.len(), 1);

        let material = &xros.materials[0];
        assert_eq!(material.cost_delta, Some(-2));
        assert!(material.zones.contains(&CompiledZone::Hand));
        assert!(material.zones.contains(&CompiledZone::BattleArea));
        assert!(pred_any(&material.filter, |pred| {
            pred.level_lte == Some(CompiledDpConstraint::Literal(5))
        }));
        assert!(pred_any(&material.filter, |pred| {
            pred.in_text_contains.as_deref() == Some("Gokuumon")
                || pred.trait_has.as_deref() == Some("SW")
        }));
    }
}

#[test]
fn ex12_assembly_play_banners_are_exposed_as_trash_material_paths() {
    for (card_id, reduction, material_count) in [
        ("EX12-016", 2, 1),
        ("EX12-017", 6, 3),
        ("EX12-031", 2, 1),
        ("EX12-046", 2, 1),
        ("EX12-048", 6, 1),
        ("EX12-063", 2, 1),
        ("EX12-076", 9, 1),
    ] {
        let card = compiled(card_id);
        let assembly = card
            .alt_paths
            .iter()
            .find(|path| path.kind == CompiledAltPathKind::Assembly)
            .unwrap_or_else(|| panic!("{card_id} prints Assembly -{reduction}"));
        assert_eq!(assembly.cost, Some(CompiledCost::Literal(reduction)));
        assert_eq!(assembly.materials.len(), material_count);
        for material in &assembly.materials {
            assert!(material.stack_under);
            assert!(material.zones.contains(&CompiledZone::Trash));
        }
    }

    let ex12_017 = compiled("EX12-017");
    let assembly_017 = ex12_017
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::Assembly)
        .expect("EX12-017 assembly");
    for (material, level) in assembly_017.materials.iter().zip([5, 4, 3]) {
        assert!(pred_any(&material.filter, |pred| pred.level_eq == Some(level)));
        assert!(pred_any(&material.filter, |pred| {
            pred.name_contains.as_deref() == Some("Agumon")
                || pred.name_contains.as_deref() == Some("Greymon")
                || pred.trait_has.as_deref() == Some("ME")
                || pred.trait_has.as_deref() == Some("VB")
        }));
    }

    let ex12_048 = compiled("EX12-048");
    let assembly_048 = ex12_048
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::Assembly)
        .expect("EX12-048 assembly");
    let material_048 = &assembly_048.materials[0];
    assert_eq!(
        material_048.repeat,
        Some(CompiledRepeat::Range { min: 3, max: 3 })
    );
    assert_eq!(material_048.distinct_by, Some(CompiledDistinctBy::Name));
    assert!(pred_any(&material_048.filter, |pred| {
        pred.name_in.as_ref().is_some_and(|names| {
            ["Gokuumon", "Sagomon", "Cho-Hakkaimon", "Sanzomon"]
                .iter()
                .all(|name| names.iter().any(|candidate| candidate == name))
        })
    }));

    let ex12_076 = compiled("EX12-076");
    let assembly_076 = ex12_076
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::Assembly)
        .expect("EX12-076 assembly");
    let material_076 = &assembly_076.materials[0];
    assert_eq!(
        material_076.repeat,
        Some(CompiledRepeat::Range { min: 8, max: 8 })
    );
    assert_eq!(material_076.distinct_by, Some(CompiledDistinctBy::Name));
    assert!(pred_any(&material_076.filter, |pred| {
        pred.trait_has.as_deref() == Some("Hybrid")
            || pred.form_is.as_deref() == Some("Hybrid")
            || pred.trait_has.as_deref() == Some("Shambala")
    }));
}

#[test]
fn ex12_printed_color_and_trait_metadata_drift_is_pinned() {
    let ex12_048 = card_data_from_compiled("EX12-048");
    assert_eq!(ex12_048.play_cost, 13);
    assert_eq!(ex12_048.dp, Some(13000));
    assert!(ex12_048.colors.contains(&CardColor::Blue));

    let ex12_065 = card_data_from_compiled("EX12-065");
    assert!(ex12_065.traits.iter().any(|trait_name| trait_name == "TB"));

    let ex12_076 = card_data_from_compiled("EX12-076");
    assert!(ex12_076.colors.contains(&CardColor::Red));
    assert!(ex12_076.traits.iter().any(|trait_name| trait_name == "TS"));
}
