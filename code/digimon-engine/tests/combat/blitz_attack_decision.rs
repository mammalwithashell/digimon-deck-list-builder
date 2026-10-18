//! `<Blitz>` (§16-16, Comprehensive Rules Ver. 4.3): the attack it offers is
//! its own resumable decision (`ResumeFrame::BlitzAttackDecision`), so a game
//! cloned while that decision is parked resolves it on the clone (accept and
//! decline) without touching the original — the engine stays cloneable for
//! search / training (make-engine-cloneable; CLAUDE.md rule 28).
//!
//! Board: P0 (memory 2, hand `[BLITZ]`) has `VANILLA_LV3` (red Lv.3) on the
//! field and digivolves it into `BLITZ` (cost 3, "[When Digivolving] <Blitz>",
//! the EX3-063 `grant_keyword` idiom): the memory moves to 1 on P1's side and
//! <Blitz> offers the attack before the turn can end (§16-16-1 / §16-16-3).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::game::Game;
use digimon_engine::resume::ResumeFrame;

const FILL_YAML: &str = r#"
card: VANILLA_LV3
name: Plainmon
kind: digimon
level: 3
color: [red]
cost: 2
dp: 1000
"#;

const BLITZ_YAML: &str = r#"
card: BLITZ
name: Rushdigimon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
alt_paths:
  - kind: digivolve
    from: { level_eq: 3, color_is: red }
    cost: 3
effects:
  - when: when_digivolving
    summary: "[When Digivolving] <Blitz> (If your opponent has 1 or more memory, this Digimon may attack.)"
    process:
      - grant_keyword:
          target: source
          keyword: Blitz
          expiry: end_of_turn
"#;

const FILL: &str = "VANILLA_LV3";

/// Digivolve P0's field `VANILLA_LV3` into `BLITZ`; returns the runner with
/// the <Blitz> decision parked and the offered attack on P1.
fn parked_blitz() -> (DebugRunner, u16) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(FILL_YAML)
        .unwrap()
        .from_dsl_yaml(BLITZ_YAML)
        .unwrap()
        .hand(0, &["BLITZ"])
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 3])
        .memory(2)
        .start();
    let base = r.place_on_field(0, FILL, Some(0));
    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    let attack = encode_attack(base.index as u16, SECURITY_TARGET);
    let sel = r
        .pending_selection()
        .expect("§16-16-3: <Blitz> offers its attack as a decision");
    assert_eq!(sel.selecting_player, 0);
    assert!(sel.is_optional, "\"This Digimon may attack\" is optional");
    assert_eq!(sel.valid_action_ids, vec![attack]);
    assert!(
        matches!(
            r.game.pending_selection_resume.as_ref().map(|s| &s.frames[..]),
            Some([ResumeFrame::BlitzAttackDecision(_)])
        ),
        "the decision is a resumable-VM frame, not a closure"
    );
    assert_eq!(r.game.turn_player(), 0, "the turn has not ended");
    (r, attack)
}

fn mask_bits(game: &Game) -> Vec<u16> {
    build_action_mask(game, 0)
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == 1.0)
        .map(|(i, _)| i as u16)
        .collect()
}

#[test]
fn blitz_decision_clones_and_resolves_on_the_clone_accept_and_decline() {
    let (r, attack) = parked_blitz();
    let original = r.game.clone();
    assert_eq!(mask_bits(&original), vec![PASS, attack]);

    // Accept on a clone: the attack checks P1's security, then the turn ends
    // (the memory is still on P1's side).
    let mut accept = original.clone();
    accept
        .resolve_selection(0, attack)
        .expect("the <Blitz> attack resolves on the clone");
    assert!(accept.pending_selection.is_none());
    assert_eq!(accept.players[1].security.len(), 2, "the <Blitz> attack");
    assert_eq!(accept.turn_player(), 1, "then the turn ended");

    // Decline on another clone: no attack; the turn ends.
    let mut decline = original.clone();
    decline
        .resolve_selection(0, PASS)
        .expect("declining <Blitz> resolves on the clone");
    assert!(decline.pending_selection.is_none());
    assert_eq!(decline.players[1].security.len(), 3, "declined: no attack");
    assert_eq!(decline.turn_player(), 1);

    // Neither clone touched the original: it is still parked on the same
    // decision, and resolving it there agrees with the accept clone.
    let mut again = original.clone();
    assert!(original.pending_selection.is_some());
    assert_eq!(original.players[1].security.len(), 3);
    assert_eq!(mask_bits(&again), vec![PASS, attack]);
    again.resolve_selection(0, attack).unwrap();
    assert_eq!(again.players[1].security.len(), 2);
    assert_eq!(again.turn_player(), 1);

    // The runner's own game (never cloned from) resolves too.
    let mut r = r;
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.turn_player(), 1);
}

