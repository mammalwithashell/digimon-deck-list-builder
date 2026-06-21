//! Real-card digivolution cost-choice coverage (rule 17 — no auto-selection).
//!
//! `digivolve_cost_choice.rs` pins the cost-choice MECHANISM with synthetic cards.
//! This suite pins it on the REAL archetype cards whose printed alt-evo boxes create a
//! genuine choice — the Ice-Snow / Suzune Kazuki line. Each Ice-Snow Digimon prints a
//! standard digivolution cost AND a cheaper alt: either
//! "[Digivolve] … w/[Ice-Snow] trait: Cost N" (Frigimon / PolarBearmon / Skadimon /
//! Icemon) or "[Digivolve] [Hiyarimon]: Cost 0" (Penguinmon). A base that satisfies
//! BOTH routes must surface a cost CHOICE (an `EffectChoice` with one option per
//! distinct cost), never auto-pay the cheaper one.
//!
//! The cheaper alt is gated on the base's [Ice-Snow] trait (or the [Hiyarimon] name) —
//! the same trait whose production-data loss (recovered in
//! `promote-official-bandai-card-source`) hid this choice in real games. The capstone
//! `*_from_production_cryspaledramon` test drives the choice off the REAL EX7-021
//! CardData built from `cards.json`, so it fails if the trait recovery regresses.
//!
//! NOTE on Appmon: no *implemented* Appmon card presents a simultaneous digivolution
//! cost choice. Its App-Fusion alt (Cost 0) digivolves over a LOWER-level fusion host
//! that can't also satisfy the higher-level normal route, and the one implemented
//! [Three Musketeers]-alt card (BT21-071 Scopemon, same cost 2 / 2) collapses. The
//! Appmon section therefore pins the *negative* contract: same-cost routes collapse to
//! one (no spurious prompt), so the trait/form recovery did not invent a phantom choice.
//!
//! CAVEAT (verified gap, not a regression): two UNIMPLEMENTED cards print a genuine
//! distinct-cost alt that WOULD create a choice once authored as a YAML `alt_paths`
//! block — BT21-074 Satellamon (Appmon: "[Three Musketeers] in text: Cost 3" vs standard
//! Cost 4 → {4,3}) and BT24-026 Hyogamon (Ice-Snow: "Lv.3 w/[Demon]/[TS]: Cost 2" vs
//! standard 3 → {3,2}). Both are JSON-only stubs today (no alt_paths, no test), so the
//! choice can't fire; add them here when the cards are implemented.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;

// ─── Shared fixtures ──────────────────────────────────────────────────────────

/// A Lv.`level` base of one colour, optionally carrying the [Ice-Snow] trait.
fn base(id: &str, level: u8, color: CardColor, ice_snow: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.traits = if ice_snow {
        vec!["Ice-Snow".to_string()]
    } else {
        vec!["Beast".to_string()]
    };
    c
}

/// A two-colour Lv.`level` base (no Ice-Snow) — for the Appmon same-cost-collapse guard.
fn two_colour_base(id: &str, level: u8, a: CardColor, b: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.colors = vec![a, b];
    c
}

/// A Hiyarimon base: Lv.2 blue, NAMED "Hiyarimon" so it matches the "[Hiyarimon]"
/// name-gated alt path AND the standard Lv.2-blue route.
fn hiyarimon_base() -> CardData {
    let mut c = make_test_card("HIYARIMON-BASE", "Hiyarimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Lesser".to_string()];
    c
}

/// Build a runner with `card_id` (DSL-loaded) in hand atop `base_card` on the field.
fn runner(card_id: &str, base_card: CardData) -> (DebugRunner, usize) {
    let base_id = base_card.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} must load from the embedded pack: {e}"))
        .add_card(base_card)
        .hand(0, &[card_id])
        .memory(15)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

/// The cost-choice option labels are "Digivolve for cost N" — match the exact `cost N`
/// token so a base level digit can never collide with a cost digit.
fn cost_token(cost: i16) -> String {
    format!("cost {cost}")
}

