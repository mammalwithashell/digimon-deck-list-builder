//! Chapter 15 (Comprehensive Rules Ver. 4.3), §15-15-5 "Isn't affected by
//! effects" cards: what an immunity blocks (§15-15-5-1), when a blocked
//! effect starts affecting the card (§15-15-5-2), that an immune card can
//! still be chosen (§15-15-5-3) and that an effect it gains while immune is
//! not considered had (§15-15-5-4).
//!
//! The scope matrix is the owner's 2026-10-16 request: Digimon-effect-only
//! immunity (Atratusmon ST23-09: "Your opponent's Digimon effects don't
//! affect this Digimon until their turn ends") against full opponent-effect
//! immunity (Magnamon (X Antibody) BT16-102: "isn't affected by your
//! opponent's effects"), each hit by an Option's [Main] (Plasma Stake BT3-098
//! / Final Shining Burst BT13-104 shapes), the same Option through its
//! [Security] activation of the [Main], a Tamer effect, a Digimon effect, a
//! token effect and an effect of the immune Digimon's own controller.
//! §15-2-1 sorts the effects: an effect activated by a Digimon is a Digimon
//! effect, by a Tamer a Tamer effect, by an Option card an Option card effect
//! -- a [Security] effect of an Option card included (official Q&A Q2119 /
//! Q2361: an "isn't affected by your opponent's Option cards" Digimon isn't
//! affected by their Options' [Security] effects either).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Engine reality: the immunities are filtered `ModifierType::CannotBeAffected`
//! entries (`grant_effect_immunity` steps for the [On Play] grants -- the
//! Atratusmon / Magnamon production idioms -- and an `effect_immunity` aura
//! for the "while suspended" card). A token's effect is a Digimon effect
//! (§15-2-1-1); `EffectSourceKind::Rule` is kept for card-less processing
//! only, so the Magnamon idiom grants digimon / tamer / option.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch15_15/*.yaml` (plus
//! `TAMER_PLAIN` from `ch17`) and the engine's registered Familiar Token
//! (`TOKEN_FAMILIAR`: "[On Deletion] 1 of your opponent's Digimon gets -3000
//! DP for the turn."), one synthetic card per file, authored to printed-style
//! text (each clause's `summary:` is that text).
//!
//! Security stacks are listed bottom first: the LAST card of `.security(..)`
//! is the top card.

use crate::fixtures;
use crate::support::*;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{EffectSourceKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

fn ch15_15() -> DebugRunnerBuilder {
    fixtures::builder(&[
        fixtures::ch16_atk_cards(),
        fixtures::ch17_cards(),
        fixtures::ch15_15_cards(),
    ])
}

/// P0 (memory 10, hand `hand0`) to move in its main phase; both players have
/// 6 deck cards; P0 has 3 security cards, P1 the stack `sec1`.
fn board(hand0: &[&str], sec1: &[&str]) -> DebugRunner {
    ch15_15()
        .hand(0, hand0)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, sec1)
        .memory(10)
        .start()
}

/// Put the red `TAMER_PLAIN` (no text) on P0's field: it meets the color
/// requirement of P0's red Options without being a Digimon any effect here
/// can choose.
fn red_anchor(r: &mut DebugRunner) {
    r.place_on_field(0, "TAMER_PLAIN", Some(0));
}

/// Place `card_id` on `player`'s field (able to attack) and activate its
/// [On Play] (the immunity grant of `IMMUNE_DIGI_ONLY` / `IMMUNE_ALL_OPP`, or
/// an "[On Play] ... gets -3000 DP" effect), settling its one-option prompts.
fn play_and_resolve(r: &mut DebugRunner, player: PlayerId, card_id: &str) -> PermanentHandle {
    let h = r.place_on_field(player, card_id, Some(0));
    r.fire_on_play(player, h.index as usize);
    resolve_prompts(r);
    h
}

/// `play_and_resolve` for an [On Play] whose one choice is the Digimon in
/// `slot` of the chooser's own or opponent's field (several candidates).
fn play_choosing(r: &mut DebugRunner, player: PlayerId, card_id: &str, slot: u16) {
    let h = r.place_on_field(player, card_id, Some(0));
    r.fire_on_play(player, h.index as usize);
    pick_field(r, slot);
    resolve_prompts(r);
}

fn dp(r: &DebugRunner, h: PermanentHandle) -> i32 {
    r.effective_dp(h)
        .unwrap_or_else(|| panic!("{h:?} is not on the field"))
}

/// The first Digimon on `player`'s field whose top card is `card_id`.
fn find(r: &DebugRunner, player: PlayerId, card_id: &str) -> Option<PermanentHandle> {
    field_top_ids(r, player)
        .iter()
        .position(|c| c == card_id)
        .map(|index| PermanentHandle {
            player,
            index: index as u8,
        })
}

fn on_field(r: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    find(r, player, card_id).is_some()
}

/// Put the engine's Familiar Token on `player`'s field, able to attack.
fn place_familiar_token(r: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    r.place_on_field(player, "TOKEN_FAMILIAR", Some(0))
}

// ─── §15-15-5-1: scope of a Digimon-effect-only immunity ────────────────────

/// §15-15-5-1 "When a card isn't affected by effects, it won't be affected by
/// processing caused by effects."
/// §15-2-1-3 "An effect activated by an Option card is considered an Option
/// card effect."
/// §17-1-3-1-1 (a Digimon whose DP is 0 is deleted by the rule check).
///
/// P1's `IMMUNE_DIGI_ONLY` (Atratusmon shape) activated its [On Play]: P0's
/// Digimon effects don't affect it until the end of P0's turn. An Option's
/// [Main] is not a Digimon effect:
/// - `OPT_DELETE_13K` ("[Main] Delete 1 of your opponent's Digimon with 13000
///   DP or more", Plasma Stake) deletes it (a 13000-DP printing of the card);
/// - `OPT_MINUS_12K` ("[Main] 1 of your opponent's Digimon gets -12000 DP
///   ...", Final Shining Burst) takes its 12000 DP to 0, and the rule check
///   deletes it.
#[test]
fn digimon_only_immunity_is_affected_by_opponent_option_main() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    cite!(
        "15-2-1-3",
        "f13435e95507f8691ff87219a6cd1eb940110e955215aeac653046406129e104"
    );
    cite!(
        "17-1-3-1-1",
        "02e57104aeaae9b771c6b28fd8c9a804176f67148a3e47868caa3cb5000597cc"
    );
    let mut r = board(&["OPT_DELETE_13K"], &[FILL; 3]);
    red_anchor(&mut r);
    r.force_base_dp("IMMUNE_DIGI_ONLY", 13000);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(imm, 0, EffectSourceKind::Digimon),
        "precondition: P0's Digimon effects don't affect it"
    );
    use_option(&mut r, 0, "OPT_DELETE_13K");
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 1, "IMMUNE_DIGI_ONLY"),
        "the Option's [Main] deletes it"
    );
    assert!(trash_ids(&r, 1).contains(&"IMMUNE_DIGI_ONLY".to_string()));

    let mut r = board(&["OPT_MINUS_12K"], &[FILL; 3]);
    red_anchor(&mut r);
    play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    use_option(&mut r, 0, "OPT_MINUS_12K");
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 1, "IMMUNE_DIGI_ONLY"),
        "-12000 DP from the Option takes it to 0 DP; the rule check deletes it"
    );
    assert!(trash_ids(&r, 1).contains(&"IMMUNE_DIGI_ONLY".to_string()));
}

