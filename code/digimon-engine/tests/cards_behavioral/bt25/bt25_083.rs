//! BT25-083 LadyDevimon — Digimon, Lv.5, Purple, DP 7000, Cost 7.
//! Traits: Fallen Angel, Titan, TS. Attribute: Virus.
//!
//! # Card text (data/card_bundles/BT25-083.md — official Bandai DB, matches
//! data/cards.json `effect_description_eng`)
//!
//! Special digivolution condition: [Digivolve] Lv.4 w/[Three Musketeers] in
//! text or w/[TS] trait: Cost 3.
//!
//! [On Play] [When Digivolving] By placing 1 [Three Musketeers] trait card
//! from your hand or trash as any of your Digimon's bottom digivolution
//! cards, ＜Draw 1＞.
//! [When Digivolving] [When Attacking] [Once Per Turn] By trashing 1 Option
//! card from any of your Digimon's digivolution cards, you may use 1 [Three
//! Musketeers] trait Option card from your trash with the cost reduced by 3.
//!
//! Inherited Effect:
//! [On Deletion] You may play 1 level 4 or lower Digimon card with [Three
//! Musketeers] in its text from your trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_083.cs
//!
//! - Region "Alt Digivolution" (None): AddSelfDigivolutionRequirementStaticEffect
//!   (PermanentCondition: TopCard.HasTSTraits || TopCard.HasText("Three
//!   Musketeers"), level: 3, digivolutionCost: 4... wait, args are (condition,
//!   cost=3, ignoreReq=false, card, null, level: 4) — Lv.4 base, cost 3.
//! - Region "OP/WD Shared" (onPlay: true, whenDigivolving: true, optional:
//!   false at the DCGO shared-effect layer, but internally offers "Don't
//!   place" as a 3rd branch + canNoSelect:true on both hand/trash picks —
//!   modeled here as select_union_zone{hand,trash} optional:true): select 1
//!   [Three Musketeers]-trait card from hand or trash, then select which own
//!   Digimon receives it as bottom source (AddDigivolutionCardsBottom), then
//!   Draw 1.
//! - Region "WD/WA Shared" (whenDigivolving: true, whenAttacking: true,
//!   maxCountPerTurn: 1): select an own Digimon with an Option-kind
//!   digivolution source (CanSelectDigimonCondition), select+trash that
//!   Option card (ITrashDigivolutionCards), then optionally select a [Three
//!   Musketeers]-trait Option from trash to use with cost -3
//!   (CanSelect3MOptionCard + ChangeCostClass -3, PlayOptionCards).
//! - Region "Inherited" (OnDestroyedAnyone): select 1 level <=4
//!   [Three Musketeers]-in-text Digimon from trash, PlayForFree.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H/G alt digivolution path (level_eq + any_of[in_text_contains, trait_has])
//! - A6 place-as-bottom-source from hand/trash union zone + own-permanent target pick
//! - E2 optional + cost-as-selection (trash 1 Option from any Digimon's sources)
//! - G4-adjacent OPT (once_per_turn) cost->use-Option-from-trash with cost reduction
//! - Inherited [On Deletion] play-from-trash-free with level + in_text gate

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "BT25-083";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A no-op OptionMain effect so synthetic Option test cards have a real
/// registered effect to resolve through `use_option_from_trash` (mirrors
/// BT24-085's `OptionMainNoop`).
struct OptionMainNoop;

impl CardEffect for OptionMainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain no-op")
            .process(|_| {})
            .build()]
    }
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon with the given level + traits (used for the alt-digivolve base
/// fixture and generic field Digimon).
fn make_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(5000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

/// A Digimon whose printed effect text contains "Three Musketeers" (for the
/// inherited OnDeletion "[Three Musketeers] in its text" gate — text, not
/// trait).
fn make_tm_text_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_digimon(id, level, &[]);
    card.effect_text = "[On Play] Three Musketeers shenanigans.".to_string();
    card
}

fn make_option(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-083 YAML parses and compiles")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_digimon("LV4-TS", 4, &["TS"]))
        .add_card(make_digimon("LV4-TM-TEXT", 4, &[]))
        .add_card(make_digimon("LV4-PLAIN", 4, &["Beast"]))
        .add_card(make_digimon("TM-TRAIT-CARD", 3, &["Three Musketeers"]))
        .add_card(make_digimon("PLAIN-FIELD", 4, &["Beast"]))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_083_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-083 must be present in embedded DSL pack");

    assert_eq!(card.name, "LadyDevimon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Fallen Angel", "Titan", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-083 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_083_has_alt_digivolve_cost3_from_lv4_tm_or_ts() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        })
        .expect("BT25-083 must include a cost-3 digivolve alt-path (the printed evo box)");

    let from = path.from.as_ref().expect("alt-path has a from: filter");
    assert_eq!(from.level_eq, Some(4), "alt-path base must be level 4");
    // any_of[in_text_contains("Three Musketeers"), trait_has(TS)]
    let any_of = &from.any_of;
    assert!(
        !any_of.is_empty(),
        "alt-path from: must use any_of for the text-or-trait gate"
    );
    assert!(
        any_of
            .iter()
            .any(|p| p.in_text_contains.as_deref() == Some("Three Musketeers")),
        "alt-path any_of must include in_text_contains(\"Three Musketeers\")"
    );
    assert!(
        any_of.iter().any(|p| p.trait_has.as_deref() == Some("TS")),
        "alt-path any_of must include trait_has(TS)"
    );
}

