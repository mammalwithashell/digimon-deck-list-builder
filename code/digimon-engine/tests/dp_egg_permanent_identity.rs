//! Separate capability: official EX2-007 Q3274/Q3276/Q3285.
//! Mother is a Digi-Egg CARD but a Digimon PERMANENT, without a printed level.
use digimon_engine::debug_runner::{DebugRunner,make_test_egg};
use digimon_engine::enums::{CardKind,CardColor};
fn runner()->DebugRunner {
 let mut c=make_test_egg("EX2-007","Mother D-Reaper");c.level=None;c.dp=Some(15000);c.colors=vec![CardColor::White];
 let mut r=DebugRunner::builder().add_card(c).build();r.place_on_field(0,"EX2-007",Some(0));r
}
#[test] fn fielded_dp_egg_is_a_digimon_for_rules() {
 let r=runner();let p=&r.game.players[0].battle_area[0];
 assert!(p.is_digimon_for_rules(&r.game.card_data,&r.game.modifiers,digimon_engine::permanent::PermanentHandle{player:0,index:0}));
}
#[test] fn rules_identity_does_not_rewrite_printed_egg_kind() {
 let r=runner();let c=r.game.players[0].battle_area[0].top_card();
 assert_eq!(c.card_kind(&r.game.card_data),CardKind::DigiEgg);

 assert_eq!(c.level(&r.game.card_data),None);assert_eq!(c.dp(&r.game.card_data),Some(15000));
}

