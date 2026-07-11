//! BT25-045 Onmon — Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Trait line (image): Stnd./Appmon | Game | Online. Attribute: Game.
//!
//! # Card text (card image + official DB bundle data/card_bundles/BT25-045.md;
//! DCGO BT25_045.cs for behavior — cards.json mis-slots the WhenLinked clause
//! as "inherited")
//!
//! Digivolve box — the card prints BOTH requirements:
//!   (a) standard circle  "Green Lv.2 / cost 0" (official DB standard circle);
//!   (b) special condition "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
//!       (DCGO `AddSelfDigivolutionRequirementStaticEffect(HasAppmonTraits, 0, lvl 2)`).
//! Self link-condition: link onto an [Appmon] host for link cost 1
//!   (DCGO `AddSelfLinkConditionStaticEffect(HasAppmonTraits, 1)`).
//! Link box DP bonus: +2000 DP to the host while linked (official DB "Link DP: DP+2000").
//! [Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game] trait card
//!   would link to this Digimon, you may reduce the cost by 1.
//!   (DCGO `WhenWouldLink` ActivateClass; face-up, NOT inherited.)
//! [When Linking] Suspend 1 of your opponent's Digimon.
//!   (DCGO `WhenLinked`, `SetIsLinkedEffect(true)`; the card's ONLY lower-box
//!   clause — there is no separate inherited effect.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_045.cs
//!
//! # Audit note (2026-07-10, mirrors sibling BT25-007)
//! Fixed drift: missing printed standard circle (Green Lv.2 / cost 0), missing
//! link-box +2000 DP aura, attribute mis-authored Free -> Game, trait line
//! missing the Stnd. and Game segments.
//!
//! # Re-adjudication note (2026-06-07)
//! Prior verdict BLOCKED (engine, facet #10) because the host-filtered optional
//! `WhenWouldLink` reducer could not be authored. RESOLVED: Gap 5 landed
//! (`when: when_would_link_to_this` + `would_link_card_trait_any_of` +
//! `reduce_link_cost`). With the reducer now expressible AND the prior Shape-B
//! link / when_linked vocabulary, every clause is faithful — no omissions.
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - alt-digivolve registration (printed circle + trait-gated special condition).
//! - digivolve-route fidelity (bt21_009.rs Section 6 recipe).
//! - DigiLink Shape-B self link-condition.
//! - linked DP aura (link-box +2000).
//! - Face-up `when_would_link_to_this` reducer (Gap 5), optional + OPT,
//!   trait-gate positive AND negative.
//! - `when: when_linked` suspend payoff (linked scope).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-045";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.colors = vec![CardColor::Green];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

/// A [Social]+[Appmon] linking source with a cost-1 link condition over Appmon
/// hosts — matches the reducer's Social/Tool/Game trait gate (positive path).
const SOCIAL_LINK_SOURCE: &str = r#"
card: TEST-SOCIAL-LINK
name: Test Social Link Source
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
traits: [Social, Appmon]
effects:
  - kind: link_condition
    cost: 1
    filter: { trait_has: Appmon }
"#;

/// A [Search]+[Appmon] linking source — Search is NOT one of Social/Tool/Game,
/// so the reducer's trait gate must exclude it (negative path).
const SEARCH_LINK_SOURCE: &str = r#"
card: TEST-SEARCH-LINK
name: Test Search Link Source
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
traits: [Search, Appmon]
effects:
  - kind: link_condition
    cost: 1
    filter: { trait_has: Appmon }
"#;

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-045 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-DIGI", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("APP-LV2", 2, 1000, 2, &["Appmon"]))
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_045_yaml_printed_metadata() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Onmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.cost, Some(3));
    // Image + official DB: Attribute is Game (was mis-authored as Free).
    assert_eq!(card.attribute.as_deref(), Some("Game"));
}

/// Trait line from the card image: Stnd./Appmon | Game | Online — all
/// segments authored as traits (sibling BT25-007 / BT21-009 convention).
#[test]
fn bt25_045_traits_contain_all_image_segments() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t: Vec<&str> = card.traits.iter().map(String::as_str).collect();
    for expected in &["Stnd.", "Appmon", "Game", "Online"] {
        assert!(
            t.contains(expected),
            "trait '{}' not found in traits {:?}",
            expected,
            t
        );
    }
}

