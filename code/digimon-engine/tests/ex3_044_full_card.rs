//! Full EX3-044 printed card. BREAK_TEST_YAML selects root-controlled RED baseline.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, encode_digivolve, ATTACK_START, PASS, SECURITY_TARGET},
    },
    card_data::CardData,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, Expiry, Keyword, ModifierType},
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
};
fn card(id: &str, name: &str, dp: i32, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.dp = Some(dp);
    c.level = Some(level);
    c.colors = vec![color];
    c
}
fn fixture() -> DebugRunner {
    let y = std::fs::read_to_string(
        std::env::var("BREAK_TEST_YAML")
            .unwrap_or_else(|_| format!("{}/cards/ex3/EX3-044.yaml", env!("CARGO_MANIFEST_DIR"))),
    )
    .unwrap();
    let mut alias = card(
        "BREAK-ALIAS",
        "Plain alias host",
        16000,
        7,
        CardColor::Green,
    );
    alias.also_treated_as.push("Examon".into());
    let mut text = card("BREAK-TEXT", "Plain text host", 16000, 7, CardColor::Green);
    text.effect_text = "Examon is mentioned in this text".into();
    let mut tamer = make_test_card("BREAK-TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut evader = card("BREAK-EVADER", "Wingdramon", 16000, 7, CardColor::Blue);
    evader.keywords.push(Keyword::Evade);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&y)
        .unwrap()
        .add_card(evader)
        .add_card(alias)
        .add_card(text)
        .add_card(tamer)
        .add_card(card("BREAK-INDRA", "Indramon", 7000, 5, CardColor::Red))
        .add_card(card("BREAK-HOST", "Examon", 16000, 7, CardColor::Green))
        .add_card(card(
            "BREAK-DRAMON",
            "Wingdramon",
            16000,
            5,
            CardColor::Blue,
        ))
        .add_card(card(
            "BREAK-GROUND",
            "Groundramon",
            7000,
            5,
            CardColor::Green,
        ))
        .add_card(card(
            "BREAK-PLAIN",
            "Plain host",
            16000,
            7,
            CardColor::Green,
        ))
        .add_card(card(
            "BREAK-GREEN",
            "Plain green",
            7000,
            5,
            CardColor::Green,
        ))
        .add_card(card("BREAK-BLUE", "Plain blue", 7000, 5, CardColor::Blue))
        .add_card(card("BREAK-RED", "Plain red", 7000, 5, CardColor::Red))
        .add_card(card(
            "BREAK-WEAK",
            "Unrelated weak Digimon",
            3000,
            3,
            CardColor::Red,
        ))
        .add_card(card(
            "BREAK-TIE",
            "Unrelated equal Digimon",
            16000,
            7,
            CardColor::Red,
        ))
        .hand(0, &["EX3-044"])
        .deck(0, &["BREAK-WEAK"; 20])
        .deck(1, &["BREAK-WEAK"; 20])
        .security(1, &["BREAK-WEAK"; 5])
        .security(0, &["BREAK-WEAK"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn act(r: &mut DebugRunner, a: u16) {
    let p = r.game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(
        build_action_mask(&r.game, p)[a as usize],
        1.0,
        "legal choice {a}"
    );
    r.execute_action(p, a).unwrap();
}
fn observer(r: &mut DebugRunner, inherited: bool) -> PermanentHandle {
    if inherited {
        r.place_stack(0, &["EX3-044", "BREAK-PLAIN"])
    } else {
        r.place_on_field(0, "EX3-044", Some(0))
    }
}
#[test]
fn breakdramon_has_printed_name_and_all_digivolution_routes() {
    for (id, costs) in [
        ("BREAK-GREEN", vec![4]),
        ("BREAK-BLUE", vec![4]),
        ("BREAK-GROUND", vec![3, 4]),
        ("BREAK-DRAMON", vec![3, 4]),
    ] {
        let mut r = fixture();
        let h = r.place_on_field(0, id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        if costs.len() > 1 {
            let pending = r
                .game
                .pending_selection
                .as_ref()
                .expect("named source offers special and normal routes");
            assert!(!pending.is_optional);
            assert_eq!(pending.valid_action_ids.len(), costs.len());
            let entries = pending.effect_choices.as_ref().unwrap().clone();
            assert_eq!(
                entries.iter().map(|e| e.label.clone()).collect::<Vec<_>>(),
                costs
                    .iter()
                    .map(|c| format!("Digivolve for cost {c}"))
                    .collect::<Vec<_>>()
            );
            assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
            let saved = r.game.clone();
            for (entry, cost) in entries.iter().zip(costs.iter()) {
                r.game = saved.clone();
                act(&mut r, entry.action_id);
                assert!(r.game.pending_selection.is_none());
                assert_eq!(r.memory(), 10 - cost);
                assert_eq!(
                    r.game.players[0].battle_area[0]
                        .top_card()
                        .card_name(&r.game.card_data),
                    "Breakdramon"
                );
                assert_eq!(r.game.players[0].hand.len(), 1, "normal evolution draw");
            }
        } else {
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.memory(), 10 - costs[0]);
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_name(&r.game.card_data),
                "Breakdramon"
            );
            assert_eq!(r.game.players[0].hand.len(), 1, "normal evolution draw");
        }
    }
    let mut r = fixture();
    let h = r.place_on_field(0, "BREAK-RED", Some(0));
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, h.index as u16) as usize],
        0.0
    );
}
#[test]
fn breakdramon_self_suspend_offers_every_opponent_digimon_and_no_decline_in_clones() {
    let mut r = fixture();
    let me = observer(&mut r, false);
    r.place_on_field(0, "BREAK-WEAK", Some(0));
    r.place_on_field(1, "BREAK-WEAK", Some(0));
    r.place_on_field(1, "BREAK-WEAK", Some(0));
    r.place_on_field(1, "BREAK-TAMER", None);
    r.game.players[1].battle_area[1].is_suspended = true;
    r.game.suspend(me);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[ATTACK_START as usize], 1.0);
    assert_eq!(mask[ATTACK_START as usize + 1], 1.0);
    assert_eq!(mask[ATTACK_START as usize + 2], 0.0);
    assert_eq!(mask[PASS as usize], 0.0);
    let saved = r.game.clone();
    for pick in [0, 1] {
        r.game = saved.clone();
        act(&mut r, ATTACK_START + pick);
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.players[1].battle_area[pick as usize].is_suspended);
        assert!(r.game.players[0].battle_area[0].is_suspended);
        assert!(!r.game.players[0].battle_area[1].is_suspended);
    }
}
#[test]
fn breakdramon_suspend_is_self_only_face_only_and_once_per_turn() {
    for inherited in [false, true] {
        let mut r = fixture();
        let me = observer(&mut r, inherited);
        let ally = r.place_on_field(0, "BREAK-WEAK", Some(0));
        r.place_on_field(1, "BREAK-WEAK", Some(0));
        r.game.suspend(ally);
        assert!(r.game.pending_selection.is_none());
        r.game.suspend(me);
        if inherited {
            assert!(r.game.pending_selection.is_none());
            continue;
        }
        act(&mut r, ATTACK_START);
        r.game.players[0].battle_area[me.index as usize].is_suspended = false;
        r.game.players[1].battle_area[0].is_suspended = false;
        r.game.suspend(me);
        assert!(r.game.pending_selection.is_none());
        assert!(!r.game.players[1].battle_area[0].is_suspended);
    }
}
#[test]
fn breakdramon_empty_target_suspend_finishes_and_spends_mandatory_opt() {
    let mut r = fixture();
    let me = observer(&mut r, false);
    r.game.suspend(me);
    assert!(r.game.pending_selection.is_none());
    r.place_on_field(1, "BREAK-WEAK", Some(0));
    r.game.players[0].battle_area[0].is_suspended = false;
    r.game.suspend(me);
    assert!(r.game.pending_selection.is_none());
    assert!(!r.game.players[1].battle_area[0].is_suspended);
}
#[test]
fn breakdramon_face_and_inherited_watch_other_own_name_qualified_winners_on_both_turns() {
    for inherited in [false, true] {
        for (id, expected) in [
            ("BREAK-HOST", 4),
            ("BREAK-DRAMON", 4),
            ("BREAK-ALIAS", 4),
            ("BREAK-PLAIN", 5),
            ("BREAK-TEXT", 5),
        ] {
            for turn in [0, 1] {
                let mut r = fixture();
                observer(&mut r, inherited);
                let ally = r.place_on_field(0, id, Some(0));
                let victim = r.place_on_field(1, "BREAK-WEAK", Some(0));
                r.game.turn_player_idx = turn;
                r.battle_digimon(ally, victim);
                let _ = r.auto_resolve();
                assert_eq!(r.game.players[1].battle_area.len(), 0);
                assert_eq!(
                    r.game.players[1].security.len(),
                    expected,
                    "inherited={inherited} id={id} turn={turn}"
                );
            }
        }
    }
}
#[test]
fn breakdramon_own_carrier_win_fires_once_without_both_face_and_inherited() {
    for inherited in [false, true] {
        for defender in [false, true] {
            let mut r = fixture();
            let me = if inherited {
                r.place_stack(0, &["EX3-044", "BREAK-HOST"])
            } else {
                observer(&mut r, false)
            };
            let victim = r.place_on_field(1, "BREAK-WEAK", Some(0));
            if defender {
                r.battle_digimon(victim, me);
            } else {
                r.battle_digimon(me, victim);
            }
            let _ = r.auto_resolve();
            assert_eq!(r.game.players[1].security.len(), 4);
            let victim = r.place_on_field(1, "BREAK-WEAK", Some(0));
            r.battle_digimon(me, victim);
            let _ = r.auto_resolve();
            assert_eq!(r.game.players[1].security.len(), 4, "once per turn");
        }
    }
}
#[test]
fn breakdramon_requires_actual_battle_deletion_and_surviving_winner() {
    for inherited in [false, true] {
        for prevent in [false, true] {
            let mut r = fixture();
            observer(&mut r, inherited);
            let ally = r.place_on_field(0, "BREAK-HOST", Some(0));
            let victim =
                r.place_on_field(1, if prevent { "BREAK-WEAK" } else { "BREAK-TIE" }, Some(0));
            if prevent {
                r.game.modifiers.add(
                    victim,
                    ModifierEntry::simple(
                        ModifierType::CannotBeDestroyedByBattle,
                        0,
                        Expiry::Permanent,
                        1,
                    ),
                );
            }
            r.battle_digimon(ally, victim);
            let _ = r.auto_resolve();
            assert_eq!(
                r.game.players[1].security.len(),
                5,
                "prevent={prevent} inherited={inherited}"
            );
        }
    }
}
#[test]
fn breakdramon_opponent_name_qualified_win_does_not_trigger_our_clause() {
    for inherited in [false, true] {
        let mut r = fixture();
        observer(&mut r, inherited);
        let victim = r.place_on_field(0, "BREAK-WEAK", Some(0));
        let enemy = r.place_on_field(1, "BREAK-HOST", Some(0));
        r.battle_digimon(enemy, victim);
        let _ = r.auto_resolve();
        assert_eq!(r.game.players[1].security.len(), 5);
        assert_eq!(r.game.players[0].security.len(), 5);
    }
}
#[test]
fn breakdramon_suspend_and_battle_clauses_have_independent_opt() {
    let mut r = fixture();
    let me = observer(&mut r, false);
    let victim = r.place_on_field(1, "BREAK-WEAK", Some(0));
    r.game.suspend(me);
    act(&mut r, ATTACK_START);
    r.battle_digimon(me, victim);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[1].security.len(), 4);
}
#[test]
fn breakdramon_security_battle_does_not_count_as_deleting_a_digimon() {
    let mut r = fixture();
    let me = observer(&mut r, true);
    let attacker = r.place_on_field(0, "BREAK-HOST", Some(0));
    let a = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[1].security.len(),
        4,
        "only ordinary security check, no inherited trash"
    );
    assert_eq!(
        r.game.players[0].battle_area[me.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "BREAK-PLAIN"
    );
}

