//! Review coverage for the owner ruling of 2026-11-23 ("keep parity with dcgo
//! on the other 10 too"; qa/rules/mechanics-gaps-2026-10.md W14 row (f)): ten
//! [Once Per Turn] clauses whose "by X" cost may be declined after the accept,
//! giving the use back (DCGO `RemoveUse()`).
//!
//! What the per-card tests in `cards_behavioral` do not already show:
//! - uses are per copy (CR 4.3 15-14-1-3): a refund on one carrier does not
//!   touch another carrier's copy, and a payment on one does not spend the
//!   other;
//! - two copies under ONE carrier (official Q7374, Gladimon (EX13-052)) are
//!   each offered once for the same removal; a copy whose cost was declined is
//!   not offered again for that removal (no loop);
//! - a cost candidate that becomes unpayable between the accept and the pick:
//!   choosing it spends the use and the carrier leaves (DCGO sets `isUsed` on
//!   the pick, before the deletion; Q7247);
//! - `Game::clone` at every decision of one accept-then-decline line per card,
//!   driven with the same decisions, ends in the same verification digest.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::Game;

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn with_text(mut c: CardData, text: &str) -> CardData {
    c.effect_text = text.to_string();
    c
}

fn with_traits(mut c: CardData, traits: &[&str]) -> CardData {
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn handle(r: &DebugRunner, player: usize, id: &str) -> PermanentHandle {
    let i = ids(r, player)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on player {player}'s field"));
    PermanentHandle {
        player: player as u8,
        index: i as u8,
    }
}

fn act(g: &mut Game, a: u16) {
    let p = g
        .pending_selection
        .as_ref()
        .expect("a pending choice")
        .selecting_player;
    assert_eq!(build_action_mask(g, p)[a as usize], 1.0, "action {a} is legal");
    g.resolve_selection(p, a).expect("resolve");
}

/// The first non-PASS legal id of the pending prompt.
fn first_non_pass(r: &DebugRunner) -> u16 {
    *r.pending_selection_view()
        .expect("prompt")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a non-PASS choice")
}

fn remove_by_opponent(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
}

#[derive(Clone, Copy)]
enum Step {
    /// Accept the pending optional prompt.
    Accept,
    /// PASS (decline) the pending prompt.
    Pass,
    /// Pick the effect-choice row whose label contains this text.
    Row(&'static str),
}

/// Drive `steps` on `r`, cloning the game before every decision; then drive
/// each clone through the same remaining decisions and compare digests.
fn clone_line(r: &mut DebugRunner, steps: &[Step]) {
    let mut clones = Vec::new();
    let mut line = Vec::new();
    for step in steps {
        let view = r.pending_selection_view().expect("a pending choice for the line");
        let a = match step {
            Step::Accept => {
                assert!(view.is_optional, "an optional prompt: {view:?}");
                first_non_pass(r)
            }
            Step::Pass => PASS,
            Step::Row(needle) => {
                view.effect_choices
                    .as_ref()
                    .and_then(|rows| rows.iter().find(|c| c.label.contains(needle)))
                    .unwrap_or_else(|| panic!("no `{needle}` row: {view:?}"))
                    .action_id
            }
        };
        clones.push(r.game.clone());
        line.push(a);
        act(&mut r.game, a);
    }
    let want = r.game.verification_digest();
    for (k, mut c) in clones.into_iter().enumerate() {
        for &a in &line[k..] {
            act(&mut c, a);
        }
        assert_eq!(
            c.verification_digest(),
            want,
            "the clone taken before decision {k} of {line:?} diverged"
        );
    }
}

// ─── Kotemon (EX13-048): per-copy uses, two copies, a stale cost candidate ──

fn kotemon_runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-048")
        .expect("EX13-048")
        .add_card(digimon("CARRIER-A", "Carrier A", 4, 4000))
        .add_card(digimon("CARRIER-B", "Carrier B", 4, 4000))
        .add_card(with_text(
            digimon("ALLY", "Ally", 4, 4000),
            "[On Play] If you have [Knightmon], draw 1.",
        ))
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .start();
    r.skip_mulligan();
    r
}

/// While <Evade> can also answer the deletion, the engine asks which
/// immediate replacement applies next (CR 4.3 15-8-5).
fn pick_replacement(r: &mut DebugRunner, evade: bool) {
    let v = r.pending_selection_view().expect("replacement prompt");
    if v.kind == SelectionKind::EffectChoice && v.prompt.contains("immediate replacement") {
        let row = v
            .effect_choices
            .as_ref()
            .and_then(|rows| rows.iter().find(|c| c.label.contains("Evade") == evade))
            .unwrap_or_else(|| panic!("replacement order rows: {:?}", v.effect_choices))
            .action_id;
        r.execute_action(v.selecting_player, row).expect("order pick");
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
}

/// CR 4.3 15-14-1-3: each copy counts its own uses. Paying on carrier A does
/// not spend carrier B's copy; declining B's cost keeps B's use only.
#[test]
fn kotemon_uses_are_per_copy_across_two_carriers() {
    let mut r = kotemon_runner();
    r.place_stack(0, &["EX13-048", "CARRIER-A"]);
    r.place_stack(0, &["EX13-048", "CARRIER-B"]);
    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(0, "ALLY", Some(0));

    // A: accept and pay.
    let a = handle(&r, 0, "CARRIER-A");
    remove_by_opponent(&mut r, a);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept on A");
    let pay = first_non_pass(&r);
    r.execute_action(0, pay).expect("pay on A");
    let _ = r.auto_resolve();
    assert!(ids(&r, 0).contains(&"CARRIER-A".to_string()), "A paid and stayed");
    assert_eq!(ids(&r, 0).iter().filter(|c| *c == "ALLY").count(), 2);

    // B: offered (its own use), accept, decline the cost; <Evade> keeps B.
    let b = handle(&r, 0, "CARRIER-B");
    r.game
        .modifiers
        .grant_keyword(b, Keyword::Evade, Expiry::Permanent, 0);
    remove_by_opponent(&mut r, b);
    pick_replacement(&mut r, false);
    r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept on B");
    r.execute_action(0, PASS).expect("decline B's cost");
    pick_replacement(&mut r, true);
    r.execute_action(0, REPLACEMENT_ACCEPT).expect("<Evade>");
    assert!(r.pending_selection().is_none(), "B's clause is not offered again for this removal");
    assert_eq!(ids(&r, 0).iter().filter(|c| *c == "ALLY").count(), 2, "B paid nothing");

    // B again: kept, so offered; decline at the accept prompt.
    let b = handle(&r, 0, "CARRIER-B");
    remove_by_opponent(&mut r, b);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "B's use was kept");
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(!ids(&r, 0).contains(&"CARRIER-B".to_string()));

    // A again: spent, with allies still able to pay.
    let a = handle(&r, 0, "CARRIER-A");
    remove_by_opponent(&mut r, a);
    assert!(
        r.pending_selection_view()
            .is_none_or(|v| v.kind != SelectionKind::Replacement),
        "A's use was spent"
    );
    let _ = r.auto_resolve();
    assert!(!ids(&r, 0).contains(&"CARRIER-A".to_string()));
}

/// Resolve the would-leave prompts of one removal, declining each copy's cost
/// after accepting (`pay_on` = the 1-based offer to pay on instead). Returns
/// how many times a copy was offered.
fn run_two_copy_removal(r: &mut DebugRunner, pay_on: Option<usize>) -> usize {
    let mut offers = 0;
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            return offers;
        };
        match v.kind {
            SelectionKind::EffectChoice if v.prompt.contains("immediate replacement") => {
                let row = v.effect_choices.as_ref().expect("rows")[0].action_id;
                r.execute_action(v.selecting_player, row).expect("order");
            }
            SelectionKind::Replacement => {
                offers += 1;
                r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
                if pay_on == Some(offers) {
                    let pay = first_non_pass(r);
                    r.execute_action(0, pay).expect("pay");
                } else {
                    assert_eq!(
                        build_action_mask(&r.game, 0)[PASS as usize],
                        1.0,
                        "the cost may be declined"
                    );
                    r.execute_action(0, PASS).expect("decline the cost");
                }
            }
            other => panic!("unexpected prompt {other:?}: {v:?}"),
        }
    }
    panic!("more than 12 prompts for one removal: a declined copy is re-offered");
}