/// Special condition (black text): "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
/// — the trait-gated Lv.2 alt-digivolve path at cost 0 — plus the cost-1
/// self link-condition over [Appmon] hosts.
#[test]
fn bt25_045_has_appmon_alt_digivolve_and_link_condition() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let alt = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(2) && f.trait_has.as_deref() == Some("Appmon")
            })
    });
    assert!(alt, "must register cost-0 alt-digivolve over a Lv.2 Appmon");
    let link = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 1
    ));
    assert!(link, "must declare a self link-condition with cost 1");
}

/// Printed standard circle: "Green Lv.2 / cost 0" (card image digivolve circle
/// + official Bandai DB). Authored as a bare {level_eq, color_is} alt-path per
/// the printed-circle convention (tests/alt_path_printed_cost_guard.rs).
/// Pre-fix (2026-07-10) only the trait-gated path existed — same under-modeling
/// as siblings BT25-007 / BT21-009.
#[test]
fn bt25_045_has_standard_green_lv2_cost0_circle() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(2)
                    && f.color_is == Some(CompiledColor::Green)
                    && f.any_of.is_empty()
                    && f.trait_has.is_none()
            })
    });
    assert!(
        has,
        "BT25-045 prints a standard Green Lv.2 / cost 0 digivolve circle — it must be \
         authored alongside the trait-gated special condition"
    );
}

/// Linked DP aura: +2000 DP to the host while this card is linked
/// (link box "DP +2000", official DB "Link DP: DP+2000"). Was missing pre-fix
/// (2026-07-10).
#[test]
fn bt25_045_has_linked_dp_aura_2000() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
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
        "BT25-045 declares a scope:linked DP aura of +2000 (link-box DP bonus)"
    );
}

#[test]
fn bt25_045_has_faceup_would_link_reducer_optional_opt() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenWouldLinkToThis) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("reducer clause present");
    assert!(
        !matches!(t.scope, CompiledScope::Inherited),
        "Onmon's reducer is a face-up effect, not inherited"
    );
    assert!(t.optional, "'you may reduce' is optional");
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(
        t.process
            .iter()
            .any(|s| matches!(s, CompiledStep::ReduceLinkCost { amount: 1 })),
        "reduces link cost by 1"
    );
}

#[test]
fn bt25_045_has_linked_when_linking_suspend() {
    // The card image's lower box is a single [When Linking] suspend clause (the
    // linked effect). cards.json mis-slots it as "inherited"; DCGO models it as
    // the `WhenLinked` `SetIsLinkedEffect(true)` clause — so there is exactly
    // one linked-scope suspend, not a separate inherited one.
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let when_linked = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenLinked)
                    && matches!(t.scope, CompiledScope::Linked)
        )
    });
    assert!(
        when_linked,
        "must have a linked [When Linking] suspend clause"
    );
}

// ─── Section 2 — Link box: +2000 DP aura reaches the host ────────────────────

/// [Link] +2000 DP reaches the host while BT25-045 is linked
/// (link box "DP +2000"; recipe from bt25_007.rs Section 2b).
#[test]
fn bt25_045_linked_dp_bonus_2000_reaches_host() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let base_dp = r.game.effective_dp(host).expect("host has DP");

    r.push_linked_owned(host, CARD_ID, 0);
    // tick so the static DP aura is registered
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(host),
        Some(base_dp + 2000),
        "linked BT25-045 contributes +2000 DP to the host"
    );
}

// ─── Section 3 — Behavioral: [When Linking] suspends an opponent Digimon ──────

#[test]
fn bt25_045_when_linked_suspends_opp_digimon() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let onmon = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    assert!(
        !r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "opp Digimon starts unsuspended"
    );

    r.game.decode_action(link_bit(onmon) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();

    assert!(
        r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "[When Linking] suspended the opponent Digimon"
    );
}

// ─── Section 4 — Behavioral: face-up reducer trait gate (positive + negative) ─

