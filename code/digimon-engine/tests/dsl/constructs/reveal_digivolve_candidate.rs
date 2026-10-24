//! CR 4.3 §8-1 (normal digivolution requirements and bonus draw).
//! Card-text idiom: BT9-104 searches revealed results, unlike
//! BT25-092's hand/trash search. This is a real On Play selection followed by
//! free evolution from the reveal pool, not merely a compiled-IR assertion.
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::{CardColor, CardKind, Expiry, ModifierType};

const SEARCH: &str = r#"
card: REVEAL-CANDIDATE
name: Reveal Candidate Probe
kind: tamer
color: [black]
cost: 0
effects:
  - when: on_play
    process:
      - reveal_top_deck: {of: you, count: 3, bind_as: revealed}
      - select_own_permanent:
          bind_as: host
          optional: true
          continue_on_decline: true
          filter:
            all_of:
              - kind: digimon
              - has_digivolve_candidate:
                  of: you
                  zone: [reveal]
                  filter:
                    all_of:
                      - kind: digimon
                      - trait_has: X Antibody
          prompt: Choose a host with an eligible revealed result
      - select_reveal:
          of: you
          bind_as: evolution
          optional: true
          filter: {kind: digimon, trait_has: X Antibody, can_digivolve_onto: host}
          prompt: Choose the revealed evolution
      - effect_initiated_digivolve: {target: host, source: evolution, cost: free}
      - per_selected:
          selection: revealed
          bind_as: rest
          body:
            - trash_from_reveal: {of: you, card: rest}
"#;

fn card(id: &str, kind: CardKind, x: bool, level: u8, color: CardColor, route: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    c.level = (kind == CardKind::Digimon).then_some(level);
    c.dp = (kind == CardKind::Digimon).then_some(5000);
    c.traits = if x { vec!["X Antibody".into()] } else { vec![] };
    if route {
        c.evo_costs.push(EvoCost {
            card_color: CardColor::Black as u8,
            level: 3,
            memory_cost: 19,
        });
    }
    c
}
fn runner(owner: u8, deck_top_first: &[&str]) -> DebugRunner {
    let deck: Vec<_> = deck_top_first.iter().rev().copied().collect();
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SEARCH)
        .expect("reveal candidate syntax must compile")
        .add_card(card(
            "HOST",
            CardKind::Digimon,
            false,
            3,
            CardColor::Black,
            false,
        ))
        .add_card(card(
            "HOST2",
            CardKind::Digimon,
            false,
            3,
            CardColor::Black,
            false,
        ))
        .add_card(card(
            "WRONG-LEVEL",
            CardKind::Digimon,
            false,
            5,
            CardColor::Black,
            false,
        ))
        .add_card(card(
            "WRONG-COLOR",
            CardKind::Digimon,
            false,
            3,
            CardColor::Red,
            false,
        ))
        .add_card(card(
            "RESULT",
            CardKind::Digimon,
            true,
            4,
            CardColor::Black,
            true,
        ))
        .add_card(card(
            "RESULT2",
            CardKind::Digimon,
            true,
            4,
            CardColor::Black,
            true,
        ))
        .add_card(card(
            "NO-TRAIT",
            CardKind::Digimon,
            false,
            4,
            CardColor::Black,
            true,
        ))
        .add_card(card(
            "NO-ROUTE",
            CardKind::Digimon,
            true,
            4,
            CardColor::Black,
            false,
        ))
        .add_card(card(
            "X-TAMER",
            CardKind::Tamer,
            true,
            0,
            CardColor::Black,
            true,
        ))
        .add_card(card(
            "X-OPTION",
            CardKind::Option,
            true,
            0,
            CardColor::Black,
            true,
        ))
        .add_card(card(
            "FILL",
            CardKind::Digimon,
            false,
            3,
            CardColor::Red,
            false,
        ))
        .hand(owner, &["REVEAL-CANDIDATE"])
        .deck(owner, &deck)
        .deck(1 - owner, &["FILL"; 15])
        .memory(8)
        .start();
    r.set_first_player(owner);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn choose(r: &mut DebugRunner, action: u16) {
    let v = r.pending_selection_view().unwrap();
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[action as usize],
        1.0
    );
    assert!(build_action_mask(&r.game, 1 - v.selecting_player)
        .iter()
        .all(|n| *n == 0.0));
    r.execute_action(v.selecting_player, action).unwrap();
}
fn use_probe(r: &mut DebugRunner, owner: u8) {
    assert_eq!(build_action_mask(&r.game, owner)[0], 1.0);
    r.game.decode_action(0, owner);
}
fn actions(r: &DebugRunner) -> Vec<u16> {
    let v = r
        .pending_selection_view()
        .expect("real eligible host/result decision");
    assert!(v.is_optional);
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[space::PASS as usize],
        1.0
    );
    v.valid_action_ids
        .into_iter()
        .filter(|a| *a != space::PASS)
        .collect()
}

