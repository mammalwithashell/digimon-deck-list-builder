//! A Digimon deleted by the §17-1-3-1-1 rule check (0 DP or less) is deleted
//! by the game rules, not by an effect -- whatever effect lowered its DP
//! (official Q&A Q1348 / Q2828; DCGO `!IsByEffect`; §17-1-2 rule processing).
//! Its `[On Deletion]` therefore reads no effect cause: `deletion_cause` is
//! `None`, `was_deleted_by_effect` / `was_deleted_by_opponent` are false.
//!
//! The deleted-object snapshot carries `EventCause::Rule` for such a
//! deletion; it must be terminal. Before the fix, `observed_deletion_cause`
//! fell through from the snapshot's `Rule` to the live
//! `current_deletion_cause` slot, which holds the cause `infer_deletion_cause`
//! guessed for the batch (an own/opponent EFFECT between effects), so a "if
//! this Digimon was deleted by your opponent's effect" rider fired for a
//! 0 DP rule-check deletion. G-ENGINE-RULE-CHECK-DELETION-READS-LIVE-EFFECT-CAUSE.

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Expiry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

fn card(card_id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

fn field_index(game: &digimon_engine::game::Game, player: u8, card_id: &str) -> u8 {
    game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&game.card_data) == card_id)
        .unwrap_or_else(|| panic!("fixture: {card_id} is on P{player}'s field")) as u8
}

/// What the victim's `[On Deletion]` observed.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Observed {
    by_effect: bool,
    by_opponent: bool,
    cause: Option<ReplacementCause>,
}

struct ReportsDeletion(Arc<Mutex<Option<Observed>>>);

impl CardEffect for ReportsDeletion {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_deletion(card)
            .name("[On Deletion] deletion-cause witness")
            .process(move |ctx| {
                *witness.lock().unwrap() = Some(Observed {
                    by_effect: ctx.was_deleted_by_effect(),
                    by_opponent: ctx.was_deleted_by_opponent(),
                    cause: ctx.deletion_cause(),
                });
            })
            .build()]
    }
}

/// P1's "[On Play] 1 of your opponent's Digimon gets -3000 DP for the turn"
/// (aimed at `RC-VICTIM`, P0's only Digimon).
struct ShrinksTheVictim;

impl CardEffect for ShrinksTheVictim {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("[On Play] 1 of your opponent's Digimon gets -3000 DP for the turn")
            .process(|ctx| {
                let opponent = ctx.game.next_clockwise(ctx.player);
                let victim = field_index(ctx.game, opponent, "RC-VICTIM");
                ctx.add_dp_modifier(
                    PermanentHandle {
                        player: opponent,
                        index: victim,
                    },
                    -3000,
                    Expiry::EndOfTurn,
                );
            })
            .build()]
    }
}

/// P1's "[On Play] Delete 1 of your opponent's Digimon" (aimed at `RC-VICTIM`).
struct DeletesTheVictim;

impl CardEffect for DeletesTheVictim {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("[On Play] Delete 1 of your opponent's Digimon")
            .process(|ctx| {
                let opponent = ctx.game.next_clockwise(ctx.player);
                let victim = field_index(ctx.game, opponent, "RC-VICTIM");
                ctx.delete_permanent(PermanentHandle {
                    player: opponent,
                    index: victim,
                });
            })
            .build()]
    }
}

fn run(killer_effect: Arc<dyn CardEffect>) -> Observed {
    let witness = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(card("RC-VICTIM", 3000))
        .add_card(card("RC-KILLER", 5000))
        .start();
    r.register_effect("RC-VICTIM", Arc::new(ReportsDeletion(witness.clone())));
    r.register_effect("RC-KILLER", killer_effect);
    r.place_on_field(0, "RC-VICTIM", Some(0));
    let killer = r.place_on_field(1, "RC-KILLER", Some(0));
    r.fire_on_play(1, killer.index as usize);
    r.game.drain_effect_queue();
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "RC-VICTIM"),
        "the victim was deleted"
    );
    let observed = witness.lock().unwrap().clone();
    observed.expect("the victim's [On Deletion] activated")
}

/// -3000 DP takes the 3000-DP victim to 0 DP: the rule check deletes it.
/// That deletion is by the rules, not by P1's effect (Q1348 / Q2828).
#[test]
fn zero_dp_rule_check_deletion_is_not_a_deletion_by_an_effect() {
    assert_eq!(
        run(Arc::new(ShrinksTheVictim)),
        Observed {
            by_effect: false,
            by_opponent: false,
            cause: None,
        },
        "deleted by the 0 DP rule check: no effect cause"
    );
}

/// Control: "Delete 1 of your opponent's Digimon" IS a deletion by the
/// opponent's effect.
#[test]
fn deletion_by_an_opponents_effect_still_reads_as_one() {
    assert_eq!(
        run(Arc::new(DeletesTheVictim)),
        Observed {
            by_effect: true,
            by_opponent: true,
            cause: Some(ReplacementCause::OpponentEffect),
        }
    );
}
