use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};
use digimon_engine::selection::SelectionKind;

#[test]
fn would_timings_exist() {
    let _ = EffectTiming::WhenWouldBeDeleted;
    let _ = EffectTiming::WhenWouldLeaveBattleArea;
    let _ = EffectTiming::WhenWouldBeReturnedToHand;
    let _ = EffectTiming::WhenWouldBeReturnedToDeck;
    let _ = EffectTiming::WhenWouldBeTrashed;
    let _ = EffectTiming::WhenWouldBeDeDigivolved;
    let _ = EffectTiming::WhenWouldLoseSecurity;
    let _ = EffectTiming::WhenWouldDraw;
    let _ = EffectTiming::WhenWouldPlaceInSecurity;
    let _ = EffectTiming::WhenWouldAttack;
    let _ = EffectTiming::WhenWouldBeAttackTarget;
}

#[test]
fn replacement_selection_kind_exists() {
    let _ = SelectionKind::Replacement;
}

#[test]
fn replacement_cause_variants_exist() {
    let _ = ReplacementCause::Battle;
    let _ = ReplacementCause::OwnEffect;
    let _ = ReplacementCause::OpponentEffect;
    let _ = ReplacementCause::SecurityCheck;
    let _ = ReplacementCause::Cost;
}

#[test]
fn replacement_outcome_defaults_none() {
    assert_eq!(ReplacementOutcome::None, ReplacementOutcome::None);
}

#[test]
fn new_keywords_exist() {
    let _ = Keyword::Evade;
    let _ = Keyword::Fragment(3);
    let _ = Keyword::Decode;
    let _ = Keyword::ArmorPurge;
}

#[test]
fn replacement_subject_variants_exist() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::Zone;
    use digimon_engine::permanent::PermanentHandle;
    let _ = ReplacementSubject::Permanent(PermanentHandle {
        player: 0,
        index: 0,
    });
    let _ = ReplacementSubject::Card(CardHandle(0), Zone::Hand);
    let _ = ReplacementSubject::Player(0);
}
