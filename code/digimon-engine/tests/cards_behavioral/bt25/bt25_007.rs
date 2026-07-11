//! BT25-007 Gatchmon — Digimon, Lv.3, Red, DP 2000, Cost 3.
//! Trait line (image): Stnd./Appmon | Social | Search. Attribute: Social.
//!
//! # Card text (card image + official DB bundle data/card_bundles/BT25-007.md;
//! DCGO BT25_007.cs for behavior — cards.json mis-slots the WhenLinked clause
//! as "inherited")
//!
//! Digivolve box — the card prints BOTH requirements:
//!   (a) standard circle  "Red Lv.2 / cost 0" (official DB standard circle);
//!   (b) special condition "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
//!       (DCGO `AddSelfDigivolutionRequirementStaticEffect(HasAppmonTraits, 0, lvl 2)`).
//! Self link-condition: this card may be linked onto an [Appmon]-trait Digimon
//!   for link cost 1 (DCGO `AddSelfLinkConditionStaticEffect(HasAppmonTraits, 1)`).
//! Link box DP bonus: +2000 DP to the host while linked (official DB "Link DP: DP+2000").
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Appmon] trait card and
//!   1 [Social], [Tool], [Reboot] or [Creation] trait card among them to the
//!   hand. Return the rest to the bottom of the deck.
//! [When Linking] Delete 1 of your opponent's Digimon with 3000 DP or less.
//!   (DCGO WhenLinked, SetIsLinkedEffect(true).)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_007.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - DigiLink Shape-B self link-condition (G-DSL-DIGILINK)
//! - when: when_linked triggered effect (linked scope)
//! - linked DP aura (link-box +2000)
//! - reveal-N two-bucket add-to-hand (select_reveal_buckets)
//! - alt-digivolve registration (printed circle + trait-gated special condition)
//! - digivolve-route fidelity (bt21_009.rs Section 6 recipe)
//! - delete by DP<=N

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-007";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Card added to deck/hand to drive the reveal buckets.
fn appmon_card(id: &str) -> CardData {
    make_digimon(id, 3, 2000, 3, &["Search", "Appmon"])
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-007 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Reveal pool: one Appmon, one Social, plus filler.
        .add_card(make_digimon("APPMON-X", 3, 2000, 3, &["Appmon"]))
        .add_card(make_digimon("SOCIAL-X", 3, 2000, 3, &["Social"]))
        // Host Appmon for the on-field link absorb path.
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        // Opponent Digimon: one <=3000, one >3000.
        .add_card(make_digimon("OPP-SMALL", 3, 3000, 3, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 4, 5000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_007_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Gatchmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.cost, Some(3));
    // Image + official DB: Attribute is Social (was mis-authored as Data).
    assert_eq!(card.attribute.as_deref(), Some("Social"));
}

/// Trait line from the card image: Stnd./Appmon | Social | Search — all
/// segments authored as traits (sibling BT21-009 convention).
#[test]
fn bt25_007_traits_contain_all_image_segments() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t: Vec<&str> = card.traits.iter().map(String::as_str).collect();
    for expected in &["Stnd.", "Appmon", "Social", "Search"] {
        assert!(
            t.contains(expected),
            "trait '{}' not found in traits {:?}",
            expected,
            t
        );
    }
}

#[test]
fn bt25_007_has_link_condition_appmon_cost_1() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 1
    ));
    assert!(
        has,
        "BT25-007 must declare a self link-condition with cost 1"
    );
}

/// Special condition (black text): "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
/// — the trait-gated Lv.2 alt-digivolve path at cost 0.
#[test]
fn bt25_007_registers_appmon_alt_digivolve() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(2) && f.trait_has.as_deref() == Some("Appmon")
            })
    });
    assert!(
        has,
        "BT25-007 must register the trait-gated Lv.2 w/[Appmon] alt-digivolve at cost 0"
    );
}

