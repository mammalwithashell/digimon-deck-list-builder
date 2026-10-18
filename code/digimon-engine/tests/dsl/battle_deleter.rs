//! G-DSL-BATTLE-DELETER — `event_battle_deleter: { <permanent predicate> }` on
//! an `on_ally_won_battle` (EndOfBattle) observer: "When any of your Digimon
//! with [X] in their texts delete your opponent's Digimon in battle" (EX13-041
//! Groundramon). A deleter is a combatant whose battle
//! opponent was actually deleted, including mutual deletion. Event identity
//! is captured before either combatant leaves the battle area.

use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

/// Observer: when any of your Digimon named [Hero] deletes an opponent's
/// Digimon in battle, trash the opponent's top security card.
const OBSERVER: &str = r#"
card: BD-OBS
name: Deleter Observer
kind: digimon
level: 4
color: [red]
cost: 4
dp: 1000
effects:
  - when: on_ally_won_battle
    condition:
      event_battle_deleter:
        owner: you
        kind: digimon
        name_is: Hero
    summary: "When your [Hero] deletes an opponent's Digimon in battle, trash their top security card"
    process:
      - trash_top_security: { of: opponent }
"#;

const DELETION_TARGET: &str = r#"
card: BD-DYING
name: Dying Opponent
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
effects:
  - when: on_deletion
    optional: true
    process:
      - gain_memory: 1
"#;

fn card(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card_with_level(id, name, 5);
    c.colors = vec![CardColor::Red];
    c.dp = Some(dp);
    c
}

fn runner() -> DebugRunner {
    runner_with(OBSERVER)
}

fn runner_with(observer: &str) -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(observer)
        .expect("observer compiles")
        .from_dsl_yaml(DELETION_TARGET)
        .expect("deletion target compiles")
        .add_card(card("BD-HERO", "Hero", 8000))
        .add_card(card("BD-HERO-TIE", "Hero", 5000))
        .add_card(card("BD-VILLAIN", "Villain", 5000))
        .add_card(card("BD-BRUTE", "Brute", 12000))
        .add_card(card("BD-PAD", "Pad", 1000))
        .add_card(card("BD-OTHER-TIE", "Other", 5000))
        .deck(0, &["BD-PAD"; 12])
        .deck(1, &["BD-PAD"; 12])
        .security(0, &["BD-PAD"; 4])
        .security(1, &["BD-PAD"; 4])
        .start()
}

fn protect(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.game.modifiers.add(
        h,
        ModifierEntry::simple(
            ModifierType::CannotBeDestroyedByBattle,
            0,
            Expiry::Permanent,
            h.player,
        ),
    );
}

#[test]
fn own_winner_that_deletes_its_opponent_fires() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "villain deleted in battle");
    assert_eq!(r.security_count(1), 3, "observer trashed 1 security");
}

#[test]
fn prevented_loser_deletion_does_not_fire() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    protect(&mut r, villain);
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "villain survived (protected)");
    assert_eq!(r.security_count(1), 4, "no deletion → no trigger");
}

#[test]
fn deleter_must_match_the_inner_predicate() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let brute = r.place_on_field(0, "BD-BRUTE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(brute, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.security_count(1), 4, "Brute is not [Hero]");
}

#[test]
fn opponent_deleter_does_not_satisfy_owner_you() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let brute = r.place_on_field(1, "BD-BRUTE", Some(0));
    r.battle_digimon(hero, brute);
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(1),
        4,
        "the opponent's Digimon was the deleter"
    );
}

#[test]
fn mutual_destruction_preserves_the_deleters_identity_for_surviving_observers() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(
        r.security_count(1),
        3,
        "Hero deleted its battle opponent even though Hero also left"
    );
}

#[test]
fn tie_with_a_protected_survivor_counts_as_deleting() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    protect(&mut r, hero);
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "villain deleted");
    assert_eq!(r.battle_area_size(0), 2, "hero survived the tie");
    assert_eq!(
        r.security_count(1),
        3,
        "hero deleted its opponent in battle"
    );
}

fn choose(r: &mut DebugRunner, decline: bool) {
    use digimon_engine::action::{build_action_mask, space::PASS};
    let view = r.pending_selection_view().expect("choice");
    let action = if decline {
        PASS
    } else {
        *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap()
    };
    assert!(build_action_mask(&r.game, view.selecting_player)[action as usize] > 0.5);
    r.execute_action(view.selecting_player, action).unwrap();
}

