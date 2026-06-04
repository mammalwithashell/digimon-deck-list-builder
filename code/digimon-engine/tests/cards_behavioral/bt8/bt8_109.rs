//! BT8-109 Flame Hellscythe — Option, Purple/Yellow, Cost 6.
//!
//! # Card text (BT8-109.json)
//! [Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, you
//! may play 1 purple or yellow Digimon card with 6000 DP or less from your
//! trash without paying its memory cost.
//! Security Effect [Security] Activate this card's [Main] effect.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Purple/BT8_109.cs
//!   - [Main]: mandatory select opp Digimon → -6000 DP UntilEachTurnEnd; THEN
//!     (independently, no "if you do" gate) optional free-play of a trash
//!     Digimon with (Purple OR Yellow) AND DP<=6000. activateETB: true → the
//!     played card's [On Play] DOES fire (NOT suppressed, unlike BT9-108).
//!   - [Security] re-runs [Main].
//!
//! # Patterns: D (targeted -DP debuff), A4 (trash → free play), E2 (optional
//! decline on trash-play), SECURITY-MIRROR. Q6: a -6000 drop to <=0 DP deletes
//! the target via the state-based <=0-DP rules-check AFTER the option resolves,
//! NOT mid-effect.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Card-data helpers ──────────────────────────────────────────────────────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// A purple Lv.3 with an [On Play] draw — observable proof the played card's
/// On Play DOES fire (BT8-109 does NOT suppress, unlike BT9-108).
const ONPLAY_PURPLE_YAML: &str = r#"
card: ONPLAY-PURPLE
name: OnPlay Purple
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 2000
effects:
  - when: on_play
    summary: "[On Play] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let card = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap_or_else(|| panic!("seed_trash: unknown card_id {card_id}"));
        let next_idx = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(data_idx, player as u8, next_idx)
    };
    runner.game.players[player].trash.push(card);
}

/// BT8-109 in P0's hand; trash-play candidates + an opponent Digimon registered.
fn hellscythe_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT8-109")
        .expect("BT8-109 YAML parses and compiles")
        .from_dsl_yaml(ONPLAY_PURPLE_YAML)
        .expect("ONPLAY-PURPLE inline DSL compiles")
        .add_card(digimon("PURPLE-6000", CardColor::Purple, 5, 6000)) // legal trash-play
        .add_card(digimon("YELLOW-5000", CardColor::Yellow, 4, 5000)) // legal trash-play
        .add_card(digimon("PURPLE-7000", CardColor::Purple, 6, 7000)) // DP too high — filtered
        .add_card(digimon("GREEN-3000", CardColor::Green, 3, 3000))   // wrong color — filtered
        .add_card(digimon("OPP-BIG", CardColor::Blue, 6, 11000))      // survives -6000
        .add_card(digimon("OPP-SMALL", CardColor::Blue, 4, 5000))     // dies to -6000 (Q6)
        .hand(0, &["BT8-109"])
        .deck(0, &["PURPLE-6000", "PURPLE-6000", "PURPLE-6000"])
        .memory(10)
        .start()
}

// ─── Section 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt8_109_is_purple_yellow_option_cost_6() {
    let runner = hellscythe_runner();
    let c = runner.compiled_card("BT8-109").expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::Option);
    assert_eq!(c.cost, Some(6));
    assert!(c.color.contains(&CompiledColor::Purple));
    assert!(c.color.contains(&CompiledColor::Yellow));
}

#[test]
fn bt8_109_has_main_and_security_mirror_clauses() {
    let runner = hellscythe_runner();
    let c = runner.compiled_card("BT8-109").expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "[Main] + [Security] mirror");
    let main = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main] clause");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    let sec = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security] clause");
    assert_eq!(sec.scope, CompiledScope::Inherited);
}

// ─── Section 2 — [Main] -6000 DP ────────────────────────────────────────────

#[test]
fn bt8_109_main_prompts_to_target_an_opponent_digimon() {
    let mut runner = hellscythe_runner();
    runner.place_on_field(1, "OPP-BIG", Some(0));

    runner.play(0, 0).expect("play BT8-109");

    let sel = runner
        .pending_selection_view()
        .expect("[Main] must prompt for an opponent Digimon target");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert_eq!(sel.valid_action_ids.len(), 1);
}

#[test]
fn bt8_109_main_applies_minus_6000_for_the_turn() {
    let mut runner = hellscythe_runner();
    let opp = runner.place_on_field(1, "OPP-BIG", Some(0)); // 11000
    assert_eq!(runner.effective_dp(opp), Some(11000));

    runner.play(0, 0).expect("play BT8-109");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("target the opponent Digimon");
    runner.auto_resolve().expect("resolve");

    let opp = runner.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-BIG")
        .map(|i| digimon_engine::permanent::PermanentHandle { player: 1, index: i as u8 })
        .expect("OPP-BIG still on field (11000 - 6000 = 5000 > 0)");
    assert_eq!(
        runner.effective_dp(opp),
        Some(5000),
        "11000 - 6000 = 5000 for the turn"
    );
}

/// Q6: a -6000 reduction that drops the target to <=0 DP deletes it via the
/// state-based rules-check AFTER the option resolves (not mid-effect). The
/// observable end-state is that the small Digimon is gone once the option fully
/// resolves.
#[test]
fn bt8_109_main_minus_6000_deletes_small_digimon_via_rules_check() {
    let mut runner = hellscythe_runner();
    let opp = runner.place_on_field(1, "OPP-SMALL", Some(0)); // 5000

    runner.play(0, 0).expect("play BT8-109");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("target the small opponent Digimon");
    runner.auto_resolve().expect("resolve + state-based rules check");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "5000 - 6000 = -1000 DP → deleted by the state-based <=0-DP rules-check"
    );
}

