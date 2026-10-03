//! EX13 "Richard Sampson / DATA SQUAD" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/richard-sampson-data-squad-ex13-model.md`.
//! A yellow security-as-resource line: Kyaromon (EX13-003) → Kudamon (EX13-026)
//! → Reppamon (EX13-030) trades its own security (Reppamon's cost, <Barrier>)
//! for Kyaromon's cost-reduced digivolves toward Kentaurosmon.
//!
//! EX13-032 Chirinmon and EX13-071 Richard Sampson are BLOCKED (partial), so no
//! combo here names them (see the model's "Blocked combos"). Cross-set roles
//! are real implemented DSL cards: EX12-046 Shishimamon (the only implemented
//! Lv.5 yellow [Holy Beast]), EX13-036 Kentaurosmon (Lv.6 name target),
//! ST24-14 Yoshino & Keenan (a non-blocked [DATA SQUAD] Tamer), ST1-02 Biyomon
//! (vanilla filler) and ST2-10 Plesiomon (vanilla 12000 DP opponent). No
//! synthetic cards.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{03,26,30}.rs`;
//! this file asserts only the cross-card system facts.

#![allow(dead_code)]

use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::enums::ModifierType;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::support::{dsl_builder, snapshot};

const KYAROMON: &str = "EX13-003";
const KUDAMON: &str = "EX13-026";
const REPPAMON: &str = "EX13-030";
const KENTAUROSMON: &str = "EX13-036";
const SHISHIMAMON: &str = "EX12-046";
const DS_TAMER: &str = "ST24-14";
const BIYOMON: &str = "ST1-02";
const PLESIOMON: &str = "ST2-10";

const REAL: &[&str] = &[
    KYAROMON,
    KUDAMON,
    REPPAMON,
    KENTAUROSMON,
    SHISHIMAMON,
    DS_TAMER,
    BIYOMON,
    PLESIOMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn builder() -> DebugRunnerBuilder {
    dsl_builder(REAL)
}

fn start(b: DebugRunnerBuilder, hand: &[&str], memory: i16) -> DebugRunner {
    let mut r = b
        .hand(0, hand)
        .deck(0, &[BIYOMON; 8])
        .deck(1, &[BIYOMON; 8])
        .security(0, &[BIYOMON; 3])
        .security(1, &[BIYOMON; 3])
        .memory(memory)
        .start();
    r.skip_mulligan();
    r
}

fn hand_index(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn find_perm(r: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let i = r.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} on P{player}'s field"));
    PermanentHandle {
        player,
        index: i as u8,
    }
}

/// Full stack bottom → top, with face-down flags.
fn stack(r: &DebugRunner, h: PermanentHandle) -> Vec<(String, bool)> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| (c.card_id(&r.game.card_data).to_string(), c.face_down))
        .collect()
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    stack(r, h).into_iter().map(|(id, _)| id).collect()
}

/// Player-driven digivolve of hand card `id` onto `base`; if the engine asks
/// which (equal-cost) digivolution route to use, take the first.
fn digivolve(r: &mut DebugRunner, id: &str, base: PermanentHandle) {
    let slot = hand_index(r, id) as u16;
    r.game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
    if let Some(view) = r.pending_selection_view() {
        if let Some(choices) = &view.effect_choices {
            if choices.iter().any(|c| c.label.contains("cost")) {
                r.execute_branch(0).expect("choose digivolution route");
            }
        }
    }
}

/// Decline every optional prompt and take the first legal option of every
/// mandatory one until the queue is empty.
fn drain(r: &mut DebugRunner) {
    for _ in 0..24 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        if view.is_optional {
            r.execute_action(view.selecting_player, PASS).expect("pass");
        } else if view.kind == SelectionKind::TriggerOrder {
            // Resolve mandatory triggers first so optional ones keep their own
            // accept/decline prompt.
            let choices = view.effect_choices.clone().unwrap_or_default();
            let pick = choices
                .iter()
                .find(|c| !c.is_optional)
                .or_else(|| choices.first())
                .map(|c| c.action_id)
                .expect("trigger-order entry");
            r.execute_action(view.selecting_player, pick).expect("order");
        } else if view.effect_choices.is_some() {
            // Take the LAST label for menus ("Don't …" options sit last).
            let last = view.valid_action_ids.len() - 1;
            r.execute_action(view.selecting_player, view.valid_action_ids[last])
                .expect("last choice");
        } else {
            r.execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("first");
        }
    }
    panic!("drain did not converge: {:?}", r.pending_selection_view());
}