/// Assert that digivolving `card_id` over `base_card` surfaces a cost CHOICE between
/// exactly the two given costs, and that picking EACH option pays exactly that many
/// memory. Proves both the prompt and that the choice is honored (not overridden by the
/// min-cost auto-pick).
fn assert_cost_choice(card_id: &str, base_card: CardData, cheaper: i16, costlier: i16) {
    // 1) The prompt surfaces with two distinct-cost options.
    let (mut r, slot) = runner(card_id, base_card.clone());
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        !proceeded,
        "{card_id}: base satisfies BOTH routes — digivolve must PAUSE for a cost choice, \
         not auto-pay the cheaper ({cheaper})"
    );
    let view = r
        .pending_selection_view()
        .unwrap_or_else(|| panic!("{card_id}: a cost-choice selection must be pending"));
    assert_eq!(view.kind, SelectionKind::EffectChoice, "{card_id}: cost choice is an EffectChoice");
    let choices = view
        .effect_choices
        .as_ref()
        .unwrap_or_else(|| panic!("{card_id}: the cost choice must carry effect_choices"));
    let labels: Vec<String> = choices.iter().map(|c| c.label.to_lowercase()).collect();
    assert_eq!(
        choices.len(),
        2,
        "{card_id}: exactly two distinct costs ({cheaper} and {costlier}) must be offered; got {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.contains(&cost_token(cheaper))),
        "{card_id}: an option for the cheaper cost {cheaper} (labels {labels:?})"
    );
    assert!(
        labels.iter().any(|l| l.contains(&cost_token(costlier))),
        "{card_id}: an option for the costlier cost {costlier} (labels {labels:?})"
    );

    // 2) The choice is HONORED, not min-auto-picked: paying the costlier option leaves
    // exactly (costlier - cheaper) less memory than paying the cheaper one. Taking the
    // DIFFERENCE between the two runs cancels any When-Digivolving / On-Play memory
    // effect the card resolves afterwards (e.g. EX8-022 Frigimon "gain 1 memory"), which
    // is identical across both runs.
    let mem_after_cheaper = pick_returns_memory(card_id, base_card.clone(), cheaper);
    let mem_after_costlier = pick_returns_memory(card_id, base_card, costlier);
    assert_eq!(
        mem_after_cheaper - mem_after_costlier,
        costlier - cheaper,
        "{card_id}: paying the cost-{costlier} route must cost exactly {} more memory than \
         the cost-{cheaper} route (choice honored, not min-auto-picked)",
        costlier - cheaper
    );
}

/// Re-stage, pick the "Digivolve for cost N" option, assert the evolver lands on top,
/// and return the resulting memory (so the caller can difference two picks).
fn pick_returns_memory(card_id: &str, base_card: CardData, cost: i16) -> i16 {
    let (mut r, slot) = runner(card_id, base_card);
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    let view = r.pending_selection_view().expect("cost choice pending");
    let choices = view.effect_choices.clone().expect("effect_choices");
    let token = cost_token(cost);
    let opt = choices
        .iter()
        .find(|c| c.label.to_lowercase().contains(&token))
        .unwrap_or_else(|| panic!("{card_id}: an option labelled with cost {cost}"));
    r.execute_action(0, opt.action_id)
        .unwrap_or_else(|e| panic!("{card_id}: resolve cost choice (cost {cost}): {e}"));
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        card_id,
        "{card_id}: the evolver must be stacked on top after the digivolve resolves"
    );
    r.game.memory
}

/// Assert that digivolving `card_id` over `base_card` does NOT prompt for a cost choice
/// (only one distinct route applies) and completes immediately paying exactly `cost`.
/// Use only for cards whose On-Play / When-Digivolving effects don't alter memory in the
/// staged board (otherwise use the cost-choice difference check or `assert_no_cost_prompt`).
fn assert_no_prompt(card_id: &str, base_card: CardData, cost: i16) {
    let (mut r, slot) = runner(card_id, base_card);
    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        proceeded,
        "{card_id}: a single applicable route must complete immediately (no spurious cost prompt)"
    );
    assert!(r.pending_selection().is_none(), "{card_id}: no selection may be pending");
    assert_eq!(
        mem_before - r.game.memory,
        cost,
        "{card_id}: the lone applicable route costs {cost} memory"
    );
}