/// Printed standard circle: "Red Lv.2 / cost 0" (card image digivolve circle +
/// official Bandai DB). Authored as a bare {level_eq, color_is} alt-path per the
/// printed-circle convention (tests/alt_path_printed_cost_guard.rs). Pre-fix
/// (2026-07-10) only the trait-gated path existed — same under-modeling as
/// sibling BT21-009.
#[test]
fn bt25_007_has_standard_red_lv2_cost0_circle() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(2)
                    && f.color_is == Some(CompiledColor::Red)
                    && f.any_of.is_empty()
                    && f.trait_has.is_none()
            })
    });
    assert!(
        has,
        "BT25-007 prints a standard Red Lv.2 / cost 0 digivolve circle — it must be \
         authored alongside the trait-gated special condition"
    );
}

/// Linked DP aura: +2000 DP to the host while this card is linked
/// (link box "DP +2000", official DB "Link DP: DP+2000").
#[test]
fn bt25_007_has_linked_dp_aura_2000() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. })
                if *scope == CompiledScope::Linked && *dp_modifier == Some(2000)
        )
    });
    assert!(
        has,
        "BT25-007 declares a scope:linked DP aura of +2000 (link-box DP bonus)"
    );
}

#[test]
fn bt25_007_has_on_play_and_when_linked_clauses() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let on_play = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )
    });
    let when_linked = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenLinked)
        )
    });
    assert!(on_play, "BT25-007 must have an [On Play] clause");
    assert!(when_linked, "BT25-007 must have a [When Linking] clause");
}

// ─── Section 2 — On Play reveal-3 two-bucket add ─────────────────────────────

#[test]
fn bt25_007_on_play_reveals_and_adds_one_of_each_bucket() {
    // Top 3 of deck = [APPMON-X, SOCIAL-X, DECK-PAD] (last element = top).
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "SOCIAL-X", "APPMON-X"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    // Play Gatchmon from hand index 0 → On Play fires.
    let _g = r.play(0, 0).expect("Gatchmon played");
    // Resolve the two mandatory bucket picks (Appmon then Social) — each is a
    // real selection surfaced through pending_selection (no auto-pick in YAML).
    r.auto_resolve().ok();

    // Net hand change: -1 (played Gatchmon) +2 (added two revealed cards) = +1.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "On Play added exactly 2 revealed cards (one per bucket) net of the play"
    );
    // The non-selected revealed card went to deck bottom, not trash.
    assert_eq!(r.trash_size(0), 0, "remainder bottomed, not trashed");
}

/// Negative bucket: reveal contains no [Appmon] card — only the
/// Social/Tool/Reboot/Creation bucket can add (net hand +0 after the play);
/// the unpicked cards still go to the deck bottom, not trash. Mirrors DCGO's
/// SimplifiedRevealDeckTopCardsAndSelect, which skips an unsatisfiable bucket.
#[test]
fn bt25_007_on_play_no_appmon_revealed_adds_only_support_bucket() {
    // Top 3 of deck = [SOCIAL-X, DECK-PAD, DECK-PAD] — no Appmon revealed.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "DECK-PAD", "SOCIAL-X"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    let _g = r.play(0, 0).expect("Gatchmon played");
    r.auto_resolve().ok();

    // Net hand change: -1 (played Gatchmon) +1 (Social pick only) = 0.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "with no [Appmon] revealed only the support bucket adds a card"
    );
    assert_eq!(r.trash_size(0), 0, "remainder bottomed, not trashed");
}

// ─── Section 2b — Link box: +2000 DP aura reaches the host ───────────────────

/// [Link] +2000 DP reaches the host while BT25-007 is linked
/// (link box "DP +2000"; recipe from bt21_009.rs Section 2).
#[test]
fn bt25_007_linked_dp_bonus_2000_reaches_host() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let base_dp = r.game.effective_dp(host).expect("host has DP");

    r.push_linked_owned(host, CARD_ID, 0);
    // tick so the static DP aura is registered
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(host),
        Some(base_dp + 2000),
        "linked BT25-007 contributes +2000 DP to the host"
    );
}

