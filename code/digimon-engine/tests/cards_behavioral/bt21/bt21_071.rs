//! BT21-071 Scopemon — Digimon, Lv.4, Purple, DP 4000, Cost 4.
//! Traits: Sup. / Appmon / Tool / Monitoring.  Attribute: Tool.
//!
//! # Card text (DCGO BT21_071.cs — authoritative)
//!
//! Digivolve paths:
//!   - Standard circle: PURPLE Lv.3 / Cost 2 (printed purple circle;
//!     cards.json evo_costs `{card_color: 6, level: 3, memory_cost: 2}`).
//!   - Alt: [Stnd.]-form trait / Cost 2 — trait-only, NO level gate
//!     (DCGO `AddSelfDigivolutionRequirementStaticEffect(EqualsTraits("Stnd."), 2)`).
//!   - Alt: Lv.3 w/ [Three Musketeers] in text / Cost 2 — BROAD in-text scan
//!     (DCGO `HasText("Three Musketeers") && Level == 3`; catches trait-only carriers).
//!
//! [On Play] [When Digivolving]:
//!   By placing 1 card with the [Appmon] or [Three Musketeers] trait from your
//!   hand or trash as 1 of your Digimon's bottom digivolution card, gain 1 memory.
//!
//! Self link-condition: [Appmon] trait host / Cost 2.
//!
//! [When Linking]: ＜Draw 2＞ and trash 2 cards in your hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_071.cs
//!
//! # Patterns this test covers
//! - select_union_zone (hand + trash) with `then:` cost-branch + place_as_bottom_source
//! - [On Play] and [When Digivolving] shared effect body (two separate clauses)
//! - Self link-condition (kind: link_condition, cost 2, trait_has Appmon)
//! - [When Linking] linked-scope draw + mandatory hand-discard
//! - Alt-path via `in_text_contains` predicate (Three Musketeers "in text" — broad HasText scan)
//! - Alt-path via `trait_has: "Stnd."` (predecessor form trait, no level gate)
//! - Standard printed circle as color+level alt_path (purple Lv.3 / cost 2)
//! - Decline flow: no card selected → no memory gain

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-071";

// ─── Card factories ───────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn appmon_card(id: &str) -> CardData {
    make_digimon(id, 4, 4000, 4, &["Appmon"])
}

fn three_musketeers_card(id: &str) -> CardData {
    make_digimon(id, 4, 4000, 4, &["Three Musketeers"])
}

fn filler_card(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Lv.3 digimon in battle area that Scopemon can place cards under.
fn lv3_host(id: &str) -> CardData {
    make_digimon(id, 3, 2000, 3, &["Reptile"])
}

/// Push a card into P0's trash by direct CardSource injection.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in card_data"));
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game.players[0].trash.push(src);
}

/// Push a card into P0's hand by direct CardSource injection.
fn push_to_hand(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in card_data"));
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game.players[0].hand.push(src);
}

/// Build the field-effect bit index for the Link action of `perm`.
fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-071 YAML parses and compiles")
        .add_card(appmon_card("APPMON-HAND"))
        .add_card(appmon_card("APPMON-TRASH"))
        .add_card(three_musketeers_card("MUSKET-HAND"))
        .add_card(lv3_host("HOST-DIGI"))
        .add_card(filler_card("FILLER"))
        // An Appmon host for the on-field link absorb path.
        .add_card(make_digimon("APPMON-HOST", 4, 4000, 4, &["Appmon"]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt21_071_yaml_compiles_and_metadata() {
    let runner = base().memory(0).start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT21-071 in compiled pack");
    assert_eq!(card.name, "Scopemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
}

#[test]
fn bt21_071_has_link_condition_appmon_cost_2() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition {
                cost, ..
            }) if *cost == 2
        )
    });
    assert!(
        has,
        "BT21-071 must declare a self link-condition with cost 2"
    );
}

#[test]
fn bt21_071_has_on_play_and_when_digivolving_clauses() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has_on_play = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )
    });
    let has_when_digi = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    assert!(has_on_play, "BT21-071 must have an [On Play] clause");
    assert!(
        has_when_digi,
        "BT21-071 must have a [When Digivolving] clause"
    );
}

#[test]
fn bt21_071_has_when_linked_clause() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenLinked)
        )
    });
    assert!(
        has,
        "BT21-071 must have a [When Linking] (when_linked) clause"
    );
}

