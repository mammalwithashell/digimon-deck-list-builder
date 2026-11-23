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

// ─── [All Turns][Once Per Turn] would-leave play (main + inherited) ──────────
//
// Silphymon (BT24-037) prints the same clause twice, once as its own effect
// and once as its inherited effect (card face, official bundle, DCGO
// BT24_037.cs "All Turns - OPT" + "All Turns - OPT - ESS", two hash keys):
//   "[All Turns] [Once Per Turn] When this Digimon would leave the battle area
//    other than by your effects, you may play 1 level 4 or lower yellow, red
//    or [TS] trait Digimon card from its digivolution cards without paying
//    the cost."
// DCGO: WhenRemoveField, optional, NOT IsByEffect(own owner); candidate =
// IsDigimon && Level <= 4 && (Yellow || Red || HasTSTraits) &&
// CanPlayAsNewPermanent(payCost false). The play does not stop the leave.

mod would_leave_play {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
    use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
    use digimon_engine::card_data::CardData;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::replacement::ReplacementCause;

    const ID: &str = "BT24-037";

    fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![color];
        c.level = Some(level);
        c.dp = Some(dp);
        c.play_cost = 5;
        c.traits = traits.iter().map(|t| t.to_string()).collect();
        c
    }

    fn tamer(id: &str, color: CardColor) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.colors = vec![color];
        c.level = None;
        c.dp = None;
        c
    }

    fn runner() -> DebugRunner {
        DebugRunner::builder()
            .dsl_card(ID)
            .expect("BT24-037 YAML")
            .dsl_card("BT24-040")
            .expect("BT24-040 YAML")
            .add_card(digimon("Y4", CardColor::Yellow, 4, 5000, &[]))
            .add_card(digimon("R4", CardColor::Red, 4, 5000, &[]))
            .add_card(digimon("TS4", CardColor::Blue, 4, 5000, &["TS"]))
            .add_card(digimon("Y3", CardColor::Yellow, 3, 4000, &[]))
            .add_card(digimon("B4", CardColor::Blue, 4, 5000, &[]))
            .add_card(digimon("Y5", CardColor::Yellow, 5, 7000, &[]))
            .add_card(digimon("WEAK", CardColor::Yellow, 4, 3000, &[]))
            .add_card(digimon("TOP", CardColor::Blue, 6, 12000, &[]))
            .add_card(digimon("TSTOP", CardColor::Blue, 6, 12000, &["TS"]))
            .add_card(digimon("BODY-A", CardColor::Blue, 3, 3000, &[]))
            .add_card(digimon("BODY-B", CardColor::Blue, 3, 3000, &[]))
            .add_card(tamer("YT", CardColor::Yellow))
            .add_card(make_test_card("PAD", "Pad"))
            .deck(0, &["PAD"; 10])
            .deck(1, &["PAD"; 10])
            .memory(5)
            .start()
    }

    fn on_field(r: &DebugRunner, player: usize, id: &str) -> bool {
        r.game.players[player]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == id)
    }

    fn non_pass(r: &DebugRunner) -> Vec<u16> {
        r.pending_selection()
            .expect("a pending choice")
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != PASS && a != REPLACEMENT_ACCEPT)
            .collect()
    }

    fn accept(r: &mut DebugRunner) {
        assert!(r.pending_is_optional(), "the printed 'you may' is a yes/no");
        r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    }

    fn pick_nth(r: &mut DebugRunner, n: usize) {
        let a = non_pass(r)[n];
        let p = r.pending_selection().unwrap().selecting_player;
        r.execute_action(p, a).expect("pick");
    }

    fn played_ids(r: &DebugRunner, candidates: &[&str]) -> Vec<String> {
        candidates
            .iter()
            .filter(|id| on_field(r, 0, id))
            .map(|s| s.to_string())
            .collect()
    }

    const ALL: [&str; 8] = ["Y4", "R4", "TS4", "Y3", "B4", "Y5", "WEAK", "YT"];

    #[test]
    fn bt24_037_prints_two_once_per_turn_would_leave_clauses() {
        let r = runner();
        let card = r.compiled_card(ID).expect("compiled BT24-037");
        let mut scopes: Vec<String> = card
            .effects
            .iter()
            .filter_map(|c| match c {
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                    trigger,
                    scope,
                    optional,
                    once_per_turn,
                    ..
                }) if trigger == "when_would_leave_battle_area" => {
                    assert!(*optional, "'you may play'");
                    assert!(*once_per_turn, "[Once Per Turn]");
                    Some(format!("{scope:?}"))
                }
                _ => None,
            })
            .collect();
        scopes.sort();
        let mut want = vec![
            format!("{:?}", CompiledScope::Inherited),
            format!("{:?}", CompiledScope::FaceUp),
        ];
        want.sort();
        assert_eq!(scopes, want, "one own and one inherited would-leave clause");
    }

    /// The pick offers exactly the level-4-or-lower yellow / red / [TS] Digimon
    /// cards: not a blue untraited Lv.4, not a Lv.5, not a Tamer.
    #[test]
    fn bt24_037_main_offers_level_4_or_lower_yellow_red_or_ts_digimon() {
        let mut played = Vec::new();
        for n in 0..4 {
            let mut r = runner();
            let mut stack = ALL.to_vec();
            stack.retain(|id| *id != "WEAK");
            stack.push(ID);
            let h = r.place_stack(0, &stack);
            let mem = r.memory();
            r.game
                .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
            accept(&mut r);
            assert_eq!(non_pass(&r).len(), 4, "Y4, R4, TS4 and Y3 are the picks");
            pick_nth(&mut r, n);
            r.auto_resolve().unwrap();
            let p = played_ids(&r, &ALL);
            assert_eq!(p.len(), 1, "exactly one card is played: {p:?}");
            played.extend(p);
            assert!(!on_field(&r, 0, ID), "the play does not stop the leave");
            assert_eq!(r.memory(), mem, "without paying the cost");
        }
        played.sort();
        assert_eq!(played, vec!["R4", "TS4", "Y3", "Y4"]);
    }

    #[test]
    fn bt24_037_main_applies_to_battle_and_opponent_effects_not_your_own() {
        for cause in [ReplacementCause::Battle, ReplacementCause::OpponentEffect] {
            let mut r = runner();
            let h = r.place_stack(0, &["Y4", ID]);
            r.game.delete_permanent_with_cause(h, cause);
            accept(&mut r);
            pick_nth(&mut r, 0);
            r.auto_resolve().unwrap();
            assert!(on_field(&r, 0, "Y4"), "{cause:?}");
        }
        let mut r = runner();
        let h = r.place_stack(0, &["Y4", ID]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OwnEffect);
        assert!(r.pending_selection().is_none(), "other than by your effects");
        assert!(!on_field(&r, 0, "Y4"));
        assert!(!on_field(&r, 0, ID));
    }

    #[test]
    fn bt24_037_main_decline_plays_nothing() {
        let mut r = runner();
        let h = r.place_stack(0, &["Y4", ID]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert!(r.pending_is_optional());
        r.execute_action(0, PASS).expect("decline");
        r.auto_resolve().unwrap();
        assert!(!on_field(&r, 0, "Y4"));
        assert!(!on_field(&r, 0, ID));
    }

    /// A card that can't be played (here: a "can't play Digimon with 3000 DP
    /// or less" lock) is never offered; with only such a card the effect is
    /// not offered at all.
    #[test]
    fn bt24_037_main_offers_only_playable_cards() {
        let lock = |r: &mut DebugRunner| {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotPlayDigimonWithDPAtMost,
                    3000,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        };
        let mut r = runner();
        let h = r.place_stack(0, &["WEAK", ID]);
        lock(&mut r);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection().is_none(), "nothing playable, nothing offered");
        assert!(!on_field(&r, 0, "WEAK"));

        let mut r = runner();
        let h = r.place_stack(0, &["WEAK", "Y4", ID]);
        lock(&mut r);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        accept(&mut r);
        assert_eq!(non_pass(&r).len(), 1, "only the playable Y4");
        pick_nth(&mut r, 0);
        r.auto_resolve().unwrap();
        assert!(on_field(&r, 0, "Y4"));
        assert!(!on_field(&r, 0, "WEAK"));
    }

    #[test]
    fn bt24_037_inherited_plays_from_the_carriers_digivolution_cards() {
        let mut r = runner();
        let h = r.place_stack(0, &["R4", "B4", ID, "TOP"]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        accept(&mut r);
        assert_eq!(
            non_pass(&r).len(),
            1,
            "the red Lv.4 (Silphymon itself is Lv.5, the blue Lv.4 has no TS)"
        );
        pick_nth(&mut r, 0);
        r.auto_resolve().unwrap();
        assert!(on_field(&r, 0, "R4"));
        assert!(!on_field(&r, 0, "TOP"), "the carrier still leaves");
    }

    #[test]
    fn bt24_037_inherited_does_not_apply_to_your_own_effects() {
        let mut r = runner();
        let h = r.place_stack(0, &["R4", ID, "TOP"]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OwnEffect);
        assert!(r.pending_selection().is_none());
        assert!(!on_field(&r, 0, "R4"));
    }

    /// A Silphymon on top of another Silphymon has both clauses; they are
    /// separate effects with separate [Once Per Turn] counts (DCGO hash keys
    /// BT24_037_AT and BT24_037_AT_ESS), so both play a card.
    #[test]
    fn bt24_037_main_and_inherited_are_separate_once_per_turn_effects() {
        let mut r = runner();
        let h = r.place_stack(0, &["Y4", "R4", ID, ID]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        // Two would-leave effects apply to one event: the player first
        // chooses which resolves first (an EffectChoice, no auto-order).
        let order = r.pending_selection_view().expect("order choice");
        assert_eq!(
            order.kind,
            digimon_engine::selection::SelectionKind::EffectChoice
        );
        assert_eq!(order.valid_action_ids.len(), 2, "both clauses are offered");
        let mut accepted = 0;
        let mut guard = 0;
        while r.pending_selection().is_some() && guard < 10 {
            guard += 1;
            if r.pending_is_optional() {
                accepted += 1;
                r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
            } else {
                pick_nth(&mut r, 0);
            }
        }
        r.auto_resolve().unwrap();
        assert_eq!(accepted, 2, "each clause asks its own 'you may'");
        assert!(on_field(&r, 0, "Y4"));
        assert!(on_field(&r, 0, "R4"));
        assert!(r.pending_selection().is_none());
    }

    /// [Once Per Turn]: Venusmon (BT24-040) keeps the [TS] Silphymon on the
    /// field the first time, after Silphymon's effect played a card. A second
    /// "would leave" in the same turn offers Silphymon's effect no more.
    fn once_per_turn_case(stack: &[&str]) {
        let mut r = runner();
        r.place_on_field(0, "BT24-040", Some(0));
        let body = r.place_on_field(0, "BODY-A", Some(0));
        r.place_on_field(0, "BODY-B", Some(0));
        let h = r.place_stack(0, stack);
        r.game.return_to_hand_from_effect(h, 1);
        // Both would-leave effects apply; accept each and pick: Venusmon pays
        // with BODY-A, Silphymon plays its first candidate.
        let mut guard = 0;
        while r.pending_selection().is_some() && guard < 8 {
            guard += 1;
            if r.pending_is_optional() {
                r.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
                continue;
            }
            let ids = non_pass(&r);
            let body_action = encode_attack(0, body.index as u16);
            let a = if ids.contains(&body_action) { body_action } else { ids[0] };
            let p = r.pending_selection().unwrap().selecting_player;
            r.execute_action(p, a).expect("pick");
        }
        r.auto_resolve().unwrap();
        assert!(on_field(&r, 0, "Y4"), "Silphymon's effect played Y4");
        assert!(!on_field(&r, 0, "BODY-A"), "Venusmon paid with BODY-A");
        let top = *stack.last().unwrap();
        let now = r.game.players[0]
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&r.game.card_data) == top)
            .expect("Venusmon kept the TS Digimon on the field");
        let h2 = PermanentHandle { player: 0, index: now as u8 };
        r.game.return_to_hand_from_effect(h2, 1);
        assert!(
            r.pending_selection().is_none(),
            "both [Once Per Turn] effects are spent"
        );
        r.auto_resolve().unwrap();
        assert!(!on_field(&r, 0, "R4"), "no second play this turn");
    }

    #[test]
    fn bt24_037_main_is_once_per_turn() {
        once_per_turn_case(&["Y4", "R4", ID]);
    }

    #[test]
    fn bt24_037_inherited_is_once_per_turn() {
        once_per_turn_case(&["Y4", "R4", ID, "TSTOP"]);
    }
}
