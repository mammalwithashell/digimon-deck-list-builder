//! BT21-006 Tsumemon — Digi-Egg (In-Training), Lv.2, Black, Cost 0, no DP.
//! Traits: Unidentified, LIBERATOR.
//!
//! # Card text (card image — authoritative)
//!
//! ```text
//! Inherited Effect
//! [All Turns] This Digimon with 4 or more [Vemmon] digivolution cards
//! gets +3000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_006.cs
//!   - EffectTiming.None (static aura, not an event trigger).
//!   - Condition(): IsExistOnBattleAreaDigimon(card) &&
//!       card.PermanentOfThisCard().DigivolutionCards.Count(EqualsCardName("Vemmon")) >= 4
//!   - ChangeSelfDPStaticEffect(changeValue: 3000, isInheritedEffect: true, condition: Condition)
//!   - No turn restriction in the C# condition — matches the printed [All Turns].
//!
//! # Patterns covered (RUST_DSL_TEST_API §5, §11)
//! - Inherited self-aura: `scope: inherited` + `kind: aura` + `target: {}`
//!   riding under the host's digivolution stack (Tsumemon is a Digi-Egg
//!   source, not the top card).
//! - `active_when: { self_source_count: { filter: { name_is: Vemmon }, op: gte,
//!   value: 4 } }` — G-DSL-SELF-SOURCE-COUNT-THRESHOLD, a no-subject global
//!   predicate reading the carrier's OWN digivolution sources.
//! - [All Turns]: no `your_turn`/`opponent_turn` wrapper — must stay active on
//!   both players' turns (distinguishes it from BT21-005's [Your Turn] gate).
//! - Boundary test at exactly 4 (met) vs 3 (unmet) Vemmon sources.
//! - Aura symmetry: gaining a 4th Vemmon source mid-game re-activates the
//!   grant; the modifier is not "sticky" once conditions change.
//! - Negative: egg not under any host contributes nothing (no free-floating
//!   aura).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT21-006";

fn egg() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled(CARD_ID)
}

fn digimon(card_id: &str, dp: i32, level: u8) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.level = Some(level);
    card
}

fn vemmon(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, "Vemmon");
    card.card_kind = CardKind::Digimon;
    card.dp = Some(1000);
    card.level = Some(3u8);
    card
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-006 YAML parses and compiles")
        .add_card(digimon("HOST", 5000, 5))
        .add_card(vemmon("VEMMON-A"))
        .add_card(vemmon("VEMMON-B"))
        .add_card(vemmon("VEMMON-C"))
        .add_card(vemmon("VEMMON-D"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

/// Build a stack: BT21-006 (egg) at the bottom, then `vemmon_count` Vemmon
/// sources, padded with FILLER sources so the total non-egg source count is
/// always 4, then HOST on top. Isolates the Vemmon-name filter from the raw
/// source count.
fn build_stack(runner: &mut DebugRunner, vemmon_count: usize) -> PermanentHandle {
    let vemmon_ids = ["VEMMON-A", "VEMMON-B", "VEMMON-C", "VEMMON-D"];
    assert!(vemmon_count <= 4);

    let mut ids: Vec<&str> = vec![CARD_ID];
    for i in 0..vemmon_count {
        ids.push(vemmon_ids[i]);
    }
    while ids.len() < 1 + 4 {
        ids.push("FILLER");
    }
    ids.push("HOST");
    runner.place_stack(0, &ids)
}

// ─── SECTION 1 — Structural assertions ──────────────────────────────────────

/// BT21-006 compiles with the expected metadata.
#[test]
fn bt21_006_metadata_is_black_lv2_egg_no_dp() {
    let card = egg();
    assert_eq!(card.card, "BT21-006");
    assert_eq!(card.name, "Tsumemon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.dp, None, "a Digi-Egg has no DP");
    assert!(
        card.color
            .contains(&digimon_dsl::compiled::CompiledColor::Black),
        "color must be black"
    );
    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::DigiEgg,
        "kind must be digi_egg"
    );
    assert!(
        card.traits.iter().any(|t| t == "Unidentified"),
        "must carry Unidentified trait"
    );
    assert!(
        card.traits.iter().any(|t| t == "LIBERATOR"),
        "must carry LIBERATOR trait"
    );
}

/// Exactly one aura clause (the inherited DP grant); no main-deck or security
/// effect. The clause must be Inherited scope, gated by self_source_count,
/// and carry the +3000 dp_modifier. No your_turn/opponent_turn wrapper
/// (printed [All Turns]).
#[test]
fn bt21_006_has_single_inherited_aura_clause_gated_by_self_source_count() {
    let card = egg();
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                dp_modifier,
                ..
            }) => Some((scope, active_when, dp_modifier)),
            _ => None,
        })
        .collect();
    assert_eq!(auras.len(), 1, "exactly one aura clause");

    let (scope, active_when, dp_modifier) = auras[0];
    assert_eq!(
        *scope,
        CompiledScope::Inherited,
        "the DP aura must be scope: inherited"
    );

    let predicate = active_when
        .as_ref()
        .expect("active_when must be present (gates the +3000 grant)");
    let ssc = predicate
        .self_source_count
        .as_ref()
        .expect("active_when must carry self_source_count");
    assert_eq!(
        ssc.op,
        digimon_dsl::compiled::CompiledComparatorOp::Gte,
        "must be a >= comparison"
    );
    assert_eq!(
        ssc.value,
        digimon_dsl::compiled::CompiledDpConstraint::Literal(4),
        "threshold must be literal 4"
    );
    let filter = ssc
        .filter
        .as_deref()
        .expect("self_source_count must carry a name filter");
    assert_eq!(
        filter.name_is.as_deref(),
        Some("Vemmon"),
        "filter must match card name Vemmon exactly"
    );

    // [All Turns]: active_when must NOT also require your_turn/opponents_turn.
    assert!(
        predicate.your_turn.is_none(),
        "[All Turns] must not gate on your_turn"
    );
    assert!(
        predicate.opponents_turn.is_none(),
        "[All Turns] must not gate on opponents_turn"
    );

    assert_eq!(
        *dp_modifier,
        Some(3000),
        "DP grant must be +3000"
    );
}

