//! BT25-040 MagnaAngemon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Archangel, Iliad, TS.  Attribute: Vaccine.
//! Evo: Lv.4 Yellow / cost 3 (printed evo_costs).
//!
//! # Card text (cards.json — verbatim)
//!
//! When effects trash this card from the security stack, you may play 1 level 4
//!   or lower [Angel] or [Iliad] trait card from your hand without paying the cost.
//! <Ascension> (When this Digimon is deleted, you may place this card as the top
//!   security card.)
//! [On Play] [When Digivolving] By trashing your top or bottom security card, 1
//!   of your opponent's Digimon gets -8000 DP until their turn ends.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When your security stack is removed from, 1 of
//!   your opponent's Digimon gets -4000 DP for the turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_040.cs
//!   - None: AddSelfDigivolutionRequirementStaticEffect(level 4, HasTSTraits, cost 3).
//!   - OnDiscardSecurity (CanTriggerOnTrashSelfSecurity): optional play-from-hand
//!     free of a Lv<=4 Angel/Iliad card.
//!   - OnDestroyedAnyone: AscensionSelfEffect.
//!   - Shared OnPlay/WhenDigivolving (skippable): trash top OR bottom security
//!     (player choice), then 1 opp Digimon gets -8000 DP until their turn ends.
//!   - OnLoseSecurity (inherited, OPT): 1 opp Digimon gets -4000 DP for the turn.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F9/security: scope:security on_discard_security → optional play-from-hand-free
//! - <Ascension> keyword grant (OnDeletion replacement)
//! - E1: [On Play][When Digivolving] top/bottom security trash branch → -8000 DP
//! - inherited [All Turns][OPT] on_own_security_removed → -4000 DP
//! - alt-digivolution Lv.4 [TS] → cost 3

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-040";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// An [Angel] Lv.4 card to play free off the security-trash clause.
fn angel_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Yellow];
    card.dp = Some(4000);
    card.traits = vec!["Angel".to_string()];
    card.play_cost = 5;
    card
}

fn opp_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 5);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(9000);
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-040 YAML parses and compiles")
        .add_card(angel_lv4("ANGEL-LV4"))
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL", "FILL", "FILL"])
        .memory(10)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_040_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-040 must be in the embedded DSL pack");

    assert_eq!(card.name, "MagnaAngemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Archangel", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-040 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_040_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-040 must have a Lv.4 [TS] → cost 3 alt-path");
}

#[test]
fn bt25_040_grants_ascension() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ascension = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => format!("{d:?}").contains("Ascension"),
        _ => false,
    });
    assert!(has_ascension, "BT25-040 must grant <Ascension>");
}

#[test]
fn bt25_040_has_security_play_op_and_inherited_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_security_play = triggered.iter().any(|t| {
        t.scope == CompiledScope::Security && t.when.contains(&CompiledTiming::OnDiscardSecurity)
    });
    let has_op_wd = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    let has_inherited_lose_sec = triggered.iter().any(|t| {
        t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::OnOwnSecurityRemoved)
            && t.once_per_turn
    });
    assert!(
        has_security_play,
        "must have [Security-trash] play-from-hand-free clause"
    );
    assert!(
        has_op_wd,
        "must have [On Play][When Digivolving] DP-minus clause"
    );
    assert!(
        has_inherited_lose_sec,
        "must have inherited [All Turns][OPT] own-security-removed DP-minus clause"
    );
}

// ─── Section 2 — [On Play][When Digivolving] trash top/bottom security → -8000 DP ─

#[test]
fn bt25_040_on_play_offers_top_bottom_security_branch() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let _opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, magna.index as usize);

    assert!(
        runner.pending_kind().is_some(),
        "OnPlay must install a selection (trash top/bottom security cost → DP-minus)"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_040_on_play_applies_minus_8000_to_opponent_digimon() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp);

    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, magna.index as usize);
    runner.auto_resolve().ok();

    let dp_after = runner.effective_dp(opp);
    assert!(
        dp_after < dp_before,
        "the opponent Digimon must receive the -8000 DP (before={dp_before:?}, after={dp_after:?})"
    );
}

