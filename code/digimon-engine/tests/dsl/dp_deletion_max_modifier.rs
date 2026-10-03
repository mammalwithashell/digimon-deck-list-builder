//! G-ENGINE-DP-DELETION-MAX-MODIFIER — "[All Turns] Add 2000 to this Digimon's
//! DP deletion effects' maximums" (EX13-007 Guilmon / EX13-010 Growlmon
//! inherited; BT17-008 / BT17-010 / BT19-007 / BT19-009 memory-gated).
//!
//! Substrate under test:
//! - `kind: aura` `modifier: ChangeDPDeleteEffectMaxDP` / `modifier_value: N`.
//!   A self-aura (face-up or inherited) lowers to
//!   `Effect::dp_delete_effect_max_delta`, read LIVE from the host's stack by
//!   `Game::dp_delete_effect_max_bonus` (so a card that becomes a source by
//!   the very digivolve whose [When Digivolving] deletes counts at once);
//!   other routes install the materialized
//!   `ModifierType::ChangeDPDeleteEffectMaxDP`, which the same API folds in.
//! - `digimon_dsl::deletion_cap` — the compile-time pass that flags the
//!   `dp_lte` of a deletion effect's target filter (`dp_lte_deletion_cap`) and
//!   the total-DP budget of a deletion-consumed `select_opponent_dp_budget`.
//! - The engine consult: a flagged cap adds the EFFECT SOURCE permanent's
//!   summed delta (DCGO `Player.MaxDP_DeleteEffect` →
//!   `ChangeDPDeleteEffectMaxDPClass.ChangeMaxDP`, which adds only when
//!   `cardEffect.EffectSourceCard.PermanentOfThisCard()` is its own permanent).

use digimon_dsl::compiled::{CompiledClause, CompiledPredicate, CompiledStep};
use digimon_dsl::{compile, CardSpec};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use std::sync::Arc;

const MAXER_YAML: &str = r#"
card: MAXER
name: Maxermon
kind: digimon
effects:
  - kind: aura
    scope: inherited
    active_when: { all_turns: true }
    target: {}
    modifier: ChangeDPDeleteEffectMaxDP
    modifier_value: 2000
"#;

/// Face-up self-aura variant: the delta lives on THIS permanent only.
const FACEUP_MAXER_YAML: &str = r#"
card: FACEUP-MAXER
name: Faceupmon
kind: digimon
effects:
  - kind: aura
    target: {}
    modifier: ChangeDPDeleteEffectMaxDP
    modifier_value: 2000
"#;

const DELETER_YAML: &str = r#"
card: DELETER
name: Deletermon
kind: digimon
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 4000
          prompt: "Delete 1 of your opponent's Digimon with 4000 DP or less"
      - delete_permanent: { target: victim }
"#;

const BOUNCER_YAML: &str = r#"
card: BOUNCER
name: Bouncermon
kind: digimon
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 4000
          prompt: "Return 1 of your opponent's Digimon with 4000 DP or less to the hand"
      - return_to_hand: { target: tgt }
"#;

const BUDGETER_YAML: &str = r#"
card: BUDGETER
name: Budgetermon
kind: digimon
effects:
  - when: when_digivolving
    process:
      - select_opponent_dp_budget:
          dp_budget: 4000
          min_picks: 1
          filter: { kind: digimon }
          bind_as: targets
          prompt: "Delete any number of your opponent's Digimon with total DP up to 4000"
          then:
            - delete_bound_permanents: { binding: targets }
"#;

fn compiled(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    compile::compile(&spec).expect("compile card yaml")
}

fn register(r: &mut DebugRunner, yaml: &str) {
    let c = compiled(yaml);
    let id = c.card.clone();
    r.register_effect(&id, Arc::new(DslCardEffect::new(Arc::new(c))));
}

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn lv4_from_red_lv3(id: &str) -> CardData {
    let mut c = digimon(id, 4, 5000);
    c.evo_costs.push(EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 1,
    });
    c
}

