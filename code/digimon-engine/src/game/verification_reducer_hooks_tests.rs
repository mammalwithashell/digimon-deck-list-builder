use crate::{action::{build_action_mask,encode_digivolve,space::PASS},debug_runner::DebugRunner,enums::GamePhase,resume::AfterSelectionHook};
fn parked()->crate::game::Game {
 // Hybrid's treated-as route reads the result's printed evolution circle.
 // Supply exactly the metadata the former permissive fixture loader inferred;
 // the production YAML's kind-restricted alternative is not a printed row.
 let spec: digimon_dsl::CardSpec = serde_yml::from_str(include_str!("../../cards/bt4/BT4-011.yaml")).unwrap();
 let compiled = digimon_dsl::compile::compile(&spec).unwrap();
 let mut agunimon = crate::debug_runner::card_data_for_test_from_compiled(&compiled);
 agunimon.evo_costs = vec![crate::card_data::EvoCost { card_color: 0, level: 3, memory_cost: 2 }];
 let mut r=DebugRunner::builder().add_card(agunimon).from_dsl_yaml(include_str!("../../cards/bt4/BT4-011.yaml")).unwrap()
  .from_dsl_yaml(include_str!("../../cards/st1/ST1-12.yaml")).unwrap()
  .from_dsl_yaml(include_str!("../../cards/st1/ST1-03.yaml")).unwrap()
  .from_dsl_yaml(r#"
card: HASH-EVOLVE-PAYER
name: Evolution Payment Source
kind: tamer
color: [white]
cost: 2
effects:
 - kind: cost_reduction
   reduction_timing: before_pay_cost
   optional: true
   when_any_ally_digivolves_into: {card_number_is: BT4-011}
   amount: 1
   pay_cost:
    - select_own_permanent:
        filter: {card_number_is: HASH-EVOLVE-PAYER}
        bind_as: payer
        prompt: Choose the payment source
    - suspend: {target: payer}
"#).unwrap().hand(0,&["BT4-011"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();
 r.skip_mulligan();r.place_on_field(0,"ST1-12",Some(0));r.place_on_field(0,"HASH-EVOLVE-PAYER",Some(0));r.game.current_phase=GamePhase::Main;r.game.tick_declarative_effects();
 let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);
 let accept=*r.game.pending_selection.as_ref().unwrap().valid_action_ids.iter().find(|&&a|a!=PASS).unwrap();assert_eq!(build_action_mask(&r.game,0)[accept as usize],1.0);r.game.resolve_selection(0,accept).unwrap();
 assert!(r.game.pending_selection.is_some());assert!(r.game.after_selection_resume_hooks.0.iter().any(|h|matches!(h,AfterSelectionHook::DigivolveReducerChain{..})));r.game
}
#[test]fn reducer_hook_paid_credit_changes_digest(){let g=parked();let mut other=g.clone();for h in &mut other.after_selection_resume_hooks.0 {if let AfterSelectionHook::DigivolveReducerChain{paid,..}=h{assert!(*paid);*paid=false;break;}}assert_ne!(g.verification_digest(),other.verification_digest());}
#[test]fn reducer_hook_amount_changes_digest(){let g=parked();let mut other=g.clone();for h in &mut other.after_selection_resume_hooks.0 {if let AfterSelectionHook::DigivolveReducerChain{amount,..}=h{*amount+=1;break;}}assert_ne!(g.verification_digest(),other.verification_digest());}
#[test]fn reducer_hook_order_changes_digest(){let mut g=parked();let mut next=g.after_selection_resume_hooks.0.iter().find(|h|matches!(h,AfterSelectionHook::DigivolveReducerChain{..})).unwrap().clone();if let AfterSelectionHook::DigivolveReducerChain{paid,..}=&mut next{*paid=false;}g.after_selection_resume_hooks.0.push(next);let mut other=g.clone();other.after_selection_resume_hooks.0.reverse();assert_ne!(g.verification_digest(),other.verification_digest());}

#[test]
fn legacy_reducer_digest_remains_explicit_and_current_clones_agree() {
    use crate::recorder::VerificationDigestVersion;
    let game = parked();
    let clone = game.clone();
    assert_eq!(game.verification_digest(), clone.verification_digest());
    assert_eq!(game.pending_selection.as_ref().unwrap().zone_owner, Some(0));
    // This archived constant predates explicit field-prompt ownership. Recreate
    // that one historical metadata field on a clone only; the live fixture and
    // current digests keep the real owner. Never replace the pinned constant.
    let mut archival = game.clone();
    archival.pending_selection.as_mut().unwrap().zone_owner = None;
    assert_eq!(archival.verification_digest_with_version(VerificationDigestVersion::LegacyV2), 12021496225411690432);
    assert_ne!(game.verification_digest_with_version(VerificationDigestVersion::LegacyV2), archival.verification_digest_with_version(VerificationDigestVersion::LegacyV2));
    assert_ne!(game.verification_digest(), game.verification_digest_with_version(VerificationDigestVersion::LegacyV2));
}

#[test]
fn reducer_hook_owner_and_cross_kind_order_change_digest() {
    let game = parked();
    let mut wrong_owner = game.clone();
    for hook in &mut wrong_owner.after_selection_resume_hooks.0 {
        if let AfterSelectionHook::DigivolveReducerChain { state, .. } = hook { state.acting_player = 1; break; }
    }
    assert_ne!(game.verification_digest(), wrong_owner.verification_digest());
    let mut first = game.clone();
    first.after_selection_resume_hooks.0.push(AfterSelectionHook::InteractiveDigivolveReducer {
        amount: 1, acting_player: 0, hand_index: 0, field_index: 0, source: crate::enums::PlaySource::ByEffect,
    });
    let mut reversed = first.clone();
    reversed.after_selection_resume_hooks.0.reverse();
    assert_ne!(first.verification_digest(), reversed.verification_digest());
    assert_eq!(crate::recorder::VerificationDigestVersion::from_engine_schema(crate::recorder::VERIFICATION_ENGINE_SCHEMA_VERSION), Some(crate::recorder::VerificationDigestVersion::CURRENT));
}
