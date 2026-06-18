use std::collections::HashMap;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCard, CompiledCardKind, CompiledClause, CompiledColor,
    CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::encode_source_select;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::selection::{AttackTarget, TriggerSource};
use digimon_engine::PermanentHandle;

use crate::dsl_card_data::compiled;

const ST2_IDS: [&str; 16] = [
    "ST2-01", "ST2-02", "ST2-03", "ST2-04", "ST2-05", "ST2-06", "ST2-07", "ST2-08", "ST2-09",
    "ST2-10", "ST2-11", "ST2-12", "ST2-13", "ST2-14", "ST2-15", "ST2-16",
];
const DECK_LIBRARY_JSON: &str = include_str!("../../../../../data/deck_library.json");

fn st2_builder() -> DebugRunnerBuilder {
    let mut builder = DebugRunner::builder();
    for card_id in ST2_IDS {
        builder = builder
            .dsl_card(card_id)
            .unwrap_or_else(|err| panic!("{card_id} must load from embedded ST2 pack: {err}"));
    }
    builder
}

fn digimon(id: &str, level: u8, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = level as u16;
    card
}

fn source_ids(runner: &DebugRunner, player: u8, index: usize) -> Vec<String> {
    runner.game.player(player).battle_area[index]
        .card_sources
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn top_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner
        .game
        .player(player)
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn official_counts() -> HashMap<&'static str, usize> {
    HashMap::from([
        ("ST2-01", 4usize),
        ("ST2-02", 4),
        ("ST2-03", 4),
        ("ST2-04", 4),
        ("ST2-05", 4),
        ("ST2-06", 2),
        ("ST2-07", 4),
        ("ST2-08", 4),
        ("ST2-09", 4),
        ("ST2-10", 2),
        ("ST2-11", 2),
        ("ST2-12", 4),
        ("ST2-13", 4),
        ("ST2-14", 4),
        ("ST2-15", 2),
        ("ST2-16", 2),
    ])
}

fn st2_deck_library_ids() -> Vec<String> {
    let root: serde_json::Value =
        serde_json::from_str(DECK_LIBRARY_JSON).expect("deck_library.json parses");
    let decklist = root["archetypes"]["ST-2 Cocytus Blue"]["decklists"][0]["decklist"]
        .as_str()
        .expect("ST-2 decklist is encoded with deck_library string convention");
    serde_json::from_str(decklist).expect("ST-2 decklist string parses as a card-id array")
}

fn choose_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .pending_selection()
            .expect("expected a pending selection");
        (
            pending.selecting_player,
            pending
                .valid_action_ids
                .first()
                .copied()
                .expect("selection has at least one action"),
        )
    };
    runner
        .execute_action(player, action)
        .expect("pending selection resolves");
    runner.game.drain_effect_queue();
}

fn triggered(
    card: &CompiledCard,
    timing: CompiledTiming,
) -> &digimon_dsl::compiled::CompiledTriggeredClause {
    card.effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&timing) => Some(t),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{} must have {timing:?} clause", card.card))
}

#[test]
fn st2_all_unique_cards_compile_from_embedded_pack() {
    for card_id in ST2_IDS {
        assert_eq!(compiled(card_id).card, card_id);
    }
}