/// An equal-DP battle has no winner (rule 14-2-1-3), but the printed
/// "deletes ... and survives" condition can still be met through Evade.
#[test]
fn breakdramon_tie_survivor_accept_and_decline_are_distinct_in_clones() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for inherited in [false, true] {
        let mut r = fixture();
        observer(&mut r, inherited);
        let ally = r.place_on_field(0, "BREAK-EVADER", Some(0));
        let victim = r.place_on_field(1, "BREAK-TIE", Some(0));
        r.battle_digimon(ally, victim);
        let pending = r.game.pending_selection.as_ref().expect("own Evade choice");
        assert_eq!(pending.selecting_player, 0);
        assert!(pending.is_optional);
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(mask[PASS as usize], 1.0);
        assert_eq!(mask[REPLACEMENT_ACCEPT as usize], 1.0);
        let saved = r.game.clone();
        for accept in [false, true] {
            r.game = saved.clone();
            act(&mut r, if accept { REPLACEMENT_ACCEPT } else { PASS });
            r.auto_resolve().unwrap();
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.players[1].battle_area.len(), 0);
            assert_eq!(r.game.players[0].battle_area.len(), if accept { 2 } else { 1 });
            assert_eq!(r.game.players[1].security.len(), if accept { 4 } else { 5 },
                "inherited={inherited}, accept={accept}");
            if accept { assert!(r.game.players[0].battle_area[1].is_suspended); }
        }
    }
}
#[test]
fn breakdramon_opponents_tie_survivor_does_not_trigger_our_clause() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for inherited in [false, true] {
        let mut r = fixture();
        observer(&mut r, inherited);
        let victim = r.place_on_field(0, "BREAK-TIE", Some(0));
        let enemy = r.place_on_field(1, "BREAK-EVADER", Some(0));
        r.battle_digimon(victim, enemy);
        assert_eq!(r.game.pending_selection.as_ref().unwrap().selecting_player, 1);
        act(&mut r, REPLACEMENT_ACCEPT);
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert_eq!(r.game.players[1].battle_area.len(), 1);
        assert_eq!(r.game.players[1].security.len(), 5);
        assert_eq!(r.game.players[0].security.len(), 5);
    }
}


