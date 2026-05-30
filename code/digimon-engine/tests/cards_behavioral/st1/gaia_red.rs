//! ST-1 Starter Deck Gaia Red — full embedded YAML coverage.
//!
//! The deck list is the worldwide ST-1 Gaia Red starter deck. Printed card text
//! is sourced from the per-card JSON fixtures under `cards/st1/`.

use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{Keyword, ModifierType, PlaySource};

const ST1_IDS: [&str; 16] = [
    "ST1-01", "ST1-02", "ST1-03", "ST1-04", "ST1-05", "ST1-06", "ST1-07", "ST1-08",
    "ST1-09", "ST1-10", "ST1-11", "ST1-12", "ST1-13", "ST1-14", "ST1-15", "ST1-16",
];

fn st1_builder() -> DebugRunnerBuilder {
    let mut builder = DebugRunner::builder();
    for id in ST1_IDS {
        builder = builder
            .dsl_card(id)
            .unwrap_or_else(|err| panic!("{id} must be in embedded DSL pack: {err}"));
    }
    builder
}

#[test]
fn st1_gaia_red_all_unique_cards_compile_from_embedded_pack() {
    let runner = st1_builder().start();

    for id in ST1_IDS {
        assert!(
            runner.compiled_card(id).is_some(),
            "{id} must compile from the embedded DSL pack"
        );
    }
}

#[test]
fn st1_vanilla_cards_have_no_effect_clauses() {
    let runner = st1_builder().start();

    for id in ["ST1-02", "ST1-04", "ST1-05", "ST1-10"] {
        let compiled = runner.compiled_card(id).expect("card in pack");
        assert!(
            compiled.effects.is_empty(),
            "{id} is vanilla in printed text and must not script effects"
        );
    }
}

#[test]
fn st1_inherited_dp_and_security_attack_effects_apply_to_carriers() {
    let mut runner = st1_builder().start();

    let low_source_count = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-11"]);
    assert_eq!(
        runner.game.source_dp_contribution(low_source_count, 0),
        0,
        "ST1-01 stays inactive below 4 digivolution cards"
    );
    assert_eq!(
        runner.game.source_dp_contribution(low_source_count, 1),
        1000,
        "ST1-03 contributes +1000 DP on your turn"
    );

    let four_sources = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-07", "ST1-08", "ST1-11"]);
    assert_eq!(
        runner.game.source_dp_contribution(four_sources, 0),
        1000,
        "ST1-01 contributes +1000 DP at 4 digivolution cards"
    );
    assert_eq!(
        runner.game.source_dp_contribution(four_sources, 1),
        1000,
        "ST1-03 still contributes +1000 DP"
    );
    assert!(
        runner
            .game
            .has_keyword(four_sources, Keyword::SecurityAttackPlus(1)),
        "ST1-07 grants inherited SecurityAttackPlus(1)"
    );
    assert_eq!(
        runner.game.dynamic_security_attack_aura_bonus(four_sources),
        Some(3),
        "ST1-11's security_attack_fn is BASE-INCLUSIVE: it returns the total base \
         check count 1 + floor(4/2) = 3 (default base-1 + the +1-per-2-cards bonus), \
         not a bare delta. Flat <Security A.> keyword deltas (e.g. ST1-07's inherited \
         +1) are summed on top in current_security_strike."
    );
}

#[test]
fn st1_06_has_blocker_and_when_attacking_loses_two_memory() {
    let mut runner = st1_builder().memory(5).security(1, &[]).start();
    let coredramon = runner.place_on_field(0, "ST1-06", Some(0));

    assert!(
        runner.game.has_keyword(coredramon, Keyword::Blocker),
        "ST1-06 has printed Blocker"
    );

    runner.attack_player(coredramon, 1, false);
    assert_eq!(
        runner.memory(),
        3,
        "ST1-06 [When Attacking] loses exactly 2 memory"
    );
}

#[test]
fn st1_09_inherited_on_block_gains_three_memory_for_blocked_carrier() {
    let mut runner = st1_builder().memory(0).start();
    let attacker = runner.place_stack(0, &["ST1-09", "ST1-11"]);
    let defender = runner.place_on_field(1, "ST1-02", Some(0));
    let blocker = runner.place_on_field(1, "ST1-06", Some(0));

    runner.attack_digimon(attacker, defender, false);
    let block_action = encode_attack(0, blocker.index as u16);
    runner
        .game
        .resolve_selection(1, block_action)
        .expect("declare ST1-06 as blocker");

    assert_eq!(
        runner.memory(),
        3,
        "ST1-09 inherited [when blocked] effect gains 3 memory"
    );
}

#[test]
fn st1_12_tai_kamiya_buffs_all_own_digimon_on_your_turn() {
    let mut runner = st1_builder().start();
    let biyomon = runner.place_on_field(0, "ST1-02", Some(0));
    let opponent = runner.place_on_field(1, "ST1-02", Some(0));

    assert_eq!(runner.effective_dp(biyomon), Some(3_000));
    runner.place_on_field(0, "ST1-12", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(biyomon),
        Some(4_000),
        "Tai buffs your Digimon by +1000 DP"
    );
    assert_eq!(
        runner.effective_dp(opponent),
        Some(3_000),
        "Tai does not buff opponent Digimon"
    );
}