/// Pass prompts until a pending selection of `kind` appears; returns whether it
/// did. Non-matching optional prompts are declined.
fn pass_until(r: &mut DebugRunner, kind: SelectionKind) -> bool {
    for _ in 0..12 {
        let Some(view) = r.pending_selection_view() else {
            return false;
        };
        if view.kind == kind {
            return true;
        }
        if view.is_optional {
            r.execute_action(view.selecting_player, PASS).expect("pass");
        } else {
            r.execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("first");
        }
    }
    false
}

/// Put a [Kyaromon, Kudamon] stack in player 0's breeding area.
fn breeding_kudamon_on_kyaromon(r: &mut DebugRunner) {
    let h = r.place_stack(0, &[KYAROMON, KUDAMON]);
    let perm = r.game.players[0].battle_area.remove(h.index as usize);
    r.game.players[0].breeding_area = Some(perm);
}

/// Stack the top of player 0's deck so `top[0]` is revealed first.
fn stack_deck_top(r: &mut DebugRunner, top: &[&str]) {
    // Deck index 0 is the bottom (see ex13_026 per-card fixture), so the
    // revealed top is the END of the vector.
    let deck = &mut r.game.players[0].deck;
    let mut extra = Vec::new();
    for id in top {
        let cs = deck
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == *id)
            .map(|i| deck.remove(i));
        extra.push(cs.unwrap_or_else(|| panic!("{id} seeded in deck")));
    }
    for cs in extra.into_iter().rev() {
        deck.push(cs);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// C1 — Kudamon breeding move feeds a [DATA SQUAD] Tamer
// ═══════════════════════════════════════════════════════════════════════════

fn c1_runner(with_tamer: bool) -> DebugRunner {
    let mut r = builder()
        .deck(0, &[BIYOMON, BIYOMON, BIYOMON, BIYOMON, BIYOMON, REPPAMON, KENTAUROSMON, BIYOMON])
        .deck(1, &[BIYOMON; 8])
        .security(1, &[BIYOMON; 3])
        .memory(3)
        .start();
    r.skip_mulligan();
    // Reveal order (top first): Reppamon, Kentaurosmon, Biyomon.
    stack_deck_top(&mut r, &[REPPAMON, KENTAUROSMON, BIYOMON]);
    if with_tamer {
        r.place_on_field(0, DS_TAMER, Some(0));
    }
    breeding_kudamon_on_kyaromon(&mut r);
    r
}

/// Combo C1 (model §C1). Cards: EX13-003 Kyaromon, EX13-026 Kudamon, ST24-14
/// ([DATA SQUAD] Tamer); reveal hits EX13-030 Reppamon ([Holy Beast]) +
/// EX13-036 Kentaurosmon ([Royal Knight]); miss ST1-02 Biyomon.
///
/// Expected: moving Kudamon (digivolved on Kyaromon) out of breeding fires its
/// [When Moving] reveal-3; one eligible card goes to hand, the other face down
/// at the BOTTOM of the DS Tamer (Official Q&A), Biyomon to deck bottom. The
/// stack lands in the battle area intact (Kyaromon beneath Kudamon).
#[test]
fn c1_kudamon_breeding_move_adds_one_and_stashes_one_under_ds_tamer() {
    let mut r = c1_runner(true);
    let before = snapshot(&r);
    assert!(r.move_from_breeding(0), "move from breeding");
    assert!(r.pending_selection().is_some(), "[When Moving] reveal fires");
    r.auto_resolve().expect("resolve reveal buckets + tamer pick");

    let kuda = find_perm(&r, 0, KUDAMON);
    assert_eq!(stack_ids(&r, kuda), vec![KYAROMON, KUDAMON], "stack moved intact");

    let tamer = find_perm(&r, 0, DS_TAMER);
    let tstack = stack(&r, tamer);
    assert_eq!(tstack.len(), 2, "one card placed under the Tamer");
    assert!(tstack[0].1, "placed face down");
    let hand = hand_ids(&r);
    assert_eq!(hand.len(), 1, "one card added to hand");
    let mut moved = vec![hand[0].clone(), tstack[0].0.clone()];
    moved.sort();
    assert_eq!(moved, vec![REPPAMON.to_string(), KENTAUROSMON.to_string()]);
    assert_eq!(
        r.game.players[0].deck[0].card_id(&r.game.card_data),
        BIYOMON,
        "the ineligible card returns to the bottom"
    );
    assert_eq!(r.deck_size(0), before.deck[0] - 2, "net −2 deck (1 hand, 1 stash)");
    assert!(r.pending_selection().is_none());
}

/// Combo C1 unhappy path: with no [DATA SQUAD] Tamer the stash half has no
/// destination — only the add-to-hand half resolves, and BOTH unpicked cards
/// return to the deck bottom.
#[test]
fn c1_without_ds_tamer_only_adds_to_hand() {
    let mut r = c1_runner(false);
    let before = snapshot(&r);
    assert!(r.move_from_breeding(0));
    r.auto_resolve().expect("resolve");
    assert_eq!(hand_ids(&r).len(), 1, "one card added");
    assert_eq!(r.deck_size(0), before.deck[0] - 1, "the other two go back to the deck");
    assert_eq!(r.battle_area_size(0), 1, "only Kudamon's stack on the field");
}

// ═══════════════════════════════════════════════════════════════════════════
// C2 — Reppamon [When Digivolving] security trash wakes Kyaromon
// ═══════════════════════════════════════════════════════════════════════════

/// Combo C2 (model §C2). Cards: EX13-003 Kyaromon, EX13-026 Kudamon, EX13-030
/// Reppamon, EX12-046 Shishimamon.
///
/// Expected: Reppamon digivolves onto Kudamon/Kyaromon (cost 2). Its [When
/// Digivolving] cost is paid with no [Richard Sampson] available (Official Q&A
/// "Yes, you can."; rule 15-7-4) → 1 security trashed → Kyaromon's inherited
/// "[Your Turn] when your security stack is removed from" fires → the stack
/// digivolves into Shishimamon from hand at 4 − 1 = 3. Net memory −5,
/// security −1, stack Kyaromon/Kudamon/Reppamon/Shishimamon.
#[test]
fn c2_reppamon_security_cost_wakes_kyaromon_into_holy_beast() {
    let mut r = start(builder(), &[REPPAMON, SHISHIMAMON], 10);
    let base = r.place_stack(0, &[KYAROMON, KUDAMON]);
    let before = snapshot(&r);

    digivolve(&mut r, REPPAMON, base);
    assert!(r.pending_is_optional(), "Reppamon [WD] offers its security-trash cost");
    r.accept_optional_trigger().expect("pay the cost");

    assert!(
        pass_until(&mut r, SelectionKind::Hand),
        "Kyaromon's hand pick surfaces after the security trash: {:?}",
        r.pending_selection_view()
    );
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 1, "only Shishimamon is eligible");
    r.execute_action(0, view.valid_action_ids[0]).expect("pick Shishimamon");
    drain(&mut r);

    let top = find_perm(&r, 0, SHISHIMAMON);
    assert_eq!(
        stack_ids(&r, top),
        vec![KYAROMON, KUDAMON, REPPAMON, SHISHIMAMON]
    );
    assert_eq!(r.security_count(0), before.security[0] - 1, "Reppamon's cost");
    assert_eq!(
        r.memory(),
        before.memory - 2 - 3,
        "Reppamon 2 + Shishimamon (4 − 1 Kyaromon) 3"
    );
}

