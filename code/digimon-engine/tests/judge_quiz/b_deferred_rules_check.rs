//! Cluster B — rules-check deferred until the ongoing effect fully resolves.
//!
//! Questions (see `card-resolution.md`):
//!   Q6  Pillomon (BT9-033) at 0 DP not deleted until Flame Hellscythe (BT8-109)
//!       resolves — judge: NO (can't play a Digimon yet).
//!   Q7  Eye of the Gorgon (BT9-108) deletes Pillomon (BT9-033) then plays a Lv3
//!       — judge: YES (sequential sub-effects).
//!   Q8  Burst-Digivolve stack (BT13-020/AD1-016/BT21-044/BT21-042/EX4-005/
//!       BT21-004); Comet Hammer (BT23-096) de-digivolves to Agumon — judge:
//!       Agumon trashed → Koromon trashed (DP-less can't remain).
//!   Q13 Nyabootmon (BT22-042)+ShoeShoemon (P-165) vs Rapidmon (X Antibody)
//!       (BT16-101) — judge: −6000 DP.
//!   Q14 Same vs ShineGreymon: Ruin Mode (EX4-074) — judge: −6000 DP.
//!   Q24 Hudiemon (BT23-101)+Tentomon (BT23-037)+Kokomon (EX6-004) vs Rapidmon
//!       (X Antibody) (BT16-101) — judge: 3000 DP (Tentomon deleted by rules
//!       check before Kokomon's trigger).
//!
//! Scenarios authored under tasks §4.
//!
//! All six are BLOCKED-CARD: each needs ≥1 unimplemented card (see
//! card-resolution.md §"Implementation status"). Stubs `#[ignore]`-d on the
//! specific missing card(s); promote once authored (cluster-B authoring, §4).

#![allow(unused_imports)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, Expiry};

fn lv4_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

// ─────────────────────────────────────────────────────────────────────────────
// Cluster-B CORE RULE PROBE — ≤0-DP deletion via state-based rules-check
// ─────────────────────────────────────────────────────────────────────────────
//
// Every cluster-B question (Q6, Q8, Q13, Q14, Q24) turns on the same rule: a
// Digimon driven to 0 DP or below is deleted by a GAME rules-check that runs
// AFTER the ongoing effect resolves — not mid-effect, but it must eventually
// run. This probe tests the rule with a synthetic Digimon (no quiz card needed),
// so it is NOT blocked on card authoring.
//
// Faithful behavior: after an effect reduces a battle-area Digimon to ≤0 DP and
// that effect's resolution completes (the effect queue drains), a rules-check
// deletes the Digimon.

/// Sanity: a non-reduced Digimon is NOT deleted by the post-effect drain.
#[test]
fn zero_dp_probe_healthy_digimon_survives_drain() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .memory(10)
        .start();
    let _ = r.place_on_field(0, "VICTIM", Some(0));
    r.game.drain_effect_queue();
    assert_eq!(r.battle_area_size(0), 1, "healthy Digimon must remain");
}