struct RemoveBattleSurvivorOnDeletion { target: PermanentHandle, bounce: bool }
impl digimon_engine::effect::CardEffect for RemoveBattleSurvivorOnDeletion {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<digimon_engine::effect::Effect> {
        let target = self.target;
        let bounce = self.bounce;
        vec![digimon_engine::effect::Effect::on_deletion(card)
            .name("Remove the combatant after battle deletion")
            .process(move |ctx| {
                if bounce { assert!(ctx.return_to_hand(target).is_some()); }
                else { ctx.delete_permanent(target); }
            }).build()]
    }
}

#[test]
fn breakdramon_battle_survival_is_not_revoked_by_later_on_deletion_removal() {
    for inherited in [false, true] {
        for bounce in [false, true] {
            let mut r = fixture();
            observer(&mut r, inherited);
            let ally = r.place_on_field(0, "BREAK-HOST", Some(0));
            r.register_effect("BREAK-WEAK", std::sync::Arc::new(RemoveBattleSurvivorOnDeletion { target: ally, bounce }));
            let victim = r.place_on_field(1, "BREAK-WEAK", Some(0));
            let saved = r.game.clone();
            for _ in 0..2 {
                r.game = saved.clone();
                r.battle_digimon(ally, victim);
                r.auto_resolve().unwrap();
                assert!(r.game.pending_selection.is_none());
                assert_eq!(r.game.players[0].battle_area.len(), 1, "observer remains; combatant removed");
                assert!(r.game.players[1].battle_area.is_empty());
                assert_eq!(r.game.players[1].security.len(), 4,
                    "battle survived before later OnDeletion: inherited={inherited}, bounce={bounce}");
            }
        }
    }
}

