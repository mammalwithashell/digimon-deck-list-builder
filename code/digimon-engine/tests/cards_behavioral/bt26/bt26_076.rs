//! BT26-076 Crowmon — Lv.5 Purple, Mysterious Bird/DATA SQUAD.
//!
//! [When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon.
//! Then, by trashing the bottom face-down card from under any of your Tamers,
//! they trash 1 card in their hand.
//! [Your Turn] [Once Per Turn] When your opponent's hand is trashed from or
//! effects trash cards from under your Tamers, this Digimon may digivolve into
//! [Ravemon] or a [DATA SQUAD] trait Digimon card in the trash with the cost
//! reduced by 1.
//! Inherited: [On Deletion] You may play 1 play cost 5 or lower card with
//! [Avian] or [Bird] in any of its traits or the [DATA SQUAD] trait from your
//! trash without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_076.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT26-076";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-076")
        .dsl_card("BT26-072")
        .expect("Peckmon")
        .dsl_card("BT26-094")
        .expect("Keenan")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon("OPP4", "Opp4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
        .add_card({
            let mut c = digimon("RAVE", "Ravemon", CardColor::Purple, 6, 12, &["Cyborg"]);
            c.evo_costs = vec![digimon_engine::card_data::EvoCost {
                card_color: CardColor::Purple as u8,
                level: 5,
                memory_cost: 4,
            }];
            c
        })
        .add_card(digimon(
            "BIRD5",
            "Birdy",
            CardColor::Purple,
            4,
            5,
            &["Mysterious Bird"],
        ))
        .add_card(digimon(
            "BIRD6",
            "Big Birdy",
            CardColor::Purple,
            5,
            6,
            &["Avian"],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_076_wd_deletes_then_pays_for_opponent_discard() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    let _ = r.pending_selection_view().expect("delete pick");
    pick_first(&mut r, 0); // the only Lv.4
    let v = r
        .pending_selection_view()
        .expect("optional Tamer pick (cost)");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().selecting_player,
        1,
        "opponent chooses"
    );
    pick_hand(&mut r, 1, "JUNK");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
    assert_eq!(sources(&r, t), 0);
    assert_eq!(r.hand_size(1), 0);
}

#[test]
fn bt26_076_wd_declining_cost_skips_discard() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP4", Some(0));
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    pick_first(&mut r, 0);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert_eq!(r.hand_size(1), 1, "no discard without the cost");
}

#[test]
fn bt26_076_opponent_hand_trash_digivolves_from_trash() {
    let mut r = setup();
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "RAVE");
    push_hand(&mut r, 1, "JUNK");
    // Peckmon's inherited [On Deletion] makes the opponent trash from hand.
    let carrier = r.place_stack(0, &["BT26-072", "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    pick_hand(&mut r, 1, "JUNK");
    r.accept_optional_trigger()
        .expect("reactive digivolve offered (yes/no)");
    pick_first(&mut r, 0); // RAVE from trash
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[0].battle_area[crow.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "RAVE"
    );
    assert_eq!(r.memory(), 5 - 3, "circle cost 4 reduced by 1");
    assert!(trash_ids(&r, 0).is_empty(), "RAVE left the trash");
}

#[test]
fn bt26_076_reactive_needs_a_trash_candidate() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 1, "JUNK");
    let carrier = r.place_stack(0, &["BT26-072", "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    pick_hand(&mut r, 1, "JUNK");
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "no [Ravemon]/[DATA SQUAD] in trash ⇒ no trigger"
    );
}

/// §15-8-3-2: source-trash reactions wait for the causing effect to finish,
/// including the opponent's discard selection. DCGO
/// BT26/Purple/BT26_076.cs:109-127 runs the Tamer trash and then the discard
/// inside one `ActivateCoroutine`; the stacked reaction resolves after it.
/// This test used to pin the reaction mid-effect (the inline drain and the
/// G-DSL-TAIL-CLOBBERS-INLINE-OBSERVER-SELECTION park;
/// G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS).
#[test]
fn bt26_076_self_caused_tamer_trash_waits_for_opponent_discard() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "RAVE");
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    pick_first(&mut r, 0); // Tamer (cost) — no delete target exists
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().selecting_player,
        1,
        "finish Crowmon's effect before activating source-trash reactions"
    );
    assert_eq!(field_ids(&r, 0).last().map(String::as_str), Some(CARD_ID));
    // The deferred queue and parked VM must survive cloning for search.
    r.game = r.game.clone();
    pick_hand(&mut r, 1, "JUNK");
    // The Tamer trash and the hand trash each trigger the [Your Turn]
    // reaction (§15-4-2-4); both entries pend in ONE pool once the
    // [When Digivolving] has finished (§15-4-3-2).
    assert_eq!(
        trigger_order_cards(&r),
        vec!["BT26-076", "BT26-076"],
        "both reactions pend together"
    );
    choose_trigger(&mut r, "BT26-076"); // order one of the two entries first
    r.accept_optional_trigger()
        .expect("its optional digivolve: yes");
    pick_first(&mut r, 0); // RAVE from the trash
    let _ = r.auto_resolve(); // the other entry: [Once Per Turn] already used
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.hand_size(1), 0);
    assert_eq!(field_ids(&r, 0).last().map(String::as_str), Some("RAVE"));
}

