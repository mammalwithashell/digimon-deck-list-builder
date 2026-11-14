//! Behavioral qualification for the shared constructs blocking EX13 Royal Knights.
//! No admission-policy changes: positive and negative assertions exercise runtime DSL.
#[path = "../support/owned_card_choice.rs"]
mod physical_menu;
use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind};

fn card(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 0;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

#[test]
fn self_digivolution_sources_trait_has_excludes_top_and_other_stacks_in_both_zones() {
    // CR 4.3 4-8-1: the top card is not a digivolution card.
    for breeding in [false, true] {
        for sources_match in [false, true] {
            let mut runner = DebugRunner::builder()
                .add_card(card("RK", &["Royal Knight"]))
                .add_card(card("OTHER", &["Knight"]))
                .build();
            runner.place_stack(0, &["RK", "OTHER"]); // unrelated matching stack
            let ids = if sources_match { ["RK", "OTHER"] } else { ["OTHER", "RK"] };
            let h = runner.place_stack(0, &ids);
            let source = runner.game.players[0].battle_area[h.index as usize].top_card().handle();
            let subject = if breeding {
                let p = runner.game.players[0].battle_area.remove(h.index as usize);
                runner.game.players[0].breeding_area = Some(p);
                PredicateSubject::BreedingPermanent(0)
            } else { PredicateSubject::Permanent(h) };
            let rctx = EffectReadContext::new(&runner.game, source, None, 0);
            let predicate = CompiledPredicate {
                self_digivolution_sources_trait_has: Some("royal knight".into()),
                ..Default::default()
            };
            assert_eq!(eval_predicate_with_bindings(&predicate, &rctx, subject, None), sources_match);
            let no_substring = CompiledPredicate {
                self_digivolution_sources_trait_has: Some("Royal".into()),
                ..Default::default()
            };
            assert!(!eval_predicate_with_bindings(&no_substring, &rctx, subject, None));
        }
    }
}

#[test]
fn trash_breeding_permanent_trashes_whole_selected_stack_and_preserves_other_zones() {
    let yaml = r#"
card: RK-TRASH
name: Breeding trash probe
kind: digimon
color: [white]
level: 6
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_own_breeding_permanent:
          bind_as: chosen
          filter: { kind: digimon }
          prompt: Choose breeding Digimon
      - trash_breeding_permanent: { target: chosen }
"#;
    let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
        .add_card(card("BASE", &[])).add_card(card("TOP", &[])).build();
    for owner in [0, 1] {
        let h = runner.place_stack(owner, &["BASE", "TOP"]);
        let p = runner.game.players[owner as usize].battle_area.remove(h.index as usize);
        runner.game.players[owner as usize].breeding_area = Some(p);
    }
    let probe = runner.place_on_field(0, "RK-TRASH", None);
    let expected: Vec<_> = runner.game.players[0].breeding_area.as_ref().unwrap()
        .card_sources.iter().map(|c| c.handle()).collect();
    runner.fire_on_play(0, probe.index as usize);
    runner.execute_action(0, encode_breeding_select(0).unwrap()).unwrap();
    assert!(runner.game.players[0].breeding_area.is_none());
    assert!(runner.game.players[1].breeding_area.is_some());
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.game.players[0].trash.iter().map(|c| c.handle()).collect::<Vec<_>>(), expected);
    assert!(runner.game.players[1].trash.is_empty());
    // No target is a no-op, not an accidental battle-area deletion.
    runner.fire_on_play(0, probe.index as usize);
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.game.players[0].trash.len(), 2);
}

