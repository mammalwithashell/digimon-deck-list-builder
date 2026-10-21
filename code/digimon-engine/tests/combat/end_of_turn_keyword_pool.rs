//! The end-of-turn attack keywords declare their attack inside the
//! `[End of Your Turn]` pool (Comprehensive Rules Ver. 4.3 §16-44-2 <Engage>,
//! §16-38-2 <Execute>, §16-33-2 <Vortex>, §16-34-2 <Overclock>;
//! G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS).
//!
//! The rule's own ordering (declaration < another `[End of Your Turn]`
//! effect < counter timing, §11-1-4 + Q3868 / Q3869) is pinned per keyword in
//! `rules_conformance::ch16_{new,attack}_keywords::*_kind_matches_rule`. These
//! cover the engine shape around it:
//!   - only one attack is in progress at a time: a second keyword whose entry
//!     activates while the first keyword's attack waits for its counter
//!     timing offers its own attack after that attack has ended, in
//!     activation order (never nested);
//!   - the parked declaration is plain data (`ResumeFrame::AttackDeclarationDecision`):
//!     a clone of the game parked on it inside the pool resolves accept and
//!     decline independently of the original (`Game: Clone` powers search;
//!     CLAUDE.md rule 28).

use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::resume::ResumeFrame;
use digimon_engine::selection::SelectionKind;

const ENGAGE_YAML: &str = r#"
card: ENGAGE
name: Engagemon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
effects:
  - kind: grant_keyword
    keyword: Engage
    summary: "<Engage> (At the end of your turn, this Digimon may attack.)"
"#;

const EOT_MEM_YAML: &str = r#"
card: EOT_MEM
name: Dusk Tamer
kind: tamer
color: [red]
cost: 3
effects:
  - when: end_of_your_turn
    summary: "[End of Your Turn] Gain 2 memory."
    process:
      - gain_memory: 2
"#;

fn board(memory: i16) -> DebugRunner {
    let mut fill = make_test_card("FILL", "FILL");
    fill.dp = Some(1000);
    fill.level = Some(3);
    let mut big = make_test_card("BIG", "BIG");
    big.dp = Some(6000);
    big.level = Some(5);
    DebugRunner::builder()
        .from_dsl_yaml(ENGAGE_YAML)
        .expect("ENGAGE loads")
        .from_dsl_yaml(EOT_MEM_YAML)
        .expect("EOT_MEM loads")
        .add_card(fill)
        .add_card(big)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(memory)
        .start()
}

/// Resolve the pending `TriggerOrder` by picking the first branch whose
/// label starts with `card_id`.
fn choose(r: &mut DebugRunner, card_id: &str) {
    let (player, action) = {
        let sel = r.pending_selection().expect("a TriggerOrder prompt");
        assert_eq!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
        let choice = sel
            .effect_choices
            .as_ref()
            .expect("branches")
            .iter()
            .find(|c| c.label.starts_with(card_id))
            .unwrap_or_else(|| panic!("no branch for {card_id}: {sel:?}"));
        (sel.selecting_player, choice.action_id)
    };
    r.execute_action(player, action).expect("TriggerOrder pick");
}

/// The attacker whose end-of-turn attack on P1 the pending decision offers.
fn offered_attacker(r: &DebugRunner, candidates: &[PermanentHandle]) -> Option<PermanentHandle> {
    let sel = r.pending_selection()?;
    if sel.selecting_player != 0 || !sel.is_optional {
        return None;
    }
    candidates.iter().copied().find(|h| {
        sel.valid_action_ids
            .contains(&encode_attack(h.index as u16, SECURITY_TARGET))
    })
}

fn parked_on_attack_declaration(game: &Game) -> bool {
    matches!(
        game.pending_selection_resume
            .as_ref()
            .map(|stack| stack.frames.as_slice()),
        Some([ResumeFrame::AttackDeclarationDecision(_)])
    )
}

/// P0 has two `ENGAGE` (<Engage>). At the end of P0's turn both pend in one
/// `TriggerOrder` (§15-4-3-5-1: the turn player orders them). P0 activates
/// one: it declares its attack on P1. The other <Engage>'s entry activates
/// before that attack's counter timing (§11-1-4), but only one attack is in
/// progress at a time, so its offer waits: nothing else is offered until the
/// first attack has checked security and ended. Then the second <Engage>
/// offers its attack; it attacks; then the turn passes.
#[test]
fn two_engage_attacks_resolve_one_at_a_time_in_activation_order() {
    let mut r = board(3);
    let a = r.place_on_field(0, "ENGAGE", Some(0));
    let b = r.place_on_field(0, "ENGAGE", Some(0));
    r.end_turn();

    choose(&mut r, "ENGAGE");
    let first = offered_attacker(&r, &[a, b]).expect("one <Engage> offers its attack");
    let second = if first == a { b } else { a };
    assert!(
        offered_attacker(&r, &[second]).is_none(),
        "one attack decision at a time"
    );
    r.execute_action(0, encode_attack(first.index as u16, SECURITY_TARGET))
        .expect("declare the first attack");

    // The first attack has run to its end (one security check); the second
    // <Engage>'s offer came only after it.
    assert!(r.game.pending_attack.is_none(), "the first attack ended");
    assert_eq!(r.security_count(1), 2, "the first attack checked security");
    assert_eq!(r.turn_player(), 0, "the end of turn is still resolving");
    assert_eq!(
        offered_attacker(&r, &[a, b]),
        Some(second),
        "then the other <Engage> offers its attack"
    );
    assert!(parked_on_attack_declaration(&r.game));

    r.execute_action(0, encode_attack(second.index as u16, SECURITY_TARGET))
        .expect("declare the second attack");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), 1, "the second attack checked security");
    assert_eq!(r.turn_player(), 1, "then the turn passed");
}

