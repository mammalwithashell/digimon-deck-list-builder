//! Play-from-hand INTERACTIVE cost reducer whose `pay_cost` deletes a chosen
//! own Digimon — `G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST` /
//! `G-DSL-BEFORE-PAY-COST-DELETE-OWN-FOR-VARIABLE-REDUCTION`.
//!
//! Driver cards:
//!   - BT18-073 Machinedramon: "When this card would be played, by deleting 1 of
//!     your [Composite]-trait Digimon, reduce the play cost by 4." (FIXED −4)
//!   - BT13-083 Gizmon:AT: "When you would play this card, by deleting 1 of your
//!     level 3 Digimon, reduce the play cost by 4." (FIXED −4)
//!   - BT13-103 Akihiro Kurata: "When you would play a card with [Belphemon] in
//!     its name, by deleting 1 of your [Gizmon]-name Digimon, reduce the play
//!     cost by the play cost of the deleted Digimon." (VARIABLE = deleted cost)
//!
//! The pay_cost is INTERACTIVE — `select_own_permanent` (the "by deleting 1 of
//! your ... Digimon" pick) followed by `delete_permanent`. It PARKS on the pick.
//! The play-from-hand cost-reduction chain must:
//!   - offer the reducer (optional accept/decline — "by deleting" is a cost);
//!   - on accept, run the parking pay_cost, credit the reduction ONLY when the
//!     delete actually happens, and re-enter the play at the reduced cost;
//!   - on decline, play at full cost with no delete.
//!
//! Every fixture here is authored as INLINE DSL YAML (`from_dsl_yaml`) so the
//! parked selection is resume-driven (`pending_selection_resume`), NOT a
//! closure park — the whole flow is `Game: Clone`-safe (rule 28). The
//! clone-mid-park test proves a forked `Game` resolves the parked delete
//! identically.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Inline DSL fixtures ──────────────────────────────────────────────────────

/// A field Tamer hosting a FIXED-amount interactive play-cost reducer, modeled
/// on BT18-073 / BT13-083: "[Your Turn] When any of your [Composite] cards would
/// be played, by deleting 1 of your [Composite]-trait Digimon, reduce the play
/// cost by 4." Optional ("by deleting …" is a cost the player may decline).
const FIXED_REDUCER_YAML: &str = r#"
card: TEST-FIXCUT
name: Fixed Delete Cutter
kind: tamer
color: [black]
cost: 3
effects:
  - kind: cost_reduction
    active_when: { your_turn: true }
    when_any_ally_played: { trait_has: "Composite" }
    optional: true
    amount: 4
    pay_cost:
      - select_own_permanent:
          bind_as: sacrifice
          filter:
            all_of:
              - kind: digimon
              - trait_has: "Composite"
          prompt: "Delete 1 of your [Composite] Digimon to reduce the play cost by 4"
      - delete_permanent: { target: sacrifice }
    summary: "[Your Turn] By deleting 1 of your [Composite] Digimon, reduce a played [Composite] card's cost by 4"
"#;

/// A cost-8 [Composite] Digimon — the played card whose cost the reducer cuts.
fn composite_digimon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["Composite".to_string()];
    c
}

/// A cost-4 [Composite] Digimon — the deletion sacrifice candidate.
fn composite_sacrifice(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["Composite".to_string()];
    c
}

/// A non-[Composite] Digimon — must NOT be a sacrifice candidate.
fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ═══════════════════════════════════════════════════════════════════════════════
// FIXED amount (BT18-073 / BT13-083)
// ═══════════════════════════════════════════════════════════════════════════════

