//! BT20-087: printed image, official bundle Q&A (field includes breeding),
//! and DCGO BT20_087.cs. CR 4.3 3-4-7: breeding evolution suppresses WD.
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ID: &str = "BT20-087";
fn digimon(id: &str, level: u8, chronicle: bool, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(8000);
    c.colors = vec![color];
    if chronicle {
        c.traits = vec!["Chronicle".into()];
    }
    c
}
fn result(id: &str, level: u8, chronicle: bool, color: CardColor) -> CardData {
    let mut c = digimon(id, level, chronicle, CardColor::Black);
    c.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level: 5,
        memory_cost: 3,
    }];
    c
}
fn builder(hand: &[&str]) -> DebugRunnerBuilder {
    let mut fake = result("FAKE", 6, true, CardColor::Black);
    fake.card_kind = CardKind::Tamer;
    fake.level = None;
    fake.dp = None;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-087 production YAML")
        .dsl_card("BT20-056")
        .expect("real Chronicle with When Digivolving Recovery")
        .add_card(digimon("ATTACKER", 5, true, CardColor::Blue))
        .add_card(digimon("OTHER", 5, false, CardColor::Blue))
        .add_card(digimon("BASE", 5, false, CardColor::Black))
        .add_card(digimon("NO-ROUTE", 3, false, CardColor::Yellow))
        .add_card(result("EVO6", 6, true, CardColor::Black))
        .add_card(result("EVO5", 5, true, CardColor::Black))
        .add_card(result("EVO7", 7, true, CardColor::Black))
        .add_card(result("WRONG-TRAIT", 6, false, CardColor::Black))
        .add_card(result("WRONG-COLOR", 6, true, CardColor::Green))
        .add_card(fake)
        .hand(0, hand)
        .deck(0, &["OTHER"; 30])
        .deck(1, &["OTHER"; 30])
        .security(0, &["OTHER"; 3])
        .security(1, &["OTHER"; 3])
        .memory(5)
}
fn ready(mut r: DebugRunner) -> DebugRunner {
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    let card = r.top_card(h);
    r.game.enqueue_triggered(
        EffectTiming::OnAllyAttack,
        TriggerSource::PlayerBattleAreaAttack {
            player: h.player,
            attacker: h,
            card,
        },
    );
    r.game.drain_effect_queue();
}
fn choices(r: &DebugRunner, n: usize, optional: bool) -> Vec<u16> {
    let p = r.pending_selection().expect("visible player choice");
    assert_eq!(p.selecting_player, 0);
    assert_eq!(p.is_optional, optional, "{}", p.prompt);
    let mut ids: Vec<_> = p
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != space::PASS)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids.len(), n, "{} {:?}", p.prompt, ids);
    for (i, v) in build_action_mask(&r.game, 0).iter().enumerate() {
        assert_eq!(
            *v,
            if ids.contains(&(i as u16)) || (optional && i == space::PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask {i}"
        );
    }
    assert!(build_action_mask(&r.game, 1).iter().all(|v| *v == 0.0));
    ids
}
fn pick(r: &mut DebugRunner, a: u16) {
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.execute_action(0, a).expect("legal choice");
}
fn accept(r: &mut DebugRunner) {
    let a = choices(r, 1, true)[0];
    pick(r, a);
}
fn top(r: &DebugRunner, breeding: bool, index: usize) -> String {
    let p = if breeding {
        r.game.players[0].breeding_area.as_ref().unwrap()
    } else {
        &r.game.players[0].battle_area[index]
    };
    p.top_card().card_id(&r.game.card_data).to_string()
}
fn setup(hand: &[&str], breeding: bool) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = ready(builder(hand).start());
    let t = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "ATTACKER", Some(0));
    if breeding {
        r.place_in_breeding(0, "BASE");
    } else {
        r.place_on_field(0, "BASE", Some(0));
    }
    (r, t, a)
}
fn host_pick(_r: &DebugRunner, breeding: bool) -> u16 {
    if breeding {
        space::encode_breeding_select(0).unwrap()
    } else {
        100 + 2
    }
}
fn result_pick(r: &DebugRunner, id: &str) -> u16 {
    space::PLAY_HAND_START
        + r.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .unwrap() as u16
}

