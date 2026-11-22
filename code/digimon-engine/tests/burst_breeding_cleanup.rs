//! Shared CR 8-3-2-1 rule processing in breeding; not a card-text workaround.
use digimon_engine::{
    action::space::BREEDING_TARGET,
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    enums::{CardKind, EffectSourceKind},
    permanent::PermanentHandle,
};
fn setup(kind: CardKind, stacked: bool) -> DebugRunner {
    let mut top = make_test_card("TOP", "Top");
    top.card_kind = kind;
    let mut r = DebugRunner::builder()
        .add_card(top)
        .add_card(make_test_card("BASE", "Base"))
        .build();
    let h = r.place_stack(0, if stacked { &["BASE", "TOP"] } else { &["TOP"] });
    r.game.players[0].breeding_area = Some(r.game.players[0].battle_area.remove(h.index as usize));
    r.place_stack(0, &["BASE", "TOP"]);
    r
}
fn apply(r: &mut DebugRunner, kind: EffectSourceKind) -> bool {
    let host = PermanentHandle {
        player: 0,
        index: BREEDING_TARGET as u8,
    };
    let card = r.game.players[0]
        .breeding_area
        .as_ref()
        .unwrap()
        .top_card()
        .handle();
    EffectContext::new_with_source_kind(&mut r.game, card, Some(host), kind, 0)
        .trash_top_source(host)
}
#[test]
fn rule_trashes_digimon_or_dual_top_and_preserves_other_stack() {
    for kind in [CardKind::Digimon, CardKind::Dual] {
        let mut r = setup(kind, true);
        assert!(apply(&mut r, EffectSourceKind::Rule));
        assert_eq!(
            r.game.players[0]
                .breeding_area
                .as_ref()
                .unwrap()
                .card_sources
                .len(),
            1
        );
        assert_eq!(
            r.game.players[0]
                .breeding_area
                .as_ref()
                .unwrap()
                .top_card()
                .card_id(&r.game.card_data),
            "BASE"
        );
        assert_eq!(r.game.players[0].trash.len(), 1);
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
    }
}
#[test]
fn rule_does_not_trash_single_card_or_non_digimon_top() {
    for (kind, stacked) in [
        (CardKind::Digimon, false),
        (CardKind::Tamer, true),
        (CardKind::Option, true),
        (CardKind::DigiEgg, true),
    ] {
        let mut r = setup(kind, stacked);
        assert!(!apply(&mut r, EffectSourceKind::Rule));
        assert!(r.game.players[0].trash.is_empty());
    }
}
#[test]
fn ordinary_effect_cannot_trash_breeding_top() {
    let mut r = setup(CardKind::Digimon, true);
    assert!(!apply(&mut r, EffectSourceKind::Digimon));
    assert_eq!(
        r.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .card_sources
            .len(),
        2
    );
}
#[test]
fn rule_routes_to_physical_owner_and_exposes_trash() {
    let mut r = setup(CardKind::Digimon, true);
    {
        let c = r.game.players[0]
            .breeding_area
            .as_mut()
            .unwrap()
            .card_sources
            .last_mut()
            .unwrap();
        c.owner = 1;
        c.face_down = true;
    }
    assert!(apply(&mut r, EffectSourceKind::Rule));
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.game.players[1].trash.len(), 1);
    assert!(!r.game.players[1].trash[0].face_down);
}
#[path = "cards_behavioral/burst_digivolve_common.rs"]
mod common;
macro_rules! legacy_control {
    ($test:ident,$card:expr,$host:expr,$color:ident,$tamer:expr) => {
        #[test]
        fn $test() {
            use common::*;
            let s = BurstSpec {
                card: $card,
                host_name: $host,
                host_color: digimon_engine::enums::CardColor::$color,
                tamer_name: $tamer,
            };
            assert_turn_end_trash(s, Host::BurstOnly);
            assert_route_choice(s);
            assert_effect_cannot_burst(s);
        }
    };
}
legacy_control!(
    legacy_shine_bt13,
    "BT13-020",
    "ShineGreymon",
    Yellow,
    "Marcus Damon"
);
legacy_control!(
    legacy_mirage_bt13,
    "BT13-033",
    "MirageGaogamon",
    Blue,
    "Thomas H. Norstein"
);
legacy_control!(
    legacy_rose_bt13,
    "BT13-060",
    "Rosemon",
    Green,
    "Yoshino Fujieda"
);
legacy_control!(
    legacy_rave_bt13,
    "BT13-092",
    "Ravemon",
    Purple,
    "Keenan Crier"
);
legacy_control!(
    legacy_shine_bt25,
    "BT25-104",
    "ShineGreymon",
    Red,
    "Marcus Damon"
);
legacy_control!(
    legacy_rose_bt26,
    "BT26-050",
    "Rosemon",
    Green,
    "Yoshino Fujieda"
);
