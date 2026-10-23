//! EX6-072 Mega Digimon Assembly! — printed card image and official Q3822.
//! CR 4.3 8-2-2-2/3: recipe stack order; 8-2-3: cost, draw and DNA triggers.
use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, PLAY_HAND_START, TRASH_EFFECT_START},
    },
    card_data::{CardData, DnaCost, DnaRequirement},
    debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder},
    effect_context::EffectContext,
    enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType},
    events::GameEvent,
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
    selection::SelectionKind,
};

const ID: &str = "EX6-072";
const RESULT_TRIGGER: &str = r#"
card: RESULT
name: Result
kind: digimon
color: [white]
level: 7
cost: 15
dp: 15000
effects:
  - when: when_digivolving
    process:
      - draw: {of: you, count: 1}
  - when: on_dna_digivolve
    process:
      - gain_memory: 1
"#;

fn card(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(12000);
    c
}
fn requirement(color: CardColor, level: u8) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}
fn result(id: &str, partner_level: u8) -> CardData {
    let mut c = card(id, CardColor::White, 7);
    c.dna_costs = vec![DnaCost {
        requirement1: requirement(CardColor::Red, 6),
        requirement2: requirement(
            if partner_level == 7 {
                CardColor::White
            } else {
                CardColor::Blue
            },
            partner_level,
        ),
        memory_cost: 2,
    }];
    c
}
fn base() -> DebugRunnerBuilder {
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Red];
    let mut white_option = make_test_card("WHITE-OPTION", "Other white Option");
    white_option.card_kind = CardKind::Option;
    white_option.level = None;
    white_option.dp = None;
    white_option.colors = vec![CardColor::White];
    DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/_examples/EX6-072.yaml"))
        .unwrap()
        .from_dsl_yaml(RESULT_TRIGGER)
        .unwrap()
        .add_card(result("RESULT", 6))
        .add_card(result("RESULT-2", 6))
        .add_card(result("SELF-RESULT", 7))
        .add_card(card("NO-RECIPE", CardColor::White, 7))
        .add_card(card("RED", CardColor::Red, 6))
        .add_card(card("BLUE", CardColor::Blue, 6))
        .add_card(card("GREEN", CardColor::Green, 6))
        .add_card(card("LOW", CardColor::Red, 5))
        .add_card(card("HIGH", CardColor::Red, 7))
        .add_card(card("SOURCE", CardColor::Red, 3))
        .add_card(card("FILL", CardColor::Red, 3))
        .add_card(tamer)
        .add_card(white_option)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
}
fn start(r: &mut DebugRunner) {
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
}
fn prompt(r: &DebugRunner, expected: &[u16], optional: bool) {
    let p = r.pending_selection_view().expect("player-visible choice");
    assert_eq!(p.valid_action_ids, expected);
    assert_eq!(p.is_optional, optional);
    let mask = build_action_mask(&r.game, p.selecting_player);
    for &a in expected {
        assert_eq!(mask[a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], if optional { 1.0 } else { 0.0 });
    let mut actual: Vec<_> = mask
        .iter()
        .enumerate()
        .filter(|(_, v)| **v > 0.5)
        .map(|(i, _)| i as u16)
        .collect();
    let mut wanted = expected.to_vec();
    if optional {
        wanted.push(PASS);
    }
    wanted.sort_unstable();
    actual.sort_unstable();
    assert_eq!(actual, wanted, "no unrelated action is legal at the prompt");
}
fn pick(r: &mut DebugRunner, action: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[action as usize], 1.0);
    r.execute_action(p, action).unwrap();
}
fn use_option(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}
fn fixture(field_id: &str, partner_id: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = base().hand(0, &[ID, "RESULT", partner_id]).start();
    start(&mut r);
    let field = r.place_stack(0, &["SOURCE", field_id]);
    r.place_on_field(1, "HIGH", Some(0));
    (r, field)
}
fn resolve_result_trigger_order(r: &mut DebugRunner) {
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder),
        "the result's mandatory When Digivolving and DNA effects trigger together");
    let actions = r.pending_selection_view().unwrap().valid_action_ids;
    assert_eq!(actions.len(), 2);
    prompt(r, &actions, false);
    pick(r, actions[0]);
    assert!(r.pending_selection().is_none(), "both mandatory scalar effects finish after the chosen first effect");
}
fn complete(r: &mut DebugRunner, field: PermanentHandle, partner: u16) {
    pick(r, 0);
    pick(r, encode_attack(0, field.index as u16));
    pick(r, partner);
    if r.pending_kind() == Some(SelectionKind::TriggerOrder) {
        resolve_result_trigger_order(r);
    }
}