/// Assert only that NO cost-choice prompt surfaces: the digivolve completes immediately
/// (a single distinct route) and the evolver lands on top. Robust to the card's own
/// On-Play / When-Digivolving effects, which may legitimately prompt or change memory
/// afterwards — used for Appmon, where same-cost multi-colour routes collapse to one.
fn assert_no_cost_prompt(card_id: &str, base_card: CardData) {
    let (mut r, slot) = runner(card_id, base_card);
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        proceeded,
        "{card_id}: the lone (collapsed same-cost) route must digivolve WITHOUT a cost-choice prompt"
    );
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        card_id,
        "{card_id}: the evolver must land on top (no cost choice was needed)"
    );
}

// ─── Penguinmon — standard Lv.2 (cost 1) vs [Hiyarimon] (cost 0) ───────────────

#[test]
fn ex11_014_penguinmon_hiyarimon_base_prompts_1_vs_0() {
    assert_cost_choice("EX11-014", hiyarimon_base(), 0, 1);
}
#[test]
fn ex11_014_penguinmon_non_hiyarimon_blue_lv2_no_prompt() {
    assert_no_prompt("EX11-014", base("PLAIN", 2, CardColor::Blue, false), 1);
}
#[test]
fn ex8_019_penguinmon_hiyarimon_base_prompts_1_vs_0() {
    assert_cost_choice("EX8-019", hiyarimon_base(), 0, 1);
}
#[test]
fn ex8_019_penguinmon_non_hiyarimon_blue_lv2_no_prompt() {
    assert_no_prompt("EX8-019", base("PLAIN", 2, CardColor::Blue, false), 1);
}

// ─── Frigimon — standard Lv.3 (cost 3) vs Lv.3 w/[Ice-Snow] (cost 2) ───────────

#[test]
fn ex11_015_frigimon_blue_ice_snow_base_prompts_3_vs_2() {
    assert_cost_choice("EX11-015", base("B", 3, CardColor::Blue, true), 2, 3);
}
#[test]
fn ex11_015_frigimon_blue_non_ice_snow_no_prompt_cost_3() {
    assert_no_prompt("EX11-015", base("B", 3, CardColor::Blue, false), 3);
}
#[test]
fn ex11_015_frigimon_offcolour_ice_snow_no_prompt_cost_2() {
    assert_no_prompt("EX11-015", base("B", 3, CardColor::Red, true), 2);
}
#[test]
fn ex8_022_frigimon_blue_ice_snow_base_prompts_3_vs_2() {
    assert_cost_choice("EX8-022", base("B", 3, CardColor::Blue, true), 2, 3);
}
#[test]
fn ex8_022_frigimon_blue_non_ice_snow_no_cost_prompt() {
    // Only the standard Lv.3-blue route applies → no cost choice. (Memory not asserted:
    // this printing gains 1 memory on [When Digivolving], so use the no-prompt-only check.)
    assert_no_cost_prompt("EX8-022", base("B", 3, CardColor::Blue, false));
}

// ─── PolarBearmon — standard Lv.4 (cost 4) vs Lv.4 w/[Ice-Snow] (cost 3) ───────

#[test]
fn ex11_016_polarbearmon_blue_ice_snow_base_prompts_4_vs_3() {
    assert_cost_choice("EX11-016", base("B", 4, CardColor::Blue, true), 3, 4);
}
#[test]
fn ex11_016_polarbearmon_blue_non_ice_snow_no_prompt_cost_4() {
    assert_no_prompt("EX11-016", base("B", 4, CardColor::Blue, false), 4);
}
#[test]
fn ex8_023_polarbearmon_blue_ice_snow_base_prompts_4_vs_3() {
    assert_cost_choice("EX8-023", base("B", 4, CardColor::Blue, true), 3, 4);
}
#[test]
fn ex8_023_polarbearmon_blue_non_ice_snow_no_cost_prompt() {
    assert_no_cost_prompt("EX8-023", base("B", 4, CardColor::Blue, false));
}
#[test]
fn ex8_023_polarbearmon_offcolour_ice_snow_no_cost_prompt() {
    assert_no_cost_prompt("EX8-023", base("B", 4, CardColor::Red, true));
}

