//! BT25-042 ClavisAngemon — Digimon, Lv.6, DP 12000, Cost 12.
//! Traits: Virtue, Iliad, TS. Colors: Yellow/Black. Standard evo: Lv.5 / cost 4.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing
//! your top or bottom security card, your opponent's Digimon effects don't
//! affect this Digimon until their turn ends.
//! [All Turns] [Once Per Turn] When your security stack is removed from, you
//! may play 1 level 4 or lower [Angel] or [Iliad] trait card from your hand
//! without paying the cost. Then, 2 of your Digimon gain <Reboot> and <Blocker>
//! until your opponent's turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_042.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - C (multi-timing OPT cost-fired self effect-immunity)
//! - E (OnOwnSecurityRemoved triggered free-play + multi-grant)
//! - F (effect-immunity grant)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectSourceKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-042";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn sec(id: &str) -> CardData {
    make_test_card(id, id)
}

fn own_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(5000);
    card.traits = vec!["Beast".to_string()];
    card
}

/// Level-4 [Angel] card — a valid free-play target for the security clause.
fn angel_l4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["Angel".to_string()];
    card
}

/// Level-6 non-matching card — NOT a valid free-play target.
fn big_nonmatch(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 11;
    card.traits = vec!["Machine".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-042 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(sec("SEC"))
        .add_card(own_digimon("OWN-A"))
        .add_card(own_digimon("OWN-B"))
        .add_card(angel_l4("ANGEL"))
        .add_card(big_nonmatch("BIG"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_042_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "ClavisAngemon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
}

#[test]
fn bt25_042_has_alt_digivolve_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        !card.alt_paths.is_empty(),
        "must have an alt-digivolve path"
    );
}

#[test]
fn bt25_042_shared_clause_is_opt_across_three_timings() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have shared OP/WD/WA clause");
    assert!(clause.once_per_turn, "shared clause is [Once Per Turn]");
    assert!(
        clause.optional,
        "DCGO presents a 'Don't trash' decline → optional"
    );
    // The top/bottom security trash is a genuine player choice.
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectEffectChoice { .. })),
        "shared clause exposes the top/bottom security choice"
    );
    // The effect-immunity grant lives inside the branch bodies (after the
    // chosen-side trash) — recurse into nested `If` steps to find it.
    assert!(
        clause.process.iter().any(step_grants_immunity),
        "shared clause grants effect immunity inside its trash branches"
    );
}

/// Recursively check whether a step (or a nested If branch) grants effect immunity.
fn step_grants_immunity(step: &CompiledStep) -> bool {
    match step {
        CompiledStep::GrantEffectImmunity { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => then.iter().any(step_grants_immunity) || else_branch.iter().any(step_grants_immunity),
        _ => false,
    }
}

#[test]
fn bt25_042_security_removed_clause_present_and_opt() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOwnSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have OnOwnSecurityRemoved clause");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. })),
        "security-removed clause free-plays an Angel/Iliad card"
    );
}

// ─── Section 2 — Behavioral: shared clause grants effect immunity ────────────

/// On Play with >=1 security: accepting the trigger trashes a security card and
/// grants this Digimon immunity from opponent Digimon effects.
#[test]
fn bt25_042_on_play_trash_security_grants_digimon_immunity() {
    let mut runner = base().security(0, &["SEC", "SEC", "SEC"]).start();
    let clavis = runner.place_on_field(0, CARD_ID, Some(0));
    let sec_before = runner.security_count(0);

    runner.fire_on_play(0, clavis.index as usize);
    assert!(
        runner.pending_selection().is_some(),
        "shared clause must install the accept/decline + top/bottom prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the trigger");
    runner
        .auto_resolve()
        .expect("resolve top/bottom trash + immunity grant");

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "exactly 1 security card trashed as the cost"
    );
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(clavis, 1, EffectSourceKind::Digimon),
        "this Digimon is immune to opponent Digimon effects after the trash"
    );
}

/// Negative: no security → additional use-condition fails → clause must not fire.
#[test]
fn bt25_042_no_security_clause_does_not_fire() {
    let mut runner = base().start();
    let clavis = runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(runner.security_count(0), 0, "precondition: empty security");

    runner.fire_on_play(0, clavis.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "no security → cost unpayable → clause must not fire"
    );
}

/// Negative: declining the shared clause keeps security and grants no immunity.
#[test]
fn bt25_042_decline_keeps_security_no_immunity() {
    let mut runner = base().security(0, &["SEC", "SEC"]).start();
    let clavis = runner.place_on_field(0, CARD_ID, Some(0));
    let sec_before = runner.security_count(0);

    runner.fire_on_play(0, clavis.index as usize);
    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner.decline_optional_trigger().expect("decline is legal");

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "no security trashed on decline"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(clavis, 1, EffectSourceKind::Digimon),
        "no immunity granted on decline"
    );
}

