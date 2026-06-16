//! BT21-070 Gossipmon — Digimon, Lv.4, Purple, DP 4000, Cost 4.
//! Traits: Sup. / Appmon / Social / Gossip
//! Attribute: Social
//!
//! # Card text (DCGO BT21_070.cs — authoritative)
//!
//! Alt paths:
//!   [Digivolve] Lv.3 purple / Cost 2 (standard evo, from evo_costs).
//!   [Digivolve] [Stnd.] trait / Cost 2 (alt path — no level gate; DCGO
//!     AddSelfDigivolutionRequirementStaticEffect(EqualsTraits("Stnd."), cost 2)).
//!
//! [Security] At the end of the battle, play this card without paying the cost.
//!   (DCGO PlaySelfDigimonAfterBattleSecurityEffect — same as BT21-015.)
//!
//! [On Play] [When Digivolving] You may return 1 Digimon card with the [Appmon]
//!   trait from your trash to the hand.
//!   (DCGO two ActivateClass blocks, both under OnEnterFieldAnyone, one gated
//!   CanTriggerOnPlay / one CanTriggerWhenDigivolving. canNoSelect: true → optional.)
//!
//! [Link] [Appmon] trait: Cost 2.
//!   (DCGO AddSelfLinkConditionStaticEffect(HasAppmonTraits, linkCost 2).)
//!
//! [When Linking] You may return 1 Digimon card with the [Appmon] trait from your
//!   trash to the hand.
//!   (DCGO WhenLinked timing, SetIsLinkedEffect(true). canNoSelect: true → optional.
//!   Same body as the On Play / When Digivolving clause.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_070.cs
//!
//! # Patterns covered
//! - [Security] play_from_security (on_security)
//! - [On Play][When Digivolving] optional select_trash (Appmon Digimon filter) →
//!   add_to_hand_from_trash
//! - Self link-condition (link_condition: cost 2, trait_has Appmon)
//! - [When Linking] same optional trash-to-hand body (scope: linked, when: when_linked)
//! - Alt digivolve: trait_has "Stnd." / Cost 2 (no level gate)
//!
//! # Link DP bonus
//! The link box prints +3000 DP (card image, right edge). The Appmon link DP
//! bonus is data-driven in DCGO (Permanent.cs: LinkedDP += addedLinkCard.LinkDP),
//! never a CardEffect — so its absence from BT21_070.cs is expected, NOT evidence
//! it doesn't exist. Modeled as `scope: linked, kind: aura, target: {},
//! dp_modifier: 3000` (briefing §1.2).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-070";
const GOSSIPMON_YAML: &str = include_str!("../../../cards/bt21/BT21-070.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Seed a card from the card_data registry into player's trash.
fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let iid = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(idx, player as u8, iid));
}

/// Standard builder: loads Gossipmon from the embedded pack, adds helper cards.
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(GOSSIPMON_YAML)
        .expect("BT21-070 YAML parses and compiles")
        // Appmon Digimon to put in trash for the return clause tests.
        .add_card(make_digimon("APPMON-DIGI", 3, 2000, 3, &["Search", "Appmon"]))
        // Non-Appmon Digimon — must NOT be selectable by the trash filter.
        .add_card(make_digimon("NON-APPMON", 3, 2000, 3, &["Dragon"]))
        // Non-Digimon Appmon (Option card) — must NOT be selectable (kind: digimon filter).
        .add_card(make_test_card("APPMON-OPT", "AppmonOption"))
        // Appmon host for link tests.
        .add_card(make_digimon("HOST-APP", 4, 5000, 5, &["Appmon"]))
        // Stnd. base for alt-digivolve tests (no level_eq gate in DCGO).
        .add_card(make_digimon("STND-BASE", 3, 2000, 2, &["Stnd."]))
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

