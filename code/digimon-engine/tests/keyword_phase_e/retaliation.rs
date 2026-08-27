//! Phase E §E1 — `Keyword::Retaliation` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Retaliation]` (no
//! hand-rolled `CardEffect`) must, when self is deleted by Battle, delete
//! the opposing combatant. Mandatory; no "may" clause (RULES_CONTEXT
//! 16-12). Cause filter: `deletion_cause() == Some(ReplacementCause::Battle)`.
//!
//! Mirrors DCGO `Retaliation.cs` — fires from `OnDestroyedAnyone` with
//! `IsByBattle(hashtable)` cause filter, targets the opposing combatant
//! (`WinnerPermanents` in DCGO terms) read from the live battle state.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::{digimon_with_keywords, plain_digimon};

fn retaliation_card(id: &str, dp: i32) -> CardData {
    digimon_with_keywords(id, 5, dp, vec![Keyword::Retaliation])
}

// ─── Test 1: happy path — self deleted in battle → deletes the winner ────────

/// Stack: P0[ATK 5000 DP], P1[RETAL 3000 DP, Retaliation].
/// P0 attacks RETAL → battle resolves, attacker wins → RETAL deleted
/// (cause=Battle) → Retaliation fires → ATK also deleted.
/// Both end in trash; both battle areas empty.
#[test]
fn retaliation_deletes_winner_when_self_loses_battle() {
    let atk = {
        let mut c = plain_digimon("ATK");
        c.dp = Some(5000);
        c
    };
    let retal = retaliation_card("RETAL", 3000);

    let mut r = DebugRunner::builder().add_card(atk).add_card(retal).start();

    // Pass Some(0) for turn_played_override to bypass summoning sickness
    // (turn_count == 1 after start_game(); turn_played == 0 < 1 → not fresh).
    let atk_h = r.place_on_field(0, "ATK", Some(0));
    let retal_h = r.place_on_field(1, "RETAL", Some(0));

    // Drive battle: ATK attacks RETAL. ATK wins (5000 > 3000).
    // RETAL is deleted with cause=Battle. Its Retaliation fires and
    // deletes ATK.
    r.attack_digimon(atk_h, retal_h, false);

    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ATK should be deleted by Retaliation"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "RETAL should be deleted by losing the battle"
    );
    assert_eq!(r.game.players[0].trash.len(), 1, "ATK lands in P0 trash");
    assert_eq!(r.game.players[1].trash.len(), 1, "RETAL lands in P1 trash");
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "ATK",
        "P0 trash should contain ATK (deleted by Retaliation)"
    );
    assert_eq!(
        r.game.players[1].trash[0].card_id(&r.game.card_data),
        "RETAL",
        "P1 trash should contain RETAL (deleted by losing battle)"
    );
}

// ─── Test 2: cause gate — effect deletion does NOT fire Retaliation ──────────

/// Cause gate: when self is deleted by an opponent's effect (not Battle),
/// Retaliation does NOT fire — there is no live pending_attack.
#[test]
fn retaliation_does_not_fire_on_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(retaliation_card("RETAL", 3000))
        .add_card(plain_digimon("BYSTANDER"))
        .start();

    let retal_h = r.place_on_field(0, "RETAL", None);
    let _by = r.place_on_field(1, "BYSTANDER", None);

    // Direct effect-cause deletion — no battle.
    r.game.delete_permanent_with_cause(
        retal_h,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    // BYSTANDER must be untouched — Retaliation did not fire.
    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "BYSTANDER must survive — Retaliation must not fire on OpponentEffect deletion"
    );
    assert_eq!(r.game.players[1].trash.len(), 0);
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "RETAL is trashed normally"
    );
}

// ─── Test 3: cause gate — own-effect deletion does NOT fire Retaliation ──────

/// Cause gate: when self is deleted by its own controller's effect,
/// Retaliation does NOT fire.
#[test]
fn retaliation_does_not_fire_on_own_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(retaliation_card("RETAL", 3000))
        .add_card(plain_digimon("BYSTANDER"))
        .start();

    let retal_h = r.place_on_field(0, "RETAL", None);
    let _by = r.place_on_field(1, "BYSTANDER", None);

    r.game.delete_permanent_with_cause(
        retal_h,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );

    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "BYSTANDER must survive — Retaliation must not fire on OwnEffect deletion"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "RETAL is trashed normally"
    );
}

// ─── Test 4: mutual destruction ──────────────────────────────────────────────