/// Card ids of the pending `TriggerOrder` branches (labels are
/// `"<card_id> slot <n> (...)"`), sorted.
fn trigger_order_cards(r: &DebugRunner) -> Vec<String> {
    let sel = r.pending_selection().expect("a TriggerOrder prompt");
    assert_eq!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
    let mut cards: Vec<String> = sel
        .effect_choices
        .as_ref()
        .expect("TriggerOrder branches")
        .iter()
        .map(|c| c.label.split(" slot ").next().unwrap_or(&c.label).to_string())
        .collect();
    cards.sort();
    cards
}

/// Resolve the pending `TriggerOrder` by picking a `card_id` branch.
fn choose_trigger(r: &mut DebugRunner, card_id: &str) {
    let (player, action) = {
        let sel = r.pending_selection().expect("a TriggerOrder prompt");
        assert_eq!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
        let choice = sel
            .effect_choices
            .as_ref()
            .expect("TriggerOrder branches")
            .iter()
            .find(|c| c.label.starts_with(&format!("{card_id} ")))
            .unwrap_or_else(|| panic!("no {card_id} branch: {sel:?}"));
        (sel.selecting_player, choice.action_id)
    };
    r.execute_action(player, action).unwrap();
}

/// §15-4-3-2 "If multiple effects are triggered before a single rule or effect
/// is resolved, they all trigger simultaneously"; §15-4-3-5-1 the turn player
/// orders them. Crowmon's [When Digivolving] trashes Keenan Crier
/// (BT26-094)'s face-down card (event 1) and makes the opponent trash a hand
/// card (event 2); Keenan's reaction and Crowmon's own [Your Turn] reaction
/// each trigger on each event (§15-4-2-4) -- four triggers of ONE effect's
/// resolution, offered as ONE pool (DCGO measured one 4-entry
/// `MultipleSkills`: the BT26-076-effect2 oracle abort,
/// `qa/qa-reports/exam-verdicts/BT26-076.json`). Both Crowmon entries are
/// offered; its [Once Per Turn] is checked at activation (DCGO
/// `timesPerTurn`). G-ENGINE-MULTI-EVENT-TRIGGERS-SPLIT-INTO-SUCCESSIVE-POOLS.
///
/// The parked 4-entry `TriggerOrder` clones faithfully for search: two clones
/// resolve it in different orders (Crowmon first / Keenan first) and both
/// complete to the same board.
#[test]
fn bt26_076_tamer_trash_and_hand_trash_triggers_pend_in_one_pool() {
    let mut r = setup();
    let keenan = tamer_with_face_down(&mut r, 0, "BT26-094", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "RAVE");
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    pick_first(&mut r, 0); // Keenan (cost) — no delete target exists
    pick_hand(&mut r, 1, "JUNK");
    assert_eq!(
        trigger_order_cards(&r),
        vec!["BT26-076", "BT26-076", "BT26-094", "BT26-094"],
        "both events' triggers pend in one pool"
    );

    for first in ["BT26-076", "BT26-094"] {
        let mut c = DebugRunner::wrap(r.game.clone());
        choose_trigger(&mut c, first);
        drain_first(&mut c);
        assert!(c.game.pending_selection.is_none(), "{first} first: settled");
        assert_eq!(
            field_ids(&c, 0).last().map(String::as_str),
            Some("RAVE"),
            "{first} first: Crowmon digivolved once"
        );
        assert!(
            c.game.players[0].battle_area[keenan.index as usize].is_suspended,
            "{first} first: Keenan activated (suspended)"
        );
        assert_eq!(c.memory(), 5 - 3, "{first} first: one digivolve paid");
    }
}

#[test]
fn bt26_076_inherited_on_deletion_plays_bird_from_trash() {
    let mut r = setup();
    push_trash(&mut r, 0, "BIRD5");
    push_trash(&mut r, 0, "BIRD6"); // cost 6 — too expensive
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    let v = r.pending_selection_view().expect("optional trash pick");
    assert!(v.is_optional);
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        1
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"BIRD5".to_string()));
}