#[test]
fn bt25_083_op_wd_clause_places_bottom_source_and_draws() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-083 must have an [On Play][When Digivolving] clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    // The place-as-bottom-source + draw steps live inside the
    // select_union_zone's `then:` tail (they only run once a card is picked).
    let union_then = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::SelectUnionZone { then, .. } => Some(then),
            _ => None,
        })
        .expect("OP/WD clause body must contain a select_union_zone step");
    let has_bottom_source = union_then
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. }));
    let has_draw = union_then
        .iter()
        .any(|s| matches!(s, CompiledStep::Draw { .. }));
    assert!(
        has_bottom_source,
        "OP/WD clause body must place a card as a bottom source"
    );
    assert!(has_draw, "OP/WD clause body must Draw 1");
}

#[test]
fn bt25_083_has_wd_wa_once_per_turn_clause_using_option_from_trash() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-083 must have a [When Digivolving][When Attacking] clause");

    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] -> once_per_turn must be true"
    );
    let has_use_option_from_trash = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::UseOptionFromTrash { .. }));
    assert!(
        has_use_option_from_trash,
        "WD/WA clause body must use_option_from_trash"
    );
}

#[test]
fn bt25_083_has_inherited_on_deletion_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnDeletion) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-083 must have an inherited [On Deletion] clause");

    assert!(
        clause.optional,
        "printed 'you may play' -> clause must be optional"
    );
    let has_free_play = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromTrashFree { .. }));
    assert!(
        has_free_play,
        "inherited OnDeletion body must play_from_trash_free"
    );
}

// ─── Section 2 — Behavior: [On Play][When Digivolving] place bottom source + Draw ─

/// Positive: a [Three Musketeers]-trait card in hand can be placed as any own
/// Digimon's bottom source, then Draw 1.
#[test]
fn bt25_083_op_places_tm_card_from_hand_and_draws() {
    let mut runner = base()
        .hand(0, &["TM-TRAIT-CARD"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_before = runner.hand_size(0);
    let sources_before = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .len();

    // Fire the OnPlay trigger directly via the effect queue (place_on_field
    // skips OnPlay dispatch — see RUST_DSL_TEST_API anti-pattern #11 — so
    // enqueue the timing explicitly).
    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "OnPlay clause must install the place-bottom-source union prompt"
    );
    runner
        .auto_resolve()
        .expect("resolve hand/trash pick + target pick + draw");

    let sources_after = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "the TM card must be added as the target Digimon's bottom source"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before, // -1 placed as source, +1 drawn = net 0
        "place 1 from hand (-1) then Draw 1 (+1) nets the same hand size"
    );
}

/// Positive: the same clause can also pull the [Three Musketeers]-trait card
/// from trash instead of hand.
#[test]
fn bt25_083_op_places_tm_card_from_trash_and_draws() {
    let mut runner = base().deck(0, &["DECK-PAD"; 10]).memory(10).start();

    let target = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));

    // Seed P0's trash with the TM-trait card.
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TM-TRAIT-CARD")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    let sources_before = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "OnPlay clause must install the place-bottom-source union prompt"
    );
    runner
        .auto_resolve()
        .expect("resolve trash pick + target pick + draw");

    let sources_after = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "the TM card must be added as the target Digimon's bottom source"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "the TM card must leave trash"
    );
}

/// Negative: no [Three Musketeers]-trait card in hand or trash -> the OnPlay
/// clause is a no-op.
#[test]
fn bt25_083_op_no_fire_without_tm_card() {
    let mut runner = base().hand(0, &["LV4-PLAIN"]).memory(10).start();

    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no [Three Musketeers]-trait card in hand or trash -> OnPlay clause is a no-op"
    );
}