// ─── Section 3 — When Linking: delete opp Digimon DP<=3000 ───────────────────

#[test]
fn bt25_007_when_linked_deletes_small_opp_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gatch = r.place_on_field(0, CARD_ID, Some(0));
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    // Activate the on-field Link ability and pick the host → absorb → WhenLinked.
    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    // WhenLinked delete prompt: resolve it (single eligible target).
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "WhenLinked deleted the <=3000 DP opponent Digimon"
    );
    assert_eq!(r.trash_size(1), 1, "deleted Digimon went to opponent trash");
}

#[test]
fn bt25_007_when_linked_cannot_delete_big_opp_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gatch = r.place_on_field(0, CARD_ID, Some(0));
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // 5000 DP — ineligible
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(1),
        opp_before,
        "5000-DP opponent Digimon is not an eligible delete target"
    );
}

// ─── Section 4 — digivolve-route fidelity ─────────────────────────────────────
// The card prints BOTH requirements (card image + official DB bundle
// data/card_bundles/BT25-007.md):
//   (a) standard circle  "Red Lv.2 / cost 0"
//   (b) special condition "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
// Pre-fix (2026-07-10) only (b) was authored, so a plain red Lv.2 without
// [Appmon] had NO digivolve route into BT25-007 (same drift as BT21-009).
// Recipe: bt21_009.rs Section 6.

/// A Lv.2 base Digimon with explicit colors/traits for digivolve-route tests.
fn make_lv2_base(
    id: &str,
    color: digimon_engine::enums::CardColor,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 1;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Digivolve BT25-007 from hand over the given base; returns (proceeded, memory delta).
fn try_digivolve_over(base_card: CardData) -> (bool, i16) {
    use digimon_engine::enums::{GamePhase, PlaySource};
    let base_id = base_card.card_id.clone();
    let mut r = base()
        .add_card(base_card)
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, &base_id, Some(0));

    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    r.game.drain_effect_queue();
    (proceeded, r.game.memory - mem_before)
}

/// Standard circle (a): a plain red Lv.2 with NO [Appmon] trait digivolves
/// into BT25-007 for the printed circle cost (0).
/// Pre-fix this failed: the YAML only authored the trait-gated condition.
#[test]
fn bt25_007_digivolves_from_plain_red_lv2_for_0() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) =
        try_digivolve_over(make_lv2_base("RED-LV2-PLAIN", CardColor::Red, &[]));
    assert!(
        proceeded,
        "printed standard circle Red Lv.2 / cost 0: a plain red Lv.2 must be a legal base"
    );
    assert_eq!(delta, 0, "the printed circle cost is 0 — no memory paid");
}

/// Special condition (b): an off-colour (blue) Lv.2 with the [Appmon] trait
/// digivolves into BT25-007 for 0 via the trait-gated path.
#[test]
fn bt25_007_digivolves_from_offcolor_appmon_lv2_for_0() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) =
        try_digivolve_over(make_lv2_base("BLUE-LV2-APPMON", CardColor::Blue, &["Appmon"]));
    assert!(
        proceeded,
        "special condition Lv.2 w/[Appmon]: an off-colour Appmon Lv.2 must be a legal base"
    );
    assert_eq!(delta, 0, "the special-condition cost is 0 — no memory paid");
}

/// Negative: an off-colour Lv.2 with NO [Appmon] trait matches neither
/// printed requirement — no digivolve route may exist.
#[test]
fn bt25_007_no_route_from_offcolor_traitless_lv2() {
    use digimon_engine::enums::CardColor;
    let (proceeded, delta) =
        try_digivolve_over(make_lv2_base("BLUE-LV2-PLAIN", CardColor::Blue, &[]));
    assert!(
        !proceeded,
        "a blue traitless Lv.2 matches neither the red circle nor the trait gate — \
         digivolve must be refused"
    );
    assert_eq!(delta, 0, "refused digivolve must not spend memory");
}
