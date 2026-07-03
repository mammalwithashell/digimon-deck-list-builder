//! BT24-063 Locomon — Digimon, Lv.5, Black, DP 7000, Cost 7.
//! Traits: Machine / Iliad / TS. Attribute: Data.
//! Standard digivolve (printed evo_costs): Lv.4 Black / cost 3.
//!
//! # Card text (data/card_bundles/BT24-063.md + cards.json — verbatim)
//!
//! <Collision> (During this Digimon's attack, all of your opponent's Digimon
//!   gain <Blocker>, and must block if possible.)
//! [On Play] [When Digivolving] Reveal the top 3 cards of your deck. You may
//!   play 1 play cost 5 or lower [Machine], [Cyborg] or [TS] trait card among
//!   them without paying the cost. Return the rest to the top or bottom of
//!   the deck.
//!
//! Inherited Effect:
//!   <Collision> (During this Digimon's attack, give all of your opponent's
//!   Digimon <Blocker>, and the opponent blocks if able.)
//!
//! Digivolution requirement (xros_req, official Bandai DB):
//!   "[Digivolve] Lv.4 w/[TS] trait: Cost 3"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Black/BT24_063.cs
//!
//! # DCGO crosscheck
//!
//! - Alt-digivolve: AddSelfDigivolutionRequirementStaticEffect gated on
//!   `TopCard.IsLevel4 && TopCard.HasTSTraits`, cost 3. Matches the printed
//!   xros_req and the official-DB bundle (Lv.4 w/[TS] trait: cost 3). The
//!   standard printed Lv.4 Black / cost 3 route (cards.json evo_costs) is
//!   also authored as an alt_path so the route exists for DSL-loaded
//!   fixtures (G-DSL-FIXTURE-EVO-COSTS: compiled cards carry no printed
//!   evo_costs) — both land at cost 3 off a Lv.4 base, matching BT24-059 /
//!   BT19-073's "standard + special" alt_path idiom.
//! - <Collision>: `CollisionSelfStaticEffect(false, ...)` (face-up) AND
//!   `CollisionSelfStaticEffect(isInheritedEffect: true, ...)` (inherited) —
//!   both fire at `EffectTiming.OnCounterTiming`. → two grant_keyword
//!   declaratives, one own-scope and one inherited-scope.
//! - Shared On Play / When Digivolving body
//!   (`SimplifiedRevealDeckTopCardsAndSelect`): reveal top 3, ONE bucket
//!   filtering `GetCostItself <= 5 && HasPlayCost && (EqualsTraits(Machine)
//!   || EqualsTraits(Cyborg) || EqualsTraits(TS))`, `canNoSelect: true`
//!   ("you may play"). `HasPlayCost` excludes Options (CEntity_Base:
//!   `!cardKind.Contains(Option) && PlayCost >= 0`), so eligible cards are
//!   Digimon OR Tamer. `PlayPermanentCards(payCost: false, isTapped: false)`
//!   — played UNSUSPENDED (unlike BT24-059's suspended play). The remainder
//!   uses `RemainingCardsPlace.DeckTopOrBottom` — a SINGLE player-elected
//!   top/bottom choice for the whole leftover pool (not per-card, not a
//!   fixed end). → reveal_top_deck(3) + select_reveal_buckets(min:0,max:1,
//!   filter: kind digimon|tamer AND (Machine|Cyborg|TS) AND cost<=5) +
//!   play_from_revealed_free + place_remainder_on_deck(position: choice).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A3 reveal + select-multi (reveal 3, choose <=1)
//! - E2-adjacent: optional bucket pick + mandatory remainder placement
//! - §3.1 StackPosition::Choice — player-elected top/bottom for the WHOLE
//!   remainder pool (BT25-038 idiom, but on place_remainder_on_deck instead
//!   of place_on_security)
//! - H5/H-Collision: <Collision> granted both face-up and inherited
//!   (BT19-073 idiom for the structural assertion; mechanic-level MUST-block
//!   behavior already covered by tests/combat/collision_mandatory.rs)
//! - G shared On Play / When Digivolving body (BT24-059 / BT25-071 idiom)
//! - H alt digivolution path (Lv.4 w/[TS] trait: cost 3)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-063";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A play cost 5 [Machine] Digimon — a legal reveal-play target.
fn machine_cost5(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 5;
    card.traits = vec!["Machine".to_string()];
    card
}

/// A play cost 4 [Cyborg] Digimon — a legal reveal-play target.
fn cyborg_cost4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 4;
    card.traits = vec!["Cyborg".to_string()];
    card
}

