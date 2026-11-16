//! AD1-019 Matt Ishida & T.K. Takaishi — Tamer, Blue+Yellow, Cost 3.
//!
//! # Card text
//!
//! Traits ADVENTURE/Hero (official bundle `data/card_bundles/AD1-019.md`;
//! the `data/cards.json` row agrees). Card details sourced per CLAUDE.md
//! "Source priority" from the official cardlist (world.digimoncard.com AD-01)
//! and the DCGO behavioral reference
//! `DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_019.cs`:
//!
//!   [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//!   [Your Turn] When any of your Digimon digivolve into an [ADVENTURE] trait
//!     Digimon, by suspending this Tamer, you may play 1 [ADVENTURE] trait
//!     card from your hand. For every 2 of your Tamers' colors, reduce this
//!     effect's play cost by 1.
//!
//! Inherited (Security):
//!   Security Effect [Security] Play this card without paying the cost.
//!
//! # Implemented slice
//! - Clause 1 — [Start of Your Main Phase] gain 1 memory if the opponent has
//!   a Digimon (positive + negative gate covered behaviorally).
//! - Clause 3 — [Security] play this card without paying the cost
//!   (structural coverage; the security-replay primitive itself is exercised
//!   end-to-end in BT22-084 / BT22-094 / EX11-054 companion tests).
//!
//! - Clause 2 — [Your Turn] suspend-this-Tamer → play 1 [ADVENTURE] card
//!   from hand with the play cost reduced by floor(distinct Tamer colors / 2)
//!   (G-FORMULA-COST-DELTA resolved 2026-05-20). `play_from_hand`'s
//!   `cost_delta` now accepts a `ReduceFn { reduce_fn: FormulaSpec }` variant;
//!   the reduction formula is
//!   `floor_div(distinct_colors_count(your Tamers), 2)`. The `on_digivolve`
//!   ADVENTURE-result observer, `active_when: { your_turn: true }`,
//!   `activation_cost: { suspend_self: true }`, and the `select_hand`
//!   ADVENTURE filter were already available. Covered behaviorally by
//!   `ad1_019_clause2_suspend_play_adventure` (positive) and
//!   `ad1_019_clause2_does_not_fire_for_non_adventure_digivolve` (negative).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "AD1-019";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn ad1_019_card_data() -> digimon_engine::card_data::CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_filler(card_id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(card_id, card_id)
}

/// A generic Lv.3 Digimon — used to give the opponent a board presence so
/// clause 1's "if your opponent has a Digimon" gate is satisfied.
fn make_digimon(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Smoke: the embedded DSL pack ships AD1-019 and it compiles as a Tamer.
#[test]
fn ad1_019_compiles_as_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

/// Card-level metadata matches the printed card: Tamer, cost 3, Blue+Yellow,
/// no level / no DP.
#[test]
fn ad1_019_card_metadata_matches_print() {
    let card = compiled(CARD_ID);
    assert_eq!(card.cost, Some(3), "AD1-019 costs 3 to play");
    assert_eq!(card.level, None, "Tamers have no level");
    assert_eq!(card.dp, None, "Tamers have no DP");

    let colors: Vec<_> = card.color.iter().copied().collect();
    assert_eq!(
        colors.len(),
        2,
        "AD1-019 is a two-color (Blue+Yellow) Tamer"
    );
    assert!(
        colors.contains(&CompiledColor::Blue),
        "AD1-019 must be Blue"
    );
    assert!(
        colors.contains(&CompiledColor::Yellow),
        "AD1-019 must be Yellow"
    );
}

/// `card_data_from_compiled` round-trips the Tamer kind / cost faithfully —
/// the shape DebugRunner consumes.
#[test]
fn ad1_019_card_data_round_trips() {
    let data = ad1_019_card_data();
    assert_eq!(data.card_id, CARD_ID);
    assert_eq!(data.card_kind, CardKind::Tamer);
    assert_eq!(data.play_cost, 3);
}

/// Three triggered clauses are authored: clause 1 ([Start of Your Main
/// Phase]), clause 2 ([Your Turn] on-digivolve suspend-play), and clause 3
/// ([Security]).
#[test]
fn ad1_019_has_start_main_and_security_clauses() {
    let card = compiled(CARD_ID);

    for timing in [
        CompiledTiming::StartOfYourMainPhase,
        CompiledTiming::OnDigivolve,
        CompiledTiming::OnSecurity,
    ] {
        assert!(
            card.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Triggered(t) if t.when.contains(&timing)
            )),
            "AD1-019 must have a {timing:?} clause"
        );
    }

    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 3,
        "AD1-019 authors exactly 3 triggered clauses (start-main + on-digivolve + security)"
    );
}

