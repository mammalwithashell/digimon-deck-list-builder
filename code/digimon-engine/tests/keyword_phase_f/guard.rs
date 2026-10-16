//! EX12 keyword behavior: `<Guard>`.
//!
//! Official rules 16-45 define Guard as an optional immediate replacement:
//! when another of your Digimon would leave the battle area by an opponent's
//! effect, delete the Guard carrier to prevent that other Digimon leaving.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

use super::helpers::{digimon_with_keywords, plain_digimon, plain_tamer};

fn guard_card(id: &str) -> CardData {
    digimon_with_keywords(id, 4, 5000, vec![Keyword::Guard])
}

fn field_ids(r: &DebugRunner, player: u8) -> Vec<String> {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn guard_accept_deletes_carrier_and_prevents_other_digimon_leaving() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    assert_ne!(guard, ally);

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Guard should offer an optional replacement for another own Digimon"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Guard replacement");

    assert_eq!(
        field_ids(&r, 0),
        vec!["ALLY".to_string()],
        "the protected Digimon should remain and the Guard carrier should be deleted"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "GUARD"),
        "Guard carrier should be in trash after paying the replacement cost"
    );
}

#[test]
fn guard_accept_saves_all_simultaneous_matching_digimon_and_keeps_indices_stable() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY-A"))
        .add_card(plain_digimon("ALLY-B"))
        .add_card(plain_digimon("BYSTANDER"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally_a = r.place_on_field(0, "ALLY-A", Some(0));
    let ally_b = r.place_on_field(0, "ALLY-B", Some(0));
    let _bystander = r.place_on_field(0, "BYSTANDER", Some(0));
    assert_ne!(guard, ally_a);
    assert_ne!(guard, ally_b);

    let outcome = r
        .game
        .delete_permanents_batch(vec![ally_a, ally_b], ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Guard should offer one optional replacement for the simultaneous leave batch"
    );
    assert!(
        outcome.completed.is_empty(),
        "batch should park before trashing any permanent"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Guard replacement");

    assert!(
        r.game.pending_selection.is_none(),
        "one Guard acceptance should settle the simultaneous matching leave batch"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec![
            "ALLY-A".to_string(),
            "ALLY-B".to_string(),
            "BYSTANDER".to_string()
        ],
        "Guard should delete only the carrier, save both simultaneous allies, and not corrupt shifted indices"
    );
    let trash_ids: Vec<_> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        trash_ids.contains(&"GUARD".to_string()),
        "Guard carrier should be the only trashed card from the batch"
    );
    assert!(
        !trash_ids.contains(&"ALLY-A".to_string())
            && !trash_ids.contains(&"ALLY-B".to_string())
            && !trash_ids.contains(&"BYSTANDER".to_string()),
        "protected allies and unrelated bystander must not be trashed"
    );
}

#[test]
fn guard_protected_carrier_allows_attempt_but_failed_payment_saves_no_ally() {
    // Official 15-7-4 [4.3 REVERSED — see changelog-3.6-to-4.3.md]: impossible payment does not suppress optional attempt (3.6 reading; 4.3 §15-7-4 says an impossible cost can't be chosen).
    for action in [REPLACEMENT_ACCEPT, PASS] {
        let mut r=DebugRunner::builder().add_card(guard_card("GUARD"))
            .add_card(plain_digimon("ALLY")).start();
        let guard=r.place_on_field(0,"GUARD",Some(0));
        let ally=r.place_on_field(0,"ALLY",Some(0));
        r.game.modifiers.add(guard,ModifierEntry::simple(ModifierType::CannotBeDestroyed,0,Expiry::Permanent,0));
        r.game.delete_permanent_with_cause(ally,ReplacementCause::OpponentEffect);
        let p=r.game.pending_selection.as_ref().expect("optional Guard attempt remains available");
        assert_eq!(p.selecting_player,0);assert!(p.is_optional);
        let mask=digimon_engine::action::build_action_mask(&r.game,0);
        assert_eq!(mask[REPLACEMENT_ACCEPT as usize],1.0);assert_eq!(mask[PASS as usize],1.0);
        r.game.resolve_selection(0,action).unwrap();
        assert!(r.game.pending_selection.is_none());
        assert_eq!(field_ids(&r,0),vec!["GUARD".to_string()]);
        assert_eq!(trash_ids(&r,0),vec!["ALLY".to_string()]);
    }
}

#[test]
fn guard_decline_allows_original_leave_event() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, PASS)
        .expect("decline Guard replacement");

    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string()],
        "declining Guard should leave the carrier and let the ally leave"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ALLY"),
        "the original Digimon should be trashed when Guard is declined"
    );
}

