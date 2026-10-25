//! BT26-097 The Thunder Emperor Awakens — Option, Yellow, use cost 2 (TS).
//!
//! Add 1 to this card's use cost for each of your security cards.
//! [Main] By placing 1 of your Tamers with [Dan Yuki] or [Kanan Yuki] in their
//! names as any of your [Aegiomon]'s bottom digivolution card, it may digivolve
//! into [Jupitermon] in the hand or trash, ignoring digivolution requirements
//! and without paying the cost. After, you may place 1 card with [Aegiochusmon]
//! in its name in your trash as any of your [Jupitermon]'s top digivolution
//! card.
//! [Security] You may play 1 play cost 5 or lower [TS] trait card from your
//! hand without paying the cost. Then, add this card to the hand.
//!
//! Official Q&A: the use-cost increase is always active (the card is referenced
//! as an Option with that cost).
//!
//! DCGO: BT26/Yellow/BT26_097.cs. Rules: §15-7-1/15-7-2 ("by X" is an optional
//! processing condition; unless it is executed, neither the digivolve nor the
//! "After" part can be executed).
//!
//! Pattern rows: use-cost increase (G-ENGINE-OPTION-SELF-USE-COST-INCREASE),
//! optional cost-then-digivolve (BT26-098 idiom), hand∪trash digivolve source,
//! place-as-top-source, security play-then-add-to-hand.

use super::support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-097";

fn ts_option(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.traits = vec!["TS".to_string()];
    c
}

fn setup_with_security(n: usize) -> DebugRunner {
    setup_full(n, &[])
}

/// `setup_with_security` plus extra real DSL cards (e.g. BT24-101 Jupitermon,
/// whose leave-battle-area replacement protects own [TS] Tamers).
fn setup_full(n: usize, extra_dsl: &[&str]) -> DebugRunner {
    let sec = vec!["FILLER"; n];
    let mut b = DebugRunner::builder().dsl_card(CARD_ID).expect("BT26-097");
    for id in extra_dsl {
        b = b.dsl_card(id).expect("extra DSL card");
    }
    let mut r = b
        .add_card(filler("FILLER"))
        .add_card(tamer("DAN", "Dan Yuki", CardColor::Yellow, &["TS"]))
        .add_card(tamer("KANAN", "Kanan Yuki", CardColor::Green, &["TS"]))
        .add_card(tamer("OTHER", "Someone Else", CardColor::Yellow, &["TS"]))
        .add_card(digimon("AEGIO", "Aegiomon", CardColor::Yellow, 4, 5, &["TS"]))
        .add_card(digimon("NOTAEGIO", "Elecmon", CardColor::Yellow, 4, 5, &["TS"]))
        .add_card(digimon("JUPI", "Jupitermon", CardColor::Yellow, 6, 12, &["TS"]))
        .add_card(digimon(
            "AEGIOCHUS",
            "Aegiochusmon: Holy",
            CardColor::Yellow,
            5,
            8,
            &["TS"],
        ))
        .add_card(digimon("TS5", "Ts Five", CardColor::Yellow, 5, 5, &["TS"]))
        .add_card(digimon("TS6", "Ts Six", CardColor::Yellow, 5, 6, &["TS"]))
        .add_card(digimon("PLAIN3", "Plain Three", CardColor::Yellow, 3, 3, &[]))
        .add_card(ts_option("TSOPT", 1))
        .add_card({
            let mut t = tamer("TSTAMER6", "Ts Tamer Six", CardColor::Yellow, &["TS"]);
            t.play_cost = 6;
            t
        })
        .add_card(digimon("ATK", "Attacker", CardColor::Blue, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &sec)
        .security(1, &["FILLER"; 3])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn setup() -> DebugRunner {
    setup_with_security(0)
}

fn use_card(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let res = r.game.play_option_from_hand(0, idx);
    assert_ne!(format!("{res:?}"), "Invalid", "the Option must be usable");
}

/// Own-side field pick (own-only `select_own_permanent` prompt).
fn pick_own(r: &mut DebugRunner, h: PermanentHandle) {
    let action = encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("own-permanent prompt");
    assert!(
        v.valid_action_ids.contains(&action),
        "slot {} is not a legal pick (valid {:?})",
        h.index,
        v.valid_action_ids
    );
    r.execute_action(0, action).unwrap();
}

fn legal_own(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.contains(&encode_attack(0, h.index as u16)))
        .unwrap_or(false)
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn drain_pass(r: &mut DebugRunner) {
    while r.game.pending_selection.is_some() {
        pass(r, 0);
    }
}

// ─── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn bt26_097_has_use_cost_increase_main_and_security() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", card.effects);
    assert!(dbg.contains("UseCostIncrease"), "use-cost increase clause");
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand))));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity))));
}

