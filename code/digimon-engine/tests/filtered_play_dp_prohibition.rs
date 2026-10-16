//! Reusable DP-ceiling play prohibition; source driver EX3-012 Volcanicdramon.
//! Install declaration-only scaffold before RED: it exposes the modifier spelling
//! without implementing any legality consult, so failures reach actual behavior.
use digimon_engine::action::build_action_mask;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, CostDelta, Expiry, GamePhase, ModifierType, PlaySource};
use digimon_engine::modifiers::PlayerModifierEntry;

fn fixture() -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(
            r#"
card: FP-SOURCE
name: Filtered play source
kind: tamer
color: [red]
cost: 1
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          optional: true
          bind_as: chosen
          filter: { can_play_from_effect_without_cost: true }
          prompt: Choose an eligible card to play
      - play_from_hand_free: { of: you, hand_index: chosen }
"#,
        )
        .unwrap()
        .from_dsl_yaml(
            r#"
card: FP-OPTION
name: Filtered play Option control
kind: option
color: [red]
cost: 1
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#,
        )
        .unwrap();
    for (id, kind, dp) in [
        ("FP-LOW", CardKind::Digimon, Some(5000)),
        ("FP-HIGH", CardKind::Digimon, Some(6000)),
        ("FP-NODP", CardKind::Digimon, None),
        ("FP-TAMER", CardKind::Tamer, None),
        ("FP-DUAL", CardKind::Dual, Some(5000)),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.dp = dp;
        c.play_cost = 1;
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &["FP-LOW", "FP-HIGH", "FP-TAMER", "FP-OPTION"])
        .hand(1, &["FP-LOW", "FP-HIGH", "FP-TAMER", "FP-OPTION"])
        .deck(0, &["FP-HIGH"; 12])
        .deck(1, &["FP-HIGH"; 12])
        .security(0, &["FP-LOW"])
        .security(1, &["FP-LOW"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "FP-SOURCE", None);
    r.place_on_field(1, "FP-SOURCE", None);
    r
}
fn lock(r: &mut DebugRunner, player: u8, value: i32, expiry: Expiry) {
    r.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonWithDPAtMost,
            value,
            expiry,
            None,
            1 - player,
        ),
    );
}
fn idx(r: &DebugRunner, p: usize, id: &str) -> usize {
    r.game.players[p]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap()
}
#[test]
fn native_mask_filters_printed_dp_only_and_keeps_tamer_option_choices() {
    let mut r = fixture();
    lock(&mut r, 0, 5000, Expiry::Permanent);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[idx(&r, 0, "FP-LOW")], 0.0);
    for id in ["FP-HIGH", "FP-TAMER", "FP-OPTION"] {
        assert_eq!(mask[idx(&r, 0, id)], 1.0, "{id}");
    }
}
#[test]
fn native_execution_rechecks_before_spending_or_moving_card() {
    let mut r = fixture();
    lock(&mut r, 0, 5000, Expiry::Permanent);
    let count = r.game.players[0].hand.len();
    let memory = r.game.memory;
    assert!(r.game.play_from_hand(0, 0).is_none());
    assert_eq!(r.game.memory, memory);
    assert_eq!(r.game.players[0].hand.len(), count);
    let high = idx(&r, 0, "FP-HIGH");
    assert!(r.game.play_from_hand(0, high).is_some());
}
#[test]
fn effect_controller_applies_across_hand_trash_security_reveal_material_breeding_and_token() {
    for destination in [0u8, 1] {
        for controller in [0u8, 1] {
            for zone in [
                "hand", "trash", "security", "reveal", "material", "breeding", "token",
            ] {
                let mut r = fixture();
                lock(&mut r, 0, 5000, Expiry::Permanent);
                let source = r.game.players[0].battle_area[0].top_card().handle();
                let target = r.place_on_field(destination, "FP-HIGH", None);
                r.inject_trash(destination, "FP-LOW");
                let added = r.add_to_hand(destination, "FP-LOW");
                let card = r.game.players[destination as usize].hand.remove(added);
                let material_card = card.handle();
                r.game.players[destination as usize].battle_area[target.index as usize]
                    .card_sources
                    .insert(0, card);
                let added = r.add_to_hand(destination, "FP-LOW");
                let revealed = r.game.players[destination as usize].hand.remove(added);
                let reveal_handle = revealed.handle();
                r.game.revealed_cards.push(revealed);
                r.game
                    .set_effect_source_player_for_test(Some(1 - controller));
                let played = {
                    let mut c = EffectContext::new(&mut r.game, source, None, controller);
                    match zone {
                        "hand" => c.play_from_hand_free(destination, 0).is_some(),
                        "trash" => c
                            .play_from_trash_with_cost(destination, 0, CostDelta::Free)
                            .is_some(),
                        "security" => c.play_from_security(destination).is_some(),
                        "reveal" => c
                            .play_from_revealed_free(destination, reveal_handle)
                            .is_some(),
                        "material" => c.play_from_materials(target, 0, CostDelta::Free).is_some(),
                        "breeding" => c.play_to_breeding_from_hand(destination, 0),
                        _ => c.play_token(destination, "familiar").is_some(),
                    }
                };
                assert_eq!(
                    played,
                    controller == 1,
                    "zone={zone} destination={destination} controller={controller}"
                );
                if controller == 0 {
                    match zone {
                        "hand" | "breeding" => assert_eq!(
                            r.game.players[destination as usize].hand[0].card_id(&r.game.card_data),
                            "FP-LOW"
                        ),
                        "trash" => assert_eq!(r.game.players[destination as usize].trash.len(), 1),
                        "security" => {
                            assert_eq!(r.game.players[destination as usize].security.len(), 1)
                        }
                        "reveal" => assert!(r
                            .game
                            .revealed_cards
                            .iter()
                            .any(|c| c.handle() == reveal_handle)),
                        "material" => assert_eq!(
                            r.game.players[destination as usize].battle_area[target.index as usize]
                                .card_sources[0]
                                .handle(),
                            material_card
                        ),
                        _ => {}
                    }
                }
            }
        }
    }
}
#[test]
fn effect_legality_and_batch_play_skip_only_prohibited_card() {
    let mut r = fixture();
    lock(&mut r, 0, 5000, Expiry::Permanent);
    let low = r.game.players[0].hand[0].handle();
    let high = r.game.players[0].hand[1].handle();
    assert!(!r
        .game
        .can_play_card_from_effect_without_cost_for_controller(0, low, 1, 0));
    assert!(r
        .game
        .can_play_card_from_effect_without_cost_for_controller(0, high, 1, 0));
    assert_eq!(r.game.play_cards_from_zones_free(0, &[low, high]), 1);
    assert!(r.game.players[0].hand.iter().any(|c| c.handle() == low));
}
#[test]
fn breeding_mask_and_entry_share_card_filter() {
    let mut r = fixture();
    lock(&mut r, 0, 5000, Expiry::Permanent);
    assert!(!r.game.can_play_to_breeding_from_hand(0, 0, 0));
    assert!(r.game.can_play_to_breeding_from_hand(0, 1, 0));
    assert!(!r.game.play_to_breeding_from_hand(0, 0));
    assert!(r.game.players[0].breeding_area.is_none());
}
#[test]
fn strongest_ceiling_is_not_summed_and_clone_expiry_is_owner_relative() {
    let mut r = fixture();
    lock(&mut r, 0, 3000, Expiry::EndOfOpponentsTurn);
    lock(&mut r, 0, 5000, Expiry::EndOfOpponentsTurn);
    let mut cloned = r.game.clone();
    for g in [&mut r.game, &mut cloned] {
        assert_eq!(build_action_mask(g, 0)[0], 0.0);
        assert_eq!(
            build_action_mask(g, 0)[1],
            1.0,
            "ceilings must not sum to 8000"
        );
        g.modifiers.expire_player_end_of_turn(1);
        assert_eq!(
            build_action_mask(g, 0)[0],
            0.0,
            "source turn ending does not expire opponent-turn restriction"
        );
        g.modifiers.expire_player_end_of_turn(0);
        assert_eq!(build_action_mask(g, 0)[0], 1.0);
    }
}
#[test]
fn real_dsl_selection_excludes_low_dp_and_preserves_optional_decline() {
    let mut r = fixture();
    lock(&mut r, 0, 5000, Expiry::Permanent);
    r.fire_play_event_triggers(0, 0, true, false);
    let p = r.pending_selection_view().unwrap();
    assert!(p.is_optional);
    let candidates = p.valid_action_ids.iter().map(|a| *a).collect::<Vec<_>>();
    assert!(
        !candidates.contains(&0),
        "low-DP hand card cannot be a candidate"
    );
    assert!(
        candidates.contains(&1),
        "high-DP control remains selectable"
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[digimon_engine::action::space::PASS as usize],
        1.0
    );
    r.execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert_eq!(r.game.players[0].hand.len(), 4);
}
#[test]
fn pending_legal_pick_rechecks_new_restriction_in_both_clones() {
    let mut r = fixture();
    r.fire_play_event_triggers(0, 0, true, false);
    let pending = r.pending_selection_view().unwrap();
    let low_action = pending.valid_action_ids[0];
    assert_eq!(build_action_mask(&r.game, 0)[low_action as usize], 1.0);
    lock(&mut r, 0, 5000, Expiry::Permanent);
    let frozen = r.game.clone();
    for cloned in [false, true] {
        if cloned {
            r.game = frozen.clone();
        }
        // The installed choice was legal when offered. The effect's play primitive
        // must recheck the restriction even if the cached candidate can resolve.
        r.game.decode_action(low_action, 0);
        assert!(r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "FP-LOW"));
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}

