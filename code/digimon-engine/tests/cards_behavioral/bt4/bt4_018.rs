//! BT4-018 Spinomon: official observation 76036f48a652cd420d6036d46e6c8187bbdbd5a05d5decc18497af9accdb96bb,
//! checked against the actual card image. R4 CardData pins the ordinary red Lv5/cost2 route.
//! Rules: general_rule.pdf 15-8-2-1..3 (pages 24-25), 4-2-4 (inherited effects),
//! 7-1/8-1 (play/evolution), 13-1/14 (security and DP battle).

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, ATTACK_END, ATTACK_START, BREEDING_TARGET,
    SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase, Keyword};

const CID: &str = "BT4-018";
const FILLER: &str = "TEST-BT4-018-FILLER";
const RED5: &str = "TEST-BT4-018-RED5";
const BLUE5: &str = "TEST-BT4-018-BLUE5";
const RED4: &str = "TEST-BT4-018-RED4";
const RED6: &str = "TEST-BT4-018-RED6";
const HOST: &str = "TEST-BT4-018-HOST";
const OPP: &str = "TEST-BT4-018-OPP";

fn fixture(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn runner(player: u8, opponent_dp: i32) -> DebugRunner {
    let data = CardData::load_from_str(include_str!(
        "../../fixtures/focused_remaining_effects_r4.json"
    ))
    .expect("pinned R4 rows");
    let mut blue = fixture(BLUE5, 5, 1000);
    blue.colors = vec![CardColor::Blue];
    let mut r = DebugRunner::builder()
        .dsl_card(CID)
        .expect("BT4-018 must exist in native embedded DSL pack")
        // Load authoritative ordinary routes AFTER native DSL registration.
        .with_card_data(data)
        .add_card(fixture(FILLER, 3, 1000))
        .add_card(fixture(RED5, 5, 1000))
        .add_card(blue)
        .add_card(fixture(RED4, 4, 1000))
        .add_card(fixture(RED6, 6, 1000))
        .add_card(fixture(HOST, 7, 14000))
        .add_card(fixture(OPP, 6, opponent_dp))
        .deck(0, &[FILLER; 12])
        .deck(1, &[FILLER; 12])
        .security(0, &[FILLER; 3])
        .security(1, &[FILLER; 3])
        .build();
    r.set_first_player(player);
    r.set_turn(2);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(10);
    r
}

fn act(r: &mut DebugRunner, player: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
}

#[test]
fn bt4_018_native_definition_matches_printed_data_and_single_face_up_aura() {
    let r = runner(0, 12000);
    let card = r.compiled_card(CID).unwrap();
    assert_eq!(card.name, "Spinomon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.color, [CompiledColor::Red]);
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(10000));
    assert_eq!(card.all_traits(), ["Mega", "Virus", "Dinosaur"]);
    assert!(card.alt_paths.is_empty(), "ordinary printed route only");
    assert_eq!(
        card.effects.len(),
        1,
        "no unprinted inherited/security/triggered effects"
    );
    match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            target,
            active_when,
            dp_modifier,
            grant_keyword,
            security_attack,
            modifier,
            ..
        }) => {
            assert_eq!(*scope, CompiledScope::FaceUp);
            assert_eq!(*target, Default::default());
            assert_eq!(active_when.as_ref().unwrap().your_turn, Some(true));
            assert_eq!(*dp_modifier, Some(3000));
            assert!(grant_keyword.is_none() && security_attack.is_none() && modifier.is_none());
        }
        _ => panic!("single persistent self DP aura required"),
    }
    let data = r.game.card_data.iter().find(|c| c.card_id == CID).unwrap();
    assert_eq!(
        data.evo_costs,
        [EvoCost {
            card_color: 0,
            level: 5,
            memory_cost: 2
        }]
    );
    assert!(
        data.inherited_text.is_empty() && data.security_text.is_empty() && data.keywords.is_empty()
    );
}

#[test]
fn bt4_018_own_turn_aura_tracks_repeated_real_turn_boundaries_for_both_seats() {
    for owner in 0..2 {
        let mut r = runner(owner, 12000);
        let spinomon = r.place_on_field(owner, CID, Some(0));
        let own_other = r.place_on_field(owner, FILLER, Some(0));
        let opponent_other = r.place_on_field(1 - owner, OPP, Some(0));
        for transition in 0..5 {
            assert_eq!(
                r.effective_dp(spinomon),
                Some(if transition % 2 == 0 { 13000 } else { 10000 })
            );
            assert_eq!(r.effective_dp(own_other), Some(1000));
            assert_eq!(r.effective_dp(opponent_other), Some(12000));
            assert!(r.pending_selection().is_none());
            r.end_turn();
        }
    }
}

#[test]
fn bt4_018_masked_play_pays_ten_and_gains_aura_without_draw_prompt_or_rush() {
    for player in 0..2 {
        let mut r = runner(player, 12000);
        let hand = r.add_to_hand(player, CID);
        let deck = r.deck_size(player);
        act(&mut r, player, hand as u16);
        assert_eq!(r.memory(), 0);
        assert_eq!(r.hand_size(player), 0);
        assert_eq!(r.deck_size(player), deck);
        assert_eq!(r.effective_dp(r.perm_handle(player, 0)), Some(13000));
        assert!(r.pending_selection().is_none());
        assert!(
            build_action_mask(&r.game, player)[ATTACK_START as usize..ATTACK_END as usize]
                .iter()
                .all(|&v| v == 0.0)
        );
    }
}