#[test]
fn breakdramon_separate_physical_observers_each_trigger_on_native_defender_survival() {
    let mut r = fixture();
    observer(&mut r, false);
    observer(&mut r, true);
    let defender = r.place_on_field(0, "BREAK-HOST", Some(0));
    let attacker = r.place_on_field(1, "BREAK-WEAK", Some(0));
    r.game.players[0].battle_area[defender.index as usize].is_suspended = true;
    r.set_first_player(1);
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let action = encode_attack(attacker.index as u16, defender.index as u16);
    assert_eq!(build_action_mask(&r.game, 1)[action as usize], 1.0);
    r.game.decode_action(action, 1);
    r.auto_resolve().unwrap();
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(r.game.players[0].battle_area.len(), 3);
    assert_eq!(r.game.players[1].security.len(), 3, "one trigger from each distinct card");
}

#[test]
fn breakdramon_two_face_and_two_inherited_copies_keep_individual_opt() {
    let mut r = fixture();
    for inherited in [false,false,true,true] { observer(&mut r,inherited); }
    let ally=r.place_on_field(0,"BREAK-HOST",Some(0));
    let first=r.place_on_field(1,"BREAK-WEAK",Some(0));
    r.battle_digimon(ally,first);r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].security.len(),1,"four distinct clauses each resolve once");
    let second=r.place_on_field(1,"BREAK-WEAK",Some(0));
    r.battle_digimon(ally,second);r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].security.len(),1,"each physical clause spent its own OPT");
}