#[test]
fn guard_replacement_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .start();

    r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Guard should park an optional replacement prompt"
    );
    assert!(
        r.game.pending_selection_resume.is_some(),
        "Guard replacement prompt must be resume-driven so cloned games can resolve it"
    );

    let mut cloned = r.game.clone();
    cloned
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("clone accepts Guard replacement");

    assert_eq!(
        cloned.players[0]
            .battle_area
            .iter()
            .map(|p| p.top_card().card_id(&cloned.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["ALLY".to_string()],
        "clone resolves to the protected state"
    );
    assert!(
        cloned.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&cloned.card_data) == "GUARD"),
        "clone deletes the Guard carrier"
    );

    assert!(
        r.game.pending_selection.is_some(),
        "original prompt survives resolving the clone"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string(), "ALLY".to_string()],
        "original battle area is untouched while the clone resolves"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("original accepts Guard replacement");
    assert_eq!(
        field_ids(&r, 0),
        vec!["ALLY".to_string()],
        "original reaches the clone's protected state"
    );
}

#[test]
fn aura_granted_guard_behaves_like_printed_guard() {
    let yaml = r#"
card: TEST-GUARD-AURA-SRC
name: Guard Aura Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, kind: digimon, trait: ME }
    grant_keyword: { keyword: Guard }
"#;

    let mut carrier = plain_digimon("AURA-GUARD-CARRIER");
    carrier.traits.push("ME".to_string());

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register Guard aura DSL source")
        .add_card(carrier)
        .add_card(plain_digimon("ALLY"))
        .start();

    r.place_on_field(0, "TEST-GUARD-AURA-SRC", Some(0));
    let guard = r.place_on_field(0, "AURA-GUARD-CARRIER", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(guard, Keyword::Guard),
        "aura should grant Guard to the ME carrier"
    );

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "aura-granted Guard should offer the same replacement as printed Guard"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept aura-granted Guard replacement");

    assert_eq!(
        field_ids(&r, 0),
        vec!["TEST-GUARD-AURA-SRC".to_string(), "ALLY".to_string()],
        "aura-granted Guard should delete the carrier and keep the ally"
    );
}

#[test]
fn guard_ignores_own_effects_self_and_non_digimon_subjects() {
    let mut r = DebugRunner::builder()
        .add_card(guard_card("GUARD"))
        .add_card(plain_digimon("ALLY"))
        .add_card(plain_tamer("TAMER"))
        .start();

    let guard = r.place_on_field(0, "GUARD", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let _tamer = r.place_on_field(0, "TAMER", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OwnEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not prompt for own-effect leave events"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string(), "TAMER".to_string()],
        "own-effect deletion should proceed"
    );

    let shifted_tamer = PermanentHandle {
        player: 0,
        index: 1,
    };
    r.game.set_effect_source_player_for_test(Some(1));
    let returned = r.game.return_to_hand(shifted_tamer);
    r.game.set_effect_source_player_for_test(None);
    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not prompt for non-Digimon subjects"
    );
    assert!(
        returned.is_some(),
        "the Tamer should return to hand normally"
    );
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string()],
        "opponent-effect Tamer leave event should proceed"
    );

    r.game
        .delete_permanent_with_cause(guard, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "Guard should not protect itself because the rule says another Digimon"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "self deletion should proceed without a Guard replacement"
    );
}

// ─── Nested replacement windows (G-NESTED-PARKED-REPLACEMENT) ───────────────
//
// <Guard>'s cost ("by deleting this Digimon") is itself a deletion with its own
// replacement windows on the carrier (<Evade>, <Armor Purge>, <Fragment>,
// <Decode>, ...). When one of those prompts, the inner window parks while
// Guard's replacement is still unresolved. The parked replacements form a
// stack drained LIFO: the carrier's deletion commits first, then Guard's
// outcome is committed — "they don't leave" only if the carrier was actually
// deleted (DCGO `GuardProcess` → `DeletePeremanentAndProcessAccordingToResult`
// success branch; rules 16-45).

