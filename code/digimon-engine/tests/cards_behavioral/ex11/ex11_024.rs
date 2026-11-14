//! EX11-024 Cendrillmon.
//! Printed text covered here:
//! - <Alliance>.
//! - <Overclock ([Puppet] Trait)>.
//! - [On Play] [When Digivolving] optional free-play of 1 level 4 or lower
//!   Puppet Digimon from hand, then optional Familiar Token generation equal
//!   to the opponent's Digimon count.
//! - [When Digivolving] [When Attacking] -3000 DP to 1 opponent Digimon for
//!   each of your Digimon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn familiar_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count()
}

fn make_digimon(
    id: &str,
    level: u8,
    dp: i32,
    traits: &[&str],
) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

#[test]
fn ex11_024_has_alliance_and_puppet_overclock_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .start();
    let cendrill = runner.place_on_field(0, "EX11-024", Some(0));

    assert!(runner.game.has_keyword(cendrill, Keyword::Alliance));
    assert!(runner.game.has_keyword(cendrill, Keyword::Overclock));
}

#[test]
fn ex11_024_on_play_may_play_level4_or_lower_puppet_then_tokens_per_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .dsl_card("EX11-019")
        .expect("EX11-019 YAML loads")
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_digimon("NON-PUPPET-L4", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .add_card(make_digimon("OPP-B", 4, 5000, &[]))
        .hand(0, &["EX11-024", "EX11-019", "EX7-027", "NON-PUPPET-L4"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));

    runner.play(0, 0).expect("Cendrillmon plays from hand");

    // DCGO EX11_024.cs:52-72: the "you may" is the hand pick itself (PASS
    // declines); no separate "play a Puppet?" choice precedes it.
    let hand_pick = runner
        .pending_selection_view()
        .expect("level 4 or lower Puppet hand prompt");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    assert!(hand_pick.is_optional, "the Puppet play is optional (PASS declines)");
    assert_eq!(
        hand_pick.valid_action_ids,
        vec![0],
        "only the level 4 Puppet is eligible"
    );
    runner
        .execute_action(0, hand_pick.valid_action_ids[0])
        .expect("select level 4 Puppet");

    let token_choice = runner
        .pending_selection_view()
        .expect("optional Familiar Token choice");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[0])
        .expect("choose token generation");
    // The play moved the memory to the opponent's side: the turn ends and
    // <Overclock> (§16-34-2) offers its cost pick; decline it.
    runner
        .auto_resolve_declining_end_of_turn_attacks()
        .expect("finish On Play effect");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-019"),
        "selected Puppet enters the battle area"
    );
    assert_eq!(
        familiar_count(&runner, 0),
        2,
        "one Familiar Token is played for each opponent Digimon"
    );
    assert_eq!(
        runner.hand_size(0),
        2,
        "ineligible level 5 Puppet and non-Puppet level 4 remain in hand"
    );
}

#[test]
fn ex11_024_declining_hand_play_still_offers_token_generation() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("PUPPET-L4", 4, 4000, &["Puppet"]))
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .hand(0, &["EX11-024", "PUPPET-L4"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));

    runner.play(0, 0).expect("Cendrillmon plays from hand");

    let hand_pick = runner
        .pending_selection_view()
        .expect("optional Puppet hand pick");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline hand play");

    let token_choice = runner
        .pending_selection_view()
        .expect("token choice still appears after declining hand play");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[0])
        .expect("choose token generation");
    // The play moved the memory to the opponent's side: the turn ends and
    // <Overclock> (§16-34-2) offers its cost pick; decline it.
    runner
        .auto_resolve_declining_end_of_turn_attacks()
        .expect("finish token generation");

    assert_eq!(runner.hand_size(0), 1, "Puppet card remains in hand");
    assert_eq!(familiar_count(&runner, 0), 1, "one token is played");
}

#[test]
fn ex11_024_when_digivolving_reduces_one_opponent_digimon_by_3000_per_own_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("BASE", 5, 7000, &[]))
        .add_card(make_digimon("ALLY-A", 3, 2000, &[]))
        .add_card(make_digimon("ALLY-B", 4, 4000, &[]))
        .add_card(make_digimon("OPP-A", 5, 10000, &[]))
        .start();
    let cendrill = runner.place_stack(0, &["BASE", "EX11-024"]);
    runner.place_on_field(0, "ALLY-A", Some(0));
    runner.place_on_field(0, "ALLY-B", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let trigger_order = runner
        .pending_selection_view()
        .expect("choose order for Cendrillmon's When Digivolving triggers");
    assert_eq!(trigger_order.kind, SelectionKind::TriggerOrder);
    runner
        .execute_action(0, trigger_order.valid_action_ids[0])
        .expect("resolve hand/token trigger first");

    // Empty hand: no Puppet can be played, so no hand pick opens (DCGO
    // EX11_024.cs:52) and the clause goes straight to the token choice.
    let token_choice = runner
        .pending_selection_view()
        .expect("token choice resolves before DP clause");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[1])
        .expect("decline token generation");

    let dp_target = runner
        .pending_selection_view()
        .expect("opponent Digimon DP target prompt");
    assert_eq!(dp_target.kind, SelectionKind::OppField);
    assert_eq!(dp_target.valid_action_ids.len(), 1);
    runner
        .execute_action(0, dp_target.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP clause");

    assert_eq!(
        runner.effective_dp(target),
        Some(1000),
        "three own Digimon produce -9000 DP for the turn"
    );
}