/// CORE RULE — a Digimon reduced to ≤0 DP by an effect must be deleted once the
/// effect resolves. Probes whether the engine has a general state-based
/// rules-check (the only ≤0-DP deletion site found is `run_rule_check_after_arts`,
/// invoked solely from the Arts-digivolve flow — game_actions.rs:1607).
///
/// CONFIRMED FAILING (2026-05-29): VICTIM at -1000 DP survives the post-effect
/// drain (battle_area_size 1, expected 0). Logged G-NO-GENERAL-ZERO-DP-RULES-CHECK
/// in qa/archetype-qa/engine-gaps.md. Root mechanic behind cluster B (Q6, Q8, Q13,
/// Q14, Q24). Un-ignore when a general state-based ≤0-DP check is wired.
#[test]
#[ignore = "DISCOVERED BUG (proven failing 2026-05-29): no general state-based ≤0-DP rules-check — a Digimon reduced to ≤0 DP by a non-Arts effect is not deleted. Only run_rule_check_after_arts (Arts-only, game_actions.rs:1607) exists. Logged G-NO-GENERAL-ZERO-DP-RULES-CHECK."]
fn zero_dp_probe_reduced_digimon_deleted_after_effect_resolves() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .add_card(make_test_card("SRC", "Src"))
        .memory(10)
        .start();
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    let src = r.place_on_field(1, "SRC", None);
    let src_card = r.game.player(1).battle_area[0].top_card().handle();

    // Simulate an opponent effect reducing VICTIM to -1000 DP, then resolving.
    {
        let mut ctx = EffectContext::new(&mut r.game, src_card, Some(src), 1);
        ctx.add_dp_modifier(victim, -4000, Expiry::Permanent);
    }
    assert_eq!(
        r.game.effective_dp(victim),
        Some(-1000),
        "precondition: VICTIM is at -1000 effective DP"
    );

    // The effect has resolved — drain the queue (the post-effect boundary).
    r.game.drain_effect_queue();

    // Judge-correct (cluster B): the ≤0-DP Digimon is deleted by the rules check.
    assert_eq!(
        r.battle_area_size(0),
        0,
        "a Digimon at ≤0 DP must be deleted by a state-based rules-check after \
         the effect resolves (see G-NO-GENERAL-ZERO-DP-RULES-CHECK)"
    );
}

/// Q6 — Pillomon (BT9-033) at 0 DP not deleted until Flame Hellscythe (BT8-109)
/// resolves. Judge: NO (can't play a Digimon yet).
#[test]
#[ignore = "BLOCKED-CARD: needs BT8-109 (Flame Hellscythe). BT9-033 implemented."]
fn q6_pillomon_zero_dp_not_deleted_until_flame_hellscythe_resolves() {}

/// Q7 — Eye of the Gorgon (BT9-108) deletes Pillomon (BT9-033) with sub-effect 1,
/// then plays a Lv3 with sub-effect 2. Judge: YES.
#[test]
#[ignore = "BLOCKED-CARD: needs BT9-108 (Eye of the Gorgon). BT9-033 implemented."]
fn q7_eye_of_the_gorgon_sequential_delete_then_play() {}

/// Q8 — Burst-Digivolve stack; Comet Hammer (BT23-096) de-digivolves to Agumon
/// (EX4-005); at EoT Burst trashes the top, DP-less Koromon (BT21-004) can't
/// remain. Judge: Agumon trashed → Koromon trashed.
#[test]
#[ignore = "BLOCKED-CARD: needs BT13-020, AD1-016, BT21-044, BT21-042, EX4-005, BT21-004. BT23-096 implemented."]
fn q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot() {}

/// Q13 — Nyabootmon (BT22-042)+ShoeShoemon (P-165); Rapidmon (X Antibody)
/// (BT16-101) measured before ShoeShoemon's On Play. Judge: −6000 DP.
#[test]
#[ignore = "BLOCKED-CARD: needs BT16-101 (Rapidmon X), ST17-07 (Rapidmon). BT22-042, P-165 implemented."]
fn q13_nyabootmon_dp_minus_measured_before_shoeshoemon_on_play() {}

/// Q14 — Same vs ShineGreymon: Ruin Mode (EX4-074); ShoeShoemon enters and gets
/// −5000 but isn't deleted until Nyabootmon resolves. Judge: −6000 DP.
#[test]
#[ignore = "BLOCKED-CARD: needs BT16-101 (Rapidmon X). BT22-042, P-165, EX4-074 implemented."]
fn q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode() {}

/// Q24 — Hudiemon (BT23-101) <Alliance> Tentomon (BT23-037); Tentomon suspended →
/// −4000 from Rapidmon X (BT16-101) → deleted by rules check before Kokomon
/// (EX6-004) [Your Turn] contributes. Judge: Hudiemon DP 3000.
#[test]
#[ignore = "BLOCKED-CARD: needs BT23-101 (Hudiemon), BT23-037 (Tentomon), EX6-004 (Kokomon), BT16-101 (Rapidmon X), ST17-07 (Rapidmon)."]
fn q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger() {}
