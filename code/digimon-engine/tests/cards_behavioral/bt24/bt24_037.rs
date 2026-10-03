//! BT24-037 Silphymon — Track D effect-created attack fixture.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{decode_attack, encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt24/BT24-037.yaml");

fn compiled_bt24_037() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT24-037.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-037.yaml compiles");
    registry
        .lookup("BT24-037")
        .expect("BT24-037 in registry")
        .clone()
}

fn silphymon_runner() -> DebugRunner {
    let mut ally = make_test_card("ALLY", "Ally");
    ally.card_kind = CardKind::Digimon;
    ally.level = Some(4);
    ally.dp = Some(5000);

    let mut defender = make_test_card("DEF", "Defender");
    defender.card_kind = CardKind::Digimon;
    defender.level = Some(4);
    defender.dp = Some(7000);

    let mut security = make_test_card("SEC", "Security");
    security.card_kind = CardKind::Digimon;
    security.dp = Some(1000);

    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-037 YAML loads")
        .add_card(ally)
        .add_card(defender)
        .add_card(security)
        .security(1, &["SEC"])
        .memory(10)
        .start()
}

#[test]
fn bt24_037_on_play_when_digivolving_clause_contains_may_attack_now() {
    let compiled = compiled_bt24_037();
    let clause = compiled
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
        .expect("BT24-037 must have a shared OnPlay/WhenDigivolving clause");

    let has_may_attack = clause.process.iter().any(|step| match step {
        CompiledStep::MayAttackNow { optional, .. } => *optional,
        CompiledStep::Optional(body) => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::MayAttackNow { optional: true, .. })),
        _ => false,
    });

    assert!(
        has_may_attack,
        "printed 'Then, 1 of your Digimon may attack' must lower to may_attack_now"
    );
}

#[test]
fn bt24_037_after_dp_down_one_digimon_may_attack() {
    let mut runner = silphymon_runner();
    let silphy = runner.place_on_field(0, "BT24-037", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let defender = runner.place_on_field(1, "DEF", Some(0));
    runner.game.turn_count = 1;
    let security_before = runner.security_count(1);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(silphy));
    runner.game.drain_effect_queue();

    let debuff_prompt = runner
        .pending_selection_view()
        .expect("On Play should select an opposing Digimon for -5000 DP");
    assert_eq!(debuff_prompt.kind, SelectionKind::OppField);
    runner
        .game
        .resolve_selection(
            debuff_prompt.selecting_player,
            debuff_prompt.valid_action_ids[0],
        )
        .expect("debuff target resolves");

    assert_eq!(
        runner.effective_dp(defender),
        Some(2000),
        "selected opponent Digimon should get -5000 DP for the turn"
    );

    let attacker_prompt = runner
        .pending_selection_view()
        .expect("post-debuff may-attack should select one own attacker");
    assert_eq!(attacker_prompt.kind, SelectionKind::OwnField);
    assert!(
        attacker_prompt.is_optional,
        "printed may-attack attacker choice must be declineable"
    );
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "attacker selection must expose PASS"
    );
    let ally_action = encode_attack(0, ally.index as u16);
    assert!(
        attacker_prompt.valid_action_ids.contains(&ally_action),
        "the established ally should be available as the may-attack attacker"
    );
    assert!(
        runner.game.can_attack(ally, false),
        "test setup should make the selected ally naturally attack-legal"
    );
    runner
        .game
        .resolve_selection(attacker_prompt.selecting_player, ally_action)
        .expect("attacker choice resolves");

    let attack_prompt = runner
        .pending_selection_view()
        .expect("chosen Digimon should open the normal attack target prompt");
    assert!(
        attack_prompt.is_optional,
        "may_attack_now target prompt must remain declineable before commitment"
    );
    let security_attack = attack_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| {
            let (_, target) = decode_attack(*action);
            target == SECURITY_TARGET
        })
        .expect("normal attack flow should allow attacking the opponent player");
    runner
        .game
        .resolve_selection(attack_prompt.selecting_player, security_attack)
        .expect("effect-created attack resolves");

    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "effect-created attack should use the normal security flow"
    );
    assert!(
        runner.game.players[0].battle_area[ally.index as usize].is_suspended,
        "chosen attacker should pay the normal suspend cost"
    );
}

