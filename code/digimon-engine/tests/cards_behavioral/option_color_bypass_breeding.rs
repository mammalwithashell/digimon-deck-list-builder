//! Printed "While you have a [X] Digimon or Tamer on the field, you can ignore
//! this card's color requirements" Options count a matching Digimon in the
//! BREEDING area.
//!
//! Sources:
//! - CR 4.3 3-4-6 ("The field is divided into the breeding area and the battle
//!   area") and 4-22-2: a Digi-Egg / Digimon in the breeding area is on the
//!   field.
//! - DCGO: every card below registers an `IgnoreColorConditionClass` whose
//!   `CanUseCondition` is `HasMatchConditionPermanent(<owner && (Digimon ||
//!   Tamer) && trait>, true)`; the trailing `true` is `isContainBreedingArea`
//!   and widens the scan to `GetFieldPermanents()` (battle + breeding).
//! - Official Q&A (BT24-100, `data/card_bundles/BT24-100.md`): "It refers to
//!   when you have the specified card in the battle area or breeding area."
//!
//! Defect this pins (replay cluster `yaml-condition-stricter-dcgo`): these YAMLs
//! gated the colour bypass on `any_permanent { zone: [battle_area] }`, so a
//! matching Digi-Egg that was only hatched into the breeding area left the Option
//! masked even though DCGO (and the printed text) allow it.
//!
//! Each case: the matching trait sits ONLY in the breeding area, in a colour that
//! does not satisfy the Option's own colour requirement, the battle area is empty
//! and memory is plentiful, so the only way the Option can be legal is the
//! printed colour bypass. The negative case uses the same egg without the trait.

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn egg(id: &str, color: CardColor, traits: &[&str]) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(2);
    card.dp = Some(2000);
    card.play_cost = 0;
    card.colors = vec![color];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn filler(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.colors = vec![CardColor::Black];
    card
}

fn option_colors(card_id: &str) -> Vec<CardColor> {
    let runner = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} YAML loads: {e:?}"))
        .start();
    runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} card data"))
        .colors
        .clone()
}

/// Mask bit for playing hand slot 0 of player 0, with a single Digi-Egg in the
/// breeding area carrying `egg_traits`.
fn breeding_egg_makes_option_playable(card_id: &str, egg_traits: &[&str]) -> bool {
    let own = option_colors(card_id);
    let off_color = [
        CardColor::Black,
        CardColor::Purple,
        CardColor::White,
        CardColor::Yellow,
        CardColor::Blue,
    ]
    .into_iter()
    .find(|c| !own.contains(c))
    .expect("an off-colour exists");

    let mut runner = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} YAML loads: {e:?}"))
        .add_card(egg("BREED-EGG", off_color, egg_traits))
        .add_card(filler("FILL"))
        .hand(0, &[card_id])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "BREED-EGG");
    assert!(
        runner.game.player(0).battle_area.is_empty(),
        "precondition: only the breeding area holds a permanent"
    );
    runner.game.enter_main_phase();
    build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize] == 1.0
}

macro_rules! breeding_bypass_case {
    ($name:ident, $card:literal, $trait:literal) => {
        #[test]
        fn $name() {
            assert!(
                breeding_egg_makes_option_playable($card, &[$trait]),
                concat!(
                    $card,
                    ": a [",
                    $trait,
                    "] Digimon in the breeding area is on the field (CR 4.3 3-4-6) \
                     and satisfies the printed colour bypass"
                )
            );
            assert!(
                !breeding_egg_makes_option_playable($card, &["Unrelated"]),
                concat!(
                    $card,
                    ": without the [",
                    $trait,
                    "] trait the off-colour egg must NOT unlock the Option"
                )
            );
        }
    };
}

breeding_bypass_case!(bt24_100_breeding_ts_digimon_unlocks_option, "BT24-100", "TS");
breeding_bypass_case!(bt24_091_breeding_ts_digimon_unlocks_option, "BT24-091", "TS");
breeding_bypass_case!(bt24_095_breeding_ts_digimon_unlocks_option, "BT24-095", "TS");
breeding_bypass_case!(bt24_097_breeding_ts_digimon_unlocks_option, "BT24-097", "TS");
breeding_bypass_case!(bt25_093_breeding_ts_digimon_unlocks_option, "BT25-093", "TS");
breeding_bypass_case!(bt25_100_breeding_ts_digimon_unlocks_option, "BT25-100", "TS");
breeding_bypass_case!(bt25_101_breeding_ts_digimon_unlocks_option, "BT25-101", "TS");
breeding_bypass_case!(bt26_101_breeding_ts_digimon_unlocks_option, "BT26-101", "TS");
breeding_bypass_case!(bt24_099_breeding_appmon_digimon_unlocks_option, "BT24-099", "Appmon");
breeding_bypass_case!(ex10_070_breeding_appmon_digimon_unlocks_option, "EX10-070", "Appmon");
breeding_bypass_case!(bt21_097_breeding_appmon_digimon_unlocks_option, "BT21-097", "Appmon");
breeding_bypass_case!(bt22_099_breeding_cs_digimon_unlocks_option, "BT22-099", "CS");
breeding_bypass_case!(bt23_096_breeding_cs_digimon_unlocks_option, "BT23-096", "CS");
breeding_bypass_case!(p_205_breeding_dm_digimon_unlocks_option, "P-205", "DM");