#[test]
fn st2_vanilla_cards_have_printed_metadata_and_no_effects() {
    let expected = [
        (
            "ST2-02",
            "Gomamon",
            3,
            2,
            3000,
            "Rookie",
            "Vaccine",
            "Sea Beast",
            2,
            0,
        ),
        (
            "ST2-04", "Bearmon", 3, 3, 4000, "Rookie", "Vaccine", "Beast", 2, 0,
        ),
        (
            "ST2-05",
            "Ikkakumon",
            4,
            4,
            5000,
            "Champion",
            "Vaccine",
            "Sea Beast",
            3,
            2,
        ),
        (
            "ST2-10",
            "Plesiomon",
            6,
            10,
            12000,
            "Mega",
            "Data",
            "Plesiosaur",
            5,
            2,
        ),
    ];

    for (id, name, level, cost, dp, form, attr, trait_name, evo_from, evo_cost) in expected {
        let card = compiled(id);
        assert_eq!(card.name, name);
        assert_eq!(card.kind, CompiledCardKind::Digimon);
        assert_eq!(card.level, Some(level));
        assert_eq!(card.cost, Some(cost));
        assert_eq!(card.dp, Some(dp));
        assert_eq!(card.color, vec![CompiledColor::Blue]);
        assert_eq!(card.form.as_deref(), Some(form));
        assert_eq!(card.attribute.as_deref(), Some(attr));
        assert!(card.traits.iter().any(|t| t == trait_name));
        assert!(card.effects.is_empty(), "{id} must not fake no-op effects");

        assert_eq!(
            card.alt_paths.len(),
            1,
            "{id} has one printed blue evo path"
        );
        let alt = &card.alt_paths[0];
        assert_eq!(alt.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(alt.cost, Some(CompiledCost::Literal(evo_cost)));
        let from = alt.from.as_ref().expect("alt path has source filter");
        assert!(matches!(
            from.level_eq,
            Some(n) if n == evo_from
        ));
        assert_eq!(from.color_is, Some(CompiledColor::Blue));
    }
}

#[test]
fn st2_01_tsunomon_grants_battle_only_dp_against_no_source_opponent() {
    let mut runner = st2_builder()
        .add_card(digimon("ATK", 3, 1000))
        .add_card(digimon("DEF", 3, 2000))
        .start();
    let attacker = runner.place_stack(0, &["ST2-01", "ATK"]);
    let defender = runner.place_on_field(1, "DEF", Some(0));
    runner.game.tick_declarative_effects();

    let result = runner
        .game
        .begin_attack(attacker, AttackTarget::Digimon(defender), false);
    assert_eq!(result, AttackResult::MutualDestruction);
    assert!(runner.game.player(0).battle_area.is_empty());
    assert!(runner.game.player(1).battle_area.is_empty());

    let mut with_source = st2_builder()
        .add_card(digimon("ATK", 3, 1000))
        .add_card(digimon("DEF-SRC", 2, 1000))
        .add_card(digimon("DEF", 3, 2000))
        .start();
    let attacker = with_source.place_stack(0, &["ST2-01", "ATK"]);
    let defender = with_source.place_stack(1, &["DEF-SRC", "DEF"]);
    with_source.game.tick_declarative_effects();

    let result = with_source
        .game
        .begin_attack(attacker, AttackTarget::Digimon(defender), false);
    assert_eq!(result, AttackResult::DefenderWins);
    assert!(with_source.game.player(0).battle_area.is_empty());
    assert_eq!(with_source.game.player(1).battle_area.len(), 1);

    let mut security_check = st2_builder()
        .add_card(digimon("ATK", 3, 1000))
        .add_card(digimon("FILL", 3, 1000))
        .security(1, &["FILL"])
        .start();
    let attacker = security_check.place_stack(0, &["ST2-01", "ATK"]);
    security_check.game.tick_declarative_effects();
    assert_eq!(
        security_check.effective_dp(attacker),
        Some(1000),
        "battle_opponent_no_sources must be false outside Digimon-vs-Digimon battle"
    );
}

#[test]
fn st2_03_and_st2_06_trash_bottom_sources_without_source_prompt() {
    for (card_id, top_id, target_stack, expected_sources, expected_trash) in [
        (
            "ST2-03",
            "ST2-05",
            vec!["BOT", "MID", "OPP-TOP"],
            vec!["MID", "OPP-TOP"],
            vec!["BOT"],
        ),
        (
            "ST2-06",
            "ST2-08",
            vec!["BOT", "OPP-TOP"],
            vec!["OPP-TOP"],
            vec!["BOT"],
        ),
    ] {
        let mut runner = st2_builder()
            .add_card(digimon("BOT", 3, 1000))
            .add_card(digimon("MID", 3, 1000))
            .add_card(digimon("OPP-TOP", 5, 5000))
            .start();
        let attacker = runner.place_stack(0, &[card_id, top_id]);
        runner.place_stack(1, &target_stack);

        fire(&mut runner, EffectTiming::WhenAttacking, attacker);
        choose_first_pending(&mut runner);

        assert!(
            runner.pending_selection().is_none(),
            "{card_id} must not install a source-selection prompt after choosing the Digimon"
        );
        assert_eq!(source_ids(&runner, 1, 0), expected_sources);
        assert_eq!(trash_ids(&runner, 1), expected_trash);
    }
}

#[test]
fn st2_06_targets_sourceless_opponent_digimon() {
    // ST2-06 Garurumon inherited [When Attacking]: "Trash the digivolution card at
    // the bottom of 1 of your opponent's Digimon." DCGO ST2_06.cs's
    // CanSelectPermanentCondition checks ONLY that the target is an opponent
    // battle-area Digimon (NO source-count requirement) — unlike ST2-03/ST2-09,
    // whose DCGO sources DO require a digivolution card. So a sourceless opponent
    // Digimon is a legal target (the trash is simply a no-op). The effect must
    // still activate and surface the choice.
    let mut runner = st2_builder().add_card(digimon("OPP-NOSRC", 5, 5000)).start();
    let attacker = runner.place_stack(0, &["ST2-06", "ST2-08"]);
    // Opponent has exactly one Digimon, with NO digivolution cards.
    runner.place_on_field(1, "OPP-NOSRC", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, attacker);

    let pending = runner.pending_selection().expect(
        "ST2-06 must activate and surface a target selection even when the only \
         opponent Digimon has no digivolution cards (matches DCGO ST2_06.cs)",
    );
    assert_eq!(pending.selecting_player, 0);

    choose_first_pending(&mut runner);

    // Trashing the bottom source of a sourceless Digimon is a clean no-op:
    // the Digimon stays on the field and nothing enters trash.
    assert_eq!(top_ids(&runner, 1), vec!["OPP-NOSRC"]);
    assert!(trash_ids(&runner, 1).is_empty());
    assert!(runner.pending_selection().is_none());
}

#[test]
fn st2_07_grizzlymon_has_blocker_and_loses_two_memory_when_attacking() {
    let mut runner = st2_builder().memory(5).start();
    let grizzlymon = runner.place_on_field(0, "ST2-07", Some(0));

    assert!(runner.game.has_keyword(grizzlymon, Keyword::Blocker));

    fire(&mut runner, EffectTiming::WhenAttacking, grizzlymon);
    assert_eq!(runner.memory(), 3);
}

#[test]
fn st2_08_weregarurumon_inherited_grants_security_attack_when_opponent_has_no_sources() {
    let mut runner = st2_builder().start();
    let carrier = runner.place_stack(0, &["ST2-08", "ST2-10"]);
    runner.place_on_field(1, "ST2-02", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(runner.game.security_attack_keyword_bonus(carrier), 1);

    let mut with_sourced_opp = st2_builder().start();
    let carrier = with_sourced_opp.place_stack(0, &["ST2-08", "ST2-10"]);
    with_sourced_opp.place_stack(1, &["ST2-02", "ST2-05"]);
    with_sourced_opp.game.tick_declarative_effects();

    assert_eq!(
        with_sourced_opp.game.security_attack_keyword_bonus(carrier),
        0
    );
}

#[test]
fn st2_09_zudomon_trashes_two_bottom_sources_on_digivolve() {
    let mut runner = st2_builder()
        .add_card(digimon("BOT", 3, 1000))
        .add_card(digimon("MID", 3, 1000))
        .add_card(digimon("TOPSRC", 3, 1000))
        .start();
    let zudomon = runner.place_on_field(0, "ST2-09", Some(0));
    runner.place_stack(1, &["BOT", "MID", "TOPSRC", "ST2-08"]);

    fire(&mut runner, EffectTiming::WhenDigivolving, zudomon);
    choose_first_pending(&mut runner);

    assert!(runner.pending_selection().is_none());
    assert_eq!(source_ids(&runner, 1, 0), vec!["TOPSRC", "ST2-08"]);
    assert_eq!(trash_ids(&runner, 1), vec!["BOT", "MID"]);
}

#[test]
fn st2_11_metalgarurumon_unsuspends_once_per_turn_when_attacking() {
    let mut runner = st2_builder().start();
    let metalgarurumon = runner.place_on_field(0, "ST2-11", Some(0));
    runner.game.player_mut(0).battle_area[metalgarurumon.index as usize].is_suspended = true;

    fire(&mut runner, EffectTiming::WhenAttacking, metalgarurumon);
    assert!(
        !runner.game.player(0).battle_area[metalgarurumon.index as usize].is_suspended,
        "first OPT attack trigger unsuspends MetalGarurumon"
    );

    runner.game.player_mut(0).battle_area[metalgarurumon.index as usize].is_suspended = true;
    fire(&mut runner, EffectTiming::WhenAttacking, metalgarurumon);
    assert!(
        runner.game.player(0).battle_area[metalgarurumon.index as usize].is_suspended,
        "second same-turn trigger is blocked by once_per_turn"
    );
}

#[test]
fn st2_12_matt_gains_memory_at_turn_start_and_security_plays_self() {
    let mut runner = st2_builder().memory(0).start();
    let matt = runner.place_on_field(0, "ST2-12", Some(0));
    runner.place_on_field(1, "ST2-02", Some(0));

    fire(&mut runner, EffectTiming::StartOfYourTurn, matt);
    assert_eq!(runner.memory(), 1);

    let security = compiled("ST2-12");
    let security_clause = triggered(&security, CompiledTiming::OnSecurity);
    assert!(security_clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromSecurity)));
}

