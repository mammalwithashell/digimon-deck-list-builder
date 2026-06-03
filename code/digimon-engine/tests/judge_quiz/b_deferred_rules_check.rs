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

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, Expiry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

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
/// effect resolves. Pins the general state-based rules-check
/// (`Game::run_state_based_rules_check`), invoked at the outermost
/// `drain_effect_queue` boundary (was Arts-only via `run_rule_check_after_arts`).
///
/// RESOLVED (2026-05-29): the general state-based ≤0-DP rules-check
/// (`Game::run_state_based_rules_check`) now runs at the outermost
/// `drain_effect_queue` boundary, so VICTIM at -1000 DP is deleted once the
/// effect resolves. Was G-NO-GENERAL-ZERO-DP-RULES-CHECK (cluster B root).
#[test]
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

// ─────────────────────────────────────────────────────────────────────────────
// Synthetic rule-check TIMING regression tests (tasks §2.6)
// ─────────────────────────────────────────────────────────────────────────────

/// Applies `value` DP to a fixed target when its (synthetic) WhenAttacking
/// timing fires — stages a single queued effect that drives a Digimon's DP.
struct DpModWhenAttacking {
    target: PermanentHandle,
    value: i32,
}
impl CardEffect for DpModWhenAttacking {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        let value = self.value;
        vec![Effect::when_attacking(card)
            .name("synthetic dp mod")
            .process(move |ctx| {
                ctx.add_dp_modifier(target, value, Expiry::Permanent);
            })
            .build()]
    }
}

/// Reduces the target to ≤0 DP then restores it ABOVE 0, both within ONE effect
/// body — pins "no mid-effect deletion".
struct ReduceThenRestoreWithinOneEffect {
    target: PermanentHandle,
}
impl CardEffect for ReduceThenRestoreWithinOneEffect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::when_attacking(card)
            .name("reduce then restore (one effect)")
            .process(move |ctx| {
                ctx.add_dp_modifier(target, -4000, Expiry::Permanent); // 3000 → -1000
                ctx.add_dp_modifier(target, 5000, Expiry::Permanent); //  -1000 → 4000
            })
            .build()]
    }
}

/// Q6-analog (§2.6a) — a Digimon driven to ≤0 DP MID-effect is NOT deleted until
/// the effect fully resolves. One effect reduces VICTIM to -1000 then restores it
/// to +4000; with the old inline mid-effect deletion VICTIM would be gone after
/// the reduce (the restore a no-op). Faithful: the rules-check defers to the
/// resolution boundary, so VICTIM survives at 4000 (Q6/Q13/Q14 root rule).
#[test]
fn q6_analog_no_mid_effect_deletion_within_single_effect() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .add_card(make_test_card("SRC", "Src"))
        .memory(10)
        .start();
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    let src = r.place_on_field(0, "SRC", None);
    r.register_effect(
        "SRC",
        Arc::new(ReduceThenRestoreWithinOneEffect { target: victim }),
    );
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.effective_dp(victim),
        Some(4000),
        "VICTIM restored within the same effect must survive (no mid-effect deletion)"
    );
    assert_eq!(r.battle_area_size(0), 2, "VICTIM + SRC both remain");
}

/// Q24-analog (§2.6b) — the rules-check fires BETWEEN top-level queued effects.
/// A turn-player effect drives VICTIM to ≤0; the rules-check deletes it BEFORE a
/// separate opponent "+DP save" trigger resolves. If the check only ran at
/// queue-empty, both would resolve (net -2000 → VICTIM at 1000) and VICTIM would
/// survive — so this distinguishes the between-effects timing (D3).
#[test]
fn q24_analog_rules_check_deletes_between_queued_effects() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .add_card(make_test_card("DAMAGER", "Damager"))
        .add_card(make_test_card("HEALER", "Healer"))
        .memory(10)
        .start();
    r.set_first_player(0); // player 0 is turn player → DAMAGER resolves first
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    let damager = r.place_on_field(0, "DAMAGER", None);
    let healer = r.place_on_field(1, "HEALER", None);
    r.register_effect(
        "DAMAGER",
        Arc::new(DpModWhenAttacking {
            target: victim,
            value: -4000,
        }),
    );
    r.register_effect(
        "HEALER",
        Arc::new(DpModWhenAttacking {
            target: victim,
            value: 2000,
        }),
    );
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(damager));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(healer));
    r.game.drain_effect_queue();
    assert_eq!(
        r.battle_area_size(1),
        1,
        "VICTIM deleted by the rules-check after the damager resolves, before the heal could save it"
    );
}