#[test]
fn bt21_071_registers_three_alt_paths() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        3,
        "BT21-071 must have exactly 3 digivolve alt-paths (purple Lv.3 standard circle + Stnd. circle + Three Musketeers in-text); got {}",
        digivolve_paths.len()
    );
    // All paths must be cost 2.
    assert!(
        digivolve_paths
            .iter()
            .all(|p| matches!(p.cost, Some(CompiledCost::Literal(2)))),
        "all digivolve paths on BT21-071 must have cost 2"
    );
}

/// Per-circle from-filter shapes (printed card face + DCGO BT21_071.cs):
///   1. Standard circle: PURPLE Lv.3 — must gate on BOTH level and color
///      (a colorless `level_eq: 3` would widen to any-color Lv.3 at cost 2).
///   2. "Stnd." circle: trait-only — DCGO `EqualsTraits("Stnd.")` carries NO
///      level check (and the printed circle shows "Stnd." in the level slot).
///   3. "[Digivolve] Lv.3 w/[Three Musketeers] in text: Cost 2" — DCGO
///      `HasText(...) && Level == 3`. "in text" is the BROAD whole-card scan
///      (`in_text_contains`, name + traits + printed text — see the card's
///      official Q&A), NOT `effect_text_contains` (which misses cards that
///      carry the [Three Musketeers] TRAIT without the literal string in
///      their effect text).
#[test]
fn bt21_071_alt_path_from_filters_match_printed_circles() {
    use digimon_dsl::compiled::CompiledColor;

    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let froms: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .map(|p| p.from.as_ref().expect("digivolve alt-path has from:"))
        .collect();

    // 1. Purple Lv.3 standard circle.
    assert!(
        froms
            .iter()
            .any(|f| f.level_eq == Some(3) && f.color_is == Some(CompiledColor::Purple)),
        "BT21-071 must gate its standard circle on purple Lv.3 (printed purple digivolve circle)"
    );
    // 2. Stnd. circle: trait-only, no level gate (DCGO EqualsTraits(\"Stnd.\") only).
    assert!(
        froms
            .iter()
            .any(|f| f.trait_has.as_deref() == Some("Stnd.") && f.level_eq.is_none()),
        "BT21-071 Stnd. circle must be trait-only with no level gate (DCGO EqualsTraits)"
    );
    // 3. Three Musketeers: Lv.3 + BROAD in-text scan.
    assert!(
        froms.iter().any(|f| f.level_eq == Some(3)
            && f.in_text_contains.as_deref() == Some("Three Musketeers")),
        "BT21-071 Three Musketeers path must use in_text_contains (DCGO HasText broad scan) at Lv.3"
    );
    assert!(
        froms.iter().all(|f| f.effect_text_contains.is_none()),
        "BT21-071 must NOT use effect_text_contains (misses [Three Musketeers]-TRAIT cards)"
    );
}

// ─── Section 1b — Alt-path behavioral digivolve routes ───────────────────────
//
// `Game::digivolve_from_hand` in Main phase exercises the same
// `alt_path_registry` route real gameplay uses (p_220 idiom).

fn purple_lv3(id: &str) -> CardData {
    use digimon_engine::enums::CardColor;
    let mut c = make_digimon(id, 3, 3000, 3, &["Reptile"]);
    c.colors = vec![CardColor::Purple];
    c
}

fn digivolve_runner(base_card: CardData) -> (DebugRunner, PermanentHandle) {
    let base_id = base_card.card_id.clone();
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-071 loads")
        .add_card(base_card)
        .add_card(filler_card("FILLER"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(20)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let base = runner.place_on_field(0, &base_id, Some(0));
    (runner, base)
}

/// Positive — standard printed circle: a PURPLE Lv.3 base digivolves at cost 2.
#[test]
fn bt21_071_can_digivolve_from_purple_lv3_standard_circle() {
    let (mut runner, base) = digivolve_runner(purple_lv3("PURPLE-LV3"));
    let proceeded = runner.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        proceeded,
        "BT21-071 must digivolve from a purple Lv.3 base (printed purple Lv.3 / cost 2 circle)"
    );
    let _ = runner.auto_resolve();
}

/// Negative — an off-color (red) Lv.3 base with no Stnd. trait and no
/// [Three Musketeers] anything must NOT be a legal base. Guards the
/// printed-circle color gate (a colorless `level_eq: 3` alt-path would
/// wrongly accept this base).
#[test]
fn bt21_071_cannot_digivolve_from_red_lv3_plain_base() {
    let (mut runner, base) = digivolve_runner(make_digimon("RED-LV3", 3, 3000, 3, &["Reptile"]));
    let proceeded = runner.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        !proceeded,
        "BT21-071 must NOT digivolve from a red Lv.3 base with no Stnd./Three Musketeers match \
         (standard circle is purple-only)"
    );
}

