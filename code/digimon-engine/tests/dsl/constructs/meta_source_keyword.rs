//! Top-source placement: card-text idiom, EX12-018 Siriusmon / EX12-077
//! Proximamon; CR 4.3 §§4-7-3/5, 4-8-1, 15-5-3 (new source observes itself).
//! Continuous keyword grants: BT26-101 Cross Arts; CR 4.3 §§15-11-2-2,
//! 15-11-2-3-2 (later entrants / changing eligibility), 16-5-1 and 12-1
//! (Blocker permits a real, optional block). Expiry is relative to the grant's
//! controller, even after its source leaves the field.

use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START, TRASH_EFFECT_START},
    },
    card_data::{CardData, EvoCost},
    card_source::{CardHandle, CardSource},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, GamePhase, Keyword},
    events::GameEvent,
    permanent::PermanentHandle,
    selection::{SelectionKind, UnionZoneSet},
};

const PLACE_TOP: &str = r#"
card: META-TOP
name: Top source placer
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          filter: {kind: digimon, trait_has: Material}
          bind_as: material
          optional: true
          prompt: Choose a material
      - select_own_permanent:
          filter: {kind: digimon, trait_has: Host}
          bind_as: host
          optional: true
          prompt: Choose a host
      - place_as_top_source: {source: material, target: host}
"#;

const PLACE_TOP_FACE_DOWN: &str = r#"
card: META-TOP-DOWN
name: Face down top source placer
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          filter: {kind: digimon, trait_has: Material}
          bind_as: material
          optional: true
          prompt: Choose a material
      - select_own_permanent:
          filter: {kind: digimon, trait_has: Host}
          bind_as: host
          optional: true
          prompt: Choose a host
      - place_as_top_source: {source: material, target: host, face_down: true}
"#;

const ADDED_SOURCE_WITNESS: &str = r#"
card: META-MATERIAL
name: Placed source witness
kind: digimon
color: [red]
level: 3
cost: 0
dp: 2000
traits: [Material]
effects:
  - scope: inherited
    when: on_add_digivolution_cards
    condition: {event_host_permanent_is_source: true}
    process:
      - gain_memory: 1
"#;

const CONTINUOUS_BLOCKER: &str = r#"
card: META-GRANT
name: Temporary continuous Blocker grant
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - grant_keyword_continuous:
          targets: {of: you, kind: digimon, trait_has: Member}
          keyword: Blocker
          expiry: end_of_opponents_turn
      - delete_permanent: {target: source}
"#;

const MIXED_SOURCE_WITNESS: &str = r#"
card: META-MIXED-WITNESS
name: Visible source identity witness
kind: digimon
color: [red]
level: 3
cost: 0
dp: 2000
effects:
  - scope: inherited
    when: on_add_digivolution_cards
    condition:
      any_of:
        - in_breeding: true
        - event_host_permanent_is_source: true
    process:
      - select_effect_choice:
          labels: [Gain 1 memory, Gain 2 memory]
          bind_as: amount
          prompt: Choose the visible source effect
      - if:
          condition: {equals: [amount, 0]}
          then:
            - gain_memory: 1
          else:
            - gain_memory: 2
"#;

fn card(id: &str, kind: CardKind, traits: &[&str], dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![CardColor::Red];
    c.play_cost = 0;
    c.level = (kind == CardKind::Digimon).then_some(3);
    c.dp = (kind == CardKind::Digimon).then_some(dp);
    c.traits = traits.iter().map(|s| (*s).into()).collect();
    c
}

fn push_zone(r: &mut DebugRunner, owner: u8, id: &str, trash: bool) -> CardHandle {
    let data = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .unwrap();
    let index = r.game.next_card_index();
    let c = CardSource::new(data, owner, index);
    let h = c.handle();
    if trash {
        r.game.players[owner as usize].trash.push(c);
    } else {
        r.game.players[owner as usize].hand.push(c);
    }
    h
}

fn stack(r: &DebugRunner, h: PermanentHandle) -> Vec<CardHandle> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect()
}

fn zone(r: &DebugRunner, owner: u8, trash: bool) -> Vec<CardHandle> {
    let p = &r.game.players[owner as usize];
    (if trash { &p.trash } else { &p.hand })
        .iter()
        .map(|c| c.handle())
        .collect()
}

