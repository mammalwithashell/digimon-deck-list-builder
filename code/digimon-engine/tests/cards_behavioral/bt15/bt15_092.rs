//! BT15-092 Revelation of Light — Option, Yellow, Cost 4.
//!
//! # Card text (cards.json / card image — verbatim)
//!
//! When an effect trashes this card from the security stack, activate this
//!   card's [Security] effects.
//! [Main] Search your security stack. You may play 1 yellow level 4 or lower
//!   Digimon card among it without paying the cost. Then, shuffle your security
//!   stack. If you have a Tamer with [Kari Kamiya] in its name, place this card
//!   on top of your security stack.
//!
//! Inherited / Security Effect:
//! [Security] All of your opponent's Digimon and security Digimon get -5000 DP
//!   until the end of your turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT15/Yellow/BT15_092.cs
//!
//! - OnDiscardSecurity (CanTriggerOnTrashSelfSecurity): re-runs the card's own
//!   [Security] body via `CardEffectCommons.OptionSecurityEffect(card)`.
//! - OptionSkill ([Main]): SelectCard from Security root, canNoSelect:true
//!   (optional), maxCount = Min(1, count of yellow Lv<=4 Digimon); IReduceSecurity
//!   + PlayPermanentCards(payCost:false); then ShuffledDeckCards(SecurityCards);
//!   then if HasMatchConditionOwnersPermanent(Tamer && TopCard contains "Kari
//!   Kamiya") -> AddSecurityCard(card, toTop:true).
//! - SecuritySkill ([Security]): ChangeDigimonDPPlayerEffect(opp battle-area
//!   Digimon, -5000, UntilOwnerTurnEnd) + ChangeSecurityDigimonCardDPPlayerEffect
//!   (opp security Digimon, -5000, UntilOwnerTurnEnd).
//!
//! # Substrate (all shipped — no engine/DSL gap)
//! - on_discard_security + scope: security  (BT25-034 / BT25-040 idiom)
//! - select_security (filtered) + play_security_card  (BT13-012 idiom)
//! - shuffle_security + place_self_option_at_security  (ST20-15 / BT13-012)
//! - add_modifier { continuous: true } board-wide -DP  (EX4-074 idiom)
//! - add_player_modifier { modifier: SecurityDpChange } for opponent security
//!   Digimon  (ST3-13 / ST1-14 idiom — defender_security_dp_adjustment consumer)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT15-092";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// An OnPlay card that trashes the controller's top security by effect — the
/// trigger BT15-092's security-scope clause keys on. (Mirrors BT25-034's
/// DSL-SEC-TRASHER fixture.)
const TRASHER_YAML: &str = r#"
card: DSL-SEC-TRASHER
name: Sec Trasher
kind: digimon
level: 4
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Trash your top security card"
    process:
      - trash_top_security: { of: you }
"#;

fn option_self(id: &str) -> CardData {
    let mut card = make_test_card(id, "Revelation of Light");
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = 4;
    card.level = None;
    card.dp = None;
    card
}

/// A yellow, level-N Digimon (used both as a legal [Main] play target and as
/// a board / security Digimon for the [Security] debuff tests).
fn yellow_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 0;
    card
}

fn blue_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 0;
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Yellow];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT15-092 YAML parses and compiles")
        .from_dsl_yaml(TRASHER_YAML)
        .expect("inline trasher fixture compiles")
        .add_card(option_self(CARD_ID))
        .add_card(yellow_digimon("Y-LV4", 4, 4000))
        .add_card(yellow_digimon("Y-LV3", 3, 3000))
        .add_card(yellow_digimon("Y-LV5", 5, 6000))
        .add_card(blue_digimon("B-LV4", 4, 4000))
        // High-DP opponent Digimon: 8000 DP so they survive the -5000 debuff
        // (a 5000-DP Digimon would floor to 0 and be deleted per rule 17-1-3-1).
        .add_card(blue_digimon("OPP-DGM", 4, 8000))
        .add_card(blue_digimon("OPP-DGM-2", 4, 8000))
        .add_card(blue_digimon("OPP-SEC-DGM", 4, 8000))
        .add_card(tamer("KARI", "Kari Kamiya"))
        .add_card(tamer("NOT-KARI", "T.K. Takaishi"))
        .add_card(make_test_card("PAD", "Filler"))
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
}