struct ParkLowPlay;
impl digimon_engine::effect::CardEffect for ParkLowPlay {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![
            digimon_engine::effect::Effect::when_permanent_would_play(card)
                .optional()
                .name("Declinable low-card replacement")
                .replacement_condition(|ctx, subject| match subject {
                    digimon_engine::replacement::ReplacementSubject::Card(h, _) => ctx
                        .game
                        .card_data_for_handle(*h)
                        .is_some_and(|d| d.card_id == "FP-LOW"),
                    _ => false,
                })
                .replacement_process(|ctx| ctx.cancel())
                .build(),
        ]
    }
}
#[test]
fn deferred_play_rechecks_captured_controller_and_restores_origin_in_clones() {
    for controller in [0u8, 1] {
        let mut r = fixture();
        r.register_effect("FP-TAMER", std::sync::Arc::new(ParkLowPlay));
        r.place_on_field(0, "FP-TAMER", None);
        r.inject_trash(1, "FP-LOW");
        let source = r.game.players[0].battle_area[0].top_card().handle();
        let card = r.game.players[1].trash[0].handle();
        {
            let mut c = EffectContext::new(&mut r.game, source, None, controller);
            assert!(c.play_from_trash_with_cost(1, 0, CostDelta::Free).is_none());
        }
        assert_eq!(
            r.pending_selection_view().unwrap().kind,
            digimon_engine::selection::SelectionKind::Replacement
        );
        lock(&mut r, 0, 5000, Expiry::Permanent);
        // Resumption must not mistake unrelated current ambient provenance for the
        // effect which declared the parked play.
        r.game
            .set_effect_source_player_for_test(Some(1 - controller));
        let snapshot = r.game.clone();
        for cloned in [false, true] {
            r.game = snapshot.clone();
            if cloned {
                r.game = r.game.clone();
            }
            let memory = r.game.memory;
            let pass = digimon_engine::action::space::PASS;
            let actor = r.game.pending_selection.as_ref().unwrap().selecting_player;
            assert!(build_action_mask(&r.game, actor)[pass as usize] > 0.0);
            r.game.resolve_selection(actor, pass).unwrap();
            let played = r.game.players[1]
                .battle_area
                .iter()
                .any(|p| p.top_card().handle() == card);
            assert_eq!(played, controller == 1, "captured controller {controller}");
            assert_eq!(
                r.game.players[1].trash.iter().any(|c| c.handle() == card),
                controller == 0
            );
            assert!(!r.game.players[1].hand.iter().any(|c| c.handle() == card));
            assert_eq!(
                r.game.memory, memory,
                "free play or prohibited play spends no memory"
            );
        }
    }
}
#[test]
fn card_kind_no_dp_and_zero_threshold_controls_are_not_broadened() {
    let mut r = fixture();
    lock(&mut r, 0, 5000, Expiry::Permanent);
    for (id, allowed) in [
        ("FP-DUAL", false),
        ("FP-NODP", true),
        ("FP-TAMER", true),
        ("FP-HIGH", true),
    ] {
        let index = r.add_to_hand(0, id);
        let h = r.game.players[0].hand[index].handle();
        assert_eq!(
            r.game
                .can_play_card_from_effect_without_cost_for_controller(0, h, 1, 0),
            allowed,
            "{id}"
        );
    }
    let mut r = fixture();
    lock(&mut r, 0, 0, Expiry::Permanent);
    let low = r.game.players[0].hand[0].handle();
    assert!(r
        .game
        .can_play_card_from_effect_without_cost_for_controller(0, low, 1, 0));
}

