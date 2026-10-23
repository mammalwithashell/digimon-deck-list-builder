//! BT9-104 X Digivolution! — actual image, official Q1911/Q5976 and DCGO
//! BT9/Black/BT9_104.cs. Free evolution retains normal requirements. The
//! bonus draw uses the unrevealed deck; When Digivolving waits for the tail.
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::{CardColor, CardKind, Expiry, ModifierType};

const ID: &str = "BT9-104";
const OBSERVER: &str = r#"
card: WATCH-EVO
name: Watch Evolution
kind: digimon
color: [black]
level: 4
cost: 5
dp: 5000
traits: [X Antibody]
alt_paths:
  - kind: digivolve
    from: {color_is: black, level_eq: 3}
    cost: 3
effects:
  - when: when_digivolving
    process:
      - select_trash:
          of: you
          bind_as: remainder
          filter: {card_number_is: REMAINDER}
          prompt: Observe the already-trashed reveal remainder
          then:
            - gain_memory: 2
"#;

fn card(id: &str, kind: CardKind, x: bool, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    c.level = (kind == CardKind::Digimon).then_some(level);
    c.dp = (kind == CardKind::Digimon).then_some(6000);
    c.play_cost = 3;
    c.traits = if x { vec!["X Antibody".into()] } else { vec![] };
    c
}
fn evo(id: &str, x: bool, color: CardColor, level: u8, cost: u16) -> CardData {
    let mut c = card(id, CardKind::Digimon, x, CardColor::Black, 4);
    c.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level,
        memory_cost: cost,
    }];
    c
}
fn builder() -> DebugRunnerBuilder {
    let mut token = card("X-TOKEN", CardKind::Token, true, CardColor::Black, 5);
    token.level = Some(5);
    token.dp = Some(6000);
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT9-104 must be implemented")
        .from_dsl_yaml(OBSERVER)
        .expect("observer fixture")
        .add_card(token)
        .add_card(card("HOST", CardKind::Digimon, false, CardColor::Black, 3))
        .add_card(card("HOST2", CardKind::Digimon, false, CardColor::Black, 3))
        .add_card(card("X-HOST", CardKind::Digimon, true, CardColor::Black, 3))
        .add_card(card("RED-X", CardKind::Digimon, true, CardColor::Red, 3))
        .add_card(card("RED", CardKind::Digimon, false, CardColor::Red, 3))
        .add_card(card("LEVEL5", CardKind::Digimon, true, CardColor::Black, 5))
        .add_card(card("X-TAMER", CardKind::Tamer, true, CardColor::Red, 0))
        .add_card(card("X-OPTION", CardKind::Option, true, CardColor::Red, 0))
        .add_card(card(
            "REMAINDER",
            CardKind::Digimon,
            false,
            CardColor::Red,
            3,
        ))
        .add_card(card("DRAW", CardKind::Digimon, false, CardColor::Red, 3))
        .add_card(card("FILL", CardKind::Digimon, false, CardColor::Red, 3))
        .add_card(evo("EVOLUTION", true, CardColor::Black, 3, 19))
        .add_card(evo("OTHER-EVO", true, CardColor::Black, 3, 4))
        .add_card(evo("NON-X-EVO", false, CardColor::Black, 3, 2))
        .add_card(evo("WRONG-LEVEL", true, CardColor::Black, 6, 2))
        .add_card(evo("WRONG-COLOR", true, CardColor::Blue, 3, 2))
        .deck(0, &["FILL"; 30])
        .deck(1, &["FILL"; 30])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(10)
}
fn start(mut r: DebugRunner) -> DebugRunner {
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn runner(top_first: &[&str]) -> DebugRunner {
    let deck: Vec<_> = top_first.iter().rev().copied().collect();
    start(builder().hand(0, &[ID]).deck(0, &deck).start())
}
fn ids(r: &DebugRunner, zone: &[CardSource]) -> Vec<String> {
    zone.iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect()
}
fn has(r: &DebugRunner, owner: u8, id: &str, trash: bool) -> bool {
    let p = &r.game.players[owner as usize];
    ids(r, if trash { &p.trash } else { &p.hand })
        .iter()
        .any(|s| s == id)
}
fn add_trash(r: &mut DebugRunner, owner: u8, id: &str) -> CardHandle {
    let i = r.add_to_hand(owner, id);
    let c = r.game.players[owner as usize].hand.remove(i);
    let h = c.handle();
    r.game.players[owner as usize].trash.push(c);
    h
}
fn find(r: &DebugRunner, owner: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[owner as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .expect("field card");
    PermanentHandle {
        player: owner,
        index: index as u8,
    }
}
fn field_action(r: &DebugRunner, id: &str) -> u16 {
    space::encode_attack(0, find(r, 0, id).index as u16)
}
fn choices(r: &DebugRunner, owner: u8, count: usize, optional: bool) -> Vec<u16> {
    let v = r.pending_selection_view().expect("visible decision");
    assert_eq!(v.selecting_player, owner);
    assert_eq!(v.is_optional, optional, "{}", v.prompt);
    let actions: Vec<_> = v
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != space::PASS)
        .collect();
    assert_eq!(actions.len(), count, "{}: {actions:?}", v.prompt);
    for (a, n) in build_action_mask(&r.game, owner).iter().enumerate() {
        assert_eq!(
            *n,
            if actions.contains(&(a as u16)) || (optional && a == space::PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask {a}: {}",
            v.prompt
        );
    }
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|n| *n == 0.0));
    actions
}
fn choose(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn choose_host(r: &mut DebugRunner, id: &str) {
    let a = field_action(r, id);
    choose(r, a);
}
fn reveal_action(r: &DebugRunner, id: &str) -> u16 {
    space::SEL_REVEAL_START
        + r.game
            .revealed_cards
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .expect("revealed") as u16
}
fn choose_reveal(r: &mut DebugRunner, id: &str) {
    let a = reveal_action(r, id);
    choose(r, a);
}
fn choose_trash(r: &mut DebugRunner, p: u8, id: &str) {
    let i = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("trash card");
    choose(r, space::TRASH_EFFECT_START + i as u16);
}
fn use_main(r: &mut DebugRunner) {
    let i = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == ID)
        .unwrap();
    let a = space::PLAY_HAND_START + i as u16;
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
}
fn bottom_is(r: &DebugRunner, owner: u8, host: &str, source: &str) {
    let h = find(r, owner, host);
    assert_eq!(
        r.game.players[owner as usize].battle_area[h.index as usize].card_sources[0]
            .card_id(&r.game.card_data),
        source
    );
}