/// Optional decline: the player may decline the whole "place as bottom
/// source" clause even when eligible cards + targets exist.
#[test]
fn bt25_083_op_decline_leaves_field_unchanged() {
    let mut runner = base()
        .hand(0, &["TM-TRAIT-CARD"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_before = runner.hand_size(0);
    let sources_before = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .len();

    runner.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner
        .execute_action(0, PASS)
        .expect("decline the union-zone prompt");

    let sources_after = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after, sources_before,
        "declining must not add a bottom source"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must not change hand size"
    );
}

// ─── Section 3 — Behavior: [When Digivolving][When Attacking][OPT] trash-Option cost + use ─

/// Positive: an own Digimon with an Option-kind digivolution source, and a
/// [Three Musketeers]-trait Option in trash -> the cost step (trash the
/// Option source) fires, then the reduced-cost use-from-trash prompt
/// installs.
#[test]
fn bt25_083_wd_wa_trashes_option_source_then_offers_reduced_use() {
    let mut runner = base()
        .add_card(make_option("STACK-OPT", 4, &[]))
        .add_card(make_option("TM-OPT-TRASH", 6, &["Three Musketeers"]))
        .memory(10)
        .start();
    runner.register_effect("TM-OPT-TRASH", Arc::new(OptionMainNoop));

    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    // Give the host an Option-kind digivolution source to satisfy the cost.
    runner.push_source(host, "STACK-OPT");

    // Seed P0's trash with a [Three Musketeers]-trait Option.
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TM-OPT-TRASH")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "WD/WA clause must install the trash-Option-source cost prompt"
    );
    // Navigate explicitly: TriggerOrder -> clause 1's union-zone pick is ALSO
    // eligible here (TM-OPT-TRASH satisfies clause 1's trait_has filter too,
    // with no kind restriction — faithful to the printed text/DCGO, which
    // gates only on HasThreeMusketeersTraits) — decline it (it is optional)
    // so clause 2 (WD/WA) resolves in isolation -> clause 2's cost pick ->
    // clause 2's reduced-use pick (accept it here).
    resolve_trigger_order_if_present(&mut runner);
    decline_clause1_union_pick_if_present(&mut runner);
    runner
        .auto_resolve()
        .expect("pay cost + use reduced Option");

    let host_sources_after = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    assert_eq!(
        host_sources_after,
        1, // just the top card remains (card_sources includes the top)
        "the Option digivolution source must have been trashed as the cost"
    );
    // The used [Three Musketeers] Option returns to trash after resolution —
    // per the Option-card lifecycle (rule: a used Option card is placed in
    // the trash after its effect resolves, regardless of origin zone) and
    // confirmed by `dispose_option`'s `OptionSubtype::Standard` branch
    // (`commit_option_trash_outcome`), which always re-trashes a
    // Standard-mode Option post-use. It is present in trash both before
    // (unused) and after (used-and-disposed) — the meaningful assertion is
    // that the cost was paid (checked above via memory / host_sources) and
    // that the Option is the SAME instance re-trashed, not a leftover from
    // never having been used. Verify via the memory spend (printed use cost
    // 6, reduced by 3 = 3) as the used-effect witness instead of trash
    // presence/absence, which is invariant either way.
    assert_eq!(
        runner.game.memory, 7,
        "the reduced-cost Option use must have paid exactly 3 memory (6 printed - 3 reduction) from the initial 10"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT-TRASH"),
        "the used [Three Musketeers] Option returns to trash after resolution (Option disposal lifecycle)"
    );
}

/// Drain a leading `TriggerOrder` selection (installed whenever 2+ clauses
/// share the same firing timing), picking the first offered order.
fn resolve_trigger_order_if_present(runner: &mut DebugRunner) {
    while matches!(runner.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        let act = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .first()
            .copied()
            .expect("TriggerOrder always has a valid action");
        runner
            .execute_action(0, act)
            .expect("resolve TriggerOrder pick");
    }
}

/// Decline clause 1's `select_union_zone{hand,trash}` place-as-bottom-source
/// pick if it is the current prompt (it is optional — PASS is legal).
fn decline_clause1_union_pick_if_present(runner: &mut DebugRunner) {
    if matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { zones })
            if zones == digimon_engine::selection::UnionZoneSet::HAND | digimon_engine::selection::UnionZoneSet::TRASH
    ) {
        runner
            .execute_action(0, PASS)
            .expect("decline clause 1's optional place-as-bottom-source pick");
    }
}