/// §15-15-5-1 (as above), §15-2-1-3 (as above).
///
/// The [Security] activation of an Option's [Main] is still that Option
/// card's effect (§15-2-1-3; official Q&A Q2361). P0's `IMMUNE_DIGI_ONLY`
/// (P1's Digimon effects don't affect it until the end of P1's turn) attacks
/// P1, whose top security card is:
/// - `OPT_DELETE_13K` ("[Security] Activate this card's [Main] effects"):
///   the attacker (a 13000-DP printing) is deleted;
/// - `OPT_MINUS_12K`: the attacker gets -12000 DP, reaches 0 DP and is
///   deleted by the rule check.
#[test]
fn digimon_only_immunity_is_affected_by_option_security_activation() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    cite!(
        "15-2-1-3",
        "f13435e95507f8691ff87219a6cd1eb940110e955215aeac653046406129e104"
    );
    for (top, dp13k) in [("OPT_DELETE_13K", true), ("OPT_MINUS_12K", false)] {
        let mut r = board(&[], &[FILL, FILL, top]);
        if dp13k {
            r.force_base_dp("IMMUNE_DIGI_ONLY", 13000);
        }
        let attacker = play_and_resolve(&mut r, 0, "IMMUNE_DIGI_ONLY");
        assert!(
            r.game
                .permanent_is_unaffected_by_effect(attacker, 1, EffectSourceKind::Digimon),
            "precondition: P1's Digimon effects don't affect it"
        );
        r.attack_player(attacker, 1, false);
        resolve_prompts(&mut r);
        assert!(
            !on_field(&r, 0, "IMMUNE_DIGI_ONLY"),
            "{top}'s [Security] activation of its [Main] affects the attacker"
        );
        assert!(trash_ids(&r, 0).contains(&"IMMUNE_DIGI_ONLY".to_string()), "{top}");
    }
}

/// §15-15-5-1 (as above).
/// §15-2-1-1 "An effect activated by a Digimon card, Digimon, or Security
/// Digimon is considered a Digimon effect."
/// §15-2-1-2 "An effect activated by a Tamer card or Tamer is considered a
/// Tamer effect."
///
/// P1's `IMMUNE_DIGI_ONLY` (12000 DP) is immune to P0's Digimon effects only:
/// - P0's Tamer `TAMER_MINUS_3K` ("[On Play] 1 of your opponent's Digimon gets
///   -3000 DP until the end of their turn"): 9000 DP;
/// - P0's Digimon `DIGI_MINUS_3K` (the same text): still 12000 DP;
/// - P1's own Digimon `OWN_MINUS_3K` ("[On Play] 1 of your Digimon gets -3000
///   DP for the turn"): 9000 DP.
#[test]
fn digimon_only_immunity_is_affected_by_opponent_tamer_and_own_effects() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    cite!(
        "15-2-1-1",
        "1b26b455c5b8ddc666117716e988f4eaf37a39f7c247beb3eba6b1a6334497a2"
    );
    cite!(
        "15-2-1-2",
        "e305b51eb9a72d449ef5c73f3a4bc05b673e53c328693e8b6fdc48f610687182"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_and_resolve(&mut r, 0, "TAMER_MINUS_3K");
    assert_eq!(dp(&r, imm), 9000, "an opponent's Tamer effect affects it");

    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_and_resolve(&mut r, 0, "DIGI_MINUS_3K");
    assert_eq!(
        dp(&r, imm),
        12000,
        "an opponent's Digimon effect doesn't affect it"
    );

    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_choosing(&mut r, 1, "OWN_MINUS_3K", imm.index as u16);
    assert_eq!(dp(&r, imm), 9000, "its own controller's effect affects it");
}

// ─── §15-15-5-1: scope of a full opponent-effect immunity ───────────────────

/// §15-15-5-1 (as above).
///
/// `IMMUNE_ALL_OPP` (Magnamon (X Antibody) shape: "isn't affected by your
/// opponent's effects until the end of their turn", 12000 DP) is not
/// affected by any effect of its opponent:
/// - P0's Option `OPT_MINUS_12K` from the hand: still 12000 DP, on the field;
/// - the same Option's [Security] activation while P0's immune Digimon
///   attacks P1: still 12000 DP, on the field (official Q&A Q2361);
/// - P0's Tamer `TAMER_MINUS_3K`: 12000 DP;
/// - P0's Digimon `DIGI_MINUS_3K`: 12000 DP;
/// - a token's effect: P0's Familiar Token attacks P1's suspended immune
///   Digimon, loses and is deleted; its "[On Deletion] 1 of your opponent's
///   Digimon gets -3000 DP for the turn" can only choose the immune Digimon:
///   12000 DP. (Control: with P1's 1000-DP `FILL` chosen instead, the token's
///   effect resolves and `FILL` is deleted at 0 DP.)
/// Its own controller's effect still affects it: P1's `OWN_MINUS_3K`: 9000 DP.
#[test]
fn full_opponent_immunity_blocks_option_tamer_digimon_and_token_effects() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    // Option [Main] from the hand.
    let mut r = board(&["OPT_MINUS_12K"], &[FILL; 3]);
    red_anchor(&mut r);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    use_option(&mut r, 0, "OPT_MINUS_12K");
    resolve_prompts(&mut r);
    assert!(on_field(&r, 1, "IMMUNE_ALL_OPP"), "Option [Main]");
    assert_eq!(dp(&r, imm), 12000, "Option [Main]");

    // Option [Security] activation of its [Main].
    let mut r = board(&[], &[FILL, FILL, "OPT_MINUS_12K"]);
    let attacker = play_and_resolve(&mut r, 0, "IMMUNE_ALL_OPP");
    r.attack_player(attacker, 1, false);
    resolve_prompts(&mut r);
    assert!(on_field(&r, 0, "IMMUNE_ALL_OPP"), "Option [Security]");
    assert_eq!(dp(&r, attacker), 12000, "Option [Security]");

    // Tamer and Digimon effects.
    for source in ["TAMER_MINUS_3K", "DIGI_MINUS_3K"] {
        let mut r = board(&[], &[FILL; 3]);
        let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
        play_and_resolve(&mut r, 0, source);
        assert_eq!(dp(&r, imm), 12000, "{source}");
    }

    // Token effect (and its control).
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    set_suspended(&mut r, imm, true);
    let token = place_familiar_token(&mut r, 0);
    r.attack_digimon(token, imm, false);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 0, "TOKEN_FAMILIAR"),
        "the token lost the battle"
    );
    assert_eq!(dp(&r, imm), 12000, "the token's [On Deletion]");

    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    r.place_on_field(1, FILL, Some(0));
    set_suspended(&mut r, imm, true);
    let token = place_familiar_token(&mut r, 0);
    r.attack_digimon(token, imm, false);
    pick_field(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 1, FILL),
        "control: the token's [On Deletion] resolves (FILL at 0 DP is deleted)"
    );

    // Own effect.
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    play_choosing(&mut r, 1, "OWN_MINUS_3K", imm.index as u16);
    assert_eq!(dp(&r, imm), 9000, "its own controller's effect affects it");
}