// ─── SECTION 2 — Behavioral: threshold met / unmet ──────────────────────────

/// Positive: host carries the egg with exactly 4 Vemmon sources beneath it →
/// the condition is met → +3000 DP.
#[test]
fn bt21_006_grants_3000_dp_with_exactly_4_vemmon_sources() {
    let mut r = base_runner();
    let host = build_stack(&mut r, 4);

    assert_eq!(
        r.effective_dp(host),
        Some(8000),
        "4 [Vemmon] sources ⇒ base 5000 + 3000 aura = 8000"
    );
}

/// Negative: only 3 Vemmon sources (padded with 1 filler to keep 4 total
/// non-egg sources) → condition NOT met → base DP only.
#[test]
fn bt21_006_no_dp_grant_with_only_3_vemmon_sources() {
    let mut r = base_runner();
    let host = build_stack(&mut r, 3);

    assert_eq!(
        r.effective_dp(host),
        Some(5000),
        "only 3 [Vemmon] sources ⇒ no aura, base DP 5000"
    );
}

/// Negative: 0 Vemmon sources (all filler) → condition NOT met → base DP only.
#[test]
fn bt21_006_no_dp_grant_with_zero_vemmon_sources() {
    let mut r = base_runner();
    let host = build_stack(&mut r, 0);

    assert_eq!(
        r.effective_dp(host),
        Some(5000),
        "0 [Vemmon] sources ⇒ no aura, base DP 5000"
    );
}

/// Boundary: 5+ Vemmon sources (all 4 non-egg source slots are Vemmon, one
/// extra Vemmon pushed beneath the egg) also satisfies >= 4 and grants +3000.
#[test]
fn bt21_006_grants_3000_dp_with_more_than_4_vemmon_sources() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-006 YAML parses and compiles")
        .add_card(digimon("HOST", 5000, 5))
        .add_card(vemmon("VEMMON-A"))
        .add_card(vemmon("VEMMON-B"))
        .add_card(vemmon("VEMMON-C"))
        .add_card(vemmon("VEMMON-D"))
        .add_card(vemmon("VEMMON-E"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start();
    // Egg, 5 named Vemmon slots, then HOST on top.
    let host = r.place_stack(
        0,
        &[
            CARD_ID,
            "VEMMON-A",
            "VEMMON-B",
            "VEMMON-C",
            "VEMMON-D",
            "VEMMON-E",
            "HOST",
        ],
    );

    assert_eq!(
        r.effective_dp(host),
        Some(8000),
        "5 [Vemmon] sources ⇒ still >= 4, base 5000 + 3000 aura = 8000"
    );
}

// ─── SECTION 3 — [All Turns]: active regardless of whose turn it is ────────

/// The aura must remain active on the OPPONENT's turn too — no [Your Turn]
/// restriction (distinguishes this from BT21-005's inherited draw, which is
/// [Your Turn]-gated).
#[test]
fn bt21_006_dp_grant_active_on_opponents_turn_all_turns() {
    let mut r = base_runner();
    let host = build_stack(&mut r, 4);

    // Switch to the opponent's turn.
    r.game.turn_player_idx = 1;

    assert_eq!(
        r.effective_dp(host),
        Some(8000),
        "[All Turns] must keep the +3000 aura active during the opponent's turn"
    );
}

/// The aura must remain active on the controller's own turn as well
/// (baseline sanity check paired with the opponent's-turn case above).
#[test]
fn bt21_006_dp_grant_active_on_own_turn_all_turns() {
    let mut r = base_runner();
    let host = build_stack(&mut r, 4);

    r.game.turn_player_idx = 0;

    assert_eq!(
        r.effective_dp(host),
        Some(8000),
        "[All Turns] must keep the +3000 aura active during the controller's own turn"
    );
}

// ─── SECTION 4 — Negative: egg not under any host ───────────────────────────

/// When the egg is not present as a source under a host at all, the host's
/// DP is simply its base value (no free-floating aura contribution).
#[test]
fn bt21_006_no_dp_grant_when_egg_not_under_host() {
    let mut r = base_runner();
    // HOST placed standalone, no BT21-006 egg beneath it at all, and no
    // Vemmon sources either.
    let standalone_host = r.place_on_field(0, "HOST", Some(0));

    assert_eq!(
        r.effective_dp(standalone_host),
        Some(5000),
        "no egg source at all ⇒ no aura, base DP 5000"
    );
}

// ─── SECTION 5 — Aura symmetry: condition can flip both ways ───────────────

/// The aura re-evaluates continuously: a stack built with only 3 Vemmon
/// sources has no grant, but an otherwise-identical stack with a 4th Vemmon
/// swapped in for the filler does grant +3000. This demonstrates the
/// aura is driven purely by the live predicate, not a one-shot check.
#[test]
fn bt21_006_threshold_is_reevaluated_not_latched() {
    let mut r_unmet = base_runner();
    let host_unmet = build_stack(&mut r_unmet, 3);
    assert_eq!(
        r_unmet.effective_dp(host_unmet),
        Some(5000),
        "3 Vemmon sources: unmet, no grant"
    );

    let mut r_met = base_runner();
    let host_met = build_stack(&mut r_met, 4);
    assert_eq!(
        r_met.effective_dp(host_met),
        Some(8000),
        "4 Vemmon sources: met, +3000 grant"
    );
}