#[test]
fn st2_14_sorrow_blue_restricts_only_no_source_opponent_digimon() {
    let mut runner = st2_builder().hand(0, &["ST2-14"]).start();
    let no_source = runner.place_on_field(1, "ST2-02", Some(0));
    let sourced = runner.place_stack(1, &["ST2-02", "ST2-05"]);

    assert!(runner.game.activate_hand_main(0, 0));
    {
        let pending = runner
            .pending_selection()
            .expect("Sorrow Blue target prompt");
        assert_eq!(pending.valid_action_ids.len(), 1);
    }
    choose_first_pending(&mut runner);

    assert!(runner
        .game
        .modifiers
        .has(no_source, ModifierType::CannotAttack));
    assert!(runner
        .game
        .modifiers
        .has(no_source, ModifierType::CannotBlock));
    assert!(!runner
        .game
        .modifiers
        .has(sourced, ModifierType::CannotAttack));
    assert!(!runner
        .game
        .modifiers
        .has(sourced, ModifierType::CannotBlock));
}

#[test]
fn st2_14_security_restriction_survives_until_your_next_turn() {
    // ST2-14 [Security] locks 1 no-source opponent Digimon "until the end of
    // YOUR next turn" (the security owner's). When P0 attacks and P1's security
    // flips Sorrow Blue, the lock is installed during P0's turn and must persist
    // through P0's turn-end, expiring only at the end of P1's next turn.
    // Both players need a deck so the per-turn draw doesn't deck-out (which would
    // end the game and stop turn rotation before P1's turn-end fires).
    let mut runner = st2_builder()
        .add_card(digimon("LOCKME", 3, 3000))
        .add_card(digimon("FILLER", 3, 1000))
        .security(1, &["ST2-14"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "LOCKME", Some(0));

    runner.attack_player(attacker, 1, false);
    for _ in 0..8 {
        if runner.pending_selection().is_none() {
            break;
        }
        runner
            .auto_resolve()
            .expect("resolve Sorrow Blue security target");
    }

    assert!(
        runner.game.modifiers.has(attacker, ModifierType::CannotAttack),
        "Sorrow Blue [Security] locks the chosen no-source opponent Digimon"
    );

    // The attacker's (P0's) turn ends — the lock must NOT expire here.
    runner.end_turn();
    assert!(
        runner.game.modifiers.has(attacker, ModifierType::CannotAttack),
        "lock must survive the attacker's turn-end (it lasts until the END of the \
         security owner's NEXT turn, not the attacker's current turn)"
    );

    // The security owner's (P1's) next turn ends — now the lock expires.
    runner.end_turn();
    assert!(
        !runner.game.modifiers.has(attacker, ModifierType::CannotAttack),
        "lock expires at the end of the security owner's next turn"
    );
}

#[test]
fn st2_15_kaiser_nail_plays_selected_digimon_source_for_free() {
    let mut runner = st2_builder().hand(0, &["ST2-15"]).memory(0).start();
    let host = runner.place_stack(0, &["ST2-03", "ST2-05"]);
    let battle_before = runner.game.player(0).battle_area.len();

    assert!(runner.game.activate_hand_main(0, 0));
    choose_first_pending(&mut runner);

    let source_action = encode_source_select(host.index as u16, 0).expect("source action");
    {
        let pending = runner
            .pending_selection()
            .expect("Kaiser Nail source prompt");
        assert!(pending.valid_action_ids.contains(&source_action));
    }
    runner
        .execute_action(0, source_action)
        .expect("source selection resolves");
    runner.game.drain_effect_queue();

    assert_eq!(source_ids(&runner, 0, host.index as usize), vec!["ST2-05"]);
    assert_eq!(runner.game.player(0).battle_area.len(), battle_before + 1);
    assert!(top_ids(&runner, 0).contains(&"ST2-03".to_string()));
    assert_eq!(runner.memory(), 0, "source play is free");
}

/// Real-path coverage: drive ST2-16 through `play_option_from_hand` — the
/// exact entry the action decoder uses for an Option play (decode.rs routes
/// `CardKind::Option` here). This pays the use cost, enforces the Option
/// color requirement (rule 4-19-2), resolves the `OptionMain` body, and
/// disposes the Option — none of which the body-only `activate_hand_main`
/// audit test below exercises. With a matching-color Digimon in play and an
/// opponent Digimon to target, the bounce resolves end-to-end.
#[test]
fn st2_16_play_option_from_hand_bounces_opponent_digimon() {
    use digimon_engine::enums::{CardColor, GamePhase};

    // P0 controls a BLUE Digimon so the Option color requirement (4-19-2)
    // is satisfied — the realistic case in a mono-Blue deck.
    let mut blue = digimon("BLUEMON", 4, 4000);
    blue.colors = vec![CardColor::Blue];

    let mut runner = st2_builder()
        .add_card(blue)
        .hand(0, &["ST2-16"])
        .memory(9)
        .start();
    runner.set_phase(GamePhase::Main);
    runner.place_on_field(0, "BLUEMON", Some(0));
    runner.place_stack(1, &["ST2-02", "ST2-05"]);

    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "precondition: opponent has 1 Digimon"
    );

    // Real play path returns Pending — an opponent-Digimon selection surfaces.
    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Pending),
        "play_option_from_hand must surface a target selection; got {result:?}"
    );
    assert!(
        runner.pending_selection().is_some(),
        "an opponent-Digimon target selection must be pending"
    );
    choose_first_pending(&mut runner);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.player(1).battle_area.is_empty(),
        "opponent Digimon must be returned to hand via the real play path"
    );
    assert!(hand_ids(&runner, 1).contains(&"ST2-05".to_string()));
}

