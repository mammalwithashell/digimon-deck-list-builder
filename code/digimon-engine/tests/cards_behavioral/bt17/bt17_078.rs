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
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardColor, CardKind, CostDelta, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId,
};
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
        .add_card(make_named_digimon(
            "BT17-GREY-LV6",
            "Greymon source",
            6,
            11000,
        ))
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
        .add_card(make_named_digimon(
            "BT17-BASE-LV6",
            "Red level 6 base",
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
/// Distinct from the regular DNA path's broader Greymon/Garurumon name family.
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

#[test]
fn bt17_078_counter_blast_dna_uses_exact_wargreymon_and_metalgarurumon() {
    let mut wargreymon = make_named_digimon("BT17-WARGREYMON", "WarGreymon", 6, 12000);
    wargreymon.colors = vec![CardColor::Red];
    let mut metalgarurumon = make_named_digimon("BT17-METALGARURUMON", "MetalGarurumon", 6, 12000);
    metalgarurumon.colors = vec![CardColor::Blue];
    let mut attacker = make_named_digimon("BT17-ATTACKER", "Attacker", 6, 17000);
    attacker.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(wargreymon)
        .add_card(metalgarurumon)
        .add_card(attacker)
        .hand(1, &["BT17-078", "BT17-METALGARURUMON"])
        .start();

    let attacking = runner.place_on_field(0, "BT17-ATTACKER", Some(0));
    let wargrey = runner.place_on_field(1, "BT17-WARGREYMON", Some(0));

    let result = runner.attack_digimon(attacking, wargrey, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let prompt = runner
        .pending_selection()
        .expect("Counter window should offer BT17-078 Blast DNA");
    assert!(prompt.valid_action_ids.contains(&DNA_DIGIVOLVE_START));
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT17-078 for Counter Blast DNA");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    assert_eq!(
        runner
            .pending_selection()
            .expect("field material prompt")
            .valid_action_ids,
        vec![0]
    );

    runner
        .execute_action(1, 0)
        .expect("choose WarGreymon as field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );
    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose MetalGarurumon as hand material");

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BT17-078"
    );
    assert!(evolved
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "BT17-METALGARURUMON"));
    assert_eq!(runner.hand_size(1), 0);
}

#[test]
fn bt17_078_counter_blast_dna_rejects_broad_greymon_garurumon_names() {
    let mut greymon = make_named_digimon("BT17-GREYMON", "Greymon", 6, 8000);
    greymon.colors = vec![CardColor::Red];
    let mut garurumon = make_named_digimon("BT17-GARURUMON", "Garurumon", 6, 8000);
    garurumon.colors = vec![CardColor::Blue];
    let mut attacker = make_named_digimon("BT17-ATTACKER", "Attacker", 6, 17000);
    attacker.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(greymon)
        .add_card(garurumon)
        .add_card(attacker)
        .hand(1, &["BT17-078", "BT17-GARURUMON"])
        .start();

    let attacking = runner.place_on_field(0, "BT17-ATTACKER", Some(0));
    let greymon = runner.place_on_field(1, "BT17-GREYMON", Some(0));

    let result = runner.attack_digimon(attacking, greymon, false);
    assert_ne!(
        runner.current_phase(),
        GamePhase::CounterTiming,
        "printed Blast DNA marker requires exact WarGreymon + MetalGarurumon, not broad normal DNA names"
    );
    assert_ne!(result, digimon_engine::combat::AttackResult::Invalid);
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
        selection.valid_action_ids.contains(&DNA_DIGIVOLVE_START),
        "BT17-078 must be selectable as the Counter Blast DNA result card: {:?}",
        selection.valid_action_ids
    );

    runner
        .game
        .resolve_selection(1, DNA_DIGIVOLVE_START)
        .expect("select BT17-078 as Counter Blast DNA result");
    let selection = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Blast DNA field-material selection must be installed");
    assert_eq!(selection.valid_action_ids, vec![0]);
    runner
        .game
        .resolve_selection(1, 0)
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
        .resolve_selection(1, DNA_DIGIVOLVE_START)
        .expect("select BT17-078 as Counter Blast DNA result");
    runner
        .game
        .resolve_selection(1, 0)
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
// NOTE (AUDIT 2026-05-11): The printed BT17-078 text DOES contain a
// [Once Per Turn] clause: "[When Attacking] [Once Per Turn] You may trash 1
// card from your hand. If you do, this Digimon gets +1000 DP, <Jamming>, and
// may attack again until end of opponent's next turn."
//
// The claim "No clause on BT17-078 is [Once Per Turn]" is INCORRECT per the
// printed card text. The following tests are marked #[ignore] pending YAML
// correction (AUDITED-DRIFT verdict for the full card spec).

// ─── SECTION 6 — Faithfulness audit: missing / drifted clauses ───────────────
//
// The YAML and tests above implement:
//   "[On Play][When Digivolving] If DNA digivolving, choose 1 of your
//    opponent's Digimon and return all of your opponent's Digimon with the
//    same level as it to the bottom of the deck. Then, delete 1 of your
//    opponent's Digimon."
//
// The PRINTED BT17-078 text (task description) contains:
//   Clause 4: "[On Play][When Digivolving] If DNA digivolved into using a
//     [WarGreymon] and a [MetalGarurumon], the chosen [WarGreymon] gains
//     <Piercing> and <Security Attack +2> for the turn."
//   Clause 5: "[When Attacking] [Once Per Turn] You may trash 1 card from
//     your hand. If you do, this Digimon gets +1000 DP, <Jamming>, and may
//     attack again until end of opponent's next turn."
//
// Clauses 4 and 5 are entirely absent from the YAML.
// The current Clause 3 text (bottom-deck-by-level + delete) does NOT appear
// in the printed BT17-078 text at all — it belongs to a different card.
//
// Verdict: AUDITED-DRIFT (wrong-card-text substitution in Clause 3 +
//   two wholly absent clauses: Clause 4 and Clause 5).
//
// DSL vocabulary gap for Clause 4:
//   "the chosen [WarGreymon] gains <Piercing> and <Security Attack +2>"
//   After DNA digivolve, WarGreymon is a digivolution SOURCE CARD under
//   Omnimon, not a battle-area permanent. The DSL `grant_keyword` step
//   (step-form) only accepts a `PermanentHandle` target via `resolve_binding_ref`.
//   `select_material` yields a `CardHandle` binding; there is no DSL step to
//   grant a keyword to a named source card within a digivolution stack.
//   This is a new DSL vocab gap: `grant_keyword_to_source`.
//
// DSL vocabulary for Clause 5:
//   - "You may trash 1 card from your hand": `select_hand { optional: true }` +
//     `trash_from_hand_by_index` — both verbs exist.
//   - "+1000 DP": `add_dp_modifier: { target: self, value: 1000, expiry: end_of_turn }` — exists.
//   - "<Jamming>": `grant_keyword: { target: self, keyword: Jamming, expiry: end_of_turn }` — exists.
//   - "may attack again until end of opponent's next turn": install a `MayAttack`
//     modifier with `end_of_opponents_next_turn` expiry. The modifier type
//     `MayAttack` exists in the engine; `end_of_opponents_next_turn` is a valid
//     expiry in the validator. This is representable as:
//       `add_modifier: { target: self, modifier: MayAttack, value: 1, expiry: end_of_opponents_next_turn }`
//     Confirm: `MayAttack` is listed in `modifier_map.rs` / `KNOWN_MODIFIER_KEYS`
//     before authoring the YAML.

// ─── Structural: YAML is missing the when_attacking OPT clause ───────────────

/// The YAML currently has no `when_attacking` triggered clause.
/// Per the printed text, BT17-078 must have exactly one `when_attacking`
/// triggered clause (face-up, optional, once_per_turn).
/// This test documents the drift: it PASSES only if the YAML is corrected.
#[test]
#[ignore = "AUDITED-DRIFT: YAML implements wrong card text. BT17-078's When \
            Attacking OPT clause is entirely absent. Fix the YAML before \
            removing this #[ignore]."]
fn bt17_078_has_when_attacking_opt_clause() {
    let c = compiled_bt17_078();
    let when_atk: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .filter(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .collect();
    assert_eq!(
        when_atk.len(),
        1,
        "BT17-078 must have exactly 1 WhenAttacking triggered clause (printed text Clause 5)"
    );
    let clause = when_atk[0];
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 5 is a face-up (own) When Attacking effect"
    );
    assert!(
        clause.optional,
        "Clause 5 opens with 'You may' — must be optional"
    );
    assert!(
        clause.once_per_turn,
        "Clause 5 has [Once Per Turn] marker"
    );
}

/// Structural: On Play / When Digivolving clause must be gated by BOTH
/// WarGreymon AND MetalGarurumon names, not merely dna_origin.
/// Per printed text: "If DNA digivolved into using a [WarGreymon] and a
/// [MetalGarurumon]" — the gate checks specific named sources.
/// The current YAML gates only on `dna_origin: true` without name checks.
/// This test documents the drift; it will pass only once the YAML is corrected.
#[test]
#[ignore = "AUDITED-DRIFT: YAML gates Clause 3 only on dna_origin:true, not on \
            WarGreymon+MetalGarurumon name check. Printed text requires both \
            specific names. Fix the YAML before removing this #[ignore]."]
fn bt17_078_on_play_dna_gate_requires_wargreymon_and_metalgarurumon_names() {
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
        })
        .expect("On Play + When Digivolving clause must exist");

    let gating = clause
        .active_when
        .as_ref()
        .expect("Clause must have active_when gate");

    // The gate must check for a WarGreymon-named source AND a
    // MetalGarurumon-named source as DNA materials, not just dna_origin.
    let checks_wargreymon = pred_any(gating, |q| {
        q.name_is.as_deref() == Some("WarGreymon")
            || q.name_contains.as_deref() == Some("WarGreymon")
    });
    let checks_metalgarurumon = pred_any(gating, |q| {
        q.name_is.as_deref() == Some("MetalGarurumon")
            || q.name_contains.as_deref() == Some("MetalGarurumon")
    });
    assert!(
        checks_wargreymon,
        "active_when must include a WarGreymon name predicate per printed text"
    );
    assert!(
        checks_metalgarurumon,
        "active_when must include a MetalGarurumon name predicate per printed text"
    );
}

