//! P-209 Titamon — Digimon, Lv.6, Purple/Green, Cost 11, DP 11000,
//! Shaman/Titan/TS (+ Rule-granted [Demon]), Virus.
//! Digivolve: Purple Lv.5 cost 3, Green Lv.5 cost 3 (card_overrides.json);
//! [Digivolve] Lv.5 w/[Demon]/[TS] trait: Cost 3.
//!
//! Printed text (official Bandai DB, `data/card_bundles/P-209.md`):
//!   <Alliance>
//!   [On Play] [When Digivolving] By trashing 1 card in your hand, suspend 1
//!   of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers
//!   can't unsuspend until their turn ends.
//!   [All Turns] [Once Per Turn] When your hand is trashed from, you may play
//!   1 level 4 or lower [Demon] or [Titan] trait card from your trash without
//!   paying the cost.
//!   (Rule) Trait: Has [Demon] Type.
//!
//! Official Q&A: if you don't trash a card ("by" cost), the rest of the
//! effect doesn't activate.
//!
//! NOTE: P-209 is NOT a reprint of BT25-084 Titamon (different text).
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/P/Purple/P_209.cs
//!   - Alt digivolve: Lv.5 && (Demon || TS traits), cost 3.
//!   - AllianceSelfEffect.
//!   - OP / WD (separate ActivateClass, non-OPT, non-optional outer):
//!     SelectHandEffect discard maxCount 1, canNoSelect TRUE; if trashed →
//!     SelectPermanent (opp Digimon/Tamer) canNoSelect false, Mode.Tap; then
//!     SelectPermanent (opp Digimon/Tamer) canNoSelect false →
//!     GainCanNotUnsuspend UntilOpponentTurnEnd.
//!   - OnDiscardHand ([All Turns][OPT], isOptional true): own hand trashed
//!     from → SelectCard from trash, canNoSelect true, IsDigimon && Level ≤ 4
//!     && (Demon || Titan) → PlayPermanentCards payCost false.
//!
//! Patterns: optional "by trashing" hand cost gating a suspend + lock;
//! on_discard_hand OPT optional play-from-trash; Rule-granted trait; trait alt
//! digivolve; Alliance keyword.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "P-209";

/// Test-only opponent card: "[On Play] Trash 1 card in your hand".
const OPP_HAND_TRASHER_YAML: &str = r#"
card: T-HANDTRASH
name: HandTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash 1 card in your hand"
    process:
      - select_hand:
          of: you
          bind_as: gone
          filter: {}
          prompt: "Trash 1 card in your hand"
      - trash_from_hand_by_index: { of: you, hand_index: gone }
"#;

fn digimon(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16 + 2;
    c.colors = vec![color];
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Blue];
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-209 DSL card")
        .from_dsl_yaml(OPP_HAND_TRASHER_YAML)
        .expect("hand trasher fixture")
        .add_card(make_test_card("FODDER", "Fodder"))
        .add_card(digimon("OPP-A", 5, CardColor::Red, &[]))
        .add_card(digimon("OPP-B", 4, CardColor::Red, &[]))
        .add_card(tamer("OPP-TAMER", &[]))
        .add_card(digimon("DEMON-4", 4, CardColor::Purple, &["Demon"]))
        .add_card(digimon("TITAN-3", 3, CardColor::Purple, &["Titan"]))
        .add_card(digimon("DEMON-5", 5, CardColor::Purple, &["Demon"]))
        .add_card(digimon("PLAIN-4", 4, CardColor::Purple, &["Beast"]))
        .add_card(tamer("DEMON-TAMER", &["Demon"]))
        .add_card(digimon("TS-5", 5, CardColor::Black, &["TS"]))
        .add_card(digimon("DEMON-BASE-5", 5, CardColor::Red, &["Demon"]))
        .add_card(digimon("PLAIN-5", 5, CardColor::Red, &["Beast"]))
        .deck(0, &["FODDER"; 8])
        .deck(1, &["FODDER"; 8])
        .memory(10)
        .start()
}

fn data_idx(r: &DebugRunner, id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .unwrap_or_else(|| panic!("unknown card {id}"))
}

