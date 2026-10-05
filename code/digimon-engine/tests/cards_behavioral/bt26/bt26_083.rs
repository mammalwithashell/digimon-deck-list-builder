//! BT26-083 Junomon: Hysteric Mode — Lv.7 Purple/Yellow.
//!
//! <Rush> <Piercing> <Execute> <Decode ([Junomon]/Lv.5 or lower w/[Iliad])>
//! [On Play] [When Digivolving] Trash all of your security cards. For each card
//! this effect trashed, delete 1 of your opponent's Digimon. Then,
//! <Recovery +3>.
//! [On Deletion] Give all of your opponent's Digimon <Security A. -1> until
//! their turn ends. Assembly -4: [Junomon].
//!
//! DCGO: BT26/Yellow/BT26_083.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};

const CARD_ID: &str = "BT26-083";

fn setup_with(sec: usize) -> DebugRunner {
    let secs = vec!["FILLER"; sec];
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-083")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("JUNO", "Junomon", CardColor::Yellow, 6, 9, &["TS"]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 6])
        .security(0, &secs)
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn picks(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&x| x != digimon_engine::action::space::PASS)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn bt26_083_keywords_alt_paths_and_assembly() {
    let r = setup_with(3);
    let card = r.compiled_card(CARD_ID).unwrap();
    let effs = format!("{:?}", card.effects);
    for kw in ["Rush", "Piercing", "Execute"] {
        assert!(effs.contains(kw), "missing {kw}");
    }
    let alts = format!("{:?}", card.alt_paths);
    assert!(alts.contains("Junomon") && alts.contains("TS"), "{alts}");
}

#[test]
fn bt26_083_on_play_trashes_all_deletes_that_many_then_recovers_three() {
    let mut r = setup_with(3);
    let j = r.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..4 {
        r.place_on_field(1, "OPP", Some(0));
    }
    fire(&mut r, EffectTiming::OnPlay, j);
    // Exactly 3 mandatory picks (3 security cards trashed).
    for i in 0..3 {
        let v = r.pending_selection_view().expect("delete pick");
        assert!(!v.is_optional, "pick {i} is mandatory");
        let p = picks(&r);
        r.execute_action(0, p[0]).unwrap();
    }
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1).len(), 1, "3 of 4 deleted");
    assert_eq!(r.security_count(0), 3, "0 + Recovery +3");
}

#[test]
fn bt26_083_delete_count_capped_by_opponent_digimon() {
    let mut r = setup_with(2);
    let j = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, j);
    while !picks(&r).is_empty() {
        let p = picks(&r);
        r.execute_action(0, p[0]).unwrap();
    }
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert_eq!(r.security_count(0), 3);
}

#[test]
fn bt26_083_no_security_deletes_nothing_but_recovers() {
    let mut r = setup_with(0);
    let j = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnPlay, j);
    assert!(
        r.game.pending_selection.is_none(),
        "nothing trashed ⇒ no deletes"
    );
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP".to_string()]);
    assert_eq!(r.security_count(0), 3);
}

fn first_non_pass(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("pending selection");
    let a = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("non-PASS action");
    r.execute_action(v.selecting_player, a).unwrap();
}

/// G-ENGINE-DECODE-ON-PLAY-BEFORE-CARRIER-LEAVES: <Decode> is an immediate
/// (replacement-timing) effect (§16-35, keyword-semantics.md). The Digimon it
/// plays has its [On Play] TRIGGERED, which must wait until the Decode
/// processing — including the carrier's leave — completes. So when BT25-044
/// Junomon's [On Play] "place 1 other Digimon as the top security card" pick
/// opens, BT26-083 must already be in the trash and must not be a candidate.
#[test]
fn bt26_083_decode_played_on_play_waits_until_carrier_has_left() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-083")
        .dsl_card("BT25-044")
        .expect("BT25-044")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("MINE", "Mine", CardColor::Yellow, 4, 5, &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    // Carrier at slot 0, another own Digimon at slot 1: the Decode-played
    // Junomon lands at slot 2 and must shift to slot 1 when the carrier goes.
    let hm = r.place_stack(0, &["BT25-044", CARD_ID]);
    r.place_on_field(0, "MINE", Some(0));
    r.place_on_field(1, "OPP", Some(0));

    r.game.delete_permanent_with_cause(
        hm,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    // Decode gate -> accept; material pick -> Junomon.
    first_non_pass(&mut r);
    first_non_pass(&mut r);
    // BT26-083's [On Deletion] and Junomon's [On Play] both triggered during
    // the Decode processing, so they wait together and the turn player orders
    // them (§15-4-3-5) — take them in queue order.
    while r
        .pending_selection()
        .is_some_and(|p| matches!(p.kind, digimon_engine::selection::SelectionKind::TriggerOrder))
    {
        first_non_pass(&mut r);
    }

    let v = r
        .pending_selection_view()
        .expect("Junomon [On Play] optional pick");
    assert!(
        !field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "the Decode carrier must have left before the played Digimon's [On Play] resolves; \
         field = {:?}, prompt = {}",
        field_ids(&r, 0),
        v.prompt
    );
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(
        field_ids(&r, 0),
        vec!["MINE".to_string(), "BT25-044".to_string()]
    );
    // Candidates: MINE + OPP only — the departed carrier is not offered.
    assert_eq!(picks(&r).len(), 2, "{:?}", v.valid_action_ids);
    // Decline ("By placing" is optional): nothing pending may hit Junomon —
    // the carrier's deletion already completed against the right slot.
    r.execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 0),
        vec!["MINE".to_string(), "BT25-044".to_string()]
    );
    assert_eq!(field_ids(&r, 1), vec!["OPP".to_string()]);
}

#[test]
fn bt26_083_on_deletion_opponent_digimon_security_minus_one() {
    let mut r = setup_with(3);
    let j = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    r.game.delete_permanents_batch(
        vec![j],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    assert!(r.modifiers().has(a, ModifierType::SecurityAttackChange));
    // Continuous: a Digimon they play later is covered too.
    let b = r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.modifiers().has(b, ModifierType::SecurityAttackChange));
}