// ─── Printed "[When Digivolving] <Blitz>" (innate keyword, no grant) ─────────
//
// §16-16-2 / §2-3-3-4: a bare printed "[When Digivolving] <Blitz>" (BT14-017,
// ST12-10, LM-039, ...) is the card's own [When Digivolving] effect; its
// processing is the same <Blitz> decision. Synthetic card: `PBLITZ` (red
// Lv.4) with the innate `Keyword::Blitz` (what `parse_printed_keywords` gives
// those printings) and no effect script; digivolves from a red Lv.3 for 3.

/// P0 (memory `memory`, hand `[PBLITZ]`) digivolves its field `PBASE` into
/// `PBLITZ`. Returns the runner and the attack on P1 by that Digimon.
fn printed_blitz_digivolved(memory: i16) -> (DebugRunner, u16) {
    let base = make_test_card("PBASE", "Printed Base");
    let mut blitz = make_test_card("PBLITZ", "Printed Blitz");
    blitz.level = Some(4);
    blitz.dp = Some(5000);
    blitz.play_cost = 5;
    blitz.keywords = vec![Keyword::Blitz];
    blitz.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 3,
    }];
    let mut r = DebugRunner::builder()
        .add_card(base)
        .add_card(blitz)
        .hand(0, &["PBLITZ"])
        .deck(0, &["PBASE"; 6])
        .deck(1, &["PBASE"; 6])
        .security(0, &["PBASE"; 3])
        .security(1, &["PBASE"; 3])
        .memory(memory)
        .start();
    let base = r.place_on_field(0, "PBASE", Some(0));
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, base.index as u16) as usize],
        1.0,
        "witness: the digivolution is legal"
    );
    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(
        r.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "PBLITZ"
    );
    (r, encode_attack(base.index as u16, SECURITY_TARGET))
}

#[test]
fn printed_when_digivolving_blitz_offers_the_attack_and_clones() {
    let (r, attack) = printed_blitz_digivolved(2);
    let sel = r
        .pending_selection()
        .expect("§16-16-1/-3: printed <Blitz> offers its attack (memory 1 on P1's side)");
    assert_eq!(sel.selecting_player, 0);
    assert!(sel.is_optional);
    assert_eq!(sel.valid_action_ids, vec![attack]);
    assert!(matches!(
        r.game.pending_selection_resume.as_ref().map(|s| &s.frames[..]),
        Some([ResumeFrame::BlitzAttackDecision(_)])
    ));
    assert_eq!(r.game.turn_player(), 0, "the turn has not ended");
    let original = r.game.clone();
    assert_eq!(mask_bits(&original), vec![PASS, attack]);

    let mut accept = original.clone();
    accept.resolve_selection(0, attack).unwrap();
    assert_eq!(accept.players[1].security.len(), 2, "the <Blitz> attack");
    assert_eq!(accept.turn_player(), 1);

    let mut decline = original.clone();
    decline.resolve_selection(0, PASS).unwrap();
    assert_eq!(decline.players[1].security.len(), 3, "declined: no attack");
    assert_eq!(decline.turn_player(), 1);

    assert!(original.pending_selection.is_some(), "the original is untouched");
}

#[test]
fn printed_when_digivolving_blitz_offers_nothing_with_memory_on_own_side() {
    // §16-16-5: memory 4 - 3 = 1 on P0's own side when <Blitz> activates.
    let (r, _) = printed_blitz_digivolved(4);
    assert!(
        r.pending_selection().is_none(),
        "no <Blitz> decision: {:?}",
        r.pending_selection().map(|s| &s.prompt)
    );
    assert_eq!(r.game.turn_player(), 0, "P0's turn continues (memory 1)");
    assert_eq!(r.game.players[1].security.len(), 3, "no attack");
}