fn trash_ids(r: &DebugRunner, player: u8) -> Vec<String> {
    r.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Guard carrier (with one card under it) that ALSO has `extra` keywords
/// acting on its own deletion.
fn nested_guard_runner(extra: Vec<Keyword>) -> (DebugRunner, PermanentHandle) {
    let mut kws = vec![Keyword::Guard];
    kws.extend(extra);
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_keywords("GUARD", 4, 5000, kws))
        .add_card(plain_digimon("UNDER"))
        .add_card(plain_digimon("ALLY"))
        .start();
    r.place_stack(0, &["UNDER", "GUARD"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.tick_declarative_effects();
    (r, ally)
}

/// Accept Guard; the carrier's own window must then open (nested), with
/// Guard's replacement still parked.
fn accept_guard_into_nested_window(r: &mut DebugRunner, ally: PermanentHandle) {
    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Guard");
    assert!(
        r.game.pending_selection.is_some(),
        "the Guard carrier's own replacement window opens inside Guard's process"
    );
    assert_eq!(
        field_ids(r, 0),
        vec!["GUARD".to_string(), "ALLY".to_string()],
        "nothing has left yet: both the cost deletion and the ally's leave are pending"
    );
}

fn resolve_all_accepting(r: &mut DebugRunner) {
    let mut steps = 0;
    while let Some(v) = r.pending_selection_view() {
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(v.selecting_player, a).unwrap();
        steps += 1;
        assert!(steps < 8, "nested drain terminates");
    }
}

#[test]
fn guard_cost_deletion_with_evade_declined_deletes_carrier_and_saves_ally() {
    let (mut r, ally) = nested_guard_runner(vec![Keyword::Evade]);
    accept_guard_into_nested_window(&mut r, ally);
    r.game.resolve_selection(0, PASS).expect("decline <Evade>");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 0), vec!["ALLY".to_string()], "ally doesn't leave");
    assert!(trash_ids(&r, 0).contains(&"GUARD".to_string()), "carrier deleted (cost paid)");
}

#[test]
fn guard_cost_deletion_prevented_by_evade_does_not_save_ally() {
    let (mut r, ally) = nested_guard_runner(vec![Keyword::Evade]);
    accept_guard_into_nested_window(&mut r, ally);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept <Evade>");
    resolve_all_accepting(&mut r);
    assert_eq!(
        field_ids(&r, 0),
        vec!["GUARD".to_string()],
        "Evade prevented the cost deletion, so Guard's 'they don't leave' does not apply"
    );
    assert!(r.game.players[0].battle_area[0].is_suspended, "Evade suspended the carrier");
    assert!(trash_ids(&r, 0).contains(&"ALLY".to_string()), "the ally leaves");
}

#[test]
fn guard_cost_deletion_prevented_by_fragment_pick_does_not_save_ally() {
    // <Fragment (1)> parks a SECOND selection (the source pick) on top of
    // Guard's parked replacement — a two-deep parked stack.
    let (mut r, ally) = nested_guard_runner(vec![Keyword::Fragment(1)]);
    accept_guard_into_nested_window(&mut r, ally);
    resolve_all_accepting(&mut r);
    assert_eq!(field_ids(&r, 0), vec!["GUARD".to_string()], "Fragment saved the carrier");
    assert!(trash_ids(&r, 0).contains(&"UNDER".to_string()), "Fragment trashed 1 source");
    assert!(trash_ids(&r, 0).contains(&"ALLY".to_string()), "cost unpaid → ally leaves");
}

