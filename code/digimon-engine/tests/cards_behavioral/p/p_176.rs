//! P-176 Dorimon: inspected card image and official Bandai bundle.
//! Inherited [When Attacking] [Once Per Turn] This Digimon may digivolve
//! into a Digimon card with the [Chronicle] trait in the hand.
//! DCGO reference: Assets/Scripts/CardEffect/P/Black/P_176.cs.
//! CR 4.3 8-1-3: legal requirement, selected cost, stacking and bonus draw.
//! CR 4.3 15-14-1: optional activation, physical-copy OPT and turn reset.
//! Patterns: inherited attack trigger, optional hand choice, paid evolution.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET},
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "P-176";
const HOST: &str = "BT20-048";
const RAPTOR: &str = "BT20-051";
const GINRYU: &str = "BT20-012";

// Adversarial target metadata isolates the printed filters and subsequent
// OPT opportunity; the primary success cases use actual Chronicle cards.
fn candidate(
    id: &str,
    chronicle: bool,
    from_color: CardColor,
    from_level: u8,
    cost: u16,
) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Black];
    c.level = Some(from_level + 1);
    c.dp = Some(8000);
    c.traits = if chronicle {
        vec!["Chronicle".into()]
    } else {
        vec!["Chronicle Variant".into()]
    };
    c.evo_costs = vec![EvoCost {
        card_color: from_color as u8,
        level: from_level,
        memory_cost: cost,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut filler = make_test_card("FILL", "Harmless security Option");
    filler.card_kind = CardKind::Option;
    filler.level = None;
    filler.dp = None;
    let mut option = candidate("NOT-DIGIMON", true, CardColor::Black, 3, 2);
    option.card_kind = CardKind::Option;
    option.level = None;
    option.dp = None;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("P-176 must be implemented")
        .dsl_card(HOST)
        .unwrap()
        .dsl_card(RAPTOR)
        .unwrap()
        .dsl_card(GINRYU)
        .unwrap()
        .add_card(filler)
        .add_card(candidate("WRONG-TRAIT", false, CardColor::Black, 3, 2))
        .add_card(candidate("WRONG-COLOR", true, CardColor::Blue, 3, 2))
        .add_card(candidate("WRONG-LEVEL", true, CardColor::Black, 5, 2))
        .add_card(candidate("NEXT", true, CardColor::Black, 4, 3))
        .add_card(candidate("UNPAYABLE", true, CardColor::Black, 3, 19))
        .add_card(option)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(8)
}

fn setup(owner: u8, hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().hand(owner, hand).start();
    r.skip_mulligan();
    r.set_first_player(owner);
    let host = r.place_stack(owner, &[ID, HOST]);
    (r, host)
}

fn choose(r: &mut DebugRunner, action: u16) {
    let view = r.pending_selection_view().expect("player-visible choice");
    assert_eq!(
        build_action_mask(&r.game, view.selecting_player)[action as usize],
        1.0
    );
    r.execute_action(view.selecting_player, action).unwrap();
}

fn attack(r: &mut DebugRunner, host: PermanentHandle) {
    let action = encode_attack(host.index as u16, SECURITY_TARGET);
    assert_eq!(
        build_action_mask(&r.game, host.player)[action as usize],
        1.0
    );
    r.game.decode_action(action, host.player);
}

fn pass_turn(r: &mut DebugRunner) {
    let player = r.game.turn_player();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(build_action_mask(&r.game, player)[PASS as usize], 1.0);
    r.game.decode_action(PASS, player);
}

fn hand_index(r: &DebugRunner, owner: u8, id: &str) -> u16 {
    r.game.players[owner as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("card remains in hand") as u16
}

fn top<'a>(r: &'a DebugRunner, h: PermanentHandle) -> &'a str {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

fn pick_route(r: &mut DebugRunner, cost: i16) {
    let view = r
        .pending_selection_view()
        .expect("distinct legal evolution requirements");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let choices = view.effect_choices.expect("route labels");
    assert!(
        choices.len() >= 2,
        "both printed and alternative costs remain choices"
    );
    for c in &choices {
        assert_eq!(
            build_action_mask(&r.game, view.selecting_player)[c.action_id as usize],
            1.0
        );
    }
    let choice = choices
        .iter()
        .find(|c| c.label.contains(&format!("{cost}")))
        .expect("requested printed cost");
    choose(r, choice.action_id);
}

fn finish_combat(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        assert!(
            view.is_optional,
            "only counter/block declines may remain: {view:?}"
        );
        assert_ne!(
            view.kind,
            SelectionKind::Hand,
            "unexpected additional evolution opportunity"
        );
        choose(r, PASS);
    }
    panic!("combat did not settle");
}