#[test]
fn early_cost_reducer_parking_preserves_effect_controller_before_would_play_capture() {
    for controller in [0u8, 1] {
        let mut low = make_test_card("COST-LOW", "Cost low");
        low.dp = Some(5000);
        low.play_cost = 2;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(
                r#"
card: COST-GATE
name: Cost gate
kind: tamer
color: [red]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_any_ally_played: { name_is: Cost low }
    optional: true
    amount: 1
"#,
            )
            .unwrap()
            .add_card(low)
            .add_card(make_test_card("COST-SOURCE", "Cost source"))
            .hand(1, &["COST-LOW"])
            .deck(0, &["COST-LOW"; 12])
            .deck(1, &["COST-LOW"; 12])
            .memory(3)
            .start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        let source = r.place_on_field(0, "COST-SOURCE", None);
        r.place_on_field(1, "COST-GATE", None);
        let source_card = r.game.players[0].battle_area[source.index as usize]
            .top_card()
            .handle();
        let physical = r.game.players[1].hand[0].handle();
        {
            let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), controller);
            assert!(ctx
                .play_from_hand_with_cost(1, 0, CostDelta::Reduce(0))
                .is_none());
        }
        assert!(
            r.pending_selection_view().is_some(),
            "actual optional cost reducer must park before play"
        );
        lock(&mut r, 0, 5000, Expiry::Permanent);
        r.game
            .set_effect_source_player_for_test(Some(1 - controller));
        let snapshot = r.game.clone();
        for cloned in [false, true] {
            r.game = snapshot.clone();
            if cloned {
                r.game = r.game.clone();
            }
            let actor = r.game.pending_selection.as_ref().unwrap().selecting_player;
            let pass = digimon_engine::action::space::PASS;
            assert!(build_action_mask(&r.game, actor)[pass as usize] > 0.0);
            r.game.resolve_selection(actor, pass).unwrap();
            assert!(r.pending_selection_view().is_none());
            assert_eq!(
                r.game.players[1]
                    .battle_area
                    .iter()
                    .any(|p| p.top_card().handle() == physical),
                controller == 1
            );
            assert_eq!(
                r.game.players[1]
                    .hand
                    .iter()
                    .any(|c| c.handle() == physical),
                controller == 0
            );
        }
    }
}

