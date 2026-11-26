//! BT25-011 Aquilamon — Digimon, Lv.4, Red/Green, DP 4000, Cost 4.
//! Traits: Giant Bird, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! <Raid> (When this Digimon attacks, you may switch the target of attack to 1
//! of your opponent's unsuspended Digimon with the highest DP.)
//! [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon. Then, if
//! it's your turn, 2 of your Digimon may DNA digivolve into [Silphymon] in the
//! hand.
//!
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_011.cs
//!
//! - Raid (OnAllyAttack): RaidSelfEffect.
//! - OP/WD Shared: MANDATORY suspend 1 opp Digimon; then if owner turn, DNA
//!   digivolve 2 own Digimon into [Silphymon] (in hand), optional.
//! - Inherit (None, inherited): +2000 DP, owner-turn gated.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H9 Raid keyword
//! - F7-adjacent suspend on play
//! - G2 effect-initiated DNA digivolve (into hand) — your-turn gated
//! - D1 inherited self +DP (your turn)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-011";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 4;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 3,
    }];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-011 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-A", 4, 5000, &["Beast"]))
        .add_card(make_digimon("OWN-A", 4, 4000, &["Beast"]))
        .add_card(make_digimon("CARRIER", 5, 7000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_011_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-011 present in pack");
    assert_eq!(card.name, "Aquilamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    for t in ["Giant Bird", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_011_grants_raid() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let grants_raid = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword.eq_ignore_ascii_case("Raid")
        )
    });
    assert!(grants_raid, "BT25-011 must grant <Raid>");
}

#[test]
fn bt25_011_has_shared_on_play_when_digivolving_clause_with_dna_step() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
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
        .expect("shared clause present");
    // The clause must contain the DNA-digivolve step (gated under the your-turn if).
    fn has_dna(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::EffectInitiatedDnaDigivolve { .. } => true,
            CompiledStep::If {
                then, else_branch, ..
            } => has_dna(then) || has_dna(else_branch),
            _ => false,
        })
    }
    assert!(
        has_dna(&clause.process),
        "shared clause must contain an effect_initiated_dna_digivolve step"
    );
}

#[test]
fn bt25_011_has_inherited_dp_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            dp_modifier, scope, ..
        }) => Some((*dp_modifier, *scope)),
        _ => None,
    });
    assert_eq!(
        aura,
        Some((Some(2000), CompiledScope::Inherited)),
        "BT25-011 must have an inherited +2000 DP aura"
    );
}

// ─── Section 2 — Behavior: suspend on play ───────────────────────────────────

/// Positive: playing Aquilamon with an opponent Digimon present installs a
/// prompt (the mandatory suspend pick).
#[test]
fn bt25_011_on_play_installs_suspend_prompt() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    runner.place_on_field(1, "OPP-A", Some(0));

    let _a = runner.play(0, 0).expect("play Aquilamon");
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay must install the suspend prompt over the opponent Digimon"
    );
}

/// Behavioral: resolving the OnPlay suspends an opponent Digimon.
#[test]
fn bt25_011_on_play_suspends_an_opponent_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    let _a = runner.play(0, 0).expect("play Aquilamon");
    runner.auto_resolve().ok();

    assert!(
        is_suspended(&runner, opp),
        "the chosen opponent Digimon must be suspended"
    );
}

/// Negative: with no opponent Digimon and no legal Silphymon/DNA pair, no
/// prompt at all should open (not even an optional DNA prompt).
#[test]
fn bt25_011_on_play_no_suspend_prompt_without_opponent_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();

    let _a = runner.play(0, 0).expect("play Aquilamon");
    assert!(
        runner.pending_selection().is_none(),
        "with no opponent Digimon and no legal Silphymon/DNA pair, OnPlay must not park any prompt"
    );
}

// ─── Section 2b — Behavior: [Your Turn] DNA digivolve into [Silphymon] ──────