/// §15-15-5-1 (as above).
/// §14-2-2 "The Digimon that loses is immediately deleted."
/// §17-1-3-1-1 (as above).
///
/// A battle and the rule check are rules, not effects (official Q&A Q6279:
/// "a battle itself is a rule that compares DP, not an effect"):
/// - P0's `HUGE_LV7` (15000 DP) attacks P1's suspended `IMMUNE_ALL_OPP`
///   (12000 DP): it loses the battle and is deleted;
/// - P1's `IMMUNE_ALL_OPP` (a 3000-DP printing) gets -3000 DP from its own
///   controller's `OWN_MINUS_3K`: at 0 DP the rule check deletes it.
#[test]
fn full_opponent_immunity_does_not_block_rule_processing() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    cite!(
        "14-2-2",
        "8eafdf6422cad42526b7df715508e30d34174cc29a32b30606253dbc6a04f0bf"
    );
    cite!(
        "17-1-3-1-1",
        "02e57104aeaae9b771c6b28fd8c9a804176f67148a3e47868caa3cb5000597cc"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    set_suspended(&mut r, imm, true);
    let attacker = r.place_on_field(0, "HUGE_LV7", Some(0));
    r.attack_digimon(attacker, imm, false);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 1, "IMMUNE_ALL_OPP"),
        "it lost the battle and was deleted"
    );
    assert!(on_field(&r, 0, "HUGE_LV7"));

    let mut r = board(&[], &[FILL; 3]);
    r.force_base_dp("IMMUNE_ALL_OPP", 3000);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    play_choosing(&mut r, 1, "OWN_MINUS_3K", imm.index as u16);
    assert!(
        !on_field(&r, 1, "IMMUNE_ALL_OPP"),
        "0 DP: the rule check deletes it"
    );
}

// ─── §15-15-5-3: an immune card can still be chosen ─────────────────────────

/// §15-15-5-3 "'Isn't affected by effects' cards can still be chosen for
/// effects. (Example: A card that isn't affected by effects can be chosen for
/// an '[On Play] Suspend 1 of your opponent's Digimon' effect, but it isn't
/// affected by that effect and it isn't suspended.)"
///
/// Official Q&A Q6176 (ST23-09) / Q3788 (an immune Digimon is chosen, and
/// the effect resolves without affecting it).
///
/// P0 uses `OPT_MINUS_12K` against P1's `IMMUNE_ALL_OPP`:
/// - beside P1's `FILL`, both are offered; choosing the immune Digimon does
///   nothing to it (12000 DP) and the Option is used (P0's trash);
/// - when it is P1's only Digimon, it is still offered (the effect is not
///   skipped for lack of a target), and choosing it does nothing.
#[test]
fn immune_permanent_is_still_choosable_and_offered_as_only_candidate() {
    cite!(
        "15-15-5-3",
        "170d539a05566095ac27d28910127cd4991e960081e9eb56d89d9f94423b976d"
    );
    use digimon_engine::action::space::ATTACK_START;

    let mut r = board(&["OPT_MINUS_12K"], &[FILL; 3]);
    red_anchor(&mut r);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    r.place_on_field(1, FILL, Some(0));
    use_option(&mut r, 0, "OPT_MINUS_12K");
    let offered = r
        .pending_selection()
        .map(|s| s.valid_action_ids.clone())
        .unwrap_or_default();
    assert!(
        offered.contains(&(ATTACK_START + imm.index as u16))
            && offered.contains(&(ATTACK_START + 1)),
        "both of P1's Digimon are offered: {offered:?}"
    );
    pick_field(&mut r, imm.index as u16);
    resolve_prompts(&mut r);
    assert_eq!(dp(&r, imm), 12000, "chosen, not affected");
    assert!(trash_ids(&r, 0).contains(&"OPT_MINUS_12K".to_string()));

    let mut r = board(&["OPT_MINUS_12K"], &[FILL; 3]);
    red_anchor(&mut r);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_ALL_OPP");
    use_option(&mut r, 0, "OPT_MINUS_12K");
    let offered = r
        .pending_selection()
        .map(|s| s.valid_action_ids.clone())
        .unwrap_or_default();
    assert_eq!(
        offered,
        vec![ATTACK_START + imm.index as u16],
        "the immune Digimon is offered as the only candidate"
    );
    pick_field(&mut r, imm.index as u16);
    resolve_prompts(&mut r);
    assert_eq!(dp(&r, imm), 12000, "chosen, not affected");
}

// ─── §15-15-5-1 / §15-15-5-2: when an immunity starts and ends ──────────────

/// §15-15-5-1 (as above). Official Q&A Q6178 (ST23-09): "If a card is
/// affected by an effect, then it later gains an 'effects don't affect'
/// effect ... As soon as it gains the 'effects don't affect' effect, it will
/// no longer be affected."
///
/// P1's `IMMUNE_DIGI_ONLY` (12000 DP, its [On Play] not yet activated) gets
/// -3000 DP until the end of P1's turn from P0's `DIGI_MINUS_3K`: 9000 DP.
/// Its [On Play] then activates (P0's Digimon effects don't affect it): it is
/// back at 12000 DP at once.
#[test]
fn effect_applied_before_immunity_stops_affecting_when_immunity_starts() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = r.place_on_field(1, "IMMUNE_DIGI_ONLY", Some(0));
    play_and_resolve(&mut r, 0, "DIGI_MINUS_3K");
    assert_eq!(dp(&r, imm), 9000, "precondition: affected");
    r.fire_on_play(1, imm.index as usize);
    resolve_prompts(&mut r);
    assert_eq!(dp(&r, imm), 12000, "no longer affected once immune");
}