// ─── Behavioral: Clause 4 — WarGreymon source gains Piercing + SecAttk+2 ─────

/// [On Play][When Digivolving] — positive: after DNA digivolve using WarGreymon
/// + MetalGarurumon, Omnimon's WarGreymon digivolution source should carry
/// Piercing and SecurityAttackPlus (+2) until end of turn.
///
/// NOTE: This test is blocked by a DSL vocab gap (`grant_keyword_to_source`
/// does not exist) in addition to the YAML drift. See the gap entry below.
#[test]
#[ignore = "AUDITED-DRIFT + DSL-VOCAB-GAP: YAML implements wrong card text \
            AND the DSL lacks `grant_keyword_to_source` to target a named \
            digivolution source card. See dsl-vocab-gaps.md entry BT17-078 \
            Clause 4 WarGreymon grant. Fix YAML + add DSL verb before \
            removing this #[ignore]."]
fn bt17_078_dna_with_wargreymon_grants_piercing_and_security_attack_plus_2_to_wargreymon_source() {
    let mut wargreymon = make_named_digimon("WAR", "WarGreymon", 6, 12000);
    wargreymon.colors = vec![CardColor::Red];
    let mut metalgarurumon = make_named_digimon("METAL", "MetalGarurumon", 6, 12000);
    metalgarurumon.colors = vec![CardColor::Blue];
    let attacker = make_named_digimon("ATK", "Attacker", 5, 7000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(wargreymon)
        .add_card(metalgarurumon)
        .add_card(attacker)
        .hand(1, &["BT17-078", "METAL"])
        .memory(5)
        .start();

    let atk_perm = runner.place_on_field(0, "ATK", Some(0));
    let wargrey_perm = runner.place_on_field(1, "WAR", Some(0));

    // Trigger Counter Blast DNA digivolve: ATK attacks wargrey_perm
    let result = runner.game.begin_attack(
        atk_perm,
        digimon_engine::selection::AttackTarget::Digimon(wargrey_perm),
        false,
    );
    assert_eq!(result, AttackResult::InProgress);
    runner
        .game
        .resolve_selection(1, DNA_DIGIVOLVE_START)
        .expect("select BT17-078 as Counter Blast DNA");
    runner
        .game
        .resolve_selection(1, 0)
        .expect("select WarGreymon as field material");
    runner
        .game
        .resolve_selection(1, PLAY_HAND_START + 1)
        .expect("select MetalGarurumon as hand material");

    // After DNA digivolve: Omnimon (BT17-078) is in battle_area[0] of player 1.
    // The WarGreymon source card (index 0 in the stack) must have Piercing and
    // SecurityAttackPlus granted for the turn.
    //
    // Since granting keywords to source cards requires a DSL verb that doesn't
    // exist yet (`grant_keyword_to_source`), the following check is pending.
    let omnimon_perm = runner.perm_handle(1, 0);

    // The WarGreymon source should carry Piercing (inherited up to Omnimon)
    assert!(
        runner.game.has_keyword(omnimon_perm, Keyword::Piercing),
        "Omnimon must have Piercing (inherited from WarGreymon source) after \
         DNA digivolve with WarGreymon + MetalGarurumon"
    );
    // Security Attack +2 check: the SecurityAttackChange modifier delta should be +2.
    let sa_delta = runner
        .game
        .modifiers
        .sum(omnimon_perm, ModifierType::SecurityAttackChange);
    assert_eq!(
        sa_delta, 2,
        "SecurityAttack +2 must be applied via SecurityAttackChange modifier; got {sa_delta}"
    );
}

// ─── Behavioral: Clause 5 — When Attacking OPT trash hand → +DP/Jamming/re-attack

/// [When Attacking][OPT] positive branch: trash 1 hand card → Omnimon gains
/// +1000 DP, Jamming, and a MayAttack grant until end of opponent's next turn.
#[test]
#[ignore = "AUDITED-DRIFT: When Attacking OPT clause is absent from the YAML. \
            Fix the YAML before removing this #[ignore]."]
fn bt17_078_when_attacking_opt_trash_hand_grants_dp_jamming_attack_again() {
    let hand_card = make_named_digimon("HAND-FILL", "HandFill", 3, 2000);
    let opp_perm_card = make_named_digimon("OPP", "OppDig", 5, 5000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(hand_card)
        .add_card(opp_perm_card)
        .hand(0, &["HAND-FILL"])
        .memory(10)
        .start();

    let omnimon = runner.place_on_field(0, "BT17-078", None);
    runner.place_on_field(1, "OPP", Some(0));
    runner.game.players[0].unsuspend_all();

    let dp_before = runner
        .game
        .effective_dp(omnimon)
        .expect("Omnimon must be on field");
    let hand_before = runner.hand_size(0);

    // Trigger When Attacking by attacking the opponent player.
    let result = runner.attack_player(omnimon, 1, false);
    // Expect a pending selection for the OPT "You may trash 1 from hand".
    let sel = runner
        .pending_selection()
        .expect("When Attacking OPT must install a pending selection");
    assert!(
        sel.is_optional,
        "OPT selection must be optional (you may)"
    );
    // Resolve: yes, trash the hand card (first valid action).
    let trash_action = sel.valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, trash_action)
        .expect("choose hand card to trash");

    // Verify hand decreased by 1.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "1 hand card must be trashed as cost"
    );
    // Verify +1000 DP.
    let dp_after = runner.game.effective_dp(omnimon).expect("still on field");
    assert_eq!(
        dp_after,
        dp_before + 1000,
        "Omnimon must gain +1000 DP after paying the trash cost; \
         before={dp_before}, after={dp_after}"
    );
    // Verify Jamming granted.
    assert!(
        runner.game.has_keyword(omnimon, Keyword::Jamming),
        "Omnimon must have Jamming after paying the trash cost"
    );
    // Verify MayAttack modifier is present (the "may attack again" grant).
    let has_may_attack = runner.game.modifiers.has(omnimon, ModifierType::MayAttack);
    assert!(
        has_may_attack,
        "Omnimon must have MayAttack modifier (may attack again until end of \
         opponent's next turn)"
    );
}

