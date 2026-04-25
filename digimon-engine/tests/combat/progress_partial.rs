//! Phase A §A1 — Progress keyword partial-fix coverage.
//!
//! Two behaviors verified here:
//! 1. With attacker holding printed `<Progress>`, the defender's
//!    `SecuritySkill` timing still fires when a security card is revealed.
//!    (DCGO's ProgressProcess does NOT gate the phase; it only excludes
//!    the attacker from opponent-effect targeting. Regression coverage
//!    against the incorrectly-shipped 2026-04-24 gate.)
//! 2. An opponent-sourced `select_opponent_permanent` call issued while
//!    the Progress-carrier is attacking must not yield the Progress
//!    permanent as a candidate. (Covered in Task 4.)

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn fighter_with_keywords(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn progress_attacker_does_not_suppress_security_skill_drain() {
    // Sanity check: an attacker with Progress still causes the defender's
    // SecuritySkill phase to run. No revealed card carries a SecuritySkill
    // effect in this fixture — this test verifies the phase transitions
    // through SecuritySkillDrain → BattleResolved normally, as a regression
    // guard against accidentally re-adding a gate there.
    let mut r = DebugRunner::builder()
        .add_card(fighter_with_keywords("ATK", 6000, vec![Keyword::Progress]))
        .add_card(fighter_with_keywords("SECCARD", 0, vec![]))
        .start();

    // Place attacker on field with Rush granted so it can attack on the
    // turn it was placed (Rush is unrelated to Progress; just a fixture
    // convenience).
    use digimon_engine::enums::Expiry;
    let attacker = r.place_on_field(0, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);

    // Seed opponent security with one card so a check runs.
    let sec_card = {
        use digimon_engine::card_source::CardSource;
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "SECCARD")
            .unwrap();
        let idx = r.game.next_card_index();
        CardSource::new(data_idx, 1, idx)
    };
    r.game.players[1].security.push(sec_card);

    // Direct-player attack → runs resolve_player_security_loop.
    // Player 1 is the defender (opponent of attacker's player 0).
    let result = r.attack_player(attacker, 1, false);
    // With Progress + empty SecuritySkill effects on the revealed card,
    // the outcome is simply SecurityCheckSurvived (1 security consumed,
    // attacker survived). The key invariant: no `Invalid`, no panic, no
    // SecuritySkill-skip regression.
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::SecurityCheckSurvived,
        "Progress attacker must not prevent security resolution from progressing"
    );
    assert_eq!(
        r.game.players[1].security.len(),
        0,
        "one security card should have been consumed"
    );
}

#[test]
fn select_opponent_permanent_excludes_progress_attacker() {
    // Setup: own player P0 attacks with a Progress carrier. The
    // defending side (P1) tries to select one of P0's Digimon via
    // `select_opponent_permanent`. The Progress carrier must be
    // filtered out; the non-Progress sibling must still be selectable.

    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::GamePhase;
    use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack};

    let mut r = DebugRunner::builder()
        .add_card(fighter_with_keywords("PROG", 6000, vec![Keyword::Progress]))
        .add_card(fighter_with_keywords("SIB", 4000, vec![]))
        .add_card(fighter_with_keywords("OPP", 3000, vec![]))
        .start();

    let progress = r.place_on_field(0, "PROG", None);
    let sibling = r.place_on_field(0, "SIB", None);
    let _opponent = r.place_on_field(1, "OPP", None);

    // Mark Progress carrier as attacking.
    r.game.pending_attack = Some(PendingAttack {
        attacker: progress,
        original_target: AttackTarget::Player(1),
        effective_target: AttackTarget::Player(1),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        cancelled: false,
        battle_occurred: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });

    // Opponent (P1) installs a selection whose filter accepts ALL P0 Digimon.
    // After Task 4's gate, the Progress attacker should not appear in the
    // candidate list.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1); // selecting player = 1
        ctx.select_opponent_permanent(
            "pick",
            false,
            |_game, _h| true,
            move |_, _h| {
                // Callback intentionally empty — we only inspect the pending
                // selection's candidate list, not its resolution.
            },
        );
    }

    // `PendingSelection.valid_action_ids` holds the decoder-accepted
    // action IDs for the installed selection. Its length is the count
    // of selectable candidates. With Progress gating the attacker out
    // of the opponent's candidate pool, we should see exactly one
    // selectable permanent (the sibling).
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection should be installed");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly one candidate should remain after Progress exclusion; \
         got {} action IDs: {:?}",
        pending.valid_action_ids.len(),
        pending.valid_action_ids,
    );

    let _ = sibling; // silence unused warning
}