#[test]
fn mutual_battle_notifies_both_sides_once_using_immutable_names_and_owners() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    r.place_on_field(1, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let opponent = r.place_on_field(1, "BD-HERO-TIE", Some(0));
    r.battle_digimon(hero, opponent);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(0), 3);
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn matching_both_mutual_deleters_emits_only_one_observer_event() {
    let mut r = runner_with(&OBSERVER.replace("owner: you", "owner: any"));
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let opponent = r.place_on_field(1, "BD-HERO-TIE", Some(0));
    r.battle_digimon(hero, opponent);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn deleted_deleter_never_borrows_a_shifted_slots_identity() {
    for qualifying in [false, true] {
        let mut r = runner();
        let hero = r.place_on_field(
            0,
            if qualifying {
                "BD-HERO-TIE"
            } else {
                "BD-OTHER-TIE"
            },
            Some(0),
        );
        r.place_on_field(
            0,
            if qualifying {
                "BD-OTHER-TIE"
            } else {
                "BD-HERO"
            },
            Some(0),
        );
        r.place_on_field(0, "BD-OBS", Some(0));
        let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
        r.battle_digimon(hero, villain);
        r.auto_resolve().unwrap();
        assert_eq!(r.security_count(1), if qualifying { 3 } else { 4 });
    }
}

#[test]
fn observer_or_inherited_host_that_dies_cannot_activate() {
    for inherited in [false, true] {
        let observer = OBSERVER
            .replace("dp: 1000", "dp: 5000")
            .replace("name: Deleter Observer", "name: Hero");
        let observer = if inherited {
            observer.replace("  - when:", "  - scope: inherited\n    when:")
        } else {
            observer
        };
        let mut r = runner_with(&observer);
        let host = if inherited {
            r.place_stack(0, &["BD-OBS", "BD-HERO-TIE"])
        } else {
            r.place_on_field(0, "BD-OBS", Some(0))
        };
        let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
        r.battle_digimon(host, villain);
        r.auto_resolve().unwrap();
        assert_eq!(r.battle_area_size(0), 0);
        assert_eq!(r.security_count(1), 4);
    }
}

#[test]
fn battle_observer_waits_for_actual_replacement_outcome_and_clones() {
    use digimon_engine::enums::Keyword;
    for mutual in [false, true] {
        let mut r = runner();
        r.place_on_field(0, "BD-OBS", Some(0));
        let hero = r.place_on_field(0, if mutual { "BD-HERO-TIE" } else { "BD-HERO" }, Some(0));
        let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
        r.game
            .modifiers
            .grant_keyword(villain, Keyword::Evade, Expiry::Permanent, 1);
        r.battle_digimon(hero, villain);
        assert!(r.pending_selection().is_some());
        assert_eq!(
            r.security_count(1),
            4,
            "no EndOfBattle before replacement resolves"
        );
        let mut declined = DebugRunner::wrap(r.game.clone());
        choose(&mut r, false);
        choose(&mut declined, true);
        for game in [&mut r, &mut declined] {
            game.auto_resolve().unwrap();
        }
        assert_eq!(
            r.security_count(1),
            4,
            "Evade prevented the opponent's deletion"
        );
        assert_eq!(
            declined.security_count(1),
            3,
            "declined replacement completed a battle deletion"
        );
        assert_eq!(r.battle_area_size(1), 1);
        assert_eq!(declined.battle_area_size(1), 0);
    }
}

#[test]
fn mutual_snapshot_text_and_optional_observer_survive_clone() {
    let observer = OBSERVER
        .replace(
            "        name_is: Hero",
            "        any_of: [{ in_text_contains: Hero }, { in_text_contains: Examon }]",
        )
        .replace("    summary:", "    optional: true\n    summary:");
    let mut r = runner_with(&observer);
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(hero, villain);
    assert!(r.pending_selection().is_some());
    let mut declined = DebugRunner::wrap(r.game.clone());
    choose(&mut r, false);
    choose(&mut declined, true);
    assert_eq!(r.security_count(1), 3);
    assert_eq!(declined.security_count(1), 4);
}

#[test]
fn declared_attack_evade_outcome_and_attack_cleanup_survive_clone() {
    use digimon_engine::action::{build_action_mask, space::encode_attack};
    use digimon_engine::combat::AttackResult;
    use digimon_engine::enums::{GamePhase, Keyword};
    for mutual in [false, true] {
        let mut r = runner();
        r.skip_mulligan();
        r.set_first_player(0);
        r.set_phase(GamePhase::Main);
        r.place_on_field(0, "BD-OBS", Some(0));
        let hero = r.place_on_field(0, if mutual { "BD-HERO-TIE" } else { "BD-HERO" }, Some(0));
        let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
        // Evade needs an unsuspended defender, so explicitly grant the legal
        // ability to attack that target instead of bypassing its action mask.
        r.game.modifiers.add(
            hero,
            ModifierEntry::simple(ModifierType::CanAttackUnsuspended, 0, Expiry::Permanent, 0),
        );
        r.game
            .modifiers
            .grant_keyword(villain, Keyword::Evade, Expiry::Permanent, 1);
        let action = encode_attack(hero.index as u16, villain.index as u16);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
        assert_eq!(
            r.attack_digimon(hero, villain, false),
            AttackResult::InProgress
        );
        assert!(
            r.game.pending_attack.is_some(),
            "attack must await its battle deletion"
        );
        assert_eq!(r.security_count(1), 4);
        let mut declined = DebugRunner::wrap(r.game.clone());
        choose(&mut r, false);
        choose(&mut declined, true);
        for game in [&mut r, &mut declined] {
            game.auto_resolve().unwrap();
            assert!(
                game.game.pending_attack.is_none(),
                "attack cleanup must finish once"
            );
            assert!(game.pending_selection().is_none());
        }
        assert_eq!(r.security_count(1), 4);
        assert_eq!(declined.security_count(1), 3);
        assert_eq!(r.battle_area_size(1), 1);
        assert_eq!(declined.battle_area_size(1), 0);
    }
}

/// §14-2-4 / §15-4-3-2: the "deletes in battle" observer and the deleted
/// Digimon's [On Deletion] are triggered by the same battle and pend together;
/// §15-4-3-5-1: the turn player's activate first. Official Q&A Q3931 (EX8-045
/// Callismon): the battle trigger and the deleted Digimon's [On Deletion]
/// "will trigger simultaneously, then the turn player's will activate first."
/// P0 (the turn player under the runner's even seed) owns the optional
/// observer, P1 the optional [On Deletion]: P0 decides first, then P1. (This
/// test used to pin the reverse -- P1's [On Deletion] choice before P0's
/// observer -- G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION.)
/// A clone taken at P0's choice resolves independently of the original.
#[test]
fn turn_players_battle_observer_choice_precedes_on_deletion_after_clone() {
    let observer = OBSERVER.replace("    summary:", "    optional: true\n    summary:");
    let mut r = runner_with(&observer);
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    let dying = r.place_on_field(1, "BD-DYING", Some(0));
    assert_eq!(r.game.turn_player(), 0, "P0 is the turn player");
    let memory0 = r.game.memory;
    r.battle_digimon(hero, dying);
    assert_eq!(
        r.battle_area_size(1),
        0,
        "deletion commits before the battle's triggered choices"
    );
    assert_eq!(
        r.pending_selection().unwrap().selecting_player,
        0,
        "the turn player's battle observer activates before the On Deletion"
    );
    assert_eq!(r.security_count(1), 4);
    assert_eq!(r.game.memory, memory0, "the On Deletion waits for the observer");
    let mut cloned = DebugRunner::wrap(r.game.clone());
    for (game, decline_observer) in [(&mut r, false), (&mut cloned, true)] {
        choose(game, decline_observer);
        assert_eq!(
            game.security_count(1),
            if decline_observer { 4 } else { 3 },
            "the observer resolved (or was declined) first"
        );
        assert_eq!(
            game.pending_selection().unwrap().selecting_player,
            1,
            "then the non-turn player's On Deletion"
        );
        assert_eq!(game.game.memory, memory0);
        choose(game, false);
        assert!(game.pending_selection().is_none());
        assert_ne!(game.game.memory, memory0, "the On Deletion resolved last");
    }
}

struct ChangeWinnerOnDeletion {
    winner: PermanentHandle,
    delete_winner: bool,
    dna_winner: bool,
    de_digivolve_winner: bool,
    replay_after_de_digivolve: bool,
}

impl digimon_engine::effect::CardEffect for ChangeWinnerOnDeletion {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let winner = self.winner;
        let delete_winner = self.delete_winner;
        let dna_winner = self.dna_winner;
        let de_digivolve_winner = self.de_digivolve_winner;
        let replay_after_de_digivolve = self.replay_after_de_digivolve;
        vec![digimon_engine::effect::Effect::on_deletion(card)
            .process(move |ctx| {
                if de_digivolve_winner {
                    assert_eq!(ctx.de_digivolve(winner, Some(3), Some(1)), 1);
                    if replay_after_de_digivolve {
                        let card = ctx.game.player(winner.player).battle_area
                            [winner.index as usize]
                            .top_card()
                            .handle();
                        ctx.delete_permanent(winner);
                        assert!(ctx
                            .play_from_trash_free_unsuspended_for(winner.player, card, true)
                            .is_some());
                    }
                } else if delete_winner {
                    ctx.delete_permanent(winner);
                } else if dna_winner {
                    let result = ctx.game.player(winner.player).hand[0].handle();
                    let partner = PermanentHandle {
                        player: winner.player,
                        index: winner.index + 1,
                    };
                    assert!(ctx
                        .effect_initiated_dna_digivolve(winner, partner, result, 0, true)
                        .is_some());
                } else {
                    assert!(ctx.effect_initiated_digivolve_ignore_requirements(
                        winner.player,
                        0,
                        winner,
                        digimon_engine::enums::CostDelta::Free
                    ));
                }
            })
            .build()]
    }
}