// ─── Clause 1: use cost +1 per own security card ─────────────────────────────

#[test]
fn bt26_097_use_cost_grows_with_own_security() {
    let mut r = setup_with_security(4);
    push_hand(&mut r, 0, CARD_ID);
    let card = r.game.players[0].hand.last().unwrap().clone();
    assert_eq!(r.game.option_use_cost(&card, 0), 2 + 4);
    r.game.players[0].security.pop();
    assert_eq!(r.game.option_use_cost(&card, 0), 2 + 3, "re-read live");
}

#[test]
fn bt26_097_use_cost_is_printed_two_with_no_security() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    let card = r.game.players[0].hand.last().unwrap().clone();
    assert_eq!(r.game.option_use_cost(&card, 0), 2);
}

#[test]
fn bt26_097_paying_the_increased_use_cost() {
    let mut r = setup_with_security(3);
    r.place_on_field(0, "NOTAEGIO", Some(0)); // a yellow Digimon: colour requirement met
    let mem = r.memory();
    use_card(&mut r);
    drain_pass(&mut r);
    assert_eq!(r.memory(), mem - 5, "2 + 3 security cards");
}

#[test]
fn bt26_097_unusable_when_the_increase_exceeds_the_memory_floor() {
    // Memory -7: the printed 2 would land on -9 (legal), but 2 + 3 lands on -12.
    let mut r = setup_with_security(3);
    r.place_on_field(0, "NOTAEGIO", Some(0)); // colour requirement met
    r.game.memory = -7;
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let res = r.game.play_option_from_hand(0, idx);
    assert_eq!(format!("{res:?}"), "Invalid");
    // Control: with no security cards the cost is the printed 2 → usable.
    r.game.players[0].security.clear();
    let res = r.game.play_option_from_hand(0, idx);
    assert_ne!(format!("{res:?}"), "Invalid");
}

// ─── Clause 2: [Main] ────────────────────────────────────────────────────────

#[test]
fn bt26_097_main_places_tamer_digivolves_from_hand_then_tops_aegiochusmon() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "AEGIOCHUS");
    use_card(&mut r);
    let mem = r.memory();

    pick_own(&mut r, dan); // the Tamer — the optional "by" cost
    pick_own(&mut r, aegio); // the [Aegiomon]
    pick_hand(&mut r, 0, "JUPI"); // may digivolve: Jupitermon from hand
    pick_trash(&mut r, 0, "AEGIOCHUS"); // After: may place an [Aegiochusmon]
    pick_own(&mut r, aegio); // ... as this [Jupitermon]'s top digivolution card
    let _ = r.auto_resolve();

    assert_eq!(
        stack_ids(&r, aegio),
        vec!["DAN", "AEGIO", "AEGIOCHUS", "JUPI"],
        "Tamer at the bottom, Aegiochusmon directly under the top card"
    );
    assert_eq!(r.memory(), mem, "digivolve without paying the cost");
    assert!(!field_ids(&r, 0).contains(&"DAN".to_string()));
    assert!(!trash_ids(&r, 0).contains(&"AEGIOCHUS".to_string()));
}

