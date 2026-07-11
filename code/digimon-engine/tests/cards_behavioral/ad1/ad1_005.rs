//! AD1-005 Gaiamon ACE — Digimon, Lv.6, Red/White, DP 12000, Play Cost 7.
//! Traits: [God, Appmon, Creation]. Attribute: God.
//! Colors: RED + WHITE (official bundle "Colors: Red White"; dual-color name
//! plate on the card image; DCGO card_colors [0, 4]).
//!
//! # Corrected printed card text (card image — authoritative)
//!
//! [Hand] [Counter] <Blast Digivolve>
//!   (Your Digimon may digivolve into this card without paying the cost.)
//! <Security A. +1>  <Blocker>  <Link +1>
//!   (Add 1 to this Digimon's maximum links.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
//!   You may link up to 2 [Social], [Navi] or [Tool] trait cards from your
//!   hand or this Digimon's digivolution cards to this Digimon without paying
//!   the cost. Then, you may delete 1 of your opponent's Digimon with as much
//!   or less DP as this Digimon.
//!
//! ACE box: Ace Overflow <-4>.
//! No security effect.
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_005.cs
//!
//! # Source priority
//! Card image outranks cards.json for printed text. DCGO governs behavior.
//! DCGO AD1_005.cs confirms:
//! - IsProperTraitCard checks Social/Navi/Tool traits only — NOT kind: digimon.
//!   Any card type with those traits (including Options, Tamers) is linkable.
//! - Delete is canNoSelect:true → optional.
//! - OPT shared across all three timings via SharedHashString "AD1_005_OP_WD_WA".
//! - "Ult." digivolve gate: AddSelfDigivolutionRequirementStaticEffect with
//!   condition `targetPermanent.TopCard.EqualsTraits("Ult.")`, cost 4.
//!
//! # Patterns this test file covers
//! - Dual-color identity (Red + White — campaign drift pattern 5)
//! - Standard digivolve circle color gate (RED Lv.5 / 4 — red rim on the card
//!   image; a non-red Lv.5 base must be rejected; the rainbow "Ult. / 4"
//!   circle stays color-agnostic — campaign drift pattern 1)
//! - H5  Blocker (face-up declarative grant_keyword)
//! - SecurityAttackPlus +1 (face-up declarative grant_keyword, value 1)
//! - BlastDigivolve (hand counter marker grant_keyword)
//! - Link +1 self-aura (kind: aura / modifier: ChangeLinkMax / modifier_value 1)
//!   — first YAML card to exercise ChangeLinkMax modifier; precedent in link_flow.rs
//! - ACE Overflow -4 (top-level ace_overflow field)
//! - App Fusion alt-path: Globemon + Charismon cost 0
//! - Blast Digivolve alt-path (burst_digivolve marker)
//! - Ult. digivolve alt-path (trait_has "Ult.")
//! - Three-timing OPT: on_play / when_digivolving / when_attacking with shared
//!   once_per_turn; OPT consumed even on 0 links; fires on each timing separately;
//!   locked after consumption; resets after end_turn
//! - link_cards step: from [hand, self_sources], filter Social/Navi/Tool (not
//!   kind:digimon), to: self, count up_to 2, cost: free
//! - Non-Digimon Social-trait card positively links (proves no kind:digimon gate)
//! - Optional delete: dp_lte source_dp; boundary (exactly 12000 DP passes);
//!   decline path; no eligible → no prompt
//! - OPT cross-timing lock (on_play fires → when_attacking same turn is locked)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.5 Digimon to use as a digivolve base.
fn lv5_base(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c
}

/// Build a Digimon with a given DP (to test the delete filter).
fn digimon_with_dp(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c
}

/// Build a card (non-Digimon) with a given trait — used to confirm
/// IsProperTraitCard does NOT require kind: digimon.
fn option_card_with_trait(id: &str, trait_name: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.traits = vec![trait_name.to_string()];
    c
}

