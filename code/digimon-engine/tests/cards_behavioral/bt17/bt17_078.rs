//! BT17-078 Omnimon — Digimon, Lv.7, White, DP 15000, Cost 9.
//! Traits: Holy Warrior, Royal Knight.
//! Evo: Lv.6 Red / Cost 5
//!
//! # Card text (cards.json — printed)
//!
//! [Hand] [Counter] <Blast DNA Digivolve ([WarGreymon] + [MetalGarurumon])>
//!   (1 of your specified Digimon and 1 specified card in hand may DNA digivolve
//!    into this card.)
//! <Raid>
//! <Blocker>
//! [On Play] [When Digivolving] If DNA digivolving, choose 1 of your opponent's
//!   Digimon and return all of your opponent's Digimon with the same level as it
//!   to the bottom of the deck. Then, delete 1 of your opponent's Digimon.
//!
//! Inherited: Ace Overflow <-5>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/White/BT17_078.cs
//!
//! # Source priority
//! Per CLAUDE.md: printed text > docs/RULES_CONTEXT.md > fandom > DCGO.
//! Where DCGO disagrees with printed text on whether the "Then, delete 1"
//! sentence is gated by "If DNA digivolving" (DCGO fires the delete arm
//! unconditionally; printed text gates both on the antecedent), we follow the
//! printed text.
//!
//! # Patterns this card exercises (test API §4.3)
//! - H5  Blocker (face-up declarative grant_keyword)
//! - H9  Raid (face-up declarative grant_keyword) — canonical example for §4.3
//! - H12 Blast (specifically Blast DNA Digivolve marker)
//! - H13 ACE Overflow (-5 metadata via `ace_overflow:`)
//! - G2  DNA digivolve alt-path (Lv.6 Greymon + Lv.6 Garurumon, cost 0)
//! - Shared OnPlay+WhenDigivolving body (DNA-gated bottom-deck-by-level + delete)
//!
//! The Blast DNA alt-path is active through `alt_paths.kind:
//! blast_dna_digivolve`, using the DCGO OnCounterTiming emitter shape: field
//! material first, then matching hand material, then zero-cost DNA into this
//! hand card. Clause 3 uses `bind_permanent_property` plus `level_eq_binding`
//! to capture the selected Digimon's level, bottom-deck every opponent Digimon
//! at that level, then surface the mandatory delete target prompt.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, CostDelta, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use std::sync::{Arc, Mutex};

// ─── Fixtures ────────────────────────────────────────────────────────────────

const YAML: &str = include_str!("../../../cards/bt17/BT17-078.yaml");

/// Compile the YAML (without going through the embedded pack lookup, which
/// requires the orchestrator to have re-built the embedded pack first).
fn compiled_bt17_078() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT17-078.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT17-078.yaml compiles");
    registry
        .lookup("BT17-078")
        .expect("BT17-078 in registry")
        .clone()
}

/// Standard fixture: BT17-078 (Omnimon) registered + room to play it.
fn omnimon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .memory(20)
        .build()
}

/// Build a Digimon CardData with explicit level + DP.
fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.card_kind = CardKind::Digimon;
    c
}

/// Recursively check whether a CompiledPredicate node (or any descendant)
/// satisfies `f`. Walks all_of/any_of/none_of/not branches.
fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    if f(p) {
        return true;
    }
    if p.all_of.iter().any(|q| pred_any(q, f)) {
        return true;
    }
    if p.any_of.iter().any(|q| pred_any(q, f)) {
        return true;
    }
    if p.none_of.iter().any(|q| pred_any(q, f)) {
        return true;
    }
    if let Some(ref n) = p.not {
        if pred_any(n, f) {
            return true;
        }
    }
    false
}

struct Bt17_078DnaOriginWitness {
    seen: Arc<Mutex<Vec<Option<bool>>>>,
}

impl CardEffect for Bt17_078DnaOriginWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::when_digivolving(card)
            .name("BT17-078 DNA-origin witness")
            .condition(|ctx| ctx.event_dna_origin() == Some(true))
            .process(move |ctx| {
                seen.lock().unwrap().push(ctx.event_dna_origin());
                ctx.gain_memory(1);
            })
            .build()]
    }
}

// ─── SECTION 1 — Structural assertions on CompiledCard ──────────────────────

/// The YAML must compile. Catches typos and predicate-name regressions.
#[test]
fn bt17_078_compiles() {
    let _compiled = compiled_bt17_078();
}

/// Card-level metadata: level 7, DP 15000, cost 9.
#[test]
fn bt17_078_card_metadata_matches_print() {
    let c = compiled_bt17_078();
    assert_eq!(c.level, Some(7), "Omnimon is Lv.7");
    assert_eq!(c.dp, Some(15000), "Omnimon is DP 15000");
    assert_eq!(c.cost, Some(9), "Omnimon costs 9 to play");
}

