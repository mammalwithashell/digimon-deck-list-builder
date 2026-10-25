use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    decode_attack, encode_attack, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT19-090";

fn xros_digimon(card_id: &str, name: &str, level: u8, dp: i32, cost: u16) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card.dp = Some(dp);
    card.play_cost = cost;
    card
}

fn named_digimon(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.colors = vec![CardColor::Red];
    card.dp = Some(5000);
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn security_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

fn choose_effect_mode(runner: &mut DebugRunner, player: PlayerId, label_substring: &str) {
    let Some(view) = runner.pending_selection_view() else {
        return;
    };
    if view.kind != SelectionKind::EffectChoice {
        return;
    }

    let needle = label_substring.to_ascii_lowercase();
    let action = view
        .effect_choices
        .as_ref()
        .and_then(|choices| {
            choices
                .iter()
                .find(|choice| choice.label.to_ascii_lowercase().contains(&needle))
                .map(|choice| choice.action_id)
        })
        .unwrap_or_else(|| {
            if needle.contains("attack") {
                *view
                    .valid_action_ids
                    .last()
                    .expect("effect-choice prompt should have an attack mode")
            } else {
                view.valid_action_ids[0]
            }
        });
    runner
        .execute_action(player, action)
        .expect("choose BT19-090 mode");
}

#[test]
fn bt19_090_main_mode_plays_low_dp_xros_heart_from_under_tamer_without_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-090 YAML loads")
        .add_card(tamer("TAMER"))
        .add_card(xros_digimon("LOW-DP-XROS", "Low DP Xros", 4, 4000, 6))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    let stash = runner.place_on_field(0, "TAMER", Some(0));
    runner.push_source(stash, "LOW-DP-XROS");
    runner.game.enter_main_phase();

    let memory_before = runner.memory();
    let _ = runner.game.play_option_from_hand(0, 0);
    choose_effect_mode(&mut runner, 0, "play");

    let prompt = runner
        .pending_selection_view()
        .expect("BT19-090 should ask which under-Tamer card to play");
    assert!(prompt.is_optional,"printed play mode remains declinable");
    let source_action = physical_menu::source_action(&runner.game,stash,0);
    runner
        .execute_action(0, source_action)
        .expect("select under-Tamer Xros Heart card");
    runner.auto_resolve().expect("finish BT19-090 play mode");

    assert_eq!(
        memory_before - runner.memory(),
        3,
        "BT19-090 should pay only the option use cost; the selected Digimon is played free"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LOW-DP-XROS"),
        "the selected low-DP Xros Heart card should be played"
    );
    assert!(
        !runner.game.players[0].battle_area[stash.index as usize]
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LOW-DP-XROS"),
        "the played card should leave the Tamer stack"
    );
}

#[test]
fn bt19_090_attack_mode_unsuspends_shoutmon_ex6_and_shootingstarmon_then_attacks_player() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-090 YAML loads")
        .add_card(named_digimon("SHOUTMON-EX6", "Shoutmon EX6"))
        .add_card(named_digimon("SHOOTING-STARMON", "ShootingStarmon"))
        .add_card(security_filler("SECURITY-CARD"))
        .hand(0, &[CARD_ID])
        .security(1, &["SECURITY-CARD"])
        .memory(10)
        .start();

    let shoutmon = runner.place_on_field(0, "SHOUTMON-EX6", Some(0));
    let shooting = runner.place_on_field(0, "SHOOTING-STARMON", Some(0));
    runner.game.players[0].battle_area[shoutmon.index as usize].is_suspended = true;
    runner.game.players[0].battle_area[shooting.index as usize].is_suspended = true;
    runner.game.enter_main_phase();

    let _ = runner.game.play_option_from_hand(0, 0);
    choose_effect_mode(&mut runner, 0, "attack");

    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if view.kind == SelectionKind::Target {
            break;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .expect("BT19-090 setup prompt should have a legal choice");
        runner
            .execute_action(0, action)
            .expect("resolve BT19-090 attack setup prompt");
    }

    assert!(
        !runner.game.players[0].battle_area[shoutmon.index as usize].is_suspended,
        "BT19-090 should unsuspend a chosen Shoutmon EX6 before attacking"
    );
    assert!(
        !runner.game.players[0].battle_area[shooting.index as usize].is_suspended,
        "BT19-090 should unsuspend a chosen ShootingStarmon before attacking"
    );

    let pending = runner
        .pending_selection_view()
        .expect("BT19-090 should install an attack target prompt");
    assert_eq!(pending.kind, SelectionKind::Target);
    let attack_action = pending
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| {
            let (_, target) = decode_attack(*action);
            target == SECURITY_TARGET
        })
        .expect("BT19-090 attack mode should attack a player");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[attack_action as usize], 1.0,
        "BT19-090's effect-driven attack must be exposed through the action mask"
    );
    assert_eq!(
        mask[encode_attack(shoutmon.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "an unsuspended named Digimon should be able to attack the player"
    );
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;

#[test]
fn bt19_090_main_and_security_may_decline_source_play_for_both_actors_and_clones() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::{PendingSecurity,TriggerSource};
    for actor in [0,1] { for security in [false,true] {
        let mut r=DebugRunner::builder().dsl_card(CARD_ID).unwrap().add_card(tamer("TAMER"))
            .add_card(xros_digimon("LOW-DP-XROS","Low DP Xros",4,4000,6)).hand(actor,&[CARD_ID]).memory(10).start();
        r.game.enter_main_phase();r.game.turn_player_idx=actor as usize;
        let host=r.place_on_field(actor,"TAMER",None);let physical=r.push_source(host,"LOW-DP-XROS");
        if security {
            let card=r.game.player_mut(actor).hand.remove(0);let handle=card.handle();
            r.game.pending_security=Some(PendingSecurity{defender:actor,card,played:false});
            r.game.enqueue_triggered(EffectTiming::SecuritySkill,TriggerSource::SecurityRevealed{defender:actor,card:handle});r.game.drain_effect_queue();
        } else {
            r.game.play_option_from_hand(actor,0);choose_effect_mode(&mut r,actor,"play");
        }
        let pending=r.game.pending_selection.as_ref().expect("may-play choice is public");assert!(pending.is_optional);
        let mask=build_action_mask(&r.game,actor);assert_eq!(mask[PASS as usize],1.0);
        let action=physical_menu::source_action(&r.game,host,0);assert_eq!(mask[action as usize],1.0);
        let before=r.memory();let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,PASS).unwrap();assert!(game.pending_selection.is_none());
            assert_eq!(game.memory,before,"declining adds no payment");assert_eq!(game.player(actor).battle_area.len(),1);
            assert_eq!(game.player(actor).battle_area[host.index as usize].card_sources[0].handle(),physical);
            assert_eq!(game.player(actor).battle_area[host.index as usize].card_sources.len(),2);
        }
        assert_eq!(before,if security{10}else{7},"main pays only the option use cost; Security pays none");
    }}
}