#[test]
fn breakdramon_native_attack_suspension_targets_cannot_suspend_and_spends_opt() {
    let mut r=fixture();let me=observer(&mut r,false);
    let target=r.place_on_field(1,"BREAK-WEAK",Some(0));
    r.game.modifiers.add(target,ModifierEntry::simple(ModifierType::CannotSuspend,1,Expiry::Permanent,1));
    let attack=encode_attack(me.index as u16,SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game,0)[attack as usize],1.0);
    r.game.decode_action(attack,0);
    assert!(r.game.players[0].battle_area[me.index as usize].is_suspended);
    assert_eq!(build_action_mask(&r.game,0)[ATTACK_START as usize],1.0);
    act(&mut r,ATTACK_START);r.auto_resolve().unwrap();
    assert!(!r.game.players[1].battle_area[target.index as usize].is_suspended);
    assert_eq!(r.game.players[1].security.len(),4,"security battle is not field deletion");
    r.game.players[0].battle_area[me.index as usize].is_suspended=false;
    r.game.suspend(me);
    assert!(r.game.pending_selection.is_none(),"mandatory suspend clause already activated");
}

#[test]
fn breakdramon_surviving_tie_without_opponent_deletion_does_not_trash_security() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for inherited in [false,true] {
        let mut r=fixture();observer(&mut r,inherited);
        let ally=r.place_on_field(0,"BREAK-EVADER",Some(0));
        let enemy=r.place_on_field(1,"BREAK-TIE",Some(0));
        r.game.modifiers.add(enemy,ModifierEntry::simple(ModifierType::CannotBeDestroyedByBattle,1,Expiry::Permanent,1));
        r.battle_digimon(ally,enemy);act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(),2);
        assert_eq!(r.game.players[1].battle_area.len(),1);
        assert_eq!(r.game.players[1].security.len(),5);
    }
}