fn assert_choice(r: &DebugRunner, owner: u8, kind: SelectionKind, ids: &[u16]) {
    let s = r.game.pending_selection.as_ref().expect("public selection");
    assert_eq!(s.selecting_player, owner);
    assert_eq!(s.kind, kind);
    assert!(s.is_optional);
    let mut ids = ids.iter().map(|&id| crate::union_fixture_action(&r.game, id)).collect::<Vec<_>>();
    if s.card_choice.is_some() { ids.push(PASS); }
    assert_eq!(s.valid_action_ids, ids);
    let mask = build_action_mask(&r.game, owner);
    for (a, bit) in mask.iter().enumerate() {
        assert_eq!(
            *bit,
            if ids.contains(&(a as u16)) || a == PASS as usize {
                1.0
            } else {
                0.0
            },
            "action {a}"
        );
    }
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|bit| *bit == 0.0));
}

fn choose(r: &mut DebugRunner, owner: u8, action: u16) {
    let action = crate::union_fixture_action(&r.game, action);
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.execute_action(owner, action).unwrap();
}

fn main_action(r: &mut DebugRunner, owner: u8, action: u16) {
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.turn_player(), owner);
    assert_eq!(
        build_action_mask(&r.game, owner)[action as usize],
        1.0,
        "action {action}, player {owner}, phase {:?}, memory {}",
        r.game.current_phase,
        r.memory()
    );
    r.game.decode_action(action, owner);
}

fn finish_turn(r: &mut DebugRunner) {
    let owner = r.game.turn_player();
    if r.game.current_phase == GamePhase::Breeding {
        main_action(r, owner, PASS);
    }
    assert_eq!(r.game.current_phase, GamePhase::Main);
    // A voluntary pass supplies the opponent 3 memory; forcing end_turn at
    // positive memory would incorrectly stage their next turn at negative memory.
    main_action(r, owner, PASS);
    assert_eq!(r.game.turn_player(), 1 - owner);
}

fn top_fixture(owner: u8, yaml: &str, hosts_match: bool) -> (DebugRunner, [PermanentHandle; 2]) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .from_dsl_yaml(ADDED_SOURCE_WITNESS)
        .unwrap()
        .add_card(card(
            "META-HOST",
            CardKind::Digimon,
            if hosts_match { &["Host"] } else { &[] },
            9000,
        ))
        .add_card(card("META-OLD-A", CardKind::Digimon, &[], 2000))
        .add_card(card("META-OLD-B", CardKind::Digimon, &[], 3000))
        .add_card(card("META-PLAIN", CardKind::Digimon, &[], 2000))
        .add_card(card(
            "META-MATERIAL-TAMER",
            CardKind::Tamer,
            &["Material"],
            0,
        ))
        .add_card(card("META-HOST-TAMER", CardKind::Tamer, &["Host"], 0))
        .deck(0, &["META-PLAIN"; 20])
        .deck(1, &["META-PLAIN"; 20])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.game.current_phase = GamePhase::Main;
    // Two copies in each origin distinguish physical identity from card number.
    for trash in [false, true] {
        for id in [
            "META-MATERIAL",
            "META-MATERIAL",
            "META-PLAIN",
            "META-MATERIAL-TAMER",
        ] {
            push_zone(&mut r, owner, id, trash);
        }
        push_zone(&mut r, 1 - owner, "META-MATERIAL", trash);
    }
    let first = r.place_stack(owner, &["META-OLD-A", "META-OLD-B", "META-HOST"]);
    let second = r.place_stack(owner, &["META-OLD-A", "META-OLD-B", "META-HOST"]);
    r.place_on_field(owner, "META-PLAIN", None);
    r.place_on_field(owner, "META-HOST-TAMER", None);
    r.place_on_field(1 - owner, "META-HOST", None);
    (r, [first, second])
}

fn start_top(r: &mut DebugRunner, owner: u8, id: &str) {
    let h = r.place_on_field(owner, id, None);
    r.fire_on_play(owner, h.index as usize);
    assert_choice(
        r,
        owner,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH,
        },
        &[
            PLAY_HAND_START,
            PLAY_HAND_START + 1,
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
        ],
    );
}

