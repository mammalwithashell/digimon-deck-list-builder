//! Compatibility contract for PendingAttack's existing verification bytes.
use digimon_engine::{selection::{PendingAttack,AttackTarget,AttackState},
    permanent::PermanentHandle,enums::GamePhase,recorder::VerificationDigestVersion,
    debug_runner::DebugRunner};
use sha2::{Digest,Sha256};
const OLD_DEBUG: &str = "PendingAttack { attacker: PermanentHandle { player: 0, index: 2 }, original_target: Player(1), effective_target: Digimon(PermanentHandle { player: 1, index: 3 }), is_blocked: true, blocker: Some(PermanentHandle { player: 1, index: 3 }), is_vortex: false, is_overclock: true, declaration_committed: true, cancelled: false, battle_occurred: true, battle_defender_deleted: false, effect_battle_piercing_pending: true, security_check_started: false, return_phase: Main, state: Battle, counter_depth: 1 }";
const OLD_PENDING_FIELD_SHA256: &str = "9b5708777eca8cef0ebe45c0189bc5586cca6f8903b81a24ccba160364e4856b";
fn sample()->PendingAttack {
    PendingAttack {
        attacker:PermanentHandle{player:0,index:2}, original_target:AttackTarget::Player(1),
        effective_target:AttackTarget::Digimon(PermanentHandle{player:1,index:3}),
        effective_target_removed:false, is_blocked:true,
        blocker:Some(PermanentHandle{player:1,index:3}),is_vortex:false,is_overclock:true,
        declaration_committed:true,cancelled:false,battle_occurred:true,battle_defender_deleted:false,
        effect_battle_piercing_pending:true,security_check_started:false,
        return_phase:GamePhase::Main,state:AttackState::Battle,counter_depth:1,
    }
}
// Exact token framing from game/verification.rs::debug_field, including Option.
fn pending_field_hash(pa:&Option<PendingAttack>)->String {
    let mut h=Sha256::new();h.update(b"pending_attack\0");
    h.update(format!("{pa:?}").as_bytes());h.update([0]);format!("{:x}",h.finalize())
}
#[test]
fn false_dna_marker_preserves_old_pending_attack_string_and_hash(){
    let pa=sample();assert_eq!(format!("{pa:?}"),OLD_DEBUG);
    assert_eq!(pending_field_hash(&Some(pa)),OLD_PENDING_FIELD_SHA256);
}
#[test]
fn true_dna_marker_is_explicit_distinct_and_clone_stable(){
    let mut pa=sample();pa.effective_target_removed=true;
    let expected=format!("{}, effective_target_removed_by_dna: true }}",OLD_DEBUG.strip_suffix(" }").unwrap());
    assert_eq!(format!("{pa:?}"),expected);
    assert_eq!(format!("{:?}",pa.clone()),expected);
    assert_ne!(pending_field_hash(&Some(pa)),OLD_PENDING_FIELD_SHA256);
}
#[test]
fn both_verification_versions_distinguish_dna_target_state_and_preserve_clone(){
    // Structural digest fixture: no action is executed with these synthetic handles.
    let mut g=DebugRunner::builder().start().game;g.pending_attack=Some(sample());
    let mut removed=g.clone();removed.pending_attack.as_mut().unwrap().effective_target_removed=true;
    for version in [VerificationDigestVersion::LegacyV2,VerificationDigestVersion::V3] {
        assert_ne!(g.verification_digest_with_version(version),removed.verification_digest_with_version(version));
        assert_eq!(g.verification_digest_with_version(version),g.clone().verification_digest_with_version(version));
        assert_eq!(removed.verification_digest_with_version(version),removed.clone().verification_digest_with_version(version));
    }
}
#[test]
fn old_nondefault_state_fields_remain_in_the_hash(){
    let base=sample();let baseline=pending_field_hash(&Some(base.clone()));
    let mut variants=Vec::new();
    let mut p=base.clone();p.attacker.index+=1;variants.push(p);
    let mut p=base.clone();p.original_target=AttackTarget::Player(0);variants.push(p);
    let mut p=base.clone();p.effective_target=AttackTarget::Player(1);variants.push(p);
    let mut p=base.clone();p.is_blocked=false;variants.push(p);
    let mut p=base.clone();p.blocker=None;variants.push(p);
    let mut p=base.clone();p.is_vortex=true;variants.push(p);
    let mut p=base.clone();p.is_overclock=false;variants.push(p);
    let mut p=base.clone();p.declaration_committed=false;variants.push(p);
    let mut p=base.clone();p.cancelled=true;variants.push(p);
    let mut p=base.clone();p.battle_occurred=false;variants.push(p);
    let mut p=base.clone();p.battle_defender_deleted=true;variants.push(p);
    let mut p=base.clone();p.effect_battle_piercing_pending=false;variants.push(p);
    let mut p=base.clone();p.security_check_started=true;variants.push(p);
    let mut p=base.clone();p.return_phase=GamePhase::SelectTarget;variants.push(p);
    let mut p=base.clone();p.state=AttackState::Cleanup;variants.push(p);
    let mut p=base.clone();p.counter_depth=0;variants.push(p);
    for (index,variant) in variants.into_iter().enumerate(){assert_ne!(pending_field_hash(&Some(variant)),baseline,"old field {index} disappeared");}
}