#[test]
fn bt24_037_dna_origin_grants_security_attack_and_dp_until_turn_end() {
    let mut yellow_lv4 = make_test_card("YELLOW-LV4", "Yellow Material");
    yellow_lv4.card_kind = CardKind::Digimon;
    yellow_lv4.level = Some(4);
    yellow_lv4.colors = vec![CardColor::Yellow];
    yellow_lv4.dp = Some(5000);

    let mut red_lv4 = make_test_card("RED-LV4", "Red Material");
    red_lv4.card_kind = CardKind::Digimon;
    red_lv4.level = Some(4);
    red_lv4.colors = vec![CardColor::Red];
    red_lv4.dp = Some(5000);

    let mut defender = make_test_card("DEF", "Defender");
    defender.card_kind = CardKind::Digimon;
    defender.level = Some(4);
    defender.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-037 YAML loads")
        .add_card(yellow_lv4)
        .add_card(red_lv4)
        .add_card(defender)
        .hand(0, &["BT24-037"])
        .memory(10)
        .start();

    let material_a = runner.place_on_field(0, "YELLOW-LV4", Some(0));
    let material_b = runner.place_on_field(0, "RED-LV4", Some(0));
    let defender = runner.place_on_field(1, "DEF", Some(0));
    let hand_card = runner.game.player(0).hand[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }

    let mut saw_debuff = false;
    let mut saw_may_attack_decline = false;
    let mut saw_dna_boost = false;
    while let Some(prompt) = runner.pending_selection_view() {
        match (prompt.kind, prompt.is_optional) {
            (SelectionKind::TriggerOrder, _) => {
                runner
                    .game
                    .resolve_selection(prompt.selecting_player, prompt.valid_action_ids[0])
                    .expect("choose next BT24-037 triggered effect");
            }
            (SelectionKind::OppField, _) => {
                runner
                    .game
                    .resolve_selection(prompt.selecting_player, prompt.valid_action_ids[0])
                    .expect("debuff target resolves");
                saw_debuff = true;
            }
            (SelectionKind::OwnField, true) => {
                runner
                    .game
                    .resolve_selection(prompt.selecting_player, PASS)
                    .expect("decline the may-attack branch");
                saw_may_attack_decline = true;
            }
            (SelectionKind::OwnField, false) => {
                runner
                    .game
                    .resolve_selection(prompt.selecting_player, prompt.valid_action_ids[0])
                    .expect("boost target resolves");
                saw_dna_boost = true;
            }
            _ => panic!("unexpected BT24-037 prompt during DNA-origin flow: {prompt:?}"),
        }
    }

    assert!(saw_debuff, "shared When Digivolving debuff should resolve");
    assert!(
        saw_may_attack_decline,
        "shared may-attack branch should remain optional during DNA digivolve"
    );
    assert!(saw_dna_boost, "DNA-origin rider should resolve");
    assert_eq!(
        runner.effective_dp(defender),
        Some(2000),
        "shared When Digivolving clause should still apply during DNA digivolve"
    );

    let evolved = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        runner.effective_dp(evolved),
        Some(13000),
        "DNA-origin rider should add +5000 DP to the selected Digimon"
    );
    assert!(
        runner
            .game
            .has_keyword(evolved, Keyword::SecurityAttackPlus(1)),
        "DNA-origin rider should grant Security A. +1"
    );

    runner.end_turn();
    assert_eq!(
        runner.effective_dp(evolved),
        Some(8000),
        "DNA-origin DP boost should expire at end of turn"
    );
    assert!(
        !runner
            .game
            .has_keyword(evolved, Keyword::SecurityAttackPlus(1)),
        "DNA-origin Security A. +1 should expire at end of turn"
    );
}

// ─── G-DSL-CAN-ATTACK-PREDICATE (2026-10-01) ─────────────────────────────────
//
// DCGO BT24_037.cs `CanSelectAttackPermanentCondition`: only a Digimon with
// `permanent.CanAttack(activateClass)` is offered as the attacker, and the
// attacker select is skipped entirely when none can attack.

fn can_attack_debuff_then_attacker_prompt(runner: &mut DebugRunner, silphy: PermanentHandleAlias) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(silphy));
    runner.game.drain_effect_queue();
    let debuff_prompt = runner
        .pending_selection_view()
        .expect("On Play selects an opposing Digimon for -5000 DP");
    assert_eq!(debuff_prompt.kind, SelectionKind::OppField);
    runner
        .game
        .resolve_selection(debuff_prompt.selecting_player, debuff_prompt.valid_action_ids[0])
        .expect("debuff target resolves");
}

type PermanentHandleAlias = digimon_engine::permanent::PermanentHandle;

#[test]
fn bt24_037_attacker_pick_offers_only_digimon_that_can_attack() {
    let mut runner = silphymon_runner();
    runner.game.turn_count = 1;
    // Silphymon was just played (summoning sickness, no <Rush>).
    let silphy = runner.place_on_field(0, "BT24-037", None);
    let suspended = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.players[0].battle_area[suspended.index as usize].is_suspended = true;
    let locked = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.modifiers.add(
        locked,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotAttack,
            1,
            digimon_engine::enums::Expiry::EndOfTurn,
            1,
        ),
    );
    let ready = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "DEF", Some(0));

    can_attack_debuff_then_attacker_prompt(&mut runner, silphy);
    let prompt = runner
        .pending_selection_view()
        .expect("one Digimon can attack, so the attacker prompt is shown");
    assert_eq!(prompt.kind, SelectionKind::OwnField);
    let offered: Vec<u16> = prompt
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        offered,
        vec![encode_attack(0, ready.index as u16)],
        "summoning-sick, suspended and CannotAttack Digimon are not offered"
    );
}

