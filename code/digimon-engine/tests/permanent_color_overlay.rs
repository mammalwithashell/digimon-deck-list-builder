use digimon_engine::{debug_runner::{DebugRunner,make_test_card},enums::{CardColor,ModifierType,Expiry},
    modifiers::{ModifierEntry,ModifierPayload},effect_context::EffectReadContext,
    dsl_cards::predicate::{eval_predicate,PredicateSubject}};
use digimon_dsl::compiled::{CompiledColor,CompiledPredicate};

fn check(only:bool){
    let mut card=make_test_card("COLOR-PROBE","Printed green");card.colors=vec![CardColor::Green];
    let mut r=DebugRunner::builder().add_card(card).start();
    let h=r.place_on_field(0,"COLOR-PROBE",None);
    let card=r.game.players[0].battle_area[0].top_card().handle();
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::ChangeBaseCardColor,0,Expiry::Permanent,0)
        .with_payload(ModifierPayload::Colors{value:vec![CardColor::Blue],base:true}));
    assert_eq!(r.game.players[0].battle_area[0].colors_for_rules(&r.game.card_data,&r.game.modifiers,h),vec![CardColor::Blue]);
    let saved=r.game.clone();
    for _ in 0..2{
        r.game=saved.clone();
        let ctx=EffectReadContext::new(&r.game,card,Some(h),0);
        for (color,field_expected,card_expected) in [(CompiledColor::Blue,true,false),(CompiledColor::Green,false,true)]{
            let predicate=if only {CompiledPredicate{color_only:Some(vec![color]),..Default::default()}}
                else{CompiledPredicate{color_is:Some(color),..Default::default()}};
            assert_eq!(eval_predicate(&predicate,&ctx,PredicateSubject::Card(card)),card_expected,
                "card search must continue to inspect printed color");
            assert_eq!(eval_predicate(&predicate,&ctx,PredicateSubject::Permanent(h)),field_expected,
                "a field predicate must not fall back to the replaced printed color");
        }
    }
}
#[test]fn color_is_obeys_effective_field_color_and_preserves_printed_search(){check(false)}
#[test]fn color_only_obeys_effective_field_color_and_preserves_printed_search(){check(true)}
