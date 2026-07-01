//! BT21-043 Sociamon — Digimon, Lv.4, Yellow, DP 4000, Play Cost 4.
//!
//! # Corrected printed card text (from card image — authoritative)
//!
//! **Traits:** Sup./Appmon | Social | SNS
//! **Attribute:** Social
//! **Digivolve:** Standard Lv.3 yellow path (JSON evo_costs) + Alt: [Stnd.] trait: Cost 2
//! **Link:** [Appmon] trait: Cost 2
//!
//! [Security] At the end of the battle, play this card without paying the cost.
//!
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -2000 DP until
//! their turn ends.
//!
//! **Link box:**
//! - Link [Appmon] trait: Cost 2
//! - +3000 DP (while linked, the host gets +3000 DP)
//! - [When Linking] 1 of your opponent's Digimon gets -2000 DP until their turn ends.
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT21/Yellow/BT21_043.cs
//! - `canNoSelect: false` in ALL three -2000 DP blocks → mandatory when eligible target exists.
//! - `EffectDuration.UntilOpponentTurnEnd` in ALL three -2000 DP blocks → expiry: end_of_opponents_turn.
//! - Alt-digivolve: `EqualsTraits("Stnd.")` only (no level gate in DCGO).
//!
//! # Pattern rows covered
//! - on_security play_from_security (same as BT21-015 / BT25-036)
//! - [On Play][When Digivolving] add_dp_modifier -2000 end_of_opponents_turn (mandatory)
//! - link_condition cost 2 filter trait_has Appmon
//! - scope: linked kind: aura dp_modifier: 3000 (G-LINK-INHERITED-ESS / DigiLink Shape-B)
//! - scope: linked when: when_linked → mandatory -2000 DP to opponent
//! - alt_path digivolve from { trait_has: "Stnd." } cost: 2 (no level gate per DCGO)
//! - DP-to-zero deletion (project memory dp_zero_deletion / rule 17-1-3-1)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT21-043";
const SOCIAMON_YAML: &str = include_str!("../../../cards/bt21/BT21-043.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c
}

fn make_appmon_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.traits = vec!["Appmon".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(SOCIAMON_YAML)
        .expect("BT21-043 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_opp_digimon("OPP-3000", 3000))
        .add_card(make_opp_digimon("OPP-2000", 2000))
        .add_card(make_opp_digimon("OPP-6000", 6000))
        .add_card(make_appmon_digimon("HOST-APPMON", 4, 5000))
        .deck(1, &["DECK-PAD"; 12])
}

/// Compute the action-ID for the Link action of a permanent at `field_index`.
fn link_action_id(field_index: u8) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK
}

/// Drive the card's link action onto a host: decode_action → resolve host selection prompt.
/// Returns the host handle.
fn do_link(runner: &mut DebugRunner, linking_perm: PermanentHandle, host: PermanentHandle) {
    runner
        .game
        .decode_action(link_action_id(linking_perm.index), 0);
    // Resolve the host-pick prompt (mandatory: picks HOST-APPMON)
    let host_action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    let _ = runner.game.resolve_selection(0, host_action);
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Rotate active player to `target`, flushing end-of-turn action so
/// `end_of_opponents_turn` modifier expiry fires (AD1-016 idiom).
fn advance_to_turn_player(runner: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if runner.turn_player() == target {
            return;
        }
        let _ = runner.auto_resolve();
        runner.pass_turn();
        let _ = runner.auto_resolve();
        if runner.game.current_phase == GamePhase::EndOfTurnAction {
            runner.game.pass_end_of_turn_action();
            let _ = runner.auto_resolve();
        }
    }
    assert_eq!(
        runner.turn_player(),
        target,
        "failed to rotate to target turn player"
    );
}

// ─── Section 1: Structural assertions ─────────────────────────────────────────

#[test]
fn bt21_043_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("BT21-043 in pack");
    assert_eq!(card.name, "Sociamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.cost, Some(4));
}

#[test]
fn bt21_043_traits_include_appmon_social_sns_sup() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    for trait_name in ["Sup.", "Appmon", "Social", "SNS"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
}

#[test]
fn bt21_043_has_on_security_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnSecurity),
        _ => false,
    });
    assert!(
        has,
        "BT21-043 must have a triggered clause with when: on_security"
    );
}

#[test]
fn bt21_043_has_on_play_when_digivolving_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        has,
        "BT21-043 must have a triggered clause with when: [on_play, when_digivolving]"
    );
}

#[test]
fn bt21_043_on_play_when_digivolving_clause_is_not_optional() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[On Play][When Digivolving] clause must exist");
    assert!(
        !clause.optional,
        "DCGO canNoSelect=false → clause must NOT be optional (mandatory when target exists)"
    );
}