/// §15-8-3-1 / §15-8-3-4: a trigger-type effect triggers when its trigger
/// condition is met, and not otherwise; §15-8-3-8: what that condition names
/// is read in the state where the battle triggered it. BD-HERO, which won the
/// battle, has no [Evolved] trait, so "When your [Evolved] Digimon wins a
/// battle" never triggers -- even with P1 the turn player, whose [On Deletion]
/// digivolves the winner into the [Evolved] BD-EVO before P0's pending effects
/// activate. (Withdraws the Task 28 ratification that it activated.)
/// G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER.
#[test]
fn winner_that_evolves_into_the_trait_during_opponents_on_deletion_did_not_trigger() {
    winner_identity_after_deletion(false, false);
}

// Since G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER this passes because the non-[Evolved] winner never triggers the observer at the battle (§15-8-3-4), not because of identity tracking.
#[test]
fn winner_predicates_do_not_follow_a_reused_slot_after_winner_deletion() {
    winner_identity_after_deletion(true, false);
}

// Since G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER this passes because the non-[Evolved] winner never triggers the observer at the battle (§15-8-3-4), not because of identity tracking.
#[test]
fn winner_predicates_do_not_follow_materials_into_a_new_dna_digimon() {
    winner_identity_after_deletion(false, true);
}