#[test]
fn st1_14_starlight_explosion_main_adds_own_security_dp_modifier() {
    let mut runner = st1_builder().memory(10).hand(0, &["ST1-14"]).start();

    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(0, ModifierType::ChangeOwnSecurityDigimonDp),
        0
    );
    runner.play(0, 0).expect("play ST1-14 from hand");

    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(0, ModifierType::ChangeOwnSecurityDigimonDp),
        7000,
        "ST1-14 gives your security Digimon +7000 DP"
    );
}

#[test]
fn st1_15_giga_destroyer_selects_up_to_two_opponent_digimon_4000_dp_or_less() {
    let mut runner = st1_builder().memory(10).hand(0, &["ST1-15"]).start();
    let weak_a = runner.place_on_field(1, "ST1-03", Some(0));
    let weak_b = runner.place_on_field(1, "ST1-04", Some(0));
    let too_large = runner.place_on_field(1, "ST1-05", Some(0));

    runner.play(0, 0).expect("play ST1-15 from hand");
    let first_prompt = runner
        .pending_selection()
        .expect("ST1-15 installs an up-to-2 selection");
    assert!(
        first_prompt.is_optional,
        "ST1-15 is an up-to-2 choice and must allow selecting zero targets"
    );
    assert_eq!(
        first_prompt.valid_action_ids.len(),
        2,
        "ST1-15 must offer only the two opponent Digimon with 4000 DP or less"
    );

    let first_pick = first_prompt.valid_action_ids[0];
    runner
        .execute_action(0, first_pick)
        .expect("pick first small Digimon");
    let second_pick = runner
        .pending_selection()
        .expect("second ST1-15 pick remains pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, second_pick)
        .expect("pick second small Digimon");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "only the two selected small Digimon are deleted"
    );
    assert_eq!(
        runner.trash_size(1),
        2,
        "deleted opponent Digimon go to trash"
    );
    let _ = (weak_a, weak_b, too_large);
}

#[test]
fn st1_16_gaia_force_selects_and_deletes_one_opponent_digimon() {
    let mut runner = st1_builder().memory(10).hand(0, &["ST1-16"]).start();
    let victim = runner.place_on_field(1, "ST1-05", Some(0));

    runner.play(0, 0).expect("play ST1-16 from hand");
    let prompt = runner
        .pending_selection()
        .expect("ST1-16 installs a mandatory opponent Digimon selection");
    assert!(
        prompt
            .valid_action_ids
            .contains(&encode_attack(0, victim.index as u16)),
        "Gaia Force must offer the opponent Digimon as a legal target"
    );

    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete selected Digimon");
    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.trash_size(1), 1);
}

fn st1_resolve_pending(runner: &mut DebugRunner) {
    for _ in 0..8 {
        if runner.pending_selection().is_none() {
            return;
        }
        runner.auto_resolve().expect("pending selection resolves");
    }
}

#[test]
fn st1_08_when_digivolving_buffs_one_own_digimon() {
    // Coverage for ST1-08 Garudamon [When Digivolving] "1 of your Digimon gets
    // +3000 DP for the turn" — a mandatory single-target buff that expires EOT.
    let mut runner = st1_builder()
        .memory(10)
        .hand(0, &["ST1-08"])
        .deck(0, &["ST1-02", "ST1-02"])
        .start();
    let base = runner.place_on_field(0, "ST1-05", Some(0));

    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));

    let prompt = runner
        .pending_selection()
        .expect("ST1-08 [When Digivolving] installs a +3000 DP target selection");
    assert!(
        !prompt.is_optional,
        "ST1-08 buffs exactly 1 of your Digimon and must not be declinable"
    );
    assert_eq!(
        prompt.valid_action_ids.len(),
        1,
        "the digivolved Garudamon is the only own Digimon to target"
    );
    let pick = prompt.valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("apply +3000 to the only own Digimon");

    assert_eq!(
        runner.effective_dp(base),
        Some(10_000),
        "Garudamon (7000) gains +3000 DP for the turn"
    );
    runner.end_turn();
    assert_eq!(
        runner.effective_dp(base),
        Some(7_000),
        "the +3000 DP expires at the end of the turn"
    );
}

#[test]
fn st1_12_security_plays_tamer_for_free() {
    // Coverage for ST1-12 Tai Kamiya [Security] "Play this card without paying
    // the cost" — flips from the security stack into the battle area for free.
    let mut runner = st1_builder().memory(10).security(1, &["ST1-12"]).start();
    let attacker = runner.place_on_field(0, "ST1-03", Some(0));

    runner.attack_player(attacker, 1, false);
    st1_resolve_pending(&mut runner);

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ST1-12"),
        "ST1-12 [Security] plays the Tamer into the battle area without paying cost"
    );
}
