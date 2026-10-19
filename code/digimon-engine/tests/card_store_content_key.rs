use digimon_engine::card_data::{
    CardData, DnaCost, DnaRequirement, DualCardData, DualDigimonFace, DualOptionFace, EvoCost,
};
use digimon_engine::enums::CardColor;
use digimon_engine::{card_store::shared_card_store, debug_runner::make_test_card};
use std::collections::HashMap;
fn base(id: &str) -> CardData {
    let mut d = make_test_card(id, "Greymon");
    d.effect_text = "AAAA".into();
    d.inherited_text = "BBBB".into();
    d.security_text = "CCCC".into();
    d.link_text = "DDDD".into();
    d.effect_class_name = "EEEE".into();
    d.also_treated_as = vec!["Alpha".into()];
    d.digixros_aliases = vec!["Alpha".into()];
    let req = DnaRequirement {
        level: 6,
        card_colors: vec![CardColor::Red],
        name_contains: "Alpha".into(),
        text_contains: "Alpha".into(),
    };
    d.dna_costs = vec![DnaCost {
        requirement1: req.clone(),
        requirement2: req,
        memory_cost: 0,
    }];
    d.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 5,
            dp: 5000,
            colors: vec![CardColor::Red],
            traits: vec!["Rock".into()],
            evo_costs: vec![EvoCost {
                card_color: 1,
                level: 4,
                memory_cost: 3,
            }],
            effect_text: "AAAA".into(),
            inherited_text: "BBBB".into(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost: 3,
            colors: vec![CardColor::Black],
            effect_text: "CCCC".into(),
            security_text: "DDDD".into(),
            keywords: vec![],
        },
    });
    d
}
fn check(id: &str, change: impl FnOnce(&mut CardData)) {
    let first_card = base(id);
    let first = shared_card_store(&HashMap::from([(id.into(), first_card.clone())]));
    let mut changed = first_card;
    change(&mut changed);
    let second = shared_card_store(&HashMap::from([(id.into(), changed.clone())]));
    assert_eq!(
        serde_json::to_value(&second.data[second.index[id]]).unwrap(),
        serde_json::to_value(changed.clone()).unwrap(),
        "cache returned stale content: {id}"
    );
    assert!(!std::sync::Arc::ptr_eq(&first, &second));
    let repeat = shared_card_store(&HashMap::from([(id.into(), changed)]));
    assert!(std::sync::Arc::ptr_eq(&second, &repeat));
}
macro_rules! case {
    ($name:ident,$change:expr) => {
        #[test]
        fn $name() {
            check(stringify!($name), $change);
        }
    };
}
case!(name_content, |d: &mut CardData| d.card_name =
    "OtherL6".into());
case!(effect_text_content, |d: &mut CardData| d.effect_text =
    "ZZZZ".into());
case!(inherited_text_content, |d: &mut CardData| d
    .inherited_text =
    "ZZZZ".into());
case!(security_text_content, |d: &mut CardData| d.security_text =
    "ZZZZ".into());
case!(link_text_content, |d: &mut CardData| d.link_text =
    "ZZZZ".into());
case!(class_name_content, |d: &mut CardData| d.effect_class_name =
    "ZZZZ".into());
case!(identity_alias_content, |d: &mut CardData| d
    .also_treated_as[0] =
    "Bravo".into());
case!(digixros_alias_content, |d: &mut CardData| d
    .digixros_aliases[0] =
    "Bravo".into());
case!(dna_name_content, |d: &mut CardData| d.dna_costs[0]
    .requirement1
    .name_contains =
    "Bravo".into());
case!(dna_text_content, |d: &mut CardData| d.dna_costs[0]
    .requirement2
    .text_contains =
    "Bravo".into());
case!(normalized_id_content, |d: &mut CardData| d.norm_id = 0.25);
case!(dual_dp_content, |d: &mut CardData| d
    .dual
    .as_mut()
    .unwrap()
    .digimon
    .dp = 9000);
case!(dual_cost_content, |d: &mut CardData| d
    .dual
    .as_mut()
    .unwrap()
    .option
    .use_cost = 9);
case!(dual_route_content, |d: &mut CardData| d
    .dual
    .as_mut()
    .unwrap()
    .digimon
    .evo_costs[0]
    .memory_cost = 9);
case!(dual_text_content, |d: &mut CardData| d
    .dual
    .as_mut()
    .unwrap()
    .option
    .security_text =
    "ZZZZ".into());