fn host_choice(r: &DebugRunner, owner: u8) {
    assert_choice(
        r,
        owner,
        SelectionKind::OwnField,
        &[encode_attack(0, 0), encode_attack(0, 1)],
    );
}

#[test]
fn top_source_from_hand_or_trash_preserves_exact_stack_and_physical_identity() {
    for owner in [0, 1] {
        for from_trash in [false, true] {
            for material_index in [0, 1] {
                let (mut r, hosts) = top_fixture(owner, PLACE_TOP, true);
                let before = [stack(&r, hosts[0]), stack(&r, hosts[1])];
                let origins = [zone(&r, owner, false), zone(&r, owner, true)];
                let chosen = origins[from_trash as usize][material_index];
                let wrong_owner = [zone(&r, 1 - owner, false), zone(&r, 1 - owner, true)];
                start_top(&mut r, owner, "META-TOP");
                let checkpoint = r.event_checkpoint();
                let action = if from_trash {
                    TRASH_EFFECT_START
                } else {
                    PLAY_HAND_START
                } + material_index as u16;
                choose(&mut r, owner, action);
                host_choice(&r, owner);
                // Merely selecting the material does not remove it from its origin.
                assert_eq!(zone(&r, owner, from_trash), origins[from_trash as usize]);
                choose(&mut r, owner, encode_attack(0, 1));
                assert!(r.game.pending_selection.is_none());
                let mut expected = before[1].clone();
                expected.insert(expected.len() - 1, chosen);
                assert_eq!(stack(&r, hosts[1]), expected);
                assert_eq!(stack(&r, hosts[0]), before[0], "unselected host unchanged");
                assert_eq!(r.top_card(hosts[1]), *before[1].last().unwrap());
                assert!(!r.game.players[owner as usize].battle_area[1].card_sources[2].face_down);
                let mut remaining = origins[from_trash as usize].clone();
                remaining.remove(material_index);
                assert_eq!(zone(&r, owner, from_trash), remaining);
                assert_eq!(
                    zone(&r, owner, !from_trash),
                    origins[(!from_trash) as usize]
                );
                assert_eq!(zone(&r, 1 - owner, false), wrong_owner[0]);
                assert_eq!(zone(&r, 1 - owner, true), wrong_owner[1]);
                assert_eq!(r.events_since(checkpoint).iter().filter(|e| matches!(e,
                    GameEvent::MemoryChange { source_card_id: Some(id), delta: 1, .. } if id == "META-MATERIAL"
                )).count(), 1, "the newly placed inherited source observes its own placement exactly once");
                assert!(
                    !r.events_since(checkpoint)
                        .iter()
                        .any(|e| matches!(e, GameEvent::Digivolve { .. })),
                    "source placement does not digivolve the host"
                );
            }
        }
    }
}

#[test]
fn top_source_declining_material_or_host_leaves_both_origins_and_stacks_untouched() {
    for decline_material in [true, false] {
        let (mut r, hosts) = top_fixture(0, PLACE_TOP, true);
        let origins = [zone(&r, 0, false), zone(&r, 0, true)];
        let before = [stack(&r, hosts[0]), stack(&r, hosts[1])];
        start_top(&mut r, 0, "META-TOP");
        let checkpoint = r.event_checkpoint();
        choose(
            &mut r,
            0,
            if decline_material {
                PASS
            } else {
                TRASH_EFFECT_START + 1
            },
        );
        host_choice(&r, 0);
        choose(
            &mut r,
            0,
            if decline_material {
                encode_attack(0, 1)
            } else {
                PASS
            },
        );
        assert!(r.game.pending_selection.is_none());
        assert_eq!(zone(&r, 0, false), origins[0]);
        assert_eq!(zone(&r, 0, true), origins[1]);
        assert_eq!(stack(&r, hosts[0]), before[0]);
        assert_eq!(stack(&r, hosts[1]), before[1]);
        assert!(!r.events_since(checkpoint).iter().any(|e| matches!(e,
            GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == "META-MATERIAL"
        )));
    }
}

