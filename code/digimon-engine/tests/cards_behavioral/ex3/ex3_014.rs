//! EX3-014 Dorbickmon — Digimon, Lv.6, Red. DP 12000, Play cost 13.
//!
//! # Card text (cards.json / printed)
//!
//! ＜Rush＞ (This Digimon can attack the turn it comes into play.)
//! [On Play] Delete 1 of your opponent's Digimon with 3000 DP or less. For
//! each card with [Dragon], [saur] or [Ceratopsian] in any of its traits in
//! this Digimon's digivolution cards, add 2000 to the maximum DP you can
//! choose with this effect.
//!
//! DigiXros Requirements [DigiXros -2] 5 Digimon cards w/different names and
//! [Dragon], [saur] or [Ceratopsian] in one of their traits.
//!
//! Inherited effect: none. Security effect: none.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX3/Red/EX3_014.cs
//! - EffectTiming.None → RushSelfStaticEffect (static <Rush>).
//! - [On Play]: maxDP = 3000 + 2000 * (count of this card's digivolution
//!   cards where HasDragonTraits — covers [Dragon], [saur], [Ceratopsian]);
//!   SelectPermanent over opponent Digimon with DP <= maxDP, maxCount =
//!   min(1, matches), canNoSelect: false (MANDATORY when >=1 valid target),
//!   Mode.Destroy. No valid target → nothing installs.
//! - DigiXros: 5 elements, each "1 Digimon card with [Dragon]/[saur]/
//!   [Ceratopsian] in one of its traits"; different card NAMES enforced; the
//!   DigiXros reduce arg is 2 (each placed card reduces play cost by 2).
//!
//! # Patterns this test covers
//! - H1 Rush (static printed keyword — declarative grant_keyword)
//! - F-adjacent: [On Play] delete with a DP cap that scales with the
//!   carrier's digivolution sources (formula source-stack-count, trait-filtered).
//!   The trait filter uses `trait_contains` (substring, DCGO `ContainsTraits`):
//!   e.g. a `[Dinosaur]` source matches "saur" and a `[Dragonkin]` source
//!   matches "Dragon". Exact `trait_has` would leave the `saur` clause dead.
//! - E-adjacent: mandatory select surfaced via pending_selection (no auto-select).
//! - C4 DigiXros source play (5 distinct Dragon-trait Digimon, cost_delta -2).
//! - Standard digivolve alt-path (Lv.5 Red).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledDistinctBy,
    CompiledRepeat, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX3-014";

fn opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, 5);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.colors = vec![CardColor::Blue];
    card
}

/// A Dragon-family digivolution source (used to scale the DP cap). The
/// `trait_name` is intentionally a REALISTIC trait that only matches the
/// printed "[Dragon]/[saur]/[Ceratopsian] in any of its traits" via SUBSTRING
/// containment (e.g. `Dragonkin` contains "Dragon", `Dinosaur` contains
/// "saur") — never as an exact-equal trait. This proves `trait_contains`
/// (DCGO `ContainsTraits`) rather than the old exact `trait_has`, under which
/// the `saur` clause was completely dead.
fn dragon_source(id: &str, name: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 5);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(9000);
    card.colors = vec![CardColor::Red];
    card.traits = vec![trait_name.to_string()];
    card
}

/// A non-Dragon digivolution source (must NOT scale the DP cap). `"Beast"`
/// contains none of "Dragon" / "saur" / "Ceratopsian" as a substring, so it
/// is a clean negative under `trait_contains`.
fn plain_source(id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 4);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(8000);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Beast".to_string()];
    card
}

/// A Dragon-family Digimon usable as a DigiXros material. As with
/// `dragon_source`, `trait_name` is a realistic SUBSTRING-matching trait
/// (e.g. `Dragonkin`, `Dinosaur`) — it must qualify via `trait_contains`,
/// not exact `trait_has`.
fn xros_material(id: &str, name: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 5);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(7000);
    card.colors = vec![CardColor::Red];
    card.traits = vec![trait_name.to_string()];
    card
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX3-014 YAML loads")
        // Dragon-family digivolution sources, one per qualifying trait. Each
        // uses a REALISTIC trait that matches only via substring containment:
        //   - "Dragonkin"   contains "Dragon"
        //   - "Dinosaur"    contains "saur"  (the clause that was DEAD under
        //                                     exact `trait_has` — saur never
        //                                     appears as a standalone trait)
        //   - "Ceratopsian" contains "Ceratopsian"
        .add_card(dragon_source("DRG-SRC", "DragonSource", "Dragonkin"))
        .add_card(dragon_source("SAUR-SRC", "SaurSource", "Dinosaur"))
        .add_card(dragon_source("CERA-SRC", "CeraSource", "Ceratopsian"))
        .add_card(plain_source("PLAIN-SRC", "PlainSource"))
        // Opponent Digimon at various DP bands.
        .add_card(opp_digimon("OPP-3000", "Opp3000", 3000))
        .add_card(opp_digimon("OPP-4000", "Opp4000", 4000))
        .add_card(opp_digimon("OPP-5000", "Opp5000", 5000))
        .add_card(opp_digimon("OPP-6000", "Opp6000", 6000))
}

