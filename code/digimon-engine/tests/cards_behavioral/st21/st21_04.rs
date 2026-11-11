//! ST21-04 Zudomon — Lv.5, Blue, DP 7000, Cost 7. Traits: Sea Beast, ADVENTURE.
//!
//! Printed (card image + data/cards.json; DCGO agrees):
//!   Digivolve: Blue Lv.4 / Cost 3; [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3
//!   [On Play] [When Digivolving] From 1 of your opponent's Digimon, trash any
//!     1 digivolution card for every 2 colors your Tamers have. Then, return 1
//!     of their Digimon with 1 or fewer digivolution cards to the hand.
//!   [Your Turn] [Once Per Turn] shared ADVENTURE observer (see st20_04.rs).
//!   Inherited: <Alliance>
//! Data drift: the official DB bundle words the first sentence "For every 2
//! colors your Tamers have, trash any 1 of your opponent's Digimon's
//! digivolution cards" (no explicit single-Digimon restriction); the printed
//! card and DCGO (SelectTrashDigivolutionCards isFromOnly1Permanent: true)
//! both restrict to ONE Digimon, which is implemented here.
//! DCGO: ST21/Blue/ST21_04.cs — strip maxCount floor(colours/2), canNoTrash
//! false, from 1 permanent; then mandatory bounce pick (≤1 sources).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures (shared ADVENTURE Lv.5 test idiom — see st20/st20_04.rs) ───────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("card YAML in embedded pack")
        .add_card(digimon("ADV", CardColor::Red, 4, 4000, &["ADVENTURE"]))
        .add_card(digimon("ALLY", CardColor::Red, 4, 5000, &["Beast"]))
        .add_card(tamer("T-RB", &[CardColor::Red, CardColor::Blue], &["ADVENTURE"]))
        .add_card(tamer("T-YG", &[CardColor::Yellow, CardColor::Green], &["ADVENTURE"]))
        .add_card(tamer("T-R", &[CardColor::Red], &["ADVENTURE"]))
        .add_card(digimon("SEC", CardColor::Blue, 3, 1000, &[]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
}

fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

fn field_action(index: u8) -> u16 {
    encode_attack(0, index as u16)
}

fn play_other(runner: &mut DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let h = runner.place_on_field(player, id, None);
    runner.fire_play_event_triggers(player, h.index as usize, false, false);
    h
}

fn fire_on_play(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.fire_on_play(h.player, h.index as usize);
}

fn fire_when_digivolving(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

fn is_alliance_prompt(runner: &DebugRunner) -> bool {
    runner
        .pending_selection_view()
        .map(|v| v.kind == SelectionKind::OwnField && v.prompt.contains("Alliance"))
        .unwrap_or(false)
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.player(h.player).battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn stack_len(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner.game.player(h.player).battle_area[h.index as usize]
        .card_sources
        .len()
}

/// Drive a mandatory multi-pick to completion, asserting PASS is never legal
/// before `n` picks are made; returns the number of picks made.
fn pick_first_n(runner: &mut DebugRunner, n: usize) {
    for i in 0..n {
        let v = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("pick {}/{} must be pending", i + 1, n));
        assert!(
            !v.valid_action_ids.contains(&PASS) && !v.is_optional,
            "mandatory pick {}/{} must not allow PASS",
            i + 1,
            n
        );
        let a = v.valid_action_ids[0];
        runner.execute_action(0, a).expect("pick");
    }
}

/// Printed structure shared by every ADVENTURE Lv.5 in this family.
fn assert_adventure_lv5_structure(name: &str, traits: &[&str]) {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, name);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in traits {
        assert!(card.traits.iter().any(|x| x == t), "missing trait {t}");
    }
    let digi: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "colour Lv.4 circle + printed Lv.4 alt box");
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(3))));
}

/// Inherited <Alliance> on the carrier.
fn assert_inherited_alliance() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;
    let host = runner.place_stack(0, &[CARD_ID, "ALLY"]);
    assert!(runner.game.has_keyword(host, Keyword::Alliance));
}

/// Shared [Your Turn][OPT] observer smoke: an own other [ADVENTURE] play →
/// mandatory Alliance pick → optional attack (declined); OPT holds.
fn assert_shared_observer() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    play_other(&mut runner, 0, "ADV");
    assert!(is_alliance_prompt(&runner), "Alliance pick must install");
    let v = runner.pending_selection_view().unwrap();
    assert!(!v.is_optional, "Alliance grant is mandatory");
    runner.execute_action(0, field_action(ally.index)).unwrap();
    assert!(runner.game.has_keyword(ally, Keyword::Alliance));
    let v = runner.pending_selection_view().expect("may-attack pick");
    assert!(v.is_optional, "'may attack' is optional");
    assert!(v.valid_action_ids.contains(&field_action(me.index)));
    runner.execute_action(0, PASS).unwrap();
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.security_count(1), 3);
    // Once per turn.
    play_other(&mut runner, 0, "ADV");
    assert!(runner.pending_selection_view().is_none(), "[Once Per Turn]");
    runner.end_turn();
    assert!(!runner.game.has_keyword(ally, Keyword::Alliance), "for the turn");
}

/// The observer ignores Digimon without [ADVENTURE].
fn assert_shared_observer_ignores_non_adventure() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    play_other(&mut runner, 0, "ALLY");
    assert!(runner.pending_selection_view().is_none());
}

const CARD_ID: &str = "ST21-04";