fn winner_identity_after_deletion(delete_winner: bool, dna_winner: bool) {
    // P1 the turn player: its [On Deletion] changes the winner first.
    winner_identity_after_deletion_with_turn(delete_winner, dna_winner, 1);
    // P0 the turn player: P0's observer activates first (§15-4-3-5-1) and
    // reads the winner before P1's [On Deletion] changes it. BD-HERO has no
    // [Evolved] trait, so nothing is trashed in any of the three shapes.
    winner_identity_after_deletion_with_turn(delete_winner, dna_winner, 0);
}

fn winner_identity_after_deletion_with_turn(
    delete_winner: bool,
    dna_winner: bool,
    turn_player: usize,
) {
    let observer = OBSERVER.replace(
        "      event_battle_deleter:\n        owner: you\n        kind: digimon\n        name_is: Hero",
        "      event_winner_owner: you\n      event_winner_trait_has: Evolved");
    let mut evolved = card("BD-EVO", "Evolved Hero", 12000);
    evolved.level = Some(6);
    evolved.traits = vec!["Evolved".into()];
    let mut bystander = card("BD-BYSTANDER", "Bystander", 8000);
    bystander.traits = vec!["Evolved".into()];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&observer)
        .unwrap()
        .add_card(card("BD-HERO", "Hero", 8000))
        .add_card(card("BD-VILLAIN", "Villain", 5000))
        .add_card(card("BD-PAD", "Pad", 1000))
        .add_card(evolved)
        .add_card(bystander)
        .hand(0, &["BD-EVO"])
        .deck(0, &["BD-PAD"; 12])
        .deck(1, &["BD-PAD"; 12])
        .security(1, &["BD-PAD"; 4])
        .start();
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    r.place_on_field(0, "BD-BYSTANDER", Some(0));
    r.place_on_field(0, "BD-OBS", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.register_effect(
        "BD-VILLAIN",
        std::sync::Arc::new(ChangeWinnerOnDeletion {
            winner: hero,
            delete_winner,
            dna_winner,
            de_digivolve_winner: false,
            replay_after_de_digivolve: false,
        }),
    );
    // P0's observer and P1's [On Deletion] are triggered by the battle and
    // pend together (§14-2-4, §15-4-3-2); the turn player's activate first
    // (§15-4-3-5-1). With P1 the turn player (as in
    // `ch15_08_effect_categories::winner_deleted_by_loser_board`) the
    // [On Deletion] changes the winner BEFORE the observer reads it; with P0
    // the turn player the observer reads the unchanged winner.
    // G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION.
    r.game.turn_player_idx = turn_player;
    r.battle_digimon(hero, villain);
    r.auto_resolve().unwrap();
    // No [Evolved] Digimon won the battle: the observer never triggers, in
    // any shape or turn order (§15-8-3-1, §15-8-3-8;
    // G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER).
    assert_eq!(r.security_count(1), 4, "turn player P{turn_player}");
    let first = &r.game.players[0].battle_area[0];
    assert_eq!(
        first.top_card().card_id(&r.game.card_data),
        if delete_winner {
            "BD-BYSTANDER"
        } else {
            "BD-EVO"
        }
    );
}