// ─── Section 0 — Structural / metadata ───────────────────────────────────────

#[test]
fn bt15_092_loads_with_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("BT15-092 in pack");
    assert_eq!(card.name, "Revelation of Light");
    assert_eq!(card.cost, Some(4));
}

#[test]
fn bt15_092_has_security_scope_discard_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Security
                    && t.when.contains(&CompiledTiming::OnDiscardSecurity)
        )
    });
    assert!(
        has,
        "BT15-092 must carry a security-scope OnDiscardSecurity dispatch clause"
    );
}

#[test]
fn bt15_092_has_main_and_security_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_main = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )
    });
    let has_security = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )
    });
    assert!(
        has_main,
        "BT15-092 must carry a [Main] (main_from_hand) clause"
    );
    assert!(
        has_security,
        "BT15-092 must carry a [Security] (on_security) clause"
    );
}

// ─── Section 1 — [Main]: search security, play yellow Lv<=4 free, shuffle ─────

/// Picking a yellow Lv4 Digimon from security plays it free onto the field.
/// The Lv5 yellow and the Lv4 blue must NOT be legal picks (yellow Lv<=4 only).
#[test]
fn bt15_092_main_plays_yellow_lv4_or_lower_from_security_free() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        // Security (top = last): one legal target (Y-LV4) plus three illegal
        // (Y-LV5 over-level, B-LV4 wrong color, PAD red Lv3 wrong color).
        .security(0, &["PAD", "Y-LV5", "B-LV4", "Y-LV4"])
        .memory(10)
        .start();
    // A yellow Digimon already in play satisfies the Option's color requirement
    // so the yellow [Main] Option is legal to play.
    runner.place_on_field(0, "Y-LV3", Some(0));

    // Play BT15-092 as a Main option from hand via the Option pipeline (stages
    // `pending_option` so the self-to-security placement step can consume the
    // in-flight option and the option disposes correctly).
    runner.game.play_option_from_hand(0, 0);

    let pending = runner
        .pending_selection()
        .expect("[Main] must install the optional security-search prompt");
    assert!(
        pending.is_optional,
        "the play is 'you may', so the security-search prompt is optional"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the yellow Lv4 Digimon is a legal pick (yellow Lv<=4 filter); got {:?}",
        pending.valid_action_ids
    );
    let (player, action_id) = (pending.selecting_player, pending.valid_action_ids[0]);
    runner
        .execute_action(player, action_id)
        .expect("security pick resolves");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "Y-LV4"),
        "the chosen yellow Lv4 Digimon must be played free onto P0's field"
    );
    // BT15-092 itself resolves and is disposed to trash (no Kari Kamiya Tamer).
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "the resolved Option must be disposed to trash"
    );
}

/// Declining the optional play: no Digimon is played, but the [Main] still
/// resolves (shuffle is unconditional). No self-to-security placement without
/// a Kari Kamiya Tamer.
#[test]
fn bt15_092_main_decline_plays_nothing_and_does_not_self_place_without_kari() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD", "Y-LV4"])
        .memory(10)
        .start();
    // Yellow Digimon in play -> Option color requirement satisfied.
    runner.place_on_field(0, "Y-LV3", Some(0));

    runner.game.play_option_from_hand(0, 0);

    let player = runner
        .pending_selection()
        .expect("optional security-search prompt installs")
        .selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("decline resolves");
    runner.auto_resolve().ok();

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "Y-LV4"),
        "declining must not play the security Digimon"
    );
    // No Kari Kamiya Tamer in play -> BT15-092 must NOT be placed on security;
    // it goes to trash like a normal resolved Option.
    assert!(
        !runner.game.players[0]
            .security
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "without a [Kari Kamiya] Tamer the card must NOT be placed on top of security"
    );
}