#[test]
fn bt26_097_main_can_digivolve_into_jupitermon_in_the_trash() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let kanan = r.place_on_field(0, "KANAN", Some(0));
    push_trash(&mut r, 0, "JUPI");
    use_card(&mut r);
    pick_own(&mut r, kanan); // [Kanan Yuki] qualifies too
    pick_own(&mut r, aegio);
    pick_trash(&mut r, 0, "JUPI");
    let _ = r.auto_resolve();
    assert_eq!(stack_ids(&r, aegio), vec!["KANAN", "AEGIO", "JUPI"]);
    assert!(!trash_ids(&r, 0).contains(&"JUPI".to_string()));
}

#[test]
fn bt26_097_main_tamer_pick_is_optional_and_declining_does_nothing() {
    // §15-7-2: without the placement neither the digivolve nor the "After"
    // part runs — even with a [Jupitermon] already out and an [Aegiochusmon]
    // in the trash.
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    let jupi = r.place_on_field(0, "JUPI", Some(0));
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "AEGIOCHUS");
    use_card(&mut r);
    let v = r.pending_selection_view().expect("Tamer prompt");
    assert!(v.is_optional, "'by placing' is an optional processing condition");
    assert!(legal_own(&r, dan));
    pass(&mut r, 0);
    assert!(r.game.pending_selection.is_none(), "nothing else is offered");
    assert_eq!(stack_ids(&r, aegio), vec!["AEGIO"]);
    assert_eq!(stack_ids(&r, jupi), vec!["JUPI"]);
    assert!(field_ids(&r, 0).contains(&"DAN".to_string()));
    assert!(trash_ids(&r, 0).contains(&"AEGIOCHUS".to_string()));
}

#[test]
fn bt26_097_main_offers_only_dan_or_kanan_tamers() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let other = r.place_on_field(0, "OTHER", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    use_card(&mut r);
    assert!(legal_own(&r, dan));
    assert!(!legal_own(&r, other), "a Tamer without [Dan Yuki]/[Kanan Yuki] in its name");
    assert!(!legal_own(&r, aegio), "a Digimon is not a Tamer");
    drain_pass(&mut r);
}

#[test]
fn bt26_097_main_does_nothing_without_a_dan_or_kanan_tamer() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    r.place_on_field(0, "OTHER", Some(0));
    push_hand(&mut r, 0, "JUPI");
    use_card(&mut r);
    assert!(r.game.pending_selection.is_none(), "no legal cost → no prompt");
    assert_eq!(stack_ids(&r, aegio), vec!["AEGIO"]);
}

#[test]
fn bt26_097_main_does_nothing_without_an_aegiomon() {
    let mut r = setup();
    let notaegio = r.place_on_field(0, "NOTAEGIO", Some(0));
    r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    use_card(&mut r);
    assert!(r.game.pending_selection.is_none(), "no [Aegiomon] → cost impossible (§15-7-4)");
    assert!(field_ids(&r, 0).contains(&"DAN".to_string()), "Tamer stays");
    assert_eq!(stack_ids(&r, notaegio), vec!["NOTAEGIO"]);
}

#[test]
fn bt26_097_main_aegiomon_pick_excludes_other_digimon() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let notaegio = r.place_on_field(0, "NOTAEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    use_card(&mut r);
    pick_own(&mut r, dan);
    assert!(legal_own(&r, aegio));
    assert!(!legal_own(&r, notaegio), "only [Aegiomon]");
    drain_pass(&mut r);
}

#[test]
fn bt26_097_main_digivolve_is_optional_after_the_placement() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    let v = r.pending_selection_view().expect("digivolve prompt");
    assert!(v.is_optional, "'it may digivolve'");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(stack_ids(&r, aegio), vec!["DAN", "AEGIO"], "Tamer still placed");
    assert!(hand_ids(&r, 0).contains(&"JUPI".to_string()));
}