/// Combo C2 unhappy path: declining Reppamon's optional cost removes no
/// security, so Kyaromon never wakes — Shishimamon stays in hand.
#[test]
fn c2_declining_reppamon_cost_leaves_kyaromon_asleep() {
    let mut r = start(builder(), &[REPPAMON, SHISHIMAMON], 10);
    let base = r.place_stack(0, &[KYAROMON, KUDAMON]);
    let before = snapshot(&r);
    digivolve(&mut r, REPPAMON, base);
    r.decline_optional_trigger().expect("decline cost");
    assert!(r.pending_selection().is_none(), "no Kyaromon prompt");
    let top = find_perm(&r, 0, REPPAMON);
    assert_eq!(stack_ids(&r, top), vec![KYAROMON, KUDAMON, REPPAMON]);
    assert_eq!(r.security_count(0), before.security[0]);
    assert_eq!(r.memory(), before.memory - 2);
    assert_eq!(
        r.hand_size(0),
        before.hand[0],
        "Reppamon left the hand, the digivolve bonus draw replaced it"
    );
    assert!(hand_ids(&r).contains(&SHISHIMAMON.to_string()), "Shishimamon stays in hand");
}

// ═══════════════════════════════════════════════════════════════════════════
// C3 — Reppamon shared [Once Per Turn] + Kudamon inherited on attack
// ═══════════════════════════════════════════════════════════════════════════