/// §15-15-5-2 "When a card isn't affected by effects, then it can be affected
/// again, effects will immediately start affecting that card. (Example: When
/// a -3000 DP effect is applied to a suspended Digimon with an '[All Turns]
/// While this Digimon is suspended, this Digimon isn't affected by the effects
/// of your opponent's Digimon' effect, the Digimon will get -3000 DP as soon
/// as it's unsuspended.)"
///
/// Official Q&A Q831 (ST17-08) / Q6179 (ST23-09).
///
/// - The rule's example: P1's suspended `IMMUNE_WHILE_SUSPENDED` (5000 DP)
///   gets -3000 DP until the end of P1's turn from P0's `DIGI_MINUS_3K`:
///   5000 DP for the rest of P0's turn. P1's turn starts; its unsuspend phase
///   unsuspends it: 2000 DP in P1's main phase.
/// - The Atratusmon shape: P1's `IMMUNE_DIGI_ONLY` (P0's Digimon effects
///   don't affect it until the end of P0's turn) gets the same -3000 DP:
///   12000 DP for the rest of P0's turn; in P1's turn the immunity has ended
///   and the -3000 DP (still in its duration) applies: 9000 DP.
#[test]
fn effect_applied_while_immune_starts_affecting_when_immunity_lapses() {
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = r.place_on_field(1, "IMMUNE_WHILE_SUSPENDED", Some(0));
    set_suspended(&mut r, imm, true);
    play_and_resolve(&mut r, 0, "DIGI_MINUS_3K");
    assert_eq!(dp(&r, imm), 5000, "suspended: not affected");
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert!(!is_suspended(&r, imm), "unsuspended in P1's unsuspend phase");
    assert_eq!(dp(&r, imm), 2000, "affected as soon as it's unsuspended");

    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_and_resolve(&mut r, 0, "DIGI_MINUS_3K");
    assert_eq!(dp(&r, imm), 12000, "immune: not affected");
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert_eq!(
        dp(&r, imm),
        9000,
        "the immunity ended with P0's turn; the -3000 DP is still in its duration"
    );
}

// ─── §15-15-5-4: an effect gained while immune ──────────────────────────────

/// §15-15-5-4 "Even if a Digimon that isn't affected by effects gains an
/// effect, the Digimon won't be considered to have that effect. (Example: If
/// an effect reads '[On Play] 1 of your opponent's Digimon gains <Security A.
/// -1> until the end of their turn,' even if a Digimon that isn't affected by
/// effects gains <Security A. -1>, the Digimon won't be considered to have
/// <Security A. -1>."
///
/// Official Q&A Q6179 (ST23-09): an effect gained while immune affects the
/// card "as soon as it can be affected by effects".
///
/// P1's `IMMUNE_DIGI_ONLY` (P0's Digimon effects don't affect it until the
/// end of P0's turn) gains <Security A. -1> until the end of P1's turn from
/// P0's `DIGI_GRANT_SA_MINUS1`:
/// - in P0's turn it is not considered to have it: it checks 1 card;
/// - in P1's turn the immunity has ended and the gained effect persists: it
///   checks 0 cards, and its attack on P0 removes none of P0's security.
///
/// The same for a gained triggered effect (official Q&A Q2120, Ice Wall /
/// BT11-093; Q3871 / Q5158: such an effect "won't trigger" only while the
/// Digimon "isn't affected by effects upon the trigger timing"): P0's
/// `DIGI_GRANT_EOT_DELETE` gives the immune Digimon "[End of Your Turn] Delete
/// this Digimon." until the end of P1's turn; the immunity ends with P0's
/// turn, so at the end of P1's turn the gained effect activates and deletes
/// it.
#[test]
fn granted_effect_not_considered_had_while_immune() {
    cite!(
        "15-15-5-4",
        "affbf2f4ae2c52abeb3bcbd00844ac85000dfdc830bb29ea70095ec8e0801229"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_and_resolve(&mut r, 0, "DIGI_GRANT_SA_MINUS1");
    assert_eq!(
        r.game.effective_security_strike(imm),
        1,
        "immune: not considered to have <Security A. -1>"
    );
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert_eq!(
        r.game.effective_security_strike(imm),
        0,
        "no longer immune: it has the gained <Security A. -1>"
    );
    let before = r.security_count(0);
    r.attack_player(imm, 0, false);
    resolve_prompts(&mut r);
    assert_eq!(r.security_count(0), before, "it checks 0 security cards");

    // A gained triggered effect.
    let mut r = board(&[], &[FILL; 3]);
    play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_and_resolve(&mut r, 0, "DIGI_GRANT_EOT_DELETE");
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert!(on_field(&r, 1, "IMMUNE_DIGI_ONLY"));
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 0, "P1's turn ended");
    assert!(
        !on_field(&r, 1, "IMMUNE_DIGI_ONLY"),
        "the gained \"[End of Your Turn] Delete this Digimon\" activated at the end of P1's turn"
    );
}

// ─── §15-15-5-1: an immunity that starts during an effect ───────────────────