#[test]
fn ex6_072_printed_clause_optionality_and_blue_security_scope() {
    let r = base().start();
    let card = r.compiled_card(ID).unwrap();
    let clauses: Vec<_> = card
        .effects
        .iter()
        .filter_map(|e| {
            if let CompiledClause::Triggered(c) = e {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(clauses.len(), 2);
    assert!(
        !clauses[0].optional,
        "the optional DNA selections implement the single printed may"
    );
    let security = clauses
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnSecurity))
        .unwrap();
    assert_eq!(
        security.scope,
        CompiledScope::FaceUp,
        "blue Security is neither inherited nor pink stack scope"
    );
    assert!(
        !security.optional,
        "return one, then add self are mandatory"
    );
}

#[test]
fn ex6_072_color_bypass_matches_opponent_battle_digimon_and_execute_gate() {
    for case in 0..7 {
        let mut r = base().hand(0, &[ID, "WHITE-OPTION"]).start();
        start(&mut r);
        match case {
            0 => {
                r.place_on_field(1, "RED", Some(0));
            }
            1 => {
                r.place_on_field(1, "HIGH", Some(0));
            }
            2 => {
                r.place_on_field(1, "LOW", Some(0));
            }
            3 => {
                r.place_on_field(0, "RED", Some(0));
            }
            4 => {
                r.place_in_breeding(1, "RED");
            }
            5 => {
                r.place_on_field(1, "TAMER", Some(0));
            }
            _ => {}
        }
        let allowed = case < 2;
        assert_eq!(
            build_action_mask(&r.game, 0)[0] > 0.5,
            allowed,
            "case {case}"
        );
        assert_eq!(
            build_action_mask(&r.game, 0)[1],
            0.0,
            "bypass is specific to EX6-072"
        );
        let outcome = r.game.play_option_from_hand(0, 0);
        assert_eq!(
            matches!(
                outcome,
                digimon_engine::selection::OptionPlayResult::Invalid
            ),
            !allowed,
            "execution case {case}"
        );
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), if allowed { 6 } else { 10 });
        assert_eq!(r.hand_size(0), if allowed { 1 } else { 2 });
    }
}

#[test]
fn ex6_072_normal_white_requirement_still_permits_use_without_opponent_level_six() {
    let mut r = base().hand(0, &[ID]).start();
    start(&mut r);
    r.place_on_field(0, "NO-RECIPE", Some(0));
    use_option(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 6);
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn ex6_072_result_field_and_partner_masks_are_correlated() {
    let mut r = base()
        .hand(
            0,
            &[
                ID,
                "RESULT",
                "NO-RECIPE",
                "BLUE",
                "BLUE",
                "GREEN",
                "LOW",
                "RESULT-2",
                "TAMER",
            ],
        )
        .start();
    start(&mut r);
    let red = r.place_on_field(0, "RED", Some(0));
    r.place_on_field(0, "GREEN", Some(0));
    r.place_on_field(0, "LOW", Some(0));
    r.place_on_field(0, "HIGH", Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    r.place_on_field(1, "RED", Some(0));
    r.place_in_breeding(0, "RED");
    use_option(&mut r);
    prompt(&r, &[0, 6], true);
    pick(&mut r, 0);
    prompt(&r, &[encode_attack(0, red.index as u16)], true);
    pick(&mut r, encode_attack(0, red.index as u16));
    prompt(&r, &[2, 3], true);
    let first_duplicate = r.game.players[0].hand[2].handle();
    let selected_duplicate = r.game.players[0].hand[3].handle();
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        pick(game, 3);
        resolve_result_trigger_order(game);
        assert!(game.game.players[0]
            .hand
            .iter()
            .any(|c| c.handle() == first_duplicate));
        assert!(game.game.players[0].battle_area[red.index as usize]
            .card_sources
            .iter()
            .any(|c| c.handle() == selected_duplicate));
        assert!(game.pending_selection().is_none());
    }
    assert_eq!(
        r.game.verification_digest(),
        clone.game.verification_digest()
    );
}

#[test]
fn ex6_072_no_complete_recipe_skips_all_choices() {
    for case in 0..4 {
        let hand = if case == 0 {
            &[ID, "NO-RECIPE", "BLUE"][..]
        } else if case == 1 {
            &[ID, "RESULT", "GREEN"][..]
        } else {
            &[ID, "RESULT", "BLUE"][..]
        };
        let mut r = base().hand(0, hand).start();
        start(&mut r);
        let h = r.place_on_field(0, if case == 2 { "LOW" } else { "RED" }, Some(0));
        if case == 3 {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
            );
        }
        r.place_on_field(1, "HIGH", Some(0));
        use_option(&mut r);
        assert!(
            r.pending_selection().is_none(),
            "case {case} has no legal triple"
        );
        assert_eq!(r.memory(), 6);
        assert_eq!(r.deck_size(0), 12);
    }
}