#[test]
fn bt9_104_color_bypass_requires_own_battle_digimon_with_trait() {
    for mode in 0..6 {
        let mut r = runner(&["FILL"; 8]);
        match mode {
            0 => {
                r.place_on_field(0, "RED-X", Some(0));
            }
            1 => {
                r.place_on_field(0, "X-TAMER", Some(0));
            }
            2 => {
                r.place_in_breeding(0, "RED-X");
            }
            3 => {
                r.place_stack(0, &["RED-X", "RED"]);
            }
            4 => {
                r.place_on_field(1, "RED-X", Some(0));
            }
            _ => {
                r.place_on_field(0, "HOST", Some(0));
            }
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[0],
            if mode == 0 || mode == 5 { 1.0 } else { 0.0 },
            "mode {mode}"
        );
    }
}
#[test]
fn bt9_104_main_free_evolution_draws_unrevealed_then_independently_places_bottom_source() {
    let mut r = runner(&["EVOLUTION", "X-OPTION", "REMAINDER", "DRAW", "FILL"]);
    r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "X-HOST", Some(0));
    use_main(&mut r);
    choices(&r, 0, 2, true);
    assert_eq!(r.game.revealed_cards.len(), 3);
    choose_host(&mut r, "HOST");
    choices(&r, 0, 1, true);
    choose_reveal(&mut r, "EVOLUTION");
    assert!(
        has(&r, 0, "DRAW", false),
        "Q1911: draw from unrevealed deck"
    );
    assert!(has(&r, 0, "REMAINDER", true));
    assert!(r.game.revealed_cards.is_empty());
    assert_eq!(r.memory(), 7, "only Option cost even for cost-19 evolution");
    choices(&r, 0, 1, false);
    choose_trash(&mut r, 0, "X-OPTION");
    choices(&r, 0, 2, false);
    choose_host(&mut r, "X-HOST");
    bottom_is(&r, 0, "X-HOST", "X-OPTION");
    bottom_is(&r, 0, "EVOLUTION", "HOST");
    assert!(has(&r, 0, ID, true));
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt9_104_main_host_mask_checks_routes_kind_zone_owner_and_prohibitions() {
    let mut r = runner(&["EVOLUTION", "REMAINDER", "FILL", "DRAW"]);
    for id in ["HOST", "HOST2", "LEVEL5", "RED-X", "X-TAMER"] {
        r.place_on_field(0, id, Some(0));
    }
    r.place_on_field(1, "HOST", Some(0));
    r.place_in_breeding(0, "HOST");
    let h = find(&r, 0, "HOST2");
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 1),
    );
    use_main(&mut r);
    assert_eq!(choices(&r, 0, 1, true), vec![field_action(&r, "HOST")]);
}
#[test]
fn bt9_104_main_result_mask_requires_revealed_x_digimon_with_a_route() {
    for bad in [
        "NON-X-EVO",
        "WRONG-LEVEL",
        "WRONG-COLOR",
        "X-TAMER",
        "X-OPTION",
    ] {
        let mut r = runner(&["EVOLUTION", bad, "OTHER-EVO", "DRAW"]);
        r.place_on_field(0, "HOST", Some(0));
        r.add_to_hand(0, "EVOLUTION");
        add_trash(&mut r, 0, "EVOLUTION");
        use_main(&mut r);
        choose_host(&mut r, "HOST");
        let actions = choices(&r, 0, 2, true);
        assert!(actions.contains(&reveal_action(&r, "EVOLUTION")));
        assert!(actions.contains(&reveal_action(&r, "OTHER-EVO")));
        assert!(!actions.contains(&reveal_action(&r, bad)), "{bad}");
    }
}
#[test]
fn bt9_104_main_host_or_result_decline_still_trashes_reveal_and_places_source() {
    for result_decline in [false, true] {
        let mut r = runner(&["EVOLUTION", "REMAINDER", "FILL", "DRAW"]);
        r.place_on_field(0, "X-HOST", Some(0));
        use_main(&mut r);
        if result_decline {
            choose_host(&mut r, "X-HOST");
        }
        choose(&mut r, space::PASS);
        assert!(r.game.revealed_cards.is_empty());
        assert!(!has(&r, 0, "DRAW", false));
        choices(&r, 0, 1, false);
        choose_trash(&mut r, 0, "EVOLUTION");
        choices(&r, 0, 1, false);
        choose_host(&mut r, "X-HOST");
        bottom_is(&r, 0, "X-HOST", "EVOLUTION");
        assert_eq!(r.memory(), 7);
    }
}
#[test]
fn bt9_104_main_hand_or_trash_evolution_cannot_enable_a_reveal_host() {
    for from_trash in [false, true] {
        let mut r = runner(&["FILL", "REMAINDER", "FILL", "DRAW"]);
        r.place_on_field(0, "HOST", Some(0));
        if from_trash {
            add_trash(&mut r, 0, "EVOLUTION");
        } else {
            r.add_to_hand(0, "EVOLUTION");
        }
        use_main(&mut r);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert!(r.game.revealed_cards.is_empty());
        assert!(has(&r, 0, "EVOLUTION", from_trash));
    }
}
#[test]
fn bt9_104_main_player_prohibition_suppresses_evolution_but_keeps_source_clause() {
    let mut r = runner(&["EVOLUTION", "REMAINDER", "FILL", "DRAW"]);
    r.place_on_field(0, "X-HOST", Some(0));
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
    use_main(&mut r);
    choices(&r, 0, 1, false);
    choose_trash(&mut r, 0, "EVOLUTION");
    choose_host(&mut r, "X-HOST");
    bottom_is(&r, 0, "X-HOST", "EVOLUTION");
}
#[test]
fn bt9_104_bottom_source_accepts_all_trait_card_kinds_and_is_mandatory() {
    for source in ["EVOLUTION", "X-TAMER", "X-OPTION"] {
        let mut r = runner(&["FILL"; 6]);
        r.place_stack(0, &["REMAINDER", "X-HOST"]);
        add_trash(&mut r, 0, source);
        add_trash(&mut r, 0, "RED");
        add_trash(&mut r, 1, "X-OPTION");
        use_main(&mut r);
        choices(&r, 0, 1, false);
        choose_trash(&mut r, 0, source);
        choices(&r, 0, 1, false);
        choose_host(&mut r, "X-HOST");
        bottom_is(&r, 0, "X-HOST", source);
        assert!(has(&r, 0, "RED", true));
        assert!(has(&r, 1, "X-OPTION", true));
    }
}
#[test]
fn bt9_104_bottom_source_mask_excludes_tokens_tamers_other_traits_and_zones() {
    let mut r = runner(&["FILL"; 6]);
    for id in ["X-HOST", "RED-X", "HOST", "X-TAMER", "X-TOKEN"] {
        r.place_on_field(0, id, Some(0));
    }
    r.place_on_field(1, "X-HOST", Some(0));
    r.place_in_breeding(0, "X-HOST");
    add_trash(&mut r, 0, "X-OPTION");
    use_main(&mut r);
    choose_trash(&mut r, 0, "X-OPTION");
    let mut actual = choices(&r, 0, 2, false);
    let mut expected = vec![field_action(&r, "X-HOST"), field_action(&r, "RED-X")];
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
#[test]
fn bt9_104_short_or_empty_deck_resolves_available_cards_without_illegal_prompt() {
    for deck in [vec![], vec!["REMAINDER"], vec!["REMAINDER", "FILL"]] {
        let mut r = runner(&deck);
        r.place_on_field(0, "HOST", Some(0));
        use_main(&mut r);
        assert!(r.pending_selection().is_none());
        assert!(r.game.revealed_cards.is_empty());
        assert_eq!(r.deck_size(0), 0);
        assert_eq!(r.game.players[0].trash.len(), deck.len() + 1);
    }
}
#[test]
fn bt9_104_when_digivolving_observes_trashed_remainder_after_option_tail_q5976() {
    let mut r = runner(&["WATCH-EVO", "REMAINDER", "FILL", "DRAW", "FILL"]);
    r.place_on_field(0, "HOST", Some(0));
    use_main(&mut r);
    choose_host(&mut r, "HOST");
    choose_reveal(&mut r, "WATCH-EVO");
    assert!(has(&r, 0, "DRAW", false));
    assert!(has(&r, 0, "REMAINDER", true));
    assert!(r.game.revealed_cards.is_empty());
    choices(&r, 0, 1, false);
    choose_trash(&mut r, 0, "REMAINDER");
    assert_eq!(r.memory(), 9, "When Digivolving observes trashed remainder");
}
#[test]
fn bt9_104_duplicate_reveal_and_pending_clone_keep_physical_identity() {
    let mut r = runner(&["EVOLUTION", "EVOLUTION", "REMAINDER", "DRAW", "FILL"]);
    r.place_on_field(0, "HOST", Some(0));
    use_main(&mut r);
    choose_host(&mut r, "HOST");
    let actions = choices(&r, 0, 2, true);
    let chosen = r.game.revealed_cards[1].handle();
    let other = r.game.revealed_cards[0].handle();
    r.game = r.game.clone();
    choose(&mut r, actions[1]);
    let h = find(&r, 0, "EVOLUTION");
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .top_card()
            .handle(),
        chosen
    );
    assert!(r.game.players[0].trash.iter().any(|c| c.handle() == other));
    choose_trash(&mut r, 0, "EVOLUTION");
    choose_host(&mut r, "EVOLUTION");
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize].card_sources[0].handle(),
        other
    );
}
#[test]
fn bt9_104_departed_selected_host_cannot_evolve_new_permanent_in_same_slot() {
    let mut r = runner(&["EVOLUTION", "REMAINDER", "FILL", "DRAW"]);
    let host = r.place_on_field(0, "HOST", Some(0));
    use_main(&mut r);
    choose_host(&mut r, "HOST");
    let action = reveal_action(&r, "EVOLUTION");
    r.game.players[0].battle_area.remove(host.index as usize);
    r.place_on_field(0, "HOST2", Some(0));
    choose(&mut r, action);
    assert_eq!(find(&r, 0, "HOST2").index, host.index);
    assert!(has(&r, 0, "EVOLUTION", true));
    assert!(!has(&r, 0, "DRAW", false));
    assert!(r.pending_selection().is_none());
}
fn security_runner(top_first: &[&str]) -> DebugRunner {
    let deck: Vec<_> = top_first.iter().rev().copied().collect();
    let mut r = start(builder().security(1, &["FILL", ID]).deck(1, &deck).start());
    let attacker = r.place_on_field(0, "HOST", Some(0));
    let action = space::encode_attack(attacker.index as u16, space::SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    r
}
#[test]
fn bt9_104_security_may_decline_reveal_without_modifying_deck_or_drawing() {
    let mut r = security_runner(&["EVOLUTION", "X-OPTION", "REMAINDER", "DRAW"]);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    choices(&r, 1, 1, true);
    let before = r.deck_size(1);
    choose(&mut r, space::PASS);
    assert_eq!(r.deck_size(1), before);
    assert!(r.game.players[1].hand.is_empty());
    assert!(has(&r, 1, ID, true));
    assert!(r.game.revealed_cards.is_empty());
}
#[test]
fn bt9_104_security_acceptance_forces_one_trait_card_of_any_kind_and_trashes_rest() {
    for source in ["EVOLUTION", "X-TAMER", "X-OPTION"] {
        let mut r = security_runner(&[source, "RED-X", "REMAINDER", "DRAW"]);
        let before = r.memory();
        choose(&mut r, space::REPLACEMENT_ACCEPT);
        choices(&r, 1, 2, false);
        choose_reveal(&mut r, source);
        assert!(has(&r, 1, source, false));
        assert!(has(&r, 1, "RED-X", true));
        assert!(has(&r, 1, "REMAINDER", true));
        assert!(!has(&r, 0, source, false));
        assert_eq!(r.deck_size(1), 1);
        assert_eq!(r.memory(), before);
        assert!(has(&r, 1, ID, true));
        assert!(r.game.revealed_cards.is_empty());
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt9_104_security_no_matches_trashes_available_reveal_without_evolution() {
    for deck in [
        vec![],
        vec!["FILL"],
        vec!["FILL", "REMAINDER", "FILL", "DRAW"],
    ] {
        let mut r = security_runner(&deck);
        choose(&mut r, space::REPLACEMENT_ACCEPT);
        assert!(r.pending_selection().is_none());
        assert!(r.game.players[1].hand.is_empty());
        assert!(r.game.revealed_cards.is_empty());
        assert_eq!(r.deck_size(1), deck.len().saturating_sub(3));
        assert!(r.game.players[1].battle_area.is_empty());
    }
}
