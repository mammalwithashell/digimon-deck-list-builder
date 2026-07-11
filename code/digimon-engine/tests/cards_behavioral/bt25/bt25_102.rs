//! BT25-102 Factorial Area — Option, Black/Red, Iliad/TS.
//!
//! Printed text (official Bandai DB bundle):
//!   While you have no face-up security cards, you can ignore this card's
//!   color requirements.
//!   {Security} [All Turns] All of your black or red [TS] trait Digimon gain
//!     ＜Blocker＞. While you have [Vulcanusmon], they also gain ＜Link +1＞.
//!   [Main] Add your bottom security card to the hand and place this card face
//!     up as the bottom security card. Then, you may play 1 black or red [TS]
//!     trait Digimon card from your hand with the cost reduced by 3.
//!   [Security] You may play 1 level 4 or lower black or red [TS] trait Digimon
//!     card from your hand or trash without paying the cost.
//!
//! DCGO oracle: BT25_102.cs. The `{Security} [All Turns]` Blocker /
//! ChangeLinkMax static effects register at `EffectTiming.None` gated on
//! `IsExistInSecurity(card, false)` — i.e. while this Option sits FACE-UP in
//! the owner's security stack (DCGO `IsFlipped == false` == face-up; a normal
//! face-down security card is `IsFlipped == true`). The [Main] effect places
//! this card face-up in security, at which point the aura goes live.
//!
//! This file is the PROOF FIXTURE adjudicating G-ENGINE-SECURITY-ZONE-SOURCED-
//! FIELD-AURA as NOT-A-GAP: the resolved Track H §5 security-zone aura
//! substrate (same path BT24-090 Sanctuary of Darkness uses) already emanates
//! a live field aura from a face-up security card. BT25-102's residual blocker
//! (a security-sourced Blocker + conditional Link+1) is expressible TODAY.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT25-102";

fn ts_digimon(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![color];
    c.traits.push("TS".to_string());
    c
}

fn plain_digimon(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![color];
    c
}

fn named_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.colors = vec![CardColor::Black];
    c.traits.push("TS".to_string());
    c
}

/// TS Digimon with an explicit level and play cost — used by the [Main]
/// (cost-reduced play) and [Security] (level-gated free play) clause tests.
fn ts_digimon_leveled(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.traits = vec!["TS".to_string()];
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = u16::from(level);
    c
}

fn attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(9000);
    c.play_cost = 4;
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn hand_action_for_id(
    runner: &DebugRunner,
    player: digimon_engine::enums::PlayerId,
    id: &str,
) -> u16 {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id)
                .then_some(digimon_engine::action::space::PLAY_HAND_START + idx as u16)
        })
        .expect("hand card exists")
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-102 in embedded DSL pack")
}

// ───────────────────── Structural ─────────────────────────────────────────

#[test]
fn bt25_102_compiles_with_security_auras() {
    let r = builder().start();
    let compiled = r.compiled_card(CARD_ID).expect("compiled BT25-102");
    assert!(
        !compiled.effects.is_empty(),
        "BT25-102 authors at least the [Security][All Turns] aura clauses"
    );
}

/// Structural: the [Main] MainFromHand clause and the [Security] inherited
/// on_security clause both compile with the expected step shapes, alongside
/// the flood-gate and two security auras (5 clauses total).
#[test]
fn bt25_102_metadata_use_requirement_main_security_and_auras_compile() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT25-102");

    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.use_requirement.is_some());
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));
    assert_eq!(
        card.effects
            .iter()
            .filter(|clause| matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    scope: digimon_dsl::compiled::CompiledScope::Security,
                    ..
                })
            ))
            .count(),
        2,
        "security Blocker and conditional Link+1 auras should compile"
    );

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(trigger)
                if trigger.when == vec![CompiledTiming::MainFromHand] =>
            {
                Some(trigger)
            }
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(matches!(
        main.process.as_slice(),
        [
            CompiledStep::AddBottomSecurityToHand { .. },
            CompiledStep::PlaceSelfOptionAtSecurity { .. },
            CompiledStep::SelectHand { .. },
            CompiledStep::PlayFromHand { .. },
        ]
    ));

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(trigger)
            if trigger.scope == digimon_dsl::compiled::CompiledScope::Inherited
                && trigger.when == vec![CompiledTiming::OnSecurity]
    )));
}

// ───────────────────── [Security][All Turns] auras ─────────────────────────

