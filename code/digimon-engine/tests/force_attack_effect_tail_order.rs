//! Official EX13-045 Q7364/65: effect battle precedes Raid attack battle.
//! Q7366: Wingdramon suspension and Examon win triggers wait until WD finishes.
#[path = "support/owned_card_choice.rs"]
mod owned_choices;

use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, SECURITY_TARGET},
    },
    debug_runner::DebugRunner,
    enums::{GamePhase, Keyword},
    selection::SelectionKind,
};
fn act(r: &mut DebugRunner, a: u16) {
    let p = r
        .game
        .pending_selection
        .as_ref()
        .map(|x| x.selecting_player)
        .unwrap_or(r.game.turn_player());
    assert_eq!(
        build_action_mask(&r.game, p)[a as usize],
        1.0,
        "legal decision {a}"
    );
    r.execute_action(p, a).unwrap();
}
fn setup(wing: bool) -> DebugRunner {
    let blue = "ST2-11";
    let mut b = DebugRunner::builder();
    for id in [
        "EX13-045", "EX13-021", "ST2-11", "ST4-13", "ST1-03", "ST1-11", "BT20-023",
    ] {
        b = b.dsl_card(id).unwrap();
    }
    let mut r = b
        .hand(0, &["EX13-045", "BT20-023"])
        .deck(0, &["ST1-03"; 20])
        .deck(1, &["ST1-03"; 20])
        .security(1, &["ST1-03"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.place_on_field(0, "ST4-13", Some(0));
    if wing {
        r.place_stack(0, &["EX13-021", blue]);
    } else {
        r.place_on_field(0, blue, Some(0));
    }
    r.place_on_field(1, "ST1-11", Some(0));
    r.place_on_field(1, "ST1-03", Some(0));
    // Actual native DNA route, then the real forced-attack target chooser.
    assert_eq!(build_action_mask(&r.game, 0)[63], 1.0);
    r.game.decode_action(63, 0);
    act(&mut r, 0);
    act(&mut r, 1);
    let v = r.pending_selection_view().expect("mandatory attack target");
    assert!(v.prompt.contains("attacks"));
    r
}
fn assert_effect_battle_first(r: &DebugRunner) {
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("WD tail selection");
    assert_eq!(p.kind, SelectionKind::OppField);
    assert!(
        p.prompt.contains("may battle"),
        "the current WD clause must finish before Raid or triggered effects: {}",
        p.prompt
    );
    assert!(
        p.valid_action_ids.contains(&101),
        "weak Digimon remains a legal direct battle target even though Raid allows only highest DP"
    );
    assert_eq!(r.security_count(1), 5);
}
#[test]
fn forced_attack_defers_raid_until_current_effect_battle_choice_in_clones() {
    let mut r = setup(false);
    let g = r.game.clone();
    for _ in 0..2 {
        r.game = g.clone();
        act(&mut r, encode_attack(0, SECURITY_TARGET));
        assert_effect_battle_first(&r);
    }
}
#[test]
fn forced_attack_defers_inherited_suspend_trigger_until_current_effect_battle_choice() {
    let mut r = setup(true);
    act(&mut r, encode_attack(0, SECURITY_TARGET));
    assert_effect_battle_first(&r);
}
#[test]
fn effect_battle_win_and_declaration_suspend_offer_joint_order_after_wd() {
    let mut r = setup(true);
    act(&mut r, encode_attack(0, SECURITY_TARGET));
    assert_effect_battle_first(&r);
    let parked = r.game.clone();
    for _ in 0..2 {
        r.game = parked.clone();
        act(&mut r, 101);
        let p = r
            .game
            .pending_selection
            .as_ref()
            .expect("joint triggered order");
        assert_eq!(
            p.kind,
            SelectionKind::TriggerOrder,
            "Q7366 allows choosing either trigger first"
        );
        let ids: Vec<_> = p
            .effect_choices
            .as_ref()
            .unwrap()
            .iter()
            .filter_map(|x| {
                x.source_card
                    .and_then(|h| r.game.card_data_for_handle(h))
                    .map(|d| d.card_id.as_str())
            })
            .collect();
        assert!(
            ids.contains(&"EX13-021") && ids.contains(&"EX13-045"),
            "both triggers must be pending together: {ids:?}"
        );
        assert_eq!(r.security_count(1), 5);
    }
}

fn setup_completion(wing: bool) -> DebugRunner {
    let blue = "ST2-11";
    let mut b = DebugRunner::builder();
    for id in [
        "EX13-045", "EX13-021", "ST2-11", "ST4-13", "ST1-03", "ST1-11", "BT20-023", "EX9-013",
        "ST1-09", "ST5-11", "ST2-07",
    ] {
        b = b.dsl_card(id).unwrap();
    }
    let mut r = b
        .hand(0, &["EX13-045", "BT20-023"])
        .hand(1, &["EX9-013"])
        .deck(0, &["ST1-03"; 20])
        .deck(1, &["ST1-03"; 20])
        .security(1, &["ST1-03"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.place_on_field(0, "ST4-13", Some(0));
    if wing {
        r.place_stack(0, &["EX13-021", blue]);
    } else {
        r.place_on_field(0, blue, Some(0));
    }
    r.place_stack(1, &["ST5-11", "ST1-11"]);
    r.place_on_field(1, "ST1-03", Some(0));
    r.place_on_field(1, "ST1-09", Some(0));
    // Rule 12-1-5 excludes the Raid target itself; keep a distinct legal blocker.
    r.place_on_field(1, "ST2-07", Some(0));
    // Official observation 064fe77ff3f21af1997ea928feb09f4244c849274d76e03e4c3624ce001de64d.
    // DebugRunner DSL metadata omits printed text; hydrate actual card filter input.
    r.game.card_data.iter_mut().find(|d|d.card_id=="BT20-023").unwrap().effect_text = "＜Jamming＞\n[Your Turn] When any of your green Digimon with [Dracomon] or [Examon] in their texts are played, this Digimon may digivolve into [Wingdramon] in the hand with the digivolution cost reduced by 2.".to_string();
    // Actual native DNA route, then the real forced-attack target chooser.
    assert_eq!(build_action_mask(&r.game, 0)[63], 1.0);
    r.game.decode_action(63, 0);
    act(&mut r, 0);
    act(&mut r, 1);
    let v = r.pending_selection_view().expect("mandatory attack target");
    assert!(v.prompt.contains("attacks"));
    r
}

// Clone every installed prompt, resolve the identical legal action twice, and
// retain the cloned continuation for the rest of the complete line.
fn cloned_act(r: &mut DebugRunner, a: u16) {
    let parked = r.game.clone();
    act(r, a);
    let expected = r.game.verification_digest();
    r.game = parked;
    act(r, a);
    assert_eq!(r.game.verification_digest(), expected);
}
fn prompt_is(r: &DebugRunner, text: &str, actor: u8) {
    let p = r.game.pending_selection.as_ref().expect(text);
    assert!(p.prompt.contains(text), "wanted {text}, got {}", p.prompt);
    assert_eq!(p.selecting_player, actor);
}
// The card's printed (non-keyword) trigger from `id`. EX13-045's <Raid> is a
// trigger-type effect (§16-23-2) pending in the same TriggerOrder as its
// effect-battle payout, and so is its <Piercing>: the effect battle was fought
// while attacking and deleted the opponent's Digimon, so <Piercing> "triggers
// simultaneously with effects triggered by the battle" (§16-7-2, §16-7-4;
// official Q&A Q3931, Q7365). Both keywords are left until the printed
// triggers have resolved; `order_raid_and_piercing` then orders them.
fn ordered_source(r: &DebugRunner, id: &str) -> u16 {
    let p = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(p.kind, SelectionKind::TriggerOrder);
    p.effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .filter(|e| e.keyword.is_none())
        .find(|e| {
            e.source_card
                .and_then(|h| r.game.card_data_for_handle(h))
                .map(|d| d.card_id.as_str())
                == Some(id)
        })
        .expect("requested simultaneous source")
        .action_id
}
fn complete_trigger(r: &mut DebugRunner, id: &str) {
    let selected_order =
        r.game.pending_selection.as_ref().unwrap().kind == SelectionKind::TriggerOrder;
    if id == "EX13-021" {
        assert!(
            r.game.players[0].battle_area[0].is_suspended,
            "inherited effect has not yet activated"
        );
    }
    if selected_order {
        let a = ordered_source(r, id);
        cloned_act(r, a);
    }
    if id == "EX13-021" {
        // Picking a branch of a multi-entry TriggerOrder only decides the
        // activation order (§15-4-3-5-1); "it may unsuspend" keeps its own
        // accept / decline decision (§15-9-2-2, `Game::run_picked_trigger`), and
        // a lone remaining optional trigger installs the same outer gate.
        prompt_is(r, "EX13-021", 0);
        let p = r.game.pending_selection.as_ref().unwrap();
        assert!(p.is_optional);
        let a = *p
            .valid_action_ids
            .first()
            .expect("legal unsuspend acceptance");
        cloned_act(r, a);
        assert!(
            !r.game.players[0].battle_area[0].is_suspended,
            "accepted inherited effect unsuspends carrier"
        );
    } else {
        prompt_is(r, "Play or use 1", 0);
        assert!(matches!(
            r.game.pending_selection.as_ref().unwrap().kind,
            SelectionKind::UnionZone { .. }
        ));
        let i = r.game.players[0]
            .hand
            .iter()
            .position(|c| r.game.card_data[c.data_index].card_id == "BT20-023")
            .unwrap();
        let action = owned_choices::zone_action(&r.game, 0, i, digimon_engine::owned_card_choice::CardChoiceZone::Hand);
        cloned_act(r, action);
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| r.game.card_data[p.top_card().data_index].card_id == "BT20-023"));
    }
}
// After both printed triggers, the turn player still has <Raid> (from the
// attack declaration) and <Piercing> (from the effect battle) pending
// together, and orders them (§15-4-3-5-1). Activating <Piercing> only makes
// its security check pending processing, performed immediately before the
// End of Attack timing (§16-7-4) -- so no card is checked here -- and it is
// the attack's only check even though the attack battle deletes a Digimon
// too (§16-7-5; Q7364). `piercing_first` drives either order; with <Raid>
// first the lone <Piercing> left after the switch activates on its own.
fn order_raid_and_piercing(r: &mut DebugRunner, piercing_first: bool) {
    let p = r.game.pending_selection.as_ref().expect("Raid/Piercing order");
    assert_eq!(p.kind, SelectionKind::TriggerOrder);
    assert_eq!(p.selecting_player, 0);
    let choices = p.effect_choices.as_ref().unwrap();
    let keywords: Vec<_> = choices.iter().map(|e| e.keyword).collect();
    assert_eq!(
        keywords.len(),
        2,
        "only <Raid> and <Piercing> remain: {keywords:?}"
    );
    let entry = |kw: Keyword| {
        choices
            .iter()
            .find(|e| e.keyword == Some(kw))
            .unwrap_or_else(|| panic!("{kw:?} pending: {keywords:?}"))
            .action_id
    };
    let (piercing, raid) = (entry(Keyword::Piercing), entry(Keyword::Raid));
    cloned_act(r, if piercing_first { piercing } else { raid });
    assert_eq!(
        r.security_count(1),
        5,
        "<Piercing>'s check waits for the End of Attack timing"
    );
}
fn complete_attack_with_order(first: &str, block: bool) {
    let mut r = setup_completion(true);
    cloned_act(&mut r, encode_attack(0, SECURITY_TARGET));
    assert_effect_battle_first(&r);
    cloned_act(&mut r, 101);
    assert_eq!(r.security_count(1), 5);
    complete_trigger(&mut r, first);
    complete_trigger(
        &mut r,
        if first == "EX13-021" {
            "EX13-045"
        } else {
            "EX13-021"
        },
    );
    order_raid_and_piercing(&mut r, first == "EX13-045");
    prompt_is(&r, "Raid - switch attack target", 0);
    assert_eq!(r.security_count(1), 5);
    cloned_act(&mut r, encode_attack(0, 0));
    prompt_is(&r, "Counter (", 1);
    assert_eq!(r.game.current_phase, GamePhase::CounterTiming);
    // In both orders <Piercing> has activated by now (with <Raid> first, the
    // lone <Piercing> activated as the switch resolved); its one check is
    // still pending processing at the End of Attack (§16-7-4, §16-7-5).
    assert_eq!(
        r.security_count(1),
        5,
        "<Piercing>'s check waits for End of Attack"
    );
    assert!(
        !r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .is_empty(),
        "real EX9-013 Blast candidate must be legal"
    );
    cloned_act(&mut r, PASS);
    prompt_is(&r, "Declare a blocker", 1);
    assert_eq!(r.game.current_phase, GamePhase::BlockTiming);
    assert_eq!(
        r.security_count(1),
        5,
        "<Piercing>'s check waits for End of Attack (§16-7-5)"
    );
    assert_eq!(build_action_mask(&r.game, 1)[encode_attack(0, 0) as usize], 0.0, "the current defender cannot block");
    cloned_act(&mut r, if block { encode_attack(0, 2) } else { PASS });
    assert!(
        r.game.pending_selection.is_none(),
        "all effects and attack windows must resolve"
    );
    assert!(r.game.pending_attack.is_none(), "attack must reach cleanup");
    assert_eq!(
        r.security_count(1),
        3,
        "one Piercing sequence with Security A.+1, not two sequences for two battles"
    );
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "both legal Q7366 orders retain unsuspend"
    );
    let trash: Vec<_> = r.game.players[1]
        .trash
        .iter()
        .map(|c| r.game.card_data[c.data_index].card_id.as_str())
        .collect();
    assert!(
        trash.contains(&if block { "ST2-07" } else { "ST1-11" }),
        "the final defender dies: the distinct blocker if accepted, otherwise the Raid target"
    );
    assert_eq!(
        trash.iter().filter(|&&id| id == "ST1-03").count(),
        3,
        "direct-battle victim plus exactly two checked security cards"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "payout resolves exactly once"
    );
}
#[test]
fn payout_first_then_raid_counter_declined_block_declined_finishes_in_clones() {
    complete_attack_with_order("EX13-045", false);
}
#[test]
fn unsuspend_first_then_raid_counter_declined_block_accepted_finishes_in_clones() {
    complete_attack_with_order("EX13-021", true);
}

fn setup_attacker_departure(wing: bool) -> DebugRunner {
    let blue = "ST2-11";
    let mut b = DebugRunner::builder();
    for id in [
        "EX13-045", "EX13-021", "ST2-11", "ST4-13", "ST1-03", "ST1-11", "BT20-023", "EX9-013",
        "ST1-09", "ST5-11", "BT18-064", "BT19-026",
    ] {
        b = b.dsl_card(id).unwrap();
    }
    let mut r = b
        .hand(0, &["EX13-045", "BT20-023"])
        .hand(1, &["EX9-013"])
        .deck(0, &["ST1-03"; 20])
        .deck(1, &["ST1-03"; 20])
        .security(1, &["ST1-03"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.place_on_field(0, "ST4-13", Some(0));
    if wing {
        r.place_stack(0, &["EX13-021", blue]);
    } else {
        r.place_on_field(0, blue, Some(0));
    }
    r.place_stack(
        1,
        &[
            "BT18-064", "BT18-064", "BT18-064", "BT18-064", "BT19-026", "BT19-026", "BT19-026",
            "BT19-026", "ST1-11",
        ],
    );
    r.place_on_field(1, "ST1-03", Some(0));
    r.place_on_field(1, "ST1-09", Some(0));
    // Official observation 064fe77ff3f21af1997ea928feb09f4244c849274d76e03e4c3624ce001de64d.
    // DebugRunner DSL metadata omits printed text; hydrate actual card filter input.
    r.game.card_data.iter_mut().find(|d|d.card_id=="BT20-023").unwrap().effect_text = "＜Jamming＞\n[Your Turn] When any of your green Digimon with [Dracomon] or [Examon] in their texts are played, this Digimon may digivolve into [Wingdramon] in the hand with the digivolution cost reduced by 2.".to_string();
    // Actual native DNA route, then the real forced-attack target chooser.
    assert_eq!(build_action_mask(&r.game, 0)[63], 1.0);
    r.game.decode_action(63, 0);
    act(&mut r, 0);
    act(&mut r, 1);
    let v = r.pending_selection_view().expect("mandatory attack target");
    assert!(v.prompt.contains("attacks"));
    r
}

#[test]
fn declined_direct_battle_still_finishes_raid_counter_block_and_attack_in_clones() {
    let mut r = setup_completion(false);
    cloned_act(&mut r, encode_attack(0, SECURITY_TARGET));
    assert_effect_battle_first(&r);
    cloned_act(&mut r, PASS);
    assert_eq!(
        r.game.players[1].battle_area.len(),
        4,
        "decline leaves every potential effect-battle victim in play"
    );
    assert_eq!(r.security_count(1), 5);
    prompt_is(&r, "Raid - switch attack target", 0);
    cloned_act(&mut r, encode_attack(0, 0));
    prompt_is(&r, "Counter (", 1);
    cloned_act(&mut r, PASS);
    prompt_is(&r, "Declare a blocker", 1);
    cloned_act(&mut r, PASS);
    complete_trigger(&mut r, "EX13-045");
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.security_count(1), 3);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "attack-battle win still pays out exactly once"
    );
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "no Wingdramon was present"
    );
    let ids: Vec<_> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| r.game.card_data[p.top_card().data_index].card_id.as_str())
        .collect();
    assert!(
        ids.contains(&"ST1-03"),
        "declined direct-battle target survives"
    );
    assert!(!ids.contains(&"ST1-11"));
    let checked = r.game.players[1]
        .trash
        .iter()
        .filter(|c| r.game.card_data[c.data_index].card_id == "ST1-03")
        .count();
    assert_eq!(
        checked, 2,
        "only checked security cards enter trash; no effect-battle deletion"
    );
}
#[test]
fn direct_battle_loss_removes_attacker_and_cancels_attack_before_raid_in_clones() {
    let mut r = setup_attacker_departure(false);
    cloned_act(&mut r, encode_attack(0, SECURITY_TARGET));
    assert_effect_battle_first(&r);
    // Four printed +2000 all-turn inherited sources, plus four +2000 opponent-
    // turn inherited sources, on 12000 DP WarGreymon = 28000 > buffed 25000.
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(25000));
    assert_eq!(r.effective_dp(r.perm_handle(1, 0)), Some(28000));
    cloned_act(&mut r, 100);
    assert!(
        r.game.players[0].battle_area.is_empty(),
        "the declared attacker lost the direct effect battle"
    );
    // §11-1-5 (Ver. 4.3): "Once an attack declaration has been made, all of
    // the timings that follow will occur. (Example: Even if an attacking
    // Digimon leaves the battle area, the non-turn player's counter timing
    // and block timing will still occur.)" The departed attacker's pending
    // <Raid> can no longer activate (§15-4-4-3), so the next decision is the
    // opponent's Counter timing, not a Raid switch; P1 has no <Blocker>, and
    // the unsuccessful attack then takes no other action (§11-5-1-4-1).
    prompt_is(&r, "Counter (", 1);
    assert_eq!(r.game.current_phase, GamePhase::CounterTiming);
    cloned_act(&mut r, PASS);
    assert!(
        r.game.pending_selection.is_none(),
        "no Raid or Block decision after the attacker leaves"
    );
    assert!(
        r.game.pending_attack.is_none(),
        "the unsuccessful attack reaches cleanup"
    );
    assert_eq!(
        r.security_count(1),
        5,
        "there is no attack battle or security check after attacker departure"
    );
    assert_eq!(r.game.players[1].battle_area.len(), 3);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| r.game.card_data[c.data_index].card_id == "EX13-045"));
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| r.game.card_data[c.data_index].card_id == "BT20-023"),
        "no win means no payout"
    );
}
