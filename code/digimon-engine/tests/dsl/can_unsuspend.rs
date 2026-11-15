//! Qualification of the unsuspend legality predicate used by Ulforce cards.
use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};

#[test]
fn can_unsuspend_matches_permanent_and_player_locks_for_both_polarities() {
    for player in 0..2 {
        for kind in [CardKind::Digimon, CardKind::Tamer] {
            for suspended in [false, true] {
                for lock in 0..3 {
                    let mut card = make_test_card("TARGET", "Target");
                    card.card_kind = kind;
                    if kind == CardKind::Tamer {
                        card.level = None;
                        card.dp = None;
                    }
                    let mut r = DebugRunner::builder().add_card(card).start();
                    let h = r.place_on_field(player, "TARGET", Some(0));
                    r.game.players[player as usize].battle_area[0].is_suspended = suspended;
                    if lock == 1 {
                        r.game.modifiers.add(
                            h,
                            ModifierEntry::simple(
                                ModifierType::CannotUnsuspend,
                                0,
                                Expiry::Permanent,
                                player,
                            ),
                        );
                    } else if lock == 2 {
                        r.game.modifiers.add_player_modifier(
                            player,
                            PlayerModifierEntry::simple(
                                ModifierType::CannotUnsuspend,
                                0,
                                Expiry::Permanent,
                                None,
                                player,
                            ),
                        );
                    }
                    let allowed = lock == 0 || (lock == 2 && kind == CardKind::Tamer);
                    let source = r.game.players[player as usize].battle_area[0]
                        .top_card()
                        .handle();
                    let ctx = EffectReadContext::new(&r.game, source, Some(h), player);
                    for want in [false, true] {
                        let pred = CompiledPredicate {
                            can_unsuspend: Some(want),
                            ..Default::default()
                        };
                        assert_eq!(eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::Permanent(h), None), allowed == want,
                            "player={player}, kind={kind:?}, suspended={suspended}, lock={lock}, want={want}");
                    }
                    r.game.unsuspend(h);
                    assert_eq!(
                        r.game.players[player as usize].battle_area[0].is_suspended,
                        suspended && !allowed
                    );
                }
            }
        }
    }
}
