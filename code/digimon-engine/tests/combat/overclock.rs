//! `<Overclock>` execution tests.
//!
//! §16-34-2 (Comprehensive Rules 4.3): <Overclock> is a trigger-type effect
//! that triggers at the end of your turn. Its `[End of Your Turn]` body parks
//! the optional processing condition (§16-34-3: "by deleting 1 of your Tokens
//! or 1 of your other specified Digimon") as a sacrifice prompt inside the
//! end-of-turn pool (`Game::install_overclock_selection`); resolving it
//! deletes the chosen sacrifice and declares the mandatory attack on the
//! opponent player without suspending the attacker (§16-34-1). There is no
//! separate `EndOfTurnAction` activation any more.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, ATTACK_START, PASS, TARGETS_PER_ATTACKER};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};
use digimon_engine::selection::{AttackTarget, SelectionKind};

fn make_digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        link_text: String::new(),
        requirement_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Standard setup: turn player has an Overclock Digimon at slot 0 and an
/// ordinary Digimon at slot 1. Returns `(runner, tp, overclock_handle)`.
fn setup_overclock_scenario() -> (
    DebugRunner,
    digimon_engine::enums::PlayerId,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("SAC", 3000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);

    // §16-34-2: ending the turn activates <Overclock>'s trigger, which parks
    // its condition's sacrifice prompt inside the end-of-turn pool.
    r.game.end_turn();
    assert!(r.game.pending_selection.is_some(), "the <Overclock> prompt");

    (r, tp, oc)
}

struct CaptureDeletionCause(Arc<Mutex<Option<ReplacementCause>>>);

impl CardEffect for CaptureDeletionCause {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cause = self.0.clone();
        vec![Effect::on_deletion(card)
            .name("capture deletion cause")
            .process(move |ctx| {
                *cause.lock().unwrap() = ctx.deletion_cause();
            })
            .build()]
    }
}

#[test]
fn end_of_turn_installs_sacrifice_prompt() {
    let (r, tp, _oc) = setup_overclock_scenario();

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("pending selection installed");
    assert_eq!(sel.kind, SelectionKind::OwnField);
    assert_eq!(sel.selecting_player, tp);
    assert!(sel.is_optional, "<Overclock> may be declined");
    // §16-34-2: offered inside the end-of-turn pool, not in a later window.
    assert_eq!(sel.previous_phase, GamePhase::EndTurn);

    // Only the non-Overclock slot (index 1) is a valid sacrifice.
    let expected_id = encode_attack(0, 1);
    assert_eq!(sel.valid_action_ids, vec![expected_id]);
    assert_eq!(r.game.current_phase, GamePhase::SelectTarget);
}

#[test]
fn no_prompt_without_keyword() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("SAC", 3000))
        .start();
    let tp = r.game.turn_player();
    let _oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    // Note: NO Overclock keyword granted.
    r.game.end_turn();

    assert!(r.game.pending_selection.is_none());
    assert_ne!(r.game.turn_player(), tp, "nothing to offer: the turn passed");
}

#[test]
fn no_trigger_without_sacrifice() {
    // DCGO `CanActivateOverclock`: no legal candidate, no activation.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    assert!(r.game.pending_selection.is_none());
    assert_ne!(r.game.turn_player(), tp, "nothing to offer: the turn passed");
}

#[test]
fn no_prompt_before_the_end_of_turn() {
    // §16-34-2: the trigger is the end of your turn; nothing is offered in
    // the main phase.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("SAC", 3000))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::Main;

    assert!(r.game.pending_selection.is_none());
}

#[test]
fn decline_consumes_current_end_turn_overclock_opportunity() {
    let (mut r, tp, oc) = setup_overclock_scenario();

    let sac_count_before = r.game.player(tp).battle_area.len();
    r.game
        .resolve_selection(tp, PASS)
        .expect("optional prompt accepts PASS");

    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        sac_count_before,
        "PASS must not delete any permanent"
    );
    let oc_perm = &r.game.player(tp).battle_area[oc.index as usize];
    assert!(
        !oc_perm.is_suspended,
        "Overclock Digimon not suspended on decline"
    );
    // §16-34-3: declining the optional condition ends <Overclock>'s
    // processing; the trigger activated once, so nothing is re-offered and
    // the end of turn continues.
    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert_ne!(
        r.game.turn_player(),
        tp,
        "with the declined Overclock consumed, the turn passes"
    );
}