#[test]
fn bt26_097_main_after_can_top_any_jupitermon_even_without_digivolving() {
    // "any of your [Jupitermon]" — the After part is not tied to the digivolve.
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let jupi = r.place_on_field(0, "JUPI", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_trash(&mut r, 0, "AEGIOCHUS");
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    // No [Jupitermon] in hand/trash → no digivolve prompt; straight to After.
    let v = r.pending_selection_view().expect("After prompt");
    assert!(v.is_optional, "'you may place'");
    pick_trash(&mut r, 0, "AEGIOCHUS");
    pick_own(&mut r, jupi);
    let _ = r.auto_resolve();
    assert_eq!(stack_ids(&r, jupi), vec!["AEGIOCHUS", "JUPI"]);
    assert_eq!(stack_ids(&r, aegio), vec!["DAN", "AEGIO"]);
}

#[test]
fn bt26_097_main_after_is_optional() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "AEGIOCHUS");
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    pick_hand(&mut r, 0, "JUPI");
    pass(&mut r, 0); // decline the After placement
    let _ = r.auto_resolve();
    assert_eq!(stack_ids(&r, aegio), vec!["DAN", "AEGIO", "JUPI"]);
    assert!(trash_ids(&r, 0).contains(&"AEGIOCHUS".to_string()));
}

#[test]
fn bt26_097_main_after_is_skipped_without_aegiochusmon_in_trash() {
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "TS5");
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    pick_hand(&mut r, 0, "JUPI");
    assert!(r.game.pending_selection.is_none(), "no [Aegiochusmon] card → no After prompt");
    assert_eq!(stack_ids(&r, aegio), vec!["DAN", "AEGIO", "JUPI"]);
}

#[test]
fn bt26_097_main_after_is_skipped_without_a_jupitermon_on_the_field() {
    // "any of your [Jupitermon]": with the digivolve declined and no
    // [Jupitermon] in play there is no host, so no After prompt — even with an
    // [Aegiochusmon] card in the trash (DCGO gates the After part on an own
    // [Jupitermon] permanent existing).
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "AEGIOCHUS");
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    let v = r.pending_selection_view().expect("digivolve prompt");
    assert!(v.is_optional);
    pass(&mut r, 0); // decline the digivolve
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "no [Jupitermon] on the field → no After prompt");
    assert_eq!(stack_ids(&r, aegio), vec!["DAN", "AEGIO"]);
    assert!(trash_ids(&r, 0).contains(&"AEGIOCHUS".to_string()));
    assert!(hand_ids(&r, 0).contains(&"JUPI".to_string()));
}

#[test]
fn bt26_097_main_trashes_the_cards_under_the_placed_tamer() {
    // CR 4.3 §4-7-7 / §4-7-8: the Tamer placed under the [Aegiomon] is removed
    // from the field, so the cards under it are trashed at the same time; only
    // the Tamer card itself becomes the bottom digivolution card. DCGO moves
    // only `selectedTamer.TopCard` (IPlacePermanentToDigivolutionCards →
    // DiscardEvoRoots). Reachable via e.g. <Material Save> onto a Dan Yuki
    // Tamer (BT10-009).
    let mut r = setup();
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_stack(0, &["PLAIN3", "DAN"]);
    push_hand(&mut r, 0, "JUPI");
    assert!(!trash_ids(&r, 0).contains(&"PLAIN3".to_string()));
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    pick_hand(&mut r, 0, "JUPI");
    let _ = r.auto_resolve();
    assert_eq!(
        stack_ids(&r, aegio),
        vec!["DAN", "AEGIO", "JUPI"],
        "only the Tamer card rides under the [Aegiomon]"
    );
    assert_eq!(
        trash_ids(&r, 0).iter().filter(|id| id.as_str() == "PLAIN3").count(),
        1,
        "the card that was under the Tamer is trashed (§4-7-8)"
    );
    assert!(!field_ids(&r, 0).contains(&"DAN".to_string()));
}