/// Positive — Stnd. circle: an off-color base with the "Stnd." form trait
/// digivolves at cost 2 (DCGO EqualsTraits(\"Stnd.\"), no color/level gate).
#[test]
fn bt21_071_can_digivolve_from_stnd_trait_base() {
    let (mut runner, base) =
        digivolve_runner(make_digimon("STND-BASE", 3, 3000, 3, &["Stnd.", "Appmon"]));
    let proceeded = runner.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        proceeded,
        "BT21-071 must digivolve from a [Stnd.]-form base via the Stnd. circle (trait-only gate)"
    );
    let _ = runner.auto_resolve();
}

/// Positive — the [Three Musketeers] alt-path must match a Lv.3 card that
/// carries the trait WITHOUT the literal string in its effect text (empty
/// text on test cards). This is exactly the case `effect_text_contains`
/// misses and DCGO's broad `HasText` (trait scan) catches.
#[test]
fn bt21_071_can_digivolve_from_three_musketeers_trait_only_base() {
    let (mut runner, base) = digivolve_runner(make_digimon(
        "MUSKET-BASE",
        3,
        3000,
        3,
        &["Three Musketeers"],
    ));
    let proceeded = runner.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        proceeded,
        "BT21-071 must digivolve from a Lv.3 [Three Musketeers]-TRAIT base (broad in-text scan; \
         effect_text_contains would miss this card)"
    );
    let _ = runner.auto_resolve();
}

// ─── Section 2 — [On Play] place+gain behavioral tests ───────────────────────

/// [On Play] with an Appmon card in hand → union-zone selection installs.
/// Selecting the card and a host Digimon → card placed as bottom source, +1 memory.
#[test]
fn bt21_071_on_play_place_appmon_from_hand_gains_memory() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();

    // Add an Appmon to P0's hand and a Digimon on field to receive the source.
    push_to_hand(&mut r, "APPMON-HAND");
    let host = r.place_on_field(0, "HOST-DIGI", Some(0));

    let field_stack_before = r.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();

    // Play Scopemon from hand index 0 (Scopemon is first in hand).
    r.play(0, 0).expect("Scopemon played");
    // Capture memory AFTER play cost is paid (before the on-play effect fires).
    let mem_after_play = r.game.memory;
    // Resolve: union-zone selection (pick APPMON-HAND), then select HOST-DIGI, then gain memory.
    r.auto_resolve().ok();

    let mem_after = r.game.memory;
    let field_stack_after = r.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();

    // +1 memory gained (cost paid = placement happened).
    assert_eq!(
        mem_after,
        mem_after_play + 1,
        "[On Play] placing Appmon from hand must gain 1 memory"
    );
    // HOST-DIGI gained a bottom source.
    assert_eq!(
        field_stack_after,
        field_stack_before + 1,
        "[On Play] HOST-DIGI must have gained a bottom digivolution source"
    );
}

/// [On Play] with an Appmon in trash → place from trash → gain memory.
#[test]
fn bt21_071_on_play_place_appmon_from_trash_gains_memory() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();

    push_to_trash(&mut r, "APPMON-TRASH");
    let host = r.place_on_field(0, "HOST-DIGI", Some(0));

    r.play(0, 0).expect("Scopemon played");
    // Capture memory AFTER play cost is paid.
    let mem_after_play = r.game.memory;
    r.auto_resolve().ok();

    let mem_after = r.game.memory;
    assert_eq!(
        mem_after,
        mem_after_play + 1,
        "[On Play] placing Appmon from trash must gain 1 memory"
    );
}