#[test]
fn top_source_without_a_matching_host_does_not_consume_the_selected_material() {
    let (mut r, hosts) = top_fixture(0, PLACE_TOP, false);
    let before = [stack(&r, hosts[0]), stack(&r, hosts[1])];
    let origin = zone(&r, 0, false);
    start_top(&mut r, 0, "META-TOP");
    choose(&mut r, 0, PLAY_HAND_START + 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(zone(&r, 0, false), origin);
    assert_eq!(stack(&r, hosts[0]), before[0]);
    assert_eq!(stack(&r, hosts[1]), before[1]);
}

#[test]
fn explicit_face_down_top_source_keeps_position_but_cannot_fire_its_inherited_witness() {
    let (mut r, hosts) = top_fixture(0, PLACE_TOP_FACE_DOWN, true);
    let before = stack(&r, hosts[1]);
    let chosen = zone(&r, 0, true)[1];
    start_top(&mut r, 0, "META-TOP-DOWN");
    let checkpoint = r.event_checkpoint();
    choose(&mut r, 0, TRASH_EFFECT_START + 1);
    host_choice(&r, 0);
    choose(&mut r, 0, encode_attack(0, 1));
    let mut expected = before;
    expected.insert(expected.len() - 1, chosen);
    assert_eq!(stack(&r, hosts[1]), expected);
    assert!(r.game.players[0].battle_area[1].card_sources[2].face_down);
    assert!(
        !r.events_since(checkpoint).iter().any(|e| matches!(e,
            GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == "META-MATERIAL"
        )),
        "face-down sources have no card information (CR 4.3 4-7-9)"
    );
}

#[test]
fn mixed_source_visibility_preserves_the_higher_cards_identity_in_battle_and_breeding() {
    for breeding in [false, true] {
        for upper_face_down in [false, true] {
            let mut r = DebugRunner::builder()
                .from_dsl_yaml(PLACE_TOP_FACE_DOWN)
                .unwrap()
                .from_dsl_yaml(ADDED_SOURCE_WITNESS)
                .unwrap()
                .from_dsl_yaml(MIXED_SOURCE_WITNESS)
                .unwrap()
                .add_card(card("META-HOST", CardKind::Digimon, &["Host"], 9000))
                .add_card(card("META-PLAIN", CardKind::Digimon, &[], 2000))
                .hand(0, &["META-MATERIAL"; 2])
                .deck(0, &["META-PLAIN"; 20])
                .deck(1, &["META-PLAIN"; 20])
                .memory(5)
                .start();
            r.skip_mulligan();
            r.game.current_phase = GamePhase::Main;
            push_zone(&mut r, 0, "META-MATERIAL", true);
            push_zone(&mut r, 0, "META-MATERIAL", true);
            r.place_on_field(0, "META-HOST", None);
            let mixed_stack = ["META-MIXED-WITNESS", "META-MIXED-WITNESS", "META-HOST"];
            if breeding {
                r.place_on_field(0, "META-HOST", None);
                r.game.stage_place_in_breeding(0, &mixed_stack);
            } else {
                r.place_stack(0, &mixed_stack);
            }
            let perm = if breeding {
                r.game.players[0].breeding_area.as_mut().unwrap()
            } else {
                &mut r.game.players[0].battle_area[1]
            };
            perm.card_sources[0].face_down = true;
            perm.card_sources[1].face_down = upper_face_down;
            let lower = perm.card_sources[0].handle();
            let higher = perm.card_sources[1].handle();
            assert_ne!(
                lower, higher,
                "two copies have distinct physical identities"
            );
            start_top(&mut r, 0, "META-TOP-DOWN");
            let checkpoint = r.event_checkpoint();
            choose(&mut r, 0, TRASH_EFFECT_START + 1);
            host_choice(&r, 0);
            choose(&mut r, 0, encode_attack(0, 1));
            if !upper_face_down {
                let s = r
                    .game
                    .pending_selection
                    .as_ref()
                    .expect("face-up source activates");
                assert_eq!(s.kind, SelectionKind::EffectChoice);
                assert_eq!(s.selecting_player, 0);
                assert!(!s.is_optional);
                assert_eq!(
                    s.source_card, higher,
                    "skipping a lower hidden source must not shift identity"
                );
                assert_eq!(s.valid_action_ids.len(), 2);
                let actions = s.valid_action_ids.clone();
                let mask = build_action_mask(&r.game, 0);
                for (a, bit) in mask.iter().enumerate() {
                    assert_eq!(
                        *bit,
                        if actions.contains(&(a as u16)) {
                            1.0
                        } else {
                            0.0
                        }
                    );
                }
                assert!(build_action_mask(&r.game, 1).iter().all(|bit| *bit == 0.0));
                choose(&mut r, 0, actions[1]);
            }
            assert!(
                r.game.pending_selection.is_none(),
                "hidden sources cannot activate or require ordering"
            );
            let gained: Vec<_> = r
                .events_since(checkpoint)
                .iter()
                .filter_map(|e| match e {
                    GameEvent::MemoryChange {
                        source_card_id: Some(id),
                        delta,
                        ..
                    } if id == "META-MIXED-WITNESS" => Some(*delta),
                    _ => None,
                })
                .collect();
            assert_eq!(gained, if upper_face_down { vec![] } else { vec![2] });
            let perm = if breeding {
                r.game.players[0].breeding_area.as_ref().unwrap()
            } else {
                &r.game.players[0].battle_area[1]
            };
            assert_eq!(perm.card_sources[0].handle(), lower);
            assert_eq!(perm.card_sources[1].handle(), higher);
        }
    }
}

fn keyword_fixture(owner: u8) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut evolved = card("META-EVOLVED", CardKind::Digimon, &[], 9000);
    evolved.level = Some(4);
    evolved.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    }];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(CONTINUOUS_BLOCKER)
        .unwrap()
        .add_card(card("META-MEMBER", CardKind::Digimon, &["Member"], 9000))
        .add_card(card("META-NONMEMBER", CardKind::Digimon, &[], 9000))
        .add_card(card("META-MEMBER-TAMER", CardKind::Tamer, &["Member"], 0))
        .add_card(card("META-ATTACKER", CardKind::Digimon, &[], 4000))
        .add_card(card("META-FILL", CardKind::Digimon, &[], 1000))
        .add_card(evolved)
        .deck(0, &["META-FILL"; 20])
        .deck(1, &["META-FILL"; 20])
        .security(0, &["META-FILL"; 3])
        .security(1, &["META-FILL"; 3])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.game.current_phase = GamePhase::Main;
    let matching = r.place_on_field(owner, "META-MEMBER", Some(0));
    r.place_on_field(owner, "META-NONMEMBER", Some(0));
    r.place_on_field(owner, "META-MEMBER-TAMER", Some(0));
    r.place_on_field(1 - owner, "META-MEMBER", Some(0));
    let attacker = r.place_on_field(1 - owner, "META-ATTACKER", Some(0));
    (r, matching, attacker)
}