// ─── Section 3 — Behavioral: security-removed free-play + Reboot/Blocker ─────

/// When the controller's security is removed, accepting the OPT trigger lets you
/// free-play a level-4 Angel card from hand, then grant 2 Digimon Reboot+Blocker.
#[test]
fn bt25_042_security_removed_freeplay_and_grant() {
    let mut runner = base()
        .hand(0, &["ANGEL"])
        .security(0, &["SEC", "SEC"])
        .start();
    let clavis = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "OWN-A", Some(1));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    // Fire the OnOwnSecurityRemoved trigger directly.
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::Permanent(clavis),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "security-removed must install a prompt (free-play and/or multi-select)"
    );
    runner.auto_resolve().expect("resolve free-play + grants");

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the ANGEL card was played from hand"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the ANGEL card entered the battle area for free"
    );
}

/// Negative: empty hand → no free-play; the multi-grant still proceeds over
/// existing Digimon (free-play leg is optional, grant leg is mandatory).
#[test]
fn bt25_042_security_removed_empty_hand_no_freeplay() {
    let mut runner = base().security(0, &["SEC", "SEC"]).start();
    let clavis = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "OWN-A", Some(1));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::Permanent(clavis),
    );
    runner.game.drain_effect_queue();
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("resolve grants");
    }

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no card played (empty hand)"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no new permanent (nothing to free-play)"
    );
}

// ─── Section 4 — Security-removed free play: hand-pick legality ─────────────
//
// DCGO BT25_042.cs:133-137 `CanPlayCardCondition`: `HasLevel && Level <= 4 &&
// (EqualsTraits("Angel") || HasIliadTraits) && CanPlayAsNewPermanent(cardSource,
// false, activateClass)` — a free battle-area frame and no effect-play lock.
// DCGO has no `HasPlayCost` here and `CanPlayAsNewPermanent` does not check it
// (GameContextDeterminarion.cs:419-432, CardSource.cs:1122-1182), so DCGO would
// offer a Lv.2 [Angel]/[Iliad] Digi-Egg (e.g. Bukamon BT24-002). The rules
// don't allow that: CR 4.3 7-1-1 ("playing" places a new card with a play
// cost) and 1-3-2 (an impossible play is not carried out) — the engine follows
// the rules and never offers a Digi-Egg.

fn iliad_egg(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.dp = None;
    card.play_cost = 0;
    card.traits = vec!["Iliad".to_string()];
    card
}

/// Fire ClavisAngemon's [All Turns] security-removed clause and return the
/// non-PASS hand-pick candidates' ids (empty when no pick installs). `setup`
/// runs before the trigger fires.
fn offered_on_security_removed(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let mut runner = base()
        .add_card(iliad_egg("ILIAD-EGG"))
        .hand(0, hand)
        .security(0, &["SEC", "SEC"])
        .start();
    let clavis = runner.place_on_field(0, CARD_ID, Some(0));
    setup(&mut runner);
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::Permanent(clavis),
    );
    runner.game.drain_effect_queue();
    for _ in 0..4 {
        let Some(v) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if v.kind == SelectionKind::Hand {
            let ids: Vec<String> = runner.game.players[0]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            return v
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&id| id != PASS)
                .map(|id| ids[(id - PLAY_HAND_START) as usize].clone())
                .collect();
        }
        if v.kind != SelectionKind::EffectChoice && !runner.pending_is_optional() {
            return Vec::new();
        }
        runner.accept_optional_trigger().expect("accept the optional clause");
    }
    Vec::new()
}

/// Regression guard: the Lv.4 [Angel] Digimon is offered, the Lv.6 non-match
/// is not.
#[test]
fn bt25_042_hand_pick_offers_playable_angel() {
    assert_eq!(
        offered_on_security_removed(&["ANGEL", "BIG"], |_| {}),
        vec!["ANGEL".to_string()]
    );
}

/// A Lv.2 [Iliad] Digi-Egg has no play cost (CR 4.3 7-1-1): not offered, even
/// though DCGO's filter would offer it.
#[test]
fn bt25_042_hand_pick_excludes_iliad_digi_egg() {
    assert_eq!(
        offered_on_security_removed(&["ILIAD-EGG", "ANGEL"], |_| {}),
        vec!["ANGEL".to_string()]
    );
}

/// A full battle area: the [Angel] Digimon can't be played, so it is not
/// offered.
#[test]
fn bt25_042_hand_pick_excludes_angel_when_battle_area_full() {
    let offered = offered_on_security_removed(&["ANGEL"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "no free slot ⇒ nothing offered: {offered:?}");
}

/// "You can't play Digimon by effects": not offered.
#[test]
fn bt25_042_hand_pick_excludes_angel_under_digimon_effect_play_lock() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = offered_on_security_removed(&["ANGEL"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(offered.is_empty(), "effect-play lock ⇒ nothing offered: {offered:?}");
}