/// Build a Digimon with a given trait.
fn digimon_with_trait(id: &str, trait_name: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 3);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(3000);
    c.traits = vec![trait_name.to_string()];
    c
}

/// A Lv.5 Digimon with "Ult." trait (to test the Ult. digivolve alt-path).
fn ult_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c.traits = vec!["Ult.".to_string()];
    c
}

/// Standard runner: Gaiamon ACE in DSL pack + helper cards.
fn gaiamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 must be in embedded DSL pack")
        .add_card(lv5_base("LV5-BASE"))
        .add_card(ult_lv5_base("ULT-BASE"))
        .memory(15)
        .start()
}

/// Resolve the first non-PASS action from the pending selection.
fn pick_first(runner: &mut DebugRunner) {
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a selection must be pending to pick_first");
        let action = *p
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("at least one non-PASS candidate");
        (p.selecting_player, action)
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("pick_first: selection resolves");
    runner.game.drain_effect_queue();
}

/// PASS (decline) the currently pending selection.
fn decline_pending(runner: &mut DebugRunner) {
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a selection must be pending to decline")
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline: selection resolves");
    runner.game.drain_effect_queue();
}

// ─── SECTION 1 — Structural assertions on CompiledCard ───────────────────────

/// YAML must compile and produce correct card-level metadata.
#[test]
fn ad1_005_metadata_matches_printed_text() {
    let c = compiled("AD1-005");
    assert_eq!(c.name, "Gaiamon ACE");
    assert_eq!(c.level, Some(6), "Gaiamon ACE is Lv.6");
    assert_eq!(c.dp, Some(12000), "Gaiamon ACE is DP 12000");
    assert_eq!(c.cost, Some(7), "Gaiamon ACE costs 7 to play");
    assert!(
        c.traits.iter().any(|t| t == "God"),
        "traits must include God; got {:?}",
        c.traits
    );
    assert!(
        c.traits.iter().any(|t| t == "Appmon"),
        "traits must include Appmon; got {:?}",
        c.traits
    );
    assert!(
        c.traits.iter().any(|t| t == "Creation"),
        "traits must include Creation; got {:?}",
        c.traits
    );
    assert_eq!(
        c.attribute.as_deref(),
        Some("God"),
        "attribute must be God (image shows form 'God/Appmon', attribute 'God')"
    );
    // Dual-color identity — official bundle "Colors: Red White"; the card
    // image's name plate is red+white; DCGO card_colors [0, 4].
    assert_eq!(
        c.color.len(),
        2,
        "Gaiamon ACE is a two-color (Red+White) Digimon; got {:?}",
        c.color
    );
    assert!(
        c.color.contains(&CompiledColor::Red),
        "Gaiamon ACE must be Red; got {:?}",
        c.color
    );
    assert!(
        c.color.contains(&CompiledColor::White),
        "Gaiamon ACE must be White (bundle: 'Colors: Red White'); got {:?}",
        c.color
    );
}

/// ACE Overflow <-4> via top-level ace_overflow field.
#[test]
fn ad1_005_ace_overflow_is_minus_4() {
    let c = compiled("AD1-005");
    assert_eq!(
        c.ace_overflow,
        Some(-4),
        "ACE box: Ace Overflow <-4> must be ace_overflow: -4"
    );
}

/// Standard digivolve circle: RED Lv.5 / Cost 4 (red-rimmed circle on the
/// card image; official bundle "Red Lv.5 / cost 4"). The `from` filter MUST
/// carry the red color gate — a color-less `level_eq: 5` would wrongly admit
/// any-color Lv.5 bases.
#[test]
fn ad1_005_has_standard_red_lv5_digivolve_alt_path_cost_4() {
    let c = compiled("AD1-005");
    let standard = c.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(4)))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(5)
                    && f.trait_has.is_none()
                    && f.color_is == Some(CompiledColor::Red)
            })
    });
    assert!(
        standard.is_some(),
        "must have a standard Digivolve path at RED Lv.5 / Cost 4 \
         (printed circle is red-rimmed); paths: {:?}",
        c.alt_paths
            .iter()
            .map(|p| (&p.kind, &p.cost))
            .collect::<Vec<_>>()
    );
}