#[test]
fn printed_keyword_requires_main_face_not_inherited_security_or_reminder_text() {
    for (effect, inherited, security, expected) in [
        ("＜Blocker＞", "", "", true),
        ("", "＜Blocker＞", "", false),
        ("", "", "＜Blocker＞", false),
        ("[On Play] 1 of your Digimon gains ＜Blocker＞ for the turn.", "", "", false),
        ("＜Reboot＞", "", "", false),
    ] {
        let mut candidate = card("CANDIDATE", &[]);
        candidate.effect_text = effect.into();
        candidate.inherited_text = inherited.into();
        candidate.security_text = security.into();
        let runner = DebugRunner::builder().add_card(candidate).hand(0, &["CANDIDATE"]).build();
        let handle = runner.game.players[0].hand[0].handle();
        let ctx = EffectReadContext::new(&runner.game, handle, None, 0);
        let predicate = CompiledPredicate { printed_keyword: Some("Blocker".into()), ..Default::default() };
        assert_eq!(eval_predicate_with_bindings(&predicate, &ctx, PredicateSubject::Card(handle), None), expected, "{effect} / {inherited} / {security}");
    }
}

#[test]
fn event_card_text_contains_reads_played_card_not_observer_or_unrelated_field() {
    let yaml = r#"
card: RK-OBSERVER
name: Knightmon Observer
kind: digimon
color: [white]
level: 6
cost: 0
dp: 12000
effects:
  - when: on_any_digimon_played
    trigger_if: { event_card_text_contains: kNiGhTmOn }
    process:
      - gain_memory: 1
"#;
    for field in ["effect", "inherited", "security", "none"] {
        let mut played = card("PLAYED", &[]);
        match field {
            "effect" => played.effect_text = "[On Play] Reveal a Knightmon.".into(),
            "inherited" => played.inherited_text = "[Your Turn] Knightmon gets +1000 DP.".into(),
            "security" => played.security_text = "[Security] Play Knightmon.".into(),
            _ => {},
        }
        let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
            .add_card(played).hand(0, &["PLAYED"]).memory(5).start();
        runner.place_on_field(0, "RK-OBSERVER", None);
        runner.play(0, 0).unwrap();
        runner.auto_resolve().unwrap();
        assert_eq!(runner.game.memory, if field == "none" {5} else {6}, "{field}");
    }
}

#[test]
fn breeding_trash_processes_overflow_on_remaining_sources() {
    // Official BT13-112 Q2369 and CR 4.3 4-19: even under a breeding card.
    let yaml = r#"
card: RK-OVERFLOW
name: Breeding trash overflow probe
kind: digimon
color: [white]
level: 6
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_own_breeding_permanent:
          bind_as: chosen
          filter: { kind: digimon }
          prompt: Choose breeding Digimon
      - trash_breeding_permanent: { target: chosen }
"#;
    let mut ace = card("ACE", &[]);
    ace.ace_overflow = Some(-4);
    let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
        .add_card(ace).add_card(card("TOP", &[])).memory(8).start();
    let h = runner.place_stack(0, &["ACE", "TOP"]);
    let p = runner.game.players[0].battle_area.remove(h.index as usize);
    runner.game.players[0].breeding_area = Some(p);
    let probe = runner.place_on_field(0, "RK-OVERFLOW", None);
    runner.fire_on_play(0, probe.index as usize);
    runner.execute_action(0, encode_breeding_select(0).unwrap()).unwrap();
    assert_eq!(runner.game.memory, 4, "ACE leaving from under breeding must pay Overflow");
    assert_eq!(runner.game.players[0].trash.len(), 2);
}

#[test]
fn select_materials_require_max_name_allows_zero_or_all_distinct_names() {
    use digimon_engine::action::space::PASS;
    let yaml = r#"
card: RK-ALL-NAMES
name: All names source probe
kind: digimon
color: [white]
level: 6
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_materials:
          of_permanent: source
          max: 12
          uniqueness: name
          require_max: true
          optional_zero: true
          bind_as: picked
          prompt: Choose all different names or none
      - gain_memory: 1
"#;
    for decline in [false, true] {
        let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
            .add_card(card("A", &[])).add_card(card("B", &[])).memory(5).start();
        let host = runner.place_stack(0, &["A", "A", "B", "RK-ALL-NAMES"]);
        runner.fire_on_play(0, host.index as usize);
        assert!(runner.pending_is_optional());
        if decline {
            runner.execute_action(0, PASS).unwrap();
        } else {
            let a = physical_menu::source_action(&runner.game, host, 0);
            runner.execute_action(0, a).unwrap();
            assert!(!runner.pending_is_optional());
            assert!(!runner.pending_selection_view().unwrap().valid_action_ids.contains(&PASS));
            let untouched = runner.game.clone();
            runner.game = runner.game.clone();
            let b = physical_menu::source_action(&runner.game, host, 2);
            runner.execute_action(0, b).unwrap();
            assert!(untouched.pending_selection.is_some(), "clone resolution must not mutate original");
            assert_eq!(untouched.memory, 5);
        }
        assert!(runner.pending_selection_view().is_none());
        assert_eq!(runner.game.memory, 6);
    }
}