#[test]
fn bt8_109_main_no_target_prompt_when_opponent_has_no_digimon() {
    let mut runner = hellscythe_runner();
    seed_trash(&mut runner, 0, "PURPLE-6000"); // ensure the trash-play arm still has a candidate

    runner.play(0, 0).expect("play BT8-109");

    // No opponent Digimon → the mandatory opp-target select installs nothing and
    // the effect proceeds straight to the (independent) optional trash-play.
    let sel = runner
        .pending_selection_view()
        .expect("with no opp Digimon, the effect proceeds to the optional trash-play");
    assert_eq!(
        sel.kind,
        SelectionKind::Trash,
        "no opp-target step (no candidates) → straight to the trash-play prompt"
    );
}

// ─── Section 3 — [Main] trash-play (purple/yellow, DP<=6000) ─────────────────

#[test]
fn bt8_109_main_trash_play_offers_only_purple_or_yellow_dp6000_or_less() {
    let mut runner = hellscythe_runner();
    let opp = runner.place_on_field(1, "OPP-BIG", Some(0));
    seed_trash(&mut runner, 0, "PURPLE-6000"); // legal
    seed_trash(&mut runner, 0, "YELLOW-5000"); // legal
    seed_trash(&mut runner, 0, "PURPLE-7000"); // DP too high
    seed_trash(&mut runner, 0, "GREEN-3000");  // wrong color

    runner.play(0, 0).expect("play BT8-109");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("apply -6000");

    let sel = runner
        .pending_selection_view()
        .expect("trash-play prompt installs");
    assert_eq!(sel.kind, SelectionKind::Trash);
    assert!(sel.is_optional, "'you may play' → optional");
    assert_eq!(
        sel.valid_action_ids.len(),
        2,
        "only the purple-6000 and yellow-5000 cards qualify (purple-7000 too big, green wrong color)"
    );
}

#[test]
fn bt8_109_main_decline_trash_play_plays_nothing() {
    let mut runner = hellscythe_runner();
    let opp = runner.place_on_field(1, "OPP-BIG", Some(0));
    seed_trash(&mut runner, 0, "PURPLE-6000");

    runner.play(0, 0).expect("play BT8-109");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("apply -6000");
    let battle_before = runner.battle_area_size(0);
    runner.execute_action(0, PASS).expect("decline trash-play");

    assert!(runner.pending_selection().is_none(), "effect ends on decline");
    assert_eq!(runner.battle_area_size(0), battle_before, "nothing played");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PURPLE-6000"),
        "the declined card stays in trash"
    );
}

#[test]
fn bt8_109_main_plays_chosen_card_from_trash_free() {
    let mut runner = hellscythe_runner();
    let opp = runner.place_on_field(1, "OPP-BIG", Some(0));
    seed_trash(&mut runner, 0, "YELLOW-5000");

    runner.play(0, 0).expect("play BT8-109");
    let mem_after_option = runner.memory();
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("apply -6000");

    let battle_before = runner.battle_area_size(0);
    let sel = runner.pending_selection_view().expect("trash-play prompt");
    runner
        .execute_action(0, sel.valid_action_ids[0])
        .expect("play yellow-5000 from trash free");
    runner.auto_resolve().expect("resolve the free play");

    assert_eq!(runner.battle_area_size(0), battle_before + 1, "card entered field");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "YELLOW-5000"),
        "the yellow Lv.4 from trash is on the field"
    );
    assert_eq!(runner.memory(), mem_after_option, "the trash play costs no memory");
}

/// BT8-109 does NOT suppress the played card's [On Play] (DCGO activateETB:
/// true) — unlike BT9-108. Proof: an [On Play]-draw card played from trash DOES
/// draw.
#[test]
fn bt8_109_main_played_card_on_play_fires() {
    let mut runner = hellscythe_runner();
    let opp = runner.place_on_field(1, "OPP-BIG", Some(0));
    seed_trash(&mut runner, 0, "ONPLAY-PURPLE");

    runner.play(0, 0).expect("play BT8-109");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("apply -6000");

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let sel = runner.pending_selection_view().expect("trash-play prompt");
    runner
        .execute_action(0, sel.valid_action_ids[0])
        .expect("play the On-Play purple from trash");
    runner.auto_resolve().expect("resolve, incl. the played card's On Play");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the played card's [On Play] draw DOES fire (no suppression)"
    );
    assert_eq!(runner.deck_size(0), deck_before - 1);
}

// ─── Section 4 — [Security] mirror ──────────────────────────────────────────

#[test]
fn bt8_109_security_mirror_runs_main_effect() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-109")
        .expect("BT8-109 compiles")
        .add_card(attacker)
        .add_card(digimon("OPP-BIG", CardColor::Blue, 6, 11000))
        .security(1, &["BT8-109"])
        .deck(1, &["ATTACKER", "ATTACKER", "ATTACKER"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    // An unsuspended P0 bystander Digimon = the mirror's -6000 target (the
    // attacker suspends on declaration; any P0 Digimon is a legal target since
    // BT8-109 targets "your opponent's Digimon" from P1's perspective).
    let bystander = runner.place_on_field(0, "OPP-BIG", Some(0));

    runner.attack_player(attacker, 1, false);

    let sel = runner
        .pending_selection_view()
        .expect("BT8-109 [Security] mirror installs the -6000 target prompt");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert_eq!(
        sel.selecting_player, 1,
        "the security owner controls the mirrored [Main] effect"
    );
    // bystander is a legal target.
    assert!(sel
        .valid_action_ids
        .contains(&encode_attack(0, bystander.index as u16)));
}