/// Combo C3 (model §C3). Cards: EX13-026 Kudamon, EX13-030 Reppamon (+ ST1-02
/// Biyomon as the opponent's Digimon).
///
/// Expected: Reppamon's [On Play]/[When Digivolving]/[When Attacking] is ONE
/// [Once Per Turn] clause — after paying it on digivolve, the same turn's
/// attack offers no second security-trash cost; Kudamon's inherited [When
/// Attacking] still gives the opponent's Digimon <Security A. −1>. Security
/// drops only once.
#[test]
fn c3_reppamon_opt_spent_on_digivolve_but_kudamon_inherited_fires_on_attack() {
    let mut r = start(builder(), &[REPPAMON], 10);
    let base = r.place_stack(0, &[KUDAMON]);
    let opp = r.place_on_field(1, BIYOMON, Some(0));
    let before = snapshot(&r);

    digivolve(&mut r, REPPAMON, base);
    r.accept_optional_trigger().expect("pay WD cost");
    drain(&mut r);
    assert_eq!(r.security_count(0), before.security[0] - 1);

    let rep = find_perm(&r, 0, REPPAMON);
    r.game.players[0].battle_area[rep.index as usize].turn_played = 0;
    r.attack_player(rep, 1, false);
    // The engine still lists the OPT-spent Reppamon [WA] in the TriggerOrder
    // menu (OPT is checked at resolution, not queue time — see
    // G-ENGINE-OPT-SPENT-TRIGGER-IN-TRIGGER-ORDER). Order it FIRST, the
    // worst case: it must resolve as a no-op (no cost prompt, no trash).
    if r.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let view = r.pending_selection_view().unwrap();
        let rep_entry = view
            .effect_choices
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.label.starts_with(REPPAMON))
            .map(|c| c.action_id)
            .expect("Reppamon entry");
        r.execute_action(0, rep_entry).expect("order Reppamon first");
    }
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "no Reppamon cost prompt — straight to Kudamon's inherited pick: {:?}",
        r.pending_selection_view()
    );
    assert_eq!(r.security_count(0), before.security[0] - 1, "OPT spent: nothing trashed");
    r.auto_resolve().expect("pick Biyomon");
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "Kudamon inherited <Security A. −1>"
    );
    assert_eq!(
        r.security_count(0),
        before.security[0] - 1,
        "no second Reppamon security trash this turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// C4 — Inherited <Barrier> self-trash wakes Kyaromon into Kentaurosmon
// ═══════════════════════════════════════════════════════════════════════════

fn c4_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = start(builder(), &[KENTAUROSMON], 10);
    let ours = r.place_stack(0, &[KYAROMON, KUDAMON, REPPAMON, SHISHIMAMON]);
    let big = r.place_on_field(1, PLESIOMON, Some(0));
    (r, ours, big)
}