// ─── Skadimon — standard Lv.5 (cost 4) vs Lv.5 w/[Ice-Snow] (cost 3) ───────────

#[test]
fn ex11_017_skadimon_blue_ice_snow_base_prompts_4_vs_3() {
    assert_cost_choice("EX11-017", base("B", 5, CardColor::Blue, true), 3, 4);
}
#[test]
fn ex11_017_skadimon_blue_non_ice_snow_no_prompt_cost_4() {
    assert_no_prompt("EX11-017", base("B", 5, CardColor::Blue, false), 4);
}
#[test]
fn ex11_017_skadimon_offcolour_ice_snow_no_prompt_cost_3() {
    assert_no_prompt("EX11-017", base("B", 5, CardColor::Red, true), 3);
}
#[test]
fn ex8_028_skadimon_blue_ice_snow_base_prompts_4_vs_3() {
    assert_cost_choice("EX8-028", base("B", 5, CardColor::Blue, true), 3, 4);
}

// NOTE: Icemon (P-215) is intentionally NOT covered here. Its printed alt
// "[Digivolve] Lv.3 w/[Ice-Snow]/[Mineral]/[Rock] trait: Cost 2" is authored in the YAML
// as COLOUR-gated (`from: { color_is: blue/black }`) rather than TRAIT-gated, so the
// trait-based cost choice does not surface as printed — a separate authoring bug flagged
// for follow-up, not exercised by this suite.

// ─── Capstone: production-data base (the trait-recovery end-to-end proof) ──────

/// Skadimon's {4,3} cost choice driven by the REAL EX7-021 CrysPaledramon CardData
/// built from `cards.json` (Lv.5 / Blue / [Dragonkin, Ice-Snow] after the official-DB
/// trait recovery). If the recovery regresses (Ice-Snow dropped), CrysPaledramon stops
/// satisfying the Ice-Snow alt → only the standard cost-4 route → this test fails.
/// Runs on a large-stack thread: building production card data parses all of cards.json.
#[test]
fn ex11_017_skadimon_from_production_cryspaledramon_prompts_4_vs_3() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let production = digimon_engine::deck_tools::full_card_data();
            let mut crys = production
                .get("EX7-021")
                .expect("EX7-021 present in cards.json")
                .clone();
            assert!(
                crys.traits.iter().any(|t| t.eq_ignore_ascii_case("Ice-Snow")),
                "EX7-021 production CardData must carry the recovered Ice-Snow trait; got {:?}",
                crys.traits
            );
            // Re-key the production CardData under a fixture id the builder can place.
            crys.card_id = "PROD-CRYS".to_string();
            assert_cost_choice("EX11-017", crys, 3, 4);
        })
        .expect("spawn large-stack thread")
        .join()
        .expect("production-base cost-choice thread panicked");
}

// ─── Appmon — negative contract (no simultaneous digivolution cost choice) ─────

/// BT21-023 (Global) prints two same-cost normal routes (Red Lv.4 / Yellow Lv.4, both
/// cost 4). A two-colour base satisfies both, but they share a cost, so they collapse
/// to ONE distinct route → NO prompt. (App Fusion is Cost 0 but needs the named cards
/// linked to a lower-level host, so it does not apply here.) Guards that the Appmon
/// form/trait recovery did not invent a phantom cost choice.
#[test]
fn bt21_023_same_cost_two_colour_base_no_spurious_prompt() {
    assert_no_cost_prompt(
        "BT21-023",
        two_colour_base("RY", 4, CardColor::Red, CardColor::Yellow),
    );
}

/// BT21-073 (Mind Control): Purple Lv.4 / Black Lv.4, both cost 4 → same collapse.
#[test]
fn bt21_073_same_cost_two_colour_base_no_spurious_prompt() {
    assert_no_cost_prompt(
        "BT21-073",
        two_colour_base("PB", 4, CardColor::Purple, CardColor::Black),
    );
}