/// Accept + delete: playing a cost-8 [Composite] card with a [Composite]
/// sacrifice on field offers the reducer; accepting parks the delete pick;
/// resolving it deletes the sacrifice and credits −4 (effective cost 4).
#[test]
fn fixed_play_reducer_deletes_and_credits_reduction() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXED_REDUCER_YAML)
        .expect("TEST-FIXCUT YAML parses")
        .add_card(composite_digimon("PLAYED", 8))
        .add_card(composite_sacrifice("SAC", 4))
        .add_card(make_filler("FILL"))
        .hand(0, &["PLAYED"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-FIXCUT", Some(0));
    let sac = runner.place_on_field(0, "SAC", Some(0));

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    // Play the cost-8 [Composite] Digimon → installs the accept/decline prompt.
    assert!(
        runner.play(0, 0).is_none(),
        "the interactive reducer parks the play until its cost prompt resolves"
    );
    assert!(
        runner.pending_selection().is_some(),
        "playing a [Composite] card installs the cost-reduction accept prompt"
    );
    assert!(runner.pending_is_optional(), "the −4 reducer is optional");
    assert_eq!(
        runner.memory(),
        mem_before,
        "no cost paid before the reducer prompt resolves"
    );

    runner
        .accept_optional_trigger()
        .expect("accept the −4 cost reduction");

    // The pay_cost parks on the [Composite] sacrifice pick (own field).
    let pending = runner
        .pending_selection()
        .expect("the delete pick must park");
    assert!(matches!(pending.kind, SelectionKind::OwnField));

    // Resolve the single-eligible sacrifice pick.
    let _ = runner.auto_resolve();
    assert!(
        runner.pending_selection().is_none(),
        "the play completes after the delete resolves"
    );

    // SAC deleted (moved to trash) as the cost.
    let sac_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SAC");
    assert!(!sac_present, "the [Composite] sacrifice must be deleted");
    let _ = sac;
    assert!(
        runner.trash_size(0) > trash_before,
        "the deleted sacrifice must land in trash"
    );

    // cost 8 − 4 = 4 paid.
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "cost 8 reduced by 4 → 4 memory paid (the reduction is credited even \
         though the pay_cost parked on the delete pick)"
    );
}

/// Decline: declining the reducer prompt plays the card at full cost (8) and
/// deletes nothing.
#[test]
fn fixed_play_reducer_decline_pays_full_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXED_REDUCER_YAML)
        .expect("TEST-FIXCUT YAML parses")
        .add_card(composite_digimon("PLAYED", 8))
        .add_card(composite_sacrifice("SAC", 4))
        .add_card(make_filler("FILL"))
        .hand(0, &["PLAYED"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-FIXCUT", Some(0));
    runner.place_on_field(0, "SAC", Some(0));

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    assert!(runner.pending_selection().is_some());
    runner
        .decline_optional_trigger()
        .expect("decline the reducer");
    let _ = runner.auto_resolve();

    let sac_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SAC");
    assert!(sac_present, "declining deletes nothing");
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "nothing trashed on decline"
    );
    assert_eq!(
        mem_before - runner.memory(),
        8,
        "declining pays the full cost 8"
    );
}

/// No sacrifice on field: the reducer never offers (its `pay_cost` has no
/// eligible target), so the card plays at full cost.
#[test]
fn fixed_play_reducer_inactive_without_sacrifice() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXED_REDUCER_YAML)
        .expect("TEST-FIXCUT YAML parses")
        .add_card(composite_digimon("PLAYED", 8))
        .add_card(plain_digimon("PLAIN"))
        .add_card(make_filler("FILL"))
        .hand(0, &["PLAYED"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-FIXCUT", Some(0));
    // Only a NON-[Composite] Digimon on field → no eligible sacrifice.
    runner.place_on_field(0, "PLAIN", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        mem_before - runner.memory(),
        8,
        "no eligible [Composite] sacrifice → full cost 8, no prompt/reduction"
    );
}

/// Clone-mid-park: a forked `Game` holding the parked delete pick must resolve
/// it identically (rule 28 clone-safety — the pay_cost's selection lives on the
/// resumable data VM, not a closure park).
#[test]
fn fixed_play_reducer_clone_resolves_park_identically() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXED_REDUCER_YAML)
        .expect("TEST-FIXCUT YAML parses")
        .add_card(composite_digimon("PLAYED", 8))
        .add_card(composite_sacrifice("SAC", 4))
        .add_card(make_filler("FILL"))
        .hand(0, &["PLAYED"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-FIXCUT", Some(0));
    runner.place_on_field(0, "SAC", Some(0));

    runner.play(0, 0);
    runner
        .accept_optional_trigger()
        .expect("accept the reducer");

    // The delete pick is parked and must be resume-driven before cloning.
    let action = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("delete pick parked");
        assert!(
            runner.game.pending_selection_resume.is_some(),
            "the parked delete pick must be resume-driven (data VM) before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, action)
        .expect("cloned delete pick resolves");
    assert!(
        clone.pending_selection.is_none(),
        "cloned play completes after the delete resolves"
    );
    let clone_mem = clone.memory;

    // The original still holds the parked pick.
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone must leave the original park intact"
    );
    runner
        .game
        .resolve_selection(0, action)
        .expect("original delete pick resolves");

    assert_eq!(
        runner.game.memory, clone_mem,
        "clone and original must replay the same reduced cost"
    );
    // Both deleted the sacrifice.
    let orig_sac = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SAC");
    let clone_sac = clone.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&clone.card_data) == "SAC");
    assert_eq!(
        orig_sac, clone_sac,
        "clone and original must replay the same delete"
    );
    assert!(!orig_sac, "the sacrifice is deleted in both");
}