#[test]
fn bt25_045_faceup_reducer_drops_social_link_cost() {
    let mut r = base()
        .from_dsl_yaml(SOCIAL_LINK_SOURCE)
        .expect("social link source compiles")
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    // Onmon is itself the host here (it is an Appmon-trait Digimon).
    let onmon = r.place_on_field(0, CARD_ID, Some(0));
    let src = r.place_on_field(0, "TEST-SOCIAL-LINK", Some(0));
    advance_to_main(&mut r);

    let mem_before = r.memory();
    r.game.decode_action(link_bit(src) as u16, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before,
        "accepting Onmon's [Social] reducer drops link cost 1 -> 0"
    );
    assert_eq!(
        r.game.player(0).battle_area[onmon.index as usize]
            .linked_cards
            .len(),
        1,
        "the [Social] source linked onto Onmon at the reduced cost"
    );
}

/// Trait-gate NEGATIVE: a [Search]/[Appmon] linking card is none of
/// [Social]/[Tool]/[Game], so the reducer must not fire — full link cost 1
/// is paid (recipe from bt25_004.rs Section 3).
#[test]
fn bt25_045_does_not_reduce_non_social_tool_game_link() {
    let mut r = base()
        .from_dsl_yaml(SEARCH_LINK_SOURCE)
        .expect("search link source compiles")
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let onmon = r.place_on_field(0, CARD_ID, Some(0));
    let src = r.place_on_field(0, "TEST-SEARCH-LINK", Some(0));
    advance_to_main(&mut r);

    let mem_before = r.memory();
    r.game.decode_action(link_bit(src) as u16, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before - 1,
        "a [Search]/[Appmon] (non Social/Tool/Game) linking card pays full link cost 1 \
         — the reducer's trait gate excludes it"
    );
    assert_eq!(
        r.game.player(0).battle_area[onmon.index as usize]
            .linked_cards
            .len(),
        1,
        "the non-matching link still resolves (just unreduced)"
    );
}

// ─── Section 5 — digivolve-route fidelity ─────────────────────────────────────
// The card prints BOTH requirements (card image + official DB bundle
// data/card_bundles/BT25-045.md):
//   (a) standard circle  "Green Lv.2 / cost 0"
//   (b) special condition "[Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
// Pre-fix (2026-07-10) only (b) was authored, so a plain green Lv.2 without
// [Appmon] had NO digivolve route into BT25-045 (same drift as BT25-007).
// Recipe: bt21_009.rs Section 6 / bt25_007.rs Section 4.

/// A Lv.2 base Digimon with explicit colors/traits for digivolve-route tests.
fn make_lv2_base(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 1;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Digivolve BT25-045 from hand over the given base; returns (proceeded, memory delta).
fn try_digivolve_over(base_card: CardData) -> (bool, i16) {
    use digimon_engine::enums::{GamePhase, PlaySource};
    let base_id = base_card.card_id.clone();
    let mut r = base()
        .add_card(base_card)
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
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

/// Standard circle (a): a plain green Lv.2 with NO [Appmon] trait digivolves
/// into BT25-045 for the printed circle cost (0).
/// Pre-fix this failed: the YAML only authored the trait-gated condition.
#[test]
fn bt25_045_digivolves_from_plain_green_lv2_for_0() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv2_base("GREEN-LV2-PLAIN", CardColor::Green, &[]));
    assert!(
        proceeded,
        "printed standard circle Green Lv.2 / cost 0: a plain green Lv.2 must be a legal base"
    );
    assert_eq!(delta, 0, "the printed circle cost is 0 — no memory paid");
}

/// Special condition (b): an off-colour (blue) Lv.2 with the [Appmon] trait
/// digivolves into BT25-045 for 0 via the trait-gated path.
#[test]
fn bt25_045_digivolves_from_offcolor_appmon_lv2_for_0() {
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
fn bt25_045_no_route_from_offcolor_traitless_lv2() {
    let (proceeded, delta) =
        try_digivolve_over(make_lv2_base("BLUE-LV2-PLAIN", CardColor::Blue, &[]));
    assert!(
        !proceeded,
        "a blue traitless Lv.2 matches neither the green circle nor the trait gate — \
         digivolve must be refused"
    );
    assert_eq!(delta, 0, "refused digivolve must not spend memory");
}