#[test]
fn winner_predicates_follow_de_digivolution_during_opponents_on_deletion() {
    winner_after_de_digivolution(false);
}

#[test]
fn winner_predicates_do_not_follow_a_de_digivolved_body_replayed_from_trash() {
    winner_after_de_digivolution(true);
}

fn winner_after_de_digivolution(replay: bool) {
    // P1 the turn player: its [On Deletion] de-digivolves the winner first.
    // (The P0-turn-player outcome is pinned, ignored, by
    // `turn_players_observer_reads_the_winners_top_card_before_de_digivolution`.)
    winner_after_de_digivolution_with_turn(replay, 1);
}

/// §15-4-3-5-1: with P0 the turn player, P0's observer activates before P1's
/// [On Deletion] de-digivolves the winner; §15-8-3-8 / §15-4-4-5: the
/// winner it reads is BD-HERO, whose top card has no [Evolved] trait -- the
/// [Evolved] BD-SOURCE is only one of its digivolution cards -- so nothing is
/// trashed. The engine reads the winner's traits from EVERY card in its stack
/// (`Permanent::has_trait`, via `TriggerContext::battle_winner_has_trait`) and
/// trashes one. Found by the Task 28 review round.
#[test]
#[ignore = "G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS"]
fn turn_players_observer_reads_the_winners_top_card_before_de_digivolution() {
    for replay in [false, true] {
        winner_after_de_digivolution_with_turn(replay, 0);
    }
}

fn winner_after_de_digivolution_with_turn(replay: bool, turn_player: usize) {
    let observer = OBSERVER.replace(
        "      event_battle_deleter:\n        owner: you\n        kind: digimon\n        name_is: Hero",
        "      event_winner_owner: you\n      event_winner_trait_has: Evolved");
    let mut source = card("BD-SOURCE", "Surviving Hero", 7000);
    source.traits = vec!["Evolved".into()];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&observer)
        .unwrap()
        .add_card(card("BD-HERO", "Hero", 8000))
        .add_card(card("BD-VILLAIN", "Villain", 5000))
        .add_card(card("BD-PAD", "Pad", 1000))
        .add_card(source)
        .deck(0, &["BD-PAD"; 12])
        .deck(1, &["BD-PAD"; 12])
        .security(1, &["BD-PAD"; 4])
        .start();
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    let data_index = r
        .game
        .card_data
        .iter()
        .position(|d| d.card_id == "BD-SOURCE")
        .unwrap();
    let card_index = r.game.next_card_index();
    r.game.players[0].battle_area[hero.index as usize]
        .card_sources
        .insert(
            0,
            digimon_engine::card_source::CardSource::new(data_index, hero.player, card_index),
        );
    r.place_on_field(0, "BD-OBS", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.register_effect(
        "BD-VILLAIN",
        std::sync::Arc::new(ChangeWinnerOnDeletion {
            winner: hero,
            delete_winner: false,
            dna_winner: false,
            de_digivolve_winner: true,
            replay_after_de_digivolve: replay,
        }),
    );
    // The turn player's effect in the battle's pool activates first
    // (§15-4-3-5-1; see `winner_identity_after_deletion`).
    r.game.turn_player_idx = turn_player;
    r.battle_digimon(hero, villain);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BD-SOURCE"));
    assert_eq!(
        r.security_count(1),
        if turn_player == 0 || replay { 4 } else { 3 },
        "only the same surviving Digimon retains its battle win (turn player P{turn_player})"
    );
}

