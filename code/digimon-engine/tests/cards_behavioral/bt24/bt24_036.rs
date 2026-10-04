//! BT24-036 Medicmon — Digimon, Lv.4, Yellow, DP 4000, Play Cost 4.
//!
//! # Official text (data/card_bundles/BT24-036.md — official Bandai DB)
//!
//! Form: Sup./Appmon | Trait: Medical | Attribute: Life
//! Digivolve: Yellow Lv.3 / cost 2
//!
//! Effect:
//!   [Security] At the end of the battle, play this card without paying the cost.
//!   [On Play] [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.
//! Link DP: DP+3000
//! Link Condition: ＜Link＞ [Appmon] trait: Cost 2 (Plug this card from the hand or
//!   battle area sideways into the specified Digimon in the battle area.)
//! Link Effect: [On Deletion] 1 of your opponent's Digimon gets -5000 DP for the turn.
//!
//! Official Q&A: when the host is deleted, the [On Deletion] LINK effect is pending
//! only for the host — Medicmon's own (face-up) [On Deletion] does not activate.
//!
//! # DCGO reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_036.cs
//! - Alt digivolve: HasStandardAppTraits ([Stnd.]) cost 2.
//! - Link condition: HasAppmonTraits, cost 2.
//! - Security: PlaySelfDigimonAfterBattleSecurityEffect.
//! - [On Play]/[On Deletion]: canNoSelect:false, -3000 UntilEachTurnEnd.
//! - Link [On Deletion]: SetIsLinkedEffect(true), canNoSelect:false, -5000 UntilEachTurnEnd.
//!
//! # Pattern tags
//! on_security play_from_security · [On Play][On Deletion] shared body ·
//! add_dp_modifier end_of_turn (mandatory) · link_condition · scope:linked aura ·
//! scope:linked on_deletion (host deletion fires link effect) · alt_path Stnd. ·
//! printed-circle alt_path · DP-zero deletion

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT24-036";
const YAML: &str = include_str!("../../../cards/bt24/BT24-036.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

fn make_appmon_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.traits = vec!["Appmon".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-036 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_opp_digimon("OPP-3000", 3000))
        .add_card(make_opp_digimon("OPP-6000", 6000))
        .add_card(make_opp_digimon("OPP-8000", 8000))
        .add_card(make_appmon_digimon("HOST-APPMON", 4, 5000))
        .add_card(make_opp_digimon("PLAIN-HOST", 5000))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn link_action_id(field_index: u8) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK
}

/// Link the on-field Medicmon onto the (single valid) host.
fn do_link(runner: &mut DebugRunner, linking_perm: PermanentHandle) {
    runner
        .game
        .decode_action(link_action_id(linking_perm.index), 0);
    let host_action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("host-pick prompt")
        .valid_action_ids[0];
    let _ = runner.game.resolve_selection(0, host_action);
    let _ = runner.auto_resolve();
}

/// Stage HOST-APPMON (field 0) with Medicmon linked into it. Returns the host.
fn host_with_medicmon_linked(runner: &mut DebugRunner) -> PermanentHandle {
    let host = runner.place_on_field(0, "HOST-APPMON", Some(0));
    let medic = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    do_link(runner, medic);
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "Medicmon left the field into the link"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "Medicmon is linked to the host"
    );
    host
}

fn advance_to_turn_player(runner: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if runner.turn_player() == target {
            return;
        }
        let _ = runner.auto_resolve();
        runner.pass_turn();
        let _ = runner.auto_resolve();
        if runner.game.current_phase == GamePhase::EndOfTurnAction {
            runner.game.pass_end_of_turn_action();
            let _ = runner.auto_resolve();
        }
    }
    assert_eq!(runner.turn_player(), target);
}

fn triggered_with<'a>(
    card: &'a digimon_dsl::compiled::CompiledCard,
    pred: impl Fn(&digimon_dsl::compiled::CompiledTriggeredClause) -> bool,
) -> Option<&'a digimon_dsl::compiled::CompiledTriggeredClause> {
    card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if pred(t) => Some(t),
        _ => None,
    })
}

// ─── Section 1: Structural ────────────────────────────────────────────────────

#[test]
fn bt24_036_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("in pack");
    assert_eq!(card.name, "Medicmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.cost, Some(4));
    for t in ["Sup.", "Appmon", "Medical"] {
        assert!(card.traits.iter().any(|x| x == t), "missing trait {t}");
    }
}

#[test]
fn bt24_036_security_clause_present() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    assert!(triggered_with(card, |t| t.when.contains(&CompiledTiming::OnSecurity)).is_some());
}