#[test]
fn full_flow_sacrifice_and_attack_resolves_on_security() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .add_card(make_digimon("SEC_DIGI", 2000))
        .security(1, &["SEC_DIGI"])
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();
    assert!(r.game.pending_selection.is_some(), "the <Overclock> prompt");

    let battle_area_before = r.game.player(tp).battle_area.len();
    let sec_before = r.game.player(opp).security.len();

    // Resolve with the sacrifice.
    let sac_action = encode_attack(0, sac.index as u16);
    r.game.resolve_selection(tp, sac_action).unwrap();

    // Sacrifice deleted.
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        battle_area_before - 1,
        "sacrifice removed from battle area"
    );
    // Security dropped by one (attacker 7000 DP > SEC_DIGI 2000 DP).
    assert_eq!(
        r.game.player(opp).security.len(),
        sec_before - 1,
        "one security card consumed by the Overclock attack"
    );

    // Attack has cleaned up; attacker still NOT suspended.
    assert!(r.game.pending_attack.is_none());
    let oc_perm = &r.game.player(tp).battle_area[0];
    assert!(
        !oc_perm.is_suspended,
        "<Overclock> attack must not suspend the attacker"
    );
    // §16-34-1 / §16-44-1 (G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP): the attack
    // was the end-of-turn action. With no further end-of-turn action left
    // (the only other Digimon was sacrificed) the end of turn continues: the
    // turn passes with no PASS-only `EndOfTurnAction` step. (When another
    // end-of-turn action remains the window stays open.)
    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert_eq!(r.game.turn_player(), opp, "the turn passed");
}

#[test]
fn overclock_sacrifice_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .add_card(make_digimon("SEC_DIGI", 2000))
        .security(1, &["SEC_DIGI"])
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    let own_before = r.game.player(tp).battle_area.len();
    let sec_before = r.game.player(opp).security.len();
    assert!(
        r.game.pending_selection_resume.is_some(),
        "Overclock sacrifice prompt must park a data frame for clone-faithful prompts"
    );

    let action = encode_attack(0, sac.index as u16);
    let mut clone = r.game.clone();
    clone
        .resolve_selection(tp, action)
        .expect("clone pays Overclock sacrifice");

    assert!(clone.pending_attack.is_none());
    assert_eq!(
        clone.player(tp).battle_area.len(),
        own_before - 1,
        "clone should delete the sacrifice"
    );
    assert_eq!(
        clone.player(opp).security.len(),
        sec_before - 1,
        "clone should run the Overclock attack"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the Overclock prompt"
    );
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        own_before,
        "resolving the clone must not delete the original sacrifice"
    );

    r.game
        .resolve_selection(tp, action)
        .expect("original pays Overclock sacrifice");
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        clone.player(tp).battle_area.len(),
        "original and clone delete the same sacrifice"
    );
    assert_eq!(
        r.game.player(opp).security.len(),
        clone.player(opp).security.len(),
        "original and clone replay the Overclock attack identically"
    );
}

#[test]
fn overclock_sacrifice_deletion_reports_overclock_cause() {
    let cause = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .start();
    r.register_effect("SAC", Arc::new(CaptureDeletionCause(cause.clone())));

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    let sac_action = encode_attack(0, sac.index as u16);
    r.game.resolve_selection(tp, sac_action).unwrap();

    assert_eq!(
        *cause.lock().unwrap(),
        Some(ReplacementCause::Overclock),
        "Overclock sacrifice deletion must preserve Overclock as the cause"
    );
}

#[test]
fn full_flow_against_empty_security_wins_game() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);

    // Empty the opponent's security stack.
    r.game.player_mut(opp).security.clear();
    r.game.end_turn();

    let sac_action = encode_attack(0, sac.index as u16);
    r.game.resolve_selection(tp, sac_action).unwrap();

    assert!(r.game.game_over, "player hit with empty security loses");
    assert_eq!(r.game.winner, Some(tp));
}