/// ACE Overflow: top-level `ace_overflow: -5` per Group 8 closure.
#[test]
fn bt17_078_ace_overflow_is_minus_5() {
    let c = compiled_bt17_078();
    assert_eq!(
        c.ace_overflow,
        Some(-5),
        "Inherited Ace Overflow <-5> must be modeled via ace_overflow: -5"
    );
}

/// Standard digivolve alt-path: Lv.6 Red / Cost 5.
#[test]
fn bt17_078_has_standard_digivolve_alt_path_lv6_red_cost_5() {
    let c = compiled_bt17_078();
    let standard = c.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && !p.ignore_requirements
            && matches!(p.cost, Some(CompiledCost::Literal(5)))
    });
    assert!(
        standard.is_some(),
        "Must have a standard Digivolve path at Lv.6 / Cost 5"
    );
    let path = standard.unwrap();
    let from = path
        .from
        .as_ref()
        .expect("Standard digivolve has a `from` predicate");
    assert!(
        pred_any(from, |q| q.level_eq == Some(6)),
        "Standard digivolve `from` must include level_eq: 6"
    );
}

/// DNA digivolve alt-path: Lv.6 Greymon-named + Lv.6 Garurumon-named, cost 0.
#[test]
fn bt17_078_has_dna_digivolve_alt_path_greymon_garurumon() {
    let c = compiled_bt17_078();
    let dna_paths: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(
        dna_paths.len(),
        1,
        "BT17-078 must have exactly one regular DNA digivolve alt-path \
         separate from the Blast DNA Counter route"
    );

    let path = dna_paths[0];
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "DNA digivolve cost is 0 (printed text, dna_costs.memory_cost)"
    );
    assert!(
        path.stacks_unsuspended,
        "DNA digivolve typically stacks both materials unsuspended"
    );
    assert_eq!(path.materials.len(), 2, "DNA requires exactly 2 materials");

    let has_greymon = path.materials.iter().any(|m| {
        pred_any(&m.filter, |q| {
            q.level_eq == Some(6) && q.name_contains.as_deref() == Some("Greymon")
        })
    });
    let has_garurumon = path.materials.iter().any(|m| {
        pred_any(&m.filter, |q| {
            q.level_eq == Some(6) && q.name_contains.as_deref() == Some("Garurumon")
        })
    });
    assert!(
        has_greymon,
        "DNA materials must include Lv.6 [Greymon]-named Digimon \
         (per cards.json dna_costs and DCGO PermanentCondition1)"
    );
    assert!(
        has_garurumon,
        "DNA materials must include Lv.6 [Garurumon]-named Digimon \
         (per cards.json dna_costs and DCGO PermanentCondition2)"
    );
}

#[test]
fn bt17_078_when_digivolving_dna_reads_dna_origin_payload() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(make_named_digimon("BT17-GREY-LV6", "Greymon source", 6, 11000))
        .add_card(make_named_digimon(
            "BT17-GARU-LV6",
            "Garurumon source",
            6,
            11000,
        ))
        .hand(0, &["BT17-078"])
        .memory(5)
        .start();

    let seen = Arc::new(Mutex::new(Vec::new()));
    runner.register_effect(
        "BT17-078",
        Arc::new(Bt17_078DnaOriginWitness {
            seen: Arc::clone(&seen),
        }),
    );

    let greymon = runner.place_on_field(0, "BT17-GREY-LV6", None);
    let garurumon = runner.place_on_field(0, "BT17-GARU-LV6", None);
    let hand_card = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(greymon, garurumon, hand_card, 0, true)
    };

    assert!(result.is_some(), "fixture DNA digivolve should succeed");
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[Some(true)],
        "BT17-078's DNA-gated WhenDigivolving body must see dna_origin=true"
    );
    assert_eq!(
        runner.game.memory,
        before + 1,
        "witness gains memory only when the DNA-origin predicate passes"
    );
}

#[test]
fn bt17_078_when_digivolving_standard_does_not_read_dna_origin() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(make_named_digimon("BT17-BASE-LV6", "Red level 6 base", 6, 11000))
        .hand(0, &["BT17-078"])
        .memory(5)
        .start();

    let seen = Arc::new(Mutex::new(Vec::new()));
    runner.register_effect(
        "BT17-078",
        Arc::new(Bt17_078DnaOriginWitness {
            seen: Arc::clone(&seen),
        }),
    );

    let base = runner.place_on_field(0, "BT17-BASE-LV6", None);
    let hand_card = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;

    let succeeded = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_digivolve_ignore_requirements(0, 0, base, CostDelta::Free)
    };

    assert!(succeeded, "fixture standard digivolve should succeed");
    assert!(
        seen.lock().unwrap().is_empty(),
        "standard digivolve must not satisfy BT17-078's dna_origin gate"
    );
    assert_eq!(
        runner.game.memory, before,
        "no witness memory gain should occur for a non-DNA digivolve"
    );
}

