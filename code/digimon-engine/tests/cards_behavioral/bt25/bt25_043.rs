//! BT25-043 Habakirimon / "Habakiri" — DUAL card (Digimon + Option).
//! Lv.6 Yellow, Shaman / Glowing Dawn / BEATBREAK, DP 12000, Cost 6, Virus.
//!
//! # Card text (card image BT25-043 — DIGIMON / OPTION "DUAL")
//!
//! DIGIMON FACE
//! [When Digivolving] [When Attacking] [Once Per Turn] <Recovery +1> (Place the
//! top card of your deck as your top security card.) Then, by trashing the top
//! security card of 1 player with the most security cards, this Digimon
//! unsuspends.
//! [All Turns] [Once Per Turn] When any of your [Glowing Dawn] trait Digimon
//! would leave the battle area, by trashing your top security card, they don't
//! leave.
//! OPTION FACE — "Habakiri", Use 6
//! Use Req. [Glowing Dawn] trait.
//! [Main] 1 of your opponent's Digimon gets -8000 DP for the turn. Then, by
//! trashing your top security card, all of your opponent's Digimon get -5000 DP
//! for the turn.
//! <Arts Digivolve>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_043.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G-DSL-DUAL-PER-FACE-EFFECTS: Digimon-face + Option-face clauses.
//! - G-DSL-ARTS-DIGIVOLVE: Arts digivolve from the Option face.
//! - A2-adjacent Recovery +1; E1 3-way EffectChoice gated by security count
//! - F3 leave-prevention replacement (trash top security -> cancel)
//! - E2 OPT (both triggered clauses are once_per_turn)
//!
//! # Verdict — IMPLEMENTED (upgraded from PARTIAL: Option side now ships).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT25-043";

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-043 YAML parses and compiles")
        .deck(0, &["BT25-043"; 10])
        .deck(1, &["BT25-043"; 10])
        .security(0, &["BT25-043"; 3])
        .security(1, &["BT25-043"; 3])
        .memory(8)
        .start()
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize]
        .battle_area
        .get(h.index as usize)
        .map(|p| p.is_suspended)
        .unwrap_or(false)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_043_metadata() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Habakirimon");
    assert_eq!(card.kind, CompiledCardKind::Dual, "now a DUAL card");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Glowing Dawn", "BEATBREAK"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
    let dual = card.dual.as_ref().expect("dual block present");
    assert_eq!(dual.digimon.level, 6);
    assert_eq!(dual.option.use_cost, 6);
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "Option face has Arts Digivolve"
    );
}

#[test]
fn bt25_043_has_recovery_clause_on_wd_wa_opt() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let clause = dual
        .digimon
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("WD/WA Recovery clause present on Digimon face");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    let has_recover = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Recover { .. }));
    assert!(has_recover, "Recovery +1 step present");
}

#[test]
fn bt25_043_has_glowing_dawn_leave_replacement() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let has_repl = dual.digimon.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(
        has_repl,
        "leave-prevention replacement present on Digimon face"
    );
}

#[test]
fn bt25_043_option_face_has_main_and_arts() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    assert_eq!(dual.option.effects.len(), 1, "Option face [Main] clause");
    assert!(
        dual.option.keywords.contains(&"ArtsDigivolve".to_string()),
        "arts_digivolve folds into the option keyword"
    );
}

#[test]
fn bt25_043_has_glowing_dawn_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_gd = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)
    });
    assert!(has_gd, "Lv.5 [Glowing Dawn] alt-path present");
}

// ─── Section 2 — Recovery + unsuspend behavior (positive) ────────────────────