#[test]
fn nested_guard_evade_park_clones_faithfully() {
    let (mut r, ally) = nested_guard_runner(vec![Keyword::Evade]);
    accept_guard_into_nested_window(&mut r, ally);
    assert!(
        r.game.pending_selection_resume.is_some(),
        "the nested <Evade> prompt is resume-driven"
    );
    let mut cloned = r.game.clone();
    cloned.resolve_selection(0, PASS).expect("clone declines <Evade>");
    assert_eq!(
        cloned.players[0]
            .battle_area
            .iter()
            .map(|p| p.top_card().card_id(&cloned.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["ALLY".to_string()],
        "clone drains both parked replacements"
    );
    // Original is untouched and resolves identically.
    assert_eq!(field_ids(&r, 0), vec!["GUARD".to_string(), "ALLY".to_string()]);
    r.game.resolve_selection(0, PASS).expect("original declines <Evade>");
    assert_eq!(field_ids(&r, 0), vec!["ALLY".to_string()]);
}

// Native plural Guard: preserve payment success and physical identity across
// source removal, with exact masks at each replacement decision.
#[test]
fn guard_plural_native_payment_preserves_both_allies_across_carrier_positions() {
    for position in 0..3 {
        let mut r = DebugRunner::builder()
            .add_card(guard_card("GUARD"))
            .add_card(plain_digimon("ALLY-A"))
            .add_card(plain_digimon("ALLY-B"))
            .add_card(plain_tamer("TAMER"))
            .add_card(plain_digimon("OPPONENT"))
            .start();
        let mut order = vec!["ALLY-A", "ALLY-B"];
        order.insert(position, "GUARD");
        let mut allies = Vec::new();
        let mut identities = Vec::new();
        for id in order {
            let h = r.place_on_field(0, id, Some(0));
            if id != "GUARD" {
                allies.push(h);
                identities.push(r.game.players[0].battle_area[h.index as usize].top_card().handle());
            }
        }
        let tamer = r.place_on_field(0, "TAMER", Some(0));
        let opponent = r.place_on_field(1, "OPPONENT", Some(0));
        let mut batch = allies.clone(); batch.extend([tamer, opponent]);
        r.game.delete_permanents_batch(batch, ReplacementCause::OpponentEffect);
        let mut copy = r.game.clone();
        for game in [&mut r.game, &mut copy] {
            let p = game.pending_selection.as_ref().expect("Guard optional window");
            assert_eq!(p.selecting_player, 0);
            assert!(p.is_optional);
            let mask = digimon_engine::action::build_action_mask(game, 0);
            assert!(mask[REPLACEMENT_ACCEPT as usize] > 0.0 && mask[PASS as usize] > 0.0);
            game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
            assert!(game.pending_selection.is_none(), "one paid Guard protects both, without a subject choice");
            assert_eq!(game.players[0].battle_area.iter().map(|p|p.top_card().handle()).collect::<Vec<_>>(), identities);
            assert!(game.players[1].battle_area.is_empty(), "opponent is not protected");
            let mut trash: Vec<_> = game.players[0].trash.iter().map(|c|c.card_id(&game.card_data)).collect(); trash.sort();
            assert_eq!(trash, vec!["GUARD", "TAMER"], "non-Digimon is not protected");
            assert_eq!(game.players[1].trash.iter().map(|c|c.card_id(&game.card_data)).collect::<Vec<_>>(), vec!["OPPONENT"]);
        }
    }
}

#[test]
fn guard_plural_nested_evade_payment_outcome_controls_both_allies() {
    for prevent_cost in [false, true] {
        let mut r = DebugRunner::builder()
            .add_card(digimon_with_keywords("GUARD", 4, 5000, vec![Keyword::Guard, Keyword::Evade]))
            .add_card(plain_digimon("ALLY-A"))
            .add_card(plain_digimon("ALLY-B"))
            .start();
        r.place_on_field(0, "GUARD", Some(0));
        let a = r.place_on_field(0, "ALLY-A", Some(0));
        let b = r.place_on_field(0, "ALLY-B", Some(0));
        r.game.delete_permanents_batch(vec![a,b], ReplacementCause::OpponentEffect);
        assert!(digimon_engine::action::build_action_mask(&r.game,0)[REPLACEMENT_ACCEPT as usize]>0.0);
        r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
        let p = r.game.pending_selection.as_ref().expect("nested Evade payment window");
        assert!(p.prompt.contains("Evade"), "must pay Guard now, not select a protected subject");
        assert_eq!(p.selecting_player,0); assert!(p.is_optional);
        let mut copy = r.game.clone();
        for game in [&mut r.game, &mut copy] {
            let action = if prevent_cost { REPLACEMENT_ACCEPT } else { PASS };
            assert!(digimon_engine::action::build_action_mask(game,0)[action as usize]>0.0);
            game.resolve_selection(0,action).unwrap();
            assert!(game.pending_selection.is_none());
            let ids:Vec<_>=game.players[0].battle_area.iter().map(|p|p.top_card().card_id(&game.card_data)).collect();
            let mut trash:Vec<_>=game.players[0].trash.iter().map(|c|c.card_id(&game.card_data)).collect();trash.sort();
            if prevent_cost {
                assert_eq!(ids,vec!["GUARD"]);assert_eq!(trash,vec!["ALLY-A","ALLY-B"]);
                assert!(game.players[0].battle_area[0].is_suspended);
            } else { assert_eq!(ids,vec!["ALLY-A","ALLY-B"]);assert_eq!(trash,vec!["GUARD"]); }
        }
    }
}


#[test]
fn guard_plural_return_supports_printed_and_aura_granted_carriers() {
    const RETURN: &str = r#"
card: GUARD-RETURN
name: Guard return producer
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - return_all_to_hand:
          over: {of: opponent, zone: [battle_area], kind: digimon, level_eq: 3}
"#;
    const AURA: &str = r#"
card: GUARD-AURA
name: Guard aura producer
kind: tamer
color: [black]
cost: 0
effects:
  - kind: aura
    target: {owner: you, kind: digimon, trait: ME}
    grant_keyword: {keyword: Guard}
"#;
    for granted in [false,true] {
        let mut carrier=if granted {plain_digimon("GUARD")} else {guard_card("GUARD")};
        carrier.level=Some(4);carrier.traits.push("ME".into());
        let mut a=plain_digimon("ALLY-A");a.level=Some(3);
        let mut b=plain_digimon("ALLY-B");b.level=Some(3);
        let mut r=DebugRunner::builder().from_dsl_yaml(RETURN).unwrap().from_dsl_yaml(AURA).unwrap()
            .add_card(carrier).add_card(a).add_card(b).add_card(plain_digimon("FILL"))
            .hand(0,&["GUARD-RETURN"]).deck(0,&["FILL";20]).deck(1,&["FILL";20]).memory(10).start();
        r.skip_mulligan();r.game.current_phase=digimon_engine::enums::GamePhase::Main;
        if granted {r.place_on_field(1,"GUARD-AURA",None);}
        let g=r.place_on_field(1,"GUARD",None);
        let a=r.place_on_field(1,"ALLY-A",None);let b=r.place_on_field(1,"ALLY-B",None);
        let ids=[a,b].map(|h|r.game.players[1].battle_area[h.index as usize].top_card().handle());
        r.game.tick_declarative_effects();assert!(r.game.has_keyword(g,Keyword::Guard));
        let play=digimon_engine::action::space::PLAY_HAND_START;
        assert_eq!(digimon_engine::action::build_action_mask(&r.game,0)[play as usize],1.0);
        r.game.decode_action(play,0);
        let p=r.game.pending_selection.as_ref().expect("Guard before simultaneous return");
        assert_eq!(p.selecting_player,1);assert!(p.is_optional);
        let mut cloned=r.game.clone();
        for game in [&mut r.game,&mut cloned] {
            assert_eq!(digimon_engine::action::build_action_mask(game,1)[REPLACEMENT_ACCEPT as usize],1.0);
            game.resolve_selection(1,REPLACEMENT_ACCEPT).unwrap();
            assert!(game.pending_selection.is_none(),"one Guard payment protects the simultaneous return group");
            let live:Vec<_>=game.players[1].battle_area.iter().map(|p|p.top_card().handle()).collect();
            assert!(ids.iter().all(|id|live.contains(id)));
            assert!(game.players[1].hand.iter().all(|c|!ids.contains(&c.handle())));
            assert_eq!(game.players[1].trash.iter().map(|c|c.card_id(&game.card_data)).collect::<Vec<_>>(),vec!["GUARD"]);
        }
    }
}