#[test]
fn bt21_043_has_link_condition_appmon_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 2
        )
    });
    assert!(
        has,
        "BT21-043 must have a self link-condition with cost 2 (filter: Appmon)"
    );
}

#[test]
fn bt21_043_has_linked_scope_dp_aura_3000() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) if *scope == CompiledScope::Linked => Some(*dp_modifier),
        _ => None,
    });
    assert_eq!(
        aura,
        Some(Some(3000)),
        "BT21-043 must declare a linked-scope dp aura of +3000 DP"
    );
}

#[test]
fn bt21_043_has_linked_scope_when_linked_triggered_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::WhenLinked) && matches!(t.scope, CompiledScope::Linked)
        }
        _ => false,
    });
    assert!(
        has,
        "BT21-043 must have a scope:linked triggered clause with when: when_linked"
    );
}

#[test]
fn bt21_043_has_stnd_alt_path() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    // The alt_path digivolve from { trait_has: "Stnd." } cost 2 must be present.
    use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .map(|f| f.trait_has.as_deref() == Some("Stnd."))
                .unwrap_or(false)
    });
    assert!(
        has,
        "BT21-043 must have an alt-digivolve path from {{ trait_has: \"Stnd.\" }} cost 2"
    );
}

// ─── Section 2: [On Play] behavioral ──────────────────────────────────────────

/// Positive: on play with opponent Digimon present → OppField selection installs.
#[test]
fn bt21_043_on_play_installs_dp_debuff_selection() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-3000", Some(0));

    runner.play(0, 0).expect("Sociamon plays from hand");

    let kind = runner
        .pending_kind()
        .expect("debuff selection must install after OnPlay");
    assert!(
        matches!(kind, SelectionKind::OppField),
        "expected OppField selection, got {:?}",
        kind
    );
}

/// Positive: after selecting target, opponent Digimon gets -2000 DP.
#[test]
fn bt21_043_on_play_applies_2000_dp_debuff() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-3000", Some(0)); // 3000 DP

    runner.play(0, 0).expect("Sociamon plays from hand");
    runner.auto_resolve().expect("selection resolves");

    assert_eq!(
        runner.dp_of(opp),
        Some(1000),
        "OPP-3000 must have 1000 DP after -2000 debuff"
    );
}

/// Negative: no opponent Digimon → no pending selection installs.
#[test]
fn bt21_043_on_play_no_selection_when_no_opponent_digimon() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    // No opponent Digimon placed.
    runner.play(0, 0).expect("Sociamon plays from hand");

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when the opponent has no Digimon on field"
    );
}

/// Behavioral: -2000 DP to a 2000 DP Digimon brings it to 0 → deleted (rule 17-1-3-1).
#[test]
fn bt21_043_on_play_deletes_2000dp_opponent_digimon() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let _opp = runner.place_on_field(1, "OPP-2000", Some(0)); // exactly 2000 DP

    assert_eq!(runner.battle_area_size(1), 1, "pre: opponent has 1 Digimon");

    runner.play(0, 0).expect("Sociamon plays from hand");
    runner.auto_resolve().expect("selection resolves");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "a 2000 DP Digimon debuffed by -2000 must reach 0 DP and be deleted (rule 17-1-3-1)"
    );
}

/// Behavioral: debuff expires at end of opponent's turn (end_of_opponents_turn).
#[test]
fn bt21_043_on_play_debuff_expires_at_end_of_opponents_turn() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .add_card(make_test_card("DECK-PAD2", "Filler2"))
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-6000", Some(0)); // 6000 DP → 4000 after debuff

    runner.play(0, 0).expect("Sociamon plays from hand");
    runner.auto_resolve().expect("selection resolves");
    assert_eq!(
        runner.dp_of(opp),
        Some(4000),
        "debuff applied (6000 - 2000)"
    );

    // Advance to player 1's turn — debuff must persist through their turn.
    advance_to_turn_player(&mut runner, 1);
    assert_eq!(
        runner.dp_of(opp),
        Some(4000),
        "debuff persists through the opponent's turn"
    );

    // Advance back to player 0's turn — opponent's turn has ended → debuff clears.
    advance_to_turn_player(&mut runner, 0);
    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "debuff must expire at end of opponent's turn"
    );
}

// ─── Section 3: [When Digivolving] behavioral ─────────────────────────────────