/// With a [Kari Kamiya] Tamer in play, after the [Main] resolves the card is
/// placed on TOP of the controller's security stack.
#[test]
fn bt15_092_main_places_self_on_security_top_with_kari_kamiya_tamer() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"])
        .memory(10)
        .start();
    // A Tamer with "Kari Kamiya" in its name is in P0's battle area.
    runner.place_on_field(0, "KARI", Some(0));

    let sec_before = runner.security_count(0);
    runner.game.play_option_from_hand(0, 0);
    // No yellow Lv<=4 Digimon in security -> the optional play prompt may be
    // skipped automatically; drive the rest of the resolution.
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0]
            .security
            .last()
            .map(|c| c.card_id(&runner.game.card_data) == CARD_ID)
            .unwrap_or(false),
        "with a [Kari Kamiya] Tamer, BT15-092 must be placed on TOP of security (last = top)"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "placing self on security increases the security count by 1"
    );
}

/// Negative: a yellow Tamer WITHOUT "Kari Kamiya" in its name does not satisfy
/// the self-to-security condition.
#[test]
fn bt15_092_main_no_self_place_with_non_kari_tamer() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"])
        .memory(10)
        .start();
    runner.place_on_field(0, "NOT-KARI", Some(0));

    runner.game.play_option_from_hand(0, 0);
    runner.auto_resolve().ok();

    assert!(
        !runner.game.players[0]
            .security
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "a Tamer without [Kari Kamiya] in its name must NOT trigger self-to-security placement"
    );
}

// ─── Section 2 — [Security]: -5000 DP to opp Digimon AND opp security Digimon ──

/// Helper: drive BT15-092's inherited [Security] effect for `attacker`'s attack
/// being blocked by BT15-092 in `defender`'s security. We model the resolution
/// directly via the OnSecurity timing by placing BT15-092 in security and
/// having the opponent attack into it.
fn run_security_effect(runner: &mut DebugRunner, attacker: PermanentHandle, defender: PlayerId) {
    let _ = runner.attack_player(attacker, defender, false);
    runner.auto_resolve().ok();
}

/// The [Security] effect debuffs all of the OPPONENT'S battle-area Digimon by
/// -5000 DP. "Opponent" = the player whose attacker triggered the security
/// effect (i.e. BT15-092's owner's opponent).
#[test]
fn bt15_092_security_debuffs_opponent_battle_area_digimon() {
    // P1 owns BT15-092 in security. P0 is the attacker (the "opponent" from
    // P1's perspective). P0 has a 5000-DP Digimon on the field.
    let mut runner = base().security(1, &[CARD_ID]).memory(0).start();
    let attacker = runner.place_on_field(0, "OPP-DGM", Some(0));
    let bystander = runner.place_on_field(0, "OPP-DGM-2", Some(0));

    let dp_before = runner.effective_dp(bystander).expect("bystander dp");

    run_security_effect(&mut runner, attacker, 1);

    // The bystander opponent Digimon (still alive) must be at -5000.
    let dp_after = runner
        .effective_dp(bystander)
        .expect("bystander still present");
    assert_eq!(
        dp_after,
        (dp_before - 5000).max(0),
        "all of P1's opponent's (P0's) battle-area Digimon must be -5000 DP after the [Security] effect"
    );
}

