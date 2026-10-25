#[cfg(test)]
mod tests {
    use crate::{debug_runner::{DebugRunner,make_test_card},game::Game,dsl_cards::bindings::Bindings,recorder::VerificationDigestVersion as V,card_source::CardHandle};
    fn fixture()->(Game,crate::permanent::PermanentHandle,u64) {
        let mut r=DebugRunner::builder().add_card(make_test_card("HOST","Host")).start();
        let host=r.place_on_field(0,"HOST",None);let identity=r.game.capture_digivolve_host_identity(host);
        r.game.dsl_resolved_tail_bindings=Some(Bindings::new());(r.game,host,identity)
    }
    #[test]
    fn private_link_pins_are_v4_only_without_authored_slots_or_debug_changes() {
        for key in ["\0link_source_carrier:0","\0link_material_carrier:0"] {
            let (base,_,identity)=fixture();let mut pinned=base.clone();
            let b=pinned.dsl_resolved_tail_bindings.as_mut().unwrap();b.pin_private_link_identity(key,0,identity);
            assert!(b.get_ref(key).is_none());assert_eq!(format!("{b:?}"),format!("{:?}",base.dsl_resolved_tail_bindings.as_ref().unwrap()));
            for version in [V::LegacyV2,V::V3] {assert_eq!(base.verification_digest_with_version(version),pinned.verification_digest_with_version(version));}
            assert_ne!(base.verification_digest(),pinned.verification_digest());
            let mut wrong=pinned.clone();wrong.dsl_resolved_tail_bindings.as_mut().unwrap().pin_private_link_identity(key,0,identity+99);
            assert_ne!(pinned.verification_digest(),wrong.verification_digest());
            assert_eq!(pinned.verification_digest(),pinned.clone().verification_digest());
        }
    }
    #[test]
    fn existing_reserved_source_identity_pin_remains_in_v3() {
        let (base,host,identity)=fixture();let mut pinned=base.clone();
        let key=crate::dsl_cards::binding_ref::source_carrier_pin_key(CardHandle(0));
        pinned.dsl_resolved_tail_bindings.as_mut().unwrap().insert_selected_permanent_with_identity(&key,host,identity);
        assert_ne!(base.verification_digest_with_version(V::V3),pinned.verification_digest_with_version(V::V3));
    }
    fn dna_scope(game:&mut Game,bindings:Bindings,steps:Vec<digimon_dsl::compiled::CompiledStep>) {
        use crate::{game_actions::effect_dna::{EffectDnaHandDeclaration,EffectDnaWouldScope},enums::{CostDelta,EffectSourceKind},resume::OuterContinuation};
        game.effect_dna_would_scopes.push(EffectDnaWouldScope{effective_cost:0,declaration:EffectDnaHandDeclaration{
            player:0,field_identity:1,field_top:CardHandle(0),partner:CardHandle(1),result:CardHandle(2),delta:CostDelta::Free,ignore_requirements:true,routes:vec![],chosen:None,
            source_card:CardHandle(0),source_permanent:None,source_kind:EffectSourceKind::Digimon,source_controller:0,trigger_context:None,result_clause:None,
            outer_conts:vec![OuterContinuation{tail:std::sync::Arc::new(steps),bindings,runtime:Default::default(),trigger_context:None,source_card:CardHandle(0),source_permanent:None,player:0}]}});
    }
    #[test]
    fn private_link_pins_in_dna_declaration_keep_v3_and_affect_v4() {
        let (mut base,_,identity)=fixture();dna_scope(&mut base,Bindings::new(),vec![]);
        let mut pinned=base.clone();pinned.effect_dna_would_scopes[0].declaration.outer_conts[0].bindings.pin_private_link_identity("\0link_source_carrier:0",0,identity);
        assert_eq!(base.verification_digest_with_version(V::V3),pinned.verification_digest_with_version(V::V3));
        assert_ne!(base.verification_digest(),pinned.verification_digest());
    }
    #[test]
    fn security_runtime_in_dna_tail_is_legacy_neutral_and_v4_alpha_equivalent() {
        use digimon_dsl::compiled::CompiledStep as S;
        let (mut base,_,_)=fixture();dna_scope(&mut base,Bindings::new(),vec![S::CompleteCostPayment{receipt:77}]);
        base.security_play_receipts.push(crate::game::SecurityPlayReceipt{id:8,player:0,card:CardHandle(1),original_index:1,committed:None});
        let mut added=base.clone();added.effect_dna_would_scopes[0].declaration.outer_conts[0].tail=std::sync::Arc::new(vec![S::SecurityPlayCaptured{receipt:8,player:0,card:1},S::SecurityPlayFinalize{receipt:8,player:0,card:1},S::CompleteCostPayment{receipt:77}]);
        for version in [V::LegacyV2,V::V3] {assert_eq!(base.verification_digest_with_version(version),added.verification_digest_with_version(version));}
        let mut renamed=added.clone();renamed.security_play_receipts[0].id=100;
        for step in std::sync::Arc::make_mut(&mut renamed.effect_dna_would_scopes[0].declaration.outer_conts[0].tail) {
            match step {S::SecurityPlayCaptured{receipt,..}|S::SecurityPlayFinalize{receipt,..}=>*receipt=100,_=>{}}
        }
        assert_eq!(added.verification_digest(),renamed.verification_digest());
    }
    #[test]
    fn private_link_clear_is_current_state_without_archival_debug_or_digest_changes() {
        let (base,_,identity)=fixture();let mut cleared=base.clone();
        let key="\0link_source_carrier:0";
        let b=cleared.dsl_resolved_tail_bindings.as_mut().unwrap();
        b.pin_private_link_identity(key,0,identity);b.clear_for_merge(key);
        assert_eq!(format!("{b:?}"),format!("{:?}",base.dsl_resolved_tail_bindings.as_ref().unwrap()));
        for version in [V::LegacyV2,V::V3] {assert_eq!(base.verification_digest_with_version(version),cleared.verification_digest_with_version(version));}
        assert_ne!(base.verification_digest(),cleared.verification_digest());
    }

}