/// Q6 — Pillomon (BT9-033) at 0 DP not deleted until Flame Hellscythe (BT8-109)
/// resolves. Judge: NO (can't play a Digimon yet).
///
/// Board (card-resolution.md Q6): Player 1 controls Pillomon (BT9-033),
/// `[All Turns] Players can't play Digimon by effects` — a `target_player: any`
/// floodgate installing `CannotPlayDigimonByEffect` on BOTH players. Player 0
/// has a purple Lv.3 (DP ≤ 6000) in trash and plays Flame Hellscythe (BT8-109):
///   sub-effect 1 — 1 opponent Digimon gets -6000 DP for the turn (targets
///                  Pillomon: 2000 → -4000, i.e. ≤ 0 DP);
///   sub-effect 2 — "you may play 1 purple/yellow Digimon DP ≤ 6000 from trash
///                  without paying its memory cost."
///
/// The CONTRAST with Q7 (which is the whole point): in Q7, Eye of the Gorgon's
/// sub-effect 1 *deletes* Pillomon, clearing its floodgate before sub-effect 2,
/// so the trash-play succeeds. Here, sub-effect 1 only *reduces* Pillomon to ≤0
/// DP — and a ≤0-DP Digimon is NOT deleted mid-effect (deletion is deferred to
/// the state-based rules-check that runs AFTER Flame Hellscythe fully resolves).
/// So when sub-effect 2 runs, Pillomon is STILL alive and its
/// `CannotPlayDigimonByEffect` floodgate is STILL up → the trash-play is blocked.
/// Judge: NO — you can't play a Digimon yet.
///
/// What this pins (so it cannot pass for the wrong reason):
///   1. CONTROL: Pillomon's floodgate is genuinely installed on Player 0.
///   2. After sub-effect 1's -6000, Pillomon is at ≤0 DP but STILL on the field
///      (no mid-effect deletion) and the floodgate is STILL active.
///   3. Sub-effect 2's trash-play is BLOCKED — the purple Lv.3 never leaves trash.
///   4. Only AFTER Flame Hellscythe resolves does the rules-check delete Pillomon.
#[test]
fn q6_pillomon_zero_dp_not_deleted_until_flame_hellscythe_resolves() {
    use digimon_engine::action::space::{encode_attack, PASS};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::{CardColor, ModifierType};
    use digimon_engine::selection::SelectionKind;

    // A purple Lv.3 (DP ≤ 6000), no [On Play] — the sub-effect-2 trash candidate.
    let mut purple_l3 = make_test_card("PURPLE-L3", "Purple L3");
    purple_l3.card_kind = CardKind::Digimon;
    purple_l3.colors = vec![CardColor::Purple];
    purple_l3.level = Some(3);
    purple_l3.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT8-109")
        .expect("BT8-109 Flame Hellscythe loads")
        .dsl_card("BT9-033")
        .expect("BT9-033 Pillomon loads")
        .add_card(purple_l3)
        .hand(0, &["BT8-109"])
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 1's Pillomon (2000 DP) — installs the floodgate.
    let pillomon = r.place_on_field(1, "BT9-033", Some(0));
    // Player 0's purple Lv.3 seeded into trash.
    {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PURPLE-L3")
            .expect("PURPLE-L3 in card_data");
        let next_idx = r.game.next_card_index();
        r.game.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, next_idx));
    }

    r.game.tick_declarative_effects();
    // (1) CONTROL — the floodgate is genuinely installed on Player 0.
    assert!(
        r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "precondition: Pillomon's [All Turns] floodgate installs CannotPlayDigimonByEffect on Player 0"
    );

    // Play Flame Hellscythe; sub-effect 1 prompts for the -6000 target.
    r.play(0, 0).expect("play BT8-109 from hand");
    let view = r
        .pending_selection_view()
        .expect("[Main] sub-effect 1: -6000 target prompt installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.game.decode_action(encode_attack(0, pillomon.index as u16), 0);

    // (2) Pillomon is at ≤0 DP but NOT deleted mid-effect; floodgate still up.
    assert!(
        r.game.effective_dp(pillomon).unwrap_or(1) <= 0,
        "Pillomon reduced to ≤0 DP (2000 - 6000)"
    );
    assert_eq!(
        r.battle_area_size(1),
        1,
        "Pillomon at ≤0 DP is NOT deleted mid-effect (deletion deferred to the rules-check)"
    );
    assert!(
        r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "Pillomon (still alive at ≤0 DP) keeps its CannotPlayDigimonByEffect floodgate up"
    );

    // (3) Sub-effect 2: the trash-play. The floodgate blocks the play — drive
    // whatever prompt installs (select the candidate if offered, else PASS); the
    // purple Lv.3 must NOT enter the battle area either way.
    let battle_before = r.battle_area_size(0);
    if let Some(play_view) = r.pending_selection_view() {
        let pick = play_view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(PASS);
        r.game.decode_action(pick, play_view.selecting_player);
    }
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(0),
        battle_before,
        "Judge NO: the trash Digimon could NOT be played — Pillomon's floodgate \
         persists because Pillomon (at ≤0 DP) is not deleted until the effect resolves"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "PURPLE-L3"),
        "the purple Lv.3 stays in Player 0's trash (the blocked play left it there)"
    );

    // (4) Only after Flame Hellscythe resolves does the rules-check delete Pillomon.
    assert_eq!(
        r.battle_area_size(1),
        0,
        "Pillomon (≤0 DP) is deleted by the state-based rules-check after the effect resolves"
    );
}