#[test]
fn ex11_024_when_attacking_reduces_one_opponent_digimon_by_3000_per_own_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("ALLY", 3, 2000, &[]))
        .add_card(make_digimon("OPP-A", 5, 10000, &[]))
        .start();
    let cendrill = runner.place_on_field(0, "EX11-024", Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let dp_target = runner
        .pending_selection_view()
        .expect("opponent Digimon DP target prompt");
    assert_eq!(dp_target.kind, SelectionKind::OppField);
    runner
        .execute_action(0, dp_target.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP clause");

    assert_eq!(
        runner.effective_dp(target),
        Some(4000),
        "two own Digimon produce -6000 DP for the turn"
    );
}

// ─── Candidate legality (CR 4.3 4-6-1..4-6-3 / 7-1-1 / 1-3-2) ────────────────
// DCGO EX11_024.cs:42-48 offers IsDigimon && EqualsTraits("Puppet") && Level<=4
// and opens the hand pick (canNoSelect: true) only when such a card is in hand
// (EX11_024.cs:52); there is no separate "play a Puppet?" yes/no. DCGO's
// `IsDigimon` is `CardKinds.Contains(Digimon)` (CardSource.cs:3519), true for a
// DUAL card, so DCGO offers a [Puppet] DUAL that `PlayPermanentCards` then
// refuses (`CanPlayAsNewPermanent` requires `!IsOption`). Under the rules a DUAL
// card is digivolved or used as an Option (CR 4.3 4-6-1, 4-6-2), is a Digimon
// only once on the field (4-6-3), and its play cost prints "Can't play to the
// field" (7-1-1), so the play would not be carried out (1-3-2). The pick is
// optional, so not offering it changes no outcome. Same for a Puppet with no free
// battle-area slot (engine `rules.field_slots`, not a CR limit, CR 4.3 3-4-8-2).

/// Play Cendrillmon (EX11-024) from hand index 0 with `others` in hand, after
/// `setup`. Returns the card ids the Puppet hand pick offers (empty when none
/// installs).
fn puppet_pick_offers(others: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut hand = vec!["EX11-024"];
    hand.extend_from_slice(others);
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .dsl_card("EX13-065")
        .expect("EX13-065 YAML loads")
        .dsl_card("EX13-066")
        .expect("EX13-066 YAML loads")
        .add_card(make_digimon("PUPPET-L4", 4, 4000, &["Puppet"]))
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .hand(0, &hand)
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));
    setup(&mut runner);
    runner.play(0, 0).expect("Cendrillmon plays from hand");
    let Some(view) = runner
        .pending_selection_view()
        .filter(|v| v.kind == SelectionKind::Hand)
    else {
        return Vec::new();
    };
    let ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut offered: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .map(|a| {
            ids[(a - digimon_engine::action::space::PLAY_HAND_START) as usize].clone()
        })
        .collect();
    offered.sort();
    offered
}

/// Sistermon Blanc (Awakened) (EX13-065, Lv.3) and Sistermon Noir (Awakened)
/// (EX13-066, Lv.4) are level 4 or lower [Puppet] DUAL cards: they cannot be
/// played, so only the playable Puppet is offered.
#[test]
fn ex11_024_puppet_pick_excludes_dual_cards() {
    assert_eq!(
        puppet_pick_offers(&["EX13-065", "EX13-066", "PUPPET-L4"], |_| {}),
        vec!["PUPPET-L4".to_string()]
    );
}

/// Cendrillmon itself fills a 1-slot battle area: no Puppet is offered. The
/// first decision must be the Familiar Token choice (not a Puppet hand pick and
/// not a "play a Puppet?" yes/no), and the playable-by-text Puppet stays in hand.
#[test]
fn ex11_024_puppet_pick_offers_nothing_when_battle_area_full() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("PUPPET-L4", 4, 4000, &["Puppet"]))
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .hand(0, &["EX11-024", "PUPPET-L4"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8 + 1;
    runner.play(0, 0).expect("Cendrillmon plays from hand");

    let view = runner
        .pending_selection_view()
        .expect("the Familiar Token choice is offered");
    assert_eq!(
        view.kind,
        SelectionKind::EffectChoice,
        "no Puppet hand pick opens into a full battle area"
    );
    assert!(
        view.prompt.contains("Familiar"),
        "the first decision is the Familiar Token choice, not a Puppet play: {:?}",
        view.prompt
    );
    runner
        .execute_action(0, view.valid_action_ids[1])
        .expect("decline token generation");
    runner
        .auto_resolve_declining_end_of_turn_attacks()
        .expect("finish On Play effect");

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(hand_ids, vec!["PUPPET-L4".to_string()], "the Puppet stays in hand");
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L4"),
        "the Puppet is not played"
    );
}

/// With only an unplayable Puppet (a DUAL) in hand there is nothing to play, so
/// no "play a Puppet" decision is offered at all (DCGO EX11_024.cs:52 gates the
/// pick on a candidate); the clause goes straight to the Familiar Token choice.
#[test]
fn ex11_024_no_playable_puppet_offers_no_puppet_decision() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .dsl_card("EX13-065")
        .expect("EX13-065 YAML loads")
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .hand(0, &["EX11-024", "EX13-065"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.play(0, 0).expect("Cendrillmon plays from hand");
    let view = runner
        .pending_selection_view()
        .expect("the Familiar Token choice is offered");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    assert!(
        view.prompt.contains("Familiar"),
        "the first decision is the Familiar Token choice, not a Puppet play: {:?}",
        view.prompt
    );
}