/// While FACE-UP in security, all your black/red [TS] Digimon gain Blocker.
/// Non-TS or off-color Digimon do not.
#[test]
fn bt25_102_face_up_security_grants_blocker_to_black_or_red_ts() {
    let mut runner = builder()
        .add_card(ts_digimon("BLACK-TS", CardColor::Black))
        .add_card(ts_digimon("RED-TS", CardColor::Red))
        .add_card(ts_digimon("GREEN-TS", CardColor::Green))
        .add_card(plain_digimon("BLACK-PLAIN", CardColor::Black))
        .security(0, &[CARD_ID])
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let black_ts = runner.place_on_field(0, "BLACK-TS", Some(0));
    let red_ts = runner.place_on_field(0, "RED-TS", Some(0));
    let green_ts = runner.place_on_field(0, "GREEN-TS", Some(0));
    let black_plain = runner.place_on_field(0, "BLACK-PLAIN", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(black_ts, Keyword::Blocker),
        "black [TS] Digimon gains Blocker"
    );
    assert!(
        runner.game.has_keyword(red_ts, Keyword::Blocker),
        "red [TS] Digimon gains Blocker"
    );
    assert!(
        !runner.game.has_keyword(green_ts, Keyword::Blocker),
        "green [TS] Digimon (off-color) does NOT gain Blocker"
    );
    assert!(
        !runner.game.has_keyword(black_plain, Keyword::Blocker),
        "non-[TS] black Digimon does NOT gain Blocker"
    );
}

/// FACE-DOWN in security (the default) → no aura (matches DCGO
/// `IsExistInSecurity(card, false)`, which requires face-up).
#[test]
fn bt25_102_face_down_security_grants_nothing() {
    let mut runner = builder()
        .add_card(ts_digimon("BLACK-TS", CardColor::Black))
        .security(0, &[CARD_ID])
        .start();
    // Deliberately do NOT mark face_up_security.
    let black_ts = runner.place_on_field(0, "BLACK-TS", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(black_ts, Keyword::Blocker),
        "face-down security source must NOT grant the [Security] aura"
    );
    assert_eq!(
        runner.game.modifiers.link_max_delta(black_ts),
        0,
        "no Link+1 while face-down"
    );
}

/// While you have [Vulcanusmon] on the field, black/red [TS] Digimon ALSO get
/// Link +1 (ChangeLinkMax modifier). Without Vulcanusmon, only Blocker.
#[test]
fn bt25_102_link_plus_one_only_with_vulcanusmon() {
    // Without Vulcanusmon: Blocker but no Link+1.
    let mut runner = builder()
        .add_card(ts_digimon("BLACK-TS", CardColor::Black))
        .security(0, &[CARD_ID])
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);
    let black_ts = runner.place_on_field(0, "BLACK-TS", Some(0));
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(black_ts, Keyword::Blocker));
    assert_eq!(
        runner.game.modifiers.link_max_delta(black_ts),
        0,
        "no Link+1 without [Vulcanusmon]"
    );

    // With Vulcanusmon on the field: Link +1 kicks in.
    let mut runner = builder()
        .add_card(ts_digimon("BLACK-TS", CardColor::Black))
        .add_card(named_digimon("VULCAN", "Vulcanusmon"))
        .security(0, &[CARD_ID])
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);
    let black_ts = runner.place_on_field(0, "BLACK-TS", Some(0));
    runner.place_on_field(0, "VULCAN", Some(0));
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(black_ts, Keyword::Blocker));
    assert_eq!(
        runner.game.modifiers.link_max_delta(black_ts),
        1,
        "with [Vulcanusmon], black/red [TS] Digimon gain Link +1"
    );
}

/// The aura evicts when the source leaves the security stack (clear+re-install
/// materialization model — no explicit OnLoseSecurity wiring needed).
#[test]
fn bt25_102_aura_evicts_when_leaving_security() {
    let mut runner = builder()
        .add_card(ts_digimon("BLACK-TS", CardColor::Black))
        .security(0, &[CARD_ID])
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);
    let black_ts = runner.place_on_field(0, "BLACK-TS", Some(0));

    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(black_ts, Keyword::Blocker));

    // Source leaves security.
    runner.game.players[0].security.clear();
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(black_ts, Keyword::Blocker),
        "Blocker evicts on next tick after BT25-102 leaves security"
    );
}

/// Owner-scoped: an OPPONENT's black [TS] Digimon does not benefit from your
/// security-sourced aura.
#[test]
fn bt25_102_aura_is_owner_scoped() {
    let mut runner = builder()
        .add_card(ts_digimon("OPP-BLACK-TS", CardColor::Black))
        .security(0, &[CARD_ID])
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);
    let opp = runner.place_on_field(1, "OPP-BLACK-TS", Some(0));

    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(opp, Keyword::Blocker),
        "opponent's black [TS] Digimon must NOT gain your security aura"
    );
}

// ───────────────────── [Main] add bottom security + replace + play -3 ─────

