//! BT23-032 Shakkoumon — Digimon, Lv.5, Yellow/Black, DP 8000, Cost 8.
//! Traits: Mutant, Hudie, CS, Angel (Rule-granted — see below).
//!
//! # Card text (card image — authoritative; see data/card_bundles/BT23-032.md)
//!
//! ```text
//! [When Digivolving] Until your opponent's turn ends, give 1 of their
//! Digimon "[Start of Your Main Phase] This Digimon attacks." Then, if DNA
//! digivolving, ＜De-Digivolve 1＞ 1 of their Digimon. (Trash the top card.
//! You can't trash past level 3 cards.)
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area
//! other than by your effects, you may play 1 level 4 or lower yellow, black
//! or [CS] trait Digimon card from its digivolution cards without paying the
//! cost.
//!
//! Inherited: identical [All Turns][Once Per Turn] would-leave clause.
//!
//! (Rule) Trait: Has [Angel] Type. — official Bandai DB grant, dropped by
//! the API-ingested cards.json `type_eng` field. Folded into `traits:`.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Yellow/BT23_032.cs
//!
//! # Patterns this test covers
//! - Standard digivolve Yellow Lv.4/cost4 + Black Lv.4/cost4 (card_overrides
//!   correction — cards.json evo_costs only carried the Yellow circle)
//! - Alt-digivolve Lv.4 w/[CS] trait / cost 3
//! - DNA digivolve Yellow Lv.4 + (Black or Blue) Lv.4 / cost 0
//! - [When Digivolving] select_opponent_permanent (mandatory) +
//!   grant_triggered_effect { start_of_your_main_phase, end_of_opponents_turn,
//!   body: [force_attack] } — verified substrate:
//!   tests/dsl/granted_effect_selection_body.rs (written specifically for
//!   this card) + EX10-034 Blastmon sibling clause.
//! - [On DNA Digivolve] <De-Digivolve 1> 1 opp Digimon, stop_at_level 3 —
//!   BT16-025 Paildramon `on_dna_digivolve` precedent (avoids the
//!   G-ENGINE-DNA-ORIGIN-PRED gap in process-`if` `dna_origin` conditions).
//! - [All Turns][OPT] non-cancelling `kind: replacement` on
//!   `when_would_leave_battle_area`, `none_of: [replacement_cause: own_effect]`
//!   — BT25-025 <Decode> / BT17-095 Clause B precedent (value-add, not a
//!   prevention).
//! - Inherited twin of the would-leave clause (`scope: inherited`).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT23-032";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Like `make_digimon` but with an explicit color list — needed for the
/// would-leave clause's `color_is: yellow`/`color_is: black` filter fixtures.
fn make_colored_digimon(
    id: &str,
    level: u8,
    dp: i32,
    cost: u16,
    colors: &[CardColor],
    traits: &[&str],
) -> CardData {
    let mut card = make_digimon(id, level, dp, cost, traits);
    card.colors = colors.to_vec();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-032 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("MAT-A", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("MAT-B", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", 4, 5000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI2", 4, 5000, 4, &["Beast"]))
        .add_card(make_colored_digimon(
            "YEL-LV4-SRC",
            4,
            3000,
            4,
            &[CardColor::Yellow],
            &["Angel"],
        ))
        .add_card(make_colored_digimon(
            "BLK-LV3-SRC",
            3,
            2000,
            3,
            &[CardColor::Black],
            &["Angel"],
        ))
        .add_card(make_digimon("NONMATCH-LV4-SRC", 4, 3000, 4, &["Beast"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

/// Push a fresh CardSource with `card_id` onto the owner's existing
/// permanent's stack (digivolution material).
fn push_on_stack(r: &mut DebugRunner, owner: u8, perm_idx: usize, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    let cs = CardSource::new(data_idx, owner, next);
    let turn = r.game.turn_count;
    r.game.players[owner as usize].battle_area[perm_idx].digivolve(cs, turn);
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt23_032_yaml_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Shakkoumon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    assert_eq!(card.cost, Some(8));
}

#[test]
fn bt23_032_has_angel_rule_trait() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert!(
        card.traits.iter().any(|t| t == "Angel"),
        "BT23-032 must carry the Rule-granted [Angel] Type trait (official Bandai DB; \
         dropped by API-ingested cards.json)"
    );
}

#[test]
fn bt23_032_has_standard_digivolve_yellow_lv4_cost4() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| f.level_eq == Some(4))
    });
    assert!(
        has,
        "BT23-032 must have a standard digivolve path from Lv.4 costing 4 (Yellow or Black)"
    );
}

