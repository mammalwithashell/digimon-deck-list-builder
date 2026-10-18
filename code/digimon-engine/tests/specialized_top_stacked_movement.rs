//! Official Q4388/Q4723 and Q5218/Q5238 boundary tests.
//! Keep existing six full-card regression tests unchanged.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{PASS, REPLACEMENT_ACCEPT},
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, StackPosition};
use std::sync::Arc;
struct EndAttack;
impl CardEffect for EndAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_attack(c)
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}
fn invisimon(tamer: bool) -> DebugRunner {
    invisimon_with(tamer, false)
}
/// `tie`: LOWER keeps make_test_card's 2000 DP, tying the 2000-DP FILL.
fn invisimon_with(tamer: bool, tie: bool) -> DebugRunner {
    let mut lower = make_test_card("LOWER", "Underlying card");
    if tamer {
        lower.card_kind = CardKind::Tamer;
        lower.level = None;
        lower.dp = None;
    } else if !tie {
        // Invisimon's check trigger resolves BEFORE the Security Digimon battle
        // (§13-1-8-2 then §13-1-8-3-1; b41f3f5f1), so the battle is fought by
        // the Digimon now topped by LOWER. It must out-DP the 2000-DP FILL
        // Security Digimon, or both lose and LOWER is deleted (§14-2-1-3,
        // §14-2-2) before its [End of Attack] can trigger.
        lower.dp = Some(3000);
    }
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-055")
        .unwrap()
        .add_card(lower)
        .add_card(make_test_card("FILL", "Filler"))
        .security(1, &["FILL"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    if !tamer {
        r.register_effect("LOWER", Arc::new(EndAttack));
    }
    r.skip_mulligan();
    r.game.memory = 3;
    r.place_stack(0, &["LOWER", "BT20-055"]);
    let sec = r.game.players[1].security[0].card_index;
    r.game.players[1].face_up_security.insert(sec);
    r.game.enter_main_phase();
    r
}
#[test]
fn q4388_new_top_end_attack_effect_triggers_after_invisimon_top_enters_security() {
    let mut r = invisimon(false);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let lower = r.game.players[0].battle_area[0].card_sources[0].handle();
    assert_ne!(
        r.attack_player(h, 1, true),
        digimon_engine::combat::AttackResult::Invalid
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().handle(), lower);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(
        r.game.memory, 4,
        "Q4388: newly visible End of Attack effect activates"
    );
    assert!(r.game.players[0].trash.is_empty());
}
#[test]
fn security_check_trigger_resolves_first_so_lower_card_ties_security_digimon_and_is_deleted() {
    // §13-1-8-2-1: Invisimon's face-up-check trigger resolves before
    // §13-1-8-3-1's Security Digimon battle, so the battle is fought by the
    // Digimon now topped by the 2000-DP LOWER, not by the 11000-DP Invisimon.
    // 2000 vs FILL's 2000: both lose (§14-2-1-3); the Digimon is deleted
    // (§14-2-2), the Security Digimon is not (§14-2-3) and is trashed after
    // the check (§13-1-8-4). The deleted Digimon's [End of Attack] never fires.
    let mut r = invisimon_with(false, true);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let lower = r.game.players[0].battle_area[0].card_sources[0].handle();
    let top = r.game.players[0].battle_area[0].top_card().handle();
    let fill = r.game.players[1].security[0].handle();
    assert_ne!(
        r.attack_player(h, 1, true),
        digimon_engine::combat::AttackResult::Invalid
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].battle_area.is_empty(), "tied LOWER-topped Digimon deleted");
    let trash0: Vec<_> = r.game.players[0].trash.iter().map(|c| c.handle()).collect();
    assert_eq!(trash0, vec![lower], "only LOWER is trashed by the deletion");
    let trash1: Vec<_> = r.game.players[1].trash.iter().map(|c| c.handle()).collect();
    assert_eq!(trash1, vec![fill], "checked Security Digimon trashed, not deleted");
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(r.game.players[0].security[0].handle(), top, "Invisimon placed before the battle");
    assert!(r.game.players[0].face_up_security.contains(&top.0));
    assert_eq!(r.game.memory, 3, "a deleted Digimon's [End of Attack] does not trigger");
    assert!(r.game.pending_attack.is_none());
}
#[test]
fn q4723_only_tamer_below_invisimon_survives_as_tamer() {
    let mut r = invisimon(true);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let lower = r.game.players[0].battle_area[0].card_sources[0].handle();
    assert_ne!(
        r.attack_player(h, 1, true),
        digimon_engine::combat::AttackResult::Invalid
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    let p = &r.game.players[0].battle_area[0];
    assert_eq!(p.top_card().handle(), lower);
    assert!(p.is_tamer(&r.game.card_data));
    assert!(r.game.players[0].trash.is_empty());
    assert!(
        r.game.pending_attack.is_none(),
        "attacking Digimon became Tamer"
    );
}
#[test]
fn top_under_operation_rotates_greymon_and_garurumon_without_source_disposal() {
    for id in ["BT23-008", "BT23-018"] {
        let mut r = DebugRunner::builder()
            .dsl_card(id)
            .unwrap()
            .add_card(make_test_card("LOWER", "Lower"))
            .deck(0, &["LOWER"; 8])
            .deck(1, &["LOWER"; 8])
            .start();
        r.skip_mulligan();
        let h = r.place_stack(0, &["LOWER", id]);
        let top = r.game.players[0].battle_area[0].top_card().handle();
        r.game.players[0].battle_area[0].is_suspended = true;
        let saved = r.game.clone();
        assert!(EffectContext::new(&mut r.game, top, Some(h), 0).place_top_source_as_bottom(h));
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        let p = &r.game.players[0].battle_area[0];
        assert_eq!(p.card_sources[0].handle(), top);
        assert_eq!(p.top_card().card_id(&r.game.card_data), "LOWER");
        assert!(p.is_suspended);
        assert!(r.game.players[0].trash.is_empty());
        assert_eq!(saved.players[0].battle_area[0].top_card().handle(), top);
    }
}
struct BlockPlacement;
impl CardEffect for BlockPlacement {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![digimon_engine::effect::EffectBuilder::new(
            c,
            digimon_engine::enums::EffectTiming::WhenWouldPlaceInSecurity,
        )
        .optional()
        .replacement_process(|ctx| ctx.cancel())
        .build()]
    }
}
#[test]
fn optional_security_replacement_clone_retains_remainder_and_destination() {
    // Fresh runner so registry is fixed before field setup/cache construction.
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-055")
        .unwrap()
        .add_card(make_test_card("LOWER", "Lower"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .deck(0, &["LOWER"; 8])
        .deck(1, &["LOWER"; 8])
        .start();
    r.register_effect("OBSERVER", Arc::new(BlockPlacement));
    r.skip_mulligan();
    let h = r.place_stack(0, &["LOWER", "BT20-055"]);
    r.place_on_field(0, "OBSERVER", Some(0));
    let top = r.game.players[0].battle_area[0].top_card().handle();
    assert!(
        !EffectContext::new(&mut r.game, top, Some(h), 0).security_place_top_stacked_card(
            h,
            1,
            StackPosition::Bottom,
            true
        )
    );
    assert!(r.pending_selection().is_some());
    let saved = r.game.clone();
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.game.resolve_selection(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.game.players[1].security[0].handle(), top);
    assert!(r.game.players[1].face_up_security.contains(&top.0));
    r.game = saved;
    assert_eq!(
        build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
    assert!(r.game.players[1].security.is_empty());
}

#[test]
fn declining_first_security_replacement_still_offers_second() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-055")
        .unwrap()
        .add_card(make_test_card("LOWER", "Lower"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .deck(0, &["LOWER"; 8])
        .deck(1, &["LOWER"; 8])
        .start();
    r.register_effect("OBSERVER", Arc::new(BlockPlacement));
    r.skip_mulligan();
    let h = r.place_stack(0, &["LOWER", "BT20-055"]);
    r.place_on_field(0, "OBSERVER", Some(0));
    r.place_on_field(0, "OBSERVER", Some(0));
    let top = r.game.players[0].battle_area[0].top_card().handle();
    assert!(
        !EffectContext::new(&mut r.game, top, Some(h), 0).security_place_top_stacked_card(
            h,
            1,
            StackPosition::Bottom,
            true
        )
    );
    let order = r
        .pending_selection_view()
        .expect("two immediate effects require an ordering decision");
    assert_eq!(
        order.kind,
        digimon_engine::selection::SelectionKind::EffectChoice
    );
    assert_eq!(order.valid_action_ids.len(), 2);
    let first = order.valid_action_ids[0];
    assert_eq!(
        build_action_mask(&r.game, order.selecting_player)[first as usize],
        1.0
    );
    r.game
        .resolve_selection(order.selecting_player, first)
        .unwrap();
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.game.resolve_selection(0, PASS).unwrap();
    assert!(
        r.pending_selection().is_some(),
        "decline must not skip second optional replacement"
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
    assert!(r.game.players[1].security.is_empty());
}

// A departing top-card aura must disappear before Overflow's memory event
// evaluates an irreversible UntilCondition expiration.
const ACE_AURA: &str = r#"
card: ACE-TOP
name: Aura ACE instrumentation
kind: digimon
color: [red]
level: 6
cost: 10
dp: 10000
ace_overflow: -4
effects:
 - kind: aura
   target: { of: you, zone: [battle_area], kind: digimon }
   dp_modifier: 2000
"#;
#[test]
fn departed_top_aura_is_gone_before_overflow_until_condition_evaluation() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut lower = make_test_card("LOWER", "Lower");
    lower.dp = Some(2000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(ACE_AURA)
        .unwrap()
        .add_card(lower)
        .deck(0, &["LOWER"; 8])
        .deck(1, &["LOWER"; 8])
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    let h = r.place_stack(0, &["LOWER", "ACE-TOP"]);
    let survivor = r.place_on_field(0, "LOWER", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.game.effective_dp(survivor), Some(4000));
    r.game.modifiers.add(
        survivor,
        ModifierEntry::simple(ModifierType::CannotSuspend, 1, Expiry::UntilCondition, 0)
            .with_until_condition(Arc::new(move |g, _| {
                g.memory >= 3 || g.effective_dp(survivor) == Some(2000)
            })),
    );
    let top = r.game.players[0].battle_area[0].top_card().handle();
    assert!(
        EffectContext::new(&mut r.game, top, Some(h), 0).security_place_top_stacked_card(
            h,
            0,
            StackPosition::Top,
            false
        )
    );
    assert_eq!(r.game.memory, -1, "one owner-relative Overflow4");
    assert_eq!(r.game.effective_dp(survivor), Some(2000));
    assert_eq!(
        r.game.effective_dp(h),
        Some(2000),
        "lower host survives with refreshed identity"
    );
    assert!(
        r.game.modifiers.has(survivor, ModifierType::CannotSuspend),
        "stale departed-top aura must not cause irreversible early expiration"
    );
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert!(r.game.players[0].trash.is_empty());
}

fn rotation_then_play_rule_check(card: &str, partner_name: &str, no_dp: bool) {
    use digimon_engine::action::space::{FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START};
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut lower = make_test_card("LOWER", "Lower");
    if no_dp {
        lower.card_kind = CardKind::DigiEgg;
        lower.level = Some(2);
        lower.dp = None;
    } else {
        lower.dp = Some(1000);
    }
    let mut partner = make_test_card("PARTNER", partner_name);
    partner.play_cost = 3;
    let mut r = DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(lower)
        .add_card(partner)
        .hand(0, &["PARTNER"])
        .deck(0, &["PARTNER"; 8])
        .deck(1, &["PARTNER"; 8])
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    let h = r.place_stack(0, &["LOWER", card]);
    // Greymon's inherited +2000 applies to the new top; Garurumon's own-turn
    // inherited modifier does not. Counter exactly that plus the printed1000.
    if !no_dp {
        let delta = if card == "BT23-008" { -3000 } else { -1000 };
        r.game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::ChangeDp, delta, Expiry::EndOfTurn, 0),
        );
    }
    r.game.enter_main_phase();
    let action = FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    for _ in 0..2 {
        let view = r.pending_selection_view().expect("rotation/play decision");
        if view.kind == digimon_engine::selection::SelectionKind::Hand {
            break;
        }
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only optional activation consent precedes hand selection"
        );
        r.execute_action(view.selecting_player, view.valid_action_ids[0])
            .unwrap();
    }
    let view = r
        .pending_selection_view()
        .expect("play remains selectable despite transient invalid top");
    assert_eq!(view.kind, digimon_engine::selection::SelectionKind::Hand);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "rule check must wait while the effect's play decision remains pending"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "LOWER"
    );
    let choices: Vec<_> = view
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(choices.len(), 1);
    assert_eq!(build_action_mask(&r.game, 0)[choices[0] as usize], 1.0);
    r.execute_action(0, choices[0]).unwrap();
    r.auto_resolve().unwrap();
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "PARTNER"),
        "printed following play resolves before cleanup"
    );
    assert!(!r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "LOWER"));
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LOWER"));
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == card));
}
#[test]
fn q5218_q5238_no_dp_top_rule_check_waits_for_following_play() {
    rotation_then_play_rule_check("BT23-008", "Gabumon", true);
    rotation_then_play_rule_check("BT23-018", "Agumon", true);
}
#[test]
fn q5219_q5239_zero_dp_top_rule_check_waits_for_following_play() {
    rotation_then_play_rule_check("BT23-008", "Gabumon", false);
    rotation_then_play_rule_check("BT23-018", "Agumon", false);
}