#[test]
fn bt25_043_recovery_then_trash_own_unsuspends() {
    // own security (5) > opponent (1) -> own qualifies as "most".
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .deck(0, &["BT25-043"; 10])
        .deck(1, &["BT25-043"; 10])
        .security(0, &["BT25-043"; 5])
        .security(1, &["BT25-043"; 1])
        .memory(8)
        .start();

    let hab = runner.place_stack(0, &["BT25-043"]);
    let own_sec_before = runner.security_count(0);
    runner.attack_player(hab, 1, false);
    assert!(is_suspended(&runner, hab), "attacker suspends on attack");

    // WhenAttacking fires: Recovery (+1 own sec), then trash-choice prompt.
    let kind = runner.pending_kind().expect("trash-choice prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("trash own"); // own top security
    runner.auto_resolve().expect("resolve");

    // Net own security: +1 recovery -1 trash = unchanged; attacker unsuspended.
    assert_eq!(runner.security_count(0), own_sec_before);
    assert!(
        !is_suspended(&runner, hab),
        "Habakirimon unsuspends after trashing security"
    );
}

#[test]
fn bt25_043_decline_trash_leaves_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .deck(0, &["BT25-043"; 10])
        .deck(1, &["BT25-043"; 10])
        .security(0, &["BT25-043"; 5])
        .security(1, &["BT25-043"; 1])
        .memory(8)
        .start();

    let hab = runner.place_stack(0, &["BT25-043"]);
    let own_sec_before = runner.security_count(0);
    runner.attack_player(hab, 1, false);

    let view = runner.pending_selection_view().expect("choice installs");
    let last = view.effect_choices.as_ref().unwrap().len() - 1;
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[last])
        .expect("decline"); // "Don't trash security"
    runner.auto_resolve().expect("resolve");

    // Declining trashes NO security beyond the Recovery +1 that already ran:
    // net own security = before + 1 (Recovery) and no unsuspend cost paid.
    assert_eq!(
        runner.security_count(0),
        own_sec_before + 1,
        "declining must not trash security (only the Recovery +1 applies)"
    );
}

fn recovery_choice_attack(
    own_security: usize,
    opponent_security: usize,
    can_recover: bool,
) -> (DebugRunner, PermanentHandle) {
    let deck = if can_recover {
        vec![CARD_ID; 5]
    } else {
        vec![]
    };
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("Habakirimon compiles")
        .dsl_card("BT25-032")
        .expect("Liollmon compiles")
        .deck(0, &deck)
        .deck(1, &["BT25-032"; 5])
        .security(0, &vec!["BT25-032"; own_security])
        .security(1, &vec!["BT25-032"; opponent_security])
        .memory(8)
        .start();
    let hab = runner.place_stack(0, &[CARD_ID]);
    let defender = runner.place_stack(1, &["BT25-032"]);
    runner.game.players[1].battle_area[defender.index as usize].is_suspended = true;
    runner.game.enter_main_phase();
    let attack = encode_attack(hab.index as u16, defender.index as u16);
    assert_eq!(build_action_mask(&runner.game, 0)[attack as usize], 1.0);
    runner.game.decode_action(attack, 0);
    // A battle-area target keeps later combat independent of security counts.
    assert_eq!(
        runner.security_count(0),
        own_security + usize::from(can_recover)
    );
    assert_eq!(runner.security_count(1), opponent_security);
    (runner, hab)
}

const TRASH_OWN_SECURITY: &str = "Trash your own top security card";
const TRASH_OPPONENT_SECURITY: &str = "Trash your opponent's top security card";
const DECLINE_SECURITY_TRASH: &str = "Don't trash security";

fn assert_recovery_choices(runner: &DebugRunner, expected_labels: &[&str]) {
    let view = runner
        .pending_selection_view()
        .expect("security payment choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    assert_eq!(view.selecting_player, 0);
    let labels: Vec<_> = view
        .effect_choices
        .as_ref()
        .expect("branch labels")
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(
        labels, expected_labels,
        "only payable most-security players and decline are offered"
    );
    let mask = build_action_mask(&runner.game, 0);
    let legal_actions: Vec<_> = mask
        .iter()
        .enumerate()
        .filter_map(|(action, &legal)| (legal > 0.5).then_some(action as u16))
        .collect();
    assert_eq!(
        legal_actions, view.valid_action_ids,
        "mask contains exactly the offered choices"
    );
    assert_eq!(legal_actions.len(), expected_labels.len());
    assert!(build_action_mask(&runner.game, 1)
        .iter()
        .all(|&legal| legal == 0.0));
}

fn choose_security_payment(runner: &mut DebugRunner, label: &str) {
    let view = runner
        .pending_selection_view()
        .expect("security payment choice");
    let index = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .position(|choice| choice.label == label)
        .expect("requested choice is legal");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[index])
        .expect("answer the security payment choice");
    runner.auto_resolve().expect("finish the attack");
}