/// Mutual destruction: both combatants have Retaliation and tie in DP.
/// Both are deleted in battle (resolve_battle's MutualDestruction branch).
/// The defender's Retaliation fires first and tries to delete the attacker;
/// the attacker's Retaliation fires but the defender is already gone.
/// No panic; both trashes have exactly 1 card each.
#[test]
fn retaliation_handles_mutual_destruction() {
    let r1 = retaliation_card("R1", 4000);
    let r2 = retaliation_card("R2", 4000);

    let mut r = DebugRunner::builder().add_card(r1).add_card(r2).start();

    let r1_h = r.place_on_field(0, "R1", Some(0));
    let r2_h = r.place_on_field(1, "R2", Some(0));

    // Equal DP → mutual destruction. Both deleted in battle; each
    // Retaliation fires against the already-deleting opponent. Graceful
    // (no panic); final state: both battle areas empty, each trash has 1.
    r.attack_digimon(r1_h, r2_h, false);

    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "R1 deleted in mutual destruction"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "R2 deleted in mutual destruction"
    );
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[1].trash.len(), 1);
}

// ─── G-ONDELETION-PARK-CLEARS-BATTLE-STATE (reproducer) ─────────────────────

/// `<Retaliation>` reads its victim from the LIVE battle
/// (`EffectContext::battle_opponent_of` → `Game::pending_attack`). When another
/// `[On Deletion]` clause on the SAME carrier parks a selection, the parking
/// unwinds `delete_permanents_batch`, which restores `pending_attack` /
/// `current_deletion_cause` before the resume drains the rest of the
/// OnDeletion bundle — so `<Retaliation>` resumes with no battle to read and
/// silently no-ops.
///
/// The carrier below takes `<Retaliation>` through the DSL's printed-keyword
/// form (`kind: grant_keyword`, the `Effect::granted_keyword` marker
/// `Game::build_effects_for_card` synthesizes from), so this reproducer is
/// independent of the aura-granted trigger dispatch added for
/// `G-ENGINE-AURA-GRANT-NO-TRIGGER` — it isolates the battle-state lifetime.
///
/// `#[ignore]`d until the gap is fixed; see `docs/RUST_ENGINE_GAPS.md`.
#[test]
#[ignore = "engine gap: G-ONDELETION-PARK-CLEARS-BATTLE-STATE — a parked sibling [On Deletion] \
clause unwinds the deletion batch and clears `pending_attack`, so <Retaliation> finds no battle \
opponent on resume; see docs/RUST_ENGINE_GAPS.md"]
fn retaliation_survives_a_parked_sibling_on_deletion_clause() {
    use digimon_engine::action::space::PASS;

    let yaml = r#"
card: DSL-RETAL-PARK
name: Retaliation Parker
kind: digimon
level: 5
color: [blue]
cost: 5
dp: 3000
effects:
  - kind: grant_keyword
    keyword: Retaliation
    summary: "<Retaliation>"
  - when: on_deletion
    summary: "[On Deletion] Return 1 of your opponent's [Decoy] Digimon to the bottom of the deck"
    process:
      - select_opponent_permanent:
          bind_as: bottom_target
          filter:
            all_of:
              - kind: digimon
              - trait_has: Decoy
          prompt: "Return 1 [Decoy] Digimon to the bottom of the deck"
      - return_to_deck: { target: bottom_target, position: bottom }
"#;

    let mut winner = plain_digimon("BIG");
    winner.dp = Some(20000);
    let mut decoy_a = plain_digimon("DECOY-A");
    decoy_a.traits = vec!["Decoy".to_string()];
    let mut decoy_b = plain_digimon("DECOY-B");
    decoy_b.traits = vec!["Decoy".to_string()];

    let mut r = DebugRunner::builder()
        .add_card(winner)
        .add_card(decoy_a)
        .add_card(decoy_b)
        .from_dsl_yaml(yaml)
        .expect("reproducer card compiles")
        .start();

    let carrier = r.place_on_field(0, "DSL-RETAL-PARK", Some(0));
    let attacker = r.place_on_field(1, "BIG", Some(0));
    // Two candidates so the sibling clause genuinely PARKS a selection.
    r.place_on_field(1, "DECOY-A", Some(0));
    r.place_on_field(1, "DECOY-B", Some(0));
    r.game.tick_declarative_effects();

    assert!(r.game.has_keyword(carrier, Keyword::Retaliation));

    r.attack_digimon(attacker, carrier, false);
    while let Some(view) = r.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        r.execute_action(view.selecting_player, action)
            .expect("resolve pending selection");
    }

    assert!(
        r.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BIG"),
        "<Retaliation> must still delete the battle winner when a sibling \
         [On Deletion] clause parked a selection first"
    );
}