// ─── Clause 2: the placement is a leave-the-battle-area event ─────────────────
//
// CR 4.3 §4-7-7: a permanent placed as a digivolution card leaves the battle
// area. DCGO routes a field top card through IPlacePermanentToDigivolutionCards
// (CardController.cs), which runs the WhenPermanentWouldRemoveField check and
// stacks WhenRemoveField effects before moving it, and sets `Placed` only on a
// real placement. The archetype's own BT24-101 Jupitermon prints "[All Turns]
// [OPT] When any of your [TS] Digimon or Tamers would leave the battle area,
// by trashing your top security card, they don't leave" — every [Dan Yuki] /
// [Kanan Yuki] Tamer is [TS], so the player must be offered that protection.
// §15-7-2 + Q7170: if the "by" placement isn't executed, neither the digivolve
// nor the "After" part is processed.

fn setup_with_bt24_101() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = setup_full(1, &["BT24-101"]);
    r.place_on_field(0, "BT24-101", Some(0)); // a [Jupitermon] on the field
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    let dan = r.place_on_field(0, "DAN", Some(0));
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "AEGIOCHUS");
    (r, aegio, dan)
}

fn assert_replacement_prompt(r: &DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("the would-leave replacement window must open for the Tamer");
    assert!(
        v.valid_action_ids.contains(&REPLACEMENT_ACCEPT) && r.pending_is_optional(),
        "expected BT24-101's optional protection prompt, got {:?}",
        v.valid_action_ids
    );
}

#[test]
fn bt26_097_main_placement_offers_bt24_101_protection_and_accepting_stops_the_effect() {
    let (mut r, aegio, dan) = setup_with_bt24_101();
    use_card(&mut r);
    let security_before = r.security_count(0);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);

    assert_replacement_prompt(&r);
    r.execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept BT24-101's protection");

    // The Tamer doesn't leave, so it is not placed: §15-7-2 / Q7170 — no
    // digivolve prompt and no "After" prompt (a [Jupitermon] is on the field
    // and an [Aegiochusmon] is in the trash, so an ungated After would ask).
    assert!(
        r.pending_selection_view().is_none(),
        "no digivolve / After prompt once the placement was prevented"
    );
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DAN".to_string()), "the Tamer stays");
    assert_eq!(stack_ids(&r, aegio), vec!["AEGIO"], "nothing went under the [Aegiomon]");
    assert!(hand_ids(&r, 0).contains(&"JUPI".to_string()), "no digivolve");
    assert!(trash_ids(&r, 0).contains(&"AEGIOCHUS".to_string()), "no After");
    assert_eq!(
        r.security_count(0),
        security_before - 1,
        "BT24-101's cost (trash the top security card) was paid"
    );
}

#[test]
fn bt26_097_main_placement_declining_bt24_101_protection_proceeds() {
    let (mut r, aegio, dan) = setup_with_bt24_101();
    use_card(&mut r);
    let security_before = r.security_count(0);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);

    assert_replacement_prompt(&r);
    pass(&mut r, 0); // decline the protection: the Tamer leaves

    pick_hand(&mut r, 0, "JUPI"); // may digivolve: Jupitermon from hand
    pick_trash(&mut r, 0, "AEGIOCHUS"); // After
    pick_own(&mut r, aegio);
    let _ = r.auto_resolve();

    assert_eq!(stack_ids(&r, aegio), vec!["DAN", "AEGIO", "AEGIOCHUS", "JUPI"]);
    assert!(!field_ids(&r, 0).contains(&"DAN".to_string()));
    assert_eq!(r.security_count(0), security_before, "declined: no security paid");
}