#[test]
fn bt24_036_on_play_on_deletion_clause_mandatory_face_up() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let t = triggered_with(card, |t| {
        t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::OnDeletion)
    })
    .expect("[On Play][On Deletion] clause");
    assert!(!t.optional, "DCGO canNoSelect:false → mandatory");
    assert!(
        !matches!(t.scope, CompiledScope::Linked),
        "the -3000 clause is the face-up effect, not the link effect"
    );
}

#[test]
fn bt24_036_link_condition_appmon_cost_2() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let has = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition {
            cost,
            filter,
            ..
        }) => *cost == 2 && filter.trait_has.as_deref() == Some("Appmon"),
        _ => false,
    });
    assert!(has, "link_condition cost 2 / trait_has Appmon");
}

#[test]
fn bt24_036_linked_dp_aura_3000() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) if *scope == CompiledScope::Linked => Some(*dp_modifier),
        _ => None,
    });
    assert_eq!(aura, Some(Some(3000)));
}

#[test]
fn bt24_036_linked_on_deletion_clause_mandatory() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let t = triggered_with(card, |t| {
        t.when.contains(&CompiledTiming::OnDeletion) && matches!(t.scope, CompiledScope::Linked)
    })
    .expect("scope: linked [On Deletion] clause");
    assert!(!t.optional);
}

#[test]
fn bt24_036_alt_paths_yellow_lv3_and_stnd_cost_2() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let digi = |pred: &dyn Fn(&digimon_dsl::compiled::CompiledAltPath) -> bool| {
        card.alt_paths.iter().any(|p| {
            matches!(p.kind, CompiledAltPathKind::Digivolve)
                && p.cost == Some(CompiledCost::Literal(2))
                && pred(p)
        })
    };
    assert!(digi(&|p| p
        .from
        .as_ref()
        .map(|f| f.trait_has.as_deref() == Some("Stnd."))
        .unwrap_or(false)));
    assert!(digi(&|p| p
        .from
        .as_ref()
        .map(|f| f.level_eq == Some(3))
        .unwrap_or(false)));
}

// ─── Section 2: Digivolve routes ──────────────────────────────────────────────

fn make_lv3_base(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn try_digivolve_over(base_card: CardData) -> (bool, i16) {
    let base_id = base_card.card_id.clone();
    let mut r = base()
        .add_card(base_card)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, &base_id, Some(0));
    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    r.game.drain_effect_queue();
    (proceeded, mem_before - r.game.memory)
}

#[test]
fn bt24_036_digivolves_from_yellow_lv3_for_2() {
    let (ok, paid) = try_digivolve_over(make_lv3_base("Y-LV3", CardColor::Yellow, &[]));
    assert!(ok, "printed circle Yellow Lv.3 / cost 2");
    assert_eq!(paid, 2);
}

#[test]
fn bt24_036_digivolves_from_offcolor_stnd_appmon_for_2() {
    let (ok, paid) = try_digivolve_over(make_lv3_base(
        "R-STND",
        CardColor::Red,
        &["Stnd.", "Appmon"],
    ));
    assert!(ok, "DCGO HasStandardAppTraits route");
    assert_eq!(paid, 2);
}

#[test]
fn bt24_036_no_route_from_offcolor_plain_lv3() {
    let (ok, paid) = try_digivolve_over(make_lv3_base("R-PLAIN", CardColor::Red, &[]));
    assert!(!ok, "red traitless Lv.3 matches neither requirement");
    assert_eq!(paid, 0);
}

// ─── Section 3: [On Play] ─────────────────────────────────────────────────────

#[test]
fn bt24_036_on_play_installs_mandatory_opp_selection() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(1, "OPP-6000", Some(0));
    runner.play(0, 0).expect("plays");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional(), "mandatory selection");
}

#[test]
fn bt24_036_on_play_applies_minus_3000() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let opp = runner.place_on_field(1, "OPP-6000", Some(0));
    runner.play(0, 0).expect("plays");
    runner.auto_resolve().unwrap();
    assert_eq!(runner.dp_of(opp), Some(3000));
}

#[test]
fn bt24_036_on_play_no_opponent_digimon_no_selection() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.play(0, 0).expect("plays");
    assert!(runner.pending_selection().is_none());
}

#[test]
fn bt24_036_on_play_deletes_3000_dp_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(1, "OPP-3000", Some(0));
    runner.play(0, 0).expect("plays");
    runner.auto_resolve().unwrap();
    assert_eq!(runner.battle_area_size(1), 0, "0 DP → deleted (17-1-3-1)");
}

#[test]
fn bt24_036_on_play_debuff_lasts_only_for_the_turn() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let opp = runner.place_on_field(1, "OPP-6000", Some(0));
    runner.play(0, 0).expect("plays");
    runner.auto_resolve().unwrap();
    assert_eq!(runner.dp_of(opp), Some(3000));
    advance_to_turn_player(&mut runner, 1);
    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "\"for the turn\" — expired at the end of the turn it was applied"
    );
}