#[test]
fn dp_ceiling_reads_the_dual_digimon_face_instead_of_compatibility_dp() {
    use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
    use digimon_engine::enums::CardColor;
    for (compatibility_dp, face_dp, allowed) in [
        (None, 5000, false),
        (Some(1000), 6000, true),
        (Some(9000), 5000, false),
    ] {
        let mut card = make_test_card("FACE-DUAL", "Dual face control");
        card.card_kind = CardKind::Dual;
        card.dp = compatibility_dp;
        card.dual = Some(DualCardData {
            digimon: DualDigimonFace {
                level: 4,
                dp: face_dp,
                colors: vec![CardColor::Red],
                traits: vec![],
                evo_costs: vec![],
                effect_text: String::new(),
                inherited_text: String::new(),
                keywords: vec![],
            },
            option: DualOptionFace {
                use_cost: 1,
                colors: vec![CardColor::Red],
                effect_text: String::new(),
                security_text: String::new(),
                keywords: vec![],
            },
        });
        let mut r = DebugRunner::builder()
            .add_card(card)
            .hand(0, &["FACE-DUAL"])
            .start();
        r.skip_mulligan();
        lock(&mut r, 0, 5000, Expiry::Permanent);
        let physical = r.game.players[0].hand[0].handle();
        assert_eq!(
            r.game
                .can_play_card_from_effect_without_cost_for_controller(0, physical, 0, 0),
            allowed,
            "compatibility={compatibility_dp:?} face={face_dp}"
        );
        assert_eq!(
            r.game
                .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByEffect)
                .is_some(),
            allowed
        );
    }
}