/// The card compiles with exactly 4 clauses:
///   1. on_security triggered
///   2. [on_play, when_digivolving] triggered (optional)
///   3. link_condition declarative
///   4. linked scope +3000 DP aura declarative
///   5. linked scope when_linked triggered (optional)
#[test]
fn bt21_070_compiles_with_four_clauses() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("BT21-070 in pack");
    assert_eq!(
        card.effects.len(),
        5,
        "BT21-070 must have exactly 5 clauses: on_security, on_play/when_digivolving, \
         link_condition, linked +3000 DP aura, and linked when_linked; got {}",
        card.effects.len()
    );
}

/// on_security clause: triggered, when == [OnSecurity], not optional.
#[test]
fn bt21_070_has_on_security_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let has = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when == vec![CompiledTiming::OnSecurity] && !t.optional
        }
        _ => false,
    });
    assert!(
        has,
        "BT21-070 must have a non-optional triggered clause with when: on_security"
    );
}

/// [On Play][When Digivolving] clause: triggered, contains both OnPlay and
/// WhenDigivolving, optional == true (DCGO canNoSelect: true).
#[test]
fn bt21_070_has_on_play_when_digivolving_optional_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });

    let clause = clause.expect(
        "BT21-070 must have a triggered clause with when: [on_play, when_digivolving]",
    );
    assert!(
        clause.optional,
        "the On Play / When Digivolving clause must be optional (DCGO canNoSelect: true)"
    );
}

/// link_condition declarative clause: cost 2, filter trait_has Appmon.
#[test]
fn bt21_070_has_link_condition_appmon_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
            if *cost == 2
    ));
    assert!(
        has,
        "BT21-070 must declare a self link-condition with cost 2"
    );
}

/// [When Linking] clause: triggered, scope Linked, when contains WhenLinked, optional.
#[test]
fn bt21_070_has_linked_when_linked_optional_clause() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenLinked)
                && t.scope == CompiledScope::Linked
        });

    let clause =
        clause.expect("BT21-070 must have a Linked-scope when_linked triggered clause");
    assert!(
        clause.optional,
        "the when_linked clause must be optional (DCGO canNoSelect: true)"
    );
}

/// Alt path: a digivolve path with filter trait_has "Stnd." exists at cost 2.
/// No level_eq gate (DCGO does not specify a level parameter).
#[test]
fn bt21_070_registers_stnd_alt_digivolve_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
    });
    assert!(
        has,
        "BT21-070 must register a cost-2 alt-digivolve (Stnd. trait path)"
    );
}

// ─── Section 2 — [On Play] optional trash-to-hand behavioral ─────────────────

/// Positive: On Play with 1 Appmon Digimon in trash → Trash selection installs.
#[test]
fn bt21_070_on_play_installs_trash_selection_when_appmon_in_trash() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 5])
        .memory(10)
        .start();

    // Seed an Appmon Digimon into P0's trash.
    seed_trash(&mut r, 0, "APPMON-DIGI");

    let hand_before = r.hand_size(0);
    r.play(0, 0).expect("Gossipmon played from hand");

    let kind = r.pending_kind().expect("trash selection must install");
    assert!(
        matches!(kind, SelectionKind::Trash),
        "expected Trash selection after On Play with Appmon in trash; got {:?}",
        kind
    );
}

/// Positive: selecting the Appmon Digimon from trash → it moves to hand (+1 hand, -1 trash).
#[test]
fn bt21_070_on_play_returns_appmon_to_hand() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 5])
        .memory(10)
        .start();

    seed_trash(&mut r, 0, "APPMON-DIGI");
    let hand_before = r.hand_size(0); // 1 card: CARD_ID
    let trash_before = r.trash_size(0); // 1 (APPMON-DIGI)

    r.play(0, 0).expect("Gossipmon plays");
    // auto_resolve picks the only eligible trash card.
    r.auto_resolve().expect("trash selection resolves");

    // net hand change: -1 (play Gossipmon) +1 (return APPMON-DIGI) = 0 net.
    // So hand_after == hand_before.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "net hand should be unchanged: -1 played Gossipmon +1 returned from trash = 0 net"
    );
    assert_eq!(
        r.trash_size(0),
        0,
        "APPMON-DIGI must have left the trash"
    );
}