/// [When Attacking][OPT] negative: if the player has no hand cards, the clause
/// can still be activated but the conditional "if you do" body must not fire
/// (the cost cannot be paid). Alternatively: if zero hand cards, the
/// selection must not install (gate on hand non-empty).
#[test]
#[ignore = "AUDITED-DRIFT: When Attacking OPT clause is absent from the YAML. \
            Fix the YAML before removing this #[ignore]."]
fn bt17_078_when_attacking_opt_with_empty_hand_does_not_fire_body() {
    let opp_perm_card = make_named_digimon("OPP", "OppDig", 5, 5000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(opp_perm_card)
        .memory(10)
        .start();

    // No hand cards for player 0.
    let omnimon = runner.place_on_field(0, "BT17-078", None);
    runner.place_on_field(1, "OPP", Some(0));
    runner.game.players[0].unsuspend_all();
    assert_eq!(runner.hand_size(0), 0, "test requires empty hand");

    let dp_before = runner.game.effective_dp(omnimon).expect("Omnimon on field");

    let _ = runner.attack_player(omnimon, 1, false);

    // With an empty hand, either:
    //   (a) no OPT selection installs (gate on hand.len() > 0), or
    //   (b) selection installs but the cost-pay resolves as no-op.
    // In either case, DP must remain unchanged.
    let _ = runner.auto_resolve();
    let dp_after = runner.game.effective_dp(omnimon).unwrap_or(dp_before);
    assert_eq!(
        dp_after, dp_before,
        "DP must not increase if no hand card was trashed"
    );
    assert!(
        !runner.game.has_keyword(omnimon, Keyword::Jamming),
        "Jamming must not be granted when the trash cost cannot be paid"
    );
}

// ─── OPT lockout: Clause 5 — second activation same turn must be gated ────────

/// OPT lockout: second When Attacking activation same turn must NOT re-install
/// the OPT selection or re-apply the DP/Jamming buffs.
#[test]
#[ignore = "AUDITED-DRIFT: When Attacking OPT clause is absent from the YAML. \
            Fix the YAML before removing this #[ignore]."]
fn bt17_078_when_attacking_opt_locks_out_second_attack_same_turn() {
    let hand_card1 = make_named_digimon("HC1", "HandCard1", 3, 2000);
    let hand_card2 = make_named_digimon("HC2", "HandCard2", 3, 2000);
    let opp_perm1 = make_named_digimon("OPP1", "OppDig1", 5, 5000);
    let opp_perm2 = make_named_digimon("OPP2", "OppDig2", 5, 5000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(hand_card1)
        .add_card(hand_card2)
        .add_card(opp_perm1)
        .add_card(opp_perm2)
        .hand(0, &["HC1", "HC2"])
        .memory(10)
        .start();

    let omnimon = runner.place_on_field(0, "BT17-078", None);
    runner.place_on_field(1, "OPP1", Some(0));
    runner.place_on_field(1, "OPP2", Some(0));
    runner.game.players[0].unsuspend_all();

    // First attack: OPT fires, player trashes HC1, gains +1000 DP + Jamming.
    let _ = runner.attack_player(omnimon, 1, false);
    if let Some(sel) = runner.pending_selection() {
        let action = sel.valid_action_ids[0];
        let _ = runner.game.resolve_selection(0, action);
    }
    let _ = runner.auto_resolve();
    let dp_after_first = runner.game.effective_dp(omnimon).unwrap_or(15000);

    // Second attack (same turn, OPT lockout): no OPT selection must install.
    runner.game.players[0].unsuspend_all(); // re-enable for test
    let _ = runner.attack_player(omnimon, 1, false);
    // Drain without resolving any OPT prompt (there should be none).
    let _ = runner.auto_resolve();
    let dp_after_second = runner.game.effective_dp(omnimon).unwrap_or(dp_after_first);

    assert_eq!(
        dp_after_second, dp_after_first,
        "second attack same turn: OPT lockout must prevent the +1000 DP grant \
         from firing again; dp_after_first={dp_after_first}, dp_after_second={dp_after_second}"
    );
}

/// OPT clears after end_turn: attack on the next turn re-enables the OPT.
#[test]
#[ignore = "AUDITED-DRIFT: When Attacking OPT clause is absent from the YAML. \
            Fix the YAML before removing this #[ignore]."]
fn bt17_078_when_attacking_opt_clears_after_end_turn() {
    let hand_card1 = make_named_digimon("HC1", "HandCard1", 3, 2000);
    let hand_card2 = make_named_digimon("HC2", "HandCard2", 3, 2000);
    let opp_perm = make_named_digimon("OPP", "OppDig", 5, 5000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-078 YAML loads")
        .add_card(hand_card1)
        .add_card(hand_card2)
        .add_card(opp_perm)
        .hand(0, &["HC1", "HC2"])
        .memory(10)
        .start();

    let omnimon = runner.place_on_field(0, "BT17-078", None);
    runner.place_on_field(1, "OPP", Some(0));
    runner.game.players[0].unsuspend_all();

    // Turn 1 attack: fire OPT, trash HC1.
    let _ = runner.attack_player(omnimon, 1, false);
    if let Some(sel) = runner.pending_selection() {
        let action = sel.valid_action_ids[0];
        let _ = runner.game.resolve_selection(0, action);
    }
    let _ = runner.auto_resolve();
    let dp_t1 = runner.game.effective_dp(omnimon).unwrap_or(15000);

    // End turn (P0→P1→P0 round-trip) to clear OPT state.
    runner.end_turn();
    runner.end_turn();

    runner.game.players[0].unsuspend_all();

    // Turn 2 attack: OPT should fire again.
    let _ = runner.attack_player(omnimon, 1, false);
    let sel_t2 = runner.pending_selection();
    assert!(
        sel_t2.is_some(),
        "OPT must re-arm after end_turn round-trip; no selection on second turn attack"
    );
    if let Some(sel) = sel_t2 {
        let action = sel.valid_action_ids[0];
        let _ = runner.game.resolve_selection(0, action); // trash HC2
    }
    let dp_t2 = runner.game.effective_dp(omnimon).unwrap_or(dp_t1);
    assert_eq!(
        dp_t2,
        dp_t1 + 1000,
        "OPT re-armed after end_turn: +1000 DP must apply again on turn 2; \
         dp_t1={dp_t1}, dp_t2={dp_t2}"
    );
}
