//! Printed colors and ordinary evolution routes, using full authored YAML.
//! The five-row JSON fixture is a frozen source-derived production-loader input.
//! Synthetic base cards isolate color/level/trait grammar, not deck reachability.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, BREEDING_TARGET, PLAY_HAND_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use std::collections::HashMap;

fn yaml(id: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("cards")
            .join(id.split('-').next().unwrap().to_lowercase())
            .join(format!("{id}.yaml")),
    )
    .unwrap()
}
fn production() -> HashMap<String, CardData> {
    CardData::load_from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/focused_five_metadata_v6.json"
    )))
    .unwrap()
}
fn data(id: &str, level: u8, color: CardColor, traits: &[&str], text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(4000);
    if level == 2 {
        c.card_kind = CardKind::DigiEgg;
        c.dp = None;
    }
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c.effect_text = text.into();
    c
}
fn runner(id: &str, seat: u8, loaded: bool, witnesses: Vec<CardData>) -> DebugRunner {
    let mut b = DebugRunner::builder();
    if loaded {
        b = b.with_card_data(production());
    }
    b = b.from_dsl_yaml(&yaml(id)).unwrap();
    b = b.add_card(data("INERT", 3, CardColor::White, &[], ""));
    for c in witnesses {
        b = b.add_card(c);
    }
    let mut r = b
        .hand(seat, &[id])
        .deck(seat, &["INERT"; 10])
        .deck(1 - seat, &["INERT"; 10])
        .build();
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, 1 - seat);
    r.game.memory = 10;
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 2;
    r
}
fn colors(id: &str, expected: &[CardColor], loaded: bool) {
    for seat in [0, 1] {
        let r = runner(id, seat, loaded, vec![]);
        let c = r.game.card_data.iter().find(|c| c.card_id == id).unwrap();
        assert_eq!(c.colors, expected, "{id}, loaded={loaded}, seat={seat}");
    }
}
fn route(
    id: &str,
    loaded: bool,
    color: CardColor,
    level: u8,
    traits: &[&str],
    text: &str,
    cost: Option<i16>,
) {
    for seat in [0, 1] {
        let mut r = runner(
            id,
            seat,
            loaded,
            vec![data("BASE", level, color, traits, text)],
        );
        let target = if level == 2 {
            r.place_in_breeding(seat, "BASE");
            BREEDING_TARGET
        } else {
            r.place_on_field(seat, "BASE", Some(0)).index as u16
        };
        let action = encode_digivolve(0, target);
        assert!(r.pending_selection().is_none());
        assert_eq!(
            build_action_mask(&r.game, seat)[action as usize],
            if cost.is_some() { 1.0 } else { 0.0 },
            "{id},loaded={loaded},seat={seat},base={color:?}/{level}/{traits:?}"
        );
        assert_eq!(
            build_action_mask(&r.game, 1 - seat)[action as usize],
            0.0,
            "opponent cannot evolve actor hand"
        );
        if let Some(cost) = cost {
            r.game.decode_action(action, seat);
            assert!(
                r.pending_selection().is_none(),
                "isolated single route and no eligible follow-up targets; no autochoices"
            );
            assert_eq!(r.memory(), 10 - cost, "printed ordinary/special cost");
            let p = &r.game.players[seat as usize];
            let evolved = if level == 2 {
                p.breeding_area.as_ref().unwrap()
            } else {
                &p.battle_area[0]
            };
            assert_eq!(evolved.top_card().card_id(&r.game.card_data), id);
        }
    }
}
fn option_colors(id: &str, loaded: bool, colors: &[CardColor], use_req: Option<&str>, legal: bool) {
    for seat in [0, 1] {
        let mut witnesses = Vec::new();
        for (i, color) in colors.iter().enumerate() {
            witnesses.push(data(&format!("COLOR-{i}"), 3, *color, &[], ""));
        }
        if let Some(t) = use_req {
            witnesses.push(data("USE-REQ", 3, CardColor::White, &[t], ""));
        }
        let mut r = runner(id, seat, loaded, witnesses);
        for i in 0..colors.len() {
            r.place_on_field(seat, &format!("COLOR-{i}"), Some(0));
        }
        if use_req.is_some() {
            r.place_on_field(seat, "USE-REQ", Some(0));
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            build_action_mask(&r.game, seat)[PLAY_HAND_START as usize],
            if legal { 1.0 } else { 0.0 },
            "{id},loaded={loaded},seat={seat},colors={colors:?},use_req={use_req:?}"
        );
    }
}
fn tamer_contribution(loaded: bool, color: CardColor, legal: bool) {
    for seat in [0, 1] {
        let mut b = DebugRunner::builder();
        if loaded {
            b = b.with_card_data(production());
        }
        b = b.from_dsl_yaml(&yaml("BT24-085")).unwrap();
        let option=format!("card: PROBE\nname: Color probe\nkind: option\ncolor: [{}]\ncost: 0\neffects:\n  - when: main_from_hand\n    process:\n      - draw: {{ of: you, count: 1 }}\n",format!("{color:?}").to_lowercase());
        b = b
            .from_dsl_yaml(&option)
            .unwrap()
            .add_card(data("INERT", 3, CardColor::White, &[], ""));
        let mut r = b.hand(seat, &["PROBE"]).deck(seat, &["INERT"; 3]).build();
        r.game.turn_player_idx = seat as usize;
        r.game.memory_pair = (seat, 1 - seat);
        r.game.memory = 10;
        r.game.current_phase = GamePhase::Main;
        r.place_on_field(seat, "BT24-085", Some(0));
        assert_eq!(
            build_action_mask(&r.game, seat)[PLAY_HAND_START as usize],
            if legal { 1.0 } else { 0.0 },
            "loaded={loaded},seat={seat},probe={color:?}"
        );
    }
}
macro_rules! color_test {($name:ident,$id:literal,$loaded:expr,$($c:ident),+)=>{#[test]fn $name(){colors($id,&[$(CardColor::$c),+],$loaded);}};}
mod batch1 {
    use super::*;
    color_test!(minervamon_yaml_colors, "BT24-041", false, Yellow, Black);
    color_test!(minervamon_loaded_colors, "BT24-041", true, Yellow, Black);
    color_test!(dan_kanan_yaml_colors, "BT24-085", false, Green, Red);
    color_test!(dan_kanan_loaded_colors, "BT24-085", true, Green, Red);
    color_test!(gazimon_yaml_colors, "BT25-078", false, Purple);
    color_test!(gazimon_loaded_colors, "BT25-078", true, Purple);
    color_test!(
        sanmyojin_yaml_colors,
        "EX12-070",
        false,
        Yellow,
        Green,
        Blue
    );
    color_test!(
        sanmyojin_loaded_colors,
        "EX12-070",
        true,
        Yellow,
        Green,
        Blue
    );
    #[test]
    fn minervamon_black_yaml() {
        route("BT24-041", false, CardColor::Black, 5, &[], "", Some(4));
    }
    #[test]
    fn minervamon_black_loaded() {
        route("BT24-041", true, CardColor::Black, 5, &[], "", Some(4));
    }
    #[test]
    fn minervamon_yellow_and_negatives() {
        for loaded in [false, true] {
            route("BT24-041", loaded, CardColor::Yellow, 5, &[], "", Some(4));
            route("BT24-041", loaded, CardColor::Purple, 5, &[], "", None);
            route("BT24-041", loaded, CardColor::Black, 4, &[], "", None);
        }
    }
    #[test]
    fn minervamon_special_recipe_preserved() {
        for loaded in [false, true] {
            for t in ["Beastkin", "Dark Dragon", "TS"] {
                route("BT24-041", loaded, CardColor::White, 5, &[t], "", Some(3));
            }
            route("BT24-041", loaded, CardColor::White, 4, &["TS"], "", None);
        }
    }
    #[test]
    fn gazimon_black_refused_yaml() {
        route("BT25-078", false, CardColor::Black, 2, &[], "", None);
    }
    #[test]
    fn gazimon_black_refused_loaded() {
        route("BT25-078", true, CardColor::Black, 2, &[], "", None);
    }
    #[test]
    fn gazimon_purple_yaml() {
        route("BT25-078", false, CardColor::Purple, 2, &[], "", Some(0));
    }
    #[test]
    fn gazimon_purple_loaded() {
        route("BT25-078", true, CardColor::Purple, 2, &[], "", Some(0));
    }
    #[test]
    fn gazimon_wrong_level_and_special_recipe() {
        for loaded in [false, true] {
            route("BT25-078", loaded, CardColor::Purple, 3, &[], "", None);
            route(
                "BT25-078",
                loaded,
                CardColor::Green,
                2,
                &["TS"],
                "",
                Some(0),
            );
            route(
                "BT25-078",
                loaded,
                CardColor::Green,
                2,
                &[],
                "[Three Musketeers]",
                Some(0),
            );
        }
    }
    #[test]
    fn dan_kanan_green_requirement() {
        for loaded in [false, true] {
            tamer_contribution(loaded, CardColor::Green, true);
        }
    }
    #[test]
    fn dan_kanan_no_yellow_requirement() {
        for loaded in [false, true] {
            tamer_contribution(loaded, CardColor::Yellow, false);
        }
    }
    #[test]
    fn sanmyojin_missing_blue_yaml() {
        option_colors(
            "EX12-070",
            false,
            &[CardColor::Yellow, CardColor::Green],
            None,
            false,
        );
    }
    #[test]
    fn sanmyojin_missing_blue_loaded() {
        option_colors(
            "EX12-070",
            true,
            &[CardColor::Yellow, CardColor::Green],
            None,
            false,
        );
    }
    #[test]
    fn sanmyojin_full_colors_or_use_req() {
        for loaded in [false, true] {
            option_colors(
                "EX12-070",
                loaded,
                &[CardColor::Yellow, CardColor::Green, CardColor::Blue],
                None,
                true,
            );
            option_colors("EX12-070", loaded, &[], Some("TB"), true);
        }
    }
}
mod batch2 {
    use super::*;
    color_test!(saneiketsu_yaml_colors, "EX12-071", false, Black, Blue, Red);
    color_test!(saneiketsu_loaded_colors, "EX12-071", true, Black, Blue, Red);
    #[test]
    fn saneiketsu_missing_red_yaml() {
        option_colors(
            "EX12-071",
            false,
            &[CardColor::Black, CardColor::Blue],
            None,
            false,
        );
    }
    #[test]
    fn saneiketsu_missing_red_loaded() {
        option_colors(
            "EX12-071",
            true,
            &[CardColor::Black, CardColor::Blue],
            None,
            false,
        );
    }
    #[test]
    fn saneiketsu_full_colors_or_use_req() {
        for loaded in [false, true] {
            option_colors(
                "EX12-071",
                loaded,
                &[CardColor::Black, CardColor::Blue, CardColor::Red],
                None,
                true,
            );
            option_colors("EX12-071", loaded, &[], Some("SW"), true);
        }
    }
}