#[test]
fn bt23_032_has_two_standard_digivolve_paths_yellow_and_black() {
    // card_overrides.json correction: cards.json's evo_costs only carried the
    // Yellow circle; the official Bandai DB / card image show both Yellow
    // Lv.4/cost4 AND Black Lv.4/cost4 as separate standard circles.
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let standard_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
        })
        .collect();
    assert_eq!(
        standard_paths.len(),
        2,
        "BT23-032 must have exactly 2 standard (cost-4) digivolve alt-paths (Yellow Lv.4 + Black Lv.4)"
    );
}

#[test]
fn bt23_032_has_cs_trait_alt_digivolve_cost3() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
    });
    assert!(
        has,
        "BT23-032 must have an alt-digivolve path (Lv.4 w/[CS] trait, cost 3)"
    );
}

#[test]
fn bt23_032_has_dna_digivolve_altpath_cost0() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let dna_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(
        dna_paths.len(),
        1,
        "BT23-032 must have exactly 1 DNA digivolve alt-path"
    );
    assert_eq!(
        dna_paths[0].cost,
        Some(CompiledCost::Literal(0)),
        "DNA digivolve cost is 0 (printed: Yellow Lv.4 + Black/Blue Lv.4)"
    );
    assert_eq!(
        dna_paths[0].materials.len(),
        2,
        "DNA digivolve requires exactly 2 materials"
    );
}

#[test]
fn bt23_032_has_when_digivolving_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    assert!(
        has,
        "BT23-032 must have a [When Digivolving] triggered clause"
    );
}

#[test]
fn bt23_032_has_on_dna_digivolve_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDnaDigivolve)
        )
    });
    assert!(
        has,
        "BT23-032 must have an [On DNA Digivolve] triggered clause for De-Digivolve 1"
    );
}

#[test]
fn bt23_032_has_face_up_and_inherited_would_leave_replacement_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let replacements: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                once_per_turn,
                optional,
                ..
            }) => Some((*scope, *once_per_turn, *optional)),
            _ => None,
        })
        .collect();
    assert_eq!(
        replacements.len(),
        2,
        "BT23-032 must have exactly 2 would-leave replacement clauses (face-up + inherited)"
    );
    assert!(
        replacements
            .iter()
            .any(|(scope, opt_turn, optional)| *scope == CompiledScope::FaceUp
                && *opt_turn
                && *optional),
        "the face-up would-leave clause must be Once-Per-Turn + optional"
    );
    assert!(
        replacements
            .iter()
            .any(|(scope, opt_turn, optional)| *scope == CompiledScope::Inherited
                && *opt_turn
                && *optional),
        "the inherited would-leave clause must be Once-Per-Turn + optional"
    );
}

// ─── Section 2 — [When Digivolving] grant force-attack to 1 opp Digimon ─────