fn grant(r: &mut DebugRunner, owner: u8) {
    let index = r.game.players[owner as usize].hand.len();
    let source = push_zone(r, owner, "META-GRANT", false);
    main_action(r, owner, PLAY_HAND_START + index as u16);
    assert!(r.game.pending_selection.is_none());
    assert!(
        zone(r, owner, true).contains(&source),
        "temporary grant survives its source leaving"
    );
}

fn enter_matching(r: &mut DebugRunner, owner: u8) -> PermanentHandle {
    let index = r.game.players[owner as usize].hand.len();
    push_zone(r, owner, "META-MEMBER", false);
    let field_index = r.game.players[owner as usize].battle_area.len() as u8;
    main_action(r, owner, PLAY_HAND_START + index as u16);
    PermanentHandle {
        player: owner,
        index: field_index,
    }
}

fn assert_keyword_filter(r: &DebugRunner, owner: u8, matching: &[PermanentHandle]) {
    for h in matching {
        assert!(
            r.game.has_keyword(*h, Keyword::Blocker),
            "matching host {h:?}"
        );
    }
    for h in [
        PermanentHandle {
            player: owner,
            index: 1,
        },
        PermanentHandle {
            player: owner,
            index: 2,
        },
        PermanentHandle {
            player: 1 - owner,
            index: 0,
        },
    ] {
        assert!(
            !r.game.has_keyword(h, Keyword::Blocker),
            "wrong trait, kind or owner {h:?}"
        );
    }
}