fn runner() -> DebugRunner {
    base_builder().memory(10).start()
}

// ── Structural ──────────────────────────────────────────────────────

#[test]
fn ex3_014_loads_with_expected_metadata() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("EX3-014 compiles");
    assert_eq!(card.name, "Dorbickmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
}

#[test]
fn ex3_014_grants_static_rush() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("EX3-014 compiles");

    let has_rush = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Rush"
        )
    });
    assert!(
        has_rush,
        "EX3-014 must declare a static <Rush> grant_keyword clause"
    );
}

#[test]
fn ex3_014_has_one_on_play_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("EX3-014 compiles");

    let on_play: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(on_play.len(), 1, "EX3-014 has exactly one [On Play] clause");
}

#[test]
fn ex3_014_declares_standard_digivolve_alt_path() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("EX3-014 compiles");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Digivolve),
        "EX3-014 must declare a standard digivolve alt path (Lv.5 Red)"
    );
}

#[test]
fn ex3_014_declares_digixros_five_distinct_dragon_materials_each_minus_two() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("EX3-014 compiles");

    let digixros = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("EX3-014 must declare a DigiXros alt path");

    // The recipe is a single repeated slot requiring exactly 5 cards with
    // different names, each reducing play cost by 2.
    assert_eq!(
        digixros.materials.len(),
        1,
        "DigiXros recipe is a single repeated material slot"
    );
    let mat = &digixros.materials[0];
    assert_eq!(
        mat.repeat,
        Some(CompiledRepeat::Range { min: 5, max: 5 }),
        "DigiXros recipe must require exactly 5 material cards"
    );
    assert_eq!(
        mat.distinct_by,
        Some(CompiledDistinctBy::Name),
        "DigiXros materials must have different names"
    );
    assert_eq!(
        mat.cost_delta,
        Some(-2),
        "each DigiXros material must reduce play cost by 2"
    );
}

// ── Behavioral: On Play delete, base cap (no Dragon sources) ─────────

#[test]
fn ex3_014_on_play_base_cap_offers_only_dp_3000_or_less() {
    let mut runner = runner();
    // Carrier with no digivolution sources → base cap 3000.
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-3000", Some(0));
    runner.place_on_field(1, "OPP-4000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("On Play must surface a mandatory delete selection (valid target exists)");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert!(
        !pending.is_optional,
        "delete is MANDATORY when a valid target exists (canNoSelect: false)"
    );

    // Only the 3000-DP Digimon is within the base cap of 3000; the 4000-DP one
    // is out. Exactly one eligible target.
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the 3000-DP opponent Digimon may be chosen at base cap (4000 excluded)"
    );
}

#[test]
fn ex3_014_on_play_base_cap_deletes_chosen_target() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-3000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let trash_before = runner.game.players[1].trash.len();
    let action_id = runner
        .pending_selection()
        .expect("delete selection installed")
        .valid_action_ids
        .first()
        .copied()
        .expect("at least one eligible delete target");
    runner
        .execute_action(0, action_id)
        .expect("delete the 3000-DP opponent Digimon");
    runner.auto_resolve().expect("resolve the delete");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the 3000-DP opponent Digimon must be deleted"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        trash_before + 1,
        "the deleted Digimon's top card moves to trash"
    );
}

#[test]
fn ex3_014_on_play_no_valid_target_installs_nothing() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    // Only a 4000-DP Digimon on the opponent side → above base cap of 3000.
    runner.place_on_field(1, "OPP-4000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection installs when no opponent Digimon is within the DP cap"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the out-of-cap Digimon must remain on the field"
    );
}

// ── Behavioral: scaling cap from Dragon-trait digivolution sources ───

#[test]
fn ex3_014_one_dragon_source_raises_cap_to_5000() {
    let mut runner = runner();
    // Carrier built with one Dragon-trait source beneath the top card.
    // place_stack: first id = bottom source, last = top card.
    let carrier = runner.place_stack(0, &["DRG-SRC", CARD_ID]);
    runner.place_on_field(1, "OPP-5000", Some(0));
    runner.place_on_field(1, "OPP-6000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("On Play must surface a delete selection");
    assert_eq!(pending.kind, SelectionKind::OppField);

    // Cap is 3000 + 2000*1 = 5000. The 5000-DP Digimon is now eligible; 6000 is
    // not. Exactly one eligible target.
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "one Dragon source raises the cap to 5000; only the 5000-DP Digimon is eligible (6000 excluded)"
    );
}

