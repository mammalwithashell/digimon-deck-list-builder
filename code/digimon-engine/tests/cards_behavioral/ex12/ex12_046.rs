use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::selection::SelectionKind;

use super::support::{
    digimon, field_contains, fire_end_of_attack, fire_opponent_security_removed, hand_contains,
    hand_index, plain_digimon, select_first_non_pass, select_hand_card, tb_digimon, top_card_id,
    with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-046";

/// Walk the prompts (accepting each non-hand prompt with its first non-PASS
/// action) to the hand pick and return the card ids it offers, sorted. Empty
/// when no hand pick installs.
fn offered_hand_ids(runner: &mut DebugRunner, player: u8) -> Vec<String> {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[player as usize]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut ids: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&a| a != PASS)
                .map(|a| {
                    let slot = if a < PLAY_HAND_END { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
                    hand[slot as usize].clone()
                })
                .collect();
            ids.sort();
            return ids;
        }
        let Some(action) = view.valid_action_ids.iter().copied().find(|&a| a != PASS) else {
            return Vec::new();
        };
        runner.execute_action(view.selecting_player, action).expect("advance prompt");
    }
    panic!("never reached the hand pick");
}

/// Shishimamon (EX12-046) under a carrier (the inherited [End of Attack]),
/// `memory` memory, `hand` in hand.
fn end_of_attack_runner(
    extra: Vec<CardData>,
    hand: &[&str],
    memory: i16,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Yellow, 3, 3000));
    for card in extra {
        builder = builder.add_card(card);
    }
    let mut runner = builder.hand(0, hand).memory(memory).start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    (runner, carrier)
}

// ── Inherited [End of Attack][OPT] free play: candidate legality ─────────────
// DCGO EX12_046.cs:203-210 `CanSelectCardCondition`: IsDigimon, DP <= 5000,
// [TB], AND `CardEffectCommons.CanPlayAsNewPermanent(cardSource, false,
// activateClass)` (GameContextDeterminarion.cs:419-432): not an Option (never a
// DUAL, CR 4.3 4-6-2), an empty battle-area frame, no play prohibition. A card
// the free play would refuse is not a legal choice (CR 4.3 1-3-2).

/// "You can't play Digimon by effects": no [TB] Digimon can be played, so none
/// is offered.
#[test]
fn ex12_046_end_of_attack_pick_offers_nothing_under_effect_play_prohibition() {
    let (mut runner, carrier) =
        end_of_attack_runner(vec![tb_digimon("TB-LOW", CardColor::Yellow, 4, 5000)], &["TB-LOW"], 10);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    fire_end_of_attack(&mut runner, carrier);
    assert!(offered_hand_ids(&mut runner, 0).is_empty());
    assert!(hand_contains(&runner, 0, "TB-LOW"));
}

/// A full battle area: no [TB] Digimon can be played, so none is offered.
#[test]
fn ex12_046_end_of_attack_pick_offers_nothing_when_battle_area_full() {
    let (mut runner, carrier) =
        end_of_attack_runner(vec![tb_digimon("TB-LOW", CardColor::Yellow, 4, 5000)], &["TB-LOW"], 10);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    fire_end_of_attack(&mut runner, carrier);
    assert!(offered_hand_ids(&mut runner, 0).is_empty());
    assert!(hand_contains(&runner, 0, "TB-LOW"));
}

/// The play is free ("without paying the cost"), so memory never gates it: a
/// cost-20 [TB] Digimon is offered with 0 memory. Regression guard against a
/// memory check on a free play.
#[test]
fn ex12_046_end_of_attack_pick_ignores_memory_for_free_play() {
    let pricey = digimon("TB-PRICEY", CardColor::Yellow, 4, 20, 5000, &["TB"]);
    let (mut runner, carrier) = end_of_attack_runner(vec![pricey], &["TB-PRICEY"], 0);
    fire_end_of_attack(&mut runner, carrier);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["TB-PRICEY".to_string()]);
}

#[test]
fn ex12_046_security_removed_effect_is_not_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-046");
    let effect = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered
                    .when
                    .contains(&CompiledTiming::OnOpponentSecurityRemoved) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed opponent-security-removed effect");

    assert!(
        !effect.once_per_turn,
        "printed effect has no [Once Per Turn] tag"
    );
}

#[test]
fn ex12_046_on_play_gives_security_attack_minus_and_dp_minus_until_opponent_turn_end() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Yellow, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-046");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish debuff");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "target gains Security Attack -1"
    );
    assert_eq!(runner.effective_dp(opp), Some(2000));
}