fn push_hand(r: &mut DebugRunner, p: u8, id: &str) {
    let idx = data_idx(r, id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

fn push_trash(r: &mut DebugRunner, p: u8, id: &str) {
    let idx = data_idx(r, id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn pick_hand(r: &mut DebugRunner, p: u8, id: &str) {
    let idx = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"));
    let action = PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{id} not a legal hand pick"
    );
    r.execute_action(p, action).expect("hand pick");
}

fn trash_action(r: &DebugRunner, p: u8, id: &str) -> u16 {
    let idx = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in trash"));
    TRASH_EFFECT_START + idx as u16
}

/// Opp-field action for permanent `h` on the opponent's side (OppField
/// selections encode `encode_attack(0, slot)`).
fn opp_action(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

fn pick(r: &mut DebugRunner, p: u8, action: u16) {
    let v = r.pending_selection_view().expect("selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "action {action} not legal (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).expect("pick");
}

/// Accept an outer optional-trigger confirm, if one is pending.
fn accept_outer(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("outer confirm pending");
    let yes = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("accept action");
    r.execute_action(0, yes).expect("accept");
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.player(h.player).battle_area[h.index as usize].is_suspended
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|x| x.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

// ── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn p_209_structure_traits_keywords_alt_path() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiles");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    for t in ["Shaman", "Titan", "TS", "Demon"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t} present");
    }
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "Alliance"
    )));
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].kind, CompiledAltPathKind::Digivolve);

    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let opwd = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(opwd.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!opwd.once_per_turn);
    let disc = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnDiscardHand))
        .expect("[All Turns] hand-trashed clause");
    assert!(disc.once_per_turn, "[Once Per Turn]");
    assert!(disc.optional, "'you may play'");
}

// ── Alt digivolve ──────────────────────────────────────────────────────────

fn try_digivolve_onto(base_id: &str) -> (bool, DebugRunner) {
    let mut r = runner();
    r.game.turn_count = 2;
    let base = r.place_on_field(0, base_id, Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.game.enter_main_phase();
    let mut mem_before = r.memory();
    let _ = r.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    // Cost choice (if two paths apply) — take the first legal option.
    if r.pending_kind().is_some() && r.game.player(0).battle_area[0].card_sources.len() == 1 {
        let a = r.pending_selection().unwrap().valid_action_ids[0];
        r.execute_action(0, a).ok();
    }
    let done = r.game.player(0).battle_area[base.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        == CARD_ID;
    if done {
        mem_before -= 3;
        assert_eq!(r.memory(), mem_before, "alt digivolve cost 3");
    }
    (done, r)
}

#[test]
fn p_209_alt_digivolve_from_lv5_demon_cost_3() {
    let (ok, _) = try_digivolve_onto("DEMON-BASE-5");
    assert!(ok, "Lv.5 [Demon] (red) base qualifies");
}

#[test]
fn p_209_alt_digivolve_from_lv5_ts_cost_3() {
    let (ok, _) = try_digivolve_onto("TS-5");
    assert!(ok, "Lv.5 [TS] (black) base qualifies");
}

#[test]
fn p_209_alt_digivolve_rejects_plain_red_lv5() {
    let (ok, _) = try_digivolve_onto("PLAIN-5");
    assert!(!ok, "a red Lv.5 with neither trait does not qualify");
}

// ── Section 2/3: [On Play] / [When Digivolving] ─────────────────────────────

#[test]
fn p_209_on_play_from_hand_trash_suspend_and_lock() {
    let mut r = runner();
    r.game.enter_main_phase();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let ot = r.place_on_field(1, "OPP-TAMER", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FODDER");
    r.play(0, 0).expect("play Titamon");

    let v = r.pending_selection_view().expect("optional hand cost");
    assert!(v.is_optional, "'by trashing' cost is declinable");
    pick_hand(&mut r, 0, "FODDER");
    // (Titamon's own [All Turns] hand-trashed trigger may queue here; the trash
    // has no eligible card so declining it changes nothing.)
    while r.pending_kind() != Some(SelectionKind::OppField) {
        let k = r.pending_kind().expect("suspend pick pending");
        assert_ne!(k, SelectionKind::Trash, "no eligible trash card offered");
        r.execute_action(0, PASS)
            .expect("decline hand-trashed trigger");
    }
    let v = r.pending_selection_view().unwrap();
    assert!(!v.is_optional, "suspend pick is mandatory once paid");
    assert_eq!(v.valid_action_ids.len(), 2, "Digimon or Tamer");
    pick(&mut r, 0, opp_action(oa));
    assert!(suspended(&r, oa));

    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional, "lock pick is mandatory");
    pick(&mut r, 0, opp_action(ot));
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(ot, ModifierType::CannotUnsuspend));
    assert!(!r.modifiers().has(oa, ModifierType::CannotUnsuspend));
    assert!(r
        .game
        .player(0)
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "FODDER"));
}