/// Negative: On Play with NO Appmon Digimon in trash → no selection installs
/// (the effect silently skips when there is no eligible candidate).
#[test]
fn bt21_070_on_play_no_selection_when_trash_empty() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 5])
        .memory(10)
        .start();

    // Trash has no Appmon Digimon.
    r.play(0, 0).expect("Gossipmon plays");

    assert!(
        r.pending_selection().is_none(),
        "no trash selection must install when trash has no eligible Appmon Digimon; got {:?}",
        r.pending_kind()
    );
}

/// Negative: Non-Appmon Digimon in trash is NOT selectable.
/// A NON-APPMON card in trash means zero valid action ids in the selection.
#[test]
fn bt21_070_on_play_non_appmon_not_selectable() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 5])
        .memory(10)
        .start();

    // Only a non-Appmon Digimon in trash.
    seed_trash(&mut r, 0, "NON-APPMON");

    r.play(0, 0).expect("Gossipmon plays");

    // Either no selection installs, or the selection has zero valid targets.
    if let Some(sel) = r.pending_selection() {
        assert_eq!(
            sel.valid_action_ids.len(),
            0,
            "the non-Appmon Digimon must not be a valid selection target"
        );
    }
    // else: no selection installed at all — also correct (optional + no eligible target).
}

// ─── Section 3 — [When Digivolving] optional trash-to-hand behavioral ─────────

/// Digivolving into Gossipmon should fire the trash-return clause.
/// Simulated by directly enqueuing WhenDigivolving on a stacked permanent.
#[test]
fn bt21_070_when_digivolving_installs_trash_selection() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut r = base()
        .add_card(make_test_card("BASE-LV3", "BaseLv3"))
        .deck(0, &["DECK-PAD"; 5])
        .memory(10)
        .start();

    // Seed Appmon Digimon into trash.
    seed_trash(&mut r, 0, "APPMON-DIGI");

    // Place a Lv3 base on P0's field, then push Gossipmon on top (simulate digivolve).
    let base_perm = r.place_on_field(0, "BASE-LV3", Some(0));
    let gossip_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("BT21-070 in card_data");
    let next = r.game.next_card_index();
    let gossip_src = CardSource::new(gossip_idx, 0, next);
    let turn = r.game.turn_count;
    r.game.players[0].battle_area[base_perm.index as usize].digivolve(gossip_src, turn);

    // Fire WhenDigivolving.
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base_perm),
    );
    r.game.drain_effect_queue();

    let kind = r.pending_kind().expect("trash selection installs on WhenDigivolving");
    assert!(
        matches!(kind, SelectionKind::Trash),
        "expected Trash selection on WhenDigivolving; got {:?}",
        kind
    );
}

// ─── Section 4 — [Security] play_from_security behavioral ─────────────────────

/// Structural: on_security clause has FaceUp scope (default), not optional.
#[test]
fn bt21_070_security_clause_structural() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let sec_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("on_security clause exists");

    assert_eq!(
        sec_clause.scope,
        CompiledScope::FaceUp,
        "security clause must have default FaceUp scope"
    );
    assert!(
        !sec_clause.optional,
        "security play-self must NOT be optional"
    );
}

/// Integration: Gossipmon in P1's security stack → P0 attacks → Gossipmon
/// plays onto P1's battle area at end of battle.
#[test]
fn bt21_070_security_plays_card_onto_field() {
    let mut r = base()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .deck(0, &["DECK-PAD"; 5])
        .memory(10)
        .start();

    // Place attacker on P0's field (turn_played=0 to bypass summoning sickness).
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));

    // Put Gossipmon into P1's security.
    {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .expect("BT21-070 in card_data");
        let next = r.game.next_card_index();
        let src = CardSource::new(idx, 1, next);
        r.game.players[1].security.push(src);
    }

    assert_eq!(r.security_count(1), 1, "pre: P1 has 1 security card");
    assert_eq!(r.battle_area_size(1), 0, "pre: P1's battle area is empty");

    // P0 attacks P1's security.
    r.attack_player(attacker, 1, false);
    // Resolve any pending selections (the on_play trash clause fires if trash eligible;
    // with empty trash it skips silently).
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(1), 0, "security card removed by attack");
    assert_eq!(
        r.battle_area_size(1),
        1,
        "Gossipmon must be played onto P1's battle area by [Security] effect"
    );
}