/// A play cost 3 [TS] Tamer — a legal reveal-play target (Tamer, not Digimon).
fn ts_tamer(id: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.play_cost = play_cost;
    card.level = None;
    card.dp = None;
    card.traits = vec!["TS".to_string()];
    card
}

/// A play cost 6 [Machine] Digimon — too expensive (cost > 5).
fn machine_cost6(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 6;
    card.traits = vec!["Machine".to_string()];
    card
}

/// A play cost 3 Digimon with NONE of the eligible traits.
fn untrait_cost3(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card.traits = vec!["Beast".to_string()];
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.colors = vec![CardColor::Red];
    card
}

fn locomon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-063 YAML loads")
}

fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_063_yaml_compiles_and_has_printed_metadata() {
    let runner = locomon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT24-063 present in embedded DSL pack");
    assert_eq!(compiled.card, "BT24-063");
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    for trait_name in ["Machine", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
}

/// Alt-path: [Digivolve] Lv.4 w/[TS] trait: Cost 3 (official Bandai DB
/// xros_req, DCGO PermanentCondition gate).
#[test]
fn bt24_063_has_ts_trait_alt_path_cost3() {
    let runner = locomon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    assert!(
        compiled.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        }),
        "must have a cost-3 digivolve alt-path; alt_paths: {:?}",
        compiled.alt_paths
    );
}

/// <Collision> is declared as a keyword grant, both face-up and inherited.
#[test]
fn bt24_063_grants_collision_face_up_and_inherited() {
    let runner = locomon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");

    let collision_grants: Vec<&CompiledDeclarativeClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d @ CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) if keyword == "Collision" => Some(d),
            _ => None,
        })
        .collect();
    assert!(
        !collision_grants.is_empty(),
        "must declare <Collision> via grant_keyword; clauses: {:?}",
        compiled.effects
    );
}

/// Exactly one shared [On Play]/[When Digivolving] triggered clause exists,
/// and it is not optional at the clause level (the reveal + remainder
/// placement are mandatory; only the play step is optional).
#[test]
fn bt24_063_has_shared_on_play_when_digivolving_clause() {
    let runner = locomon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");

    let shared: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        shared.len(),
        1,
        "exactly one shared On Play/When Digivolving clause"
    );
    assert!(
        !shared[0].optional,
        "clause itself is mandatory (reveal + remainder always resolve)"
    );
}

/// The shared clause reveals the top 3, offers exactly one optional (min:0,
/// max:1) bucket, plays the pick for free, and places the remainder with a
/// player-elected top/bottom choice.
#[test]
fn bt24_063_shared_clause_reveals_3_and_offers_optional_bucket() {
    let runner = locomon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let shared = compiled
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
        .expect("shared clause");

    assert!(
        shared
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "must reveal top 3"
    );

    let bucket_ok = shared.process.iter().any(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets.len() == 1
                && buckets[0].min == 0
                && buckets[0].max == 1)
    });
    assert!(bucket_ok, "must offer exactly one optional single-pick bucket");

    assert!(
        shared
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromRevealedFree { .. })),
        "must play the picked card for free"
    );

    // The remainder placement is a player-elected top/bottom choice (single
    // choice for the WHOLE remainder pool — DCGO RemainingCardsPlace.
    // DeckTopOrBottom), not a fixed top or bottom.
    assert!(
        shared.process.iter().any(|s| matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Choice,
                ..
            }
        )),
        "remainder placement must be a player-elected top/bottom choice"
    );
}

/// The remainder placement must NOT be a fixed top or fixed bottom — the
/// printed text says "top OR bottom" (a single choice), unlike BT24-059
/// (always trash) or BT25-071 (always bottom).
#[test]
fn bt24_063_remainder_is_not_fixed_position() {
    let runner = locomon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let shared = compiled
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
        .expect("shared clause");
    let fixed_top_or_bottom = shared.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Top,
                ..
            }
        ) || matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Bottom,
                ..
            }
        )
    });
    assert!(
        !fixed_top_or_bottom,
        "remainder must not be hard-coded to a fixed end"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: reveal-3 play-1 (positive branches)
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] reveals top 3; a play-cost-5 [Machine] card is a legal pick and
/// is offered via a RevealBucket selection.
#[test]
fn bt24_063_on_play_installs_optional_reveal_selection() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        // MACHINE-5 on top of deck (last in array) → revealed index 0.
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-5"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, loco);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    assert!(runner.pending_is_optional(), "'You may play' → optional");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "the [Machine] cost-5 card at reveal index 0 must be a legal pick"
    );
}