// ─── Section 4: [On Deletion] (face-up) ───────────────────────────────────────

#[test]
fn bt24_036_on_deletion_applies_minus_3000() {
    let mut runner = base().memory(10).start();
    runner.game.enter_main_phase();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-6000", Some(0));
    runner.game_mut().delete_permanent_with_effects(me);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner.auto_resolve().unwrap();
    assert_eq!(runner.dp_of(opp), Some(3000));
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn bt24_036_on_deletion_no_opponent_digimon_no_selection() {
    let mut runner = base().memory(10).start();
    runner.game.enter_main_phase();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game_mut().delete_permanent_with_effects(me);
    assert!(runner.pending_selection().is_none());
}

// ─── Section 5: [Security] ────────────────────────────────────────────────────

#[test]
fn bt24_036_security_plays_card_after_battle() {
    let mut runner = base()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .unwrap();
        let next = runner.game.next_card_index();
        runner.game.players[1]
            .security
            .push(CardSource::new(idx, 1, next));
    }
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Medicmon played onto its owner's battle area free"
    );
    let top = runner.game.players[1].battle_area[0]
        .card_sources
        .last()
        .unwrap()
        .data_index;
    assert_eq!(runner.game.card_data[top].card_id, CARD_ID);
}

// ─── Section 6: Link ──────────────────────────────────────────────────────────

#[test]
fn bt24_036_link_gives_host_plus_3000_dp() {
    let mut runner = base().memory(10).start();
    let host = runner.place_on_field(0, "HOST-APPMON", Some(0));
    let medic = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    let before = runner.effective_dp(host).unwrap();
    do_link(&mut runner, medic);
    assert_eq!(runner.effective_dp(host).unwrap(), before + 3000);
}

#[test]
fn bt24_036_cannot_link_to_non_appmon_host() {
    let mut runner = base().memory(10).start();
    let _host = runner.place_on_field(0, "PLAIN-HOST", Some(0));
    let medic = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    runner.game.decode_action(link_action_id(medic.index), 0);
    assert_eq!(
        runner.game.players[0].battle_area[0].linked_cards.len(),
        0,
        "no [Appmon] host → no link"
    );
    assert_eq!(runner.battle_area_size(0), 2);
}

/// Link [On Deletion]: host deleted → -5000 to an opponent Digimon. Medicmon's
/// own face-up -3000 [On Deletion] must NOT also fire (official Q&A).
#[test]
fn bt24_036_link_on_deletion_applies_minus_5000_only() {
    let mut runner = base().memory(10).start();
    let host = host_with_medicmon_linked(&mut runner);
    let opp = runner.place_on_field(1, "OPP-8000", Some(0));
    let trash_before = runner.trash_size(0);

    runner.game_mut().delete_permanent_with_effects(host);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional());
    runner.auto_resolve().unwrap();
    assert!(
        runner.pending_selection().is_none(),
        "exactly one effect fired"
    );
    assert_eq!(
        runner.dp_of(opp),
        Some(3000),
        "8000 - 5000 (link effect only; no extra -3000)"
    );
    assert_eq!(runner.battle_area_size(0), 0);
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "host + Medicmon trashed"
    );
}

#[test]
fn bt24_036_link_on_deletion_deletes_5000_or_less() {
    let mut runner = base().memory(10).start();
    let host = host_with_medicmon_linked(&mut runner);
    runner.place_on_field(1, "OPP-3000", Some(0));
    runner.game_mut().delete_permanent_with_effects(host);
    runner.auto_resolve().unwrap();
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn bt24_036_link_on_deletion_no_opponent_digimon_no_selection() {
    let mut runner = base().memory(10).start();
    let host = host_with_medicmon_linked(&mut runner);
    runner.game_mut().delete_permanent_with_effects(host);
    assert!(runner.pending_selection().is_none());
}

/// Negative: the link [On Deletion] only fires on the host's deletion, not when
/// a different Digimon of ours is deleted.
#[test]
fn bt24_036_link_on_deletion_not_fired_by_other_deletion() {
    let mut runner = base().memory(10).start();
    let host = host_with_medicmon_linked(&mut runner);
    let other = runner.place_on_field(0, "PLAIN-HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-8000", Some(0));
    runner.game_mut().delete_permanent_with_effects(other);
    let _ = runner.auto_resolve();
    assert_eq!(runner.dp_of(opp), Some(8000));
    assert_eq!(
        runner.game.players[0].battle_area[0].linked_cards.len(),
        1,
        "Medicmon still linked to the surviving host"
    );
}