/// §15-15-5-1 (as above).
///
/// An effect whose later processing meets an immunity that started during
/// that same effect:
/// - P0's `DIGI_SUSPEND_THEN_MINUS3K` ("[On Play] Suspend 1 of your
///   opponent's Digimon. Then, that Digimon gets -3000 DP for the turn")
///   chooses P1's unsuspended `IMMUNE_WHILE_SUSPENDED` (5000 DP): it is
///   suspended, and from then on P0's Digimon effects don't affect it -- the
///   -3000 DP doesn't: 5000 DP, suspended. That -3000 DP lasts for the turn
///   only, so when the Digimon unsuspends in P1's turn it is still 5000 DP
///   (§15-15-5-2 restarts an effect only while it lasts). (Control: P1's
///   `LV5_PLAIN`, 7000 DP and no immunity, ends suspended at 4000 DP.)
/// - The reverse, own-effect shape (judge-quiz Q18: an immunity to ALL
///   Digimon effects covers its controller's own): P0's `SELF_IMM_THEN_UP`
///   ("[On Play] This Digimon isn't affected by the effects of Digimon for the
///   turn. Then, this Digimon gets +3000 DP for the turn") stays at 7000 DP.
#[test]
fn immunity_granted_mid_effect_covers_later_steps() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    let mut r = board(&[], &[FILL; 3]);
    let target = r.place_on_field(1, "IMMUNE_WHILE_SUSPENDED", Some(0));
    play_and_resolve(&mut r, 0, "DIGI_SUSPEND_THEN_MINUS3K");
    assert!(is_suspended(&r, target), "the first step affected it");
    assert_eq!(
        dp(&r, target),
        5000,
        "suspended: the later -3000 DP doesn't affect it"
    );
    // The -3000 DP was "for the turn": it ends with P0's turn, so when the
    // Digimon unsuspends in P1's unsuspend phase it never comes back.
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert!(!is_suspended(&r, target), "unsuspended in P1's unsuspend phase");
    assert_eq!(
        dp(&r, target),
        5000,
        "the -3000 DP expired while it was immune: it never applies"
    );

    // Control: the same effect on P1's `LV5_PLAIN` (7000 DP, no immunity)
    // applies both steps.
    let mut r = board(&[], &[FILL; 3]);
    let plain = r.place_on_field(1, "LV5_PLAIN", Some(0));
    play_and_resolve(&mut r, 0, "DIGI_SUSPEND_THEN_MINUS3K");
    assert!(is_suspended(&r, plain), "control");
    assert_eq!(dp(&r, plain), 4000, "control");

    let mut r = board(&[], &[FILL; 3]);
    let me = play_and_resolve(&mut r, 0, "SELF_IMM_THEN_UP");
    assert_eq!(
        dp(&r, me),
        7000,
        "its own +3000 DP is a Digimon effect: it doesn't affect it"
    );
}

// ─── §16-39-1 <Progress>: an attack-scoped immunity ─────────────────────────

/// §16-39-1 "<Progress> is a keyword effect that reads 'this Digimon isn't
/// affected by your opponent's effects while attacking.'"
/// §15-15-5-2 (as above).
///
/// P0's `PROGRESS` (6000 DP) attacks P1, whose `OPP_ATK_ALL_MINUS_12K`
/// ("When an opponent's Digimon attacks, all of your opponent's Digimon get
/// -12000 DP until the end of their turn" -- it chooses nothing) triggers on
/// the declaration. While attacking, `PROGRESS` isn't affected: it checks
/// P1's top security card (the 1000-DP `FILL`) and wins that battle at 6000
/// DP. When the attack ends the -12000 DP (still in its duration) starts
/// affecting it: 0 DP, and the rule check deletes it.
///
/// Control: the same attack by P0's `LV5_PLAIN` (7000 DP, no <Progress>)
/// drops it to 0 DP at once; it is deleted before any security check.
#[test]
fn progress_attacker_effect_applied_during_attack_applies_after_it() {
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    let mut r = board(&[], &[FILL; 3]);
    let attacker = r.place_on_field(0, "PROGRESS", Some(0));
    r.place_on_field(1, "OPP_ATK_ALL_MINUS_12K", Some(0));
    r.attack_player(attacker, 1, false);
    resolve_prompts(&mut r);
    assert_eq!(
        r.security_count(1),
        2,
        "not affected while attacking: it checked P1's security"
    );
    assert!(
        trash_ids(&r, 1).contains(&FILL.to_string()),
        "the 1000-DP security Digimon lost the battle to the 6000-DP attacker"
    );
    assert!(
        !on_field(&r, 0, "PROGRESS"),
        "the attack is over: -12000 DP applies, 0 DP, deleted by the rule check"
    );

    let mut r = board(&[], &[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "OPP_ATK_ALL_MINUS_12K", Some(0));
    r.attack_player(attacker, 1, false);
    resolve_prompts(&mut r);
    assert!(!on_field(&r, 0, "LV5_PLAIN"), "control: 0 DP, deleted");
    assert_eq!(
        r.security_count(1),
        3,
        "control: deleted at once, before any security check"
    );
}

/// §16-39-1 (as above). §15-15-5-1 (as above); official Q&A Q6178: an effect
/// already affecting a card stops affecting it as soon as it can't be
/// affected.
///
/// During P0's turn P1's `DIGI_MINUS_3K` gives P0's `PROGRESS` -3000 DP until
/// the end of P0's turn (3000 DP). `PROGRESS` then attacks P1's suspended
/// `DIGI_MINUS_3K` (4000 DP): while attacking it isn't affected by that
/// opponent's effect, so it battles at 6000 DP, wins and survives; after the
/// attack it is at 3000 DP again.
#[test]
fn progress_attacker_not_affected_by_earlier_opponent_effect_while_attacking() {
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    let mut r = board(&[], &[FILL; 3]);
    let attacker = r.place_on_field(0, "PROGRESS", Some(0));
    let target = play_and_resolve(&mut r, 1, "DIGI_MINUS_3K");
    assert_eq!(dp(&r, attacker), 3000, "precondition: affected before attacking");
    set_suspended(&mut r, target, true);
    r.attack_digimon(attacker, target, false);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 1, "DIGI_MINUS_3K"),
        "6000 DP while attacking: it wins the battle"
    );
    assert!(on_field(&r, 0, "PROGRESS"), "it survives the battle");
    assert_eq!(dp(&r, attacker), 3000, "affected again after the attack");
}