/// Synthetic attack-phase unsuspend enables an Evade choice during a native
/// attack. It is fixture behavior, not an extra printed Breakdramon effect.
struct ReadyAttacker;
impl digimon_engine::effect::CardEffect for ReadyAttacker {
    fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
        vec![digimon_engine::effect::Effect::when_attacking(c).process(|ctx|{
            if let Some(h)=ctx.source_permanent {ctx.unsuspend(h);}
        }).build()]
    }
}
#[test]
fn breakdramon_native_attacker_tie_evade_accept_decline_and_continuation() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for inherited in [false,true] {
        let mut r=fixture();observer(&mut r,inherited);
        r.register_effect("BREAK-EVADER",std::sync::Arc::new(ReadyAttacker));
        let ally=r.place_on_field(0,"BREAK-EVADER",Some(0));
        let enemy=r.place_on_field(1,"BREAK-TIE",Some(0));
        r.game.players[1].battle_area[enemy.index as usize].is_suspended=true;
        let action=encode_attack(ally.index as u16,enemy.index as u16);
        assert_eq!(build_action_mask(&r.game,0)[action as usize],1.0);
        r.game.decode_action(action,0);
        // Advance mandatory/optional battle windows only until the actual
        // Evade offer; never auto-answer the discriminator itself.
        for _ in 0..12 {
            let pending=r.game.pending_selection.as_ref().expect("attack reaches Evade");
            if pending.kind==digimon_engine::selection::SelectionKind::Replacement {break;}
            let actor=pending.selecting_player;
            assert_eq!(build_action_mask(&r.game,actor)[PASS as usize],1.0);
            r.game.resolve_selection(actor,PASS).unwrap();r.game.drain_effect_queue();
        }
        assert_eq!(r.game.pending_selection.as_ref().unwrap().selecting_player,0);
        assert_eq!(build_action_mask(&r.game,0)[REPLACEMENT_ACCEPT as usize],1.0);
        let saved=r.game.clone();
        for accept in [false,true] {
            r.game=saved.clone();act(&mut r,if accept {REPLACEMENT_ACCEPT}else{PASS});r.auto_resolve().unwrap();
            assert!(r.game.pending_selection.is_none());
            assert!(r.game.pending_attack.is_none(),"native attack finishes after replacement");
            assert_eq!(r.game.players[1].battle_area.len(),0);
            assert_eq!(r.game.players[1].security.len(),if accept {4}else{5});
        }
    }
}

#[test]
fn breakdramon_dramon_name_follows_official_language_exception() {
    // Bandai Language Standardization Rules, page4: Indramon is explicitly
    // not treated as including Dramon. This synthetic battle fixture keeps
    // its printed name and DP; only irrelevant card behavior is absent.
    for inherited in [false,true] {
        let mut r=fixture();observer(&mut r,inherited);
        let ally=r.place_on_field(0,"BREAK-INDRA",Some(0));
        let victim=r.place_on_field(1,"BREAK-WEAK",Some(0));
        r.battle_digimon(ally,victim);r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(),2);
        assert!(r.game.players[1].battle_area.is_empty());
        assert_eq!(r.game.players[1].security.len(),5,"Indramon is not a Dramon rules-name match");
    }
}

#[test]
fn breakdramon_renamed_battle_winner_does_not_regain_printed_dramon_name() {
    use digimon_engine::modifiers::ModifierPayload;
    for inherited in [false,true] {
        for name in ["Plain", "Indramon", "Groundramon"] {
            let mut r=fixture();observer(&mut r,inherited);
            let ally=r.place_on_field(0,"BREAK-DRAMON",Some(0));
            r.game.modifiers.add(ally,ModifierEntry::simple(ModifierType::ChangeBaseCardName,0,Expiry::EndOfTurn,0).with_payload(ModifierPayload::Name{value:name.into(),base:true}));
            let victim=r.place_on_field(1,"BREAK-WEAK",Some(0));
            r.battle_digimon(ally,victim);r.auto_resolve().unwrap();
            assert!(r.game.players[1].battle_area.is_empty());
            assert_eq!(r.game.players[1].security.len(),if name=="Groundramon" {4} else {5},"effective winner {name}, inherited={inherited}");
        }
    }
}