#[test]
fn bt24_037_no_attacker_prompt_when_no_digimon_can_attack() {
    let mut runner = silphymon_runner();
    runner.game.turn_count = 1;
    let silphy = runner.place_on_field(0, "BT24-037", None);
    let suspended = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.players[0].battle_area[suspended.index as usize].is_suspended = true;
    runner.place_on_field(1, "DEF", Some(0));

    can_attack_debuff_then_attacker_prompt(&mut runner, silphy);
    assert!(
        runner.pending_selection_view().is_none(),
        "no Digimon can attack ⇒ no (dead) attacker prompt"
    );
}

// ─── Colour-gated standard-circle alt path (W1 regression guard) ────────────
//
// The `kind: digivolve` alt path mirroring the printed "Yellow Lv.4" circle
// used to say `from: { level_eq: 4, color: yellow }`. `color` is not a
// DSL predicate key, so it was silently dropped and the path accepted a
// level-4 base of ANY colour. Now `color_is` — a wrong-colour base must be
// rejected; a right-colour base still digivolves.

fn bt24_037_color_gate_base(id: &str, color: digimon_engine::enums::CardColor) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, "Wrongcolormon");
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = Vec::new();
    c
}

/// Try to digivolve BT24-037 (from hand) onto a level-4 base of `color`.
fn bt24_037_color_gate_try(color: digimon_engine::enums::CardColor) -> bool {
    let mut r = digimon_engine::debug_runner::DebugRunner::builder()
        .dsl_card("BT24-037")
        .expect("BT24-037 in embedded DSL pack")
        .add_card(bt24_037_color_gate_base("CG-BASE", color))
        .add_card(digimon_engine::debug_runner::make_test_card("CG-FILL", "CG-FILL"))
        .hand(0, &["BT24-037"])
        .deck(0, &["CG-FILL"; 5])
        .deck(1, &["CG-FILL"])
        .memory(10)
        .start();
    let base = r.place_on_field(0, "CG-BASE", Some(0));
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT24-037")
        .expect("BT24-037 in hand");
    r.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    )
}

#[test]
fn bt24_037_standard_alt_path_rejects_wrong_color_base() {
    assert!(
        !bt24_037_color_gate_try(digimon_engine::enums::CardColor::Blue),
        "a blue level-4 base must NOT digivolve into BT24-037 via the Yellow Lv.4 alt path"
    );
}

#[test]
fn bt24_037_standard_alt_path_accepts_right_color_base() {
    assert!(
        bt24_037_color_gate_try(digimon_engine::enums::CardColor::Yellow),
        "a yellow level-4 base digivolves into BT24-037 via the Yellow Lv.4 alt path"
    );
}

// ─── Colour-gated DNA materials (W1 regression guard) ───────────────────────
//
// The printed "[DNA Digivolve] Yellow Lv.4 + red/green Lv.4" alt path's materials used
// `color:` — not a DSL predicate key — so it was silently dropped and ANY two
// level-4 Digimon formed a legal DNA pair. Now `color_is`.

fn bt24_037_dna_mat(id: &str, color: digimon_engine::enums::CardColor) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.traits = Vec::new();
    c
}

fn bt24_037_dna_route(a: digimon_engine::enums::CardColor, b: digimon_engine::enums::CardColor) -> bool {
    let mut r = digimon_engine::debug_runner::DebugRunner::builder()
        .dsl_card("BT24-037")
        .expect("BT24-037 in embedded DSL pack")
        .add_card(bt24_037_dna_mat("DNA-A", a))
        .add_card(bt24_037_dna_mat("DNA-B", b))
        .add_card(digimon_engine::debug_runner::make_test_card("DNA-FILL", "DNA-FILL"))
        .hand(0, &["BT24-037"])
        .deck(0, &["DNA-FILL"; 5])
        .deck(1, &["DNA-FILL"])
        .memory(10)
        .start();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    r.place_on_field(0, "DNA-A", Some(0));
    r.place_on_field(0, "DNA-B", Some(0));
    r.game.tick_declarative_effects();
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT24-037")
        .expect("BT24-037 in hand");
    r.game.has_valid_dna_route_for_hand_card(0, hand_idx)
}

#[test]
fn bt24_037_dna_rejects_two_blue_level4_materials() {
    use digimon_engine::enums::CardColor;
    assert!(
        !bt24_037_dna_route(CardColor::Blue, CardColor::Blue),
        "two blue Lv.4 Digimon are not a legal Yellow Lv.4 + red/green Lv.4 DNA pair"
    );
}

#[test]
fn bt24_037_dna_accepts_printed_color_pair() {
    use digimon_engine::enums::CardColor;
    assert!(
        bt24_037_dna_route(CardColor::Yellow, CardColor::Green),
        "a Yellow Lv.4 + Green Lv.4 pair is a legal Yellow Lv.4 + red/green Lv.4 DNA pair"
    );
}