/// Clause 1 is not player-optional and not [Once Per Turn] (printed text has
/// no "may" and no [Once Per Turn] marker).
#[test]
fn ad1_019_clause1_start_main_not_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("AD1-019 must have a start_of_your_main_phase clause");

    assert!(
        !clause.optional,
        "clause 1 (gain 1 memory) is not player-optional"
    );
    assert!(
        !clause.once_per_turn,
        "clause 1 has no [Once Per Turn] in printed text"
    );
}

/// Clause 3 ([Security]) is not player-optional — "[Security] Play this card
/// without paying the cost" has no "may"; DCGO uses
/// `PlaySelfTamerSecurityEffect` with no opt-out parameter.
#[test]
fn ad1_019_clause3_on_security_not_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("AD1-019 must have an on_security clause");

    assert!(
        !clause.optional,
        "the [Security] clause is not player-optional"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1: [Start of Your Main Phase] gain 1 memory if the
//                       opponent has a Digimon.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: opponent has a Digimon at the start of P0's main phase → P0
/// gains 1 memory.
#[test]
fn ad1_019_clause1_gains_memory_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(ad1_019_card_data())
        .add_card(make_digimon("OPP-DIGIMON", "Patamon"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "P0 must gain 1 memory when the opponent has a Digimon at start of main phase"
    );
}

/// Negative: opponent has NO Digimon → clause 1's condition fails → memory
/// unchanged.
#[test]
fn ad1_019_clause1_no_gain_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(ad1_019_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "memory must not change when the opponent controls no Digimon"
    );
}