/// Official Q7374 (Gladimon (EX13-052), the same clause): two copies under one
/// Digimon trigger together and each may activate. A copy whose cost was
/// declined is not offered again for the same removal, and the other copy is
/// still offered: two offers, then the carrier leaves.
#[test]
fn kotemon_two_copies_under_one_carrier_each_offered_once_per_removal() {
    let mut r = kotemon_runner();
    r.place_stack(0, &["EX13-048", "EX13-048", "CARRIER-A"]);
    r.place_on_field(0, "ALLY", Some(0));
    let a = handle(&r, 0, "CARRIER-A");
    remove_by_opponent(&mut r, a);
    assert_eq!(run_two_copy_removal(&mut r, None), 2, "each copy offered exactly once");
    let _ = r.auto_resolve();
    assert!(!ids(&r, 0).contains(&"CARRIER-A".to_string()), "nothing paid: it leaves");
    assert!(ids(&r, 0).contains(&"ALLY".to_string()), "nothing deleted");
}

/// First copy declined after the accept (kept), second copy paid (spent): the
/// carrier stays, and the next removal this turn offers exactly one copy.
#[test]
fn kotemon_two_copies_declined_one_kept_paid_one_spent() {
    let mut r = kotemon_runner();
    r.place_stack(0, &["EX13-048", "EX13-048", "CARRIER-A"]);
    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    let a = handle(&r, 0, "CARRIER-A");
    remove_by_opponent(&mut r, a);
    assert_eq!(run_two_copy_removal(&mut r, Some(2)), 2);
    let _ = r.auto_resolve();
    assert!(ids(&r, 0).contains(&"CARRIER-A".to_string()), "the second copy paid");
    assert_eq!(ids(&r, 0).iter().filter(|c| *c == "ALLY").count(), 1);

    let a = handle(&r, 0, "CARRIER-A");
    remove_by_opponent(&mut r, a);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "the declined copy kept its use"
    );
    r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept the kept copy");
    let pay = first_non_pass(&r);
    r.execute_action(0, pay).expect("pay");
    assert!(
        r.pending_selection_view()
            .is_none_or(|v| v.kind != SelectionKind::Replacement),
        "the paid copy is spent: not offered after the kept copy saved it"
    );
    let _ = r.auto_resolve();
    assert!(ids(&r, 0).contains(&"CARRIER-A".to_string()));
    assert!(!ids(&r, 0).contains(&"ALLY".to_string()));
}