// ═══════════════════════════════════════════════════════════════════════════════
// FIXED amount, `when_playing_this` (the exact BT18-073 / BT13-083 trigger shape:
// the reducer is hosted ON the card being played — "When this card would be
// played, by deleting 1 of your ... Digimon, reduce the play cost by 4").
// ═══════════════════════════════════════════════════════════════════════════════

/// The played card ITSELF hosts the fixed −4 delete reducer (`when_playing_this`).
const SELF_REDUCER_YAML: &str = r#"
card: TEST-SELFCUT
name: Machinedramon-style Self Cutter
kind: digimon
color: [black]
cost: 8
level: 6
dp: 11000
traits: [Composite]
effects:
  - kind: cost_reduction
    when_playing_this: true
    optional: true
    amount: 4
    pay_cost:
      - select_own_permanent:
          bind_as: sacrifice
          filter:
            all_of:
              - kind: digimon
              - trait_has: "Composite"
          prompt: "Delete 1 of your [Composite] Digimon to reduce this card's play cost by 4"
      - delete_permanent: { target: sacrifice }
    summary: "When this card would be played, by deleting 1 of your [Composite] Digimon, reduce the play cost by 4"
"#;

/// `when_playing_this` (BT18-073/BT13-083 shape): playing the cost-8 self-reducer
/// with a [Composite] sacrifice on field offers the −4 reducer; accept deletes the
/// sacrifice and pays 4.
#[test]
fn self_hosted_play_reducer_deletes_and_credits_reduction() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SELF_REDUCER_YAML)
        .expect("TEST-SELFCUT YAML parses")
        .add_card(composite_sacrifice("SAC", 4))
        .add_card(make_filler("FILL"))
        .hand(0, &["TEST-SELFCUT"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "SAC", Some(0));

    let mem_before = runner.memory();
    assert!(
        runner.play(0, 0).is_none(),
        "the self-hosted interactive reducer parks the play"
    );
    assert!(
        runner.pending_selection().is_some(),
        "playing the self-reducer installs the accept prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the −4 reduction");
    let _ = runner.auto_resolve();

    let sac_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SAC");
    assert!(!sac_present, "the [Composite] sacrifice must be deleted");
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "cost 8 reduced by 4 → 4 paid (when_playing_this self-reducer)"
    );
}