struct ParkMaterialLeave;
impl digimon_engine::effect::CardEffect for ParkMaterialLeave {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![
            digimon_engine::effect::Effect::when_would_leave_battle_area(c)
                .optional()
                .name("Material leave control")
                .replacement_process(|ctx| ctx.cancel())
                .build(),
        ]
    }
}
#[test]
fn digixros_material_and_leave_parking_keep_declared_effect_controller() {
    for controller in [0u8, 1] {
        for stage in ["material", "leave"] {
            let mut r = DebugRunner::builder()
                .from_dsl_yaml(
                    r#"
card: XROS-LOW
name: Xros low
kind: digimon
level: 4
color: [red]
cost: 3
dp: 5000
alt_paths:
  - kind: digixros
    cost: 3
    materials:
      - filter: { name_is: Xros material }
        repeat: { min: 1, max: 1 }
        zones: [battle_area]
        cost_delta: -1
"#,
                )
                .unwrap()
                .add_card(make_test_card("XROS-MAT", "Xros material"))
                .add_card(make_test_card("XROS-SOURCE", "Xros source"))
                .hand(1, &["XROS-LOW"])
                .deck(0, &["XROS-MAT"; 12])
                .deck(1, &["XROS-MAT"; 12])
                .memory(3)
                .start();
            r.skip_mulligan();
            r.game.current_phase = GamePhase::Main;
            let source = r.place_on_field(0, "XROS-SOURCE", None);
            r.register_effect("XROS-MAT", std::sync::Arc::new(ParkMaterialLeave));
            r.place_on_field(1, "XROS-MAT", None);
            let source_card = r.game.players[0].battle_area[source.index as usize]
                .top_card()
                .handle();
            let physical = r.game.players[1].hand[0].handle();
            {
                let mut ctx =
                    EffectContext::new(&mut r.game, source_card, Some(source), controller);
                assert!(ctx
                    .play_from_hand_with_cost(1, 0, CostDelta::Reduce(0))
                    .is_none());
            }
            assert_eq!(
                r.pending_selection_view().unwrap().kind,
                digimon_engine::selection::SelectionKind::Material
            );
            if stage == "leave" {
                let actor = r.game.pending_selection.as_ref().unwrap().selecting_player;
                let pick = r.pending_selection_view().unwrap().valid_action_ids[0];
                assert!(build_action_mask(&r.game, actor)[pick as usize] > 0.0);
                r.game.resolve_selection(actor, pick).unwrap();
                // An exhausted material chooser may still require its ordinary stop.
                if r.pending_selection_view()
                    .is_some_and(|v| v.kind == digimon_engine::selection::SelectionKind::Material)
                {
                    let actor = r.game.pending_selection.as_ref().unwrap().selecting_player;
                    let pass = digimon_engine::action::space::PASS;
                    assert!(build_action_mask(&r.game, actor)[pass as usize] > 0.0);
                    r.game.resolve_selection(actor, pass).unwrap();
                }
                assert_eq!(
                    r.pending_selection_view().unwrap().kind,
                    digimon_engine::selection::SelectionKind::Replacement
                );
            }
            lock(&mut r, 0, 5000, Expiry::Permanent);
            r.game
                .set_effect_source_player_for_test(Some(1 - controller));
            let snapshot = r.game.clone();
            for cloned in [false, true] {
                r.game = snapshot.clone();
                if cloned {
                    r.game = r.game.clone();
                }
                let actor = r.game.pending_selection.as_ref().unwrap().selecting_player;
                let pass = digimon_engine::action::space::PASS;
                assert!(build_action_mask(&r.game, actor)[pass as usize] > 0.0);
                r.game.resolve_selection(actor, pass).unwrap();
                assert!(
                    r.pending_selection_view().is_none(),
                    "complete {stage} control"
                );
                assert_eq!(
                    r.game.players[1]
                        .battle_area
                        .iter()
                        .any(|p| p.top_card().handle() == physical),
                    controller == 1,
                    "{stage} controller={controller}"
                );
                assert_eq!(
                    r.game.players[1]
                        .hand
                        .iter()
                        .any(|c| c.handle() == physical),
                    controller == 0
                );
            }
        }
    }
}