#[test]
fn p_209_on_play_decline_cost_does_nothing() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FODDER");
    fire(&mut r, EffectTiming::OnPlay, tm);
    r.execute_action(0, PASS).expect("decline cost");
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, oa), "no suspend without the trash cost");
    assert!(!r.modifiers().has(oa, ModifierType::CannotUnsuspend));
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn p_209_on_play_empty_hand_does_nothing() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].hand.clear();
    fire(&mut r, EffectTiming::OnPlay, tm);
    let _ = r.auto_resolve();
    assert!(!suspended(&r, oa));
    assert!(!r.modifiers().has(oa, ModifierType::CannotUnsuspend));
}

#[test]
fn p_209_when_digivolving_suspends_tamer_and_locks_same_target() {
    let mut r = runner();
    let ot = r.place_on_field(1, "OPP-TAMER", Some(0));
    r.place_on_field(1, "OPP-B", Some(0));
    let tm = r.place_stack(0, &["DEMON-BASE-5", CARD_ID]);
    push_hand(&mut r, 0, "FODDER");
    fire(&mut r, EffectTiming::WhenDigivolving, tm);
    pick_hand(&mut r, 0, "FODDER");
    while r.pending_kind() != Some(SelectionKind::OppField) {
        r.execute_action(0, PASS)
            .expect("decline hand-trashed trigger");
    }
    pick(&mut r, 0, opp_action(ot));
    assert!(suspended(&r, ot), "a Tamer can be suspended");
    pick(&mut r, 0, opp_action(ot));
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(ot, ModifierType::CannotUnsuspend));
}

#[test]
fn p_209_lock_lasts_through_opponents_turn() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FODDER");
    fire(&mut r, EffectTiming::OnPlay, tm);
    pick_hand(&mut r, 0, "FODDER");
    while r.pending_kind() != Some(SelectionKind::OppField) {
        r.execute_action(0, PASS).expect("decline");
    }
    pick(&mut r, 0, opp_action(oa));
    pick(&mut r, 0, opp_action(oa));
    let _ = r.auto_resolve();
    r.end_turn(); // → opponent's turn: unsuspend phase must not unsuspend it
    assert!(
        suspended(&r, oa),
        "can't unsuspend during their unsuspend phase"
    );
    assert!(r.modifiers().has(oa, ModifierType::CannotUnsuspend));
    r.end_turn(); // their turn ends → lock expires
    assert!(!r.modifiers().has(oa, ModifierType::CannotUnsuspend));
}

// ── Section 3/5: [All Turns][OPT] hand trashed → play from trash ───────────

/// Drive Titamon's own [On Play] cost to trash FODDER (an own-effect hand
/// trash), which fires the [All Turns] observer.
fn own_trash_via_on_play(r: &mut DebugRunner, tm: PermanentHandle) {
    push_hand(r, 0, "FODDER");
    fire(r, EffectTiming::OnPlay, tm);
    pick_hand(r, 0, "FODDER");
}

/// Resolve the remaining [On Play] picks (suspend + lock) on OPP-A.
fn finish_on_play(r: &mut DebugRunner, oa: PermanentHandle) {
    while r.pending_kind() == Some(SelectionKind::OppField) {
        pick(r, 0, opp_action(oa));
    }
}

#[test]
fn p_209_hand_trashed_offers_only_lv4_or_lower_demon_or_titan_digimon() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    for id in ["DEMON-4", "TITAN-3", "DEMON-5", "PLAIN-4", "DEMON-TAMER"] {
        push_trash(&mut r, 0, id);
    }
    own_trash_via_on_play(&mut r, tm);
    // Drive until the trash pick appears (outer confirm accepted).
    let mut guard = 0;
    while r.pending_kind() != Some(SelectionKind::Trash) {
        guard += 1;
        assert!(guard < 10, "trash pick never surfaced");
        match r.pending_kind() {
            Some(SelectionKind::OppField) => pick(&mut r, 0, opp_action(oa)),
            Some(_) => accept_outer(&mut r),
            None => panic!("hand-trashed trigger did not fire"),
        }
    }
    let v = r.pending_selection_view().unwrap();
    let mut legal: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    legal.sort();
    let mut want = vec![
        trash_action(&r, 0, "DEMON-4"),
        trash_action(&r, 0, "TITAN-3"),
    ];
    want.sort();
    assert_eq!(legal, want, "only Lv.4- [Demon]/[Titan] Digimon");
    assert!(v.is_optional, "may choose none (DCGO canNoSelect)");
}