/// `when_playing_this` negative: with no [Composite] sacrifice on field the
/// self-reducer is not offered — the card plays at full cost 8.
#[test]
fn self_hosted_play_reducer_inactive_without_sacrifice() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SELF_REDUCER_YAML)
        .expect("TEST-SELFCUT YAML parses")
        .add_card(plain_digimon("PLAIN"))
        .add_card(make_filler("FILL"))
        .hand(0, &["TEST-SELFCUT"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "PLAIN", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    assert_eq!(
        mem_before - runner.memory(),
        8,
        "no [Composite] sacrifice → self-reducer not offered, full cost 8 paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// VARIABLE amount = the deleted permanent's printed play cost (BT13-103)
// ═══════════════════════════════════════════════════════════════════════════════

/// A field Tamer hosting a VARIABLE-amount interactive play-cost reducer, modeled
/// on BT13-103 Akihiro Kurata: "[Your Turn] When you would play a [Belphemon]-name
/// card, by deleting 1 of your [Gizmon]-name Digimon, reduce the play cost by the
/// play cost of the deleted Digimon." The `amount` reads `binding_play_cost` of the
/// permanent chosen+deleted inside the pay_cost.
const VARIABLE_REDUCER_YAML: &str = r#"
card: TEST-VARCUT
name: Variable Delete Cutter
kind: tamer
color: [purple]
cost: 3
effects:
  - kind: cost_reduction
    active_when: { your_turn: true }
    when_any_ally_played: { name_contains: "Belphemon" }
    optional: true
    pay_cost:
      - select_own_permanent:
          bind_as: sacrifice
          filter:
            all_of:
              - kind: digimon
              - name_contains: "Gizmon"
          prompt: "Delete 1 of your [Gizmon] Digimon to reduce the play cost by its cost"
      - delete_for_cost_reduction: { target: sacrifice }
    summary: "[Your Turn] By deleting 1 of your [Gizmon] Digimon, reduce a played [Belphemon] card's cost by the deleted Digimon's cost"
"#;

/// A [Belphemon]-name Digimon — the played card whose cost the variable reducer
/// cuts.
fn belphemon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, "Belphemon: Rage Mode");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// A [Gizmon]-name Digimon (the deletion candidate) with printed play cost `cost`.
fn gizmon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, "Gizmon: AT");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// Variable: playing a cost-5 [Belphemon] card, deleting a cost-3 [Gizmon]
/// reduces the play cost by 3 (the deleted Digimon's printed cost) → effective 2.
#[test]
fn variable_play_reducer_reduces_by_deleted_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(VARIABLE_REDUCER_YAML)
        .expect("TEST-VARCUT YAML parses")
        .add_card(belphemon("BELPH", 5))
        .add_card(gizmon("GIZMON", 3))
        .add_card(make_filler("FILL"))
        .hand(0, &["BELPH"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-VARCUT", Some(0));
    runner.place_on_field(0, "GIZMON", Some(0));

    let mem_before = runner.memory();

    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "playing a [Belphemon] card offers the variable reducer"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the reducer");
    let _ = runner.auto_resolve();

    // GIZMON deleted as the cost.
    let gizmon_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "GIZMON");
    assert!(!gizmon_present, "GIZMON must be deleted as the cost");

    // Belphemon cost 5 reduced by deleted Gizmon's cost 3 → effective 2.
    assert_eq!(
        mem_before - runner.memory(),
        2,
        "Belphemon (cost 5) reduced by the deleted Gizmon's cost (3) → 2 paid"
    );
}

/// Variable amount tracks the SPECIFIC deleted permanent's cost: a cost-1 Gizmon
/// reduces by only 1 (not the same constant as the cost-3 case above).
#[test]
fn variable_play_reducer_amount_tracks_specific_deleted_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(VARIABLE_REDUCER_YAML)
        .expect("TEST-VARCUT YAML parses")
        .add_card(belphemon("BELPH", 5))
        .add_card(gizmon("GIZMON1", 1))
        .add_card(make_filler("FILL"))
        .hand(0, &["BELPH"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-VARCUT", Some(0));
    runner.place_on_field(0, "GIZMON1", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    runner
        .accept_optional_trigger()
        .expect("accept the reducer");
    let _ = runner.auto_resolve();

    assert_eq!(
        mem_before - runner.memory(),
        4,
        "cost 5 reduced by the deleted cost-1 Gizmon → 4 paid (amount tracks the \
         specific deleted permanent's cost, not a constant)"
    );
}

/// Variable decline: declining plays at full cost and deletes nothing.
#[test]
fn variable_play_reducer_decline_pays_full_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(VARIABLE_REDUCER_YAML)
        .expect("TEST-VARCUT YAML parses")
        .add_card(belphemon("BELPH", 5))
        .add_card(gizmon("GIZMON", 3))
        .add_card(make_filler("FILL"))
        .hand(0, &["BELPH"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-VARCUT", Some(0));
    runner.place_on_field(0, "GIZMON", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    runner
        .decline_optional_trigger()
        .expect("decline the reducer");
    let _ = runner.auto_resolve();

    let gizmon_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "GIZMON");
    assert!(gizmon_present, "declining deletes nothing");
    assert_eq!(
        mem_before - runner.memory(),
        5,
        "declining pays the full cost 5"
    );
}

/// Variable clone-mid-park: a forked `Game` holding the parked Gizmon pick must
/// resolve to the SAME variable reduction (rule 28 clone-safety).
#[test]
fn variable_play_reducer_clone_resolves_park_identically() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(VARIABLE_REDUCER_YAML)
        .expect("TEST-VARCUT YAML parses")
        .add_card(belphemon("BELPH", 5))
        .add_card(gizmon("GIZMON", 3))
        .add_card(make_filler("FILL"))
        .hand(0, &["BELPH"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "TEST-VARCUT", Some(0));
    runner.place_on_field(0, "GIZMON", Some(0));

    runner.play(0, 0);
    runner
        .accept_optional_trigger()
        .expect("accept the reducer");

    let action = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Gizmon delete pick parked");
        assert!(
            runner.game.pending_selection_resume.is_some(),
            "the parked Gizmon pick must be resume-driven before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, action)
        .expect("cloned Gizmon pick resolves");
    let clone_mem = clone.memory;
    runner
        .game
        .resolve_selection(0, action)
        .expect("original Gizmon pick resolves");

    assert_eq!(
        runner.game.memory, clone_mem,
        "clone and original must replay the same variable reduction"
    );
}
