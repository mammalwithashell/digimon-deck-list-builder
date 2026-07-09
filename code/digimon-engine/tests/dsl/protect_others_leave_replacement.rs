//! G-DSL-PROTECT-OTHER-BY-SELF-DELETE + EX7-048 clause 2 — board-wide
//! protect-OTHERS leave replacement.
//!
//! Two driver shapes:
//!
//! ## EX7-048 Gundramon (clause 2)
//! "[All Turns] When your [Three Musketeers] trait Digimon would leave the
//! battle area other than by your effects, by trashing 1 Option card from this
//! Digimon's digivolution cards, they don't leave." (DCGO `EX7_048.cs`
//! `WhenRemoveField`: `CanTriggerWhenPermanentRemoveField(IsOwnerPermanent 3M)`
//! + `!IsByEffect(IsOwnerEffect)` → `SelectPermanent(willBeRemoveField)` →
//! `SelectCard(DigivolutionCards IsOption)` → trash → `willBeRemoveField = false`.)
//! Cost = trash 1 Option from the CARRIER'S digivolution cards (single-carrier
//! variant of the Gap-A picker, scoped to `source`).
//!
//! ## BT25-039 Sirenmon (clause 3)
//! "[All Turns] When any of your OTHER [Shaman]/[Iliad] Digimon or Tamers would
//! leave the battle area other than by your effects, by deleting this Digimon,
//! they don't leave." (DCGO `BT25_039.cs` `WhenRemoveField`: `PermanentCondition`
//! (`!= self` + owner + Digimon|Tamer + Shaman|Iliad) + `!IsByEffect(IsOwnerEffect)`
//! → `DeletePeremanentAndProcessAccordingToResult(self)` → cancel the leave.)
//! Cost = `delete_self`; subject filter carries `other: true`.
//!
//! Both fire at `when_would_leave_battle_area` with an `active_when` that reads a
//! CROSS-PERMANENT subject (`replacement_subject_is_mine` + trait/kind filter,
//! and for BT25-039 `other: true`), gated by `none_of: [replacement_cause:
//! own_effect]` ("other than by your effects"). The engine's existing
//! `can_match_cross_permanent_subject` path (lower_replacement.rs) already lets
//! the carrier's replacement match a non-self subject; this file proves the two
//! COST variants gate/prevent correctly, and the cause gate's positive+negative.
//!
//! Does NOT author EX7-048 / BT25-039 (each has other clauses).

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

fn digimon_with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(4);
    d.dp = Some(4000);
    d.traits = traits.iter().map(|t| t.to_string()).collect();
    d
}

fn tamer_with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Tamer;
    d.level = None;
    d.dp = None;
    d.traits = traits.iter().map(|t| t.to_string()).collect();
    d
}

fn option_card(id: &str) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Option;
    d.level = None;
    d.dp = None;
    d
}

// ══════════════════════════════════════════════════════════════════════════════
// BT25-039 shape — protect OTHER own [Shaman]/[Iliad] Digimon/Tamer by DELETING
// this Digimon.
// ══════════════════════════════════════════════════════════════════════════════

/// The BT25-039 carrier YAML: `delete_self` cost, subject = OTHER own
/// Shaman/Iliad Digimon or Tamer, "other than by your effects".
const BT25_039_CARRIER: &str = r#"
card: TST-PROTECT-DELETE
name: ProtectOthersByDelete
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 4000
traits: [Shaman]
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - other: true
        - any_of:
            - all_of: [ { kind: digimon } ]
            - all_of: [ { kind: tamer } ]
        - any_of:
            - trait_has: Shaman
            - trait_has: Iliad
        - none_of:
            - replacement_cause: own_effect
    cost:
      delete_self: true
    outcome: prevent
"#;

fn bt25_039_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT25_039_CARRIER)
        .unwrap()
        .add_card(digimon_with_traits("SUBJECT-SHAMAN", &["Shaman"]))
        .add_card(tamer_with_traits("SUBJECT-TAMER", &["Iliad"]))
        .add_card(digimon_with_traits("SUBJECT-OFF", &["OffTrait"]))
        .start()
}