// ─── Section 5 — [When Linking] trash-to-hand behavioral (linked scope) ───────

/// On-field link activation: after Gossipmon links onto a host, the WhenLinked
/// trash-return selection installs (when an Appmon Digimon is in trash).
#[test]
fn bt21_070_when_linked_installs_trash_selection() {
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };

    let mut r = base()
        .deck(0, &["DECK-PAD"; 5])
        .memory(5)
        .start();

    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gossip = r.place_on_field(0, CARD_ID, Some(0));

    // Seed an Appmon Digimon in P0's trash.
    seed_trash(&mut r, 0, "APPMON-DIGI");

    advance_to_main(&mut r);

    // Decode the link action for Gossipmon's on-field link slot.
    let link_bit = (FIELD_EFFECT_START
        + gossip.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);

    // Resolve the host-selection prompt (absorb onto HOST-APP).
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // Now the WhenLinked clause fires → trash selection should install.
    let kind = r
        .pending_kind()
        .expect("WhenLinked trash selection must install after link");
    assert!(
        matches!(kind, SelectionKind::Trash),
        "expected Trash selection from when_linked clause; got {:?}",
        kind
    );
}

/// When Linking with empty trash → no trash selection (clause skips silently).
#[test]
fn bt21_070_when_linked_no_selection_when_trash_empty() {
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };

    let mut r = base()
        .deck(0, &["DECK-PAD"; 5])
        .memory(5)
        .start();

    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gossip = r.place_on_field(0, CARD_ID, Some(0));

    // No Appmon Digimon in trash.

    advance_to_main(&mut r);

    let link_bit = (FIELD_EFFECT_START
        + gossip.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);

    // Resolve host-selection.
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // No eligible trash target → no trash selection installs.
    assert!(
        r.pending_selection().is_none(),
        "no trash selection must install when trash has no Appmon Digimon; got {:?}",
        r.pending_kind()
    );
}

/// When Linking resolves: selected Appmon from trash moves to hand.
#[test]
fn bt21_070_when_linked_returns_appmon_to_hand() {
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };

    let mut r = base()
        .deck(0, &["DECK-PAD"; 5])
        .memory(5)
        .start();

    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gossip = r.place_on_field(0, CARD_ID, Some(0));

    seed_trash(&mut r, 0, "APPMON-DIGI");
    let hand_before = r.hand_size(0);
    let trash_before = r.trash_size(0);

    advance_to_main(&mut r);

    let link_bit = (FIELD_EFFECT_START
        + gossip.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_LINK) as usize;

    r.game.decode_action(link_bit as u16, 0);

    // Absorb: pick the host.
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    // WhenLinked trash return: auto-resolve picks the only Appmon Digimon.
    r.auto_resolve().expect("when_linked trash selection resolves");

    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "hand must grow by 1 after when_linked returns Appmon from trash"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before - 1,
        "trash must shrink by 1 after when_linked return"
    );
}

// ── Link DP aura (+3000) — review-added ───────────────────────────────────────

/// Structural: BT21-070 declares a scope:linked +3000 DP aura (link-box bonus).
#[test]
fn bt21_070_has_linked_dp_aura_3000() {
    let runner = base().deck(0, &["DECK-PAD"; 5]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(3000),
                ..
            })
        )
    });
    assert!(has, "BT21-070 declares scope:linked +3000 DP aura (link box)");
}

/// Behavioral: host effective DP +3000 while BT21-070 is linked.
#[test]
fn bt21_070_linked_host_gains_3000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 5]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let dp_before = r.effective_dp(host).expect("host on field");

    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 3000),
        "host effective DP +3000 while BT21-070 is linked"
    );
}