fn runner(hand: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(digimon("MAXER", 3, 1000))
        .add_card(digimon("FACEUP-MAXER", 3, 1000))
        .add_card(digimon("PLAIN3", 3, 2000))
        .add_card(lv4_from_red_lv3("DELETER"))
        .add_card(lv4_from_red_lv3("BOUNCER"))
        .add_card(lv4_from_red_lv3("BUDGETER"))
        .add_card(digimon("OPP-4K", 4, 4000))
        .add_card(digimon("OPP-6K", 4, 6000))
        .add_card(digimon("OPP-7K", 4, 7000))
        .add_card(digimon("OPP-9K", 5, 9000))
        .add_card(digimon("FILL", 3, 1000))
        .hand(0, hand)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(5)
        .start();
    for y in [
        MAXER_YAML,
        FACEUP_MAXER_YAML,
        DELETER_YAML,
        BOUNCER_YAML,
        BUDGETER_YAML,
    ] {
        register(&mut r, y);
    }
    r
}

fn digivolve(r: &mut DebugRunner, id: &str, base: PermanentHandle) {
    let slot = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("card in hand") as u16;
    r.game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
}

fn offered(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.pending_selection_view().is_some_and(|v| {
        v.valid_action_ids
            .contains(&(ATTACK_START + h.index as u16))
    })
}