#[test]
fn bt25_039_accept_deletes_self_and_protects_the_other_subject() {
    let mut runner = bt25_039_runner();
    let carrier = runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-SHAMAN", Some(0));

    // An opponent effect would delete the OTHER own Shaman Digimon.
    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    // The optional replacement parks an accept prompt (offered to the carrier's
    // controller). Accept → pay `delete_self` → the subject is protected.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("protect-others replacement offers an accept prompt");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");

    assert!(runner.game.pending_selection.is_none(), "resolved");
    // Subject STAYED (protected). Carrier is GONE (the delete-self cost).
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-SHAMAN"),
        "the OTHER Shaman Digimon is protected (stays)"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TST-PROTECT-DELETE"),
        "the carrier is deleted (delete_self cost paid)"
    );
    let _ = carrier;
}

#[test]
fn bt25_039_decline_lets_the_subject_leave() {
    let mut runner = bt25_039_runner();
    runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-SHAMAN", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    let sp = runner
        .game
        .pending_selection
        .as_ref()
        .expect("accept prompt installs")
        .selecting_player;
    runner
        .game
        .resolve_selection(sp, digimon_engine::action::space::PASS)
        .expect("decline");

    assert!(runner.game.pending_selection.is_none(), "resolved");
    // Declined → subject LEAVES, carrier UNTOUCHED.
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-SHAMAN"),
        "the subject leaves when the cost is declined"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TST-PROTECT-DELETE"),
        "the carrier survives when the cost is declined"
    );
}

#[test]
fn bt25_039_protects_an_other_tamer_subject() {
    let mut runner = bt25_039_runner();
    runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("protect prompt for an OTHER Iliad Tamer");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-TAMER"),
        "the OTHER Iliad Tamer is protected"
    );
}

#[test]
fn bt25_039_cause_gate_own_effect_does_not_fire() {
    let mut runner = bt25_039_runner();
    runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-SHAMAN", Some(0));

    // The OWNER'S OWN effect deletes the subject → "other than by your effects"
    // is FALSE → the replacement must NOT fire, no accept prompt, subject leaves.
    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "own-effect leave does NOT offer the protect replacement (cause gate)"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-SHAMAN"),
        "the subject leaves (own-effect cause is not protected)"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TST-PROTECT-DELETE"),
        "the carrier is untouched (cost never paid)"
    );
}

#[test]
fn bt25_039_off_trait_subject_does_not_fire() {
    let mut runner = bt25_039_runner();
    runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-OFF", Some(0));

    // A non-Shaman/non-Iliad own Digimon leaving → subject filter fails → no
    // replacement.
    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "an off-trait subject does not match the protect filter"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-OFF"),
        "the off-trait subject leaves"
    );
}

#[test]
fn bt25_039_other_gate_self_leave_does_not_fire() {
    // The carrier itself leaving must NOT trigger its own OTHER-scoped protect
    // (the `other: true` gate excludes the carrier as subject).
    let mut runner = bt25_039_runner();
    let carrier = runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "the carrier's own leave does not fire the OTHER-scoped protect"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TST-PROTECT-DELETE"),
        "the carrier leaves (it does not protect itself under `other: true`)"
    );
}

#[test]
fn bt25_039_clone_safe_mid_park() {
    let mut runner = bt25_039_runner();
    runner.place_on_field(0, "TST-PROTECT-DELETE", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-SHAMAN", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert!(
        runner.game.pending_selection.is_some(),
        "parked accept prompt"
    );

    // Clone the parked game and drive the clone to accept — no closure-park panic.
    let mut cloned = runner.game.clone();
    let (aid, sp) = {
        let p = cloned
            .pending_selection
            .as_ref()
            .expect("clone kept the prompt");
        (p.valid_action_ids[0], p.selecting_player)
    };
    cloned.resolve_selection(sp, aid).expect("clone accepts");
    assert!(cloned.pending_selection.is_none(), "clone resolved");
    assert!(
        cloned.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&cloned.card_data) == "SUBJECT-SHAMAN"),
        "clone protected the subject via the resumable VM"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// EX7-048 shape — protect own [Three Musketeers] Digimon by TRASHING 1 Option
// from THIS Digimon's digivolution cards.
// ══════════════════════════════════════════════════════════════════════════════