#[test]
fn reveal_candidates_surface_two_hosts_and_two_results_for_either_controller() {
    for owner in [0, 1] {
        let mut r = runner(owner, &["RESULT", "RESULT2", "FILL", "FILL", "FILL"]);
        let h0 = r.place_on_field(owner, "HOST", Some(0));
        let h1 = r.place_on_field(owner, "HOST2", Some(0));
        use_probe(&mut r, owner);
        assert_eq!(
            actions(&r),
            vec![
                space::encode_attack(0, h0.index as u16),
                space::encode_attack(0, h1.index as u16)
            ]
        );
        choose(&mut r, space::encode_attack(0, h1.index as u16));
        assert_eq!(
            actions(&r),
            vec![space::SEL_REVEAL_START, space::SEL_REVEAL_START + 1]
        );
        let chosen = r.game.revealed_cards[1].handle();
        let memory = r.memory();
        r.game = r.game.clone();
        choose(&mut r, space::SEL_REVEAL_START + 1);
        assert_eq!(
            r.game.players[owner as usize].battle_area[h1.index as usize]
                .top_card()
                .handle(),
            chosen
        );
        assert_eq!(
            r.memory(),
            memory,
            "route-only host gate permits free cost-19 evolution"
        );
        assert_eq!(r.game.players[owner as usize].hand.len(), 1, "bonus draw");
        assert!(r.game.revealed_cards.is_empty());
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn reveal_candidate_composite_filter_rejects_wrong_trait_kind_and_route() {
    for bad in ["NO-TRAIT", "NO-ROUTE", "X-TAMER", "X-OPTION"] {
        let mut r = runner(0, &[bad, "FILL", "FILL", "FILL"]);
        r.place_on_field(0, "HOST", Some(0));
        use_probe(&mut r, 0);
        assert!(
            r.pending_selection().is_none(),
            "{bad} cannot enable a host"
        );
        assert!(r.game.revealed_cards.is_empty());
    }
}

#[test]
fn reveal_candidate_never_borrows_a_result_owned_by_the_other_player() {
    for owner in [0, 1] {
        let mut r = runner(owner, &["FILL"; 5]);
        r.place_on_field(owner, "HOST", Some(0));
        let index = r.add_to_hand(1 - owner, "RESULT");
        let foreign = r.game.players[(1 - owner) as usize].hand.remove(index);
        let handle = foreign.handle();
        r.game.revealed_cards.push(foreign);
        use_probe(&mut r, owner);
        assert!(
            r.pending_selection().is_none(),
            "opponent's shared reveal pool is not own zone"
        );
        assert_eq!(r.game.revealed_cards.len(), 1);
        assert_eq!(r.game.revealed_cards[0].handle(), handle);
    }
}

#[test]
fn reveal_candidate_does_not_search_hand_trash_deck_security_or_field_sources() {
    for zone in 0..5 {
        let mut r = runner(0, &["FILL", "FILL", "FILL", "RESULT", "FILL"]);
        r.place_on_field(0, "HOST", Some(0));
        if zone != 2 {
            let index = r.add_to_hand(0, "RESULT");
            if zone != 0 {
                let result = r.game.players[0].hand.remove(index);
                match zone {
                    1 => r.game.players[0].trash.push(result),
                    3 => r.game.players[0].security.push(result),
                    4 => r.game.players[0].battle_area[0]
                        .card_sources
                        .insert(0, result),
                    _ => unreachable!(),
                }
            }
        }
        use_probe(&mut r, 0);
        assert!(r.pending_selection().is_none(), "zone {zone}");
        assert!(r.game.revealed_cards.is_empty());
    }
}

#[test]
fn reveal_candidate_routes_exclude_wrong_level_color_and_breeding_hosts() {
    let mut r = runner(0, &["RESULT", "FILL", "FILL", "FILL"]);
    let good = r.place_on_field(0, "HOST", Some(0));
    for id in ["WRONG-LEVEL", "WRONG-COLOR", "X-TAMER"] {
        r.place_on_field(0, id, Some(0));
    }
    r.place_on_field(1, "HOST", Some(0));
    r.place_in_breeding(0, "HOST");
    use_probe(&mut r, 0);
    assert_eq!(
        actions(&r),
        vec![space::encode_attack(0, good.index as u16)]
    );
}

#[test]
fn reveal_candidate_respects_host_and_effect_evolution_prohibitions() {
    for player_prohibition in [false, true] {
        let mut r = runner(0, &["RESULT", "FILL", "FILL", "FILL"]);
        let host = r.place_on_field(0, "HOST", Some(0));
        if player_prohibition {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotDigivolveDigimonByEffect,
                    0,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        } else {
            r.game.modifiers.add(
                host,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 1),
            );
        }
        use_probe(&mut r, 0);
        assert!(r.pending_selection().is_none());
        assert_eq!(
            r.game.players[0].battle_area[host.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "HOST"
        );
    }
}

#[test]
fn reveal_candidate_optional_host_or_result_decline_runs_reveal_cleanup() {
    for decline_result in [false, true] {
        for path in ["resume", "cloned resume", "legacy callback"] {
            let mut r = runner(0, &["RESULT", "RESULT2", "FILL", "FILL"]);
            let h = r.place_on_field(0, "HOST", Some(0));
            use_probe(&mut r, 0);
            if decline_result {
                choose(&mut r, space::encode_attack(0, h.index as u16));
            }
            match path {
                "cloned resume" => r.game = r.game.clone(),
                "legacy callback" => r.game.pending_selection_resume = None,
                _ => {}
            }
            let memory = r.memory();
            choose(&mut r, space::PASS);
            assert!(
                r.game.revealed_cards.is_empty(),
                "{path}, result={decline_result}"
            );
            assert_eq!(r.game.players[0].trash.len(), 3);
            assert!(r.game.players[0].hand.is_empty());
            assert!(r.pending_selection().is_none());
            assert_eq!(r.memory(), memory);
        }
    }
}

#[test]
fn reveal_candidate_retains_legacy_hand_and_trash_zone_support() {
    for zone in ["hand", "trash"] {
        let yaml = SEARCH.replace("zone: [reveal]", &format!("zone: [{zone}]"));
        // Syntax validation checks both old zones remain legal after widening.
        DebugRunner::builder()
            .from_dsl_yaml(&yaml)
            .expect("legacy candidate zone remains supported");
    }
}