#[test]
fn bt23_032_when_digivolving_installs_opp_permanent_select_and_grants_force_attack() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let shakko = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shakko));
    r.game.drain_effect_queue();

    let pending = r
        .pending_selection_view()
        .expect("mandatory opp-permanent select must install");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert!(
        !pending.is_optional,
        "select 1 opp Digimon is mandatory (DCGO canNoSelect: false)"
    );

    let action_id = pending.valid_action_ids[0];
    r.execute_action(0, action_id).expect("select opp target");
    r.auto_resolve().ok();

    assert_eq!(
        r.game
            .modifiers
            .granted_triggered_for_timing(opp, EffectTiming::StartOfYourMainPhase)
            .len(),
        1,
        "the selected opp Digimon must carry the granted Start-of-Main mandatory-attack body"
    );
}

#[test]
fn bt23_032_when_digivolving_no_select_prompt_when_no_opp_digimon() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let shakko = r.place_on_field(0, CARD_ID, Some(0));
    // No opponent Digimon placed.

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shakko));
    r.game.drain_effect_queue();
    r.auto_resolve().ok();

    assert!(
        r.pending_selection().is_none(),
        "with no opponent Digimon, [When Digivolving] must resolve cleanly with no prompt"
    );
}

#[test]
fn bt23_032_granted_force_attack_fires_at_opponent_start_of_main() {
    // End-to-end: grant installs, then P1's turn begins → StartOfYourMainPhase
    // fires for P1's battle area, parking a MANDATORY attack selection driven
    // by the granted body (tests/dsl/granted_effect_selection_body.rs substrate).
    let mut r = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"])
        .security(1, &["DECK-PAD", "DECK-PAD"])
        .memory(10)
        .start();
    let shakko = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shakko));
    r.game.drain_effect_queue();

    let pending = r.pending_selection_view().expect("opp-select prompt");
    let action_id = pending.valid_action_ids[0];
    r.execute_action(0, action_id).expect("select opp target");
    r.auto_resolve().ok();

    // P0 -> P1: StartOfYourMainPhase fires for P1's battle area.
    r.end_turn();

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("granted force_attack must park a mandatory attack-target selection");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert_eq!(
        pending.selecting_player, 1,
        "the carrier's controller (P1, the opponent of the grantor) chooses the attack target"
    );
    assert!(
        !pending.is_optional,
        "the granted attack is mandatory — no decline"
    );
    assert!(!pending.valid_action_ids.contains(&PASS));
}

// ─── Section 3 — [On DNA Digivolve] <De-Digivolve 1> ────────────────────────

#[test]
fn bt23_032_dna_digivolve_triggers_de_digivolve_prompt_on_opp_digimon() {
    let mat_a = "MAT-A";
    let mat_b = "MAT-B";

    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let material_a = r.place_on_field(0, mat_a, None);
    let material_b = r.place_on_field(0, mat_b, None);
    // 2-high stack so De-Digivolve 1 has a source to pop (a bare 1-card stack
    // is a documented no-op — de_digivolve_on_single_card_stack_is_noop).
    let opp = r.place_stack(1, &["BLK-LV3-SRC", "OPP-DIGI"]);

    let hand_card = r.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }

    // [When Digivolving] fires first (mandatory opp-select for the force-attack
    // grant); resolve it, then [On DNA Digivolve] must park the mandatory
    // De-Digivolve 1 select.
    let pending = r
        .pending_selection_view()
        .expect("[When Digivolving] opp-select must park first");
    let action_id = pending.valid_action_ids[0];
    r.execute_action(0, action_id).expect("resolve WD select");

    let pending = r
        .pending_selection_view()
        .expect("[On DNA Digivolve] must park a mandatory De-Digivolve 1 select");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert!(
        !pending.is_optional,
        "De-Digivolve 1 select is mandatory (DCGO canNoSelect: false)"
    );

    let sources_before = r.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    let action_id = pending.valid_action_ids[0];
    r.execute_action(0, action_id).expect("select De-Digivolve target");
    r.auto_resolve().ok();

    let sources_after = r
        .game
        .players[1]
        .battle_area
        .get(opp.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);
    assert_eq!(
        sources_after,
        sources_before - 1,
        "<De-Digivolve 1> must trash exactly the top card of the targeted opp Digimon"
    );
}