#[test]
fn bt20_087_printed_metadata_and_both_static_names() {
    let r = builder(&[]).start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.kind, digimon_dsl::compiled::CompiledCardKind::Tamer);
    assert_eq!(c.cost, Some(5));
    assert_eq!(
        c.color,
        vec![
            digimon_dsl::compiled::CompiledColor::Black,
            digimon_dsl::compiled::CompiledColor::Red
        ]
    );
    assert_eq!(c.traits, vec!["Chronicle"]);
    assert_eq!(c.also_treated_as, vec!["Kota Domoto", "Yuji Musya"]);
}
#[test]
fn bt20_087_memory_setter_at_two_and_below_preserves_three_and_above() {
    for (before, after) in [(-2, 3), (0, 3), (2, 3), (3, 3), (6, 6)] {
        let mut r = ready(builder(&[]).start());
        let t = r.place_on_field(0, ID, Some(0));
        r.game.memory = before;
        r.game
            .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(t));
        r.game.drain_effect_queue();
        assert_eq!(r.memory(), after);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt20_087_attack_evolves_battle_or_breeding_with_reduced_cost_and_one_draw() {
    for breeding in [false, true] {
        let (mut r, t, a) = setup(&["EVO6"], breeding);
        fire(&mut r, a);
        accept(&mut r);
        assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
        let host = host_pick(&r, breeding);
        assert!(choices(&r, 1, true).contains(&host));
        pick(&mut r, host);
        let evo = result_pick(&r, "EVO6");
        assert_eq!(choices(&r, 1, true), vec![evo]);
        pick(&mut r, evo);
        assert_eq!(top(&r, breeding, 2), "EVO6");
        assert_eq!(r.memory(), 3);
        assert_eq!(r.hand_size(0), 1, "consumed one result and drew once");
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt20_087_both_field_zones_are_visible_choices_and_exclude_opponents_and_tamers() {
    let (mut r, _, a) = setup(&["EVO6"], false);
    r.place_in_breeding(0, "BASE");
    r.place_on_field(0, "NO-ROUTE", None);
    r.place_on_field(1, "BASE", None);
    fire(&mut r, a);
    accept(&mut r);
    let ids = choices(&r, 2, false);
    let p = r.pending_selection().unwrap();
    let labels: Vec<_> = p
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert!(labels.iter().any(|s| s.contains("battle")));
    assert!(labels.iter().any(|s| s.contains("breeding")));
    assert_eq!(ids.len(), 2);
    let choose = p
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.label.contains("battle"))
        .unwrap()
        .action_id;
    pick(&mut r, choose);
    assert_eq!(choices(&r, 1, true), vec![102]);
}
#[test]
fn bt20_087_hand_result_filters_level_trait_kind_and_legal_evolution() {
    for breeding in [false, true] {
        let (mut r, _, a) = setup(
            &["EVO6", "EVO5", "EVO7", "WRONG-TRAIT", "WRONG-COLOR", "FAKE"],
            breeding,
        );
        fire(&mut r, a);
        accept(&mut r);
        let host = host_pick(&r, breeding);
        pick(&mut r, host);
        let expected = vec![result_pick(&r, "EVO6"), result_pick(&r, "EVO5")];
        assert_eq!(choices(&r, 2, true), expected);
        let evo = result_pick(&r, "EVO5");
        pick(&mut r, evo);
        assert_eq!(top(&r, breeding, 2), "EVO5");
    }
}
#[test]
fn bt20_087_decline_before_suspension_and_each_later_pick() {
    for stage in 0..3 {
        let (mut r, t, a) = setup(&["EVO6"], false);
        fire(&mut r, a);
        if stage > 0 {
            accept(&mut r);
        }
        if stage > 1 {
            pick(&mut r, 102);
        }
        let mut clone = r.game.clone();
        assert_eq!(build_action_mask(&r.game, 0)[space::PASS as usize], 1.0);
        pick(&mut r, space::PASS);
        clone.decode_action(space::PASS, 0);
        assert_eq!(top(&r, false, 2), "BASE");
        assert_eq!(r.memory(), 5);
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(
            r.game.players[0].battle_area[t.index as usize].is_suspended,
            stage > 0
        );
        assert_eq!(clone.memory, r.memory());
        assert!(r.pending_selection().is_none());
        assert!(clone.pending_selection.is_none());
    }
}
#[test]
fn bt20_087_attack_trigger_requires_own_chronicle_digimon_and_your_turn() {
    for (id, owner, turn) in [
        ("OTHER", 0, 0),
        ("FAKE", 0, 0),
        ("ATTACKER", 1, 0),
        ("ATTACKER", 0, 1),
    ] {
        let (mut r, t, _) = setup(&["EVO6"], false);
        let a = r.place_on_field(owner, id, None);
        r.game.turn_player_idx = turn;
        fire(&mut r, a);
        assert!(r.pending_selection().is_none());
        assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    }
}
#[test]
fn bt20_087_suspended_tamer_and_missing_legal_or_payable_result_are_not_offered() {
    for case in 0..5 {
        let (mut r, t, a) = setup(
            if case == 2 {
                &["WRONG-COLOR"]
            } else {
                &["EVO6"]
            },
            false,
        );
        if case == 0 {
            r.game.players[0].battle_area[t.index as usize].is_suspended = true;
        }
        if case == 1 {
            r.game.players[0].hand.clear();
        }
        if case == 3 {
            r.game.memory = -9;
        }
        if case == 4 {
            r.game.players[0].battle_area.pop();
        }
        fire(&mut r, a);
        assert!(r.pending_selection().is_none(), "case {case}");
        assert_eq!(
            r.game.players[0].battle_area[t.index as usize].is_suspended,
            case == 0
        );
    }
}
#[test]
fn bt20_087_trash_card_is_not_a_hand_evolution_candidate() {
    let (mut r, t, a) = setup(&["EVO6"], true);
    let evo = r.game.players[0].hand.remove(0);
    r.game.players[0].trash.push(evo);
    fire(&mut r, a);
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert_eq!(top(&r, true, 0), "BASE");
}
#[test]
fn bt20_087_breeding_suppresses_real_when_digivolving_but_battle_activates_it() {
    for breeding in [false, true] {
        let (mut r, _, a) = setup(&["BT20-056"], breeding);
        let before = r.security_count(0);
        fire(&mut r, a);
        accept(&mut r);
        let host = host_pick(&r, breeding);
        pick(&mut r, host);
        let evo = result_pick(&r, "BT20-056");
        pick(&mut r, evo);
        assert_eq!(top(&r, breeding, 2), "BT20-056");
        assert_eq!(r.security_count(0), before + usize::from(!breeding));
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt20_087_unsuspending_tamer_allows_another_attack_without_artificial_opt() {
    let (mut r, t, a) = setup(&["EVO6"], false);
    fire(&mut r, a);
    accept(&mut r);
    pick(&mut r, space::PASS);
    r.game.players[0].battle_area[t.index as usize].is_suspended = false;
    fire(&mut r, a);
    accept(&mut r);
    assert_eq!(choices(&r, 1, true), vec![102]);
}
#[test]
fn bt20_087_security_plays_without_paying_five() {
    let mut r = ready(builder(&[]).security(1, &[ID]).start());
    let a = r.place_on_field(0, "OTHER", Some(0));
    let before = r.memory();
    r.attack_player(a, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
}
