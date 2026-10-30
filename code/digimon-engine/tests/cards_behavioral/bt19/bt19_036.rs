//! BT19-036: checked against the actual image and official bundle.
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::owned_card_choice::CardChoiceOperation;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};
const ID: &str = "BT19-036";
fn card(
    id: &str,
    name: &str,
    color: CardColor,
    kind: CardKind,
    cost: u8,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.colors = vec![color];
    c.card_kind = kind;
    c.play_cost = cost as u16;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}
fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT19-036 production YAML")
        .add_card(card(
            "SEC",
            "Security",
            CardColor::Red,
            CardKind::Digimon,
            3,
            &[],
        ))
        .add_card(card(
            "WIZ",
            "Wizardmon",
            CardColor::Yellow,
            CardKind::Digimon,
            5,
            &["Data"],
        ))
        .add_card(card(
            "X",
            "X Antibody",
            CardColor::White,
            CardKind::Option,
            0,
            &[],
        ))
        .add_card(card(
            "NEAR",
            "Wizardmon (X Antibody)",
            CardColor::Yellow,
            CardKind::Digimon,
            6,
            &[],
        ))
        .add_card(card(
            "Y5",
            "Yellow five",
            CardColor::Yellow,
            CardKind::Option,
            5,
            &[],
        ))
        .add_card(card(
            "P5",
            "Purple five",
            CardColor::Purple,
            CardKind::Option,
            5,
            &[],
        ))
        .add_card(card(
            "Y6",
            "Yellow six",
            CardColor::Yellow,
            CardKind::Option,
            6,
            &[],
        ))
        .add_card(card(
            "R5",
            "Red five",
            CardColor::Red,
            CardKind::Option,
            5,
            &[],
        ))
        .add_card(card(
            "YD",
            "Yellow data",
            CardColor::Yellow,
            CardKind::Digimon,
            5,
            &["Data"],
        ))
        .add_card(card(
            "YW",
            "Yellow witch",
            CardColor::Yellow,
            CardKind::Digimon,
            5,
            &["Witchelny"],
        ))
        .add_card(card(
            "RD",
            "Red data",
            CardColor::Red,
            CardKind::Digimon,
            5,
            &["Data"],
        ))
        .add_card(card(
            "YN",
            "Yellow plain",
            CardColor::Yellow,
            CardKind::Digimon,
            5,
            &[],
        ))
        .deck(0, &["SEC"; 8])
        .deck(1, &["SEC"; 8])
        .memory(5)
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn setup(source: Option<&str>, security: &[&str], hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().security(0, security).hand(0, hand).start();
    r.skip_mulligan();
    let h = match source {
        Some(s) => r.place_stack(0, &[s, ID]),
        None => r.place_on_field(0, ID, Some(0)),
    };
    (r, h)
}
fn hand_rows(r: &DebugRunner) -> Vec<digimon_engine::owned_card_choice::CardChoiceRowView> {
    let p = r.game.pending_selection.as_ref().expect("hand prompt");
    p.card_choice
        .as_ref()
        .expect("owned hand choice")
        .view(&r.game, p.selecting_player)
        .rows
}
fn decline_action(r: &DebugRunner) -> u16 {
    hand_rows(r)
        .into_iter()
        .find(|row| matches!(row.operation, CardChoiceOperation::Decline))
        .expect("optional decline row")
        .action_id
}
fn pick_hand(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().unwrap();
    let a = hand_rows(r)
        .into_iter()
        .find(|row| {
            matches!(row.operation, CardChoiceOperation::CommitCard)
                && row.card_id.as_deref() == Some(id)
        })
        .expect("eligible physical hand card")
        .action_id;
    let mask = digimon_engine::action::build_action_mask(&r.game, v.selecting_player);
    assert!(mask[a as usize] > 0.5);
    assert!(mask[decline_action(r) as usize] > 0.5);
    r.execute_action(v.selecting_player, a).unwrap();
}
#[test]
fn bt19_036_on_play_adds_top_security_without_source() {
    let (mut r, h) = setup(None, &["Y5", "SEC"], &["P5"]);
    let n = r.game.players[0].hand.len();
    fire(&mut r, h, EffectTiming::OnPlay);
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(r.game.players[0].hand.len(), n + 1);
    assert_eq!(
        r.game.players[0]
            .hand
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "SEC"
    );
    assert!(r.pending_selection_view().is_none());
}
#[test]
fn bt19_036_evolution_filters_options_and_places_at_bottom() {
    for id in ["Y5", "P5"] {
        let (mut r, h) = setup(
            Some("WIZ"),
            &["SEC", "SEC"],
            &["Y5", "P5", "Y6", "R5", "YD"],
        );
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        let v = r.pending_selection_view().expect("hand choice");
        assert!(matches!(v.kind, SelectionKind::UnionZone { .. }));
        assert_eq!(
            hand_rows(&r)
                .iter()
                .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
                .count(),
            2,
            "only yellow/purple Options <=5"
        );
        pick_hand(&mut r, id);
        assert_eq!(r.game.players[0].security.len(), 2);
        assert_eq!(r.game.players[0].security[0].card_id(&r.game.card_data), id);
    }
}
#[test]
fn bt19_036_x_antibody_source_and_zero_security_still_offer_placement() {
    let (mut r, h) = setup(Some("X"), &[], &["Y5"]);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    pick_hand(&mut r, "Y5");
    assert_eq!(r.game.players[0].security.len(), 1);
}
#[test]
fn bt19_036_decline_keeps_hand_card() {
    let (mut r, h) = setup(Some("WIZ"), &["SEC"], &["P5"]);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    let v = r.pending_selection_view().unwrap();
    let decline = decline_action(&r);
    assert!(v.valid_action_ids.contains(&decline));
    r.execute_action(v.selecting_player, decline).unwrap();
    assert!(r.game.players[0].security.is_empty());
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "P5"));
}
#[test]
fn bt19_036_name_substrings_do_not_satisfy_source_condition() {
    let (mut r, h) = setup(Some("NEAR"), &["SEC"], &["P5"]);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].security.is_empty());
}
#[test]
fn bt19_036_no_eligible_hand_cards_does_not_open_choice() {
    let (mut r, h) = setup(Some("WIZ"), &[], &["Y6", "R5", "YD"]);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection_view().is_none());
}
fn inherited(carrier: &str, security: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().security(0, security).start();
    r.skip_mulligan();
    let h = r.place_stack(0, &[ID, carrier]);
    (r, h)
}
#[test]
fn bt19_036_inherited_saves_yellow_data_or_witchelny_once_per_turn() {
    for id in ["YD", "YW"] {
        let (mut r, h) = inherited(id, &["Y5", "SEC"]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
        r.accept_optional_trigger().unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert_eq!(r.game.players[0].security.len(), 1);
        assert_eq!(
            r.game.players[0]
                .trash
                .last()
                .unwrap()
                .card_id(&r.game.card_data),
            "SEC"
        );
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection_view().is_none());
        assert!(r.game.players[0].battle_area.is_empty());
    }
}
#[test]
fn bt19_036_inherited_filters_color_trait_cause_and_empty_security() {
    for (id, sec, cause) in [
        ("RD", true, ReplacementCause::OpponentEffect),
        ("YN", true, ReplacementCause::OpponentEffect),
        ("YD", false, ReplacementCause::OpponentEffect),
        ("YD", true, ReplacementCause::OwnEffect),
        ("YD", true, ReplacementCause::Battle),
    ] {
        let (mut r, h) = inherited(id, if sec { &["SEC"] } else { &[] });
        r.game.delete_permanent_with_cause(h, cause);
        assert!(
            r.pending_selection_view().is_none(),
            "ineligible replacement"
        );
        assert!(r.game.players[0].battle_area.is_empty());
    }
}
#[test]
fn bt19_036_inherited_decline_does_not_pay() {
    let (mut r, h) = inherited("YD", &["SEC"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.decline_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.game.players[0].security.len(), 1);
}

#[test]
fn bt19_036_inherited_does_not_protect_other_carriers_or_its_own_top() {
    let (mut r, _) = inherited("YD", &["SEC"]);
    let other = r.place_on_field(0, "YW", Some(1));
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].security.len(), 1);
    let (mut r, h) = setup(None, &["SEC"], &[]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn bt19_036_metadata_and_all_three_digivolution_paths() {
    use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
    let r = builder().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(6000), Some(6)));
    let paths: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(paths.len(), 3);
    assert_eq!(
        paths
            .iter()
            .filter(|p| p.cost == Some(CompiledCost::Literal(3)))
            .count(),
        2
    );
    assert_eq!(
        paths
            .iter()
            .filter(|p| p.cost == Some(CompiledCost::Literal(0)))
            .count(),
        1
    );
}