/// Negative: no own Digimon has an Option-kind digivolution source -> even if
/// the outer "you may activate" gate is accepted (it surfaces regardless of
/// downstream eligibility — the EX11-065/EX11-038 `select_union_zone`
/// precedent), the inner cost pick has zero eligible candidates and silently
/// no-ops (no forced/fabricated pick, no state change).
#[test]
fn bt25_083_wd_wa_no_fire_without_option_source() {
    let mut runner = base().memory(10).start();

    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    // Non-Option digivolution source only.
    runner.push_source(host, "PLAIN-FIELD");

    let host_sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    // Two clauses share the [When Digivolving] timing (clause 1 OP/WD and
    // clause 2 WD/WA), so a TriggerOrder pick surfaces first; clause 1 has no
    // TM-trait card available either, so declining/no-op there too.
    resolve_trigger_order_if_present(&mut runner);
    decline_clause1_union_pick_if_present(&mut runner);
    // Accept clause 2's outer gate (it surfaces regardless of eligibility).
    if matches!(runner.pending_kind(), Some(SelectionKind::Replacement)) {
        runner
            .accept_optional_trigger()
            .expect("accept clause 2's outer accept/decline gate");
    }

    // The inner select_union_zone{material, kind: option} step must find zero
    // candidates (PLAIN-FIELD is a Digimon, not an Option) and silently
    // no-op — no further prompt should install.
    if let Some(sel) = runner.pending_selection() {
        panic!(
            "unexpected pending selection after accepting the outer gate: kind={:?} prompt={:?} valid_action_ids={:?}",
            sel.kind, sel.prompt, sel.valid_action_ids
        );
    }
    let host_sources_after = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    assert_eq!(
        host_sources_after, host_sources_before,
        "no Option-kind digivolution source anywhere -> nothing gets trashed"
    );
}

/// OPT lockout: [Once Per Turn] must block a second activation on the same
/// turn (e.g. a subsequent [When Attacking] fire).
#[test]
fn bt25_083_wd_wa_opt_locks_second_activation_same_turn() {
    let mut runner = base()
        .add_card(make_option("STACK-OPT-1", 4, &[]))
        .add_card(make_option("STACK-OPT-2", 4, &[]))
        .memory(10)
        .start();

    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    runner.push_source(host, "STACK-OPT-1");
    runner.push_source(host, "STACK-OPT-2");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_some(),
        "first activation must install the cost prompt"
    );
    runner.auto_resolve().expect("resolve first activation");

    // Second attempt same turn (When Attacking leg) must not re-fire.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must lock the WD/WA clause on a second activation this turn"
    );
}

/// Declining the follow-up reduced-cost Option use still leaves the cost
/// (trashed Option source) paid — the "may use" is a separate optional leg
/// from the mandatory trash-as-cost.
#[test]
fn bt25_083_wd_wa_declining_reduced_use_still_pays_cost() {
    let mut runner = base()
        .add_card(make_option("STACK-OPT", 4, &[]))
        .add_card(make_option("TM-OPT-TRASH", 6, &["Three Musketeers"]))
        .memory(10)
        .start();
    runner.register_effect("TM-OPT-TRASH", Arc::new(OptionMainNoop));

    let ladydevimon = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "LV4-PLAIN", Some(0));
    runner.push_source(host, "STACK-OPT");

    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TM-OPT-TRASH")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(ladydevimon),
    );
    runner.game.drain_effect_queue();

    // Two clauses share [When Digivolving] (clause 1 OP/WD, clause 2 WD/WA);
    // drain any TriggerOrder pick(s) first. TM-OPT-TRASH also satisfies
    // clause 1's trait_has filter (no kind restriction, per the printed text
    // + DCGO), so decline clause 1's optional pick if it surfaces. Clause 2's
    // outer accept/decline gate then surfaces — accept it.
    resolve_trigger_order_if_present(&mut runner);
    decline_clause1_union_pick_if_present(&mut runner);
    if matches!(runner.pending_kind(), Some(SelectionKind::Replacement)) {
        runner
            .accept_optional_trigger()
            .expect("accept clause 2's outer accept/decline gate");
    }

    // Step 1: the mandatory cost pick (select_union_zone{material}, kind:
    // option) — pick the (only) eligible Option digivolution source.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: digimon_engine::selection::UnionZoneSet::MATERIAL
        }),
        "first prompt must be the trash-Option-source cost pick"
    );
    let cost_act = runner
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .first()
        .copied()
        .expect("the Option source is a valid cost pick");
    runner
        .execute_action(0, cost_act)
        .expect("pay the trash-Option-source cost");

    // Step 2: the follow-up reduced-cost Option-use prompt (select_trash) —
    // decline it.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "second prompt must be the reduced-cost Option-use pick"
    );
    assert!(
        runner.pending_is_optional(),
        "the 'you may use' leg must be declineable"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline the optional reduced-cost Option use");

    let host_sources_after = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    assert_eq!(
        host_sources_after, 1, // just the top card remains (card_sources includes the top)
        "the Option digivolution source must still be trashed as the cost, even if the follow-up use is declined"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT-TRASH"),
        "declining the reduced-cost use must leave the [Three Musketeers] Option in trash"
    );
}

