//! A token's effect is a Digimon effect (§15-2-1-1 "An effect activated by a
//! Digimon card, Digimon, or Security Digimon is considered a Digimon
//! effect"; §4-21-1 a token is played "as if [it has] card information", so a
//! Digimon token is a Digimon). DCGO agrees: `AutoProcessing.cs` marks a token
//! permanent's `ActivateClass`es `IsDigimonEffect`.
//! G-ENGINE-TOKEN-EFFECT-SOURCE-KIND-IS-RULE.
//!
//! Attribution: a Digimon deleted by a token's effect is "deleted by your
//! opponent's Digimon's effect" — the deleting effect is the opponent's
//! (`was_deleted_by_opponent`) and it is a Digimon effect (its context's
//! `source_kind` / `source_is_digimon`).
//!
//! The victim's `[On Deletion]` resolves after the token's effect has
//! finished, so the live `current_deletion_cause` slot is already restored;
//! `was_deleted_by_opponent` / `was_deleted_by_effect` read the deleted
//! object's snapshot first, like `deletion_cause` (rule 25).
//!
//! The deletion here is a deletion BY the token's effect ("Delete 1 of your
//! opponent's Digimon"). A -DP effect that takes a Digimon to 0 DP is not:
//! that Digimon is deleted by the rule check (§17-1-3-1-1), "by the game
//! rules, not a card effect" (official Q&A Q1348 / Q2828), whatever kind of
//! effect lowered its DP.

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, Expiry};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

fn card(card_id: &str, kind: CardKind, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = kind;
    card.colors = vec![CardColor::Yellow];
    card.level = if kind == CardKind::Token { None } else { Some(4) };
    card.dp = Some(dp);
    card.play_cost = if kind == CardKind::Token { 0 } else { 5 };
    card
}

/// The token's "[On Deletion] Delete 1 of your opponent's Digimon." The test
/// pins the target (`ATTR-VICTIM`) instead of surfacing a choice: the subject
/// is the deleting effect's attribution, not the pick. Records the effect's
/// source kind and whether it reads as a Digimon effect.
struct TokenDeletesOpponentDigimon(Arc<Mutex<Option<(EffectSourceKind, bool)>>>);

impl CardEffect for TokenDeletesOpponentDigimon {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_deletion(card)
            .name("[On Deletion] Delete 1 of your opponent's Digimon")
            .process(move |ctx| {
                *witness.lock().unwrap() = Some((ctx.source_kind(), ctx.source_is_digimon()));
                let opponent = ctx.game.next_clockwise(ctx.player);
                let victim = field_index(ctx.game, opponent, "ATTR-VICTIM");
                ctx.delete_permanent(PermanentHandle {
                    player: opponent,
                    index: victim,
                });
            })
            .build()]
    }
}

fn field_index(game: &digimon_engine::game::Game, player: u8, card_id: &str) -> u8 {
    game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&game.card_data) == card_id)
        .unwrap_or_else(|| panic!("fixture: {card_id} is on P{player}'s field")) as u8
}

/// The victim's "[On Deletion]" observer: records how it was deleted.
struct ReportsDeletion(Arc<Mutex<Option<(bool, Option<ReplacementCause>)>>>);

impl CardEffect for ReportsDeletion {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_deletion(card)
            .name("[On Deletion] deletion-cause witness")
            .process(move |ctx| {
                *witness.lock().unwrap() =
                    Some((ctx.was_deleted_by_opponent(), ctx.deletion_cause()));
            })
            .build()]
    }
}

/// P1's "[On Play] Delete 1 of your opponent's Digimon" — deletes P0's token
/// (P0's only Digimon), which starts the chain.
struct DeletesTheToken;

impl CardEffect for DeletesTheToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("[On Play] Delete 1 of your opponent's Digimon")
            .process(|ctx| {
                let opponent = ctx.game.next_clockwise(ctx.player);
                let token = field_index(ctx.game, opponent, "ATTR-TOKEN");
                ctx.delete_permanent(PermanentHandle {
                    player: opponent,
                    index: token,
                });
            })
            .build()]
    }
}