/// The only cost candidate becomes undeletable between the accept and the
/// pick. Choosing it is still a choice (DCGO EX13_048.cs sets `isUsed` in
/// the after-select coroutine before the deletion): the use is spent and the
/// carrier leaves (Q7247 on the same "by deleting" shape).
#[test]
fn kotemon_cost_candidate_made_undeletable_after_the_accept_spends_the_use() {
    let mut r = kotemon_runner();
    r.place_stack(0, &["EX13-048", "CARRIER-A"]);
    r.place_stack(0, &["EX13-048", "CARRIER-B"]);
    r.place_on_field(0, "ALLY", Some(0));
    let a = handle(&r, 0, "CARRIER-A");
    remove_by_opponent(&mut r, a);
    r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    let ally = handle(&r, 0, "ALLY");
    r.game.modifiers.add(
        ally,
        ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect, 0, Expiry::Permanent, 0),
    );
    let pay = first_non_pass(&r);
    r.execute_action(0, pay).expect("choose the now-undeletable ally");
    let _ = r.auto_resolve();
    assert!(ids(&r, 0).contains(&"ALLY".to_string()), "its deletion was prevented");
    assert!(!ids(&r, 0).contains(&"CARRIER-A".to_string()), "not paid: A leaves");
    // Carrier B (its own copy) is unaffected and still offered: the ally is
    // still on the field but can no longer be deleted, so B has no payable
    // candidate in our engine (15-7-4, the pair's `can_pay` filter).
    let b = handle(&r, 0, "CARRIER-B");
    remove_by_opponent(&mut r, b);
    let _ = r.auto_resolve();
    assert!(!ids(&r, 0).contains(&"CARRIER-B".to_string()));
}

// ─── Game::clone at every decision of one accept-then-decline line per card ──

#[test]
fn clone_titamon_bt25_084_declined_hand_cost() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-084")
        .expect("BT25-084")
        .add_card(digimon("PAD", "Pad", 3, 3000))
        .hand(0, &["PAD", "PAD", "PAD", "PAD"])
        .memory(13)
        .start();
    r.set_first_player(0);
    let t = r.place_on_field(0, "BT25-084", Some(0));
    remove_by_opponent(&mut r, t);
    clone_line(&mut r, &[Step::Accept, Step::Pass]);
    assert_eq!(r.game.players[0].hand.len(), 4);
}

#[test]
fn clone_lilamon_bt26_044_declined_tamer_cost() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT26-044")
        .expect("BT26-044")
        .add_card(with_traits(digimon("DS6", "DS carrier", 6, 12000), &["DATA SQUAD"]))
        .add_card(tamer("TAMER"))
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    let t = r.place_stack(0, &["FILL", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    let c = r.place_stack(0, &["BT26-044", "DS6"]);
    r.game
        .delete_permanents_batch(vec![c], ReplacementCause::OpponentEffect);
    clone_line(&mut r, &[Step::Accept, Step::Pass]);
}

#[test]
fn clone_gallantmon_ex13_015_declined_deletion_cost() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-015")
        .expect("EX13-015")
        .add_card(digimon("TINY", "Tiny", 3, 3000))
        .start();
    let g = r.place_on_field(0, "EX13-015", Some(0));
    r.place_on_field(1, "TINY", Some(0));
    remove_by_opponent(&mut r, g);
    clone_line(&mut r, &[Step::Accept, Step::Pass]);
    assert_eq!(ids(&r, 1), vec!["TINY".to_string()]);
}