#[test]
fn continuous_keyword_covers_existing_and_new_hosts_and_exposes_both_real_block_choices() {
    for owner in [0, 1] {
        for block in [false, true] {
            let (mut r, existing, attacker) = keyword_fixture(owner);
            assert!(!r.game.has_keyword(existing, Keyword::Blocker));
            grant(&mut r, owner);
            let late = enter_matching(&mut r, owner);
            assert_keyword_filter(&r, owner, &[existing, late]);
            finish_turn(&mut r);
            assert_eq!(r.game.turn_player(), 1 - owner);
            r.game.enter_main_phase();
            assert_keyword_filter(&r, owner, &[existing, late]);
            let security = r.game.players[owner as usize].security.len();
            let attacker_card = r.top_card(attacker);
            main_action(&mut r, 1 - owner, encode_attack(attacker.index as u16, 14));
            assert_eq!(r.game.current_phase, GamePhase::BlockTiming);
            assert_choice(
                &r,
                owner,
                SelectionKind::OwnField,
                &[
                    encode_attack(0, existing.index as u16),
                    encode_attack(0, late.index as u16),
                ],
            );
            choose(
                &mut r,
                owner,
                if block {
                    encode_attack(0, late.index as u16)
                } else {
                    PASS
                },
            );
            assert!(r.game.pending_selection.is_none());
            assert!(r.game.pending_attack.is_none());
            assert_eq!(
                r.game.players[owner as usize].battle_area[late.index as usize].is_suspended,
                block
            );
            assert_eq!(
                r.game.players[owner as usize].security.len(),
                security - usize::from(!block)
            );
            assert_eq!(
                zone(&r, 1 - owner, true).contains(&attacker_card),
                block,
                "accepting redirects battle to the 9000-DP blocker; declining checks security"
            );
        }
    }
}

#[test]
fn continuous_keyword_stops_affecting_a_host_that_digivolves_out_of_its_trait_filter() {
    for owner in [0, 1] {
        let (mut r, existing, attacker) = keyword_fixture(owner);
        grant(&mut r, owner);
        let late = enter_matching(&mut r, owner);
        let index = r.game.players[owner as usize].hand.len();
        push_zone(&mut r, owner, "META-EVOLVED", false);
        main_action(
            &mut r,
            owner,
            encode_digivolve(index as u16, existing.index as u16),
        );
        assert_eq!(
            r.game.players[owner as usize].battle_area[existing.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "META-EVOLVED"
        );
        assert!(!r.game.has_keyword(existing, Keyword::Blocker));
        assert!(r.game.has_keyword(late, Keyword::Blocker));
        finish_turn(&mut r);
        r.game.enter_main_phase();
        main_action(&mut r, 1 - owner, encode_attack(attacker.index as u16, 14));
        assert_choice(
            &r,
            owner,
            SelectionKind::OwnField,
            &[encode_attack(0, late.index as u16)],
        );
        choose(&mut r, owner, encode_attack(0, late.index as u16));
        assert!(r.game.pending_attack.is_none());
    }
}

#[test]
fn continuous_keyword_expires_at_its_controllers_opponent_turn_end_on_either_side() {
    for owner in [0, 1] {
        for during_opponent_turn in [false, true] {
            let (mut r, existing, attacker) = keyword_fixture(owner);
            if during_opponent_turn {
                r.set_first_player(1 - owner);
                let source = r.place_on_field(owner, "META-GRANT", None);
                r.fire_on_play(owner, source.index as usize);
            } else {
                grant(&mut r, owner);
                finish_turn(&mut r);
                assert_eq!(r.game.turn_player(), 1 - owner);
            }
            assert!(
                r.game.has_keyword(existing, Keyword::Blocker),
                "live through the opponent's turn"
            );
            finish_turn(&mut r);
            assert_eq!(r.game.turn_player(), owner);
            assert!(!r.game.has_keyword(existing, Keyword::Blocker));
            r.game.enter_main_phase();
            let late = enter_matching(&mut r, owner);
            assert!(
                !r.game.has_keyword(late, Keyword::Blocker),
                "expired descriptor cannot affect a later entrant"
            );
            finish_turn(&mut r);
            r.game.enter_main_phase();
            let security = r.game.players[owner as usize].security.len();
            main_action(&mut r, 1 - owner, encode_attack(attacker.index as u16, 14));
            assert!(
                r.game.pending_selection.is_none(),
                "expired Blocker cannot install a block prompt"
            );
            assert!(r.game.pending_attack.is_none());
            assert_eq!(r.game.players[owner as usize].security.len(), security - 1);
        }
    }
}