/// Picking the eligible card plays it UNSUSPENDED (DCGO isTapped: false) for
/// free (no memory spent), and installs a top/bottom choice for the rest.
#[test]
fn bt24_063_picking_eligible_card_plays_it_unsuspended_for_free() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-5"])
        .memory(0) // cannot afford cost-5 normally — must be free
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    fire_timing(&mut runner, EffectTiming::OnPlay, loco);
    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick the [Machine] card");
    runner.game.drain_effect_queue();

    // A player-elected top/bottom EffectChoice for the remainder must now be
    // pending — resolve it (either branch is fine for this assertion).
    let view = runner
        .pending_selection_view()
        .expect("remainder top/bottom choice must install");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let labels: Vec<&str> = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Top of deck", "Bottom of deck"]);
    runner.execute_branch(0).expect("choose top");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        battle_area_contains(&runner, 0, "MACHINE-5"),
        "the [Machine] card must be played onto the field"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "exactly one card played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_from_revealed_free must not spend memory"
    );
    let played = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "MACHINE-5")
        .expect("MACHINE-5 on field");
    assert!(
        !played.is_suspended,
        "the played card must enter UNSUSPENDED (DCGO isTapped: false)"
    );
}

/// A [Cyborg] trait card is also eligible (one of the three named traits).
#[test]
fn bt24_063_cyborg_trait_is_eligible() {
    let mut runner = locomon_runner()
        .add_card(cyborg_cost4("CYBORG-4"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "CYBORG-4"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, loco);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs for a [Cyborg] card");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "[Cyborg] cost-4 card at index 0 must be a legal pick"
    );
}

/// A [TS] trait TAMER (not a Digimon) is also eligible — DCGO HasPlayCost
/// admits both Digimon and Tamer.
#[test]
fn bt24_063_ts_tamer_is_eligible() {
    let mut runner = locomon_runner()
        .add_card(ts_tamer("TS-TAMER", 3))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "TS-TAMER"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, loco);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs for a [TS] Tamer");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "TS Tamer at index 0 must be a legal pick"
    );
}

/// [When Digivolving] fires the identical shared body.
#[test]
fn bt24_063_when_digivolving_installs_optional_reveal_selection() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-5"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::WhenDigivolving, loco);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs on When Digivolving");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "the eligible card must be a legal pick on When Digivolving too"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: negative / ineligible / decline branches
// ═══════════════════════════════════════════════════════════════════════════════

/// A play-cost-6 [Machine] card is too expensive (cost must be <=5) and is
/// NOT a legal pick.
#[test]
fn bt24_063_overcost_machine_card_not_eligible() {
    let mut runner = locomon_runner()
        .add_card(machine_cost6("MACHINE-6"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-6"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, loco);

    // With no eligible card, the bucket step auto-skips straight to the
    // mandatory remainder top/bottom EffectChoice (whose branch action ids
    // happen to share the same numeric range as SEL_REVEAL_START — HAND_
    // EFFECT_START == SEL_REVEAL_START == 30 — so the assertion must gate on
    // `view.kind`, not raw action-id magnitude).
    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::RevealBucket { .. }) {
            assert!(
                !view.valid_action_ids.contains(&SEL_REVEAL_START),
                "the cost-6 [Machine] card must not be a legal pick: {:?}",
                view.valid_action_ids
            );
        } else {
            assert_eq!(
                view.kind,
                SelectionKind::EffectChoice,
                "with no eligible pick, the only other legal prompt is the remainder top/bottom choice"
            );
        }
    }
}

/// A card with none of [Machine]/[Cyborg]/[TS] is NOT eligible even at a
/// legal cost.
#[test]
fn bt24_063_untraited_card_not_eligible() {
    let mut runner = locomon_runner()
        .add_card(untrait_cost3("PLAIN-3"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "PLAIN-3"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, loco);

    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::RevealBucket { .. }) {
            assert!(
                !view.valid_action_ids.contains(&SEL_REVEAL_START),
                "the untraited card must not be a legal pick: {:?}",
                view.valid_action_ids
            );
        } else {
            assert_eq!(
                view.kind,
                SelectionKind::EffectChoice,
                "with no eligible pick, the only other legal prompt is the remainder top/bottom choice"
            );
        }
    }
}