fn builder() -> DebugRunnerBuilder {
    let mut b = base_builder();
    for i in 0..6 {
        b = b.add_card(digimon(&format!("SRC{i}"), CardColor::Blue, 3, 1000, &[]));
    }
    b.add_card(digimon("TOP-A", CardColor::Blue, 5, 6000, &[]))
        .add_card(digimon("TOP-B", CardColor::Blue, 4, 4000, &[]))
        .add_card(digimon("TOP-C", CardColor::Blue, 4, 4000, &[]))
}

/// Opp stacks: A = 3 sources, B = 0 sources, C = 1 source.
fn opp_board(runner: &mut DebugRunner) -> (PermanentHandle, PermanentHandle, PermanentHandle) {
    let a = runner.place_stack(1, &["SRC0", "SRC1", "SRC2", "TOP-A"]);
    let b = runner.place_on_field(1, "TOP-B", Some(0));
    let c = runner.place_stack(1, &["SRC3", "TOP-C"]);
    (a, b, c)
}

fn opp_field_action(index: u8) -> u16 {
    encode_attack(0, index as u16)
}

fn opp_hand_has(runner: &DebugRunner, id: &str) -> bool {
    runner
        .game
        .player(1)
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

#[test]
fn st21_04_structure() {
    assert_adventure_lv5_structure("Zudomon", &["Sea Beast", "ADVENTURE"]);
    assert_inherited_alliance();
}

/// 4 Tamer colours → N = 2: the host pick offers only opponent Digimon with
/// digivolution cards; exactly 2 are trashed from the chosen one; then the
/// mandatory bounce offers Digimon with ≤1 digivolution cards (A now has 1).
#[test]
fn st21_04_on_play_strips_two_from_one_digimon_then_bounces() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    runner.place_on_field(0, "T-YG", Some(0));
    let (a, b, c) = opp_board(&mut runner);
    let zudo = runner.place_on_field(0, CARD_ID, None);
    fire_on_play(&mut runner, zudo);

    let v = runner.pending_selection_view().expect("strip host pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 2, "A and C carry digivolution cards; B doesn't");
    runner.execute_action(0, opp_field_action(a.index)).unwrap();

    let v = runner.pending_selection_view().expect("source pick");
    assert!(
        matches!(v.kind, SelectionKind::SourceMulti { min: 2, max: 2, .. }),
        "exactly 2 from the chosen Digimon; got {:?}",
        v.kind
    );
    pick_first_n(&mut runner, 2);
    assert_eq!(stack_len(&runner, a), 2, "TOP-A + 1 remaining source");

    let v = runner.pending_selection_view().expect("bounce pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(!v.is_optional, "the return is mandatory");
    assert_eq!(v.valid_action_ids.len(), 3, "A (1 src), B (0), C (1) all qualify");
    runner.execute_action(0, opp_field_action(b.index)).unwrap();
    runner.auto_resolve().unwrap();
    assert!(opp_hand_has(&runner, "TOP-B"));
}

/// N = 2 but the chosen Digimon has only 1 source → the pick clamps to 1.
#[test]
fn st21_04_strip_clamps_to_chosen_digimons_sources() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    runner.place_on_field(0, "T-YG", Some(0));
    let (a, b, c) = opp_board(&mut runner);
    let zudo = runner.place_stack(0, &["ALLY", CARD_ID]);
    fire_when_digivolving(&mut runner, zudo);
    runner.execute_action(0, opp_field_action(c.index)).unwrap();
    let v = runner.pending_selection_view().expect("source pick");
    assert!(
        matches!(v.kind, SelectionKind::SourceMulti { min: 1, max: 1, .. }),
        "clamped to C's single source; got {:?}",
        v.kind
    );
    pick_first_n(&mut runner, 1);
    assert_eq!(stack_len(&runner, c), 1);
    assert_eq!(stack_len(&runner, a), 4, "other Digimon untouched");
}

/// 1 Tamer colour → no strip; the bounce still resolves and never offers a
/// Digimon with 2+ digivolution cards.
#[test]
fn st21_04_one_color_skips_strip_but_still_bounces() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-R", Some(0));
    let (a, b, c) = opp_board(&mut runner);
    let zudo = runner.place_on_field(0, CARD_ID, None);
    fire_on_play(&mut runner, zudo);
    let v = runner.pending_selection_view().expect("bounce pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert_eq!(v.valid_action_ids.len(), 2, "B and C only — A has 3 sources");
    assert!(!v.valid_action_ids.contains(&opp_field_action(a.index)));
    runner.execute_action(0, opp_field_action(c.index)).unwrap();
    runner.auto_resolve().unwrap();
    assert!(opp_hand_has(&runner, "TOP-C"));
    assert_eq!(stack_len(&runner, a), 4);
}

/// 2 Tamer colours but no opponent Digimon has digivolution cards → the
/// strip has no host and is skipped; "Then" still reaches the bounce.
#[test]
fn st21_04_strip_without_any_sources_still_bounces() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    let b = runner.place_on_field(1, "TOP-B", Some(0));
    let zudo = runner.place_on_field(0, CARD_ID, None);
    fire_on_play(&mut runner, zudo);
    let v = runner.pending_selection_view().expect("bounce pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert_eq!(v.valid_action_ids, vec![opp_field_action(b.index)]);
    runner.execute_action(0, opp_field_action(b.index)).unwrap();
    runner.auto_resolve().unwrap();
    assert!(opp_hand_has(&runner, "TOP-B"));
}

#[test]
fn st21_04_shared_observer() {
    assert_shared_observer();
    assert_shared_observer_ignores_non_adventure();
}
