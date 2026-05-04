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
//! - H12 Blast (specifically Blast DNA Digivolve marker — BLOCKED, see below)
//! - H13 ACE Overflow (-5 metadata via `ace_overflow:`)
//! - G2  DNA digivolve alt-path (Lv.6 Greymon + Lv.6 Garurumon, cost 0)
//! - Shared OnPlay+WhenDigivolving body (DNA-gated bottom-deck-by-level + delete)
//!
//! # Known gaps that block behavioral coverage of Clause 3
//!
//! - **G-BIND-SELECTED-PROPERTY-FOR-EACH** (qa/dsl-vocab-gaps.md, BT17-078 entry):
//!   no DSL pattern for "bind one selected permanent's property (level), then
//!   for-each opponent permanent matching that bound property". The
//!   `level_eq` predicate is `Option<u8>` literal-only; no `level_eq_binding`
//!   form exists. Suggested syntax in the gap entry:
//!     - bind_selected: { name: chosen_level, selector: opponent_digimon, property: level }
//!     - for_each_opponent_permanent: { where: { level_eq: "$chosen_level" }, do: return_to_deck_bottom }
//!
//! - **G-BLAST-DNA-DIGIVOLVE** (NEW — to be filed by the orchestrator on this
//!   card's verdict): the engine has `BurstDigivolve` (single-card hand-counter
//!   via the BlastDigivolve keyword) and `dna_digivolve` (board-pair, normal
//!   timing), but no combined alt-path that uses 2 specific named DNA materials
//!   firing from hand at counter timing with cost 0. The
//!   `[Hand][Counter] <Blast DNA Digivolve ([WarGreymon] + [MetalGarurumon])>`
//!   entry path is therefore unmodelable; only the standard digivolve and the
//!   regular DNA path are present.
//!
//! All behavioral tests for Clause 3 and the Blast DNA alt-path are
//! `#[ignore]`'d with the appropriate gap tag. Structural tests for Clauses
//! 1–2, the Ace Overflow value, and the two implementable alt-paths run
//! green and lock the YAML shape.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

const YAML: &str = include_str!("../../../cards/bt17/BT17-078.yaml");

/// Compile the YAML (without going through the embedded pack lookup, which
/// requires the orchestrator to have re-built the embedded pack first).
fn compiled_bt17_078() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(YAML).expect("BT17-078.yaml parses");
    let registry = digimon_dsl::CardRegistry::from_specs("test", &[spec])
        .expect("BT17-078.yaml compiles");
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
         (the Blast DNA variant is BLOCKED — see G-BLAST-DNA-DIGIVOLVE)"
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
    assert_eq!(
        path.materials.len(),
        2,
        "DNA requires exactly 2 materials"
    );

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

/// BLOCKED structural assertion — when G-BLAST-DNA-DIGIVOLVE closes, the
/// Blast DNA alt-path becomes a third entry. Materials would be specifically
/// WarGreymon + MetalGarurumon (distinct from the regular DNA path's broader
/// Greymon/Garurumon name family).
#[test]
#[ignore = "pending: G-BLAST-DNA-DIGIVOLVE from qa/dsl-vocab-gaps.md \
            -- engine has BurstDigivolve (single-card hand-counter) and \
            dna_digivolve (board-pair, normal timing), but no combined \
            blast_dna_digivolve alt-path kind that takes 2 specific named \
            materials firing from hand at counter timing with cost 0."]