/// Positive: fire WhenDigivolving on a stacked permanent → OppField selection installs.
#[test]
fn bt21_043_when_digivolving_installs_dp_debuff_selection() {
    let mut runner = base()
        .add_card(make_test_card("BASE-LV3", "BaseLv3"))
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();

    let base_perm = runner.place_on_field(0, "BASE-LV3", Some(0));

    // Push Sociamon as top card source (simulate digivolve).
    let socio_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("BT21-043 in card_data");
    let next = runner.game.next_card_index();
    let socio_src = CardSource::new(socio_idx, 0, next);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[base_perm.index as usize].digivolve(socio_src, turn);

    runner.place_on_field(1, "OPP-3000", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base_perm),
    );
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("debuff selection must install on WhenDigivolving");
    assert!(
        matches!(kind, SelectionKind::OppField),
        "expected OppField, got {:?}",
        kind
    );
}

// ─── Section 4: [Security] play_from_security behavioral ──────────────────────

/// Integration: Sociamon in P1's security → P0 attacks → Sociamon plays onto P1's field.
#[test]
fn bt21_043_security_plays_card_onto_field_after_battle() {
    let mut runner = base()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Push Sociamon into P1's security.
    {
        let socio_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .expect("BT21-043 in card_data");
        let next = runner.game.next_card_index();
        let socio_src = CardSource::new(socio_idx, 1, next);
        runner.game.players[1].security.push(socio_src);
    }

    assert_eq!(runner.security_count(1), 1, "pre: P1 has 1 security card");

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "security card must be removed by attack"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Sociamon must play onto P1's battle area by [Security] effect"
    );
}

// ─── Section 5: Link DP aura (+3000) behavioral ────────────────────────────────

/// Positive: when Sociamon is linked to a host, host's effective DP increases by +3000.
#[test]
fn bt21_043_linked_aura_gives_host_plus_3000_dp() {
    let mut runner = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();

    let host = runner.place_on_field(0, "HOST-APPMON", Some(0)); // 5000 base DP
    let socio = runner.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut runner);

    // Effective DP before linking
    let dp_before = runner.effective_dp(host).expect("host has DP");

    // Link Sociamon onto the host.
    do_link(&mut runner, socio, host);
    // Sociamon is now absorbed into host's linked_cards.
    runner.auto_resolve().ok();

    // Effective DP after linking must increase by 3000.
    let dp_after = runner.effective_dp(host).expect("host still has DP");
    assert_eq!(
        dp_after,
        dp_before + 3000,
        "host effective_dp must increase by +3000 while Sociamon is linked"
    );
}

// ─── Section 6: [When Linking] behavioral ─────────────────────────────────────

/// Positive: when Sociamon links to host while opponent has a Digimon, -2000 DP prompt surfaces.
#[test]
fn bt21_043_when_linked_installs_dp_debuff_selection() {
    let mut runner = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();

    let host = runner.place_on_field(0, "HOST-APPMON", Some(0));
    let socio = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-3000", Some(0));
    advance_to_main(&mut runner);

    // Initiate link action.
    runner.game.decode_action(link_action_id(socio.index), 0);
    // Resolve host-pick selection.
    let host_action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    let _ = runner.game.resolve_selection(0, host_action);

    // After linking, When Linking trigger must surface an OppField selection.
    let kind = runner
        .pending_kind()
        .expect("when_linked debuff selection must surface after linking");
    assert!(
        matches!(kind, SelectionKind::OppField),
        "expected OppField selection after linking, got {:?}",
        kind
    );
}

/// Positive: selecting target in When Linking applies -2000 DP.
#[test]
fn bt21_043_when_linked_applies_2000_dp_debuff() {
    let mut runner = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();

    let host = runner.place_on_field(0, "HOST-APPMON", Some(0));
    let socio = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-3000", Some(0)); // 3000 DP → 1000 after debuff
    advance_to_main(&mut runner);

    // Link and resolve.
    do_link(&mut runner, socio, host);
    runner.auto_resolve().ok(); // resolves the when_linked debuff selection

    assert_eq!(
        runner.dp_of(opp),
        Some(1000),
        "OPP-3000 must have 1000 DP after when_linked -2000 debuff"
    );
}

/// Negative: when Sociamon links but opponent has no Digimon → no pending selection.
#[test]
fn bt21_043_when_linked_no_selection_when_no_opponent_digimon() {
    let mut runner = base().deck(0, &["DECK-PAD"; 12]).memory(10).start();

    let host = runner.place_on_field(0, "HOST-APPMON", Some(0));
    let socio = runner.place_on_field(0, CARD_ID, Some(0));
    // No opponent Digimon.
    advance_to_main(&mut runner);

    // Initiate link.
    runner.game.decode_action(link_action_id(socio.index), 0);
    let host_action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    let _ = runner.game.resolve_selection(0, host_action);
    runner.auto_resolve().ok();

    // No opponent Digimon → when_linked effect should not produce a pending selection.
    assert!(
        runner.pending_selection().is_none(),
        "no selection should remain after when_linked when no opponent Digimon exists"
    );
}