/// Combo C4 (model §C4). Cards: EX13-003 Kyaromon, EX13-026 Kudamon, EX13-030
/// Reppamon (inherited <Barrier>), EX12-046 Shishimamon (carrier), EX13-036
/// Kentaurosmon (name target); ST2-10 Plesiomon (12000) as the wall.
///
/// Expected: Shishimamon (7000) attacks Plesiomon on our turn and loses; the
/// inherited <Barrier> trashes our top security instead of deletion (§16);
/// that removal fires Kyaromon's [Your Turn] trigger → digivolve into
/// Kentaurosmon at 3 − 1 = 2. Net: memory −2, security −1, Shishimamon kept
/// under Kentaurosmon.
#[test]
fn c4_inherited_barrier_save_wakes_kyaromon_into_kentaurosmon() {
    let (mut r, ours, big) = c4_runner();
    r.game.players[0].battle_area[ours.index as usize].turn_played = 0;
    r.game.players[1].battle_area[big.index as usize].is_suspended = true;
    let before = snapshot(&r);

    r.attack_digimon(ours, big, false);
    assert!(
        pass_until(&mut r, SelectionKind::Replacement),
        "inherited <Barrier> offers the save: {:?}",
        r.pending_selection_view()
    );
    r.accept_optional_trigger().expect("accept Barrier");
    assert!(
        pass_until(&mut r, SelectionKind::Hand),
        "Kyaromon's hand pick after the Barrier trash: {:?}",
        r.pending_selection_view()
    );
    let view = r.pending_selection_view().unwrap();
    r.execute_action(0, view.valid_action_ids[0]).expect("pick Kentaurosmon");
    drain(&mut r);

    let top = find_perm(&r, 0, KENTAUROSMON);
    assert_eq!(
        stack_ids(&r, top),
        vec![KYAROMON, KUDAMON, REPPAMON, SHISHIMAMON, KENTAUROSMON]
    );
    assert_eq!(r.security_count(0), before.security[0] - 1, "Barrier trashed 1");
    assert_eq!(r.memory(), before.memory - 2, "Kentaurosmon 3 − 1");
}

/// Combo C4 unhappy path: the same <Barrier> save on the OPPONENT's turn still
/// trashes security, but Kyaromon's [Your Turn] gate keeps it asleep —
/// Kentaurosmon stays in hand.
#[test]
fn c4_barrier_save_on_opponents_turn_does_not_wake_kyaromon() {
    let (mut r, ours, big) = c4_runner();
    r.end_turn();
    drain(&mut r);
    assert_eq!(r.turn_player(), 1);
    r.game.players[0].battle_area[ours.index as usize].is_suspended = true;
    r.game.players[1].battle_area[big.index as usize].turn_played = 0;
    r.game.players[1].battle_area[big.index as usize].is_suspended = false;
    let before = snapshot(&r);

    r.attack_digimon(big, ours, false);
    assert!(
        pass_until(&mut r, SelectionKind::Replacement),
        "inherited <Barrier> offers the save: {:?}",
        r.pending_selection_view()
    );
    r.accept_optional_trigger().expect("accept Barrier");
    assert!(
        !pass_until(&mut r, SelectionKind::Hand),
        "[Your Turn]: Kyaromon does not fire on the opponent's turn"
    );
    drain(&mut r);
    let top = find_perm(&r, 0, SHISHIMAMON);
    assert_eq!(stack_ids(&r, top).len(), 4, "no digivolve");
    assert_eq!(r.security_count(0), before.security[0] - 1);
    assert!(hand_ids(&r).contains(&KENTAUROSMON.to_string()));
}