struct CancelSecurityLoss {
    optional: bool,
}
impl digimon_engine::effect::CardEffect for CancelSecurityLoss {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let b = digimon_engine::effect::Effect::when_would_be_trashed(card)
            .replacement_condition(|_, subject| {
                matches!(
                    subject,
                    digimon_engine::replacement::ReplacementSubject::Card(
                        _,
                        digimon_engine::enums::Zone::Security
                    )
                )
            })
            .replacement_process(|rctx| rctx.cancel());
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
fn guard(r: &mut DebugRunner, optional: bool) {
    r.register_effect("YN", std::sync::Arc::new(CancelSecurityLoss { optional }));
    r.place_on_field(0, "YN", Some(0));
}
#[test]
fn bt19_036_canceled_inherited_payment_does_not_prevent_deletion() {
    let (mut r, h) = inherited("YD", &["SEC"]);
    guard(&mut r, false);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.len(), 1);
    assert!(!r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "YD"));
}
#[test]
fn bt19_036_parked_inherited_payment_resumes_from_physical_result() {
    let (mut r, h) = inherited("YD", &["SEC"]);
    guard(&mut r, true);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    assert_eq!(r.game.players[0].security.len(), 1);
    let mut canceled = builder().start();
    canceled.game = r.game.clone();
    canceled.accept_optional_trigger().unwrap();
    canceled.auto_resolve().unwrap();
    assert_eq!(canceled.game.players[0].security.len(), 1);
    assert!(!canceled.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&canceled.game.card_data) == "YD"));
    r.decline_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].security.is_empty());
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "YD"));
}

#[test]
fn bt19_036_hand_placement_pages_every_legal_card_without_consuming_it() {
    let mut hand = vec!["Y5"; 35];
    hand.push("P5");
    let (mut r, h) = setup(Some("X"), &[], &hand);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    let size = r.game.players[0].hand.len();
    let v = r.pending_selection_view().unwrap();
    let next = hand_rows(&r)
        .into_iter()
        .find(|row| matches!(row.operation, CardChoiceOperation::NextPage))
        .expect("all candidates are paged")
        .action_id;
    let mask = digimon_engine::action::build_action_mask(&r.game, v.selecting_player);
    assert!(mask[next as usize] > 0.5);
    r.execute_action(v.selecting_player, next).unwrap();
    assert_eq!(r.game.players[0].hand.len(), size);
    assert!(r.game.players[0].security.is_empty());
    pick_hand(&mut r, "P5");
    assert_eq!(r.game.players[0].hand.len(), size - 1);
    assert_eq!(
        r.game.players[0].security[0].card_id(&r.game.card_data),
        "P5"
    );
}
