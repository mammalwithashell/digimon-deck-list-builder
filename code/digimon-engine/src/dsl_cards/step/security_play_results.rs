//! Runtime security plays credit only their correlated physical completion.
use digimon_dsl::compiled::CompiledStep;
use crate::{card_source::CardHandle, effect_context::EffectContext, game::SecurityPlayReceipt};
use crate::dsl_cards::{bindings::Bindings, binding_ref::{resolve_binding_ref, ResolvedBinding}};

pub(super) fn expand(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> Option<Vec<CompiledStep>> {
    let (player, card) = match step {
        CompiledStep::PlaySecurityCard { of, card } => {
            let player = super::resolve_player(ctx, *of);
            let Some(ResolvedBinding::Card(card)) = resolve_binding_ref(card, ctx, bindings) else { return Some(vec![]); };
            (player, card)
        }
        CompiledStep::PlayFromSecurity if ctx.game.pending_security.is_none() => {
            let security = &ctx.game.player(ctx.player).security;
            let Some(card) = security.iter().find(|c| c.handle() == ctx.source_card).or_else(|| security.last()) else { return Some(vec![]); };
            (ctx.player, card.handle())
        }
        _ => return None,
    };
    // Validate before allocating a receipt. Opaque cards are materialized by the play primitive.
    let Some(original_index) = ctx.game.player(player).security.iter().position(|c| c.handle() == card) else { return Some(vec![]); };
    let receipt = ctx.game.next_security_play_receipt;
    ctx.game.next_security_play_receipt = receipt.checked_add(1).expect("security play receipt exhausted");
    ctx.game.security_play_receipts.push(SecurityPlayReceipt { id: receipt, player, card, original_index, committed: None });
    Some(vec![CompiledStep::SecurityPlayCaptured { receipt, player, card: card.0 },
              CompiledStep::SecurityPlayFinalize { receipt, player, card: card.0 }])
}

pub(super) fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
        CompiledStep::SecurityPlayCaptured { receipt, player, card } => {
            if ctx.game.security_play_receipts.iter().any(|r| r.id == *receipt && r.player == *player && r.card.0 == *card) {
                if let Some(index) = ctx.game.player(*player).security.iter().position(|c| c.handle() == CardHandle(*card)) {
                    ctx.play_from_security_index_with_receipt(*player, index, Some(*receipt));
                }
            }
            true
        }
        CompiledStep::SecurityPlayFinalize { receipt, player, card } => {
            if let Some(index) = ctx.game.security_play_receipts.iter().position(|r| r.id == *receipt && r.player == *player && r.card.0 == *card) {
                if let Some((played, _)) = ctx.game.security_play_receipts.remove(index).committed {
                    bindings.record_played(played);
                }
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn armed() -> crate::game::Game {
        let mut game=crate::debug_runner::DebugRunner::builder().start().game;
        game.security_play_receipts.push(SecurityPlayReceipt {id:7,player:0,card:CardHandle(3),original_index:1,committed:None});
        game.next_security_play_receipt=8;
        game
    }
    #[test]
    fn security_receipts_are_cleared_on_reset_and_terminal_clones() {
        let original=armed();let mut reset=original.clone();let mut terminal=original.clone();
        reset.reset_for_replay();assert!(reset.security_play_receipts.is_empty());assert_eq!(reset.next_security_play_receipt,0);
        terminal.finish_game(None,crate::game::TerminalOutcomeReason::EngineDeclared);
        assert!(terminal.security_play_receipts.is_empty());
        assert_eq!(original.security_play_receipts.len(),1);assert_eq!(original.next_security_play_receipt,8);
    }
    #[test]
    fn aborted_security_tail_removes_only_its_exact_correlated_receipt() {
        let mut game=armed();game.security_play_receipts.push(SecurityPlayReceipt {id:7,player:1,card:CardHandle(4),original_index:2,committed:None});
        let clone=game.clone();
        super::super::cleanup_aborted_steps(&mut game,&[CompiledStep::SecurityPlayFinalize{receipt:7,player:0,card:3}]);
        assert_eq!(game.security_play_receipts.len(),1);assert_eq!(game.security_play_receipts[0].card,CardHandle(4));
        assert_eq!(clone.security_play_receipts.len(),2);
    }
}