/// §15-15-5-2 (as above). §16-39-1 (as above). §17-1-3-1-1 (a Digimon whose
/// DP is 0 is deleted by the rule check).
///
/// P0 (4 security cards) has `TAMER_PROGRESS_SEC4` (while P0 has 4 or more
/// security cards, all of P0's Digimon gain <Progress>) and a 4000-DP
/// `DIGI_MINUS_3K`. During P0's turn P1's `DIGI_MINUS_3K` gives it -3000 DP
/// until the end of P0's turn: 1000 DP. It attacks P1. P1's
/// `OPP_WA_MINUS3000` and `OPP_ATK_TRASH_OPP_SEC` trigger; while P1 orders
/// them the attack is under way and the earlier -3000 DP doesn't affect it:
/// 4000 DP. `OPP_WA_MINUS3000` gives it another -3000 DP, which doesn't affect
/// it either; then `OPP_ATK_TRASH_OPP_SEC` trashes P0's top security card
/// (it affects P0, not the Digimon): 3 security cards, no <Progress>, and
/// both -3000 DP affect it at once -- 0 DP, deleted by the rule check before
/// any security check (P1 still has 3 security cards).
#[test]
fn progress_lost_mid_attack_applies_held_opponent_effects_immediately() {
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    let mut r = ch15_15()
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(10)
        .start();
    r.place_on_field(0, "TAMER_PROGRESS_SEC4", Some(0));
    let attacker = r.place_on_field(0, "DIGI_MINUS_3K", Some(0));
    play_and_resolve(&mut r, 1, "DIGI_MINUS_3K");
    assert_eq!(dp(&r, attacker), 1000, "precondition: affected before attacking");
    r.place_on_field(1, "OPP_WA_MINUS3000", Some(0));
    r.place_on_field(1, "OPP_ATK_TRASH_OPP_SEC", Some(0));
    r.attack_player(attacker, 1, false);
    assert_eq!(
        dp(&r, attacker),
        4000,
        "attacking with <Progress>: the earlier -3000 DP doesn't affect it"
    );
    choose(&mut r, "OPP_WA_MINUS3000");
    resolve_prompts(&mut r);
    assert_eq!(r.security_count(0), 3, "P1's effect trashed P0's top security card");
    assert!(
        !on_field(&r, 0, "DIGI_MINUS_3K"),
        "no <Progress> any more: both -3000 DP apply at once, 0 DP, deleted"
    );
    assert_eq!(
        r.security_count(1),
        3,
        "deleted by the rule check before any security check"
    );

    // The direct shape: the attacker's OWN "[All Turns] While you have 4 or
    // more security cards, this Digimon gains <Progress>" (`PROGRESS_SEC4`,
    // 4000 DP) -- the same sequence, with the conditional keyword read from
    // the card's own clause (G-TEST-DEBUGRUNNER-IGNORES-ACTIVE-WHEN-ON-OWN-
    // GRANT-KEYWORD: the DebugRunner no longer folds it into the printed
    // keywords, so it is seen switching off).
    let mut r = ch15_15()
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(10)
        .start();
    let attacker = r.place_on_field(0, "PROGRESS_SEC4", Some(0));
    assert!(r.game.has_keyword(attacker, Keyword::Progress), "4 security cards: <Progress>");
    play_and_resolve(&mut r, 1, "DIGI_MINUS_3K");
    assert_eq!(dp(&r, attacker), 1000, "precondition: affected before attacking");
    r.place_on_field(1, "OPP_WA_MINUS3000", Some(0));
    r.place_on_field(1, "OPP_ATK_TRASH_OPP_SEC", Some(0));
    r.attack_player(attacker, 1, false);
    assert_eq!(
        dp(&r, attacker),
        4000,
        "attacking with its own conditional <Progress>: the earlier -3000 DP doesn't affect it"
    );
    choose(&mut r, "OPP_WA_MINUS3000");
    resolve_prompts(&mut r);
    assert_eq!(r.security_count(0), 3, "P1's effect trashed P0's top security card");
    assert!(
        !on_field(&r, 0, "PROGRESS_SEC4"),
        "3 security cards: no <Progress>, both -3000 DP apply at once, 0 DP, deleted"
    );
    assert_eq!(
        r.security_count(1),
        3,
        "deleted by the rule check before any security check"
    );
}

/// §16-39-1 (as above). §15-15-5-2 (as above).
///
/// P0's field: `FILL` (slot 0), `PROGRESS` (slot 1, 6000 DP) and `LV5_PLAIN`
/// (slot 2, 7000 DP). During P0's turn P1's two `DIGI_MINUS_3K` give
/// `PROGRESS` and `LV5_PLAIN` -3000 DP each until the end of P0's turn (3000
/// / 4000 DP). `PROGRESS` attacks P1; P1's `OPP_ATK_DELETE` and
/// `OPP_WA_MINUS3000` trigger and P1 resolves `OPP_ATK_DELETE` first: it
/// deletes `FILL`, so `PROGRESS` moves to slot 0 and `LV5_PLAIN` to slot 1,
/// the attacker's old slot. At `OPP_WA_MINUS3000`'s choice the attack is
/// under way: the attacker isn't affected by the earlier -3000 DP (6000 DP);
/// `LV5_PLAIN`, now in its old slot, is (4000 DP). P1 gives `LV5_PLAIN`
/// another -3000 DP (1000 DP). After the attack `PROGRESS` is at 3000 DP
/// again.
#[test]
fn progress_shield_follows_the_attacker_through_compaction() {
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    let mut r = board(&[], &[FILL; 3]);
    r.place_on_field(0, FILL, Some(0));
    let attacker = r.place_on_field(0, "PROGRESS", Some(0));
    let plain = r.place_on_field(0, "LV5_PLAIN", Some(0));
    assert_eq!((attacker.index, plain.index), (1, 2), "precondition: slots");
    play_choosing(&mut r, 1, "DIGI_MINUS_3K", 1);
    play_choosing(&mut r, 1, "DIGI_MINUS_3K", 2);
    assert_eq!(dp(&r, attacker), 3000, "precondition: affected before attacking");
    assert_eq!(dp(&r, plain), 4000, "precondition: affected");
    r.place_on_field(1, "OPP_ATK_DELETE", Some(0));
    r.place_on_field(1, "OPP_WA_MINUS3000", Some(0));
    r.attack_player(attacker, 1, false);
    choose(&mut r, "OPP_ATK_DELETE");
    pick_field(&mut r, 0);
    assert!(!on_field(&r, 0, FILL), "OPP_ATK_DELETE deleted the slot-0 Digimon");
    let attacker = find(&r, 0, "PROGRESS").expect("the attacker is on the field");
    let plain = find(&r, 0, "LV5_PLAIN").expect("LV5_PLAIN is on the field");
    assert_eq!(
        (attacker.index, plain.index),
        (0, 1),
        "compaction: LV5_PLAIN slid into the attacker's old slot"
    );
    assert_eq!(
        dp(&r, attacker),
        6000,
        "still attacking with <Progress>: the earlier -3000 DP doesn't affect it"
    );
    assert_eq!(
        dp(&r, plain),
        4000,
        "the Digimon in the attacker's old slot is not shielded"
    );
    pick_field(&mut r, 1);
    resolve_prompts(&mut r);
    assert_eq!(dp(&r, plain), 1000, "LV5_PLAIN took the second -3000 DP");
    assert_eq!(
        dp(&r, attacker),
        3000,
        "the attack is over: the earlier -3000 DP affects it again"
    );
}

// ─── §15-2-1-1: a token's effect is a Digimon effect ────────────────────────

