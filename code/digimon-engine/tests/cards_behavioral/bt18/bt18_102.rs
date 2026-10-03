//! BT18-102 Omnimon Zwart Defeat (spec lives in `cards/_examples/BT18-102.yaml`).
//!
//! # Printed text (official Bandai DB bundle, abbreviated)
//! [When Attacking] By placing up to 5 Tamer cards from this Digimon's
//! digivolution cards as your bottom security cards, for each one, trash your
//! opponent's top security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/White/BT18_102.cs — "When Attacking
//! Trash Security": select up to min(5, #Tamer sources) Tamer digivolution
//! cards, `AddSecurityCard(toTop: false)` each, then `IDestroySecurity` on the
//! opponent for `cardSources.Count` cards.
//!
//! # W1 regression guard
//! The per-placed-card loop used to be `for_each: { over: { binding: placed } }`.
//! `binding` is not a predicate key, so it was silently dropped: `for_each`
//! scanned an EMPTY predicate (every permanent), bound each PERMANENT as `m`,
//! the `place_on_security { card: m }` no-op'd (a permanent is not a card
//! source) and the opponent lost one security card PER PERMANENT ON THE FIELD
//! instead of one per Tamer placed. Now `per_selected` over the selection.

#![allow(dead_code)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT18-102";

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::White];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT18-102 in embedded DSL pack")
        .add_card(tamer("TAMER-A"))
        .add_card(tamer("TAMER-B"))
        .add_card(digimon("SRC-DIGI"))
        .add_card(digimon("ALLY"))
        .add_card(make_test_card("SEC", "SEC"))
        .add_card(make_test_card("FILL", "FILL"))
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC", "SEC", "SEC"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn drive_accept_all(r: &mut DebugRunner) {
    let mut guard = 0;
    while r.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 64, "selection chain did not terminate");
        let actions: Vec<u16> = r
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .collect();
        let selecting = r.pending_selection().unwrap().selecting_player;
        if let Some(a) = actions.iter().copied().find(|a| *a != PASS) {
            if r.execute_action(selecting, a).is_ok() {
                continue;
            }
        }
        let _ = r.execute_action(selecting, PASS);
    }
    let _ = r.auto_resolve();
}

fn security_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .security
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Two Tamer sources placed → own security +2 (the two Tamers), opponent
/// security −2 — independent of how many other permanents are on the field.
#[test]
fn bt18_102_when_attacking_places_tamer_sources_and_trashes_one_opp_security_each() {
    let mut r = runner();
    let omni = r.place_stack(0, &["TAMER-A", "SRC-DIGI", "TAMER-B", CARD_ID]);
    // Extra permanents on BOTH sides: the old `for_each {}` scan iterated these.
    r.place_on_field(0, "ALLY", None);
    r.place_on_field(1, "ALLY", None);
    r.place_on_field(1, "ALLY", None);

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(omni));
    r.game.drain_effect_queue();
    drive_accept_all(&mut r);

    let own = security_ids(&r, 0);
    assert_eq!(own.len(), 4, "both Tamer sources were placed into own security: {own:?}");
    assert!(own.iter().any(|c| c == "TAMER-A") && own.iter().any(|c| c == "TAMER-B"));
    assert_eq!(
        r.security_count(1),
        3,
        "opponent trashes exactly one top security card per Tamer placed (5 → 3)"
    );
    let omni_now = PermanentHandle {
        player: 0,
        index: omni.index,
    };
    let sources: Vec<String> = r.game.players[0].battle_area[omni_now.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        !sources.iter().any(|c| c.starts_with("TAMER")),
        "the Tamer sources left the stack: {sources:?}"
    );
}