fn ready_again(r: &mut DebugRunner, host: PermanentHandle) {
    finish_combat(r);
    // Stage a later attack opportunity, without changing the physical egg
    // source or touching the engine's per-turn activation records.
    r.game.players[host.player as usize].battle_area[host.index as usize].is_suspended = false;
}

#[test]
fn printed_metadata_and_inherited_optional_opt_timing() {
    let r = builder().start();
    let card = r.compiled_card(ID).unwrap();
    assert_eq!(card.name, "Dorimon");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert_eq!(card.form.as_deref(), Some("In-Training"));
    assert_eq!(card.traits, vec!["Lesser", "X Antibody", "Chronicle"]);
    assert!(card.alt_paths.is_empty());
    assert_eq!(card.effects.len(), 1);
    let CompiledClause::Triggered(effect) = &card.effects[0] else {
        panic!("inherited trigger");
    };
    assert_eq!(effect.scope, CompiledScope::Inherited);
    assert_eq!(effect.when, vec![CompiledTiming::WhenAttacking]);
    assert!(effect.once_per_turn);
    assert!(
        effect.optional,
        "declining the entire may effect must preserve its use"
    );
}

#[test]
fn both_actual_chronicle_cards_and_each_legal_cost_are_selectable() {
    for id in [RAPTOR, GINRYU] {
        for cost in [2, 3] {
            let (mut r, h) = setup(0, &[RAPTOR, GINRYU]);
            let egg_identity =
                r.game.players[0].battle_area[h.index as usize].card_sources[0].handle();
            let deck_before = r.game.players[0].deck.len();
            attack(&mut r, h);
            let view = r.pending_selection_view().expect("inherited may evolve");
            assert_eq!(view.kind, SelectionKind::Hand);
            assert_eq!(view.selecting_player, 0);
            assert!(view.is_optional);
            assert_eq!(
                view.valid_action_ids,
                vec![PLAY_HAND_START, PLAY_HAND_START + 1]
            );
            assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
            let index = hand_index(&r, 0, id);
            choose(&mut r, PLAY_HAND_START + index);
            pick_route(&mut r, cost);
            assert_eq!(top(&r, h), id);
            assert_eq!(r.memory(), 8 - cost, "P-176 grants no cost reduction");
            assert_eq!(r.game.players[0].deck.len(), deck_before - 1);
            assert_eq!(
                r.game.players[0].battle_area[h.index as usize].card_sources[0].handle(),
                egg_identity
            );
            assert!(
                r.game.players[0].battle_area[h.index as usize].is_suspended,
                "evolving preserves attack suspension"
            );
        }
    }
}

#[test]
fn hand_choice_masks_trait_kind_color_level_and_unaffordable_cards() {
    let hand = [
        RAPTOR,
        GINRYU,
        "WRONG-TRAIT",
        "WRONG-COLOR",
        "WRONG-LEVEL",
        "NOT-DIGIMON",
        "UNPAYABLE",
    ];
    let (mut r, h) = setup(0, &hand);
    r.inject_trash(0, RAPTOR);
    attack(&mut r, h);
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::Hand);
    let mask = build_action_mask(&r.game, 0);
    for (i, id) in hand.iter().enumerate() {
        assert_eq!(
            mask[PLAY_HAND_START as usize + i],
            if i < 2 { 1.0 } else { 0.0 },
            "{id}"
        );
    }
    assert_eq!(
        view.valid_action_ids,
        vec![PLAY_HAND_START, PLAY_HAND_START + 1]
    );
    choose(&mut r, PASS);
    finish_combat(&mut r);
    assert_eq!(
        r.trash_size(0),
        1,
        "hand-only effect cannot take its matching trash card"
    );
}