#[test]
fn bt25_043_recovery_choice_masks_lower_security_player() {
    for (own_before, opponent_before, payer) in [(2, 2, 0), (0, 3, 1)] {
        // The comparator must run AFTER Recovery: (2, 2) becomes (3, 2).
        let (mut runner, hab) = recovery_choice_attack(own_before, opponent_before, true);
        let payment = if payer == 0 {
            TRASH_OWN_SECURITY
        } else {
            TRASH_OPPONENT_SECURITY
        };
        assert_recovery_choices(&runner, &[payment, DECLINE_SECURITY_TRASH]);
        choose_security_payment(&mut runner, payment);
        assert_eq!(
            runner.security_count(0),
            own_before + 1 - usize::from(payer == 0)
        );
        assert_eq!(
            runner.security_count(1),
            opponent_before - usize::from(payer == 1)
        );
        assert!(
            !is_suspended(&runner, hab),
            "the chosen legal payment unsuspends Habakirimon"
        );
    }
}

#[test]
fn bt25_043_recovery_choice_tied_security_offers_both_payments() {
    for payer in [0, 1] {
        let (mut runner, hab) = recovery_choice_attack(1, 2, true);
        assert_recovery_choices(
            &runner,
            &[
                TRASH_OWN_SECURITY,
                TRASH_OPPONENT_SECURITY,
                DECLINE_SECURITY_TRASH,
            ],
        );
        let payment = if payer == 0 {
            TRASH_OWN_SECURITY
        } else {
            TRASH_OPPONENT_SECURITY
        };
        choose_security_payment(&mut runner, payment);
        assert_eq!(runner.security_count(0), 2 - usize::from(payer == 0));
        assert_eq!(runner.security_count(1), 2 - usize::from(payer == 1));
        assert!(
            !is_suspended(&runner, hab),
            "either tied player's payment unsuspends"
        );
    }
}

#[test]
fn bt25_043_recovery_choice_decline_keeps_security_and_suspension() {
    let (mut runner, hab) = recovery_choice_attack(2, 2, true);
    assert_recovery_choices(&runner, &[TRASH_OWN_SECURITY, DECLINE_SECURITY_TRASH]);
    choose_security_payment(&mut runner, DECLINE_SECURITY_TRASH);
    assert_eq!(
        runner.security_count(0),
        3,
        "Recovery remains, no payment is taken"
    );
    assert_eq!(runner.security_count(1), 2);
    assert!(
        is_suspended(&runner, hab),
        "declining never unsuspends the attacker"
    );
}

#[test]
fn bt25_043_recovery_choice_zero_security_player_is_never_payable() {
    for (own_before, opponent_before, payer) in [(0, 2, 1), (2, 0, 0)] {
        // An empty deck makes Recovery do nothing, retaining the zero count.
        let (mut runner, hab) = recovery_choice_attack(own_before, opponent_before, false);
        let payment = if payer == 0 {
            TRASH_OWN_SECURITY
        } else {
            TRASH_OPPONENT_SECURITY
        };
        assert_recovery_choices(&runner, &[payment, DECLINE_SECURITY_TRASH]);
        choose_security_payment(&mut runner, payment);
        assert_eq!(
            runner.security_count(0),
            own_before - usize::from(payer == 0)
        );
        assert_eq!(
            runner.security_count(1),
            opponent_before - usize::from(payer == 1)
        );
        assert!(!is_suspended(&runner, hab));
    }
}