/// Q7 — Eye of the Gorgon (BT9-108) deletes Pillomon (BT9-033) with sub-effect 1,
/// then plays a Lv3 with sub-effect 2. Judge: YES.
///
/// Board (card-resolution.md Q7): Player 1 controls Pillomon (BT9-033),
/// `[All Turns] Players can't play Digimon by effects` — a `target_player: any`
/// floodgate that installs `CannotPlayDigimonByEffect` on BOTH players. Player 0
/// has a purple Lv.3 Digimon in trash and plays Eye of the Gorgon (BT9-108):
///   sub-effect 1 — Delete 1 opponent unsuspended Digimon (Pillomon);
///   sub-effect 2 — "If you do, you may play 1 purple Lv.3 Digimon from your
///                   trash without paying its cost."
/// The two sub-effects resolve SEQUENTIALLY within the one [Main] effect. By the
/// time sub-effect 2 runs, sub-effect 1 has already deleted Pillomon, so its
/// `CannotPlayDigimonByEffect` floodgate is gone and the trash-play is no longer
/// blocked. Judge: YES — the Lv.3 play succeeds.
///
/// What this pins (so it cannot pass for the wrong reason):
///   1. CONTROL: while Pillomon is alive, `CannotPlayDigimonByEffect` is really
///      installed on Player 0 — a direct `play_from_trash_free` IS blocked. (If
///      the floodgate were never active, the test would false-pass.)
///   2. The real BT9-108 [Main] effect runs through the engine's action path
///      (`decode_action`, which ticks declarative state between selections), so
///      after sub-effect 1 deletes Pillomon the floodgate clears before
///      sub-effect 2's play resolves.
///   3. The purple Lv.3 leaves trash and enters Player 0's battle area — the
///      sequential play actually happened.
#[test]
fn q7_eye_of_the_gorgon_sequential_delete_then_play() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::card_source::CardSource;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::{CardColor, ModifierType};
    use digimon_engine::selection::SelectionKind;

    // A purple Lv.3 Digimon (no [On Play]) — the sub-effect-2 trash-play target.
    let mut purple_l3 = make_test_card("PURPLE-L3", "Purple L3");
    purple_l3.card_kind = CardKind::Digimon;
    purple_l3.colors = vec![CardColor::Purple];
    purple_l3.level = Some(3);
    purple_l3.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT9-108")
        .expect("BT9-108 Eye of the Gorgon loads")
        .dsl_card("BT9-033")
        .expect("BT9-033 Pillomon loads")
        .add_card(purple_l3)
        .hand(0, &["BT9-108"])
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 1's Pillomon — `[All Turns] Players can't play Digimon by effects`.
    let pillomon = r.place_on_field(1, "BT9-033", Some(0));
    // Player 0's purple Lv.3 seeded into trash (the sub-effect-2 candidate).
    {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PURPLE-L3")
            .expect("PURPLE-L3 in card_data");
        let next_idx = r.game.next_card_index();
        r.game.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, next_idx));
    }

    // Materialize Pillomon's floodgate.
    r.game.tick_declarative_effects();
    assert!(
        r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "precondition: Pillomon's [All Turns] floodgate installs \
         CannotPlayDigimonByEffect on Player 0"
    );

    // CONTROL — while Pillomon is alive, a direct effect-play of the purple Lv.3
    // from trash is BLOCKED by the floodgate (returns None). This proves the gate
    // is genuinely active; without it the Q7 outcome would be vacuous. Uses the
    // SAME engine entry point BT9-108's sub-effect 2 uses
    // (`play_from_trash_free_unsuspended`, the `play_from_trash_free` step body).
    {
        let src_card = r.game.player(1).battle_area[pillomon.index as usize]
            .top_card()
            .handle();
        let trash_card = r.game.player(0).trash[0].handle();
        let mut ctx = EffectContext::new(&mut r.game, src_card, Some(pillomon), 0);
        let blocked = ctx.play_from_trash_free_unsuspended(trash_card);
        assert!(
            blocked.is_none(),
            "CONTROL: with Pillomon alive, playing a Digimon from trash by effect \
             must be blocked by CannotPlayDigimonByEffect"
        );
    }
    assert_eq!(
        r.battle_area_size(0),
        0,
        "CONTROL must not have actually played anything"
    );
    assert_eq!(
        r.trash_size(0),
        1,
        "the purple Lv.3 is still in Player 0's trash after the blocked control play"
    );

    // ── Now run the REAL Eye of the Gorgon [Main] effect ──────────────────────
    // Play BT9-108 from hand; this installs the mandatory delete prompt.
    r.play(0, 0).expect("play BT9-108 from hand");
    let view = r
        .pending_selection_view()
        .expect("[Main] sub-effect 1: delete-target prompt installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_attack(0, pillomon.index as u16)],
        "Pillomon (unsuspended) is the sole legal delete target"
    );

    // Resolve the delete via `decode_action` so declarative state is re-ticked
    // AFTER Pillomon is removed — the engine's real between-selection refresh
    // that clears the floodgate before sub-effect 2 resolves.
    r.game
        .decode_action(view.valid_action_ids[0], 0);

    // Sub-effect 1 happened: Pillomon deleted, floodgate cleared.
    assert_eq!(
        r.battle_area_size(1),
        0,
        "sub-effect 1 deleted Pillomon"
    );
    assert!(
        !r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "deleting Pillomon clears its CannotPlayDigimonByEffect floodgate"
    );

    // Sub-effect 2: the "If you do, you may play 1 purple Lv.3 from trash" prompt.
    let play_view = r
        .pending_selection_view()
        .expect("[Main] sub-effect 2: trash-play prompt installs after the delete");
    assert_eq!(
        play_view.kind,
        SelectionKind::Trash,
        "sub-effect 2 selects a Digimon from your own trash"
    );
    assert_eq!(
        play_view.selecting_player, 0,
        "the controller chooses the card to play"
    );
    assert!(
        play_view.is_optional,
        "the printed 'you may play' makes the trash play optional"
    );
    let pick = play_view.valid_action_ids[0];

    let trash_before = r.trash_size(0);
    let battle_before = r.battle_area_size(0);
    // Resolve the play via `decode_action` (the engine's action path).
    r.game.decode_action(pick, 0);
    r.auto_resolve().ok();

    // Judge: YES — the sequential play succeeds.
    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "sub-effect 2 played the purple Lv.3 — sequential resolution let it land \
         after Pillomon's floodgate was removed by sub-effect 1 (judge-quiz Q7: YES)"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "PURPLE-L3"),
        "the purple Lv.3 from trash is now on Player 0's battle area"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before - 1,
        "the played purple Lv.3 left the trash"
    );
}

/// Q8 — Burst-Digivolve stack; Comet Hammer (BT23-096) de-digivolves to Agumon
/// (EX4-005); at EoT Burst trashes the top, DP-less Koromon (BT21-004) can't
/// remain. Judge: Agumon trashed → Koromon trashed.
#[test]
#[ignore = "BLOCKED-PRIMITIVE: G-BURST-ON-TURN-END-NOT-EXECUTED — the Burst-Digivolve `on_burst_turn_end` step list (trash the top card at the end of the burst turn) is compiled but NEVER scheduled/executed (BurstDigivolve is lowered only to a blast-counter marker in dsl_cards/mod.rs). So 'Agumon trashed → Koromon trashed' can't occur. (Also: the DP-less-can't-remain rule, and a DebugRunner burst-digivolve driver, are needed.) All quiz cards are implemented."]
fn q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot() {}

/// Q13 — Nyabootmon (BT22-042)+ShoeShoemon (P-165) vs Rapidmon (X Antibody)
/// (BT16-101). Judge: −6000 DP.
///
/// Board (card-resolution.md Q13): Player 0 digivolves into Nyabootmon
/// (BT22-042) with ShoeShoemon (P-165) — a Lv.4 [Puppet] — in hand. Player 1
/// controls a Rapidmon (X Antibody) (BT16-101) stack (ST17-07 underneath).
///
/// Nyabootmon's [When Digivolving] resolves in two sub-effects:
///   (a) play 1 Lv.4-or-lower [Puppet] from hand free → ShoeShoemon;
///   (b) "Then, to 1 of your opponent's Digimon, give -3000 DP until their turn
///       ends FOR EACH OF YOUR DIGIMON."
/// ShoeShoemon's own [On Play] (play a [Familiar] Token — another Digimon)
/// triggers when it is played by (a), but that trigger QUEUES and resolves only
/// AFTER Nyabootmon's effect fully resolves. So when (b) counts "your Digimon"
/// it sees exactly two — Nyabootmon + ShoeShoemon — NOT the Familiar token.
/// Judge: -3000 × 2 = -6000 DP (the token, added by ShoeShoemon's deferred
/// On Play, is not counted).
///
/// Pins (so it can't pass for the wrong reason): the debuff is EXACTLY -6000
/// (count 2), AND the Familiar token IS on the field afterward (proving
/// ShoeShoemon's On Play did run — just too late to be counted; a count of 3
/// would give -9000).
#[test]
fn q13_nyabootmon_dp_minus_measured_before_shoeshoemon_on_play() {
    use digimon_engine::action::space::PASS;

    let mut r = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 Nyabootmon loads")
        .dsl_card("P-165")
        .expect("P-165 ShoeShoemon loads")
        .dsl_card("BT16-101")
        .expect("BT16-101 Rapidmon (X Antibody) loads")
        .dsl_card("ST17-07")
        .expect("ST17-07 Rapidmon loads")
        .hand(0, &["P-165"])
        .memory(10)
        .start();
    r.skip_mulligan();

    let nya = r.place_on_field(0, "BT22-042", Some(0));
    // Opponent's Rapidmon (X Antibody) on top of Rapidmon (ST17-07) — DP 11000.
    let opp = r.place_stack(1, &["ST17-07", "BT16-101"]);
    let opp_dp_before = r.game.effective_dp(opp).expect("opp DP");
    let p0_digimon_before = r.game.players[0].battle_area.len(); // just Nyabootmon = 1

    // Fire Nyabootmon's [When Digivolving].
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(nya));
    r.game.drain_effect_queue();

    // Drive sub-effect (a) play ShoeShoemon, then (b) pick the opp debuff target.
    // (First non-PASS at each prompt: ShoeShoemon is the only Lv.4 Puppet in hand;
    // the Rapidmon X is the only opponent Digimon.)
    let mut guard = 0;
    while let Some(view) = r.pending_selection_view() {
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(PASS);
        r.game.decode_action(pick, view.selecting_player);
        guard += 1;
        if guard > 12 {
            break;
        }
    }
    r.auto_resolve().ok();

    // Judge: exactly -6000 (Nyabootmon + ShoeShoemon = 2; the token is not counted).
    let opp_dp_after = r.game.effective_dp(opp).expect("opp DP after");
    assert_eq!(
        opp_dp_before - opp_dp_after,
        6000,
        "Nyabootmon's debuff must be -3000 × 2 (Nyabootmon + ShoeShoemon) = -6000 — \
         the Familiar token from ShoeShoemon's deferred [On Play] is NOT counted \
         (got before={opp_dp_before:?}, after={opp_dp_after:?})"
    );

    // ShoeShoemon's [On Play] DID run (added the Familiar token) — just after the
    // count. Player 0 now has 3 Digimon: Nyabootmon + ShoeShoemon + the token.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        p0_digimon_before + 2,
        "ShoeShoemon was played AND its [On Play] added a Familiar token (so the token \
         existed by end of resolution — it simply was not counted by Nyabootmon's debuff)"
    );
}

/// Q14 — Same as Q13 but the opponent controls ShineGreymon: Ruin Mode
/// (EX4-074), whose `[When Digivolving] all of your opponent's Digimon get
/// -5000 DP until end of opponent's next turn` is ACTIVE. ShoeShoemon (4000 DP)
/// enters via Nyabootmon and gets -5000 (→ -1000, i.e. ≤0 DP), but is NOT
/// deleted until Nyabootmon's effect resolves — so it is STILL counted by
/// Nyabootmon's `-3000 × (your Digimon)`. Count = 2 (Nyabootmon + the about-to-
/// die ShoeShoemon) → judge: -6000 DP.
///
/// The distinguishing assertion is that ShoeShoemon is actually at ≤0 DP (the
/// -5000 caught it) — otherwise this would pass for the wrong reason (a healthy
/// ShoeShoemon trivially counted). That requires Ruin Mode's "all opponent
/// Digimon -5000" to be a CONTINUOUS effect catching a Digimon that enters
/// during the window — not a one-time snapshot of the Digimon present when it
/// resolved.
///
/// BLOCKED-PRIMITIVE (discovered 2026-05-30): EX4-074's mass debuff is authored
/// as a one-time snapshot — `add_modifier target: { of: opponent, kind: digimon }`
/// applies `ChangeDp -5000` to the opponent's CURRENT battle-area Digimon only;
/// it does NOT catch a Digimon (ShoeShoemon) played AFTER it resolves (verified:
/// ShoeShoemon stays at 4000, not -1000). The faithful behavior is a CONTINUOUS
/// "all opponent Digimon -5000 until end of opp's next turn" effect. Logged as
/// G-CONTINUOUS-MASS-DP-DEBUFF. The body below is the ready-to-unblock pin (it
/// correctly FAILS today rather than false-passing on a healthy ShoeShoemon).
/// Focused substrate pin for G-CONTINUOUS-MASS-DP-DEBUFF (isolated from Q14's
/// rules-check chain): EX4-074's "[When Digivolving] all opponent Digimon -5000
/// until end of opp's next turn" is CONTINUOUS — it debuffs both the opponent
/// Digimon present at install AND one that ENTERS during the window, leaves the
/// source's OWN Digimon untouched, and lifts at the right turn-end.
#[test]
fn q14_ruin_mode_mass_debuff_is_continuous_catches_later_entrant() {
    // High-DP synthetic Digimon so the -5000 keeps them positive (isolating the
    // modifier application from the ≤0-DP deletion rule).
    fn big_digimon(id: &str) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![digimon_engine::enums::CardColor::Red];
        c.level = Some(6);
        c.dp = Some(12000);
        c
    }

    let mut r = DebugRunner::builder()
        .dsl_card("EX4-074")
        .expect("EX4-074 ShineGreymon: Ruin Mode loads")
        .add_card(big_digimon("OPP-EARLY")) // opponent Digimon present at install
        .add_card(big_digimon("OPP-LATE")) // opponent Digimon that enters later
        .add_card(big_digimon("OWN-CTRL")) // source-side Digimon (must stay 12000)
        .add_card(make_test_card("FILLER", "Filler"))
        // Decks so the turn rotation across two end_turns does not deck-out.
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 0 (the start turn player) controls Ruin Mode (the debuff source) +
    // an OWN control Digimon — matching the real scenario, where Ruin Mode
    // digivolves on its controller's turn.
    let ruin = r.place_on_field(0, "EX4-074", Some(0));
    let own = r.place_on_field(0, "OWN-CTRL", Some(0));
    // Player 1 (the OPPONENT of the source) has one Digimon up front.
    let early = r.place_on_field(1, "OPP-EARLY", Some(0));
    assert_eq!(r.turn_player(), 0, "Ruin Mode installs during its controller's turn");

    // Fire Ruin Mode's [When Digivolving] → install the continuous mass debuff.
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(ruin));
    r.game.drain_effect_queue();
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(early),
        Some(7000),
        "the opponent Digimon present at install gets -5000 (12000 → 7000)"
    );
    assert_eq!(
        r.game.effective_dp(own),
        Some(12000),
        "the SOURCE's own Digimon is untouched ('your opponent's Digimon')"
    );

    // A NEW opponent Digimon ENTERS during the window → continuous effect catches it.
    let late = r.place_on_field(1, "OPP-LATE", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.effective_dp(late),
        Some(7000),
        "a later-entering opponent Digimon ALSO gets -5000 (continuous, not a \
         one-time snapshot) — G-CONTINUOUS-MASS-DP-DEBUFF"
    );

    // Expiry: installed on the source's own turn (player 0), so "until the end of
    // your opponent's next turn" = the end of player 1's upcoming turn. It must
    // SURVIVE player 0's own turn-end and lift at player 1's turn-end.
    r.end_turn(); // → player 0's turn ends → player 1's turn begins
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.effective_dp(early),
        Some(7000),
        "the debuff survives the source's OWN turn-end (it expires at the end of \
         the opponent's next turn, not the source's)"
    );
    r.end_turn(); // → end of player 1's (opponent's next) turn → debuff expires
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.effective_dp(early),
        Some(12000),
        "the debuff lifts at the end of the opponent's next turn (back to 12000)"
    );
    assert!(
        r.game.floating_mass_modifiers.is_empty(),
        "the floating descriptor is pruned once expired"
    );
}