/// Declining the optional play still resolves cleanly: all 3 revealed cards
/// return to the deck (none trashed — this is a top/bottom RETURN, not a
/// trash), driven by the player-elected top/bottom choice for the remainder.
#[test]
fn bt24_063_declining_still_returns_all_three_to_deck() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-5"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, loco);
    // Decline the optional bucket pick.
    let view = runner.pending_selection_view().expect("bucket prompt");
    assert!(view.valid_action_ids.contains(&PASS) || runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline the optional play");
    runner.game.drain_effect_queue();

    // The mandatory top/bottom choice for the remainder must now install.
    let view = runner
        .pending_selection_view()
        .expect("remainder top/bottom choice installs even when declining the play");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("choose bottom");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "declining the play returns all 3 revealed cards to the deck (no card leaves)"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "no cards are trashed — this effect RETURNS the remainder, unlike BT24-059"
    );
    assert!(
        !battle_area_contains(&runner, 0, "MACHINE-5"),
        "nothing is played when the optional pick is declined"
    );
}

/// Negative: with no eligible card among the top 3, the reveal still runs (the
/// reveal + remainder placement are mandatory) but nothing is playable.
#[test]
fn bt24_063_no_eligible_card_installs_no_play_pick() {
    let mut runner = locomon_runner()
        .add_card(untrait_cost3("PLAIN-A"))
        .add_card(untrait_cost3("PLAIN-B"))
        .add_card(machine_cost6("MACHINE-6"))
        .deck(0, &["MACHINE-6", "PLAIN-B", "PLAIN-A"])
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);
    let field_before = runner.battle_area_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, loco);
    // No eligible pick — either no selection installs at all for the bucket
    // (auto-skips to the remainder placement) or only PASS is legal.
    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::RevealBucket { .. }) {
            assert!(
                !view.valid_action_ids.iter().any(|a| *a >= SEL_REVEAL_START),
                "no card should be pickable: {:?}",
                view.valid_action_ids
            );
            let _ = runner.execute_action(0, PASS);
            runner.game.drain_effect_queue();
        }
    }
    // Resolve the remainder top/bottom choice.
    if let Some(view) = runner.pending_selection_view() {
        assert_eq!(view.kind, SelectionKind::EffectChoice);
        runner.execute_branch(0).expect("choose top");
        runner.game.drain_effect_queue();
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no eligible card → nothing is played"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all 3 revealed cards return to the deck"
    );
}

/// Negative: with no opponent-independent state, the shared clause runs even
/// when the deck is short (fewer than 3 cards) — reveal count is capped by
/// what's available; no crash / stuck selection.
#[test]
fn bt24_063_shared_clause_handles_short_deck_without_stalling() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .deck(0, &["MACHINE-5"]) // only 1 card in deck
        .memory(0)
        .start();

    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, loco);

    // Drive to completion regardless of branch shape; must terminate cleanly.
    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 20, "selection loop did not terminate");
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Top/Bottom remainder placement (both branches explicitly)
// ═══════════════════════════════════════════════════════════════════════════════

/// Choosing "Top of deck" for the remainder places the un-picked cards back
/// on top (verifiable by the next reveal drawing them again in the same
/// relative order is out of scope here — we assert deck size + no trash).
#[test]
fn bt24_063_remainder_top_branch_keeps_deck_size_stable() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-5"])
        .memory(0)
        .start();
    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, loco);
    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick MACHINE-5");
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("remainder choice pending");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("choose Top of deck");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "only the played card leaves the deck; the other 2 return to the top"
    );
}

/// Choosing "Bottom of deck" for the remainder also returns the un-picked
/// cards (to the opposite end) without trashing them.
#[test]
fn bt24_063_remainder_bottom_branch_keeps_deck_size_stable() {
    let mut runner = locomon_runner()
        .add_card(machine_cost5("MACHINE-5"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "MACHINE-5"])
        .memory(0)
        .start();
    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, loco);
    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick MACHINE-5");
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("remainder choice pending");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("choose Bottom of deck");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "only the played card leaves the deck; the other 2 return to the bottom"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — <Collision> keyword grant (structural + surface behavior)
// ═══════════════════════════════════════════════════════════════════════════════

/// The carrier itself has <Collision> live on the field (face-up grant).
#[test]
fn bt24_063_carrier_has_collision_on_field() {
    let mut runner = locomon_runner().memory(10).start();
    let loco = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(loco, Keyword::Collision),
        "Locomon must carry <Collision> while on the field"
    );
}

/// Inherited <Collision>: a Digimon with Locomon as a digivolution SOURCE
/// (not the live top card) also carries <Collision> via the inherited grant.
#[test]
fn bt24_063_inherited_collision_reaches_carrier() {
    let mut runner = locomon_runner()
        .add_card(opp_digimon("CARRIER"))
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(carrier, Keyword::Collision),
        "a carrier with Locomon as a digivolution source must inherit <Collision>"
    );
}
