//! ST21-03 Ikkakumon — Digimon, Lv.4, Blue, DP 4000, Cost 4.
//! Traits: Sea Beast / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/ST21-03.md — official Bandai DB, verbatim)
//!
//! ```text
//! Digivolve: Blue Lv.3 / cost 2
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//! [Security] At the end of the battle, play this card without paying the cost.
//! [On Play] [When Digivolving] Trash the top 2 digivolution cards of 1 of your
//! opponent's Digimon. Then, 1 of their Digimon with no digivolution cards
//! can't attack or block until their turn ends.
//! Inherited Effect
//! [When Attacking] [Once Per Turn] If you have 7 or fewer cards in your hand,
//! <Draw 1>. (Draw 1 card from your deck.)
//!
//! Official Q&A: the second half "affects 1 of your opponent's Digimon with no
//! digivolution cards ... Even if it gains a digivolution card after being
//! affected by the effect, the effect will continue processing."
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST21/Blue/ST21_03.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - "[Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2" alt-path (ST20-10 exemplar)
//! - `[Security]` after-battle self-play (`on_security` + `play_from_security`)
//! - Shared [On Play]/[When Digivolving] body (BT16-021 Togemogumon twin):
//!   mandatory opp pick → top-2 source strip → SECOND gated mandatory pick
//!   evaluated AFTER the strip → CannotAttack + CannotBlock until the opponent's
//!   turn ends
//! - Inherited [When Attacking] [Once Per Turn] with a hand-size gate
//!   (BT18-037 Lobomon idiom) — boundary 7 vs 8 and OPT lockout/reset
//! - Event-log cost test (digivolve cost 2 → MemoryChange -2)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-03";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn dig(id: &str, color: CardColor, level: u8, dp: i32, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-03 in embedded DSL pack")
        .add_card(dig("OPP", CardColor::Red, 4, 5000, false))
        .add_card(dig("SRC1", CardColor::Red, 3, 3000, false))
        .add_card(dig("SRC2", CardColor::Red, 3, 3000, false))
        .add_card(dig("SRC3", CardColor::Red, 3, 3000, false))
        .add_card(dig("BLUE3", CardColor::Blue, 3, 3000, false))
        .add_card(dig("RED3-ADV", CardColor::Red, 3, 3000, true))
        .add_card(dig("RED3-PLAIN", CardColor::Red, 3, 3000, false))
        .add_card(dig("RED2-ADV", CardColor::Red, 2, 1000, true))
        .add_card(dig("BIG-HOST", CardColor::Red, 4, 9000, false))
        .add_card(dig("SEC", CardColor::Red, 3, 1000, false))
        .add_card(dig("ATTACKER", CardColor::Red, 3, 2000, false))
        .add_card(dig("BYSTANDER", CardColor::Red, 3, 2000, false))
        .add_card(make_test_card("FILLER", "Filler"))
}