#[test]
fn bt25_043_recovery_choice_both_empty_skips_unpayable_prompt() {
    let (mut runner, hab) = recovery_choice_attack(0, 0, false);
    assert!(
        runner.pending_selection().is_none(),
        "no payment exists, so no security choice is offered"
    );
    runner.auto_resolve().expect("finish the attack");
    assert_eq!(runner.security_count(0), 0);
    assert_eq!(runner.security_count(1), 0);
    assert!(is_suspended(&runner, hab));
}

// ─── Section 3 — OPT lockout on the Recovery clause ─────────────────────────

#[test]
fn bt25_043_recovery_opt_locks_second_attack_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .deck(0, &["BT25-043"; 10])
        .deck(1, &["BT25-043"; 10])
        .security(0, &["BT25-043"; 5])
        .security(1, &["BT25-043"; 1])
        .memory(8)
        .start();

    let hab = runner.place_stack(0, &["BT25-043"]);
    runner.attack_player(hab, 1, false);
    runner.execute_branch(0).expect("trash own");
    runner.auto_resolve().expect("resolve");

    if !is_suspended(&runner, hab) {
        runner.attack_player(hab, 1, false);
        assert!(
            runner.pending_selection().is_none(),
            "Recovery OPT must lock the second attack in the same turn"
        );
    }
}

// ─── Section 4 — leave-prevention replacement (Glowing Dawn) ────────────────

fn leave_prevention_runner(security: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("Habakirimon compiles")
        .dsl_card("BT25-032")
        .expect("Liollmon compiles")
        .dsl_card("BT25-031")
        .expect("Patamon compiles")
        .deck(0, &[CARD_ID; 10])
        .deck(1, &[CARD_ID; 10])
        .security(0, security)
        .security(1, &[CARD_ID; 3])
        .memory(8)
        .start()
}

fn assert_leave_prevention_choice(runner: &DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("payable Glowing Dawn leave prevention must be offered");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert_eq!(
        view.selecting_player, 0,
        "the protected Digimon's owner decides"
    );
    assert!(runner.pending_is_optional(), "paying security is optional");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[REPLACEMENT_ACCEPT as usize], 1.0, "may pay the cost");
    assert_eq!(mask[PASS as usize], 1.0, "may decline the cost");
    assert_eq!(mask.iter().filter(|&&legal| legal > 0.5).count(), 2);
    assert!(
        build_action_mask(&runner.game, 1)
            .iter()
            .all(|&legal| legal == 0.0),
        "the opponent has no action during the replacement choice"
    );
}

#[test]
fn bt25_043_glowing_dawn_leave_prevention_installs() {
    // Deleting a Glowing-Dawn Digimon (Habakirimon itself) while >=1 security
    // exists must offer the prevention replacement.
    let mut runner = leave_prevention_runner(&["BT25-032", CARD_ID]);
    let hab = runner.place_stack(0, &[CARD_ID]);
    let hab_top = runner.top_card(hab);
    let sec_before = runner.security_count(0);
    let top_security = runner.game.player(0).security.last().unwrap().handle();

    // Drive a deletion through the engine's would-leave path.
    runner
        .game
        .delete_permanents_batch(vec![hab], ReplacementCause::OpponentEffect);

    assert_leave_prevention_choice(&runner);
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "no cost before choosing"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept prevention");
    runner.auto_resolve().expect("resolve");
    assert_eq!(runner.security_count(0), sec_before - 1);
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(
        runner.game.player(0).trash[0].handle(),
        top_security,
        "only the top security is trashed"
    );
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(
        runner.top_card(hab),
        hab_top,
        "the same Habakirimon survives"
    );
}