#[test]
fn declining_preserves_hand_memory_and_opt_for_a_later_attack() {
    let (mut r, h) = setup(0, &[RAPTOR]);
    for _ in 0..2 {
        attack(&mut r, h);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
        choose(&mut r, PASS);
        ready_again(&mut r, h);
        assert_eq!(top(&r, h), HOST);
        assert_eq!(r.memory(), 8);
        assert_eq!(r.hand_size(0), 1);
    }
    attack(&mut r, h);
    choose(&mut r, PLAY_HAND_START);
    pick_route(&mut r, 2);
    assert_eq!(top(&r, h), RAPTOR);
    assert_eq!(r.memory(), 6);
}

#[test]
fn successful_activation_is_once_per_turn_and_resets_next_turn() {
    let (mut r, h) = setup(0, &[RAPTOR, "NEXT"]);
    attack(&mut r, h);
    choose(&mut r, PLAY_HAND_START);
    pick_route(&mut r, 2);
    ready_again(&mut r, h);
    attack(&mut r, h);
    assert_ne!(
        r.pending_kind(),
        Some(SelectionKind::Hand),
        "same physical egg has spent its use"
    );
    finish_combat(&mut r);
    assert_eq!(top(&r, h), RAPTOR);
    pass_turn(&mut r);
    assert_eq!(r.game.turn_player(), 1);
    pass_turn(&mut r);
    assert_eq!(r.game.turn_player(), 0);
    attack(&mut r, h);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Hand),
        "turn change resets the egg's activation"
    );
    let before = r.memory();
    let index = hand_index(&r, 0, "NEXT");
    choose(&mut r, PLAY_HAND_START + index);
    assert_eq!(top(&r, h), "NEXT");
    assert_eq!(r.memory(), before - 3);
}

#[test]
fn physical_egg_copies_on_separate_hosts_have_independent_uses() {
    let (mut r, first) = setup(0, &[RAPTOR, RAPTOR]);
    let second = r.place_stack(0, &[ID, HOST]);
    for h in [first, second] {
        attack(&mut r, h);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
        let index = hand_index(&r, 0, RAPTOR);
        choose(&mut r, PLAY_HAND_START + index);
        pick_route(&mut r, 2);
        finish_combat(&mut r);
        assert_eq!(top(&r, h), RAPTOR);
    }
    assert_eq!(r.memory(), 4);
}

#[test]
fn opponent_owner_uses_own_hand_pays_own_cost_and_only_evolves_own_host() {
    let (mut r, h) = setup(1, &[RAPTOR, GINRYU]);
    let other = r.place_stack(0, &[ID, HOST]);
    r.add_to_hand(0, RAPTOR);
    let memory_before = r.memory();
    attack(&mut r, h);
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(view.selecting_player, 1);
    choose(&mut r, PLAY_HAND_START);
    pick_route(&mut r, 2);
    assert_eq!(top(&r, h), RAPTOR);
    assert_eq!(top(&r, other), HOST);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), memory_before - 2);
}

#[test]
fn no_legal_hand_candidate_has_no_evolution_prompt_even_with_matching_trash() {
    for hand in [
        &[][..],
        &[
            "WRONG-TRAIT",
            "WRONG-COLOR",
            "WRONG-LEVEL",
            "NOT-DIGIMON",
            "UNPAYABLE",
        ][..],
    ] {
        let (mut r, h) = setup(0, hand);
        r.inject_trash(0, RAPTOR);
        attack(&mut r, h);
        assert_ne!(r.pending_kind(), Some(SelectionKind::Hand));
        finish_combat(&mut r);
        assert_eq!(top(&r, h), HOST);
        assert_eq!(r.memory(), 8);
        assert_eq!(r.hand_size(0), hand.len());
        assert_eq!(r.trash_size(0), 1);
    }
}

#[test]
fn another_ally_attack_does_not_activate_the_eggs_host_effect() {
    let (mut r, h) = setup(0, &[RAPTOR]);
    let attacker = r.place_on_field(0, HOST, Some(0));
    attack(&mut r, attacker);
    assert_ne!(r.pending_kind(), Some(SelectionKind::Hand));
    finish_combat(&mut r);
    assert_eq!(top(&r, h), HOST);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn face_down_egg_source_cannot_supply_the_inherited_attack_effect() {
    let (mut r, h) = setup(0, &[RAPTOR]);
    r.game.players[0].battle_area[h.index as usize].card_sources[0].face_down = true;
    attack(&mut r, h);
    assert_ne!(r.pending_kind(), Some(SelectionKind::Hand));
    finish_combat(&mut r);
    assert_eq!(top(&r, h), HOST);
    assert_eq!(r.hand_size(0), 1);
}