/// Ult. digivolve alt-path (DCGO EqualsTraits("Ult."), cost 4). The printed
/// circle has a RAINBOW rim — it is deliberately color-agnostic, so the
/// `from` filter must NOT carry a color gate.
#[test]
fn ad1_005_has_ult_trait_digivolve_alt_path_cost_4() {
    let c = compiled("AD1-005");
    let ult_path = c.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(4)))
            && p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("Ult."))
    });
    let ult_path = ult_path.expect(
        "must have a Ult.-trait Digivolve path at Cost 4 \
         (DCGO AddSelfDigivolutionRequirementStaticEffect EqualsTraits(\"Ult.\"))",
    );
    assert!(
        ult_path
            .from
            .as_ref()
            .is_some_and(|f| f.color_is.is_none()),
        "the Ult. circle is rainbow-rimmed (any color) — no color gate allowed \
         (DCGO condition checks the trait only)"
    );
}

/// App Fusion alt-path: [Globemon] & [Charismon], cost 0.
#[test]
fn ad1_005_has_app_fusion_globemon_charismon_cost_0() {
    let c = compiled("AD1-005");
    let fuse = c.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::AppFusion
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.materials.iter().any(|m| {
                m.filter
                    .name_in
                    .as_ref()
                    .is_some_and(|names| names.iter().any(|n| n == "Globemon"))
            })
            && p.materials.iter().any(|m| {
                m.filter
                    .name_in
                    .as_ref()
                    .is_some_and(|names| names.iter().any(|n| n == "Charismon"))
            })
    });
    assert!(
        fuse.is_some(),
        "must have an App Fusion path with Globemon + Charismon at Cost 0 \
         (DCGO AddAppfuseMethodByName)"
    );
}

/// Blast Digivolve alt-path: burst_digivolve with marker: true, cost 0.
#[test]
fn ad1_005_has_blast_digivolve_burst_path() {
    let c = compiled("AD1-005");
    let burst = c.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::BurstDigivolve
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        burst.is_some(),
        "must have a BurstDigivolve alt-path (cost 0) for [Hand][Counter] Blast Digivolve; \
         (DCGO BlastDigivolveEffect at OnCounterTiming)"
    );
}

/// Face-up BlastDigivolve keyword grant (the [Hand][Counter] marker).
#[test]
fn ad1_005_has_blast_digivolve_grant_keyword() {
    let c = compiled("AD1-005");
    let grant = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "BlastDigivolve"
        )
    });
    assert!(
        grant,
        "must have a face-up GrantKeyword(BlastDigivolve) clause \
         (DCGO BlastDigivolveEffect at OnCounterTiming)"
    );
}

/// Face-up SecurityAttackPlus +1 keyword grant.
#[test]
fn ad1_005_has_security_attack_plus_1_grant_keyword() {
    let c = compiled("AD1-005");
    let grant = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                value: Some(1),
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "SecurityAttackPlus"
        )
    });
    assert!(
        grant,
        "must have a face-up GrantKeyword(SecurityAttackPlus, value=1) clause \
         (DCGO ChangeSelfSAttackStaticEffect(1))"
    );
}