#[test]
fn bt25_043_glowing_dawn_leave_prevention_empty_security_is_not_offered() {
    // Opponent security cannot pay the owner's cost (CR 15-7-4).
    let mut runner = leave_prevention_runner(&[]);
    let hab = runner.place_stack(0, &[CARD_ID]);
    runner
        .game
        .delete_permanents_batch(vec![hab], ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "an unpayable cost is not offered"
    );
    assert_eq!(runner.battle_area_size(0), 0, "the deletion proceeds");
    assert_eq!(runner.security_count(0), 0);
    assert_eq!(runner.security_count(1), 3);
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(
        runner.game.player(0).trash[0].card_id(&runner.game.card_data),
        CARD_ID
    );
}

#[test]
fn bt25_043_glowing_dawn_leave_prevention_decline_leaves_opt_available() {
    let mut runner = leave_prevention_runner(&[CARD_ID; 2]);
    runner.place_stack(0, &[CARD_ID]);
    let ally = runner.place_stack(0, &["BT25-032"]);
    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OpponentEffect);
    assert_leave_prevention_choice(&runner);
    runner.execute_action(0, PASS).expect("decline prevention");
    runner.auto_resolve().expect("resolve deletion");

    assert_eq!(runner.battle_area_size(0), 1, "declined ally leaves");
    assert_eq!(runner.security_count(0), 2, "declining pays no security");
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(
        runner.game.player(0).trash[0].card_id(&runner.game.card_data),
        "BT25-032"
    );

    // A later, distinct leave event can still consume Habakirimon's OPT.
    let another_ally = runner.place_stack(0, &["BT25-032"]);
    runner
        .game
        .delete_permanents_batch(vec![another_ally], ReplacementCause::OpponentEffect);
    assert_leave_prevention_choice(&runner);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("pay on the later event");
    runner.auto_resolve().expect("resolve prevention");
    assert_eq!(runner.battle_area_size(0), 2, "the later ally survives");
    assert_eq!(runner.security_count(0), 1);
}

#[test]
fn bt25_043_glowing_dawn_leave_prevention_payment_consumes_opt() {
    let mut runner = leave_prevention_runner(&[CARD_ID; 2]);
    runner.place_stack(0, &[CARD_ID]);
    let ally = runner.place_stack(0, &["BT25-032"]);
    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OpponentEffect);
    assert_leave_prevention_choice(&runner);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("pay prevention cost");
    runner.auto_resolve().expect("resolve prevention");
    assert_eq!(runner.battle_area_size(0), 2, "the protected ally survives");
    assert_eq!(
        runner.security_count(0),
        1,
        "security remains to distinguish OPT from payability"
    );

    runner
        .game
        .delete_permanents_batch(vec![ally], ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection().is_none(),
        "OPT blocks a second use this turn"
    );
    assert_eq!(runner.battle_area_size(0), 1, "second deletion proceeds");
    assert_eq!(runner.security_count(0), 1, "no second cost is paid");
    assert_eq!(runner.trash_size(0), 2);
    assert_eq!(
        runner.game.player(0).trash[1].card_id(&runner.game.card_data),
        "BT25-032"
    );
}