fn bt17_078_has_blast_dna_digivolve_alt_path_wargreymon_metalgarurumon() {
    let c = compiled_bt17_078();
    let blast_dna = c.alt_paths.iter().find(|p| {
        // When the gap closes, this will be a new CompiledAltPathKind variant
        // (e.g. `BlastDnaDigivolve`). For now we approximate the shape: a DNA
        // digivolve at cost 0 whose materials are name_is "WarGreymon" /
        // name_is "MetalGarurumon" — distinct from the regular DNA path
        // above which uses name_contains "Greymon" / "Garurumon".
        p.kind == CompiledAltPathKind::DnaDigivolve
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.materials.iter().any(|m| {
                pred_any(&m.filter, |q| q.name_is.as_deref() == Some("WarGreymon"))
            })
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

/// BLOCKED structural assertion — Clause 3 (DNA-gated bottom-deck + delete).
/// When G-BIND-SELECTED-PROPERTY-FOR-EACH closes, the YAML will gain a
/// triggered clause covering OnPlay + WhenDigivolving with a DNA gate. Check
/// the shape so the unblock is mechanical.
#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH from qa/dsl-vocab-gaps.md \
            -- DSL lacks bind_selected + for_each over a captured property; the \
            Clause 3 body is a commented stub in BT17-078.yaml until the gap \
            closes."]
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
    assert!(
        !t.once_per_turn,
        "Clause 3 has no [Once Per Turn] marker"
    );
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
// Clause 3's only condition is `dna_origin: true`, which is gap-blocked at the
// YAML level (the entire clause is currently a commented stub). The two
// condition-gating tests below are the planned positive/negative pair —
// `#[ignore]`'d with the gap tag until the YAML clause is uncommented.

#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH -- Clause 3 body absent from YAML"]
fn bt17_078_on_play_dna_gate_blocks_when_played_normally_from_hand() {
    // Negative: playing Omnimon from hand (no DNA digivolve in the trigger
    // hashtable) must NOT install any pending selection from Clause 3.
    let mut runner = omnimon_runner();
    let omni = runner.place_on_field(0, "BT17-078", None);
    runner.fire_on_play(0, omni.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "[On Play] Clause 3 must skip when not DNA-digivolving (no IsJogress)."
    );
}

#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH -- Clause 3 body absent from YAML"]
fn bt17_078_on_play_dna_gate_passes_when_dna_digivolving_with_opp_digimon() {
    // Positive: DNA-digivolving onto the field with at least one opp Digimon
    // must install the bottom-deck selection.
    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV6", "OppLv6", 6, 8000));
    let _opp = runner.place_on_field(1, "OPP-LV6", Some(0));
    let omni = runner.place_on_field(0, "BT17-078", None);
    // Synthesize a DNA-origin trigger by enqueueing WhenDigivolving with a
    // DNA hashtable. Until G-BIND-SELECTED-PROPERTY-FOR-EACH closes, this
    // path is gap-blocked.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(omni));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_some(),
        "DNA-digivolving onto the field with opp Digimon must install the \
         choose-1-opponent-Digimon selection."
    );
}

// ─── SECTION 3 — Behavioral outcome (per branch, integrated) ─────────────────

#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH from qa/dsl-vocab-gaps.md \
            -- Clause 3 body is commented stub; cannot return-by-level until \
            DSL gains bind_selected + for_each over a captured property."]
fn bt17_078_on_play_dna_returns_all_opponent_digimon_of_chosen_level_to_deck_bottom() {
    // Setup: opp has two Lv.5 Digimon and one Lv.6 Digimon; player chooses
    // one of the Lv.5s. Expect: BOTH Lv.5 Digimon move to the bottom of the
    // opp deck; the Lv.6 stays on the field.
    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-A-LV5", "OppA", 5, 5000));
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-B-LV5", "OppB", 5, 6000));
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-C-LV6", "OppC", 6, 9000));

    let _opp_a = runner.place_on_field(1, "OPP-A-LV5", Some(0));
    let _opp_b = runner.place_on_field(1, "OPP-B-LV5", Some(0));
    let _opp_c = runner.place_on_field(1, "OPP-C-LV6", Some(0));

    let opp_battle_before = runner.battle_area_size(1);
    let opp_deck_before = runner.deck_size(1);
    assert_eq!(opp_battle_before, 3);

    let omni = runner.place_on_field(0, "BT17-078", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(omni));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve after pick");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Both Lv.5 Digimon must leave the battle area (chosen + same-level peer)"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 2,
        "Both Lv.5 Digimon must land at the bottom of the opp deck"
    );
}

#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH -- Clause 3 body absent from YAML"]
fn bt17_078_on_play_dna_then_delete_installs_after_bottom_deck() {
    // Setup: opp has one Lv.5 (will be bounced) and one Lv.6 (survives). After
    // the bottom-deck step, the "Then, delete 1 of your opponent's Digimon"
    // step must install an OppField selection over the surviving Lv.6.
    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV5", "Opp5", 5, 5000));
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV6", "Opp6", 6, 9000));
    let _opp_5 = runner.place_on_field(1, "OPP-LV5", Some(0));
    let _opp_6 = runner.place_on_field(1, "OPP-LV6", Some(0));

    let omni = runner.place_on_field(0, "BT17-078", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(omni));
    runner.game.drain_effect_queue();
    // Resolve the bottom-deck pick (auto-pick Lv.5).
    runner.auto_resolve().expect("auto-resolve bottom-deck pick");

    let pending = runner
        .pending_selection()
        .expect("delete prompt must install after bottom-deck step");
    assert_eq!(
        pending.kind,
        SelectionKind::OppField,
        "Second step is select_opponent_permanent → OppField"
    );
    assert!(
        !pending.is_optional,
        "DCGO line 301 SelectPermanentEffect.SetUp passes canNoSelect: false → \
         delete-target select is mandatory once eligible targets exist"
    );
}

#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH -- Clause 3 body absent from YAML"]
fn bt17_078_when_digivolving_fires_same_body_as_on_play() {
    // DCGO uses identical SharedActivateCoroutine bodies for OnPlay and
    // WhenDigivolving (lines 142-313 mirror lines 316-487). Verify the
    // observable outcome is the same when WhenDigivolving fires directly.
    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV4", "Opp4", 4, 4000));
    let _opp = runner.place_on_field(1, "OPP-LV4", Some(0));

    let omni = runner.place_on_field(0, "BT17-078", None);
    let opp_battle_before = runner.battle_area_size(1);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(omni));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve");

    assert!(
        runner.battle_area_size(1) < opp_battle_before,
        "WhenDigivolving must fire the same body as OnPlay (DNA-gated)."
    );
}