/// [On Play] with no eligible cards in hand or trash → no selection installs, no memory gain.
#[test]
fn bt21_071_on_play_no_eligible_cards_no_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-071 YAML parses")
        .add_card(filler_card("FILLER"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(5)
        .start();

    // No Appmon/Three Musketeers cards anywhere.
    r.play(0, 0).expect("Scopemon played");
    // Capture memory AFTER play cost is paid; effect should not change memory.
    let mem_after_play = r.game.memory;
    let _ = r.auto_resolve();

    let mem_after = r.game.memory;
    assert_eq!(
        mem_after, mem_after_play,
        "[On Play] with no eligible cards must not gain memory"
    );
}

/// [On Play] with no OTHER Digimon on the field: Scopemon itself is a legal
/// host — printed "1 of your Digimon" and DCGO `CanTuckUnderCondition`
/// (own battle-area Digimon, incl. the effect's own permanent) both include
/// it. Placing the Appmon under Scopemon gains 1 memory and grows its stack.
#[test]
fn bt21_071_on_play_can_place_under_itself_gains_memory() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();

    push_to_hand(&mut r, "APPMON-HAND");
    // No other Digimon on P0's field; Scopemon itself is the only host.

    r.play(0, 0).expect("Scopemon played");
    let mem_after_play = r.game.memory;
    let _ = r.auto_resolve();

    let mem_after = r.game.memory;
    assert_eq!(
        mem_after,
        mem_after_play + 1,
        "[On Play] Scopemon itself is a legal host ('1 of your Digimon') — placing under itself must gain 1 memory"
    );
    // Scopemon's own stack grew by the tucked card (top + 1 bottom source).
    let scopo_stack = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|p| p.card_sources.len())
        .expect("Scopemon on field");
    assert_eq!(
        scopo_stack, 2,
        "[On Play] the Appmon must be placed as Scopemon's bottom digivolution card"
    );
}

/// [On Play] decline the union-zone pick (PASS) → no placement, no memory.
#[test]
fn bt21_071_on_play_decline_no_placement_no_memory() {
    let mut r = base().hand(0, &[CARD_ID]).memory(5).start();

    push_to_hand(&mut r, "APPMON-HAND");
    let _host = r.place_on_field(0, "HOST-DIGI", Some(0));

    r.play(0, 0).expect("Scopemon played");
    // Capture memory AFTER play cost is paid; declining the effect must not change memory.
    let mem_after_play = r.game.memory;

    // Decline the union-zone pick.
    if let Some(_view) = r.pending_selection_view() {
        let _ = r.execute_action(0, PASS);
    }
    let _ = r.auto_resolve();

    let mem_after = r.game.memory;
    assert_eq!(
        mem_after, mem_after_play,
        "[On Play] declining the placement must not gain memory"
    );
}

// ─── Section 3 — [When Digivolving] place+gain behavioral tests ──────────────

/// [When Digivolving] with a Three Musketeers card in trash → place → gain memory.
/// Scopemon is placed directly as the top card of a permanent, then the WhenDigivolving
/// trigger is fired on that permanent's handle (matching the bt21_013 pattern).
#[test]
fn bt21_071_when_digivolving_place_three_musketeers_from_trash_gains_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-071 YAML parses")
        .add_card(lv3_host("HOST2"))
        .add_card(three_musketeers_card("MUSKET-TRASH"))
        .add_card(filler_card("FILLER"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(5)
        .start();

    // Place Scopemon on field (it is the top card of its permanent).
    // A second Digimon (HOST2) is needed to receive the bottom source.
    let scopo = r.place_on_field(0, CARD_ID, Some(0));
    let host2 = r.place_on_field(0, "HOST2", Some(0));
    push_to_trash(&mut r, "MUSKET-TRASH");

    let mem_before = r.game.memory;

    // Fire WhenDigivolving on the permanent whose top card is Scopemon.
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(scopo),
    );
    r.game.drain_effect_queue();
    r.auto_resolve().ok();

    let mem_after = r.game.memory;
    assert_eq!(
        mem_after,
        mem_before + 1,
        "[When Digivolving] placing Three Musketeers from trash must gain 1 memory"
    );
}

// ─── Section 4 — [When Linking] draw 2 + trash 2 tests ───────────────────────