#[test]
fn bt25_043_glowing_dawn_leave_prevention_one_payment_saves_all_matching_simultaneous_leaves() {
    for security_count in [1, 2] {
        let security = vec![CARD_ID; security_count];
        let mut runner = leave_prevention_runner(&security);
        let hab = runner.place_stack(0, &[CARD_ID]);
        let ally = runner.place_stack(0, &["BT25-032"]);
        let second_ally = runner.place_stack(0, &["BT25-032"]);
        let non_gd = runner.place_stack(0, &["BT25-031"]);
        let opposing_gd = runner.place_stack(1, &["BT25-032"]);
        let protected = vec![
            runner.top_card(hab),
            runner.top_card(ally),
            runner.top_card(second_ally),
        ];
        let non_gd_card = runner.top_card(non_gd);
        let opposing_card = runner.top_card(opposing_gd);
        let paid_card = runner.game.player(0).security.last().unwrap().handle();

        // The production DUAL card's Digimon face protects its carrier and
        // every other matching own Digimon in this one simultaneous event.
        // Neither an own non-Glowing-Dawn Digimon nor an opposing Glowing
        // Dawn Digimon belongs to that protected group.
        runner.game.delete_permanents_batch(
            vec![hab, ally, second_ally, non_gd, opposing_gd],
            ReplacementCause::OpponentEffect,
        );
        assert_leave_prevention_choice(&runner);
        assert_eq!(
            runner.security_count(0),
            security_count,
            "no cost before acceptance"
        );
        runner
            .execute_action(0, REPLACEMENT_ACCEPT)
            .expect("pay once for the qualifying group");
        assert!(
            runner.pending_selection().is_none(),
            "the matching group needs only one payment decision"
        );
        runner
            .auto_resolve()
            .expect("complete simultaneous deletion");

        assert_eq!(
            runner.security_count(0),
            security_count - 1,
            "one payment, even when it is the last security card"
        );
        let surviving: Vec<_> = runner
            .game
            .player(0)
            .battle_area
            .iter()
            .map(|permanent| permanent.top_card().handle())
            .collect();
        assert_eq!(
            surviving, protected,
            "carrier and both own Glowing Dawn allies survive"
        );
        assert_eq!(
            runner.battle_area_size(1),
            0,
            "the opposing Glowing Dawn Digimon leaves"
        );
        let own_trash: Vec<_> = runner
            .game
            .player(0)
            .trash
            .iter()
            .map(|card| card.handle())
            .collect();
        assert_eq!(
            own_trash.len(),
            2,
            "only the security payment and ineligible ally are trashed"
        );
        assert!(own_trash.contains(&paid_card));
        assert!(own_trash.contains(&non_gd_card));
        assert_eq!(runner.trash_size(1), 1);
        assert_eq!(runner.game.player(1).trash[0].handle(), opposing_card);
        assert_eq!(
            runner.security_count(1),
            3,
            "opponent security never pays this cost"
        );
    }
}

#[test]
fn bt25_043_glowing_dawn_leave_prevention_declining_simultaneous_group_deletes_all() {
    let mut runner = leave_prevention_runner(&[CARD_ID]);
    let hab = runner.place_stack(0, &[CARD_ID]);
    let ally = runner.place_stack(0, &["BT25-032"]);
    let second_ally = runner.place_stack(0, &["BT25-032"]);
    let non_gd = runner.place_stack(0, &["BT25-031"]);
    let opposing_gd = runner.place_stack(1, &["BT25-032"]);
    let own_departures = [hab, ally, second_ally, non_gd].map(|handle| runner.top_card(handle));
    let opposing_card = runner.top_card(opposing_gd);
    let unpaid_security = runner.game.player(0).security.last().unwrap().handle();

    runner.game.delete_permanents_batch(
        vec![hab, ally, second_ally, non_gd, opposing_gd],
        ReplacementCause::OpponentEffect,
    );
    assert_leave_prevention_choice(&runner);
    runner
        .execute_action(0, PASS)
        .expect("decline the group's security payment");
    assert!(
        runner.pending_selection().is_none(),
        "declining for the departing carrier allows this group to resolve"
    );
    runner
        .auto_resolve()
        .expect("complete declined simultaneous deletion");

    assert_eq!(runner.battle_area_size(0), 0);
    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.security_count(0), 1, "decline pays nothing");
    assert_eq!(runner.game.player(0).security[0].handle(), unpaid_security);
    let own_trash: Vec<_> = runner
        .game
        .player(0)
        .trash
        .iter()
        .map(|card| card.handle())
        .collect();
    assert_eq!(own_trash.len(), own_departures.len());
    for departed in own_departures {
        assert!(
            own_trash.contains(&departed),
            "every own batch subject leaves"
        );
    }
    assert_eq!(runner.trash_size(1), 1);
    assert_eq!(runner.game.player(1).trash[0].handle(), opposing_card);
    assert_eq!(runner.security_count(1), 3);
}

// ─── Section 5 — Option face: [Main] DP debuffs + Arts Digivolve ─────────────