/// [Main] adds the bottom security card to hand, places BT25-102 face up as
/// the new bottom security card, then may play 1 black/red [TS] Digimon from
/// hand with the play cost reduced by 3. Off-color / non-TS cards are not
/// offered.
#[test]
fn bt25_102_main_replaces_bottom_security_with_self_face_up_and_plays_reduced_ts() {
    let mut runner = builder()
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(ts_digimon_leveled("BLACK-TS", CardColor::Black, 6))
        .add_card(ts_digimon_leveled("RED-TS", CardColor::Red, 4))
        .add_card(ts_digimon_leveled("GREEN-TS", CardColor::Green, 4))
        .hand(0, &[CARD_ID, "BLACK-TS", "RED-TS", "GREEN-TS"])
        .security(0, &["BOTTOM", "TOP"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "no face-up security should satisfy the option's color bypass"
    );
    let hand_prompt = runner
        .pending_selection_view()
        .expect("reduced play prompt");
    assert_eq!(hand_prompt.kind, SelectionKind::Hand);
    assert!(hand_prompt.is_optional);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID,
        "Factorial Area should be placed as the bottom security card"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&runner.game.players[0].security[0].card_index),
        "placed security card must be face-up"
    );
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "BOTTOM"));

    let black_action = hand_action_for_id(&runner, 0, "BLACK-TS");
    assert!(hand_prompt.valid_action_ids.contains(&black_action));
    assert!(!hand_prompt
        .valid_action_ids
        .contains(&hand_action_for_id(&runner, 0, "GREEN-TS")));
    runner
        .execute_action(hand_prompt.selecting_player, black_action)
        .expect("play black TS with reduced cost");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BLACK-TS"));
    assert_eq!(
        runner.memory(),
        memory_before - 6,
        "option cost 3 plus play cost 6 reduced by 3 should spend 6 memory total"
    );
}

/// The [Main] hand selection is optional — passing leaves the security swap
/// intact but plays no Digimon.
#[test]
fn bt25_102_main_hand_selection_is_optional() {
    let mut runner = builder()
        .add_card(filler("BOTTOM"))
        .add_card(ts_digimon_leveled("BLACK-TS", CardColor::Black, 6))
        .hand(0, &[CARD_ID, "BLACK-TS"])
        .security(0, &["BOTTOM"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let hand_prompt = runner
        .pending_selection_view()
        .expect("reduced play prompt");
    assert!(
        hand_prompt.is_optional,
        "PASS is legal for an optional selection via is_optional, not literal presence in valid_action_ids"
    );
    runner
        .execute_action(hand_prompt.selecting_player, PASS)
        .expect("decline to play a Digimon");

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BLACK-TS"),
        "declined Digimon must not be played"
    );
    assert_eq!(
        runner.memory(),
        memory_before - 3,
        "only the option's own cost (3) should be spent when declining the free play"
    );
}

// ───────────────────── [Security] play lvl<=4 black/red [TS] free ─────────

/// [Security] may play 1 level 4 or lower black/red [TS] Digimon from hand
/// or trash without paying the cost. A level 5+ candidate is not offered.
#[test]
fn bt25_102_security_effect_plays_level_four_black_or_red_ts_from_hand_or_trash_free() {
    let mut runner = builder()
        .add_card(ts_digimon_leveled("HAND-TS", CardColor::Red, 4))
        .add_card(ts_digimon_leveled("TRASH-TS", CardColor::Black, 4))
        .add_card(ts_digimon_leveled("HIGH-TS", CardColor::Black, 5))
        .add_card(attacker("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["HAND-TS", "HIGH-TS"])
        .deck(1, &["TRASH-TS"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let trash_card = runner.game.players[1].deck.pop().expect("trash seed");
    runner.game.players[1].trash.push(trash_card);
    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    let union = runner
        .pending_selection_view()
        .expect("hand/trash union selection");
    assert!(union.is_optional);
    let chosen = union
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .expect("eligible hand or trash card");
    runner
        .execute_action(union.selecting_player, chosen)
        .expect("play eligible TS card");
    runner.auto_resolve().expect("settle security effect");

    assert!(runner.game.players[1].battle_area.iter().any(|perm| {
        let id = perm.top_card().card_id(&runner.game.card_data);
        id == "HAND-TS" || id == "TRASH-TS"
    }));
    assert_eq!(runner.memory(), memory_before, "security play is free");
    assert!(!runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "HIGH-TS"));
}

/// The [Security] free play is optional — passing leaves the security check
/// resolution (card to trash) undisturbed and plays nothing.
#[test]
fn bt25_102_security_effect_is_optional() {
    let mut runner = builder()
        .add_card(ts_digimon_leveled("HAND-TS", CardColor::Red, 4))
        .add_card(attacker("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["HAND-TS"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    let union = runner
        .pending_selection_view()
        .expect("hand/trash union selection");
    assert!(
        union.is_optional,
        "PASS is legal for an optional selection via is_optional, not literal presence in valid_action_ids"
    );
    runner
        .execute_action(union.selecting_player, PASS)
        .expect("decline the free security play");
    runner.auto_resolve().expect("settle security effect");

    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "HAND-TS"),
        "declined Digimon must not be played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory spent when declining"
    );
}