/// RESOLVED 2026-06-02 (G-CONTINUOUS-MASS-DP-DEBUFF): EX4-074's mass debuff is
/// now authored `continuous: true`, installing a source-independent floating
/// mass modifier (`crate::floating_modifier`) re-applied to the live candidate
/// set each tick — so the ShoeShoemon (P-165) Nyabootmon plays AFTER Ruin Mode
/// resolved IS caught by the -5000 and sits at ≤0 DP when Nyabootmon's debuff
/// counts it.
#[test]
fn q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode() {
    use digimon_engine::action::space::PASS;

    let mut r = DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 Nyabootmon loads")
        .dsl_card("P-165")
        .expect("P-165 ShoeShoemon loads")
        .dsl_card("EX4-074")
        .expect("EX4-074 ShineGreymon: Ruin Mode loads")
        .hand(0, &["P-165"])
        .memory(10)
        .start();
    r.skip_mulligan();

    let nya = r.place_on_field(0, "BT22-042", Some(0));
    // Opponent's Ruin Mode (the -5000 source AND Nyabootmon's debuff target;
    // Rapidmon X from the quiz board is omitted — not load-bearing for this
    // ruling, which turns only on the deferred-deletion count).
    let ruin = r.place_on_field(1, "EX4-074", Some(0));

    // Activate Ruin Mode's [When Digivolving] mass -5000 (must be CONTINUOUS to
    // catch the ShoeShoemon Nyabootmon plays next).
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(ruin));
    r.game.drain_effect_queue();

    let ruin_dp_before = r.game.effective_dp(ruin).expect("ruin DP");

    // Fire Nyabootmon's [When Digivolving]: play ShoeShoemon, then debuff Ruin Mode.
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(nya));
    r.game.drain_effect_queue();
    // Drive the SINGLE judge-scenario resolution: accept the play-Puppet choice
    // and the debuff-target pick, but DECLINE every optional re-activation
    // (`SelectionKind::Replacement`). Nyabootmon's [On Any Deletion] clause "you
    // MAY activate this Digimon's When Digivolving effect" re-offers the whole
    // effect when the ≤0-DP ShoeShoemon is deleted; a player matching the judge
    // ruling declines it (the ruling is about the FIRST debuff's count, not the
    // recursion). Accepting it would double the debuff (−12000).
    let mut guard = 0;
    while let Some(view) = r.pending_selection_view() {
        let pick = if view.kind == digimon_engine::selection::SelectionKind::Replacement {
            PASS // decline the optional [On Any Deletion] re-activation
        } else {
            view.valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != PASS)
                .unwrap_or(PASS)
        };
        r.game.decode_action(pick, view.selecting_player);
        guard += 1;
        if guard > 12 {
            break;
        }
    }

    // ShoeShoemon must be on the field at ≤0 DP (the -5000 caught it) at the
    // moment Nyabootmon's debuff counted — proving the "≤0-but-still-counted"
    // scenario rather than a trivially-healthy ShoeShoemon.
    let shoe = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "P-165")
        .map(|i| PermanentHandle { player: 0, index: i as u8 });
    if let Some(shoe) = shoe {
        assert!(
            r.game.effective_dp(shoe).unwrap_or(4000) <= 0,
            "ShoeShoemon (4000 DP) must be at ≤0 DP from Ruin Mode's continuous -5000 \
             (got {:?}) — the scenario requires the mass debuff to catch a later-played Digimon",
            r.game.effective_dp(shoe)
        );
    }

    let _ = ruin_dp_before;

    // Judge: −6000 — ShoeShoemon was counted despite being at ≤0 DP (it is not
    // deleted until Nyabootmon's effect resolves). The ruling is about the
    // DEBUFF'S COUNT, so we assert on the modifier Nyabootmon installed on Ruin
    // Mode (each `ChangeDp` entry = −3000 × the Digimon counted): it must be
    // −6000, i.e. count = 2 (Nyabootmon + the ≤0 ShoeShoemon).
    //
    // (The NET Ruin DP additionally reflects Nyabootmon's faithful, OPTIONAL
    // `[On Any Deletion]` clause — "when any of your other Digimon are deleted,
    // you may activate this Digimon's When Digivolving effect" — which re-offers
    // the debuff once the ≤0 ShoeShoemon is deleted. That recursion is a separate
    // "you may", not part of this ruling, so we pin the per-application −6000
    // rather than the net total.)
    let ruin_dp_mods: Vec<i32> = r
        .game
        .modifiers
        .get(ruin, digimon_engine::enums::ModifierType::ChangeDp)
        .iter()
        .map(|e| e.value)
        .collect();
    assert!(
        !ruin_dp_mods.is_empty() && ruin_dp_mods.iter().all(|&v| v == -6000),
        "Nyabootmon's debuff must be −6000 per application — count = 2 (Nyabootmon \
         + the ≤0 ShoeShoemon, still counted because it is not deleted until the \
         effect resolves). Got ChangeDp mods {ruin_dp_mods:?}"
    );

    r.auto_resolve().ok();
}

/// Q24 — Hudiemon (BT23-101) <Alliance> Tentomon (BT23-037); Tentomon suspended →
/// −4000 from Rapidmon X (BT16-101) → deleted by rules check before Kokomon
/// (EX6-004) [Your Turn] contributes. Judge: Hudiemon DP 3000.
#[test]
#[ignore = "BLOCKED-PRIMITIVE: needs EX6-004 (Kokomon), which is itself BLOCKED on G-SUSPEND-EFFECT-INITIATED (the suspend event carries no by_effect bit, so Kokomon's 'when an EFFECT suspends' clause is un-gatable). Other cards (BT23-101, BT23-037, BT16-101, ST17-07) implemented."]
fn q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger() {}