#[test]
fn ex6_072_selected_result_cannot_also_be_its_own_material() {
    for copies in [1, 2] {
        let hand = if copies == 1 {
            &[ID, "SELF-RESULT"][..]
        } else {
            &[ID, "SELF-RESULT", "SELF-RESULT"][..]
        };
        let mut r = base().hand(0, hand).start();
        start(&mut r);
        let field = r.place_on_field(0, "RED", Some(0));
        r.place_on_field(1, "HIGH", Some(0));
        use_option(&mut r);
        if copies == 1 {
            assert!(
                r.pending_selection().is_none(),
                "one physical card cannot be result and partner"
            );
        } else {
            prompt(&r, &[0, 1], true);
            pick(&mut r, 0);
            pick(&mut r, encode_attack(0, field.index as u16));
            prompt(&r, &[1], true);
            pick(&mut r, 1);
            assert_eq!(
                r.game.players[0].battle_area[field.index as usize]
                    .card_sources
                    .len(),
                3
            );
        }
    }
}

#[test]
fn ex6_072_declining_any_dna_selection_consumes_only_option_use() {
    for stage in 0..3 {
        let (mut r, field) = fixture("RED", "BLUE");
        let original = r.game.players[0].battle_area[field.index as usize]
            .card_sources
            .clone();
        use_option(&mut r);
        if stage >= 1 {
            pick(&mut r, 0);
        }
        if stage >= 2 {
            pick(&mut r, encode_attack(0, field.index as u16));
        }
        let mut clone = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut clone] {
            assert!(game.pending_is_optional());
            pick(game, PASS);
            assert!(
                game.pending_selection().is_none(),
                "declining stage {stage} ends DNA"
            );
            assert_eq!(game.memory(), 6);
            assert_eq!(game.hand_size(0), 2);
            assert_eq!(game.deck_size(0), 12);
            assert_eq!(
                game.game.players[0].battle_area[field.index as usize]
                    .card_sources
                    .iter()
                    .map(|c| c.handle())
                    .collect::<Vec<_>>(),
                original.iter().map(|c| c.handle()).collect::<Vec<_>>()
            );
            assert!(game.game.players[0]
                .trash
                .iter()
                .any(|c| c.card_id(&game.game.card_data) == ID));
        }
        assert_eq!(
            r.game.verification_digest(),
            clone.game.verification_digest()
        );
    }
}

#[test]
fn ex6_072_dna_pays_printed_cost_orders_materials_draws_and_fires_both_timings() {
    for (field_id, partner_id, expected) in [
        ("RED", "BLUE", vec!["BLUE", "SOURCE", "RED", "RESULT"]),
        ("BLUE", "RED", vec!["SOURCE", "BLUE", "RED", "RESULT"]),
    ] {
        let (mut r, field) = fixture(field_id, partner_id);
        r.game.players[0].battle_area[field.index as usize].is_suspended = true;
        use_option(&mut r);
        complete(&mut r, field, 1);
        assert!(r.pending_selection().is_none());
        let p = &r.game.players[0].battle_area[field.index as usize];
        assert_eq!(
            p.card_sources
                .iter()
                .map(|c| c.card_id(&r.game.card_data))
                .collect::<Vec<_>>(),
            expected
        );
        assert!(!p.is_suspended, "DNA starts a new unsuspended Digimon");
        assert_eq!(
            r.memory(),
            5,
            "10 minus use 4 minus DNA 2 plus DNA trigger 1"
        );
        assert_eq!(
            r.deck_size(0),
            10,
            "one evolution bonus draw plus one When Digivolving draw"
        );
        assert_eq!(r.hand_size(0), 2);
        assert_eq!(r.events_since(0).iter().filter(|e| matches!(e, GameEvent::Digivolve { top_card_id, was_dna: true, memory_paid: 2, .. } if top_card_id == "RESULT")).count(), 1);
    }
}