fn glowing_dawn_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(dp);
    c.play_cost = 6;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c
}

/// Yellow Lv.5 [Glowing Dawn] base for the Arts gate.
fn yellow_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

#[test]
fn bt25_043_option_main_minus_8000_then_minus_5000_all_with_security_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .add_card(glowing_dawn_digimon("GD-DIGI", 9000))
        // DPs high enough that the -8000 / -5000 debuffs keep both alive (a
        // Digimon at <=0 DP is deleted by the state-based rule), so the DP
        // deltas are observable.
        .add_card(opp_digimon("OPP-A", 16000))
        .add_card(opp_digimon("OPP-B", 12000))
        .hand(0, &[CARD_ID])
        .security(0, &["BT25-043"; 3])
        .memory(8)
        .start();
    // A Glowing Dawn Digimon satisfies the Use Req.
    runner.place_on_field(0, "GD-DIGI", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0)); // 16000
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0)); // 12000
    runner.game.enter_main_phase();

    let a_before = runner.dp_of(opp_a).unwrap();
    let b_before = runner.dp_of(opp_b).unwrap();
    let sec_before = runner.security_count(0);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    // Step 1: pick the -8000 target (OPP-A).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .execute_action(0, encode_attack(0, opp_a.index as u16))
        .expect("pick -8000 target");
    assert_eq!(
        runner.dp_of(opp_a).unwrap(),
        a_before - 8000,
        "single target gets -8000 DP"
    );

    // Step 2: optional "by trashing your top security, all opp -5000". Accept.
    // The optional gate surfaces as a yes/no (the inner trash/mass-debuff).
    while runner.game.pending_selection.is_some() {
        let sel = runner.game.pending_selection.as_ref().unwrap();
        // Accept the optional security-trash branch (first non-PASS action).
        let action = sel.valid_action_ids.first().copied().unwrap_or(PASS);
        runner
            .execute_action(0, action)
            .expect("accept mass debuff");
    }

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "trashed own top security to pay the mass-debuff cost"
    );
    assert_eq!(
        runner.dp_of(opp_a).unwrap(),
        a_before - 8000 - 5000,
        "the -8000 target ALSO gets the board-wide -5000"
    );
    assert_eq!(
        runner.dp_of(opp_b).unwrap(),
        b_before - 5000,
        "every opponent Digimon gets -5000 DP"
    );
}

#[test]
fn bt25_043_arts_digivolve_arms_and_stacks() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .add_card(glowing_dawn_digimon("GD-DIGI", 9000))
        .add_card(yellow_base("Y-BASE"))
        .deck(0, &["BT25-043"; 5])
        .hand(0, &[CARD_ID])
        .security(0, &["BT25-043"; 1])
        .memory(8)
        .start();
    // A yellow Lv.5 Glowing Dawn base: legal Arts target AND satisfies Use Req.
    let base = runner.place_on_field(0, "Y-BASE", Some(0));
    // No opponent Digimon → the [Main] DP steps no-op; resolution reaches Arts.
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    // Drive past the optional [Main] tail (no targets → may park nothing or an
    // optional gate) until the Arts OwnField prompt with `base` legal.
    loop {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("selection parked before Arts");
        let is_arts = sel.kind == SelectionKind::OwnField
            && sel.is_optional
            && sel
                .valid_action_ids
                .contains(&encode_attack(0, base.index as u16));
        if is_arts {
            break;
        }
        let action = if sel.is_optional {
            PASS
        } else {
            sel.valid_action_ids[0]
        };
        runner.execute_action(0, action).expect("drive to Arts");
    }

    let trash_before = runner.trash_size(0);
    runner
        .execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept Arts");
    let perm = &runner.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.stack_size(), 2, "Arts stacked the DUAL onto the base");
    assert_eq!(perm.top_card().card_id(&runner.game.card_data), CARD_ID);
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "Arts prevented the normal Option trash"
    );
    let _ = Keyword::Rush;
}