#[test]
fn bt26_097_main_works_when_the_tamer_sits_left_of_the_aegiomon() {
    // The Tamer usually entered play first, so it occupies a LOWER battle-area
    // slot than the [Aegiomon]; removing it shifts the [Aegiomon] down one
    // slot. The digivolve and the "After" must still find that [Aegiomon]
    // (by identity), not whatever now sits in its old slot.
    let mut r = setup();
    let dan = r.place_on_field(0, "DAN", Some(0));
    let aegio = r.place_on_field(0, "AEGIO", Some(0));
    r.place_on_field(0, "NOTAEGIO", Some(0)); // would slide into aegio's old slot
    push_hand(&mut r, 0, "JUPI");
    push_trash(&mut r, 0, "AEGIOCHUS");
    use_card(&mut r);
    pick_own(&mut r, dan);
    pick_own(&mut r, aegio);
    pick_hand(&mut r, 0, "JUPI");
    let aegio_now = PermanentHandle { player: 0, index: aegio.index - 1 };
    pick_trash(&mut r, 0, "AEGIOCHUS");
    pick_own(&mut r, aegio_now);
    let _ = r.auto_resolve();
    assert_eq!(
        stack_ids(&r, aegio_now),
        vec!["DAN", "AEGIO", "AEGIOCHUS", "JUPI"],
        "the [Aegiomon] that received the Tamer is the one that digivolves"
    );
    assert_eq!(stack_ids(&r, aegio), vec!["NOTAEGIO"], "the neighbour is untouched");
}

// ─── Clause 3: [Security] ────────────────────────────────────────────────────

fn reveal_from_security(r: &mut DebugRunner) {
    let atk = r.place_on_field(1, "ATK", Some(0));
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    r.set_first_player(1);
    r.attack_player(atk, 0, false);
}

#[test]
fn bt26_097_security_plays_cost_five_or_lower_ts_from_hand_free_then_adds_self() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS5");
    push_hand(&mut r, 0, "TS6");
    push_hand(&mut r, 0, "PLAIN3");
    push_hand(&mut r, 0, "TSOPT");
    let mem = r.memory();
    reveal_from_security(&mut r);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional, "'you may play'");
    let rows=physical_menu::commits(&r.game);
    assert_eq!(rows.len(),1,"only the cost-five-or-lower TS Digimon is a legal play; TS6, plain and Option are excluded");
    let index=r.game.player(0).hand.iter().position(|card|card.card_id(&r.game.card_data)=="TS5").unwrap();
    assert_eq!(rows[0].action_id,physical_menu::zone_action(&r.game,0,index,digimon_engine::owned_card_choice::CardChoiceZone::Hand));
    pick_hand(&mut r, 0, "TS5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TS5".to_string()));
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()), "then add this card to the hand");
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt26_097_security_declined_still_adds_self_to_hand() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS5");
    reveal_from_security(&mut r);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TS5".to_string()));
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_097_security_with_no_eligible_card_still_adds_self_to_hand() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS6");
    reveal_from_security(&mut r);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(hand_ids(&r, 0).contains(&"TS6".to_string()));
}

#[test]
fn bt26_097_security_can_play_a_cost_five_or_lower_ts_tamer_free() {
    // "1 play cost 5 or lower [TS] trait card": Tamers qualify (DCGO HasPlayCost).
    let mut r = setup();
    push_hand(&mut r, 0, "DAN"); // [TS] Tamer, play cost 3
    push_hand(&mut r, 0, "TSTAMER6"); // [TS] Tamer, play cost 6
    let mem = r.memory();
    reveal_from_security(&mut r);
    let v = r.pending_selection_view().expect("security play prompt");
    let pos = |r: &DebugRunner, id: &str| {
        r.game.players[0].hand.iter().position(|c| c.card_id(&r.game.card_data) == id).unwrap()
    };
    let rows=physical_menu::commits(&r.game);
    assert_eq!(rows.len(),1,"only the cost-three TS Tamer can play; cost-six Tamer is excluded");
    assert_eq!(rows[0].action_id,physical_menu::zone_action(&r.game,0,pos(&r,"DAN"),digimon_engine::owned_card_choice::CardChoiceZone::Hand));
    pick_hand(&mut r, 0, "DAN");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DAN".to_string()), "the Tamer is played");
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()), "then add this card to the hand");
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