#[test]
fn begin_attack_overclock_skips_suspend_directly() {
    // Low-level coverage for the combat-side flag: begin_attack_overclock
    // must leave the attacker unsuspended. No selection plumbing.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));

    let result = r.game.begin_attack_overclock(oc, AttackTarget::Player(opp));
    // Security exists by default (5 cards); attack resolves terminally
    // (attacker may be hit by a security Digimon, but for this card DP
    // we expect it to survive or deal a security — either way, the flag
    // semantics are: no suspend on declaration, regardless of outcome).
    assert_ne!(result, AttackResult::Invalid);

    let oc_perm = &r.game.player(tp).battle_area[0];
    assert!(
        !oc_perm.is_suspended,
        "is_overclock path must not suspend the attacker"
    );
}

#[test]
fn regular_attack_still_suspends() {
    // Regression guard: the refactor extracting begin_attack_impl must not
    // break the normal suspend-on-attack behavior for non-Overclock paths.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 5000))
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let atk = r.place_on_field(tp, "ATK", Some(0));
    r.game.current_phase = GamePhase::Main;

    let _ = r.game.attack_player(atk, opp, /* vortex = */ false);
    let atk_perm = &r.game.player(tp).battle_area[0];
    assert!(
        atk_perm.is_suspended,
        "normal attack must still suspend the attacker"
    );
}

