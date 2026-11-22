//! Petrification Token — Medusamon archetype (Purple).
//!
//! Printed on Medusamon (BT21-029), Medusamon (EX11-012) and Medusamon
//! (BT24-017), official Bandai DB:
//!   (Digimon/White/3000 DP/
//!    [Your Turn] This Digimon can't suspend.
//!    [On Deletion] Trash your top security card.)
//!
//! Both clauses are implemented:
//!
//! * `[On Deletion] Trash your top security card.` — "your" is the token's
//!   controller.
//!
//! * `[Your Turn] This Digimon can't suspend.` — a continuous effect that
//!   gives the token's own permanent `ModifierType::CannotSuspend` while it is
//!   its CONTROLLER's turn (the opponent of the Medusamon that made them play
//!   it: "they play 2 [Petrification] Tokens", so "[Your Turn]" on the token is
//!   the token controller's turn). Mirrors DCGO `BT21_029_token.cs` region
//!   "Cant Suspend": `CantSuspendStaticEffect(permanent == this token)` gated
//!   on `IsExistOnBattleArea(card) && IsOwnerTurn(card)`.
//!
//!   It is a `declarative` effect, so the condition is re-read on every
//!   declarative tick and the modifier is materialized for that tick only: it
//!   holds on EVERY one of the controller's turns and lifts on every opponent
//!   turn. (An install-once `until_condition` entry would be evicted when the
//!   first turn ends and never come back.)
//!
//!   The consumers already exist: CR 4.3 11-2-5 "An attack declaration can't
//!   be made using a Digimon that can't suspend" (`Game::can_attack*`, the
//!   attack mask), the blocker / Alliance / Training / suspend-cost gates, and
//!   the universal suspend chokepoint (`Game::suspend_state_only`).
//!
//! Both bodies are capture-free and read only the effect's own source and the
//! turn player, so the factory stays read-fence constructible
//! (`tokens::register`).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::{Expiry, ModifierType};

pub struct PetrificationToken;

impl CardEffect for PetrificationToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_deletion(card)
                .name("[On Deletion] Trash top of owner's security")
                .process(|ctx| {
                    // The token's owner = the player who controls the token
                    // permanent = `ctx.player` (EffectContext is always
                    // scoped to the source's controller).
                    let owner = ctx.player;
                    ctx.trash_top_security(owner);
                })
                .build(),
            Effect::declarative(card)
                .name("[Your Turn] This Digimon can't suspend")
                // `[Your Turn]`: the token controller's turn. `ctx.player` is
                // the controller of the effect's source (the token).
                .condition(|ctx| ctx.game.turn_player() == ctx.player)
                .materializes_declarative_state()
                .process(|ctx| {
                    let Some(token) = ctx.source_permanent else {
                        return;
                    };
                    ctx.add_declarative_modifier(
                        token,
                        ModifierType::CannotSuspend,
                        0,
                        Expiry::Permanent,
                    );
                })
                .build(),
        ]
    }
}