#[test]
fn bt23_032_non_dna_digivolve_does_not_trigger_de_digivolve() {
    let mut r = base().memory(10).start();
    let shakko = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    let sources_before = r.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    // Fire WhenDigivolving without DNA context — on_dna_digivolve must NOT fire.
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shakko));
    r.game.drain_effect_queue();
    // Resolve the [When Digivolving] mandatory opp-select (force-attack grant).
    if let Some(pending) = r.pending_selection_view() {
        let action_id = pending.valid_action_ids[0];
        r.execute_action(0, action_id).expect("resolve WD select");
    }
    r.auto_resolve().ok();

    assert!(
        r.pending_selection().is_none(),
        "non-DNA digivolve must not leave a dangling De-Digivolve prompt"
    );
    let sources_after = r.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after, sources_before,
        "non-DNA digivolve must NOT trigger <De-Digivolve 1> on the opponent"
    );
}

#[test]
fn bt23_032_de_digivolve_stops_at_level_3() {
    // Opponent stack: [Lv3, Lv4]. De-Digivolve 1 pops the Lv4, leaving Lv3 —
    // matches the printed parenthetical "You can't trash past level 3 cards."
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let material_a = r.place_on_field(0, "MAT-A", None);
    let material_b = r.place_on_field(0, "MAT-B", None);
    let opp = r.place_stack(1, &["BLK-LV3-SRC", "YEL-LV4-SRC"]);

    let hand_card = r.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }

    // Resolve [When Digivolving] opp-select (force-attack grant target).
    if let Some(pending) = r.pending_selection_view() {
        let action_id = pending.valid_action_ids[0];
        r.execute_action(0, action_id).expect("resolve WD select");
    }

    // Resolve [On DNA Digivolve] De-Digivolve select — target the 2-high stack.
    let pending = r
        .pending_selection_view()
        .expect("[On DNA Digivolve] De-Digivolve select must park");
    let action_id = pending.valid_action_ids[0];
    r.execute_action(0, action_id).expect("select De-Digivolve target");
    r.auto_resolve().ok();

    let perm = &r.game.players[1].battle_area[opp.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "De-Digivolve 1 pops exactly the Lv4 top card, leaving the Lv3 base"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "BLK-LV3-SRC",
        "the Lv3 base card must remain on top after De-Digivolve 1"
    );
}

// ─── Section 4 — [All Turns][OPT] would-leave-other-than-by-your-effects ────

#[test]
fn bt23_032_would_leave_by_opponent_effect_installs_optional_source_play_prompt() {
    // Build Shakkoumon on TOP of a YEL-LV4-SRC digivolution source (place_stack:
    // last element is the top card), then delete it via an OPPONENT-attributed
    // effect (own_effect exclusion must NOT apply — the clause fires).
    let mut r = base().memory(10).start();
    let shakko = r.place_stack(0, &["YEL-LV4-SRC", CARD_ID]);

    // Simulate an opponent-controlled effect currently resolving.
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(shakko);
    r.game.set_effect_source_player_for_test(None);

    let pending = r
        .pending_selection_view()
        .expect("[All Turns][OPT] would-leave-by-opponent-effect must offer the optional source play");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(
        pending.is_optional,
        "the printed clause is 'you may play' — the select must be optional"
    );
}

#[test]
fn bt23_032_would_leave_by_own_effect_does_not_fire() {
    // Same setup, but the deleting effect is attributed to the CARRIER's OWN
    // controller (own_effect) — the clause's `none_of: [replacement_cause:
    // own_effect]` gate must suppress the prompt entirely.
    let mut r = base().memory(10).start();
    let shakko = r.place_stack(0, &["YEL-LV4-SRC", CARD_ID]);

    // Simulate the CARRIER's OWNER's own effect currently resolving.
    r.game.set_effect_source_player_for_test(Some(0));
    r.game.delete_permanent_with_effects(shakko);
    r.game.set_effect_source_player_for_test(None);

    assert!(
        r.pending_selection().is_none(),
        "when the leave is caused by the carrier owner's OWN effect, the would-leave \
         clause must NOT fire ('other than by your effects')"
    );
}