/// A <Piercing> [Hero] that the observer watches.
const PIERCING_HERO: &str = r#"
card: BD-PIERCE-HERO
name: Hero
kind: digimon
level: 5
color: [red]
cost: 5
dp: 8000
effects:
  - kind: grant_keyword
    keyword: Piercing
    summary: "<Piercing>"
"#;

/// §14-2-4 / §15-4-3-2 / §16-7-4: one battle triggers P0's "deletes in
/// battle" observer, P0's <Piercing> and P1's [On Deletion] -- one pool of
/// three entries across two players. §15-4-3-5-1: P0 (the turn player) orders
/// its two in one `TriggerOrder` and both activate before P1's [On Deletion];
/// the <Piercing> check is pending processing, performed before the End of
/// Attack timing after the [On Deletion]. A clone taken while P0's
/// `TriggerOrder` is parked resolves identically to the original.
#[test]
fn three_entry_two_player_battle_pool_survives_clone() {
    use digimon_engine::enums::Keyword;
    use digimon_engine::selection::SelectionKind;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(OBSERVER)
        .expect("observer compiles")
        .from_dsl_yaml(DELETION_TARGET)
        .expect("deletion target compiles")
        .from_dsl_yaml(PIERCING_HERO)
        .expect("piercing hero compiles")
        .add_card(card("BD-PAD", "Pad", 1000))
        .deck(0, &["BD-PAD"; 12])
        .deck(1, &["BD-PAD"; 12])
        .security(0, &["BD-PAD"; 4])
        .security(1, &["BD-PAD"; 4])
        .start();
    assert_eq!(r.game.turn_player(), 0, "P0 is the turn player");
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-PIERCE-HERO", Some(0));
    let dying = r.place_on_field(1, "BD-DYING", Some(0));
    r.game.players[1].battle_area[dying.index as usize].is_suspended = true;
    let memory0 = r.game.memory;

    r.attack_digimon(hero, dying, false);

    assert_eq!(r.battle_area_size(1), 0, "BD-DYING was deleted in battle");
    let piercing = {
        let order = r.pending_selection().expect("P0's ordering decision");
        assert_eq!(order.kind, SelectionKind::TriggerOrder, "{order:?}");
        assert_eq!(order.selecting_player, 0, "the turn player orders first");
        let choices = order.effect_choices.as_ref().expect("branches");
        assert_eq!(choices.len(), 2, "P0's observer and <Piercing>: {choices:?}");
        choices
            .iter()
            .find(|c| c.keyword == Some(Keyword::Piercing))
            .expect("<Piercing> is in the pool")
            .action_id
    };
    assert_eq!(r.security_count(1), 4, "nothing resolved before the order");
    assert_eq!(r.game.memory, memory0, "the [On Deletion] is still pending");

    let mut cloned = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut cloned] {
        game.execute_action(0, piercing).unwrap();
        // P0's other entry (the observer) follows; it trashes 1 security.
        while game
            .pending_selection()
            .is_some_and(|s| s.selecting_player == 0)
        {
            let view = game.pending_selection_view().unwrap();
            game.execute_action(0, view.valid_action_ids[0]).unwrap();
        }
        assert_eq!(game.security_count(1), 3, "the observer resolved");
        assert_eq!(game.game.memory, memory0, "the [On Deletion] waits for P0");
        assert_eq!(
            game.pending_selection().unwrap().selecting_player,
            1,
            "then the non-turn player's optional [On Deletion]"
        );
        choose(game, false);
        game.auto_resolve().unwrap();
        assert_ne!(game.game.memory, memory0, "the [On Deletion] resolved");
        assert_eq!(
            game.security_count(1),
            2,
            "the pending <Piercing> check, after the [On Deletion]"
        );
        assert!(game.game.pending_attack.is_none(), "the attack ended");
    }
    assert_eq!(
        r.game.verification_digest(),
        cloned.game.verification_digest(),
        "the clone resolved identically"
    );
}