#[test]
fn bt4_018_ordinary_masked_evolution_requires_red_five_pays_two_draws_one() {
    for player in 0..2 {
        for target in [0, BREEDING_TARGET] {
            for (base, legal) in [(RED5, true), (BLUE5, false), (RED4, false), (RED6, false)] {
                let mut r = runner(player, 12000);
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
                let deck = r.deck_size(player);
                act(&mut r, player, action);
                assert_eq!(r.memory(), 8);
                assert_eq!(r.hand_size(player), 1);
                assert_eq!(r.deck_size(player), deck - 1);
                let p = &r.game.players[player as usize];
                let stack = if target == BREEDING_TARGET {
                    p.breeding_area.as_ref().unwrap()
                } else {
                    &p.battle_area[0]
                };
                assert_eq!(stack.top_card().card_id(&r.game.card_data), CID);
                assert_eq!(stack.card_sources.len(), 2);
                if target != BREEDING_TARGET {
                    assert_eq!(r.effective_dp(r.perm_handle(player, 0)), Some(13000));
                }
                assert!(r.pending_selection().is_none());
            }
        }
    }
}

#[test]
fn bt4_018_two_legal_evolution_targets_remain_independent_choices() {
    for player in 0..2 {
        for chosen in 0..2 {
            let mut r = runner(player, 12000);
            let hand = r.add_to_hand(player, CID);
            r.place_on_field(player, RED5, Some(0));
            r.place_on_field(player, RED5, Some(0));
            for target in 0..2 {
                assert_eq!(
                    build_action_mask(&r.game, player)
                        [encode_digivolve(hand as u16, target) as usize],
                    1.0
                );
            }
            act(&mut r, player, encode_digivolve(hand as u16, chosen));
            for target in 0..2 {
                assert_eq!(
                    r.game.players[player as usize].battle_area[target]
                        .top_card()
                        .card_id(&r.game.card_data),
                    if target == chosen as usize { CID } else { RED5 }
                );
            }
        }
    }
}

#[test]
fn bt4_018_live_dp_wins_own_turn_battle_but_loses_opponent_turn_battle() {
    for owner in 0..2 {
        let mut r = runner(owner, 12000);
        r.place_on_field(owner, CID, Some(0));
        r.place_field_stack(1 - owner, &[OPP], true, 0);
        act(&mut r, owner, encode_attack(0, 0));
        assert_eq!(r.battle_area_size(owner), 1);
        assert_eq!(r.battle_area_size(1 - owner), 0);
        assert_eq!(r.security_count(1 - owner), 3, "no unprinted Piercing");
        let mut r = runner(1 - owner, 12000);
        r.place_on_field(1 - owner, OPP, Some(0));
        r.place_field_stack(owner, &[CID], true, 0);
        act(&mut r, 1 - owner, encode_attack(0, 0));
        assert_eq!(r.battle_area_size(owner), 0);
        assert_eq!(r.battle_area_size(1 - owner), 1);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn bt4_018_security_battle_uses_printed_ten_thousand_without_security_effect() {
    for player in 0..2 {
        let mut r = runner(player, 12000);
        r.game
            .stage_inject_card(1 - player, CID, "security_top")
            .unwrap();
        r.place_on_field(player, OPP, Some(0));
        act(&mut r, player, encode_attack(0, SECURITY_TARGET));
        assert_eq!(r.battle_area_size(player), 1);
        assert_eq!(r.battle_area_size(1 - player), 0);
        assert_eq!(r.security_count(1 - player), 3);
        assert_eq!(r.trash_size(1 - player), 1);
        assert_eq!(r.hand_size(1 - player), 0);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn bt4_018_breeding_is_inactive_and_move_to_battle_activates_self_aura() {
    for player in 0..2 {
        let mut r = runner(player, 12000);
        r.place_in_breeding(player, CID);
        let other = r.place_on_field(player, FILLER, Some(0));
        r.game.tick_declarative_effects();
        assert_eq!(r.effective_dp(other), Some(1000));
        assert_eq!(
            r.game
                .static_dp_aura_bonus(digimon_engine::permanent::PermanentHandle {
                    player,
                    index: BREEDING_TARGET as u8
                }),
            0
        );
        r.set_phase(GamePhase::Breeding);
        act(&mut r, player, 61);
        assert_eq!(r.effective_dp(r.perm_handle(player, 1)), Some(13000));
    }
}

#[test]
fn bt4_018_buried_source_grants_no_aura_keyword_or_attack_trigger() {
    for player in 0..2 {
        let mut r = runner(player, 12000);
        let host = r.place_stack(player, &[CID, HOST]);
        assert_eq!(r.effective_dp(host), Some(14000));
        assert_eq!(r.game.source_dp_contribution(host, 0), 0);
        for keyword in [
            Keyword::Rush,
            Keyword::Piercing,
            Keyword::Blocker,
            Keyword::SecurityAttackPlus(1),
        ] {
            assert!(!r.game.has_keyword(host, keyword));
        }
        let deck = r.deck_size(player);
        act(&mut r, player, encode_attack(0, SECURITY_TARGET));
        assert_eq!(r.security_count(1 - player), 2);
        assert_eq!(r.deck_size(player), deck);
        assert_eq!(r.hand_size(player), 0);
        assert_eq!(r.memory(), 10);
        assert!(r.pending_selection().is_none());
    }
}