#[test]
fn p_209_hand_trashed_plays_demon_free_then_opt_locks() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "DEMON-4");
    push_trash(&mut r, 0, "TITAN-3");
    let mem = r.memory();
    own_trash_via_on_play(&mut r, tm);
    let mut guard = 0;
    while r.pending_kind() != Some(SelectionKind::Trash) {
        guard += 1;
        assert!(guard < 10);
        match r.pending_kind() {
            Some(SelectionKind::OppField) => pick(&mut r, 0, opp_action(oa)),
            Some(_) => accept_outer(&mut r),
            None => panic!("hand-trashed trigger did not fire"),
        }
    }
    assert!(
        suspended(&r, oa) && r.modifiers().has(oa, ModifierType::CannotUnsuspend),
        "the [On Play] effect finishes (suspend + lock) before the hand-trashed \
         trigger resolves"
    );
    let a = trash_action(&r, 0, "DEMON-4");
    pick(&mut r, 0, a);
    finish_on_play(&mut r, oa);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DEMON-4".to_string()), "played");
    assert_eq!(r.memory(), mem, "without paying the cost");

    // Second own hand-trash this turn: OPT spent → no play offered.
    own_trash_via_on_play(&mut r, tm);
    let mut guard = 0;
    while r.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 10);
        assert_ne!(
            r.pending_kind(),
            Some(SelectionKind::Trash),
            "[Once Per Turn] — no second play this turn"
        );
        match r.pending_kind() {
            Some(SelectionKind::OppField) => pick(&mut r, 0, opp_action(oa)),
            _ => panic!("unexpected prompt {:?}", r.pending_kind()),
        }
    }
    assert!(!field_ids(&r, 0).contains(&"TITAN-3".to_string()));
}

#[test]
fn p_209_hand_trashed_opt_resets_next_turn() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "DEMON-4");
    push_trash(&mut r, 0, "TITAN-3");
    own_trash_via_on_play(&mut r, tm);
    let mut guard = 0;
    while r.pending_kind() != Some(SelectionKind::Trash) {
        guard += 1;
        assert!(guard < 10);
        match r.pending_kind() {
            Some(SelectionKind::OppField) => pick(&mut r, 0, opp_action(oa)),
            Some(_) => accept_outer(&mut r),
            None => panic!("did not fire"),
        }
    }
    let a = trash_action(&r, 0, "DEMON-4");
    pick(&mut r, 0, a);
    finish_on_play(&mut r, oa);
    let _ = r.auto_resolve();

    r.end_turn(); // opponent's turn — [All Turns] works here too
    own_trash_via_on_play(&mut r, tm);
    let mut guard = 0;
    while r.pending_kind() != Some(SelectionKind::Trash) {
        guard += 1;
        assert!(guard < 10, "OPT should be available again next turn");
        match r.pending_kind() {
            Some(SelectionKind::OppField) => pick(&mut r, 0, opp_action(oa)),
            Some(_) => accept_outer(&mut r),
            None => panic!("OPT did not reset on the next turn"),
        }
    }
    let a = trash_action(&r, 0, "TITAN-3");
    pick(&mut r, 0, a);
    finish_on_play(&mut r, oa);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN-3".to_string()));
}

#[test]
fn p_209_hand_trashed_decline_outer_plays_nothing() {
    let mut r = runner();
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let tm = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "DEMON-4");
    own_trash_via_on_play(&mut r, tm);
    let mut guard = 0;
    while r.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 10);
        match r.pending_kind() {
            Some(SelectionKind::OppField) => pick(&mut r, 0, opp_action(oa)),
            _ => r.execute_action(0, PASS).expect("decline"),
        }
    }
    assert!(!field_ids(&r, 0).contains(&"DEMON-4".to_string()));
}

#[test]
fn p_209_opponent_hand_trash_does_not_trigger() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "DEMON-4");
    push_hand(&mut r, 1, "FODDER");
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, "FODDER");
    let _ = r.auto_resolve();
    assert!(
        r.pending_selection().is_none(),
        "opponent's hand trash is not 'your hand'"
    );
    assert!(!field_ids(&r, 0).contains(&"DEMON-4".to_string()));
}
