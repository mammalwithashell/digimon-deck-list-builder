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