/// Real-path guard for the Option color requirement (rule 4-19-2): with NO
/// matching-color permanent in play, the Option play is rejected outright —
/// it never reaches the body. (From security the requirement is ignored,
/// 4-19-5, but that is the OnSecurity path, not this one.)
#[test]
fn st2_16_play_from_hand_rejected_without_matching_color_in_play() {
    use digimon_engine::enums::GamePhase;
    use digimon_engine::selection::OptionPlayResult;

    let mut runner = st2_builder().hand(0, &["ST2-16"]).memory(9).start();
    runner.set_phase(GamePhase::Main);
    runner.place_stack(1, &["ST2-02", "ST2-05"]);

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, OptionPlayResult::Invalid),
        "no Blue Digimon/Tamer in play → Option play is illegal (4-19-2); got {result:?}"
    );
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "rejected play must not bounce anything"
    );
}

#[test]
fn st2_16_cocytus_breath_returns_opponent_digimon_to_hand() {
    let mut runner = st2_builder().hand(0, &["ST2-16"]).start();
    runner.place_stack(1, &["ST2-02", "ST2-05"]);

    assert!(runner.game.activate_hand_main(0, 0));
    choose_first_pending(&mut runner);

    assert!(runner.game.player(1).battle_area.is_empty());
    assert!(hand_ids(&runner, 1).contains(&"ST2-05".to_string()));
    assert_eq!(
        trash_ids(&runner, 1),
        vec!["ST2-02"],
        "return-to-hand routes digivolution sources through normal trash rules"
    );
}