// ─── Section 3 — security-trash self → play Angel/Iliad free ─────────────────

#[test]
fn bt25_040_security_trash_self_is_optional_and_security_scoped() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let sec_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Security
                && t.when.contains(&CompiledTiming::OnDiscardSecurity)
        })
        .expect("security-trash play clause present");
    assert!(
        sec_clause.optional,
        "the security-trash play-from-hand clause must be optional ('you may')"
    );
}

// ─── Section 4 — inherited [All Turns][OPT] own-security-removed → -4000 DP ───

#[test]
fn bt25_040_has_inherited_minus_4000_clause_structure() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let inh = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnOwnSecurityRemoved)
        })
        .expect("inherited own-security-removed clause present");
    assert!(
        inh.once_per_turn,
        "inherited security-removed DP-minus clause must be [Once Per Turn]"
    );
}

// ─── Section 5 — Security-trash play: hand-pick legality ────────────────────
//
// DCGO BT25_040.cs:48-55 `CanPlayCardCondition`: `HasPlayCost && HasLevel &&
// Level <= 4 && (EqualsTraits("Angel") || EqualsTraits("Iliad")) &&
// CanPlayAsNewPermanent(cardSource, false, activateClass)` — a free battle-area
// frame and no effect-play lock. CR 4.3 1-3-2 (an impossible play is not
// carried out) and 7-1-1 (a Digi-Egg has no play cost and is never played).

// An OnPlay card that trashes the controller's top security by effect (the
// trigger the security-scope clause keys on; BT25-034 test idiom).
const TRASHER_YAML_040: &str = r#"
card: DSL-SEC-TRASHER-040
name: Sec Trasher
kind: digimon
level: 4
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Trash your top security card"
    process:
      - trash_top_security: { of: you }
"#;

fn iliad_egg(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 2);
    card.card_kind = digimon_engine::enums::CardKind::DigiEgg;
    card.colors = vec![CardColor::Yellow];
    card.dp = None;
    card.play_cost = 0;
    card.traits = vec!["Iliad".to_string()];
    card
}

/// Play the trasher (trashing MagnaAngemon from the top of security by effect)
/// and return the non-PASS hand-pick candidates' ids (empty when no pick
/// installs). `setup` runs before the trasher is played.
fn offered_after_effect_trash(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let mut runner = base()
        .from_dsl_yaml(TRASHER_YAML_040)
        .expect("inline trasher fixture compiles")
        .add_card(iliad_egg("ILIAD-EGG"))
        .hand(0, hand)
        .security(0, &["FILL", "FILL", CARD_ID])
        .start();
    setup(&mut runner);
    runner.play(0, 0).expect("play the trasher");
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
        runner.accept_optional_trigger().expect("accept the optional clause");
    }
    Vec::new()
}

/// Regression guard: the Lv.4 [Angel] Digimon is offered; a Lv.2 [Iliad]
/// Digi-Egg is not.
#[test]
fn bt25_040_hand_pick_offers_playable_angel_not_digi_egg() {
    let offered =
        offered_after_effect_trash(&["DSL-SEC-TRASHER-040", "ANGEL-LV4", "ILIAD-EGG"], |_| {});
    assert_eq!(offered, vec!["ANGEL-LV4".to_string()]);
}

/// A full battle area (the trasher takes the last slot): no candidate.
#[test]
fn bt25_040_hand_pick_excludes_angel_when_battle_area_full() {
    let offered = offered_after_effect_trash(&["DSL-SEC-TRASHER-040", "ANGEL-LV4"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8 + 1;
    });
    assert!(offered.is_empty(), "no free slot ⇒ nothing offered: {offered:?}");
}

/// "You can't play Digimon by effects": no candidate.
#[test]
fn bt25_040_hand_pick_excludes_angel_under_digimon_effect_play_lock() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = offered_after_effect_trash(&["DSL-SEC-TRASHER-040", "ANGEL-LV4"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(offered.is_empty(), "effect-play lock ⇒ nothing offered: {offered:?}");
}