/// Blast DNA alt-path: [Hand][Counter] WarGreymon + MetalGarurumon, cost 0.
#[test]
fn bt17_078_has_blast_dna_digivolve_alt_path_wargreymon_metalgarurumon() {
    let c = compiled_bt17_078();
    let blast_dna = c.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::BlastDnaDigivolve
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.materials
                .iter()
                .any(|m| pred_any(&m.filter, |q| q.name_is.as_deref() == Some("WarGreymon")))
            && p.materials.iter().any(|m| {
                pred_any(&m.filter, |q| {
                    q.name_is.as_deref() == Some("MetalGarurumon")
                })
            })
    });
    assert!(
        blast_dna.is_some(),
        "BT17-078 must have a Blast DNA Digivolve alt-path with materials \
         [WarGreymon] + [MetalGarurumon] firing from hand at counter timing"
    );
}

/// Face-up Raid grant_keyword clause must be present.
#[test]
fn bt17_078_has_face_up_raid_grant_keyword() {
    let c = compiled_bt17_078();
    let raid = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Raid"
        )
    });
    assert!(
        raid,
        "BT17-078 must declare a face-up GrantKeyword(Raid) clause \
         (DCGO RaidSelfEffect at OnAllyAttack, isInheritedEffect: false)"
    );
}

/// Face-up Blocker grant_keyword clause must be present.
#[test]
fn bt17_078_has_face_up_blocker_grant_keyword() {
    let c = compiled_bt17_078();
    let blocker = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Blocker"
        )
    });
    assert!(
        blocker,
        "BT17-078 must declare a face-up GrantKeyword(Blocker) clause \
         (DCGO BlockerSelfStaticEffect at None, isInheritedEffect: false)"
    );
}

/// Clause 3 (DNA-gated bottom-deck + delete) is shared by OnPlay and
/// WhenDigivolving, with the printed "If DNA digivolving" antecedent modeled
/// as an `active_when` gate.
#[test]
fn bt17_078_has_on_play_when_digivolving_dna_gated_clause() {
    let c = compiled_bt17_078();
    let clause = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });
    let t = clause.expect(
        "Clause 3 must fire on both OnPlay and WhenDigivolving \
         (DCGO shared SetUpActivateClass body)",
    );
    assert_eq!(
        t.scope,
        CompiledScope::FaceUp,
        "Clause 3 is a face-up triggered effect on the played/digivolved Omnimon"
    );
    assert!(
        !t.optional,
        "Clause 3 has no \"you may\" — mandatory when condition holds"
    );
    assert!(!t.once_per_turn, "Clause 3 has no [Once Per Turn] marker");
    assert!(
        t.active_when
            .as_ref()
            .map(|p| pred_any(p, |q| q.dna_origin == Some(true)))
            .unwrap_or(false),
        "Clause 3 must be gated by `dna_origin: true` per the printed text \
         antecedent \"If DNA digivolving\"."
    );
}

// ─── SECTION 2 — Condition gating (positive + negative) ──────────────────────
//
// Clauses 1 (Raid) and 2 (Blocker) are unconditional declarative grants — no
// condition gating to test.
//
#[test]
fn bt17_078_on_play_dna_gate_blocks_when_played_normally_from_hand() {
    // Negative: playing Omnimon from hand (no DNA digivolve in the trigger
    // hashtable) must NOT install any pending selection from Clause 3.
    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV6", "OppLv6", 6, 8000));
    let _opp = runner.place_on_field(1, "OPP-LV6", Some(0));
    let omni = runner.place_on_field(0, "BT17-078", None);
    runner.fire_on_play(0, omni.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "[On Play] Clause 3 must skip when not DNA-digivolving (no IsJogress)."
    );
}

// ─── SECTION 3 — Behavioral outcome (integrated Blast DNA route) ─────────────