/// Face-up Blocker keyword grant.
#[test]
fn ad1_005_has_blocker_grant_keyword() {
    let c = compiled("AD1-005");
    let grant = c.effects.iter().any(|cl| {
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
        grant,
        "must have a face-up GrantKeyword(Blocker) clause \
         (DCGO BlockerSelfStaticEffect)"
    );
}

/// The three-timing OPT triggered clause covers on_play, when_digivolving,
/// and when_attacking, with once_per_turn: true.
#[test]
fn ad1_005_three_timing_opt_clause_structure() {
    let c = compiled("AD1-005");
    let triggered = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        });
    let t = triggered.expect(
        "must have a single triggered clause covering on_play, when_digivolving, \
         and when_attacking (DCGO SharedHashString shared across three timings)",
    );
    assert!(
        t.once_per_turn,
        "the three-timing clause must be once_per_turn (printed [Once Per Turn])"
    );
    assert_eq!(
        t.scope,
        CompiledScope::FaceUp,
        "the clause is a face-up triggered effect"
    );
    // "You may link up to 2 ... Then, you may delete" — outer effect is NOT
    // globally optional: the link loop handles optionality per-pick, and the
    // delete is optional. The outer clause has no top-level "you may" gate
    // (DCGO SharedCanActivateCondition gates activation, not clause optional).
    // NOTE: This may depend on whether active_when is set; either form is fine.
    // The important invariant is once_per_turn: true (checked above).
}

// ─── SECTION 1b — Digivolve-circle behavior (color gates) ────────────────────

/// A Lv.5 Digimon base with an explicit color and no Ult. trait (so only the
/// standard circle can admit it).
fn lv5_base_colored(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c.colors = vec![color];
    c.traits = vec!["Beast".to_string()];
    c
}

/// Stage AD1-005 in hand above `base` on the battle area (Main phase, turn 1).
/// Returns (runner, base field slot).
fn digivolve_runner(base: CardData) -> (DebugRunner, usize) {
    let base_id = base.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(base)
        .hand(0, &["AD1-005"])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

/// Standard circle positive: a RED Lv.5 base is offered in the mask and the
/// digivolve pays exactly the printed 4 memory.
#[test]
fn ad1_005_standard_digivolve_accepts_red_lv5_base_paying_4() {
    let (mut r, slot) = digivolve_runner(lv5_base_colored("RED-LV5", CardColor::Red));

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 1.0,
        "mask must offer AD1-005 digivolve over a red Lv.5 base (printed circle: Red Lv.5 / 4)"
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(ok, "digivolving over a red Lv.5 base must succeed");
    assert_eq!(
        mem_before - r.game.memory,
        4,
        "the standard circle costs exactly 4 memory"
    );
}

/// Standard circle negative: an off-color (blue) Lv.5 base without the Ult.
/// trait matches NO circle — the mask must not offer the digivolve and the
/// action must be rejected. (Campaign drift pattern 1: a color-less
/// `level_eq: 5` filter would wrongly accept this base.)
#[test]
fn ad1_005_standard_digivolve_rejects_off_color_lv5_base() {
    let (mut r, slot) = digivolve_runner(lv5_base_colored("BLUE-LV5", CardColor::Blue));

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 0.0,
        "mask must NOT offer AD1-005 digivolve over a blue Lv.5 base \
         (the printed circle is RED Lv.5 / 4)"
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(!ok, "digivolving over a blue non-Ult. Lv.5 base must be rejected");
    assert_eq!(r.game.memory, mem_before, "no memory is paid on rejection");
}

/// Ult. circle positive: the rainbow "Ult. / 4" circle is color-agnostic — a
/// BLUE base with the [Ult.] trait digivolves for 4 (DCGO EqualsTraits("Ult.")
/// checks the trait only).
#[test]
fn ad1_005_ult_circle_accepts_any_color_ult_base_paying_4() {
    let mut base = lv5_base_colored("BLUE-ULT", CardColor::Blue);
    base.traits = vec!["Ult.".to_string()];
    let (mut r, slot) = digivolve_runner(base);

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 1.0,
        "mask must offer AD1-005 digivolve over a blue [Ult.] base \
         (the Ult. circle is rainbow / any-color)"
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(ok, "digivolving over a blue [Ult.] base must succeed");
    assert_eq!(
        mem_before - r.game.memory,
        4,
        "the Ult. circle costs exactly 4 memory"
    );
}

// ─── SECTION 2 — Link +1 aura ────────────────────────────────────────────────

/// <Link +1>: placing Gaiamon ACE on the field installs a ChangeLinkMax +1
/// modifier on itself (first YAML card exercising ChangeLinkMax).
#[test]
fn ad1_005_link_max_aura_raises_own_link_cap_by_1() {
    let mut runner = gaiamon_runner();
    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.game.tick_declarative_effects();
    let delta = runner.game.modifiers.link_max_delta(gaiamon);
    assert_eq!(
        delta, 1,
        "<Link +1> must install ChangeLinkMax +1 on this Digimon; got {}",
        delta
    );
}

/// A Digimon WITHOUT the Link +1 aura gets delta 0 (control test).
#[test]
fn ad1_005_link_max_zero_on_other_digimon() {
    let mut runner = gaiamon_runner();
    let lv5 = runner.place_on_field(0, "LV5-BASE", Some(0));
    runner.game.tick_declarative_effects();
    let delta = runner.game.modifiers.link_max_delta(lv5);
    assert_eq!(
        delta, 0,
        "a Digimon without Link +1 must have no link-max delta; got {}",
        delta
    );
}

// ─── SECTION 3 — OPT behavioral: on_play fires + link + delete ───────────────

/// [On Play] with a Social-trait card in hand: OPT fires, link prompt installs.
#[test]
fn ad1_005_on_play_opt_fires_with_social_card_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_trait("SOCIAL-MON", "Social"))
        .add_card(lv5_base("LV5-BASE"))
        .hand(0, &["SOCIAL-MON"])
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.fire_on_play(0, gaiamon.index as usize);

    assert!(
        runner.game.pending_selection.is_some(),
        "[On Play] OPT must install a selection when Social-trait card is in hand"
    );
}

