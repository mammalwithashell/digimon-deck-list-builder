//! ST19-09 Pandamon.
//! Printed text covered here: <Blocker>. [On Deletion] You may play 1 level 3
//! Puppet Digimon card from your hand without paying the cost.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_09_has_blocker_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-09")
        .expect("ST19-09 YAML loads")
        .start();
    let panda = runner.place_on_field(0, "ST19-09", Some(0));

    assert!(
        runner.game.has_keyword(panda, Keyword::Blocker),
        "ST19-09 has printed Blocker"
    );
}

#[test]
fn st19_09_on_deletion_may_play_level3_puppet_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-09")
        .expect("ST19-09 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .add_card(make_puppet_level4("PUPPET-L4"))
        .hand(0, &["PUPPET-L3", "PUPPET-L4"])
        .start();
    let panda = runner.place_on_field(0, "ST19-09", Some(0));

    runner
        .game
        .delete_permanent_with_cause(panda, ReplacementCause::OpponentEffect);

    assert!(runner.pending_is_optional(), "printed play effect says may");
    let view = runner
        .pending_selection_view()
        .expect("level 3 Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the level 3 Puppet is listed as a candidate"
    );
    let play_action = view.valid_action_ids[0];

    runner
        .execute_action(0, play_action)
        .expect("choose level 3 Puppet");
    runner.auto_resolve().expect("finish play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"),
        "selected level 3 Puppet enters battle area"
    );
    let hand_ids = runner.game.players[0]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect::<Vec<_>>();
    assert_eq!(hand_ids, vec!["PUPPET-L4".to_string()]);
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    make_puppet(id, 3)
}

fn make_puppet_level4(id: &str) -> digimon_engine::card_data::CardData {
    make_puppet(id, 4)
}

fn make_puppet(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.traits = vec!["Puppet".to_string()];
    card
}

// ─── Candidate legality (select-hand audit 2026-11-13) ───────────────────────
// DCGO ST19_09.cs:44-51 offers `IsDigimon && HasLevel && IsLevel3 &&
// ContainsTraits("Puppet") && CanPlayAsNewPermanent(payCost: false)`.
// `CanPlayAsNewPermanent` (GameContextDeterminarion.cs:419-433) refuses an
// Option-kind card — a DUAL's kinds contain Option — and needs an empty frame
// and no play lock. CR 4.3 4-6-2: a DUAL is digivolved or used, never played
// (Sistermon Blanc (Awakened) (EX13-065) is a Lv.3 [Puppet] DUAL); 1-3-11-1: a
// card that can't be played can't be declared for play. Regression guards for
// `can_play_from_effect_without_cost`.

fn make_puppet_dual(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_puppet(id, 3);
    card.card_kind = digimon_engine::enums::CardKind::Dual;
    card
}

/// Delete Pandamon with `hand` in hand after `setup`; returns the runner and the
/// hand card ids the [On Deletion] pick offers (empty when no pick installs).
fn on_deletion_offered(
    hand: &[&str],
    setup: impl FnOnce(&mut DebugRunner),
) -> (DebugRunner, Vec<String>) {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-09")
        .expect("ST19-09 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .add_card(make_puppet_dual("PUPPET-DUAL"))
        .hand(0, hand)
        .start();
    let panda = runner.place_on_field(0, "ST19-09", Some(0));
    setup(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(panda, ReplacementCause::OpponentEffect);
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let offered = runner
        .pending_selection_view()
        .filter(|v| v.kind == SelectionKind::Hand)
        .map(|v| {
            v.valid_action_ids
                .iter()
                .filter_map(|&a| {
                    [
                        digimon_engine::action::space::PLAY_HAND_START,
                        digimon_engine::action::space::HAND_EFFECT_START,
                    ]
                    .into_iter()
                    .find(|&base| a >= base && ((a - base) as usize) < hand_ids.len())
                    .map(|base| hand_ids[(a - base) as usize].clone())
                })
                .collect()
        })
        .unwrap_or_default();
    (runner, offered)
}

/// A Lv.3 [Puppet] DUAL card is never played: not offered.
#[test]
fn st19_09_on_deletion_does_not_offer_level3_puppet_dual() {
    let (_r, offered) = on_deletion_offered(&["PUPPET-DUAL", "PUPPET-L3"], |_| {});
    assert_eq!(offered, vec!["PUPPET-L3".to_string()], "the DUAL is not a play candidate");
}

/// "You can't play Digimon by effects": nothing is offered and the Puppet stays
/// in hand.
#[test]
fn st19_09_on_deletion_not_offered_when_effect_digimon_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, offered) = on_deletion_offered(&["PUPPET-L3"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(offered.is_empty(), "nothing playable is offered: {offered:?}");
    let _ = runner.auto_resolve();
    assert!(!runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"));
}
