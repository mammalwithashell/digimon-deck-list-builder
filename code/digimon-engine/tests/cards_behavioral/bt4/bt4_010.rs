//! BT4-010 Fugamon: native definition and ordinary vanilla behavior.
//!
//! Printed authority: official observation
//! 2fe740a6e814d7b563931e0a8699c2826bbdbadef0e9530df3ec1d0753f768b3,
//! checked against BT4-010.webp in the 2026-10-13 witness plan.
//! The pinned R4 fixture preserves the printed red Lv.3 -> cost 1 route.
//! Rules: general_rule.pdf sections 7-1-3, 8-1-2/3, 11-2, 13-1 and 14
//! (ordinary play, digivolution/draw, attack eligibility and DP battles).

use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, ATTACK_END, ATTACK_START, BREEDING_TARGET,
    SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

const CID: &str = "BT4-010";
const FILLER: &str = "TEST-BT4-010-FILLER";
const RED_LV3: &str = "TEST-BT4-010-RED-LV3";
const BLUE_LV3: &str = "TEST-BT4-010-BLUE-LV3";
const RED_LV2: &str = "TEST-BT4-010-RED-LV2";
const RED_LV4: &str = "TEST-BT4-010-RED-LV4";
const HOST: &str = "TEST-BT4-010-HOST";
const OPPONENT: &str = "TEST-BT4-010-OPPONENT";

fn fixture(id: &str, level: u8, dp: i32) -> CardData {
    // Synthetic supporting cards only; never substitute for Fugamon.
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn runner(player: u8, opposing_dp: i32) -> DebugRunner {
    let data = CardData::load_from_str(include_str!("../../fixtures/focused_five_vanilla_r4.json"))
        .expect("pinned R4 card rows");
    let mut blue_lv3 = fixture(BLUE_LV3, 3, 1000);
    blue_lv3.colors = vec![CardColor::Blue];
    let mut r = DebugRunner::builder()
        .dsl_card(CID)
        .expect("BT4-010 must exist in the native embedded DSL pack")
        // AFTER dsl_card: inferred metadata must not supply or alter the
        // printed ordinary digivolution route under test.
        .with_card_data(data)
        .add_card(fixture(FILLER, 3, 1000))
        .add_card(fixture(RED_LV3, 3, 1000))
        .add_card(blue_lv3)
        .add_card(fixture(RED_LV2, 2, 1000))
        .add_card(fixture(RED_LV4, 4, 1000))
        .add_card(fixture(HOST, 5, 6000))
        .add_card(fixture(OPPONENT, 3, opposing_dp))
        .deck(0, &[FILLER; 8])
        .deck(1, &[FILLER; 8])
        .security(0, &[FILLER; 3])
        .security(1, &[FILLER; 3])
        .build();
    r.set_first_player(player);
    r.set_turn(2);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(8);
    r
}

fn act(r: &mut DebugRunner, player: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
}

#[test]
fn bt4_010_native_definition_matches_printed_vanilla_metadata() {
    let r = runner(0, 3000);
    let card = r.compiled_card(CID).expect("native definition");
    assert_eq!(card.card, CID);
    assert_eq!(card.name, "Fugamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.all_traits(), ["Champion", "Virus", "Demon"]);
    assert!(
        card.effects.is_empty(),
        "no main, inherited, security or keyword clauses"
    );
    assert!(
        card.alt_paths.is_empty(),
        "only an ordinary printed digivolution circle"
    );
    assert!(card.identity.is_none());
    assert!(card.also_treated_as.is_empty());
    let data = r.game.card_data.iter().find(|c| c.card_id == CID).unwrap();
    assert_eq!(data.card_kind, CardKind::Digimon);
    assert_eq!(data.traits, ["Champion", "Virus", "Demon"]);
    assert_eq!(
        data.evo_costs,
        vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 1
        }]
    );
    assert!(data.effect_text.is_empty());
    assert!(data.inherited_text.is_empty());
    assert!(data.security_text.is_empty());
    assert!(data.keywords.is_empty());
}

#[test]
fn bt4_010_masked_play_pays_three_without_prompt_draw_or_immediate_attack() {
    for player in 0..2 {
        let mut r = runner(player, 3000);
        let hand = r.add_to_hand(player, CID);
        let deck_before = r.deck_size(player);
        act(&mut r, player, hand as u16);
        assert_eq!(r.memory(), 5);
        assert_eq!(r.hand_size(player), 0);
        assert_eq!(r.deck_size(player), deck_before);
        assert_eq!(r.battle_area_size(player), 1);
        assert_eq!(r.dp_of(r.perm_handle(player, 0)), Some(3000));
        assert!(r.pending_selection().is_none());
        let mask = build_action_mask(&r.game, player);
        assert!(mask[ATTACK_START as usize..ATTACK_END as usize]
            .iter()
            .all(|&v| v == 0.0));
    }
}