fn make_silphymon(id: &str, with_dna: bool) -> CardData {
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
    use digimon_engine::enums::CardColor;

    let mut card = make_test_card(id, "Silphymon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red, CardColor::Yellow];
    card.level = Some(5);
    card.dp = Some(8000);
    card.play_cost = 8;
    if with_dna {
        let lv4 = |color| DnaRequirement {
            level: 4,
            card_colors: vec![color],
            name_is: String::new(),
            name_contains: String::new(),
            text_contains: String::new(),
        };
        card.dna_costs = vec![DnaCost {
            requirement1: lv4(CardColor::Red),
            requirement2: lv4(CardColor::Yellow),
            memory_cost: 0,
        }];
    }
    card
}

fn make_colored_lv4(id: &str, color: digimon_engine::enums::CardColor) -> CardData {
    let mut card = make_digimon(id, 4, 4000, &["Beast"]);
    card.colors = vec![color];
    card
}

/// Regression for g14 step 23: playing Aquilamon with only itself on board (or
/// with a Silphymon that has no legal DNA pair) must NOT park a DNA prompt.
#[test]
fn bt25_011_on_play_does_not_open_dna_prompt_without_legal_pair() {
    use digimon_engine::enums::CardColor;

    let mut runner = base()
        .add_card(make_silphymon("SILPHY-DNA", true))
        .add_card(make_silphymon("SILPHY-NODNA", false))
        .add_card(make_colored_lv4("BLUE-LV4", CardColor::Blue))
        .hand(0, &[CARD_ID, "SILPHY-DNA", "SILPHY-NODNA"])
        .memory(6)
        .start();
    runner.place_on_field(0, "BLUE-LV4", Some(0));

    let _a = runner.play(0, 0).expect("play Aquilamon");
    assert!(
        runner.pending_selection().is_none(),
        "Red/Green Aquilamon + Blue Lv.4 cannot satisfy Red+Yellow Silphymon DNA: no prompt may open"
    );
}

/// Positive: on your turn with a legal [Silphymon] and matching field pair,
/// Aquilamon suspends an opponent Digimon and offers the optional DNA into
/// [Silphymon] (excluding non-DNA Silphymon printings).
#[test]
fn bt25_011_on_play_your_turn_dna_digivolves_into_silphymon() {
    use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PLAY_HAND_START};
    use digimon_engine::enums::CardColor;
    use digimon_engine::selection::SelectionKind;

    let mut runner = base()
        .add_card(make_silphymon("SILPHY-DNA", true))
        .add_card(make_silphymon("SILPHY-NODNA", false))
        .add_card(make_colored_lv4("YELLOW-LV4", CardColor::Yellow))
        .hand(0, &[CARD_ID, "SILPHY-NODNA", "SILPHY-DNA"])
        .memory(6)
        .start();
    let yellow = runner.place_on_field(0, "YELLOW-LV4", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    let aquilamon = runner.play(0, 0).expect("play Aquilamon");
    // Step 1: mandatory suspend of opponent Digimon.
    let susp = runner
        .pending_selection_view()
        .expect("opponent Digimon suspend prompt");
    assert_eq!(susp.kind, SelectionKind::OppField);
    runner
        .execute_action(susp.selecting_player, susp.valid_action_ids[0])
        .expect("suspend opponent Digimon");
    assert!(is_suspended(&runner, opp));

    // Step 2: optional hand pick of [Silphymon] that can DNA now.
    let hand_view = runner
        .pending_selection_view()
        .expect("optional Silphymon hand prompt");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert!(hand_view.is_optional);
    let slot_action = |id: &str| {
        let slot = runner.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&runner.game.card_data) == id)
            .unwrap() as u16;
        [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
            .into_iter()
            .find(|a| hand_view.valid_action_ids.contains(a))
    };
    assert!(
        slot_action("SILPHY-NODNA").is_none(),
        "non-DNA Silphymon must not be offered"
    );
    let silphy_pick = slot_action("SILPHY-DNA").expect("legal DNA Silphymon must be offered");
    runner
        .execute_action(0, silphy_pick)
        .expect("select Silphymon in hand");

    // Step 3: pick the two DNA materials (Yellow Lv.4 + Red/Green Aquilamon).
    let first = runner
        .pending_selection_view()
        .expect("first DNA material prompt");
    let yellow_act = encode_attack(0, yellow.index as u16);
    assert!(first.valid_action_ids.contains(&yellow_act));
    runner
        .execute_action(first.selecting_player, yellow_act)
        .expect("select Yellow Lv.4");

    let second = runner
        .pending_selection_view()
        .expect("second DNA material prompt");
    let aquila_act = encode_attack(0, aquilamon as u16);
    assert!(second.valid_action_ids.contains(&aquila_act));
    runner
        .execute_action(second.selecting_player, aquila_act)
        .expect("select Aquilamon");
    runner.auto_resolve().expect("settle DNA digivolve");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SILPHY-DNA"),
        "Silphymon must be DNA digivolved onto the battle area"
    );
}

/// Negative: on the opponent's turn, even with a legal Silphymon and pair,
/// the DNA leg is skipped ("if it's your turn").
#[test]
fn bt25_011_on_play_opponents_turn_skips_dna() {
    use digimon_engine::enums::{CardColor, EffectTiming};
    use digimon_engine::selection::TriggerSource;

    let mut runner = base()
        .add_card(make_silphymon("SILPHY-DNA", true))
        .add_card(make_colored_lv4("YELLOW-LV4", CardColor::Yellow))
        .hand(0, &["SILPHY-DNA"])
        .memory(6)
        .start();
    let _yellow = runner.place_on_field(0, "YELLOW-LV4", Some(0));
    let aquilamon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.turn_player_idx = 1;

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(aquilamon));
    runner.game.drain_effect_queue();

    let susp = runner
        .pending_selection_view()
        .expect("opponent Digimon suspend still fires on opponent's turn");
    runner
        .execute_action(susp.selecting_player, susp.valid_action_ids[0])
        .expect("suspend opponent Digimon");
    assert!(is_suspended(&runner, opp));
    assert!(
        runner.pending_selection().is_none(),
        "DNA leg must not be offered on opponent's turn"
    );
}

// ─── Section 3 — Behavior: inherited +2000 DP on your turn ───────────────────

#[test]
fn bt25_011_inherited_aura_buffs_carrier_on_your_turn() {
    let mut runner = base().memory(6).start();
    // Aquilamon as a digivolution source beneath a carrier (last id = top).
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    let dp = runner.effective_dp(carrier).expect("carrier DP");
    assert_eq!(
        dp, 9000,
        "inherited [Your Turn] +2000 must apply (carrier base 7000)"
    );
}

#[test]
fn bt25_011_inherited_aura_inactive_on_opponents_turn() {
    let mut runner = base().memory(6).start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.end_turn();

    let dp = runner.effective_dp(carrier).expect("carrier DP");
    assert_eq!(
        dp, 7000,
        "on the opponent's turn the [Your Turn] aura is inactive (base 7000)"
    );
}