/// Negative (ownership): an opponent Tamer (not a Digimon) does NOT satisfy
/// the gate — the printed text reads "if your opponent has a Digimon".
#[test]
fn ad1_019_clause1_no_gain_when_opponent_only_has_a_tamer() {
    let mut opp_tamer = make_test_card("OPP-TAMER", "Opp Tamer");
    opp_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(ad1_019_card_data())
        .add_card(opp_tamer)
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "an opponent Tamer must not satisfy the \"has a Digimon\" gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 3: [Security] play this card without paying the cost.
//
// Structural-only here: the security-replay primitive (`play_from_security`)
// is exercised end-to-end by the companion tests for sibling Tamers
// (BT22-084 / BT22-094 / EX11-054) that use the same step.
// ═══════════════════════════════════════════════════════════════════════════════

/// The on_security clause exists and lowers to a `play_from_security` body.
#[test]
fn ad1_019_clause3_on_security_structural_present() {
    let card = compiled(CARD_ID);
    let on_sec = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity]);
    assert!(
        on_sec.is_some(),
        "AD1-019 must compile an on_security clause for the [Security] play-self effect"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 2: [Your Turn] suspend-this-Tamer → play 1 [ADVENTURE]
//             card from hand with the play cost reduced by
//             floor(distinct Tamer colors / 2).
//
// G-FORMULA-COST-DELTA resolved 2026-05-20: the `CostDelta` enum now admits
// a `ReduceFn { reduce_fn: FormulaSpec }` variant, so `play_from_hand`'s
// `cost_delta` can carry the dynamic reduction
//   floor_div(distinct_colors_count{ of: you, zone: battle_area,
//                                    filter: { kind: tamer } }, 2).
// AD1-019 is a Blue+Yellow Tamer → 2 distinct colors → floor(2/2) = 1, so
// the ADVENTURE card's printed play cost is reduced by 1.
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: a Lv.3 Digimon usable as a digivolve base on the battle area.
fn make_lv3_base(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// Helper: a Lv.4 ADVENTURE-trait Digimon to digivolve into — the digivolve
/// result whose ADVENTURE trait satisfies clause 2's `event_target_trait_has`
/// gate. Carries a Lv.3 / cost-0 evo cost so `digivolve_from_hand` accepts it
/// onto `make_lv3_base`.
fn make_lv4_adventure(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.traits = vec!["ADVENTURE".to_string()];
    c.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0, // Red — matches make_test_card's default color
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// Helper: an ADVENTURE-trait Digimon to be the hand card played by clause 2.
fn make_adventure_hand_card(
    card_id: &str,
    name: &str,
    play_cost: u8,
) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = play_cost as u16;
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

/// Clause 2 end-to-end: P0 digivolves a Lv.3 base into a Lv.4 ADVENTURE
/// Digimon on their own turn. AD1-019's [Your Turn] clause triggers, suspends
/// AD1-019 as its activation cost, and plays a chosen ADVENTURE card from hand
/// with the play cost reduced by floor(distinct Tamer colors / 2). With only
/// AD1-019 (Blue+Yellow) as the controller's Tamer, the reduction is
/// floor(2/2) = 1.
#[test]
fn ad1_019_clause2_suspend_play_adventure() {
    let mut runner = DebugRunner::builder()
        .add_card(ad1_019_card_data())
        .add_card(make_lv3_base("BASE", "Base"))
        .add_card(make_lv4_adventure("EVO", "Adventure Evo"))
        .add_card(make_adventure_hand_card("ADV-HAND", "Adventure Hand", 6))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &["EVO", "ADV-HAND"])
        .memory(10)
        .start();

    // AD1-019 (Tamer) and a Lv.3 base on P0's field; AD1-019 starts unsuspended.
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "AD1-019 must start unsuspended"
    );

    let battle_before = runner.battle_area_size(0);

    // Digivolve EVO (the ADVENTURE Lv.4) from hand onto BASE — this is the
    // "your Digimon digivolves into an [ADVENTURE] trait Digimon" trigger.
    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVO")
        .expect("EVO in P0 hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        evo_idx,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve into the ADVENTURE Digimon must succeed");

    let mem_after_digivolve = runner.memory();
    let _ = runner.auto_resolve();

    // AD1-019 must have been suspended by the activation_cost step.
    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "AD1-019 must be suspended (the 'by suspending this Tamer' activation cost)"
    );

    // The ADVENTURE hand card must have been played onto P0's field.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-HAND"),
        "the chosen [ADVENTURE] card must have been played from hand"
    );

    // Cost: ADV-HAND printed cost 6, reduced by floor(distinct Tamer colors / 2)
    // = floor(2/2) = 1 → 5 memory spent on the play.
    let spent = mem_after_digivolve - runner.memory();
    assert_eq!(
        spent,
        5,
        "ADVENTURE card (printed cost 6) must be played at cost 6 - floor(2/2) = 5; \
         memory went {mem_after_digivolve} -> {} (spent {spent})",
        runner.memory()
    );

    // Battle area grew by exactly 1 net (BASE digivolved in place — no count
    // change — plus the ADV-HAND play).
    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "P0 battle area must grow by exactly 1 (the played ADVENTURE card)"
    );
}

/// Negative: digivolving into a NON-ADVENTURE Digimon must NOT trigger clause
/// 2 — `event_target_trait_has: ADVENTURE` rejects, AD1-019 stays unsuspended,
/// and no card is played from hand.
#[test]
fn ad1_019_clause2_does_not_fire_for_non_adventure_digivolve() {
    let mut plain_evo = make_lv4_adventure("PLAIN-EVO", "Plain Evo");
    plain_evo.traits = vec![]; // strip ADVENTURE — must not satisfy the gate

    let mut runner = DebugRunner::builder()
        .add_card(ad1_019_card_data())
        .add_card(make_lv3_base("BASE", "Base"))
        .add_card(plain_evo)
        .add_card(make_adventure_hand_card("ADV-HAND", "Adventure Hand", 6))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &["PLAIN-EVO", "ADV-HAND"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    let hand_before = runner.hand_size(0);

    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "PLAIN-EVO")
        .expect("PLAIN-EVO in P0 hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        evo_idx,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve into the non-ADVENTURE Digimon must succeed");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "AD1-019 must NOT be suspended — clause 2 must not fire for a non-ADVENTURE digivolve"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ADV-HAND"),
        "the ADVENTURE hand card must remain in hand (clause 2 did not fire)"
    );
    // Hand net-unchanged: PLAIN-EVO leaves as the digivolve material (-1) and
    // `digivolve_from_hand` draws 1 (+1); ADV-HAND is untouched. Clause 2
    // playing nothing means no extra hand-card movement.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand net-unchanged (digivolve material -1, digivolve draw +1); clause 2 played nothing"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause 2 hand-pick legality: only cards that can be played AND