// ─── Battle-area control group: the printed bypass must work from HAND ────────
//
// `kind: flood_gate` + `IgnoreColorRequirement` is a FIELD-scoped declarative
// modifier; an Option still in the hand is only unlocked by a card-level
// `use_requirement` (the mask's `option_color_requirement_bypass`, see
// `option_use_requirement_or_color_available`). These cards must therefore carry
// a `use_requirement` mirroring their gate, or the printed colour bypass never
// applies from hand at all.

fn permanent_makes_option_playable(card_id: &str, kind: CardKind, traits: &[&str]) -> bool {
    let own = option_colors(card_id);
    let off_color = [
        CardColor::Black,
        CardColor::Purple,
        CardColor::White,
        CardColor::Yellow,
        CardColor::Blue,
    ]
    .into_iter()
    .find(|c| !own.contains(c))
    .expect("an off-colour exists");
    let mut src = egg("BOARD-SRC", off_color, traits);
    src.card_kind = kind;
    if kind == CardKind::Tamer {
        src.level = None;
        src.dp = None;
    }
    let mut runner = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} YAML loads: {e:?}"))
        .add_card(src)
        .add_card(filler("FILL"))
        .hand(0, &[card_id])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    runner.place_on_field(0, "BOARD-SRC", Some(0));
    runner.game.enter_main_phase();
    build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize] == 1.0
}

macro_rules! board_bypass_case {
    ($name:ident, $card:literal, $kind:expr, $trait:expr) => {
        #[test]
        fn $name() {
            assert!(
                permanent_makes_option_playable($card, $kind, &[$trait]),
                concat!($card, ": the printed colour bypass must unlock the Option from hand")
            );
            assert!(
                !permanent_makes_option_playable($card, $kind, &["Unrelated"]),
                concat!($card, ": an off-colour, off-trait board source must not unlock it")
            );
        }
    };
}

board_bypass_case!(bt22_099_battle_cs_digimon_unlocks_option_from_hand, "BT22-099", CardKind::Digimon, "CS");
board_bypass_case!(bt22_099_battle_cs_tamer_unlocks_option_from_hand, "BT22-099", CardKind::Tamer, "CS");
board_bypass_case!(bt23_096_battle_cs_digimon_unlocks_option_from_hand, "BT23-096", CardKind::Digimon, "CS");
board_bypass_case!(bt23_096_battle_cs_tamer_unlocks_option_from_hand, "BT23-096", CardKind::Tamer, "CS");

/// ST22-11 "While you have a Tamer": any Tamer, no trait requirement.
#[test]
fn st22_11_any_tamer_unlocks_option_from_hand() {
    assert!(
        permanent_makes_option_playable("ST22-11", CardKind::Tamer, &[]),
        "ST22-11: an off-colour Tamer on the field unlocks the Option from hand"
    );
    assert!(
        !permanent_makes_option_playable("ST22-11", CardKind::Digimon, &[]),
        "ST22-11: an off-colour Digimon is not a Tamer and does not unlock it"
    );
}

/// BT19-093 "While you don't have [Queen Device] in the battle area, you may
/// ignore this card's color requirements" (DCGO negates
/// `HasMatchConditionOwnersPermanent(EqualsCardName("Queen Device"))`).
#[test]
fn bt19_093_no_queen_device_unlocks_option_from_hand() {
    let own = option_colors("BT19-093");
    let off_color = [
        CardColor::Black,
        CardColor::Purple,
        CardColor::White,
        CardColor::Yellow,
        CardColor::Blue,
        CardColor::Green,
    ]
    .into_iter()
    .find(|c| !own.contains(c))
    .expect("an off-colour exists");

    let runner_with = |name: &str| {
        let mut src = egg("BOARD-SRC", off_color, &[]);
        src.card_name = name.to_string();
        let mut runner = DebugRunner::builder()
            .dsl_card("BT19-093")
            .expect("BT19-093 YAML loads")
            .add_card(src)
            .add_card(filler("FILL"))
            .hand(0, &["BT19-093"])
            .deck(0, &["FILL"; 6])
            .deck(1, &["FILL"; 6])
            .memory(10)
            .start();
        runner.place_on_field(0, "BOARD-SRC", Some(0));
        runner.game.enter_main_phase();
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize] == 1.0
    };

    assert!(
        runner_with("Some Other Card"),
        "no [Queen Device] in the battle area: the colour bypass applies"
    );
    assert!(
        !runner_with("Queen Device"),
        "a [Queen Device] in the battle area withdraws the bypass (off-colour board)"
    );
}