#[test]
fn st2_option_and_tamer_structural_shapes_match_printed_clauses() {
    let sorrow = compiled("ST2-14");
    assert_eq!(
        triggered(&sorrow, CompiledTiming::MainFromHand)
            .process
            .len(),
        3
    );
    assert_eq!(
        triggered(&sorrow, CompiledTiming::OnSecurity).process.len(),
        3
    );

    let kaiser = compiled("ST2-15");
    for timing in [CompiledTiming::MainFromHand, CompiledTiming::OnSecurity] {
        let clause = triggered(&kaiser, timing);
        assert!(clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectMaterial { .. })));
        assert!(clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayFromMaterials { .. })));
    }

    let breath = compiled("ST2-16");
    for timing in [CompiledTiming::MainFromHand, CompiledTiming::OnSecurity] {
        assert!(triggered(&breath, timing)
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::ReturnToHand { .. })));
    }
}

#[test]
fn st2_inherited_and_triggered_shapes_use_expected_substrate() {
    let tsunomon = compiled("ST2-01");
    assert!(tsunomon.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            active_when: Some(active),
            dp_modifier: Some(1000),
            ..
        }) if active
            .all_of
            .iter()
            .any(|p| p.battle_opponent_no_sources == Some(true))
    )));

    for card_id in ["ST2-03", "ST2-06", "ST2-09"] {
        let card = compiled(card_id);
        let timing = if card_id == "ST2-09" {
            CompiledTiming::WhenDigivolving
        } else {
            CompiledTiming::WhenAttacking
        };
        assert!(triggered(&card, timing)
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::TrashBottomSources { .. })));
    }

    let weregarurumon = compiled("ST2-08");
    assert!(weregarurumon.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            grant_keyword: Some(keyword),
            active_when: Some(active),
            ..
        }) if keyword.keyword == "SecurityAttackPlus"
            && keyword.value == Some(1)
            && active
                .all_of
                .iter()
                .any(|p| p.any_permanent.is_some())
    )));

    let grizzlymon = compiled("ST2-07");
    assert!(grizzlymon.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "Blocker"
    )));
    assert!(matches!(
        triggered(&grizzlymon, CompiledTiming::WhenAttacking).process[0],
        CompiledStep::LoseMemory(2)
    ));
}

#[test]
fn st2_starter_deck_artifact_counts_validate_and_registry_is_complete() {
    let expected = official_counts();
    let deck = st2_deck_library_ids();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for id in &deck {
        *counts.entry(id.as_str()).or_default() += 1;
    }

    assert_eq!(counts, expected);
    assert_eq!(counts.values().sum::<usize>(), 54);
    assert_eq!(counts.len(), 16);

    let parsed = digimon_engine::deck_tools::classify_parsed(deck.clone());
    assert_eq!(parsed.egg_deck.len(), 4);
    assert_eq!(parsed.main_deck.len(), 50);

    let validation =
        digimon_engine::deck_tools::validate_deck_for_game_mode(&deck, "no_restriction")
            .expect("no_restriction mode is supported");
    assert!(
        validation.is_valid,
        "ST2 no-restriction starter artifact must validate: {:?}",
        validation.errors
    );

    let registry = digimon_engine::dsl_registry::from_embedded().expect("embedded registry loads");
    for card_id in ST2_IDS {
        assert!(
            registry.lookup(card_id).is_some(),
            "{card_id} must be present in the implemented DSL registry"
        );
    }
}