#[test]
fn ex3_014_three_dragon_sources_raise_cap_to_9000() {
    let mut runner = runner();
    // Dragon + saur + Ceratopsian sources → cap 3000 + 2000*3 = 9000.
    let carrier = runner.place_stack(0, &["DRG-SRC", "SAUR-SRC", "CERA-SRC", CARD_ID]);
    runner.place_on_field(1, "OPP-6000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    // Cap is 3000 + 2000*3 = 9000 (Dragon + saur + Ceratopsian sources all
    // qualify). The 6000-DP Digimon is now within cap.
    let pending = runner
        .pending_selection()
        .expect("On Play must surface a delete selection at cap 9000");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "three Dragon-family sources raise the cap to 9000; the 6000-DP Digimon is eligible"
    );
}

#[test]
fn ex3_014_dinosaur_source_counts_via_saur_substring() {
    // LOAD-BEARING substring proof. SAUR-SRC carries the realistic trait
    // "Dinosaur", which contains "saur". Under the OLD exact `trait_has: saur`
    // this source matched NOTHING (the `saur` clause was completely dead — no
    // card has a standalone "saur" trait), so the cap would stay at base 3000
    // and the 4000-DP Digimon would be out of cap. With `trait_contains: saur`
    // the cap rises to 3000 + 2000 = 5000 and the 4000-DP target is eligible.
    let mut runner = runner();
    let carrier = runner.place_stack(0, &["SAUR-SRC", CARD_ID]);
    runner.place_on_field(1, "OPP-4000", Some(0));
    runner.place_on_field(1, "OPP-6000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let pending = runner.pending_selection().expect(
        "a single [Dinosaur] source must raise the cap to 5000 via the `saur` substring \
         (this selection would NOT install under the old exact-match `trait_has: saur`)",
    );
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "cap 5000 from one [Dinosaur] source: only the 4000-DP Digimon is eligible (6000 excluded)"
    );
}

#[test]
fn ex3_014_non_dragon_source_does_not_raise_cap() {
    let mut runner = runner();
    // A Beast (non-qualifying) source must NOT raise the cap above base 3000.
    let carrier = runner.place_stack(0, &["PLAIN-SRC", CARD_ID]);
    runner.place_on_field(1, "OPP-4000", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "a non-Dragon digivolution source must not raise the cap; 4000 stays out of cap 3000"
    );
}

// ── Behavioral: <Rush> ──────────────────────────────────────────────

#[test]
fn ex3_014_has_rush_on_field() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(carrier, Keyword::Rush),
        "EX3-014 must carry <Rush> as a static keyword"
    );
}

// ── Behavioral: DigiXros source play ────────────────────────────────

#[test]
fn ex3_014_digixros_five_field_materials_reduce_cost_and_attach_sources() {
    // Five distinct Dragon-family Digimon on the field as materials. Their
    // traits qualify only via SUBSTRING containment (Dragonkin⊃Dragon,
    // Dinosaur⊃saur, Ceratopsian⊃Ceratopsian) — proving the DigiXros material
    // filter uses `trait_contains`, not exact `trait_has`.
    let mut runner = base_builder()
        .add_card(xros_material("XR-1", "XrosOne", "Dragonkin"))
        .add_card(xros_material("XR-2", "XrosTwo", "Beast Dragon"))
        .add_card(xros_material("XR-3", "XrosThree", "Dinosaur"))
        .add_card(xros_material("XR-4", "XrosFour", "Ceratopsian"))
        .add_card(xros_material("XR-5", "XrosFive", "Dragonkin"))
        .memory(10)
        .hand(0, &[CARD_ID])
        .start();

    for id in ["XR-1", "XR-2", "XR-3", "XR-4", "XR-5"] {
        runner.place_on_field(0, id, Some(0));
    }

    let memory_before = runner.memory();
    let _ = runner.play(0, 0); // EX3-014 is the only hand card (index 0)

    let prompt = runner
        .pending_selection()
        .expect("DigiXros play must ask for recipe materials before paying cost");
    assert!(
        prompt.is_optional,
        "DigiXros material selection must allow stopping (you MAY place)"
    );
    // Select all five Dragon-trait materials (slots 0..5).
    for slot in 0..5u16 {
        let pending = runner
            .pending_selection()
            .expect("material prompt should re-install per material");
        assert!(
            pending.valid_action_ids.contains(&slot),
            "field material slot {slot} must be selectable"
        );
        runner
            .execute_action(0, slot)
            .unwrap_or_else(|_| panic!("select DigiXros material slot {slot}"));
    }
    runner
        .execute_action(0, PASS)
        .expect("stop selecting DigiXros materials");

    // Play cost 13 reduced by 5 * -2 = -10 → effective cost 3.
    assert_eq!(
        memory_before - runner.memory(),
        3,
        "EX3-014 cost 13 reduced by five DigiXros -2 materials should cost 3"
    );

    let dorbickmon = &runner.game.players[0].battle_area[0];
    assert_eq!(
        dorbickmon.top_card().card_id(&runner.game.card_data),
        CARD_ID
    );
    // Top card + 5 attached materials.
    assert_eq!(
        dorbickmon.card_sources.len(),
        6,
        "all five field materials should become digivolution sources beneath Dorbickmon"
    );
}
