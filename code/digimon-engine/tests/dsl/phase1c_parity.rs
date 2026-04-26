//! Parity: BT5-093 (Tai Kamiya & Matt Ishida — Tamer aura) DSL-compiled
//! lowering vs a hand-written `CardEffect` that produces the same
//! observable shape (a filtered aura granting SecurityAttackPlus(1) to
//! "Omnimon" permanents during your turn).
//!
//! Phase 1c does not lower triggered clauses, so the DSL version emits
//! exactly one declarative effect (the aura). The hand-written reference
//! is deliberately minimal — it only mirrors the aura clause, not the
//! `start_of_your_turn` or `on_security` clauses that BT5-093 also carries
//! (those are Phase 2).

use std::sync::Arc;

use digimon_dsl::compiled::CompiledCard;
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;

/// Hand-written reference for BT5-093's filtered aura.
struct HandBt5093;
impl CardEffect for HandBt5093 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("BT5-093 aura (hand)")
            .condition(|rctx| rctx.game.turn_player() == rctx.player)
            .process(|ctx| {
                let mut hits: Vec<PermanentHandle> = Vec::new();
                {
                    let rctx = ctx.as_read();
                    let me = rctx.player;
                    let n = rctx.game.player(me).battle_area.len();
                    for i in 0..n {
                        let perm = &rctx.game.player(me).battle_area[i];
                        let top = perm.top_card();
                        let data = &rctx.game.card_data[top.data_index];
                        if data.card_name.contains("Omnimon") {
                            hits.push(PermanentHandle { player: me, index: i as u8 });
                        }
                    }
                }
                for h in hits {
                    ctx.grant_keyword(h, Keyword::SecurityAttackPlus(1), Expiry::Permanent);
                }
            })
            .build()]
    }
}

fn dsl_bt5093_compiled() -> Arc<CompiledCard> {
    let pack = digimon_engine::dsl_registry::from_embedded()
        .expect("embedded pack loads");
    Arc::new(pack.lookup("BT5-093").expect("BT5-093 in pack").clone())
}

#[test]
fn dsl_and_hand_bt5093_emit_same_declarative_effect_count() {
    let dsl_effects = DslCardEffect::new(dsl_bt5093_compiled()).effects(CardHandle(0));
    let hand_effects = HandBt5093.effects(CardHandle(0));

    let dsl_decl: Vec<_> = dsl_effects.iter().filter(|e| e.declarative).collect();
    let hand_decl: Vec<_> = hand_effects.iter().filter(|e| e.declarative).collect();
    assert_eq!(
        dsl_decl.len(),
        hand_decl.len(),
        "declarative effect count parity"
    );
    assert_eq!(hand_decl.len(), 1, "hand reference emits exactly 1");
}