#[test]
fn ex6_072_effect_use_on_opponents_turn_charges_the_controller() {
    let (mut r, field) = fixture("RED", "BLUE");
    r.game.turn_player_idx = 1;
    r.game.memory = 1;
    r.game.use_option_from_hand_without_paying_cost(0, 0);
    complete(&mut r, field, 1);
    assert_eq!(
        r.memory(),
        2,
        "P0 DNA costs 2 toward P1; P0 DNA trigger gains 1 back"
    );
    assert_eq!(r.deck_size(0), 10);
    assert_eq!(r.deck_size(1), 12);
}

#[test]
fn ex6_072_dna_reducer_payment_is_optional_visible_and_clone_safe() {
    for accept in [false, true] {
        let (mut r, field) = fixture("RED", "BLUE");
        let source = r.game.players[0].battle_area[field.index as usize]
            .top_card()
            .handle();
        EffectContext::new(&mut r.game, source, Some(field), 0).arm_player_digivolve_cost_reducer(
            1,
            true,
            Some(CardColor::Red),
            true,
        );
        use_option(&mut r);
        complete(&mut r, field, 1);
        assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
        assert!(r.pending_is_optional());
        assert_eq!(r.memory(), 6, "DNA payment waits for the optional reducer");
        let mut clone = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut clone] {
            if accept {
                let a = game.pending_selection_view().unwrap().valid_action_ids[0];
                pick(game, a);
                prompt(game, &[encode_attack(0, field.index as u16)], false);
                pick(game, encode_attack(0, field.index as u16));
            } else {
                pick(game, PASS);
            }
            resolve_result_trigger_order(game);
            assert!(game.pending_selection().is_none());
            assert_eq!(game.memory(), if accept { 6 } else { 5 });
            assert_eq!(game.deck_size(0), 10);
            assert!(!game.game.players[0].battle_area[field.index as usize].is_suspended);
        }
        assert_eq!(
            r.game.verification_digest(),
            clone.game.verification_digest()
        );
    }
}

#[test]
fn ex6_072_security_mandatory_recovery_filters_kind_level_owner_then_adds_self() {
    let mut r = base().security(1, &[ID]).start();
    start(&mut r);
    for id in ["RED", "HIGH", "LOW", "TAMER"] {
        r.inject_trash(1, id);
    }
    r.inject_trash(0, "RED");
    let attacker = r.place_on_field(0, "BLUE", Some(0));
    r.attack_player(attacker, 1, false);
    prompt(&r, &[TRASH_EFFECT_START, TRASH_EFFECT_START + 1], false);
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        pick(game, TRASH_EFFECT_START + 1);
        assert!(game.pending_selection().is_none());
        let hand: Vec<_> = game.game.players[1]
            .hand
            .iter()
            .map(|c| c.card_id(&game.game.card_data))
            .collect();
        assert_eq!(hand, ["HIGH", ID]);
        assert_eq!(game.trash_size(1), 3);
        assert_eq!(game.trash_size(0), 1);
        assert_eq!(game.memory(), 10);
    }
    assert_eq!(
        r.game.verification_digest(),
        clone.game.verification_digest()
    );
}

#[test]
fn ex6_072_security_adds_self_even_without_recoverable_trash() {
    for empty in [false, true] {
        let mut r = base().security(1, &[ID]).start();
        start(&mut r);
        if !empty {
            r.inject_trash(1, "LOW");
            r.inject_trash(1, "TAMER");
        }
        let attacker = r.place_on_field(0, "BLUE", Some(0));
        r.attack_player(attacker, 1, false);
        assert!(r.pending_selection().is_none());
        assert_eq!(
            r.game.players[1]
                .hand
                .iter()
                .map(|c| c.card_id(&r.game.card_data))
                .collect::<Vec<_>>(),
            [ID]
        );
        assert_eq!(r.trash_size(1), if empty { 0 } else { 2 });
        assert_eq!(r.memory(), 10);
    }
}