#[test]
fn action_id_decodes_correctly_when_sac_at_higher_index() {
    // Verify action-ID round-trip: install prompt over sacrifice at field
    // index 3, resolve with that exact ID, confirm the correct slot was
    // deleted (not slot 0 or 1 from a miscalculated decode).
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("FILLER", 3000))
        .add_card(make_digimon("SAC", 3000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _ = r.place_on_field(tp, "FILLER", Some(0));
    let _ = r.place_on_field(tp, "FILLER", Some(0));
    let sac3 = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    let sel = r.game.pending_selection.as_ref().unwrap();
    // Three sacrifices (slots 1, 2, 3) are legal.
    assert_eq!(sel.valid_action_ids.len(), 3);
    assert!(sel
        .valid_action_ids
        .contains(&encode_attack(0, sac3.index as u16)));

    // Resolve with slot 3 specifically.
    let action = encode_attack(0, 3);
    assert_eq!(
        (action - ATTACK_START) % TARGETS_PER_ATTACKER,
        3,
        "sanity check on the encoding"
    );
    let before = r.game.player(tp).battle_area.len();
    r.game.resolve_selection(tp, action).unwrap();

    // One permanent gone, Overclock and first two fillers still present.
    assert_eq!(r.game.player(tp).battle_area.len(), before - 1);
}

// ─── G-ENGINE-OVERCLOCK-COST-REPLACEMENT-PARKS-AFTER-POOL ─────────────────────

/// `[End of Your Turn] Gain 2 memory.`
struct EndOfTurnGainTwo;

impl CardEffect for EndOfTurnGainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("[End of Your Turn] Gain 2 memory")
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

/// P0: `OC` (<Overclock>, any other Digimon is a cost), `SAC` as a 2-card
/// stack with <Armor Purge> (its deletion parks an optional replacement), and
/// `EOT` ("[End of Your Turn] Gain 2 memory"). P0 ends the turn and activates
/// <Overclock> first, then picks `SAC`; the <Armor Purge> decision parks
/// inside the cost's deletion. Returns the runner, the event checkpoint and
/// P1's security count.
fn overclock_cost_parks_board() -> (DebugRunner, usize, usize) {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .add_card(make_digimon("SAC_BASE", 2000))
        .add_card(make_digimon("EOT", 1000))
        .start();
    r.register_effect("EOT", Arc::new(EndOfTurnGainTwo));
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_stack(tp, &["SAC_BASE", "SAC"]);
    r.place_on_field(tp, "EOT", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game
        .modifiers
        .grant_keyword(sac, Keyword::ArmorPurge, Expiry::EndOfTurn, tp);
    let security_before = r.game.player(1 - tp).security.len();
    let cp = r.event_checkpoint();
    r.game.end_turn();
    let order = r.pending_selection().expect("the end-of-turn pool's TriggerOrder");
    assert_eq!(order.kind, SelectionKind::TriggerOrder, "{order:?}");
    let pick = order
        .effect_choices
        .as_ref()
        .and_then(|cs| cs.iter().find(|c| c.label.starts_with("OC")))
        .map(|c| c.action_id)
        .expect("<Overclock> in the TriggerOrder");
    r.game.resolve_selection(tp, pick).expect("activate <Overclock> first");
    r.game
        .resolve_selection(tp, encode_attack(0, sac.index as u16))
        .expect("pick SAC as the cost");
    assert!(r.pending_selection().is_some(), "<Armor Purge> parks inside the deletion");
    (r, cp, security_before)
}

/// §11-1-4 (Ver. 4.3) "After declaring an attack, the counter timing won't
/// begin until any triggered [When Attacking] effects and other triggered
/// effects have been resolved"; Q3869: the other `[End of Your Turn]` effect
/// activates AFTER the keyword's attack declaration. When <Overclock>'s cost
/// deletion parks a decision (here <Armor Purge>, declined), the attack is
/// declared from `resume_pending_overclock_attack` at the `resolve_selection`
/// tail, AFTER the tail's drain has already resolved the pool's remaining
/// entry: `EOT`'s memory gain comes before the declaration.
#[test]
#[ignore = "G-ENGINE-OVERCLOCK-COST-REPLACEMENT-PARKS-AFTER-POOL ordering half: stays open, non-blocking by controller ruling (§11-1-4, §16-34-2; Task 21 re-review M-4 / Task 36 brief)"]
fn overclock_attack_declared_before_the_rest_of_the_pool_when_its_cost_parks() {
    let (mut r, cp, _) = overclock_cost_parks_board();
    let tp = r.turn_player();
    r.game.resolve_selection(tp, PASS).expect("decline <Armor Purge>");
    let events = r.events_since(cp);
    let declared = events
        .iter()
        .position(|e| matches!(e, digimon_engine::events::GameEvent::Attack { .. }))
        .expect("the <Overclock> attack was declared");
    let gain = events
        .iter()
        .position(|e| matches!(e, digimon_engine::events::GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == "EOT"))
        .expect("EOT resolved");
    assert!(declared < gain, "declaration (#{declared}) < EOT (#{gain})");
}

/// §16-34-3 "If the content of that condition is executed, the processing
/// for "the Digimon with this effect attacks a player without suspending"
/// will be mandatory"; §15-7-2: an optional processing condition that is not
/// executed does not let the processing after it happen. <Armor Purge>
/// prevents the cost's deletion, so nothing was deleted and <Overclock> does
/// not attack (DCGO `OverclockProcess` attacks only on `isDeleted`).
#[test]
fn overclock_does_not_attack_when_its_cost_deletion_is_prevented() {
    let (mut r, cp, security_before) = overclock_cost_parks_board();
    r.accept_optional_trigger().expect("<Armor Purge> prevents the deletion");
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    let attacked = r
        .events_since(cp)
        .iter()
        .any(|e| matches!(e, digimon_engine::events::GameEvent::Attack { .. }));
    assert!(!attacked, "the condition was not executed: no attack");
    assert_eq!(r.game.player(1).security.len(), security_before);
}

/// The `<Armor Purge>` decision of [`overclock_cost_parks_board`] is a parked
/// VM decision, so the game clones there (`Game: Clone`, rule 28). Each
/// branch keys the attack on the cost's actual outcome (§16-34-3, §15-7-1,
/// §15-8-5-2): accepting <Armor Purge> prevents the deletion (no attack, the
/// stack stays with `SAC_BASE` on top); declining lets the deletion happen
/// and the mandatory attack follows (a security check). Resolving either
/// clone leaves the original parked at the prompt.
#[test]
fn overclock_cost_prevention_clones_while_parked_and_each_branch_keys_on_the_outcome() {
    let (mut r, cp, security_before) = overclock_cost_parks_board();
    let tp = r.turn_player();
    let opp = 1 - tp;
    assert!(
        r.game.pending_selection_resume.is_some(),
        "<Armor Purge> parks on a data frame"
    );
    let accepted = r.game.clone();
    let declined = r.game.clone();
    let original = std::mem::replace(&mut r.game, accepted);

    r.accept_optional_trigger()
        .expect("<Armor Purge> prevents the deletion");
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    let attacked = r
        .events_since(cp)
        .iter()
        .any(|e| matches!(e, digimon_engine::events::GameEvent::Attack { .. }));
    assert!(!attacked, "accepted: the deletion was prevented, no attack");
    assert_eq!(r.game.player(opp).security.len(), security_before);
    let tops: Vec<String> = r
        .game
        .player(tp)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        tops.contains(&"SAC_BASE".to_string()),
        "accepted: <Armor Purge> trashed the top card, the Digimon stayed: {tops:?}"
    );

    r.game = declined;
    r.game
        .resolve_selection(tp, PASS)
        .expect("decline <Armor Purge>");
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    let attacked = r
        .events_since(cp)
        .iter()
        .any(|e| matches!(e, digimon_engine::events::GameEvent::Attack { .. }));
    assert!(
        attacked,
        "declined: the cost was deleted, the attack is mandatory"
    );
    // P1 has no security cards on this board: the attack's check is the
    // direct hit that wins the game.
    assert_eq!(security_before, 0);
    assert_eq!(r.game.winner, Some(tp), "declined: the attack hit P1");
    assert!(
        r.game
            .player(tp)
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "SAC_BASE"),
        "declined: SAC was deleted"
    );

    r.game = original;
    assert!(
        r.pending_selection().is_some(),
        "resolving the clones leaves the original at the <Armor Purge> prompt"
    );
    assert_eq!(r.game.player(opp).security.len(), security_before);
}