#[test]
fn event_target_level_lte_uses_deleted_snapshot_not_observer_level() {
    use digimon_engine::replacement::ReplacementCause;
    let yaml = r#"
card: RK-LEVEL
name: Deleted level observer
kind: digimon
color: [white]
level: 6
cost: 0
dp: 12000
effects:
  - when: on_any_deletion
    trigger_if: { event_target_level_lte: 5 }
    process:
      - gain_memory: 1
"#;
    for level in [None, Some(3), Some(5), Some(6)] {
        for owner in [0, 1] {
            let mut target = card("TARGET", &[]);
            target.level = level;
            let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
                .add_card(target).memory(4).start();
            runner.place_on_field(0, "RK-LEVEL", None);
            let victim = runner.place_on_field(owner, "TARGET", None);
            runner.game.delete_permanent_with_cause(victim, ReplacementCause::OwnEffect);
            runner.game.drain_effect_queue();
            assert_eq!(runner.memory(), if matches!(level, Some(3 | 5)) {5} else {4}, "level={level:?}, owner={owner}");
        }
    }
}

#[test]
fn bind_permanent_property_color_count_reads_selected_top_not_its_sources_or_observer() {
    let yaml=r#"
card: RK-COLORS
name: Selected colors
kind: digimon
color: [white]
level: 6
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
          prompt: Choose target
      - bind_permanent_property: { from: target, property: color_count, bind_as: colors }
      - gain_memory: { binding_value: colors }
"#;
    for colors in [vec![CardColor::Blue],vec![CardColor::Blue,CardColor::Red]] {
        let n=colors.len() as i16;let mut target=card("COLOR-TARGET",&[]);target.colors=colors;
        let mut r=DebugRunner::builder().from_dsl_yaml(yaml).unwrap().add_card(target).add_card(card("WHITE-SOURCE",&[])).memory(4).start();
        r.place_stack(1,&["WHITE-SOURCE","COLOR-TARGET"]);let h=r.place_on_field(0,"RK-COLORS",None);r.fire_on_play(0,h.index as usize);
        let p=r.pending_selection_view().unwrap();r.execute_action(p.selecting_player,p.valid_action_ids[0]).unwrap();assert_eq!(r.memory(),4+n);
    }
}

#[test]
fn can_digivolve_into_source_card_checks_exact_source_routes_and_prohibitions() {
    use digimon_engine::card_data::EvoCost;
    use digimon_engine::enums::{Expiry,ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut evo=card("EVOLUTION",&[]);evo.evo_costs=vec![EvoCost{card_color:1,level:3,memory_cost:4}];
    let mut legal=card("LEGAL",&[]);legal.colors=vec![CardColor::Blue];legal.level=Some(3);
    let mut r=DebugRunner::builder().add_card(evo).add_card(legal).add_card(card("WRONG",&[])).hand(0,&["EVOLUTION"]).build();
    let h=r.place_on_field(0,"LEGAL",None);let wrong=r.place_on_field(0,"WRONG",None);let source=r.game.players[0].hand[0].handle();
    let p=CompiledPredicate{can_digivolve_into_source_card:Some(true),..Default::default()};
    let check=|r:&DebugRunner,h| {let c=EffectReadContext::new(&r.game,source,None,0);eval_predicate_with_bindings(&p,&c,PredicateSubject::Permanent(h),None)};
    assert!(check(&r,h));assert!(!check(&r,wrong));
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::CannotDigivolve,1,Expiry::Permanent,0));assert!(!check(&r,h));
}