#[test]
fn bt17_078_counter_blast_dna_uses_wargreymon_and_metalgarurumon() {
    let mut wargreymon = make_named_digimon("WAR", "WarGreymon", 6, 11000);
    let mut metalgarurumon = make_named_digimon("METAL", "MetalGarurumon", 6, 11000);
    let attacker_card = make_named_digimon("ATK", "Attacker", 4, 1000);

    wargreymon.card_name = "WarGreymon".to_string();
    metalgarurumon.card_name = "MetalGarurumon".to_string();

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(wargreymon)
        .add_card(metalgarurumon)
        .add_card(attacker_card)
        .hand(1, &["BT17-078", "METAL"])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let target = runner.place_on_field(1, "WAR", Some(0));

    let result = runner.game.begin_attack(
        attacker,
        digimon_engine::selection::AttackTarget::Digimon(target),
        false,
    );
    assert_eq!(result, AttackResult::InProgress);

    let selection = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Blast DNA CounterTiming selection must be installed");
    assert_eq!(selection.selecting_player, 1);
    assert!(
        selection.valid_action_ids.contains(&encode_digivolve(0, 0)),
        "field WarGreymon must be selectable as the first Blast DNA material: {:?}",
        selection.valid_action_ids
    );

    runner
        .game
        .resolve_selection(1, encode_digivolve(0, 0))
        .expect("select field WarGreymon");
    let selection = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Blast DNA hand-material selection must be installed");
    assert_eq!(selection.valid_action_ids, vec![PLAY_HAND_START + 1]);

    runner
        .game
        .resolve_selection(1, PLAY_HAND_START + 1)
        .expect("select hand MetalGarurumon");

    assert_eq!(runner.game.player(1).hand.len(), 0);
    assert_eq!(runner.game.player(1).battle_area.len(), 1);
    let stack_ids: Vec<_> = runner.game.player(1).battle_area[0]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        stack_ids,
        vec!["WAR", "METAL", "BT17-078"],
        "Blast DNA must stack field WarGreymon, hand MetalGarurumon, then BT17-078"
    );
}

#[test]
fn bt17_078_blast_dna_bottom_decks_same_level_then_prompts_delete() {
    let mut wargreymon = make_named_digimon("WAR", "WarGreymon", 6, 11000);
    let mut metalgarurumon = make_named_digimon("METAL", "MetalGarurumon", 6, 11000);
    let attacker_card = make_named_digimon("ATK", "Attacker", 5, 7000);
    let opp_peer_lv5 = make_named_digimon("OPP-PEER-LV5", "OppPeer5", 5, 6000);
    let opp_survivor_lv6 = make_named_digimon("OPP-LV6", "Opp6", 6, 9000);

    wargreymon.card_name = "WarGreymon".to_string();
    metalgarurumon.card_name = "MetalGarurumon".to_string();

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(wargreymon)
        .add_card(metalgarurumon)
        .add_card(attacker_card)
        .add_card(opp_peer_lv5)
        .add_card(opp_survivor_lv6)
        .hand(1, &["BT17-078", "METAL"])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let _peer = runner.place_on_field(0, "OPP-PEER-LV5", Some(0));
    let _survivor = runner.place_on_field(0, "OPP-LV6", Some(0));
    let target = runner.place_on_field(1, "WAR", Some(0));
    let p0_deck_before = runner.deck_size(0);

    let result = runner.game.begin_attack(
        attacker,
        digimon_engine::selection::AttackTarget::Digimon(target),
        false,
    );
    assert_eq!(result, AttackResult::InProgress);

    runner
        .game
        .resolve_selection(1, encode_digivolve(0, 0))
        .expect("select field WarGreymon");
    runner
        .game
        .resolve_selection(1, PLAY_HAND_START + 1)
        .expect("select hand MetalGarurumon");

    let choose_level = runner
        .pending_selection()
        .expect("BT17-078 DNA-origin clause must ask for the level anchor");
    assert_eq!(choose_level.kind, SelectionKind::OppField);
    assert_eq!(choose_level.selecting_player, 1);
    let choose_level_action = choose_level.valid_action_ids[0];
    runner
        .game
        .resolve_selection(1, choose_level_action)
        .expect("choose a Lv.5 opponent Digimon");

    assert_eq!(
        runner.deck_size(0),
        p0_deck_before + 2,
        "chosen Lv.5 and same-level peer must both go to opponent deck bottom"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only the nonmatching Lv.6 opponent Digimon should remain before delete"
    );

    let delete_prompt = runner
        .pending_selection()
        .expect("delete prompt must install after bottom-decking same-level Digimon");
    assert_eq!(delete_prompt.kind, SelectionKind::OppField);
    assert_eq!(delete_prompt.selecting_player, 1);
    assert!(
        !delete_prompt.is_optional,
        "the printed delete step is mandatory once an opponent Digimon remains"
    );
}

// ─── SECTION 5 — OPT enforcement ─────────────────────────────────────────────
//
// No clause on BT17-078 is `[Once Per Turn]`. Skipped intentionally.