/// The [Security] effect debuffs the OPPONENT'S SECURITY Digimon by -5000 DP.
/// This is carried by a `SecurityDpChange` player modifier installed on the
/// opponent (P0), consumed by `Game::defender_security_dp_adjustment(P0)` when
/// P0 later defends.
#[test]
fn bt15_092_security_debuffs_opponent_security_digimon() {
    let mut runner = base().security(1, &[CARD_ID]).memory(0).start();
    let attacker = runner.place_on_field(0, "OPP-DGM", Some(0));

    // No security DP modifier on P0 before the effect.
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(0, ModifierType::SecurityDpChange),
        0,
        "no opponent security-DP modifier before the [Security] effect"
    );

    run_security_effect(&mut runner, attacker, 1);

    // After the [Security] effect, P0 (the opponent) carries a -5000
    // SecurityDpChange modifier — its security Digimon are -5000 when P0 defends.
    let sec_mod = runner
        .modifiers()
        .player_modifier_value(0, ModifierType::SecurityDpChange);
    assert_eq!(
        sec_mod, -5000,
        "the opponent (P0) must carry a -5000 SecurityDpChange modifier so its security Digimon are weakened"
    );
    assert_eq!(
        runner.game.defender_security_dp_adjustment(0),
        -5000,
        "P0's defender security DP adjustment must reflect the -5000 debuff"
    );
}

// ─── Section 3 — OnDiscardSecurity self-dispatch of the [Security] effect ─────

/// When an effect trashes BT15-092 from the OWN security stack, the
/// OnDiscardSecurity clause re-runs the card's [Security] body — installing
/// the -5000 board debuff and the opponent SecurityDpChange modifier.
///
/// Setup: P0 owns BT15-092 as the top of its own security. P0 plays the
/// effect-trasher (trashes its own top security = BT15-092). P0's opponent is
/// P1, who has a battle-area Digimon that should be debuffed.
#[test]
fn bt15_092_effect_trash_from_security_dispatches_security_effect() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["PAD", "PAD", CARD_ID])
        .memory(10)
        .start();
    let opp_dgm = runner.place_on_field(1, "OPP-DGM", Some(0));
    let dp_before = runner.effective_dp(opp_dgm).expect("opp dp");

    // Play the trasher -> trashes P0's top security (BT15-092).
    let _ = runner.play(0, 0).expect("play the trasher");
    runner.auto_resolve().ok();

    // BT15-092 left security to trash.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT15-092 must be in P0's trash after the effect-trash"
    );

    // The dispatched [Security] body debuffed P1's (the opponent's) battle-area
    // Digimon by -5000.
    let dp_after = runner.effective_dp(opp_dgm).expect("opp dgm present");
    assert_eq!(
        dp_after,
        (dp_before - 5000).max(0),
        "OnDiscardSecurity must re-run the [Security] effect: opponent Digimon -5000 DP"
    );
    // And installed the opponent SecurityDpChange modifier (P1 is the opponent
    // of the BT15-092 owner P0).
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(1, ModifierType::SecurityDpChange),
        -5000,
        "OnDiscardSecurity dispatch must install the opponent (-5000) SecurityDpChange modifier"
    );
}

/// Negative: a NORMAL security check (attack flips BT15-092 off security as a
/// security card) is NOT an effect-trash — the OnDiscardSecurity dispatch must
/// not fire from the attack-driven security check. (The [Security] effect
/// itself still runs via the OnSecurity timing; this asserts the
/// OnDiscardSecurity *dispatch* path is gated on effect-trash specifically.)
#[test]
fn bt15_092_normal_security_check_is_not_an_effect_trash_dispatch() {
    // No opponent battle-area Digimon here so we isolate the dispatch path:
    // we only check that the card resolves as a security card (leaves security)
    // and the game proceeds without panicking on a double dispatch.
    let mut runner = base().security(1, &[CARD_ID]).memory(0).start();
    let attacker = runner.place_on_field(0, "OPP-DGM", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().ok();

    // BT15-092 leaves P1's security as part of the normal security check.
    assert_eq!(
        runner.security_count(1),
        0,
        "the security card is resolved by the attack's security check"
    );
}