#[test]
fn bt23_032_would_leave_prompt_offers_only_matching_sources() {
    // Two sources under Shakkoumon: one matching (Angel-trait yellow Lv.4),
    // one non-matching (Beast-trait, non-yellow/black/CS, Lv.4). Only the
    // matching source should be selectable. Shakkoumon (CARD_ID) is last =
    // the top card; the two sources sit underneath.
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;

    let mut r = base().memory(10).start();
    let shakko = r.place_stack(0, &["NONMATCH-LV4-SRC", "YEL-LV4-SRC", CARD_ID]);

    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(shakko);

    // Drive past the outer would-leave accept/decline gate first.
    let outer = r
        .pending_selection_view()
        .expect("would-leave prompt must surface with a matching source present");
    assert_eq!(outer.kind, SelectionKind::Replacement);
    r.execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the would-leave activation");
    r.game.set_effect_source_player_for_test(None);

    // The nested material select must now show exactly the matching source.
    let pending = r
        .pending_selection_view()
        .expect("select_material prompt must surface after accepting");
    assert_eq!(pending.kind, SelectionKind::Material);
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the single matching [Angel]/yellow Lv.4 source should be a legal pick"
    );
}

#[test]
fn bt23_032_would_leave_no_prompt_when_no_matching_sources() {
    // Only a non-matching source underneath — the optional select has no
    // eligible candidates and must silently skip (no dangling prompt).
    let mut r = base().memory(10).start();
    let shakko = r.place_stack(0, &["NONMATCH-LV4-SRC", CARD_ID]);

    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(shakko);
    r.game.set_effect_source_player_for_test(None);

    assert!(
        r.pending_selection().is_none(),
        "with no yellow/black/[CS] Lv.4- source available, the would-leave clause \
         must resolve without a dangling prompt"
    );
}

#[test]
fn bt23_032_declining_would_leave_play_does_not_play_anything() {
    let mut r = base().memory(10).start();
    let shakko = r.place_stack(0, &["YEL-LV4-SRC", CARD_ID]);

    let battle_area_before = r.game.player(0).battle_area.len();

    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(shakko);

    let pending = r.pending_selection_view().expect("optional prompt surfaces");
    // The outer would-leave accept/decline gate (SelectionKind::Replacement)
    // lists only REPLACEMENT_ACCEPT in valid_action_ids, but PASS is still the
    // legal decline action for any `is_optional: true` selection (see
    // return_own_sources_leave_replacement.rs precedent).
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.is_optional, "the printed 'you may' clause is optional");
    r.execute_action(0, PASS).expect("decline the optional play");
    r.game.set_effect_source_player_for_test(None);
    r.auto_resolve().ok();

    // Declining must not add a new permanent from the source.
    assert!(
        r.game
            .player(0)
            .battle_area
            .len()
            <= battle_area_before,
        "declining the optional play must not add a new permanent to the battle area"
    );
}