// ─── SECTION 4 — Cost-firing event-log assertions ────────────────────────────
//
// Clause 3 has no cost (the bottom-deck and delete are body steps, not
// payments). Raid/Blocker are static keyword grants without cost firing.
// Nothing to assert here beyond observing that no cost-firing GameEvent
// (e.g., Trash, LoseSecurity) is emitted by the keyword grants themselves.
//
// Once Clause 3 is unblocked, an event-log assertion for the delete arm
// (GameEvent::Delete on the chosen surviving opp Digimon) becomes appropriate.

#[test]
#[ignore = "pending: G-BIND-SELECTED-PROPERTY-FOR-EACH -- Clause 3 body absent from YAML; \
            re-enable to assert GameEvent::Delete on the surviving opp Digimon."]
fn bt17_078_on_play_dna_delete_arm_emits_delete_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV5", "Opp5", 5, 5000));
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV6", "Opp6", 6, 9000));
    let _opp_5 = runner.place_on_field(1, "OPP-LV5", Some(0));
    let _opp_6 = runner.place_on_field(1, "OPP-LV6", Some(0));
    let omni = runner.place_on_field(0, "BT17-078", None);

    let cp = runner.event_checkpoint();
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(omni));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve");

    let events = runner.events_since(cp);
    // Note: GameEvent currently has no `Delete` variant. The closest
    // observable event for a delete is `Trash` (the deleted permanent's top
    // card lands in trash). Once GameEvent::Delete is wired, replace this
    // matcher.
    let has_trash = events
        .iter()
        .any(|e| matches!(e, GameEvent::Trash { .. }));
    assert!(
        has_trash,
        "delete arm must emit at least one Trash event for the surviving opp Digimon \
         (proxy for the not-yet-wired GameEvent::Delete)"
    );
}

// ─── SECTION 5 — OPT enforcement ─────────────────────────────────────────────
//
// No clause on BT17-078 is `[Once Per Turn]`. Skipped intentionally.
