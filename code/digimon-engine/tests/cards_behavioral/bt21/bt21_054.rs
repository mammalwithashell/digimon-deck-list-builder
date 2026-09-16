//! BT21-054 Shotmon — Digimon (Appmon), Lv.3, Black, DP 1000, Cost 3.
//! Traits: Stnd. / Appmon / Shooting. Attribute: Game.
//!
//! # Card text (data/card_bundles/BT21-054.md — official Bandai DB, confirmed
//! against the card image)
//!
//! Digivolve: Black Lv.2 / cost 0; and
//! [Digivolve] Lv.2 w/[Three Musketeers] in text or w/[Appmon] trait: Cost 0.
//!
//! [On Play] By trashing 1 card with the [Appmon] or [Three Musketeers] trait
//! from any of your Digimon's digivolution cards, ＜De-Digivolve 1＞ 1 of your
//! opponent's Digimon.
//! Special Rule: +2000 DP (link box)
//! Inherited: ＜Link＞ [Appmon] trait: Cost 1  [When Linking] Delete 1 of your
//! opponent's Digimon with a play cost of 3 or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_054.cs
//! - Link Condition: AddSelfLinkConditionStaticEffect(HasAppmonTraits, cost 1).
//! - Alt Digivolution: IsLevel2 && (EqualsTraits("Appmon") || HasText("Three Musketeers")), cost 0.
//! - On Play (`-1, true` = optional): SelectTrashDigivolutionCards over ANY own
//!   Digimon (isFromOnly1Permanent: false, canNoTrash: false) filtered
//!   Appmon-or-TM trait; if trashed → SelectPermanentEffect over opponent
//!   Digimon (canNoSelect: false) → IDegeneration(1).
//! - WhenLinked (SetIsLinkedEffect): mandatory Destroy select over opponent
//!   Digimon with HasPlayCost && GetCostItself ≤ 3.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - DigiLink self link-condition + linked DP aura (link box +2000)
//! - E optional [On Play] with a union-material trash cost → De-Digivolve 1
//! - when: when_linked (linked scope) delete with a play-cost gate

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};

const CARD_ID: &str = "BT21-054";

fn digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-054 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "DECK-PAD"))
        .add_card(digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(digimon("HOST-PLAIN", 4, 4000, 4, &["Beast"]))
        .add_card(digimon("SRC-APP", 3, 2000, 3, &["Appmon"]))
        .add_card(digimon("SRC-TM", 3, 2000, 3, &["Three Musketeers"]))
        .add_card(digimon("SRC-PLAIN", 3, 2000, 3, &["Beast"]))
        .add_card(digimon("OPP-BASE", 3, 2000, 3, &["Beast"]))
        .add_card(digimon("OPP-TOP", 4, 4000, 4, &["Beast"]))
        .add_card(digimon("OPP-COST3", 3, 2000, 3, &["Beast"]))
        .add_card(digimon("OPP-COST4", 4, 4000, 4, &["Beast"]))
        .deck(0, &["DECK-PAD"; 5])
        .deck(1, &["DECK-PAD"; 5])
}

fn fire_link_onto_host(runner: &mut DebugRunner, host: PermanentHandle) {
    let linked = runner.push_linked_owned(host, CARD_ID, 0);
    for pid in 0..2u8 {
        runner.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: linked,
            },
        );
    }
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt21_054_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT21-054 in embedded pack");
    assert_eq!(card.name, "Shotmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    for t in ["Stnd.", "Appmon", "Shooting"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
}

#[test]
fn bt21_054_alt_paths_black_lv2_and_appmon_or_tm_text_lv2_cost0() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(0))
                && p.from
                    .as_ref()
                    .is_some_and(|f| f.level_eq == Some(2) && f.color_is.is_some())
        }),
        "Black Lv.2 / 0 circle"
    );
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(0))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(2)
                        && f.any_of
                            .iter()
                            .any(|q| q.trait_has.as_deref() == Some("Appmon"))
                        && f.any_of
                            .iter()
                            .any(|q| q.in_text_contains.as_deref() == Some("Three Musketeers"))
                })
        }),
        "[Digivolve] Lv.2 w/[Three Musketeers] in text or w/[Appmon] trait: Cost 0"
    );
}