#[test]
fn bt23_032_would_leave_opt_is_tracked_per_permanent_not_globally() {
    // [Once Per Turn] activation counters (`Permanent::effect_activations`)
    // are tracked PER PERMANENT INSTANCE, not per-player/globally — each
    // physical Shakkoumon card has its own independent OPT counter. A "would
    // leave" trigger cannot fire twice on the SAME permanent (it only leaves
    // once), so the meaningful OPT-composition check is: consuming shakko_a's
    // OPT (accept + actually play a material, since decline-only is refunded
    // per G-OPT-REFUND-ON-DECLINE) must NOT block a DIFFERENT permanent
    // (shakko_b)'s independent would-leave activation in the same turn.
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;

    let mut r = base().memory(10).start();
    let shakko_a = r.place_stack(0, &["YEL-LV4-SRC", CARD_ID]);
    let shakko_b = r.place_stack(0, &["BLK-LV3-SRC", CARD_ID]);

    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(shakko_a);

    let pending = r.pending_selection_view().expect("first OPT fire");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    r.execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the first would-leave activation (consumes shakko_a's OPT)");
    // Actually pick the matching material so the OPT use is NOT refunded.
    let material_pending = r
        .pending_selection_view()
        .expect("select_material prompt must surface after accepting");
    assert_eq!(material_pending.kind, SelectionKind::Material);
    let pick = material_pending.valid_action_ids[0];
    r.execute_action(0, pick).expect("play the matching source");
    r.auto_resolve().ok();

    // shakko_b index may have shifted after shakko_a's removal; re-resolve by
    // scanning battle area for the BT23-032 permanent still present.
    let shakko_b_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID);
    let idx = shakko_b_idx.expect("shakko_b must still be on field");
    let handle = PermanentHandle {
        player: 0,
        index: idx as u8,
    };
    r.game.delete_permanent_with_effects(handle);
    r.game.set_effect_source_player_for_test(None);

    let pending_b = r
        .pending_selection_view()
        .expect("shakko_b's independent (per-permanent) OPT must still fire this turn");
    assert_eq!(pending_b.kind, SelectionKind::Replacement);
}

#[test]
fn bt23_032_would_leave_opt_lockout_same_permanent_twice_in_one_turn() {
    // Directly verifies the OPT counter itself: invoke the replacement stage
    // TWICE against the SAME still-on-field permanent within one turn without
    // an intervening physical removal (e.g. a would-leave observation whose
    // outcome was Cancelled/Redirected the first time, or a raw synthetic
    // double-fire for counter coverage). The second `try_replace` call for the
    // SAME permanent's face-up clause must be blocked once its OPT is durably
    // consumed (accept + actually play a material — decline-only refunds the
    // OPT per G-OPT-REFUND-ON-DECLINE, so this test must complete a real play).
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    use digimon_engine::enums::{EffectTiming, Zone};
    use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};

    let mut r = base().memory(10).start();
    let shakko = r.place_stack(0, &["YEL-LV4-SRC", CARD_ID]);

    r.game.set_effect_source_player_for_test(Some(1));
    let subject = ReplacementSubject::Permanent(shakko);
    let cause = ReplacementCause::OpponentEffect;

    // First fire: accept + play the material (durable OPT consumption).
    let _ = r.game.try_replace(
        EffectTiming::WhenWouldLeaveBattleArea,
        subject,
        cause,
        Some(Zone::Trash),
    );
    let pending = r.pending_selection_view().expect("first would-leave fire");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    r.execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the first would-leave activation");
    let material_pending = r
        .pending_selection_view()
        .expect("select_material prompt must surface after accepting");
    let pick = material_pending.valid_action_ids[0];
    r.execute_action(0, pick).expect("play the matching source");
    r.auto_resolve().ok();
    assert!(
        r.pending_selection().is_none(),
        "first activation must resolve cleanly before the second fire"
    );

    // Second fire: same permanent, same turn — the once-fired guard
    // (`Game::replacement_fired`) would also block an immediate re-dispatch
    // of the identical (timing, subject) pair within one call chain, but here
    // we drive a FRESH top-level `try_replace` call (a new call chain, fresh
    // `replacement_fired` set) so the assertion isolates the OPT counter
    // specifically, not the same-chain dedup guard.
    let _ = r.game.try_replace(
        EffectTiming::WhenWouldLeaveBattleArea,
        subject,
        cause,
        Some(Zone::Trash),
    );
    r.game.set_effect_source_player_for_test(None);

    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn] must block a second would-leave activation of the SAME \
         permanent's face-up clause within the same turn"
    );
}