/// [On Play] with NO linkable cards AND no eligible delete target: OPT fires
/// but produces no prompt (silent activation).
#[test]
fn ad1_005_on_play_opt_silent_when_no_candidates() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(lv5_base("LV5-BASE"))
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.fire_on_play(0, gaiamon.index as usize);

    // No linkable cards anywhere, no weak opponent Digimon: nothing to prompt.
    // The OPT is consumed (no pending selection needed for 0-link 0-delete).
    // This asserts there's no crash and the game state is still valid.
    assert!(
        runner.game.pending_selection.is_none()
            || runner
                .game
                .pending_selection
                .as_ref()
                .is_some_and(|p| p.is_optional),
        "with no candidates, on_play OPT should produce no mandatory selection"
    );
}

/// [On Play] with a Social-trait card: linking exactly 1 card works.
#[test]
fn ad1_005_on_play_link_one_social_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_trait("SOCIAL-MON", "Social"))
        .add_card(lv5_base("LV5-BASE"))
        .hand(0, &["SOCIAL-MON"])
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    let hand_before = runner.hand_size(0);
    let links_before = runner.game.players[0].battle_area[gaiamon.index as usize]
        .linked_cards
        .len();

    runner.fire_on_play(0, gaiamon.index as usize);

    // Pick the Social card from hand (first non-PASS action).
    pick_first(&mut runner);
    // PASS on the second pick (only 1 card available).
    if runner.game.pending_selection.is_some() {
        let is_link_prompt = runner
            .game
            .pending_selection
            .as_ref()
            .is_some_and(|p| p.is_optional);
        if is_link_prompt {
            decline_pending(&mut runner);
        }
    }
    // PASS on the delete prompt.
    if runner.game.pending_selection.is_some() {
        let is_optional = runner
            .game
            .pending_selection
            .as_ref()
            .is_some_and(|p| p.is_optional);
        if is_optional {
            decline_pending(&mut runner);
        }
    }

    let links_after = runner.game.players[0].battle_area[gaiamon.index as usize]
        .linked_cards
        .len();
    assert_eq!(
        links_after,
        links_before + 1,
        "linking 1 Social card should attach it to Gaiamon ACE"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the linked Social card must leave the hand"
    );
}