#[test]
fn late_prohibition_after_cost_prompt_restores_trash_origin() {
    for controller in [0u8, 1] {
        let mut low = make_test_card("COST-LOW", "Cost low");
        low.dp = Some(5000);
        low.play_cost = 2;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(
                r#"
card: COST-GATE
name: Cost gate
kind: tamer
color: [red]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_any_ally_played: { name_is: Cost low }
    optional: true
    amount: 1
"#,
            )
            .unwrap()
            .add_card(low)
            .add_card(make_test_card("COST-SOURCE", "Cost source"))
            .deck(0, &["COST-LOW"; 12])
            .deck(1, &["COST-LOW"; 12])
            .memory(3)
            .start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        let source = r.place_on_field(0, "COST-SOURCE", None);
        r.place_on_field(1, "COST-GATE", None);
        let source_card = r.game.players[0].battle_area[source.index as usize]
            .top_card()
            .handle();
        r.inject_trash(1, "COST-LOW");
        let physical = r.game.players[1].trash[0].handle();
        {
            let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), controller);
            assert!(ctx
                .play_from_trash_with_cost(1, 0, CostDelta::Reduce(0))
                .is_none());
        }
        assert!(
            r.pending_selection_view().is_some(),
            "actual optional cost reducer must park before play"
        );
        lock(&mut r, 0, 5000, Expiry::Permanent);
        r.game
            .set_effect_source_player_for_test(Some(1 - controller));
        let snapshot = r.game.clone();
        for cloned in [false, true] {
            r.game = snapshot.clone();
            if cloned {
                r.game = r.game.clone();
            }
            let actor = r.game.pending_selection.as_ref().unwrap().selecting_player;
            let pass = digimon_engine::action::space::PASS;
            assert!(build_action_mask(&r.game, actor)[pass as usize] > 0.0);
            r.game.resolve_selection(actor, pass).unwrap();
            assert!(r.pending_selection_view().is_none());
            assert_eq!(
                r.game.players[1]
                    .battle_area
                    .iter()
                    .any(|p| p.top_card().handle() == physical),
                controller == 1
            );
            assert_eq!(
                r.game.players[1]
                    .trash
                    .iter()
                    .any(|c| c.handle() == physical),
                controller == 0,
                "failed resumed play restores actual origin"
            );
            assert!(!r.game.players[1]
                .hand
                .iter()
                .any(|c| c.handle() == physical));
        }
    }
}