#[test]
fn bt4_010_ordinary_evolution_requires_red_level_three_pays_one_and_draws_one() {
    for player in 0..2 {
        for target in [0, BREEDING_TARGET] {
            for (base, legal) in [
                (RED_LV3, true),
                (BLUE_LV3, false),
                (RED_LV2, false),
                (RED_LV4, false),
            ] {
                let mut r = runner(player, 3000);
                let hand = r.add_to_hand(player, CID);
                if target == BREEDING_TARGET {
                    r.place_in_breeding(player, base);
                } else {
                    r.place_on_field(player, base, Some(0));
                }
                let action = encode_digivolve(hand as u16, target);
                assert_eq!(
                    build_action_mask(&r.game, player)[action as usize],
                    if legal { 1.0 } else { 0.0 }
                );
                if !legal {
                    continue;
                }
                let deck_before = r.deck_size(player);
                act(&mut r, player, action);
                assert_eq!(r.memory(), 7, "printed evolution cost is one");
                assert_eq!(
                    r.hand_size(player),
                    1,
                    "exactly the ordinary evolution draw"
                );
                assert_eq!(r.deck_size(player), deck_before - 1);
                let state = &r.game.players[player as usize];
                let stack = if target == BREEDING_TARGET {
                    state.breeding_area.as_ref().unwrap()
                } else {
                    &state.battle_area[0]
                };
                assert_eq!(stack.top_card().card_id(&r.game.card_data), CID);
                assert_eq!(stack.card_sources.len(), 2);
                assert_eq!(stack.card_sources[0].card_id(&r.game.card_data), RED_LV3);
                assert!(r.pending_selection().is_none());
            }
        }
    }
}

#[test]
fn bt4_010_two_legal_evolution_stacks_remain_independent_masked_choices() {
    for player in 0..2 {
        for chosen in 0..2 {
            let mut r = runner(player, 3000);
            let hand = r.add_to_hand(player, CID);
            r.place_on_field(player, RED_LV3, Some(0));
            r.place_on_field(player, RED_LV3, Some(0));
            let mask = build_action_mask(&r.game, player);
            for target in 0..2 {
                assert_eq!(mask[encode_digivolve(hand as u16, target) as usize], 1.0);
            }
            act(&mut r, player, encode_digivolve(hand as u16, chosen));
            for target in 0..2 {
                let stack = &r.game.players[player as usize].battle_area[target];
                assert_eq!(
                    stack.top_card().card_id(&r.game.card_data),
                    if target == chosen as usize {
                        CID
                    } else {
                        RED_LV3
                    }
                );
                assert_eq!(
                    stack.card_sources.len(),
                    if target == chosen as usize { 2 } else { 1 }
                );
            }
            assert_eq!(r.memory(), 7);
            assert_eq!(r.hand_size(player), 1);
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt4_010_battle_uses_three_thousand_dp_without_piercing_or_retaliation() {
    for player in 0..2 {
        for opposing_dp in [2000, 3000, 4000] {
            let opponent = 1 - player;
            let mut r = runner(player, opposing_dp);
            r.place_on_field(player, CID, Some(0));
            r.place_field_stack(opponent, &[OPPONENT], true, 0);
            act(&mut r, player, encode_attack(0, 0));
            assert!(r.pending_selection().is_none());
            assert_eq!(r.battle_area_size(player), usize::from(opposing_dp < 3000));
            assert_eq!(
                r.battle_area_size(opponent),
                usize::from(opposing_dp > 3000)
            );
            assert_eq!(r.security_count(opponent), 3, "no unprinted Piercing");
            assert_eq!(r.memory(), 8);
        }
    }
}

#[test]
fn bt4_010_security_battle_uses_printed_dp_without_security_effect() {
    for player in 0..2 {
        for attacking_dp in [2000, 3000, 4000] {
            let defender = 1 - player;
            let mut r = runner(player, attacking_dp);
            r.game
                .stage_inject_card(defender, CID, "security_top")
                .unwrap();
            r.place_on_field(player, OPPONENT, Some(0));
            act(&mut r, player, encode_attack(0, SECURITY_TARGET));
            assert!(r.pending_selection().is_none());
            assert_eq!(r.battle_area_size(player), usize::from(attacking_dp > 3000));
            assert_eq!(r.battle_area_size(defender), 0);
            assert_eq!(r.security_count(defender), 3, "exactly one check");
            assert_eq!(r.trash_size(defender), 1);
            assert_eq!(
                r.game.players[defender as usize].trash[0].card_id(&r.game.card_data),
                CID
            );
            assert_eq!(r.hand_size(defender), 0);
            assert_eq!(r.memory(), 8);
        }
    }
}

#[test]
fn bt4_010_as_source_grants_no_dp_keyword_or_attack_trigger() {
    for player in 0..2 {
        let mut r = runner(player, 3000);
        let host = r.place_stack(player, &[CID, HOST]);
        r.game.tick_declarative_effects();
        assert_eq!(r.dp_of(host), Some(6000));
        for keyword in [
            Keyword::Rush,
            Keyword::Piercing,
            Keyword::Blocker,
            Keyword::SecurityAttackPlus(1),
        ] {
            assert!(!r.game.has_keyword(host, keyword));
        }
        let deck_before = r.deck_size(player);
        act(&mut r, player, encode_attack(0, SECURITY_TARGET));
        assert!(r.pending_selection().is_none());
        assert_eq!(r.security_count(1 - player), 2);
        assert_eq!(r.deck_size(player), deck_before);
        assert_eq!(r.hand_size(player), 0);
        assert_eq!(r.memory(), 8);
    }
}