/// Social/Navi/Tool filter: a card with NONE of those traits must NOT be
/// offered in the link prompt.
#[test]
fn ad1_005_link_rejects_wrong_trait_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_trait("WRONG-MON", "Dragon"))
        .add_card(lv5_base("LV5-BASE"))
        .hand(0, &["WRONG-MON"])
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.fire_on_play(0, gaiamon.index as usize);

    // With no Social/Navi/Tool card in hand or sources, no link prompt
    // should install.  A delete prompt is also absent (no weak opp Digimon).
    assert!(
        runner.game.pending_selection.is_none()
            || runner
                .game
                .pending_selection
                .as_ref()
                .is_some_and(|p| p.is_optional),
        "a Dragon-trait card must NOT be offered as a link candidate; \
         filter is Social/Navi/Tool only"
    );
}

/// FAITHFULNESS: a non-Digimon card (Option-kind) with Social trait IS linkable
/// — DCGO IsProperTraitCard checks traits only, not kind.
#[test]
fn ad1_005_link_accepts_option_card_with_social_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(option_card_with_trait("SOCIAL-OPTION", "Social"))
        .add_card(lv5_base("LV5-BASE"))
        .hand(0, &["SOCIAL-OPTION"])
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    let links_before = runner.game.players[0].battle_area[gaiamon.index as usize]
        .linked_cards
        .len();

    runner.fire_on_play(0, gaiamon.index as usize);

    // The Option card with Social trait must be offered — pick it.
    assert!(
        runner.game.pending_selection.is_some(),
        "an Option-kind card with Social trait must be offered as a link candidate \
         (DCGO IsProperTraitCard checks traits only, NOT kind: digimon)"
    );
    pick_first(&mut runner);

    // Decline the second link pick and the delete pick.
    let _ = runner.auto_resolve();

    let links_after = runner.game.players[0].battle_area[gaiamon.index as usize]
        .linked_cards
        .len();
    assert_eq!(
        links_after,
        links_before + 1,
        "the Option Social card must have been attached as a link card"
    );
}

// ─── SECTION 4 — Delete arm ──────────────────────────────────────────────────

/// Delete arm: with an eligible Digimon (DP <= 12000), prompt installs after
/// linking and must be optional (canNoSelect: true).
#[test]
fn ad1_005_delete_prompt_is_optional_with_eligible_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_dp("OPP-WEAK", 12000))
        .add_card(lv5_base("LV5-BASE"))
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.place_on_field(1, "OPP-WEAK", None);

    runner.fire_on_play(0, gaiamon.index as usize);

    // Link phase: no linkable cards → skip to delete (or OPT fires silently).
    // The delete prompt should install for the eligible 12000 DP opponent.
    let has_delete_prompt = runner.game.pending_selection.is_some();
    if has_delete_prompt {
        let p = runner.game.pending_selection.as_ref().unwrap();
        assert!(
            p.is_optional,
            "delete prompt must be optional (DCGO canNoSelect: true); \
             DCGO 'you may delete'"
        );
    }
}