/// P0's Digimon token is deleted by P1's effect; its "[On Deletion] Delete 1
/// of your opponent's Digimon" deletes P1's `ATTR-VICTIM`. That effect is a
/// Digimon effect (§15-2-1-1) controlled by P1's opponent, so the victim was
/// deleted by its opponent's Digimon's effect.
#[test]
fn a_digimon_deleted_by_a_tokens_effect_is_deleted_by_an_opponents_digimon_effect() {
    let token_witness = Arc::new(Mutex::new(None));
    let victim_witness = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(card("ATTR-TOKEN", CardKind::Token, 3000))
        .add_card(card("ATTR-VICTIM", CardKind::Digimon, 6000))
        .add_card(card("ATTR-KILLER", CardKind::Digimon, 5000))
        .start();
    r.register_effect(
        "ATTR-TOKEN",
        Arc::new(TokenDeletesOpponentDigimon(token_witness.clone())),
    );
    r.register_effect("ATTR-VICTIM", Arc::new(ReportsDeletion(victim_witness.clone())));
    r.register_effect("ATTR-KILLER", Arc::new(DeletesTheToken));

    r.place_on_field(0, "ATTR-TOKEN", Some(0));
    r.place_on_field(1, "ATTR-VICTIM", Some(0));
    let killer = r.place_on_field(1, "ATTR-KILLER", Some(0));
    r.fire_on_play(1, killer.index as usize);
    r.game.drain_effect_queue();

    let token_ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert!(token_ids.is_empty(), "the token was deleted: {token_ids:?}");
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "ATTR-VICTIM"),
        "the token's [On Deletion] deleted the victim"
    );
    assert_eq!(
        *token_witness.lock().unwrap(),
        Some((EffectSourceKind::Digimon, true)),
        "§15-2-1-1: the token's [On Deletion] is a Digimon effect"
    );
    assert_eq!(
        *victim_witness.lock().unwrap(),
        Some((true, Some(ReplacementCause::OpponentEffect))),
        "the victim was deleted by its opponent's (the token's) effect"
    );
}

/// The token's "[On Deletion] 1 of your Digimon gets -1000 DP for the turn"
/// (aimed at `OWN-IMM`, the controller's only other Digimon).
struct TokenShrinksOwnDigimon;

impl CardEffect for TokenShrinksOwnDigimon {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("[On Deletion] 1 of your Digimon gets -1000 DP for the turn")
            .process(|ctx| {
                let own = field_index(ctx.game, ctx.player, "OWN-IMM");
                ctx.add_dp_modifier(
                    PermanentHandle {
                        player: ctx.player,
                        index: own,
                    },
                    -1000,
                    Expiry::EndOfTurn,
                );
            })
            .build()]
    }
}

/// An immunity to the OPPONENT's Digimon effects filters on the effect's
/// controller as well as its kind (§15-15-5-1): P0's Digimon token is P0's
/// Digimon, so its effect on P0's own `OWN-IMM` (immune to P1's Digimon
/// effects) applies — the kind change does not widen an opponent-only shield.
#[test]
fn a_tokens_effect_on_its_controllers_digimon_ignores_an_opponent_only_immunity() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATTR-TOKEN", CardKind::Token, 3000))
        .add_card(card("OWN-IMM", CardKind::Digimon, 6000))
        .add_card(card("ATTR-KILLER", CardKind::Digimon, 5000))
        .start();
    r.register_effect("ATTR-TOKEN", Arc::new(TokenShrinksOwnDigimon));
    r.register_effect("ATTR-KILLER", Arc::new(DeletesTheToken));

    r.place_on_field(0, "ATTR-TOKEN", Some(0));
    let own = r.place_on_field(0, "OWN-IMM", Some(0));
    r.game.modifiers.add(
        own,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );
    assert!(
        r.game.permanent_is_unaffected_by_effect(own, 1, EffectSourceKind::Digimon),
        "precondition: P1's Digimon effects don't affect it"
    );
    assert!(
        !r.game.permanent_is_unaffected_by_effect(own, 0, EffectSourceKind::Digimon),
        "precondition: its own controller's Digimon effects do"
    );
    let killer = r.place_on_field(1, "ATTR-KILLER", Some(0));
    r.fire_on_play(1, killer.index as usize);
    r.game.drain_effect_queue();

    let own = PermanentHandle {
        player: 0,
        index: field_index(&r.game, 0, "OWN-IMM"),
    };
    assert_eq!(
        r.effective_dp(own),
        Some(5000),
        "the token's own-side effect is its controller's effect: it applies"
    );
}