/// "When this Digimon would be deleted, it isn't deleted" -- a MANDATORY
/// replacement: the cost's deletion is prevented without parking a decision.
struct PreventOwnDeletion;

impl CardEffect for PreventOwnDeletion {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("When this Digimon would be deleted, it isn't deleted")
            .replacement_condition(|ctx, subject| {
                matches!(subject, ReplacementSubject::Permanent(h) if Some(*h) == ctx.source_permanent)
            })
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}

/// §16-34-3 / §15-7-1 "If the player chooses to execute the processing for
/// the conditions and is successful, the processing after the conditions is
/// performed"; §15-8-5-2: a "would be deleted" effect that prevents the
/// removal means "it isn't deleted". The non-parking path: a mandatory
/// prevention stops the cost's deletion with no decision, so the condition
/// was not executed and <Overclock> does not attack (DCGO `OverclockProcess`
/// attacks only on `isDeleted`).
#[test]
fn overclock_does_not_attack_when_a_mandatory_replacement_prevents_its_cost_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .start();
    r.register_effect("SAC", Arc::new(PreventOwnDeletion));
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    let security_before = r.game.player(opp).security.len();
    let cp = r.event_checkpoint();
    r.game.end_turn();
    r.game
        .resolve_selection(tp, encode_attack(0, sac.index as u16))
        .expect("pick SAC as the cost");
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    let attacked = r
        .events_since(cp)
        .iter()
        .any(|e| matches!(e, digimon_engine::events::GameEvent::Attack { .. }));
    assert!(!attacked, "the deletion was prevented: no attack");
    assert_eq!(r.game.player(opp).security.len(), security_before);
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        2,
        "SAC was not deleted"
    );
}

/// §15-7-4 (Ver. 4.3) "If executing an optional processing condition's
/// contents isn't possible, then the player can't choose to execute them."
/// A Digimon that can't be deleted (a passive `CannotBeDestroyed`) is not a
/// cost <Overclock> can choose: with another candidate only that one is
/// offered; with no other candidate the condition is not offered at all.
#[test]
fn overclock_does_not_offer_a_cost_that_cannot_be_deleted() {
    let build = || {
        let mut r = DebugRunner::builder()
            .add_card(make_digimon("OC", 7000))
            .add_card(make_digimon("SAC", 3000))
            .add_card(make_digimon("SAC2", 3000))
            .start();
        let tp = r.game.turn_player();
        let oc = r.place_on_field(tp, "OC", Some(0));
        let sac = r.place_on_field(tp, "SAC", Some(0));
        r.game
            .modifiers
            .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
        r.game.modifiers.add(
            sac,
            ModifierEntry::simple(ModifierType::CannotBeDestroyed, 1, Expiry::EndOfTurn, tp),
        );
        (r, tp, sac)
    };

    // Another candidate: only it is offered.
    let (mut r, tp, sac) = build();
    let sac2 = r.place_on_field(tp, "SAC2", Some(0));
    r.game.end_turn();
    let sel = r.pending_selection().expect("the condition's pick");
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, sac2.index as u16)],
        "only the deletable Digimon is offered, not {sac:?}"
    );

    // No other candidate: the condition is not offered and the turn passes.
    let (mut r, tp, _) = build();
    r.game.end_turn();
    assert!(
        !r.pending_is_end_of_turn_keyword_decision(),
        "no <Overclock> pick: {:?}",
        r.pending_selection()
    );
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    assert_eq!(r.game.turn_player(), 1 - tp, "the turn passed");
}