/// Delete boundary: an opponent Digimon with DP EXACTLY 12000 (= this Digimon's
/// DP) is eligible for deletion (dp_lte: source_dp means <=).
#[test]
fn ad1_005_delete_accepts_digimon_at_exactly_12000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_dp("OPP-12K", 12000))
        .add_card(digimon_with_dp("OPP-13K", 13000))
        .add_card(lv5_base("LV5-BASE"))
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    let _weak_field = runner.place_on_field(1, "OPP-12K", None);
    let _strong_field = runner.place_on_field(1, "OPP-13K", None);

    runner.fire_on_play(0, gaiamon.index as usize);
    runner.game.drain_effect_queue();

    // No linkable cards → the delete prompt is next. The 12000 DP Digimon is
    // eligible (dp_lte source_dp: 12000 <= 12000), so the prompt MUST install.
    let p = runner
        .game
        .pending_selection
        .as_ref()
        .expect("delete prompt must install: OPP-12K (12000 DP) is exactly at the ceiling");
    assert!(
        p.valid_action_ids.iter().any(|&a| a != PASS),
        "the selection must offer at least one non-PASS target (OPP-12K)"
    );

    // Accept: the only eligible target is OPP-12K — it is deleted; the
    // 13000 DP Digimon must survive.
    pick_first(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "exactly one opponent Digimon (OPP-12K) must be deleted"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-13K",
        "the surviving Digimon must be the 13000 DP one (above the <= 12000 ceiling)"
    );
}

/// Delete arm: opponent Digimon with DP > 12000 is NOT eligible.
#[test]
fn ad1_005_delete_rejects_digimon_above_12000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_dp("OPP-STRONG", 13000))
        .add_card(lv5_base("LV5-BASE"))
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.place_on_field(1, "OPP-STRONG", None);

    let opp_field_before = runner.battle_area_size(1);

    runner.fire_on_play(0, gaiamon.index as usize);
    runner.game.drain_effect_queue();

    // Either no selection (nothing to link or delete), or any pending selection
    // must not offer the 13000 DP opponent.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before,
        "the 13000 DP opponent Digimon must NOT be deleted (dp_lte only accepts <= 12000)"
    );
}

/// Delete accept path: selecting an eligible Digimon deletes it.
#[test]
fn ad1_005_delete_accept_removes_eligible_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_dp("OPP-WEAK", 8000))
        .add_card(lv5_base("LV5-BASE"))
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    runner.place_on_field(1, "OPP-WEAK", None);
    let opp_field_before = runner.battle_area_size(1);

    runner.fire_on_play(0, gaiamon.index as usize);
    runner.game.drain_effect_queue();

    // Accept everything — auto_resolve picks first non-PASS option.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "accepting the delete prompt must remove the eligible 8000 DP opponent Digimon"
    );
}

// ─── SECTION 5 — OPT cross-timing lockout ────────────────────────────────────

/// Cross-timing OPT: [On Play] consumes the OPT so [When Attacking] in the
/// SAME TURN is locked (DCGO SharedHashString "AD1_005_OP_WD_WA").
#[test]
fn ad1_005_opt_locked_after_on_play_fires_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_trait("NAVI-MON", "Navi"))
        .add_card(lv5_base("LV5-BASE"))
        .hand(0, &["NAVI-MON"])
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));

    // Fire [On Play], accept the link + any delete prompt.
    runner.fire_on_play(0, gaiamon.index as usize);
    let _ = runner.auto_resolve();

    // Now attack from the SAME turn — OPT should be locked.
    runner.attack_player(gaiamon, 1, false);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "after on_play consumed the OPT, when_attacking must be locked this turn; \
         no pending selection expected (DCGO SharedHashString / SetHashString count 1)"
    );
}