/// Play-flow runner: both decks and security stocked, plenty of memory.
fn runner() -> DebugRunner {
    let mut r = base_builder()
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC", "SEC"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

/// Attack-flow runner with `hand` cards in P0's hand and a deep deck.
fn attack_runner(hand: usize) -> DebugRunner {
    let mut r = base_builder()
        .hand(0, &vec!["FILLER"; hand])
        .deck(0, &["FILLER"; 15])
        .deck(1, &["FILLER"; 15])
        .security(1, &["SEC"; 8])
        .memory(10)
        .start();
    r.game.turn_count = 3;
    r
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn opp_pick(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("selection pending");
    let a = opp_pick(h);
    assert!(
        v.valid_action_ids.contains(&a),
        "{h:?} not a legal pick: {:?}",
        v.valid_action_ids
    );
    r.execute_action(v.selecting_player, a).unwrap();
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let cards = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    cards[..cards.len() - 1]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_has(r: &DebugRunner, player: u8, id: &str) -> bool {
    r.game.players[player as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn locked(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.modifiers().has(h, ModifierType::CannotAttack)
        && r.modifiers().has(h, ModifierType::CannotBlock)
}

/// Add Ikkakumon to P0's hand and play it onto the field (fires [On Play]).
fn play_ikka(r: &mut DebugRunner) -> PermanentHandle {
    let hi = r.add_to_hand(0, CARD_ID);
    let idx = r.play(0, hi).expect("Ikkakumon plays from hand");
    PermanentHandle {
        player: 0,
        index: idx as u8,
    }
}

/// Digivolve Ikkakumon from hand onto `base` (placed on P0's field).
/// Returns (digivolve succeeded, memory paid).
fn digivolve_from(base: &str) -> (bool, i16) {
    let mut r = runner();
    let h = r.place_on_field(0, base, Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand);
    (ok, before - r.memory())
}

fn triggered<'a>(
    card: &'a digimon_dsl::compiled::CompiledCard,
    f: impl Fn(&CompiledTriggeredClause) -> bool,
) -> Option<&'a CompiledTriggeredClause> {
    card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if f(t) => Some(t),
        _ => None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_03_identity_matches_printed_card() {
    let c = compiled(CARD_ID);
    assert_eq!(c.name, "Ikkakumon");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.level, Some(4));
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.dp, Some(4000));
    assert_eq!(c.color, vec![CompiledColor::Blue]);
    assert!(c.traits.iter().any(|t| t == "Sea Beast"), "{:?}", c.traits);
    assert!(c.traits.iter().any(|t| t == "ADVENTURE"), "{:?}", c.traits);
}

#[test]
fn st21_03_has_two_cost_two_requirement_respecting_digivolve_paths() {
    let c = compiled(CARD_ID);
    let paths: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(paths.len(), 2, "printed blue Lv.3 circle + ADVENTURE box");
    for p in &paths {
        assert_eq!(p.cost, Some(CompiledCost::Literal(2)));
        assert!(!p.ignore_requirements);
    }
    let std_from = paths[0].from.as_ref().expect("blue Lv.3 `from:`");
    assert_eq!(std_from.level_eq, Some(3));
    assert_eq!(std_from.color_is, Some(CompiledColor::Blue));
    let adv_from = paths[1].from.as_ref().expect("ADVENTURE Lv.3 `from:`");
    assert_eq!(adv_from.level_eq, Some(3));
    assert!(format!("{adv_from:?}").contains("ADVENTURE"));
}

#[test]
fn st21_03_clause_shapes() {
    let c = compiled(CARD_ID);
    let sec =
        triggered(&c, |t| t.when.contains(&CompiledTiming::OnSecurity)).expect("[Security] clause");
    assert!(!sec.optional, "security self-play is mandatory");

    let shared = triggered(&c, |t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] clause");
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert_eq!(shared.scope, CompiledScope::FaceUp);
    assert!(!shared.optional, "the strip/freeze body is mandatory");
    assert!(
        !shared.once_per_turn,
        "no [Once Per Turn] on the shared body"
    );

    let inh = triggered(&c, |t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("inherited [When Attacking] clause");
    assert_eq!(inh.scope, CompiledScope::Inherited);
    assert!(inh.once_per_turn, "[Once Per Turn]");
    assert!(!inh.optional, "<Draw 1> is not a 'you may'");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve conditions + cost event
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_03_digivolves_from_blue_lv3_for_two() {
    let (ok, paid) = digivolve_from("BLUE3");
    assert!(ok, "a blue Lv.3 satisfies the printed circle");
    assert_eq!(paid, 2);
}

#[test]
fn st21_03_digivolves_from_non_blue_lv3_with_adventure_for_two() {
    let (ok, paid) = digivolve_from("RED3-ADV");
    assert!(ok, "[Digivolve] Lv.3 w/[ADVENTURE] lifts the colour gate");
    assert_eq!(paid, 2);
}

#[test]
fn st21_03_cannot_digivolve_from_non_blue_lv3_without_adventure() {
    let (ok, _) = digivolve_from("RED3-PLAIN");
    assert!(!ok);
}

#[test]
fn st21_03_cannot_digivolve_from_lv2_adventure() {
    let (ok, _) = digivolve_from("RED2-ADV");
    assert!(
        !ok,
        "both circles need Lv.3 — a Lv.2 [ADVENTURE] is the wrong level"
    );
}

#[test]
fn st21_03_digivolve_cost_event_is_two_memory() {
    let mut r = runner();
    let h = r.place_on_field(0, "RED3-ADV", Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let cp = r.event_checkpoint();
    assert!(r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand));
    let paid_two = r
        .events_since(cp)
        .iter()
        .any(|e| matches!(e, GameEvent::MemoryChange { delta: -2, .. }));
    assert!(
        paid_two,
        "the digivolution cost of 2 must hit the event log"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play]: strip top 2, then freeze a stripped-bare Digimon
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_03_on_play_strips_top_two_then_locks_a_digimon_with_no_sources() {
    let mut r = runner();
    let a = r.place_stack(1, &["SRC1", "SRC2", "SRC3", "OPP"]); // 3 sources
    let b = r.place_on_field(1, "OPP", Some(0)); // no sources
    play_ikka(&mut r);

    let v = r.pending_selection_view().expect("strip pick");
    assert_eq!(v.selecting_player, 0, "Ikkakumon's controller chooses");
    assert!(!v.is_optional, "the strip pick is mandatory");
    assert!(v.valid_action_ids.contains(&opp_pick(a)));
    assert!(
        v.valid_action_ids.contains(&opp_pick(b)),
        "DCGO: ANY opponent Digimon is a legal strip target (no sources required)"
    );
    pick(&mut r, a);
    assert_eq!(
        source_ids(&r, a),
        vec!["SRC1".to_string()],
        "the TOP 2 digivolution cards (SRC3, SRC2) are trashed; the bottom one stays"
    );
    assert!(trash_has(&r, 1, "SRC3"));
    assert!(trash_has(&r, 1, "SRC2"));
    assert!(!trash_has(&r, 1, "SRC1"));

    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional, "the lock pick is mandatory");
    assert!(
        v.valid_action_ids.contains(&opp_pick(b)),
        "B has no digivolution cards"
    );
    assert!(
        !v.valid_action_ids.contains(&opp_pick(a)),
        "A still has a digivolution card"
    );
    pick(&mut r, b);
    r.auto_resolve().unwrap();
    assert!(locked(&r, b), "B can't attack or block");
    assert!(!locked(&r, a));
}

#[test]
fn st21_03_stripped_target_becomes_lock_eligible() {
    // Q&A / DCGO: the lock candidate set is evaluated AFTER the strip.
    let mut r = runner();
    let a = r.place_stack(1, &["SRC1", "SRC2", "OPP"]); // exactly 2 sources
    play_ikka(&mut r);
    pick(&mut r, a);
    assert!(source_ids(&r, a).is_empty(), "both sources trashed");
    let v = r.pending_selection_view().expect("lock pick after strip");
    assert!(v.valid_action_ids.contains(&opp_pick(a)));
    pick(&mut r, a);
    r.auto_resolve().unwrap();
    assert!(locked(&r, a));
}

#[test]
fn st21_03_strip_takes_only_what_is_available() {
    let mut r = runner();
    let a = r.place_stack(1, &["SRC1", "OPP"]); // only 1 source
    play_ikka(&mut r);
    pick(&mut r, a);
    assert!(source_ids(&r, a).is_empty(), "the single source is trashed");
    assert!(trash_has(&r, 1, "SRC1"));
    // Stripped bare ⇒ it is now the lock candidate.
    pick(&mut r, a);
    r.auto_resolve().unwrap();
    assert!(locked(&r, a));
}

#[test]
fn st21_03_no_lock_target_resolves_after_strip() {
    let mut r = runner();
    let a = r.place_stack(1, &["SRC1", "SRC2", "SRC3", "OPP"]);
    play_ikka(&mut r);
    pick(&mut r, a);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, a).len(), 1, "one source left");
    assert!(!locked(&r, a), "no Digimon without sources ⇒ no lock");
    assert!(r.pending_selection().is_none());
}

#[test]
fn st21_03_nothing_to_target_when_opponent_has_no_digimon() {
    let mut r = runner();
    play_ikka(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "no opponent Digimon ⇒ both halves resolve silently"
    );
}

#[test]
fn st21_03_lock_persists_through_opponents_turn_then_expires() {
    let mut r = runner();
    let b = r.place_on_field(1, "OPP", Some(0));
    play_ikka(&mut r);
    pick(&mut r, b); // strip (no sources: no-op)
    pick(&mut r, b); // lock
    r.auto_resolve().unwrap();
    assert!(locked(&r, b));

    r.end_turn(); // → opponent's turn
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert!(locked(&r, b), "still locked during their turn");

    r.end_turn(); // end of their turn
    let _ = r.auto_resolve();
    assert!(!locked(&r, b), "expires when their turn ends");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [When Digivolving] shares the body
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_03_when_digivolving_runs_the_same_strip_and_freeze() {
    let mut r = runner();
    let a = r.place_stack(1, &["SRC1", "SRC2", "SRC3", "OPP"]);
    let b = r.place_on_field(1, "OPP", Some(0));
    let base = r.place_on_field(0, "BLUE3", Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    assert!(r
        .game
        .digivolve_from_hand(0, hi, base.index as usize, PlaySource::ByHand));

    let v = r
        .pending_selection_view()
        .expect("[When Digivolving] must install the strip pick");
    assert!(!v.is_optional);
    pick(&mut r, a);
    assert_eq!(source_ids(&r, a), vec!["SRC1".to_string()]);
    pick(&mut r, b);
    r.auto_resolve().unwrap();
    assert!(locked(&r, b));
    assert!(!locked(&r, a));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [Security] At the end of the battle, play this card free
// ═══════════════════════════════════════════════════════════════════════════

fn push_security(r: &mut DebugRunner, owner: u8, card_id: &str) {
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[owner as usize]
        .security
        .push(CardSource::new(idx, owner, next));
}

#[test]
fn st21_03_security_plays_card_after_battle() {
    let mut r = base_builder()
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(10)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    push_security(&mut r, 1, CARD_ID);

    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        0,
        "the flipped security card left the stack"
    );
    assert_eq!(
        r.battle_area_size(1),
        1,
        "Ikkakumon is played onto its owner's battle area free"
    );
    let top = r.game.players[1].battle_area[0]
        .card_sources
        .last()
        .unwrap()
        .data_index;
    assert_eq!(r.game.card_data[top].card_id, CARD_ID);
}

#[test]
fn st21_03_security_played_copy_runs_its_on_play() {
    // The security-played Ikkakumon belongs to P1, so its [On Play] strips /
    // freezes P0's Digimon. ATTACKER is deleted in the security battle (2000 vs
    // 4000 DP), BYSTANDER survives and has no digivolution cards ⇒ locked.
    let mut r = base_builder()
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(10)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let bystander = r.place_on_field(0, "BYSTANDER", Some(0));
    push_security(&mut r, 1, CARD_ID);

    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();

    let locked_any = r.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .any(|(i, _)| {
            locked(
                &r,
                PermanentHandle {
                    player: 0,
                    index: i as u8,
                },
            )
        });
    assert!(
        locked_any,
        "the security-played Ikkakumon's [On Play] must freeze a surviving opposing Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited [When Attacking] [Once Per Turn] draw (hand <= 7)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_03_inherited_draw_hand_size_boundary_seven_draws_eight_does_not() {
    for hand in [6usize, 7, 8] {
        let mut r = attack_runner(hand);
        let host = r.place_stack(0, &[CARD_ID, "BIG-HOST"]);
        let before = r.hand_size(0);
        assert_eq!(before, hand);
        let _ = r.attack_player(host, 1, false);
        r.auto_resolve().unwrap();
        assert_eq!(
            r.hand_size(0),
            before + usize::from(before <= 7),
            "hand {hand}: draws 1 only with 7 or fewer cards"
        );
    }
}

#[test]
fn st21_03_inherited_draw_is_once_per_turn_and_rearms_next_turn() {
    let mut r = attack_runner(3);
    let host = r.place_stack(0, &[CARD_ID, "BIG-HOST"]);

    let before = r.hand_size(0);
    let _ = r.attack_player(host, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), before + 1, "first attack draws");

    // Second attack the same turn: the host must be unsuspended to attack again.
    r.game.players[0].battle_area[host.index as usize].is_suspended = false;
    let before2 = r.hand_size(0);
    let _ = r.attack_player(host, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.hand_size(0),
        before2,
        "[Once Per Turn]: the second attack in the same turn draws nothing"
    );

    // Two end_turns later it is my turn again and the once-per-turn use is back.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    r.game.players[0].battle_area[host.index as usize].is_suspended = false;
    let before3 = r.hand_size(0);
    assert!(before3 <= 7, "stay inside the hand-size gate");
    let _ = r.attack_player(host, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.hand_size(0),
        before3 + 1,
        "the once-per-turn use re-arms on my next turn"
    );
}

#[test]
fn st21_03_inherited_draw_does_not_apply_without_the_card_in_the_stack() {
    let mut r = attack_runner(3);
    let host = r.place_on_field(0, "BIG-HOST", Some(0));
    let before = r.hand_size(0);
    let _ = r.attack_player(host, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), before, "no Ikkakumon under it ⇒ no draw");
}