#[test]
fn ex12_046_security_removed_may_digivolve_into_tb_from_hand_cost_reduced_by_two() {
    let tb_lv6 = with_evo_cost(
        tb_digimon("TB-LV6", CardColor::Yellow, 6, 11000),
        CardColor::Yellow,
        5,
        4,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(tb_lv6)
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-LV6"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    fire_opponent_security_removed(&mut runner, 1);
    select_hand_card(&mut runner, 0, "TB-LV6");
    runner.auto_resolve().expect("finish effect digivolve");

    assert_eq!(top_card_id(&runner, 0, 0), "TB-LV6");
    assert_eq!(
        runner.memory(),
        3,
        "printed cost 4 is reduced by 2 and paid"
    );
}

#[test]
fn ex12_046_inherited_end_of_attack_plays_low_dp_tb_from_hand_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .add_card(tb_digimon("TB-LOW", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-LOW"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    fire_end_of_attack(&mut runner, carrier);
    select_hand_card(&mut runner, 0, "TB-LOW");
    runner.auto_resolve().expect("finish inherited play");

    assert!(field_contains(&runner, 0, "TB-LOW"));
    assert_eq!(runner.memory(), 10, "the inherited play is free");
}

/// §8-1-3-3: "The player places the Digimon for the digivolution on top of the
/// chosen card, **draws 1 card**, and the digivolution process is resolved."
///
/// The draw is part of the digivolution PROCEDURE, so it applies to an
/// effect-initiated digivolve exactly as it does to the main-phase action --
/// the engine used to skip it on the effect path, reasoning that the draw was
/// "a player-action benefit, not an effect mechanic". §8-1-2-8 settles it the
/// other way: digivolution is possible when a draw isn't, and only then "is
/// performed without drawing a card", which presumes the draw otherwise.
///
/// Found by the DCGO card-clause exam (`qa/dcgo-exams/EX12/EX12-046-effect2.yaml`):
/// DCGO's hand held one more card than ours from this digivolve onward.
#[test]
fn ex12_046_effect_initiated_digivolve_still_draws_the_rule_8_1_3_3_card() {
    let tb_lv6 = with_evo_cost(
        tb_digimon("TB-LV6", CardColor::Yellow, 6, 11000),
        CardColor::Yellow,
        5,
        4,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(tb_lv6)
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("DECKTOP", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-LV6"])
        .deck(0, &["DECKTOP"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    let deck_before = runner.deck_size(0);

    fire_opponent_security_removed(&mut runner, 1);
    select_hand_card(&mut runner, 0, "TB-LV6");
    runner.auto_resolve().expect("finish effect digivolve");

    assert_eq!(
        top_card_id(&runner, 0, 0),
        "TB-LV6",
        "the digivolve resolved"
    );
    // Hand: TB-LV6 left it (-1) and the §8-1-3-3 draw replaced it (+1).
    assert_eq!(
        runner.hand_size(0),
        1,
        "digivolving from hand draws 1, so the hand returns to its prior size"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the drawn card came off the deck"
    );
}

/// DCGO's `DigivolveIntoHandOrTrashCard` offers only hand cards that can
/// actually digivolve onto this Digimon at the reduced cost, and opens no pick
/// when none can (the recording then has no row for it). A [TB] Digimon in hand
/// that has no digivolution route onto this Lv.5 must therefore not park a prompt.
#[test]
fn ex12_046_security_removed_offers_no_pick_when_no_tb_in_hand_can_digivolve() {
    // Lv.5 [TB] whose only route is from a Lv.4: it cannot go onto the Lv.5 EX12-046.
    let needs_lv4 = with_evo_cost(
        tb_digimon("NEEDS-LV4", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        4,
        3,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(needs_lv4)
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["NEEDS-LV4"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    fire_opponent_security_removed(&mut runner, 1);

    assert!(
        runner.pending_selection().is_none(),
        "no [TB] card in hand can digivolve onto EX12-046, so no pick is parked: {:?}",
        runner.pending_selection()
    );
    assert_eq!(top_card_id(&runner, 0, 0), CARD_ID);
}

/// Kaguyamon (EX12-065) prints Purple/Green Lv.5 circles (cost 4) and the black
/// text "[Digivolve] Lv.5 w/[Puppet]/[Shambala] trait: Cost 3". Onto the
/// Yellow/Red Shishimamon (EX12-046) only the trait route applies, so DCGO's
/// `CostList` holds just {3} and `DigivolveIntoHandOrTrashCard`'s reduce-by-2
/// makes it cost 1 (CR 4.3 §8-1-2: the digivolution cost is the cost of the
/// condition the card is digivolving by; a cost reduction lowers that cost).
///
/// Pinned from DCGO PvP game raw-20261113T025510Z#g4 (turn 5): DCGO declined
/// Genshi Continent & Ashino Island (EX12-074)'s "-1" digivolve and took this
/// "-2" one after the security check, paying 1. The replay applied the
/// Kaguyamon pick to EX12-074's prompt instead (cost 2), so Toho Braves started
/// turn 9 with 4 memory instead of DCGO's 5. The engine's cost here was already
/// right; this test guards the 1-memory figure the replay evidence relies on.
#[test]
fn ex12_046_security_removed_digivolve_into_kaguyamon_uses_the_shambala_route_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .dsl_card("EX12-065")
        .expect("EX12-065 YAML loads")
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("DECKTOP", CardColor::Yellow, 3, 3000))
        .hand(0, &["EX12-065"])
        .deck(0, &["DECKTOP"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    fire_opponent_security_removed(&mut runner, 1);
    select_hand_card(&mut runner, 0, "EX12-065");
    runner.auto_resolve().expect("finish effect digivolve");

    assert_eq!(top_card_id(&runner, 0, 0), "EX12-065", "the digivolve resolved");
    assert_eq!(
        runner.memory(),
        4,
        "Shambala route cost 3, reduced by 2, pays 1 (not the Purple/Green circle's 4)"
    );
}
