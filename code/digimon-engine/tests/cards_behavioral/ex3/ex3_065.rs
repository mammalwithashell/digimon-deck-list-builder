//! EX3-065 image and official P2 observation dfdd563f61f01c3da9889155a550cdc4e44ac37ed3c414b999c2e7ab7a9a7df1.
//! Q3430/Q3431: each Hina resolves separately; the refired effect is one On Play.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;
const ID: &str = "EX3-065";
fn base(trait_name: &str, effects: bool) -> DebugRunnerBuilder {
    custom_base(
        trait_name,
        if effects {
            "effects:\n  - when: on_play\n    process:\n      - gain_memory: 1\n  - when: on_play\n    process:\n      - draw: { of: you, count: 2 }\n"
        } else {
            ""
        },
    )
}
fn custom_base(trait_name: &str, effects: &str) -> DebugRunnerBuilder {
    let yaml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/cards/ex3/EX3-065.yaml"
    ))
    .expect("EX3-065 production YAML must exist");
    let target = format!("card: TARGET\nname: Target\nkind: digimon\ncolor: [red]\nlevel: 4\ncost: 4\ndp: 4000\ntraits: ['{trait_name}']\nalt_paths:\n  - kind: digivolve\n    from: {{level_eq: 3, color_is: red}}\n    cost: 0\n{effects}");
    let mut lower = make_test_card("LOWER", "Lower");
    lower.level = Some(3);
    lower.colors = vec![CardColor::Red];
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .from_dsl_yaml(&target)
        .unwrap()
        .add_card(lower)
        .add_card(tamer)
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(5)
}
fn evolve(r: &mut DebugRunner, player: u8) {
    let h = r.place_on_field(player, "LOWER", Some(0));
    let hi = r.add_to_hand(player, "TARGET");
    if player == 0 && r.game.turn_player_idx == 0 {
        let a = encode_digivolve(hi as u16, h.index as u16);
        assert_eq!(build_action_mask(&r.game, player)[a as usize], 1.0);
        r.game.decode_action(a, player);
    } else {
        assert!(r.game.digivolve_from_hand(
            player,
            hi,
            h.index as usize,
            digimon_engine::enums::PlaySource::ByDigivolve
        ));
    }
    r.game.drain_effect_queue();
}
/// Resolve a pending `TriggerOrder` by picking its first entry (a no-op when
/// none is pending).
fn pick_trigger_order_first(r: &mut DebugRunner) {
    if let Some(v) = r.pending_selection_view() {
        if v.kind == SelectionKind::TriggerOrder {
            r.execute_action(0, v.valid_action_ids[0]).unwrap();
        }
    }
}
fn accept(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("optional paid activation");
    assert!(v.is_optional);
    let mask = build_action_mask(&r.game, v.selecting_player);
    assert_eq!(mask[PASS as usize], 1.0);
    assert_eq!(mask[v.valid_action_ids[0] as usize], 1.0);
    r.execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
}
#[test]
fn all_four_traits_offer_paid_one_of_two_on_play_choices_and_clone() {
    for trait_name in [
        "Rock Dragon",
        "Earth Dragon",
        "Machine Dragon",
        "Sky Dragon",
    ] {
        let mut r = base(trait_name, true).start();
        r.skip_mulligan();
        r.place_on_field(0, ID, Some(0));
        evolve(&mut r, 0);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        accept(&mut r);
        assert!(r.game.players[0].battle_area[0].is_suspended);
        let v = r
            .pending_selection_view()
            .expect("choose exactly one On Play");
        assert_eq!(v.kind, SelectionKind::EffectChoice);
        assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids.len(), 2);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        for a in &v.valid_action_ids {
            assert_eq!(build_action_mask(&r.game, 0)[*a as usize], 1.0);
        }
        let mut clone = DebugRunner::wrap(r.game.clone());
        r.execute_action(0, v.valid_action_ids[0]).unwrap();
        clone.execute_action(0, v.valid_action_ids[1]).unwrap();
        assert_eq!((r.memory(), r.hand_size(0)), (6, 1));
        assert_eq!((clone.memory(), clone.hand_size(0)), (5, 3));
    }
}
#[test]
fn decline_and_no_on_play_effect_preserve_real_optional_cost() {
    let mut r = base("Rock Dragon", true).start();
    r.skip_mulligan();
    r.place_on_field(0, ID, Some(0));
    evolve(&mut r, 0);
    r.execute_action(0, PASS).unwrap();
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.memory(), 5);
    evolve(&mut r, 0);
    accept(&mut r);
    assert!(r.pending_selection().is_some());
    let mut n = base("Rock Dragon", false).start();
    n.skip_mulligan();
    n.place_on_field(0, ID, Some(0));
    evolve(&mut n, 0);
    accept(&mut n);
    assert!(n.game.players[0].battle_area[0].is_suspended);
    assert!(n.pending_selection().is_none());
    assert_eq!(n.memory(), 5);
}
#[test]
fn wrong_trait_owner_turn_and_already_suspended_do_not_activate() {
    for case in 0..4 {
        let mut r = base(if case == 0 { "Dragonkin" } else { "Sky Dragon" }, true).start();
        r.skip_mulligan();
        r.place_on_field(0, ID, Some(0));
        if case == 2 {
            r.game.turn_player_idx = 1;
        }
        if case == 3 {
            r.game.players[0].battle_area[0].is_suspended = true;
        }
        evolve(&mut r, if case == 1 { 1 } else { 0 });
        assert!(r.pending_selection().is_none(), "case {case}");
    }
}
#[test]
fn cannot_suspend_hina_cannot_pay_even_when_target_has_on_play() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = base("Rock Dragon", true).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 1),
    );
    evolve(&mut r, 0);
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert_eq!((r.memory(), r.hand_size(0)), (5, 1));
}
#[test]
fn two_hinas_refire_separately_and_departed_event_target_is_not_another_slot() {
    for removed in [false, true] {
        let mut r = base("Rock Dragon", true).start();
        r.skip_mulligan();
        r.place_on_field(0, ID, Some(0));
        r.place_on_field(0, ID, Some(0));
        evolve(&mut r, 0);
        // Simultaneous trigger-order choice (both optional, so it can be
        // declined as a whole), then the selected Hina's own "you may": the
        // pick only decides the order (§15-4-3-5-1, §15-7; DCGO asks each
        // optional effect as it activates).
        pick_trigger_order_first(&mut r);
        accept(&mut r);
        let v = r.pending_selection_view().unwrap();
        assert_eq!(v.kind, SelectionKind::EffectChoice);
        r.execute_action(0, v.valid_action_ids[0]).unwrap();
        assert_eq!(r.memory(), 6);
        if removed {
            let h = digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 2,
            };
            // This is an identity-rebinding fixture while Hina's optional
            // prompt is already parked. The full deletion/replacement driver
            // cannot be entered during an unrelated pending selection: it
            // parks without deleting the subject. Q3431 below separately
            // exercises real deletion through a refired On Play effect.
            let departed_card = r.game.players[0].battle_area[2].top_card().handle();
            r.game.players[0].delete_permanent(h.index as usize);
            assert_eq!(r.battle_area_size(0), 2);
            assert!(r.game.players[0]
                .trash
                .iter()
                .any(|c| c.handle() == departed_card));
            let replacement = r.place_on_field(0, "TARGET", Some(0));
            assert_eq!(replacement.index, h.index);
            assert_ne!(
                r.game.players[0].battle_area[2].top_card().handle(),
                departed_card
            );
        }
        if let Some(v) = r.pending_selection_view() {
            if !v.is_optional {
                r.execute_action(0, v.valid_action_ids[0]).unwrap();
            }
        }
        if r.pending_selection().is_some() {
            accept(&mut r);
        }
        if removed {
            assert!(
                r.pending_selection().is_none(),
                "dead target cannot refire a replacement slot: {:?}",
                r.pending_selection_view()
            );
            assert_eq!(r.memory(), 6);
        } else {
            let v = r
                .pending_selection_view()
                .expect("second Hina's effect choice");
            r.execute_action(0, v.valid_action_ids[1]).unwrap();
            assert_eq!((r.memory(), r.hand_size(0)), (6, 3));
            assert!(r.game.players[0].battle_area[0].is_suspended);
            assert!(r.game.players[0].battle_area[1].is_suspended);
        }
    }
}
#[test]
fn start_turn_memory_requires_opponent_battle_area_digimon() {
    for mode in 0..4 {
        let mut r = base("Rock Dragon", true).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, ID, Some(0));
        match mode {
            0 => {
                r.place_on_field(1, "LOWER", Some(0));
            }
            1 => {
                r.place_on_field(1, "TAMER", Some(0));
            }
            2 => {
                r.place_in_breeding(1, "LOWER");
            }
            _ => {}
        }
        r.game.enqueue_triggered(
            digimon_engine::enums::EffectTiming::StartOfYourTurn,
            digimon_engine::selection::TriggerSource::Permanent(h),
        );
        r.game.drain_effect_queue();
        assert_eq!(r.memory(), if mode == 0 { 6 } else { 5 });
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn security_plays_hina_free_and_normal_play_cost_is_three() {
    let mut r = base("Rock Dragon", false).security(1, &[ID]).start();
    r.skip_mulligan();
    let a = r.place_on_field(0, "LOWER", Some(0));
    r.attack_player(a, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
    let mut p = base("Rock Dragon", false).hand(0, &[ID]).start();
    p.skip_mulligan();
    p.play(0, 0).unwrap();
    assert_eq!(p.memory(), 2);
}
#[test]
fn q3431_new_on_deletion_resolves_before_second_hina() {
    let effects = r#"effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: Choose a Digimon
      - delete_permanent: { target: victim }
"#;
    let victim = r#"card: VICTIM
name: Victim
kind: digimon
color: [purple]
level: 3
cost: 3
dp: 1000
effects:
  - when: on_deletion
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: Choose a Digimon
      - delete_permanent: { target: victim }
"#;
    let mut r = custom_base("Rock Dragon", effects)
        .from_dsl_yaml(victim)
        .unwrap()
        .start();
    r.skip_mulligan();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "VICTIM", Some(0));
    evolve(&mut r, 0);
    pick_trigger_order_first(&mut r);
    accept(&mut r);
    let v = r.pending_selection_view().expect("refired On Play target");
    assert_eq!(v.selecting_player, 0);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let v = r
        .pending_selection_view()
        .expect("new On Deletion must preempt second Hina");
    assert_eq!(v.selecting_player, 1);
    assert!(v.is_optional);
    r.execute_action(1, v.valid_action_ids[0]).unwrap();
    let v = r
        .pending_selection_view()
        .expect("opponent selects evolved Digimon");
    assert_eq!(v.selecting_player, 1);
    assert_eq!(v.valid_action_ids.len(), 1);
    r.execute_action(1, v.valid_action_ids[0]).unwrap();
    // Q3430: the second Hina cannot activate the departed Digimon's On Play.
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(r.battle_area_size(1), 0);
    assert!(r.pending_selection().is_none());
}
#[test]
fn refired_on_play_is_digimon_effect_not_hinas_tamer_effect() {
    use digimon_engine::enums::{EffectSourceKind, Expiry};
    use digimon_engine::modifiers::ModifierEntry;
    let effects = r#"effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: Choose a Digimon
      - delete_permanent: { target: victim }
"#;
    for kind in [EffectSourceKind::Tamer, EffectSourceKind::Digimon] {
        let mut r = custom_base("Rock Dragon", effects).start();
        r.skip_mulligan();
        r.place_on_field(0, ID, Some(0));
        let h = r.place_on_field(1, "LOWER", Some(0));
        r.game.modifiers.add(
            h,
            ModifierEntry::cannot_be_affected_by_opponents_source_kind(kind, Expiry::Permanent, 1),
        );
        evolve(&mut r, 0);
        accept(&mut r);
        if let Some(v) = r.pending_selection_view() {
            r.execute_action(0, v.valid_action_ids[0]).unwrap();
        }
        r.auto_resolve().unwrap();
        assert_eq!(
            r.battle_area_size(1),
            if kind == EffectSourceKind::Digimon {
                1
            } else {
                0
            },
            "On Play attribution against {kind:?} immunity"
        );
    }
}