/// §15-2-1-1 "An effect activated by a Digimon card, Digimon, or Security
/// Digimon is considered a Digimon effect." A Familiar Token is a Digimon
/// (it has Digimon card information), so its effect is a Digimon effect
/// (DCGO `AutoProcessing.cs` marks token effects `IsDigimonEffect`).
///
/// P1's `IMMUNE_DIGI_ONLY` (P0's Digimon effects don't affect it until the
/// end of P0's turn) is suspended; P0's Familiar Token attacks it, loses and
/// is deleted. Its "[On Deletion] 1 of your opponent's Digimon gets -3000 DP
/// for the turn" can only choose the immune Digimon, and doesn't affect it:
/// 12000 DP.
#[test]
fn digimon_only_immunity_blocks_a_digimon_tokens_effect() {
    cite!(
        "15-2-1-1",
        "1b26b455c5b8ddc666117716e988f4eaf37a39f7c247beb3eba6b1a6334497a2"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    set_suspended(&mut r, imm, true);
    let token = place_familiar_token(&mut r, 0);
    r.attack_digimon(token, imm, false);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 0, "TOKEN_FAMILIAR"),
        "the token lost the battle"
    );
    assert_eq!(
        dp(&r, imm),
        12000,
        "the token's [On Deletion] is a Digimon effect: it doesn't affect it"
    );
}

// ─── §15-15-5-4 / §15-15-5-2 / §16-39-1: a GAINED KEYWORD (registry grant) ──

/// §15-15-5-4 (as above: the rule's own example is a gained <Security A. -1>).
/// §15-15-5-2 (as above). Official Q&A Q6179.
///
/// `granted_effect_not_considered_had_while_immune` authors the gained
/// <Security A. -1> with the modifier idiom; this is the same effect authored
/// as a keyword grant (`DIGI_GRANT_KW_SA_MINUS1`, the `grant_keyword` registry
/// path), which must agree: P1's `IMMUNE_DIGI_ONLY` (P0's Digimon effects
/// don't affect it until the end of P0's turn) gains <Security A. -1> until
/// the end of P1's turn --
/// - in P0's turn it is not considered to have it: it checks 1 card;
/// - in P1's turn the immunity has ended and the gained keyword, still in
///   its duration, is had: it checks 0 cards, and its attack on P0 removes
///   none of P0's security.
#[test]
fn granted_keyword_not_considered_had_while_immune_had_once_immunity_lapses() {
    cite!(
        "15-15-5-4",
        "affbf2f4ae2c52abeb3bcbd00844ac85000dfdc830bb29ea70095ec8e0801229"
    );
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    play_and_resolve(&mut r, 0, "DIGI_GRANT_KW_SA_MINUS1");
    assert_eq!(
        r.game.effective_security_strike(imm),
        1,
        "immune: not considered to have the gained <Security A. -1>"
    );
    assert!(
        !r.game.has_keyword(imm, Keyword::SecurityAttackMinus(1)),
        "immune: the keyword is not had"
    );
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert!(
        r.game.has_keyword(imm, Keyword::SecurityAttackMinus(1)),
        "no longer immune: the gained keyword, still in its duration, is had"
    );
    assert_eq!(
        r.game.effective_security_strike(imm),
        0,
        "no longer immune: it has the gained <Security A. -1>"
    );
    let before = r.security_count(0);
    r.attack_player(imm, 0, false);
    resolve_prompts(&mut r);
    assert_eq!(r.security_count(0), before, "it checks 0 security cards");
}

/// §15-15-5-1 (as above). Official Q&A Q6178 (ST23-09): "As soon as it gains
/// the 'effects don't affect' effect, it will no longer be affected."
///
/// The converse direction for a gained keyword: P1's `IMMUNE_DIGI_ONLY` (its
/// [On Play] not yet activated) gains <Security A. -1> from P0's
/// `DIGI_GRANT_KW_SA_MINUS1` and has it (it would check 0 cards). Its [On
/// Play] then activates (P0's Digimon effects don't affect it): the keyword
/// is no longer had -- it checks 1 card again -- for as long as the immunity
/// lasts.
#[test]
fn granted_keyword_stops_being_had_when_immunity_starts() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = r.place_on_field(1, "IMMUNE_DIGI_ONLY", Some(0));
    play_and_resolve(&mut r, 0, "DIGI_GRANT_KW_SA_MINUS1");
    assert!(
        r.game.has_keyword(imm, Keyword::SecurityAttackMinus(1)),
        "precondition: the gained keyword is had"
    );
    assert_eq!(r.game.effective_security_strike(imm), 0, "precondition");
    r.fire_on_play(1, imm.index as usize);
    resolve_prompts(&mut r);
    assert!(
        !r.game.has_keyword(imm, Keyword::SecurityAttackMinus(1)),
        "immune: the keyword gained before the immunity is no longer had"
    );
    assert_eq!(
        r.game.effective_security_strike(imm),
        1,
        "immune: it checks 1 card again"
    );
}

/// §16-39-1 (as above). §15-15-5-4 (as above).
///
/// P1's `DIGI_GRANT_KW_SA_MINUS1` gives P0's `PROGRESS` <Security A. -1>
/// until the end of P0's turn; not attacking, `PROGRESS` has it (0 checks).
/// It attacks P1 (3 security cards): while attacking it isn't affected by
/// P1's effect, so the gained keyword is not had and it checks 1 card. When
/// the attack ends the keyword (still in its duration) is had again.
#[test]
fn progress_attacker_does_not_have_a_keyword_an_opponent_granted_while_attacking() {
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    cite!(
        "15-15-5-4",
        "affbf2f4ae2c52abeb3bcbd00844ac85000dfdc830bb29ea70095ec8e0801229"
    );
    let mut r = board(&[], &[FILL; 3]);
    let attacker = r.place_on_field(0, "PROGRESS", Some(0));
    play_and_resolve(&mut r, 1, "DIGI_GRANT_KW_SA_MINUS1");
    assert!(
        r.game.has_keyword(attacker, Keyword::SecurityAttackMinus(1)),
        "not attacking: the opponent's granted keyword is had"
    );
    assert_eq!(r.game.effective_security_strike(attacker), 0, "0 checks");
    r.attack_player(attacker, 1, false);
    resolve_prompts(&mut r);
    assert_eq!(
        r.security_count(1),
        2,
        "while attacking the gained <Security A. -1> is not had: it checked 1 card"
    );
    assert!(on_field(&r, 0, "PROGRESS"), "6000 DP beats the 1000-DP security Digimon");
    assert!(
        r.game.has_keyword(attacker, Keyword::SecurityAttackMinus(1)),
        "the attack is over: the keyword is had again"
    );
}

// ─── §16-39-1 / §15-15-5-1: a FORMULA aura (computed live, no stored entry) ─