fn sukamon_line(card: &str) {
    let mut r = DebugRunner::builder()
        .dsl_card(card)
        .expect("Sukamon-line card")
        .add_card(digimon("CARRIER", "Carrier", 4, 4000))
        .add_card(digimon("SUKA", "Sukamon", 4, 4000))
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .start();
    r.skip_mulligan();
    let c = r.place_stack(0, &[card, "CARRIER"]);
    r.place_on_field(1, "SUKA", Some(0));
    remove_by_opponent(&mut r, c);
    clone_line(&mut r, &[Step::Accept, Step::Pass]);
    assert_eq!(ids(&r, 1), vec!["SUKA".to_string()], "the opponent's Sukamon stays");
}

#[test]
fn clone_chuumon_ex13_027_declined_deletion_cost() {
    sukamon_line("EX13-027");
}

#[test]
fn clone_sukamon_ex13_028_declined_deletion_cost() {
    sukamon_line("EX13-028");
}

fn chirinmon_line(steps: &[Step]) {
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-032")
        .expect("EX13-032")
        .add_card(digimon("FILL", "Filler", 3, 1000))
        .add_card(digimon("BASE-L4", "Base", 4, 4000))
        .add_card(digimon("OPP", "Opp", 4, 4000))
        .add_card(with_traits(tamer("TAMER"), &["DATA SQUAD"]))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL", "FILL"])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    let t = r.place_stack(0, &["FILL", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    let chirin = r.place_stack(0, &["BASE-L4", "EX13-032"]);
    r.place_on_field(1, "OPP", Some(0));
    r.attack_player(chirin, 1, false);
    clone_line(&mut r, steps);
    assert_eq!(r.security_count(0), 2, "nothing paid");
}

#[test]
fn clone_chirinmon_ex13_032_dont_pay() {
    chirinmon_line(&[Step::Accept, Step::Row("Don't pay")]);
}

#[test]
fn clone_chirinmon_ex13_032_declined_tamer_pick() {
    chirinmon_line(&[Step::Accept, Step::Row("Tamers"), Step::Pass]);
}

fn knightmon_line(card: &str) {
    let mut r = kotemon_runner_for(card);
    let c = r.place_stack(0, &[card, "CARRIER-A"]);
    r.place_on_field(0, "ALLY", Some(0));
    remove_by_opponent(&mut r, c);
    clone_line(&mut r, &[Step::Accept, Step::Pass]);
    assert!(ids(&r, 0).contains(&"ALLY".to_string()));
}

fn kotemon_runner_for(card: &str) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(card)
        .expect("Knightmon-text line card")
        .add_card(digimon("CARRIER-A", "Carrier A", 4, 4000))
        .add_card(with_text(
            digimon("ALLY", "Ally", 4, 4000),
            "[On Play] If you have [Knightmon], draw 1.",
        ))
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .start();
    r.skip_mulligan();
    r
}

#[test]
fn clone_kotemon_ex13_048_declined_deletion_cost() {
    knightmon_line("EX13-048");
}

#[test]
fn clone_gladimon_ex13_052_declined_deletion_cost() {
    knightmon_line("EX13-052");
}

fn bigmamemon_line(inherited: bool) {
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-059")
        .expect("EX13-059")
        .add_card(digimon("TOP", "Top", 6, 6000))
        .add_card(digimon("OWN-MAME", "Mamemon", 4, 4000))
        .add_card({
            let mut c = digimon("OPP-C3", "Opp three", 3, 3000);
            c.play_cost = 3;
            c
        })
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .deck(0, &["FILL"; 6])
        .memory(2)
        .start();
    r.skip_mulligan();
    let host = if inherited {
        r.place_stack(0, &["EX13-059", "TOP"])
    } else {
        r.place_on_field(0, "EX13-059", Some(0))
    };
    r.place_on_field(0, "OWN-MAME", Some(0));
    r.place_on_field(1, "OPP-C3", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    clone_line(&mut r, &[Step::Accept, Step::Pass]);
    assert!(r.pending_selection().is_none(), "the same trigger offers nothing more");
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(r.battle_area_size(1), 1, "no reward");
}

#[test]
fn clone_bigmamemon_ex13_059_face_up_declined_cost() {
    bigmamemon_line(false);
}

#[test]
fn clone_bigmamemon_ex13_059_inherited_declined_cost() {
    bigmamemon_line(true);
}