/// As above, but P0 declines the second <Engage> when it is offered: no
/// second attack, and the turn passes.
#[test]
fn waiting_engage_offer_can_be_declined() {
    let mut r = board(3);
    let a = r.place_on_field(0, "ENGAGE", Some(0));
    let b = r.place_on_field(0, "ENGAGE", Some(0));
    r.end_turn();
    choose(&mut r, "ENGAGE");
    let first = offered_attacker(&r, &[a, b]).expect("one <Engage> offers its attack");
    let second = if first == a { b } else { a };
    r.execute_action(0, encode_attack(first.index as u16, SECURITY_TARGET))
        .expect("declare the first attack");
    assert_eq!(offered_attacker(&r, &[a, b]), Some(second));
    r.decline_optional_trigger().expect("decline the second <Engage>");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), 2, "only the first attack");
    assert_eq!(r.turn_player(), 1, "the turn passed");
}

/// P0 has `ENGAGE` and a Digimon with <Overclock> (raw keyword grant: any
/// other Digimon is a legal deletion) plus a third Digimon to delete. P0
/// activates <Engage> first and attacks; <Overclock>'s entry activates while
/// that attack waits for its counter timing, so its condition is offered only
/// after the <Engage> attack has ended. P0 deletes the third Digimon; the
/// <Overclock> Digimon attacks P1 without suspending; the turn passes.
#[test]
fn overclock_offer_waits_for_the_engage_attack() {
    let mut r = board(3);
    let engage = r.place_on_field(0, "ENGAGE", Some(0));
    let oc = r.place_on_field(0, "BIG", Some(0));
    let sac = r.place_on_field(0, "FILL", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, 0);
    r.end_turn();

    choose(&mut r, "ENGAGE");
    assert_eq!(offered_attacker(&r, &[engage]), Some(engage));
    r.execute_action(0, encode_attack(engage.index as u16, SECURITY_TARGET))
        .expect("declare the <Engage> attack");

    assert!(r.game.pending_attack.is_none(), "the <Engage> attack ended");
    assert_eq!(r.security_count(1), 2);
    let sel = r.pending_selection().expect("then <Overclock>'s condition");
    assert_eq!(sel.kind, SelectionKind::OwnField, "{sel:?}");
    assert!(sel.is_optional);
    let pick = encode_attack(0, sac.index as u16);
    assert!(sel.valid_action_ids.contains(&pick), "{sel:?}");
    r.execute_action(0, pick).expect("delete the third Digimon");

    assert_eq!(r.security_count(1), 1, "the <Overclock> attack checked security");
    assert!(
        !r.game.player(0).battle_area[oc.index as usize].is_suspended,
        "without suspending"
    );
    assert_eq!(r.turn_player(), 1, "then the turn passed");
}

/// P0 has `ENGAGE` and `EOT_MEM` ("[End of Your Turn] Gain 2 memory"). At the
/// end of P0's turn P0 activates <Engage> first: its attack declaration is
/// parked inside the pool with `EOT_MEM` still pending. Clone the game there:
/// accepting on the clone declares the attack, `EOT_MEM` resolves, the attack
/// checks security and the turn passes; the original is untouched, and
/// declining on it resolves `EOT_MEM` with no attack.
#[test]
fn clone_while_engage_declaration_is_parked_in_the_pool() {
    let mut r = board(3);
    let engage = r.place_on_field(0, "ENGAGE", Some(0));
    r.place_on_field(0, "EOT_MEM", Some(0));
    r.end_turn();
    choose(&mut r, "ENGAGE");
    assert_eq!(offered_attacker(&r, &[engage]), Some(engage));
    assert!(
        parked_on_attack_declaration(&r.game),
        "the declaration is a data frame, not a closure"
    );
    assert_eq!(r.game.memory, 3, "EOT_MEM is still pending");
    assert_eq!(r.game.effect_queue.len(), 1, "EOT_MEM waits in the pool");

    // Accept on the clone.
    let mut other = r.game.clone();
    std::mem::swap(&mut r.game, &mut other);
    r.execute_action(0, encode_attack(engage.index as u16, SECURITY_TARGET))
        .expect("accept on the clone");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), 2, "the clone attacked");
    assert_eq!(r.turn_player(), 1, "the clone's turn passed");
    assert_eq!(r.game.memory, -5, "EOT_MEM resolved: 3 + 2, seen from P1");
    std::mem::swap(&mut r.game, &mut other);

    // The original is still parked, untouched.
    assert!(parked_on_attack_declaration(&r.game));
    assert_eq!(r.security_count(1), 3);
    assert_eq!(r.game.memory, 3);
    assert_eq!(r.turn_player(), 0);

    // Decline on the original.
    r.decline_optional_trigger().expect("decline on the original");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), 3, "declined: no attack");
    assert_eq!(r.turn_player(), 1, "the original's turn passed");
    assert_eq!(r.game.memory, -5, "EOT_MEM resolved: 3 + 2, seen from P1");
}