#[test]
fn bt21_054_link_condition_aura_and_clause_shapes() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 1
    )), "<Link> [Appmon] trait: Cost 1");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. })
                if *scope == CompiledScope::Linked && *dp_modifier == Some(2000)
        )),
        "link box +2000 DP aura"
    );
    let op = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnPlay] => Some(t),
            _ => None,
        })
        .expect("[On Play]");
    assert!(op.optional, "DCGO isOptional: true");
    let wl = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::WhenLinked] => Some(t),
            _ => None,
        })
        .expect("[When Linking]");
    assert_eq!(wl.scope, CompiledScope::Linked);
    assert!(!wl.optional);
}

// ─── Section 2 — [On Play] trash Appmon/TM source → De-Digivolve 1 ──────────

fn on_play_runner() -> (
    DebugRunner,
    PermanentHandle,
    PermanentHandle,
    PermanentHandle,
) {
    let mut runner = base().memory(5).start();
    let shot = runner.place_on_field(0, CARD_ID, Some(0));
    let host_a = runner.place_on_field(0, "HOST-APP", Some(0));
    runner.push_source(host_a, "SRC-APP");
    let host_b = runner.place_on_field(0, "HOST-PLAIN", Some(0));
    runner.push_source(host_b, "SRC-TM");
    runner.push_source(host_b, "SRC-PLAIN");
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    (runner, shot, host_b, opp)
}

#[test]
fn bt21_054_on_play_trashes_source_from_any_digimon_then_de_digivolves() {
    let (mut runner, shot, host_b, opp) = on_play_runner();
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(shot));
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::MATERIAL
        })
    );
    assert!(
        !runner.pending_is_optional(),
        "DCGO canNoTrash: false once activated"
    );
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "SRC-APP (under HOST-APP) + SRC-TM (under HOST-PLAIN); SRC-PLAIN excluded"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional());
    runner.auto_resolve().expect("de-digivolve");

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-BASE",
        "opponent Digimon lost its top card (De-Digivolve 1)"
    );
    let total_sources: usize = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len())
        .sum();
    assert_eq!(
        total_sources,
        1 + 2 + 3 - 1,
        "exactly one source trashed across the field (Shotmon 1 + HOST-APP 2 + HOST-PLAIN 3, minus 1)"
    );
}

#[test]
fn bt21_054_on_play_decline_trashes_nothing() {
    let (mut runner, shot, _host_b, opp) = on_play_runner();
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(shot));
    runner.game.drain_effect_queue();
    runner.decline_optional_trigger().expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        2
    );
}

#[test]
fn bt21_054_on_play_no_eligible_source_no_prompt() {
    let mut runner = base().memory(5).start();
    let shot = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "HOST-PLAIN", Some(0));
    runner.push_source(host, "SRC-PLAIN");
    runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(shot));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "unexpected prompt: {:?}",
        runner.pending_selection_view()
    );
}

// ─── Section 3 — Link box + [When Linking] ────────────────────────────────────

#[test]
fn bt21_054_linked_dp_bonus_2000_reaches_host() {
    let mut runner = base().memory(5).start();
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    let dp_before = runner.game.effective_dp(host).unwrap();
    runner.push_linked_owned(host, CARD_ID, 0);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.game.effective_dp(host), Some(dp_before + 2000));
}

#[test]
fn bt21_054_when_linked_deletes_opp_digimon_with_cost_up_to_3() {
    let mut runner = base().memory(5).start();
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    runner.place_on_field(1, "OPP-COST3", Some(0));
    runner.place_on_field(1, "OPP-COST4", Some(0));
    fire_link_onto_host(&mut runner, host);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the play-cost-3 Digimon is eligible"
    );
    runner.auto_resolve().ok();
    assert_eq!(runner.battle_area_size(1), 1);
    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .all(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-COST4"));
}

#[test]
fn bt21_054_when_linked_no_prompt_without_eligible_target() {
    let mut runner = base().memory(5).start();
    let host = runner.place_on_field(0, "HOST-APP", Some(0));
    runner.place_on_field(1, "OPP-COST4", Some(0));
    fire_link_onto_host(&mut runner, host);
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(1), 1);
}
