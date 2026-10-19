//! Rules4-2-8/4-5-8: sources of a leaving permanent are disposed by rule.
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::effect::{CardEffect,Effect};
use digimon_engine::card_source::CardHandle;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::enums::StackPosition;
struct Observer;
impl CardEffect for Observer {
 fn effects(&self,c:CardHandle)->Vec<Effect>{vec![Effect::on_digivolution_card_trashed(c).inherited().process(|ctx|{
  assert_eq!(ctx.event_cause(),Some(digimon_engine::trigger_context::EventCause::Rule));
  ctx.gain_memory(1);
 }).build()]}
}
fn runner()->DebugRunner {
 let mut r=DebugRunner::builder().add_card(make_test_card("TOP","Top")).add_card(make_test_card("SOURCE","Source"))
 .deck(0,&["TOP";10]).deck(1,&["TOP";10]).memory(3).build();
 r.register_effect("SOURCE",std::sync::Arc::new(Observer));r.place_stack(0,&["SOURCE","TOP"]);r.game.memory=3;r
}
fn h()->PermanentHandle{PermanentHandle{player:0,index:0}}
#[test]fn hand_return_disposes_old_source_by_rule(){let mut r=runner();r.game.return_to_hand(h());assert_eq!(r.memory(),4);assert_eq!(r.game.players[0].trash.len(),1);}
#[test]fn deck_bottom_return_disposes_old_source_by_rule(){let mut r=runner();r.game.return_to_deck(h(),StackPosition::Bottom);assert_eq!(r.memory(),4);assert_eq!(r.game.players[0].trash.len(),1);}
#[test]fn deck_top_return_disposes_old_source_by_rule(){let mut r=runner();r.game.return_to_deck(h(),StackPosition::Top);assert_eq!(r.memory(),4);assert_eq!(r.game.players[0].trash.len(),1);}