//             paid for at the reduced cost
//
// DCGO AD1_019.cs:52-57 IsAdventureDigimon = `HasPlayCost &&
// EqualsTraits("ADVENTURE") && CanPlayAsNewPermanent(cardSource, payCost:
// true, activateClass, fixedCost: Math.Max(0, GetCostItself - reduction))`,
// with `reduction = GetDifferenetColorCardCount(tamerCards) / 2` (:68-79): no
// Digi-Egg (no play cost, CR 4.3 7-1-1 — Koromon ST20-01), no Option (it is
// used, never played — Our Courage United ST20-14), a free battle-area slot,
// no effect-play lock, and the reduced cost payable. CR 4.3 1-3-11-1: a card
// can be declared for play only if it is able to be played, and not if its
// cost can't be paid.
// ═══════════════════════════════════════════════════════════════════════════════

/// AD1-019 + a Lv.3 base on P0's field, EVO + `hand` in hand, 10 memory.
/// `prepare` runs before the digivolve. Digivolves EVO (an [ADVENTURE]
/// Digimon) onto the base, accepts the optional trigger, and returns the
/// non-PASS hand-pick candidates (sorted card ids; empty when no pick opens).
fn adventure_offered(
    hand: &[&str],
    prepare: impl FnOnce(&mut DebugRunner),
) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let mut full_hand = vec!["EVO"];
    full_hand.extend_from_slice(hand);
    let mut runner = DebugRunner::builder()
        .add_card(ad1_019_card_data())
        .add_card(make_lv3_base("BASE", "Base"))
        .add_card(make_lv4_adventure("EVO", "Adventure Evo"))
        .add_card(make_adventure_hand_card("ADV-HAND", "Adventure Hand", 6))
        .add_card(make_adventure_hand_card("ADV-21", "ADV-21", 21))
        .add_card(make_adventure_hand_card("ADV-22", "ADV-22", 22))
        .add_card(card_data_from_compiled("ST20-01"))
        .add_card(card_data_from_compiled("ST20-14"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"; 5])
        .deck(1, &["FILLER-DECK"; 5])
        .hand(0, &full_hand)
        .memory(10)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    prepare(&mut runner);
    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVO")
        .expect("EVO in P0 hand");
    assert!(runner.game.digivolve_from_hand(
        0,
        evo_idx,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    ));
    runner.game.drain_effect_queue();
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[0]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut out: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&id| id != PASS)
                .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
                .collect();
            out.sort();
            return out;
        }
        runner
            .accept_optional_trigger()
            .expect("accept the optional [Your Turn] trigger");
    }
    panic!("the [ADVENTURE] hand pick never opened");
}

/// Koromon (ST20-01) is an [ADVENTURE] Digi-Egg with no play cost: not offered.
#[test]
fn ad1_019_clause2_hand_pick_excludes_adventure_digi_egg() {
    assert_eq!(
        adventure_offered(&["ST20-01", "ADV-HAND"], |_| {}),
        vec!["ADV-HAND".to_string()]
    );
}

/// Our Courage United (ST20-14) is an [ADVENTURE] Option: it is used, never
/// played, so a "play" effect can't choose it.
#[test]
fn ad1_019_clause2_hand_pick_excludes_adventure_option() {
    assert_eq!(
        adventure_offered(&["ST20-14", "ADV-HAND"], |_| {}),
        vec!["ADV-HAND".to_string()]
    );
}

/// A full battle area: the [ADVENTURE] Digimon can't be placed.
#[test]
fn ad1_019_clause2_hand_pick_offers_nothing_when_battle_area_full() {
    let offered = adventure_offered(&["ADV-HAND"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "no free slot → nothing can be chosen (got {offered:?})");
}

/// "You can't play Digimon by effects": nothing is offered.
#[test]
fn ad1_019_clause2_hand_pick_offers_nothing_under_digimon_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = adventure_offered(&["ADV-HAND"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(offered.is_empty(), "effect plays of Digimon are locked (got {offered:?})");
}

/// Affordability at the REDUCED cost: AD1-019 alone is 2 Tamer colours → -1.
/// With 10 memory, ADV-21 costs 20 (memory ends at the opponent's 10, legal);
/// ADV-22 costs 21 (one past the gauge) and can't be paid, so it isn't offered.
#[test]
fn ad1_019_clause2_hand_pick_offers_only_cards_payable_at_the_reduced_cost() {
    assert_eq!(
        adventure_offered(&["ADV-21", "ADV-22"], |_| {}),
        vec!["ADV-21".to_string()]
    );
}