// ─── Section 4 — Behavior: inherited [On Deletion] play lv<=4 TM-text Digimon free ─
//
// Per rule 25 / effect_queue.rs's `inherited_sources` dispatch, a card's
// `scope: inherited` clause only fires from a BELOW-TOP digivolution source
// on the deleted permanent's stack — never from the card as the lone/top
// form of its own permanent. So these tests place LadyDevimon as the BOTTOM
// source under a HOST Digimon (place_stack) and delete the whole permanent
// (the BT19-006 idiom), rather than deleting a standalone LadyDevimon.

/// Positive: a level 4 Digimon with "Three Musketeers" in its printed text in
/// trash can be played free when the stack (with LadyDevimon as a source) is
/// deleted.
#[test]
fn bt25_083_inherited_od_plays_lv4_tm_text_digimon_free() {
    let mut runner = base().memory(10).start();

    let stack = runner.place_stack(0, &[CARD_ID, "LV4-PLAIN"]);

    {
        let card = make_tm_text_digimon("TM-TEXT-TRASH", 4);
        runner.game.card_data.push(card);
        let idx = runner.game.card_data.len() - 1;
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    let battle_area_before = runner.battle_area_size(0);

    runner.game.delete_permanent_with_effects(stack);

    assert!(
        runner.pending_selection().is_some(),
        "inherited OnDeletion clause must install the play-from-trash-free prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the OnDeletion trigger");
    runner
        .auto_resolve()
        .expect("pick + play the TM-text Digimon free");

    assert_eq!(
        runner.battle_area_size(0),
        battle_area_before, // the whole stack left (-1), TM-text Digimon entered (+1) = net 0
        "the stack leaves (-1) and the TM-text Digimon enters (+1) nets the same field size"
    );
}

/// Negative: a level 5 TM-text Digimon in trash is NOT eligible (level 4 or
/// lower only).
#[test]
fn bt25_083_inherited_od_no_fire_for_level5_tm_text() {
    let mut runner = base().memory(10).start();

    let stack = runner.place_stack(0, &[CARD_ID, "LV4-PLAIN"]);

    {
        let card = make_tm_text_digimon("TM-TEXT-LV5", 5);
        runner.game.card_data.push(card);
        let idx = runner.game.card_data.len() - 1;
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    runner.game.delete_permanent_with_effects(stack);

    assert!(
        runner.pending_selection().is_none(),
        "level 5 TM-text Digimon in trash is not eligible -> no prompt"
    );
}

/// Negative: a level 4 Digimon in trash WITHOUT "Three Musketeers" in its
/// text/traits is not eligible.
#[test]
fn bt25_083_inherited_od_no_fire_without_tm_text() {
    let mut runner = base().memory(10).start();

    let stack = runner.place_stack(0, &[CARD_ID, "LV4-PLAIN"]);

    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "LV4-PLAIN")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    runner.game.delete_permanent_with_effects(stack);

    assert!(
        runner.pending_selection().is_none(),
        "no [Three Musketeers]-in-text Digimon in trash -> no prompt"
    );
}

/// Decline: the player may decline the inherited OnDeletion free-play.
#[test]
fn bt25_083_inherited_od_decline_leaves_trash_unchanged() {
    let mut runner = base().memory(10).start();

    let stack = runner.place_stack(0, &[CARD_ID, "LV4-PLAIN"]);

    {
        let card = make_tm_text_digimon("TM-TEXT-DCL", 4);
        runner.game.card_data.push(card);
        let idx = runner.game.card_data.len() - 1;
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    let trash_before = runner.trash_size(0);

    runner.game.delete_permanent_with_effects(stack);

    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner
        .decline_optional_trigger()
        .expect("decline the OnDeletion trigger");

    // trash_before was captured before LadyDevimon itself trashed; declining
    // must not remove the TM-text Digimon from trash.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-TEXT-DCL"),
        "declining must leave the TM-text Digimon in trash"
    );
}

// ─── Section 5 — Compilation smoke test ──────────────────────────────────────

#[test]
fn bt25_083_yaml_compiles() {
    let runner = base().start();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT25-083 must be present in the embedded DSL pack"
    );
}