/// [When Linking] draws 2 and trashes 2 hand cards.
/// Start with 4 hand cards; after linking, hand should have 4 - 2 = 2 (drew 2, trashed 2... net 0),
/// but actually: drew 2 (+2), then trashed 2 (-2) = net 0. So hand size stays the same.
/// Wait — the player starts with 4 in hand, draws 2 (now 6), trashes 2 (now 4). Net 0.
#[test]
fn bt21_071_when_linked_draw2_trash2_net_zero() {
    let mut r = base().memory(5).start();

    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let scopo = r.place_on_field(0, CARD_ID, Some(0));

    // Give P0 exactly 4 hand cards (fillers) so we can verify draw+trash.
    for _ in 0..4 {
        push_to_hand(&mut r, "FILLER");
    }
    let hand_before = r.hand_size(0);

    advance_to_main(&mut r);

    // Activate link action.
    r.game.decode_action(link_bit(scopo) as u16, 0);

    if let Some(sel) = r.game.pending_selection.as_ref() {
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
    }
    // WhenLinked: draw 2 then trash 2 from hand.
    r.auto_resolve().ok();
    r.auto_resolve().ok();

    let hand_after = r.hand_size(0);
    // Net = +2 drawn - 2 trashed = 0 net change.
    assert_eq!(
        hand_after, hand_before,
        "[When Linking] draw 2 + trash 2 must be net zero hand change"
    );
    // Trash has 2 new cards.
    assert_eq!(
        r.trash_size(0),
        2,
        "[When Linking] must trash exactly 2 cards from hand"
    );
}

/// [When Linking] from an EMPTY hand: draw 2 (deck has fillers), then the
/// mandatory trash consumes exactly the 2 drawn cards — hand back to 0 and
/// trash grows by 2. Exercises the Math.Min(2, HandCards.Count) accounting
/// when the pre-link hand contributes nothing.
#[test]
fn bt21_071_when_linked_empty_hand_draws_then_trashes_both() {
    let mut r = base().memory(5).start();

    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let scopo = r.place_on_field(0, CARD_ID, Some(0));

    // P0 has exactly 0 hand cards — draw 2 then trash min(2, hand) = 2.
    assert_eq!(r.hand_size(0), 0, "precondition: empty hand");
    let trash_before = r.trash_size(0);

    advance_to_main(&mut r);

    r.game.decode_action(link_bit(scopo) as u16, 0);
    if let Some(sel) = r.game.pending_selection.as_ref() {
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
    }
    r.auto_resolve().ok();
    r.auto_resolve().ok();

    // Drew 2 from an empty hand, then mandatorily trashed both.
    assert_eq!(
        r.hand_size(0),
        0,
        "[When Linking] from empty hand: draw 2 then trash 2 must end at 0 hand cards"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 2,
        "[When Linking] both drawn cards must end up in the trash"
    );
}

// ─── Section 5 — Link condition structural gate ───────────────────────────────

/// Only [Appmon] trait Digimon hosts should allow Scopemon to be linked;
/// a non-Appmon host must not surface the link action.
/// This test verifies the link-condition gate via action-mask inspection.
#[test]
fn bt21_071_link_condition_only_appmon_host() {
    let non_appmon = make_digimon("NON-APPMON", 4, 4000, 4, &["Dragon"]);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-071 YAML parses")
        .add_card(non_appmon)
        .add_card(filler_card("FILLER"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(5)
        .start();

    let scopo = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    // Place Scopemon in hand to test the Link action presence.
    // (The link action encodes as FIELD_EFFECT_SLOT_FOR_LINK on the candidate.)
    let link_action = link_bit(scopo) as u16;

    // Mask check: link action should be masked off (no eligible Appmon host).
    let mask = build_action_mask(&r.game, 0);
    // NOTE: link action is on the Scopemon permanent itself; the mask should show
    // whether the link can fire. With no Appmon host present, it should be masked (0.0).
    assert_eq!(
        mask[link_action as usize], 0.0f32,
        "Link action must be masked when no [Appmon] host is present; link_action={link_action}"
    );
}

// ── Link DP aura (+3000) — review-added ───────────────────────────────────────

/// Structural: BT21-071 declares a scope:linked +3000 DP aura (link-box bonus).
#[test]
fn bt21_071_has_linked_dp_aura_3000() {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(3000),
                ..
            })
        )
    });
    assert!(
        has,
        "BT21-071 declares scope:linked +3000 DP aura (link box)"
    );
}

/// Behavioral: host effective DP +3000 while BT21-071 is linked.
#[test]
fn bt21_071_linked_host_gains_3000_dp() {
    let mut r = base().memory(5).start();
    let host = r.place_on_field(0, "APPMON-HOST", Some(0));
    let dp_before = r.effective_dp(host).expect("host on field");

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 3000),
        "host effective DP +3000 while BT21-071 is linked"
    );
}