/// The EX7-048 carrier YAML: `trash_option_from_own_digivolution_cards` cost,
/// subject = own Three Musketeers Digimon, "other than by your effects". Note the
/// printed card does NOT say "other" — the carrier can protect itself too — so
/// this filter omits `other: true`.
const EX7_048_CARRIER: &str = r#"
card: TST-PROTECT-TRASH
name: ProtectOthersByTrash
kind: digimon
level: 4
color: [black]
cost: 4
dp: 4000
traits: [Three Musketeers]
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: Three Musketeers
        - none_of:
            - replacement_cause: own_effect
    cost:
      trash_option_from_own_digivolution_cards: true
    outcome: prevent
"#;

/// Build a runner + place the EX7-048 carrier with an Option in its
/// digivolution stack. Returns (runner, carrier handle). The carrier is built
/// via `place_stack` so it carries `OPT-UNDER` as a digivolution source.
fn ex7_048_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(EX7_048_CARRIER)
        .unwrap()
        .add_card(option_card("OPT-UNDER"))
        .add_card(digimon_with_traits("SUBJECT-3M", &["Three Musketeers"]))
        .add_card(digimon_with_traits("SUBJECT-OFF", &["OffTrait"]))
        .start()
}

/// Seat the carrier as a stack with an Option digivolution source under it.
fn place_ex7_048_carrier(runner: &mut DebugRunner) -> PermanentHandle {
    // bottom = OPT-UNDER (Option source), top = the carrier.
    runner.place_stack(0, &["OPT-UNDER", "TST-PROTECT-TRASH"])
}

#[test]
fn ex7_048_accept_trashes_carrier_option_and_protects_the_subject() {
    let mut runner = ex7_048_runner();
    let carrier = place_ex7_048_carrier(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    // Accept the outer replacement prompt → the cost installs a NESTED Option
    // pick over the CARRIER's digivolution cards.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("protect-by-trash replacement offers an accept prompt");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");

    // Nested: pick which Option of the carrier's digivolution cards to trash.
    let (opt_id, opt_sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("nested Option-of-carrier-digivolution pick installs");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner
        .game
        .resolve_selection(opt_sp, opt_id)
        .expect("trash the Option");

    assert!(runner.game.pending_selection.is_none(), "resolved");
    // The carrier's digivolution Option went to trash; the carrier stays.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OPT-UNDER"),
        "the carrier's digivolution Option was trashed as the cost"
    );
    let carrier_perm = &runner.game.players[0].battle_area[carrier.index as usize];
    assert_eq!(
        carrier_perm.card_sources.len(),
        1,
        "the carrier lost its Option source (2 → 1) but stays on field"
    );
    // The subject is protected (stays).
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-3M"),
        "the Three Musketeers subject is protected"
    );
}

#[test]
fn ex7_048_unpayable_when_carrier_has_no_option_source_not_offered() {
    let mut runner = ex7_048_runner();
    // Seat the carrier as a lone top card — NO Option in its digivolution cards.
    let _carrier = runner.place_on_field(0, "TST-PROTECT-TRASH", Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    // The cost is unpayable (carrier has no Option digivolution source) → the
    // optional accept prompt is NOT offered → the subject leaves.
    assert!(
        runner.game.pending_selection.is_none(),
        "unpayable trash cost is not offered"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-3M"),
        "the subject leaves when the carrier cannot pay"
    );
}

#[test]
fn ex7_048_cause_gate_own_effect_does_not_fire() {
    let mut runner = ex7_048_runner();
    place_ex7_048_carrier(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "own-effect leave does NOT offer the protect-by-trash replacement"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-3M"),
        "the subject leaves (own-effect cause)"
    );
}

#[test]
fn ex7_048_battle_cause_fires_and_protects() {
    // "other than by your effects" includes BATTLE — a battle leave IS protected.
    let mut runner = ex7_048_runner();
    place_ex7_048_carrier(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::Battle);

    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("battle-cause leave IS protectable (not by your effects)");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");
    let (opt_id, opt_sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("nested Option pick");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner
        .game
        .resolve_selection(opt_sp, opt_id)
        .expect("trash");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUBJECT-3M"),
        "the battle-leaving subject is protected"
    );
}