fn first_process(c: &digimon_dsl::compiled::CompiledCard) -> &Vec<CompiledStep> {
    match &c.effects[0] {
        CompiledClause::Triggered(t) => &t.process,
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn select_filter(step: &CompiledStep) -> &CompiledPredicate {
    match step {
        CompiledStep::SelectOpponentPermanent { filter, .. } => filter,
        other => panic!("expected select_opponent_permanent, got {other:?}"),
    }
}

// ─── Compile-time marking ────────────────────────────────────────────────────

#[test]
fn deletion_target_filter_dp_cap_is_flagged_as_deletion_cap() {
    let c = compiled(DELETER_YAML);
    let f = select_filter(&first_process(&c)[0]);
    let cap = f
        .all_of
        .iter()
        .find(|p| p.dp_lte.is_some())
        .expect("dp_lte leaf");
    assert!(
        cap.dp_lte_deletion_cap,
        "a delete's DP cap is a deletion maximum"
    );
}

#[test]
fn non_deletion_dp_cap_is_not_flagged() {
    let c = compiled(BOUNCER_YAML);
    let f = select_filter(&first_process(&c)[0]);
    let cap = f
        .all_of
        .iter()
        .find(|p| p.dp_lte.is_some())
        .expect("dp_lte leaf");
    assert!(
        !cap.dp_lte_deletion_cap,
        "a return-to-hand DP cap is not a DP deletion effect's maximum"
    );
}

#[test]
fn deletion_consumed_dp_budget_is_flagged() {
    let c = compiled(BUDGETER_YAML);
    match &first_process(&c)[0] {
        CompiledStep::SelectOpponentDpBudget { deletion_cap, .. } => assert!(*deletion_cap),
        other => panic!("expected select_opponent_dp_budget, got {other:?}"),
    }
}

#[test]
fn inherited_aura_grants_the_carrier_a_live_deletion_max_bonus() {
    let mut r = runner(&[]);
    let carrier = r.place_stack(0, &["MAXER", "PLAIN3"]);
    let bare = r.place_on_field(0, "MAXER", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.game.dp_delete_effect_max_bonus(carrier), 2000);
    assert_eq!(
        r.game.dp_delete_effect_max_bonus(bare),
        0,
        "inherited: a face-up MAXER grants nothing to itself"
    );
}

// ─── Runtime consult ─────────────────────────────────────────────────────────

#[test]
fn inherited_delta_raises_the_carriers_deletion_cap() {
    let mut r = runner(&["DELETER"]);
    let base = r.place_stack(0, &["MAXER", "PLAIN3"]);
    let o4 = r.place_on_field(1, "OPP-4K", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    let o7 = r.place_on_field(1, "OPP-7K", Some(0));
    digivolve(&mut r, "DELETER", base);
    assert!(offered(&r, o4), "4000 DP within 4000+2000");
    assert!(offered(&r, o6), "6000 DP within 4000+2000");
    assert!(!offered(&r, o7), "7000 DP exceeds 6000");
}

#[test]
fn without_the_modifier_the_printed_cap_holds() {
    let mut r = runner(&["DELETER"]);
    let base = r.place_on_field(0, "PLAIN3", Some(0));
    let o4 = r.place_on_field(1, "OPP-4K", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    digivolve(&mut r, "DELETER", base);
    assert!(offered(&r, o4));
    assert!(!offered(&r, o6), "printed 4000 cap");
}

#[test]
fn non_deletion_cap_is_not_raised() {
    let mut r = runner(&["BOUNCER"]);
    let base = r.place_stack(0, &["MAXER", "PLAIN3"]);
    let o4 = r.place_on_field(1, "OPP-4K", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    digivolve(&mut r, "BOUNCER", base);
    assert!(offered(&r, o4));
    assert!(!offered(&r, o6), "a bounce is not a DP deletion effect");
}

#[test]
fn another_permanents_delta_does_not_apply() {
    let mut r = runner(&["DELETER"]);
    let base = r.place_on_field(0, "PLAIN3", Some(0));
    let other = r.place_on_field(0, "FACEUP-MAXER", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.game.dp_delete_effect_max_bonus(other), 2000);
    digivolve(&mut r, "DELETER", base);
    assert!(
        !offered(&r, o6),
        "only the deletion effect's own source permanent's delta counts"
    );
}

#[test]
fn stacked_deltas_sum() {
    let mut r = runner(&["DELETER"]);
    let base = r.place_stack(0, &["MAXER", "MAXER", "PLAIN3"]);
    let o7 = r.place_on_field(1, "OPP-7K", Some(0));
    let o9 = r.place_on_field(1, "OPP-9K", Some(0));
    digivolve(&mut r, "DELETER", base);
    assert!(offered(&r, o7), "4000 + 2000 + 2000 = 8000");
    assert!(!offered(&r, o9));
}

#[test]
fn deletion_dp_budget_is_raised() {
    let mut r = runner(&["BUDGETER"]);
    let base = r.place_stack(0, &["MAXER", "PLAIN3"]);
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    let o7 = r.place_on_field(1, "OPP-7K", Some(0));
    digivolve(&mut r, "BUDGETER", base);
    assert!(offered(&r, o6), "total-DP budget 4000 + 2000");
    assert!(!offered(&r, o7));
}

#[test]
fn source_joining_by_this_digivolve_counts_immediately() {
    // MAXER is face-up before the digivolve and becomes a source BY it — the
    // [When Digivolving] delete must already see the inherited +2000 (no
    // declarative tick runs between the digivolve and its trigger).
    let mut r = runner(&["DELETER"]);
    let base = r.place_on_field(0, "MAXER", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    digivolve(&mut r, "DELETER", base);
    assert!(offered(&r, o6), "4000 + 2000 from the just-stacked MAXER");
}

#[test]
fn materialized_modifier_from_other_routes_also_counts() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = runner(&["DELETER"]);
    let base = r.place_on_field(0, "PLAIN3", Some(0));
    let o7 = r.place_on_field(1, "OPP-7K", Some(0));
    r.game.modifiers.add(
        base,
        ModifierEntry::simple(
            ModifierType::ChangeDPDeleteEffectMaxDP,
            3000,
            Expiry::EndOfTurn,
            0,
        ),
    );
    assert_eq!(r.game.dp_delete_effect_max_bonus(base), 3000);
    digivolve(&mut r, "DELETER", base);
    assert!(offered(&r, o7), "4000 + 3000");
}