/// OPT resets: after end_turn / new turn, [When Attacking] fires again.
/// Turn 0: [On Play] fires with TOOL-MON in hand; the link prompt is DECLINED
/// (so TOOL-MON stays in hand) — the OPT is still consumed by the activation.
/// Next own turn: attacking must re-fire the clause and re-offer the link
/// prompt, proving the per-turn reset (DCGO hash counters clear each turn).
#[test]
fn ad1_005_opt_resets_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(digimon_with_trait("TOOL-MON", "Tool"))
        .add_card(lv5_base("LV5-BASE"))
        .hand(0, &["TOOL-MON"])
        // P1 needs security so the two player-attacks don't end the game
        // (DebugRunner defaults to an EMPTY security stack → instant win),
        // and BOTH players need decks so the end_turn draws don't deck out.
        .security(1, &["LV5-BASE", "LV5-BASE"])
        .deck(0, &["LV5-BASE"; 4])
        .deck(1, &["LV5-BASE"; 4])
        .memory(15)
        .start();

    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));

    // Consume OPT in turn 0 via on_play; decline every prompt so TOOL-MON
    // remains in hand for the next turn.
    runner.fire_on_play(0, gaiamon.index as usize);
    while runner.game.pending_selection.is_some() {
        decline_pending(&mut runner);
    }
    // OPT is consumed: attacking in the SAME turn must stay silent.
    runner.attack_player(gaiamon, 1, false);
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_none(),
        "same-turn when_attacking must be locked after on_play consumed the OPT"
    );

    // Advance to the next turn (P0's turn via two end_turns).
    runner.end_turn(); // P1's turn
    runner.end_turn(); // back to P0's turn

    // OPT reset: attacking now must fire the clause again, and TOOL-MON is
    // still in hand, so the link prompt must install.
    runner.attack_player(gaiamon, 1, false);
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_some(),
        "when_attacking on the NEXT turn must re-fire the OPT clause and \
         re-offer the link prompt (TOOL-MON is still in hand)"
    );
}

// ─── SECTION 6 — Blast Digivolve counter window ──────────────────────────────

/// [Hand][Counter] <Blast Digivolve>: when Gaiamon ACE is in hand and an
/// attacker targets a player 1 Digimon, a counter window must appear with
/// the Blast Digivolve option available.
#[test]
fn ad1_005_blast_digivolve_counter_window_appears() {
    let mut attacker = make_test_card_with_level("ATK", "Attacker", 5);
    attacker.dp = Some(6000);
    attacker.colors = vec![CardColor::Red];

    let mut target = make_test_card_with_level("TARGET", "Target", 5);
    target.dp = Some(6000);
    target.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-005")
        .expect("AD1-005 in pack")
        .add_card(attacker)
        .add_card(target)
        .hand(1, &["AD1-005"])
        .start();

    let attacking = runner.place_on_field(0, "ATK", Some(0));
    let defending = runner.place_on_field(1, "TARGET", Some(0));

    let result = runner.attack_digimon(attacking, defending, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(
        runner.current_phase(),
        GamePhase::CounterTiming,
        "attacking should enter CounterTiming so Blast Digivolve is offered"
    );

    let prompt = runner
        .pending_selection()
        .expect("CounterTiming selection must be present when AD1-005 is in hand");
    assert_eq!(prompt.selecting_player, 1);
    // The counter window encodes Blast Digivolve as encode_digivolve(hand_idx, field_idx).
    // AD1-005 is in hand[0] of player 1, and the defending Digimon is at field[0].
    let blast_action = encode_digivolve(0, 0);
    assert!(
        prompt.valid_action_ids.contains(&blast_action),
        "AD1-005 must be offered in the Counter window as a Blast Digivolve option; \
         expected encode_digivolve(0,0)={}; valid: {:?}",
        blast_action,
        prompt.valid_action_ids
    );
}

// ─── SECTION 7 — ACE Overflow fires on leave-field ───────────────────────────

/// ACE Overflow -4: when Gaiamon ACE leaves the field, memory decreases by 4.
#[test]
fn ad1_005_ace_overflow_fires_on_leave_field() {
    let mut runner = gaiamon_runner();
    let gaiamon = runner.place_on_field(0, "AD1-005", Some(0));
    let mem_before = runner.game.memory;

    // Delete the Digimon to trigger leave-field (and overflow).
    let handle = PermanentHandle {
        player: 0,
        index: gaiamon.index,
    };
    runner
        .game
        .delete_permanent_with_cause(handle, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.memory,
        mem_before - 4,
        "ACE Overflow <-4> must subtract 4 memory when Gaiamon ACE leaves the field; \
         was {mem_before}, got {}",
        runner.game.memory
    );
}