/// §16-39-1 (as above). §15-15-5-2 (as above).
///
/// P1's `OPP_AURA_MINUS_PER_FIELD` ("[All Turns] All of your opponent's
/// Digimon get -1000 DP for each Digimon and Tamer you have") is a formula
/// aura, summed live inside `effective_dp` rather than stored as a modifier
/// entry. P1 has it and a suspended `DIGI_MINUS_3K` (4000 DP): P0's
/// `PROGRESS` (6000 DP) is at 4000 DP. It attacks the suspended Digimon:
/// while attacking it isn't affected by P1's aura, battles at 6000 DP, wins
/// and survives. After the attack P1 has one permanent left: 5000 DP.
///
/// Control: P0's `LV5_PLAIN` (7000 DP, no <Progress>) is at 5000 DP and
/// attacks the same 4000-DP target at 5000 DP: it wins too, but battled
/// affected (a 4000-DP control would have been deleted with its target).
#[test]
fn progress_attacker_not_affected_by_opponent_formula_aura_while_attacking() {
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    let mut r = board(&[], &[FILL; 3]);
    let attacker = r.place_on_field(0, "PROGRESS", Some(0));
    r.place_on_field(1, "OPP_AURA_MINUS_PER_FIELD", Some(0));
    let target = r.place_on_field(1, "DIGI_MINUS_3K", Some(0));
    set_suspended(&mut r, target, true);
    assert_eq!(dp(&r, attacker), 4000, "precondition: -2000 DP from the aura");
    r.attack_digimon(attacker, target, false);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 1, "DIGI_MINUS_3K"),
        "6000 DP while attacking: it wins the battle"
    );
    assert!(on_field(&r, 0, "PROGRESS"), "it survives the battle");
    assert_eq!(
        dp(&r, attacker),
        5000,
        "affected again after the attack (P1 has one permanent left)"
    );

    // Control: the same battle without <Progress> is fought at the reduced DP.
    let mut r = board(&[], &[FILL; 3]);
    let attacker = r.place_on_field(0, "DIGI_MINUS_3K", Some(0));
    r.place_on_field(1, "OPP_AURA_MINUS_PER_FIELD", Some(0));
    let target = r.place_on_field(1, "DIGI_MINUS_3K", Some(0));
    set_suspended(&mut r, target, true);
    assert_eq!(dp(&r, attacker), 2000, "control: 4000 - 2000");
    r.attack_digimon(attacker, target, false);
    resolve_prompts(&mut r);
    assert!(
        !on_field(&r, 0, "DIGI_MINUS_3K"),
        "control: at 2000 DP the attacker loses to the 4000-DP target"
    );
    assert!(on_field(&r, 1, "DIGI_MINUS_3K"), "control: the target survives");
}

/// §15-15-5-1 (as above). §15-15-5-2 (as above).
///
/// P0's `OPP_AURA_MINUS_PER_FIELD` (a Digimon effect) reaches P1's
/// `IMMUNE_DIGI_ONLY` (12000 DP) before its [On Play]: 11000 DP. Once P0's
/// Digimon effects don't affect it, the live formula aura doesn't either:
/// 12000 DP. In P1's turn the immunity has ended: 11000 DP again.
#[test]
fn immunity_suppresses_an_opponent_formula_aura_while_it_lasts() {
    cite!(
        "15-15-5-1",
        "5f62baf83d024884f00e48e9464e881636542e141762f6036fdc9c43db57c3d2"
    );
    cite!(
        "15-15-5-2",
        "780144a7af6d35d35c824ad67fea8c742362d65bff1707531bdbd9955dc54ffc"
    );
    let mut r = board(&[], &[FILL; 3]);
    r.place_on_field(0, "OPP_AURA_MINUS_PER_FIELD", Some(0));
    let imm = r.place_on_field(1, "IMMUNE_DIGI_ONLY", Some(0));
    assert_eq!(dp(&r, imm), 11000, "precondition: the formula aura affects it");
    r.fire_on_play(1, imm.index as usize);
    resolve_prompts(&mut r);
    assert_eq!(dp(&r, imm), 12000, "immune: the formula aura doesn't affect it");
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    assert_eq!(dp(&r, imm), 11000, "the immunity ended: affected again");
}

// ─── §15-2-1-1: a departed token's deferred effect ──────────────────────────

/// §15-2-1-1 (as above). §4-21-5 (a token that leaves the battle area is
/// removed from the game).
///
/// P0's `TOKEN_OD_MINUS_3K` ("[On Deletion] 1 of your opponent's Digimon gets
/// -3000 DP for the turn") attacks P1's suspended `IMMUNE_DIGI_ONLY` (P0's
/// Digimon effects don't affect it until the end of P0's turn), loses and is
/// deleted. Its [On Deletion] offers P1's two Digimon; P0 chooses the immune
/// one. The -3000 DP is applied after the choice, by a context rebuilt when
/// the token is already out of the game -- it is still the token's Digimon
/// effect, so the immune Digimon is not affected: 12000 DP. Control: chosen,
/// P1's `LV5_PLAIN` (7000 DP) is affected: 4000 DP.
#[test]
fn departed_tokens_deferred_effect_is_still_a_digimon_effect() {
    cite!(
        "15-2-1-1",
        "1b26b455c5b8ddc666117716e988f4eaf37a39f7c247beb3eba6b1a6334497a2"
    );
    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    let plain = r.place_on_field(1, "LV5_PLAIN", Some(0));
    set_suspended(&mut r, imm, true);
    let token = r.place_on_field(0, "TOKEN_OD_MINUS_3K", Some(0));
    r.attack_digimon(token, imm, false);
    assert!(!on_field(&r, 0, "TOKEN_OD_MINUS_3K"), "the token lost the battle");
    let sel = r.pending_selection().expect("the token's [On Deletion] choice");
    assert_eq!(sel.selecting_player, 0, "{sel:?}");
    assert_eq!(sel.valid_action_ids.len(), 2, "both of P1's Digimon are offered: {sel:?}");
    pick_field(&mut r, imm.index as u16);
    resolve_prompts(&mut r);
    assert_eq!(
        dp(&r, imm),
        12000,
        "the departed token's effect is a Digimon effect: it doesn't affect the immune Digimon"
    );
    assert_eq!(dp(&r, plain), 7000, "the other Digimon was not chosen");

    let mut r = board(&[], &[FILL; 3]);
    let imm = play_and_resolve(&mut r, 1, "IMMUNE_DIGI_ONLY");
    let plain = r.place_on_field(1, "LV5_PLAIN", Some(0));
    set_suspended(&mut r, imm, true);
    let token = r.place_on_field(0, "TOKEN_OD_MINUS_3K", Some(0));
    r.attack_digimon(token, imm, false);
    pick_field(&mut r, plain.index as u16);
    resolve_prompts(&mut r);
    assert_eq!(dp(&r, plain), 4000, "control: the chosen plain Digimon is affected");
}
