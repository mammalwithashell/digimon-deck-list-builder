//! EX7-023 Hexeblaumon — Digimon, Lv.6, Blue, DP 12000, Cost 12.
//! Traits: Magic Knight, Witchelny. Attribute: Data. Form: Mega. Rarity: SR.
//! Rule box (card image): "Rule: Trait: Has [Ice-Snow] Type." — the card
//! permanently has the [Ice-Snow] trait in addition to its printed type line
//! (cards.json type_eng carries only ["Magic Knight", "Witchelny"]; same
//! API-ingest gap as siblings EX7-016 / EX7-020 / EX7-021).
//! Standard digivolution: Lv.5 blue / cost 4 (cards.json evo_costs; the card
//! image shows a single digivolve circle — no alt evo box printed).
//!
//! # Card text (card image EX7-023.webp — authoritative; matches cards.json)
//!
//! **<Security A. +1>** (This Digimon checks 1 additional security card.)
//!
//! **<Iceclad>** (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//!
//! **[When Digivolving]** Trash any 4 digivolution cards of your opponent's
//! Digimon. Then, if your opponent has no Digimon with digivolution cards,
//! return 1 of your opponent's Tamers to the bottom of the deck.
//!
//! **[Opponent's Turn]** None of your opponent's Digimon with as many or
//! fewer digivolution cards as this Digimon can suspend.
//!
//! No inherited effect.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Blue/EX7_023.cs
//!   - None: ChangeSelfSAttackStaticEffect(changeValue: 1, isInheritedEffect:
//!     false) + IcecladSelfStaticEffect(isInheritedEffect: false) — both
//!     face-up unconditional statics.
//!   - When Digivolving (OnEnterFieldAnyone): the trash half is gated on
//!     HasMatchConditionPermanent(opponent battle-area Digimon), then
//!     SelectTrashDigivolutionCards(maxCount: 4, canNoTrash: FALSE —
//!     MANDATORY, isFromOnly1Permanent: FALSE — cross-permanent). The helper
//!     caps at min(total selectable digivolution cards, 4). THEN,
//!     unconditionally after the trash loop: if
//!     !HasMatchConditionOpponentsPermanent(p => p.IsDigimon &&
//!     !p.HasNoDigivolutionCards) AND the opponent has >= 1 Tamer →
//!     MANDATORY SelectPermanentEffect(maxCount: min(1, count), canNoSelect:
//!     false, mode: PutLibraryBottom) over opponent Tamers. With no Tamer
//!     the select is skipped entirely (HasMatchConditionPermanent guard).
//!   - None (CanNotSuspendClass): CantSuspendCondition = opponent battle-area
//!     permanent && IsDigimon && permanent.DigivolutionCards.Count <=
//!     card.PermanentOfThisCard().DigivolutionCards.Count (a LIVE
//!     source-count compare against this card's own stack), CanUseCondition =
//!     self on battle area && IsOpponentTurn.
//!
//! # Faithfulness notes
//! - "Trash any 4" is cross-permanent and mandatory: exactly min(total
//!   available, 4) picks, no PASS (DCGO canNoTrash: false). Encoded as the
//!   EX7-021 / EX8-023 if/else availability ladder generalized to 4.
//! - The "Then" half runs unconditionally AFTER the trash (also when nothing
//!   could be trashed) and observes the post-trash board.
//! - The [Opponent's Turn] lock compares the opponent Digimon's digivolution
//!   card count against THIS Digimon's own LIVE count — "as many or fewer".
//!   Both DCGO `DigivolutionCards.Count` values exclude the top card, so the
//!   compare is sources(opp) <= sources(self) ⟺ stack(opp) <= stack(self).
//! - KNOWN ENGINE GAP (pre-existing, shared with BT25-026/028, EX9-019,
//!   BT20-084, EX8-023): `CannotSuspend` installs/expires correctly but has
//!   no engine consult site yet — attack/suspend actions are not blocked.
//!   Enforcement-dependent assertions are `#[ignore]`d below; the
//!   non-ignored aura tests assert the modifier lands on exactly the right
//!   Digimon set.
//!
//! # Patterns this test covers
//! - Face-up unconditional <Security A. +1> (BT10-013 self-aura idiom) and
//!   <Iceclad> (EX8-022 grant_keyword idiom).
//! - [When Digivolving] cross-permanent `select_opponent_sources` with the
//!   min(total, 4) clamp ladder (5-arm generalization of EX7-021/EX8-023).
//! - Post-trash `no_permanent` gate + mandatory opponent-Tamer select +
//!   `return_to_deck position: bottom` (BT17-077 return shape).
//! - Continuous opponent-turn aura installing a named modifier
//!   (CannotSuspend) on a DYNAMIC source-count-compare target filter
//!   (`stack_size_lte` formula over `source_stack_count`).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-023";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A blue Lv.5 Digimon — the digivolution base for EX7-023 (Lv.5 blue / cost 4).
fn blue_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "BlueUltimate");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(5);
    c.dp = Some(7000);
    c
}

/// Generic Digimon / stack filler.
fn junk(id: &str) -> CardData {
    let mut c = make_test_card(id, "Junkmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// Opponent Digimon with parameterized DP for the Iceclad battle test.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Opp Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

/// An opponent Tamer — target of the "Then" deck-bottom return.
fn opp_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Opp Tamer");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-023 YAML loads from the embedded pack")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

/// Builder pre-loaded with everything the When Digivolving tests need: the
/// blue Lv.5 base, deck filler, an opponent Tamer, and a pool of uniquely-id'd
/// source/top cards for arbitrary opponent stack layouts.
fn wd_builder() -> DebugRunnerBuilder {
    let mut b = base_builder()
        .add_card(blue_lv5("BASE"))
        .add_card(junk("FILL"))
        .add_card(opp_tamer("OPP-TAMER"));
    for i in 0..10 {
        b = b.add_card(junk(&format!("SRC-{i}")));
    }
    for i in 0..4 {
        b = b.add_card(junk(&format!("TOP-{i}")));
    }
    b.hand(0, &["EX7-023"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
}

/// Place opponent (player 1) Digimon stacks with the given SOURCE counts
/// (stack t gets `stacks[t]` digivolution cards under top card `TOP-t`).
fn place_opp_stacks(runner: &mut DebugRunner, stacks: &[usize]) {
    let mut s = 0;
    for (t, &n) in stacks.iter().enumerate() {
        let mut ids: Vec<String> = Vec::with_capacity(n + 1);
        for _ in 0..n {
            ids.push(format!("SRC-{s}"));
            s += 1;
        }
        ids.push(format!("TOP-{t}"));
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        runner.place_stack(1, &refs);
    }
}

/// Digivolve EX7-023 (hand index 0) onto the given base slot; the
/// [When Digivolving] trigger fires inside this call.
fn digivolve_ex7_023(runner: &mut DebugRunner, base: PermanentHandle) {
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-023 must digivolve from a blue Lv.5 base (cost 4)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX7-023",
        "EX7-023 is now the top card of the digivolved stack"
    );
}

/// Drive the mandatory min/max=N source pick to completion (always picking
/// the first live candidate; the pickers re-enumerate between picks).
fn pick_n_sources(runner: &mut DebugRunner, n: usize) {
    for i in 0..n {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("source pick {}/{} must be pending", i + 1, n));
        assert!(
            !view.valid_action_ids.contains(&PASS),
            "PASS must not be exposed before the mandatory minimum is met (pick {}/{})",
            i + 1,
            n
        );
        let action = view.valid_action_ids[0];
        runner
            .execute_action(0, action)
            .unwrap_or_else(|e| panic!("source pick {}/{} failed: {e:?}", i + 1, n));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_023_compiles_with_printed_stats_and_lv5_blue_path() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-023 in compiled cards");

    assert_eq!(compiled.card, "EX7-023");
    assert_eq!(compiled.name, "Hexeblaumon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for t in ["Magic Knight", "Witchelny"] {
        assert!(
            compiled.traits.iter().any(|x| x == t),
            "printed type [{t}] missing"
        );
    }
    assert!(
        compiled.traits.iter().any(|t| t == "Ice-Snow"),
        "rule box 'Trait: Has [Ice-Snow] Type.' must surface as an Ice-Snow trait"
    );

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5) || from.all_of.iter().any(|p| p.level_eq == Some(5)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|p| p.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Hexeblaumon must digivolve from a blue Lv.5 for cost 4"
    );
}

/// Both printed face-up keywords are encoded: <Iceclad> as a face-up
/// grant_keyword clause and <Security A. +1> as an unconditional face-up
/// self-aura with security_attack: 1.
#[test]
fn ex7_023_face_up_keyword_clause_shapes() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-023 in compiled cards");

    let has_iceclad_clause = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Iceclad" && *scope != CompiledScope::Inherited
        )
    });
    assert!(
        has_iceclad_clause,
        "EX7-023 must encode <Iceclad> as a face-up grant_keyword clause"
    );

    let sa_aura = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            active_when,
            security_attack: Some(sa),
            modifier: None,
            ..
        }) if *scope == CompiledScope::FaceUp => Some((active_when, *sa)),
        _ => None,
    });
    let (active_when, sa) = sa_aura
        .expect("EX7-023 must encode <Security A. +1> as a face-up self-aura with security_attack");
    assert_eq!(sa, 1, "the aura grants a flat <Security A. +1>");
    assert!(
        active_when.is_none(),
        "the printed <Security A. +1> is unconditional — no active_when gate"
    );
}

/// The [When Digivolving] clause is face-up, mandatory, not once-per-turn,
/// and NOT gated at clause level (the "Then" half must run even when the
/// opponent has no Digimon at all — DCGO's CanActivateCondition only checks
/// self-on-field).
#[test]
fn ex7_023_wd_clause_shape_mandatory_and_ungated() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-023 in compiled cards");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "EX7-023 has exactly one triggered clause (When Digivolving)"
    );

    let wd = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenDigivolving])
        .expect("When Digivolving clause exists");
    assert_eq!(wd.scope, CompiledScope::FaceUp);
    assert!(
        !wd.optional,
        "the trash/return has no 'you may' — mandatory (DCGO canNoTrash=false, canNoSelect=false)"
    );
    assert!(!wd.once_per_turn);
    assert!(
        wd.condition.is_none(),
        "the When Digivolving clause must NOT be gated at clause level: the \
         'Then' half observes the post-trash board and runs even with no \
         trashable sources"
    );
}

/// The [Opponent's Turn] lock is a face-up declarative aura: active_when
/// present (opponents_turn), named modifier CannotSuspend, and a non-empty
/// target filter (opponent Digimon with stack size <= this stack's size).
#[test]
fn ex7_023_opp_turn_aura_clause_shape() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-023 in compiled cards");

    let aura = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            active_when,
            target,
            modifier: Some(modifier),
            ..
        }) if *scope == CompiledScope::FaceUp => Some((active_when, target, modifier)),
        _ => None,
    });
    let (active_when, target, modifier) = aura.expect(
        "EX7-023 must encode the [Opponent's Turn] suspend lock as a face-up \
         declarative aura with a named modifier",
    );
    assert_eq!(
        modifier, "CannotSuspend",
        "the aura installs CannotSuspend on matching opponent Digimon"
    );
    assert!(
        active_when.is_some(),
        "the aura must carry the [Opponent's Turn] active_when gate"
    );
    assert_ne!(
        *target,
        Default::default(),
        "the aura targets opponent Digimon (non-empty target filter)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Face-up keywords live: Security A. +1 + Iceclad
// ─────────────────────────────────────────────────────────────────────────────

/// The placed Hexeblaumon answers has_keyword(Iceclad) and, after a
/// declarative tick, carries SecurityAttackChange +1 — total strike 2.
#[test]
fn ex7_023_face_up_keywords_live_on_placed_permanent() {
    let mut runner = base_builder().start();

    let hexe = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(hexe, Keyword::Iceclad),
        "the placed Hexeblaumon must answer has_keyword(Iceclad)"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(hexe, ModifierType::SecurityAttackChange),
        1,
        "the face-up <Security A. +1> aura installs SecurityAttackChange +1"
    );
    assert_eq!(
        runner.game.effective_security_strike(hexe),
        2,
        "total security strike = base 1 + <Security A. +1> = 2"
    );
}

/// <Security A. +1> in the actual security-check count: attacking the player
/// consumes TWO security cards instead of one.
#[test]
fn ex7_023_security_attack_plus_1_checks_two_security_cards() {
    let mut runner = base_builder()
        .add_card(junk("FILL"))
        .security(1, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .start();

    let hexe = runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(runner.security_count(1), 4);

    let result = runner.attack_player(hexe, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "12000-DP Hexeblaumon survives two 4000-DP security Digimon checks"
    );
    assert_eq!(
        runner.security_count(1),
        2,
        "<Security A. +1>: the attack must check (and consume) 2 security cards"
    );
}

/// <Iceclad>: bare Hexeblaumon (12000 DP, 1-card stack) attacks a 1000-DP
/// defender with 2 digivolution cards (3-card stack) — the stack-size compare
/// (1 < 3) deletes Hexeblaumon despite its vastly higher DP.
#[test]
fn ex7_023_iceclad_loses_with_fewer_sources_despite_higher_dp() {
    let mut runner = base_builder()
        .add_card(junk("SRC-A"))
        .add_card(junk("SRC-B"))
        .add_card(opp_digimon("WEAK-DEF", 1000))
        .start();

    let hexe = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_stack(1, &["SRC-A", "SRC-B", "WEAK-DEF"]);

    let result = runner.attack_digimon(hexe, defender, false);

    assert_eq!(
        result,
        AttackResult::DefenderWins,
        "Iceclad compares stack sizes (1 < 3) — Hexeblaumon loses despite 12000 vs 1000 DP"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[1].battle_area.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [When Digivolving] trash any 4 + conditional Tamer bottom-deck
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (full 4 + Tamer return): the opponent has two Digimon with 2
/// digivolution cards each (total 4) and a Tamer. The player must pick
/// exactly 4 sources cross-permanent (no PASS); both stacks are stripped
/// bare, the opponent is left with no sourced Digimon, so the mandatory
/// Tamer select fires and the Tamer goes to the BOTTOM of its owner's deck.
#[test]
fn ex7_023_wd_trashes_four_across_permanents_then_bottoms_tamer() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    place_opp_stacks(&mut runner, &[2, 2]);
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    let deck_before = runner.deck_size(1);

    digivolve_ex7_023(&mut runner, base);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 4,
            max: 4,
            picked: 0
        }),
        "a mandatory exactly-4 cross-permanent source selection must install"
    );
    assert!(
        !runner.pending_is_optional(),
        "the trash is not 'you may' — mandatory (DCGO canNoTrash=false)"
    );
    pick_n_sources(&mut runner, 4);

    // All 4 sources trashed; both stacks stripped to bare tops.
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "stack A stripped 3 -> 1"
    );
    assert_eq!(
        runner.game.players[1].battle_area[1].card_sources.len(),
        1,
        "stack B stripped 3 -> 1"
    );
    assert_eq!(
        runner.trash_size(1),
        4,
        "all 4 trashed sources land in the opponent's trash"
    );

    // Then: opponent has no sourced Digimon → mandatory Tamer select.
    let view = runner
        .pending_selection_view()
        .expect("the 'Then' Tamer selection must install after the trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "the Tamer return is mandatory (DCGO canNoSelect=false)"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Tamer is a legal target (the sourceless Digimon are NOT)"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Tamer");
    runner.auto_resolve().expect("finish When Digivolving");

    // Tamer left the field and sits at the BOTTOM of its owner's deck.
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-TAMER"),
        "the Tamer must leave the battle area"
    );
    let _ = tamer;
    assert_eq!(
        runner.deck_size(1),
        deck_before + 1,
        "the Tamer returns to its owner's deck"
    );
    assert_eq!(
        runner.game.players[1].deck[0].card_id(&runner.game.card_data),
        "OPP-TAMER",
        "the Tamer sits at the BOTTOM of the deck (deck[0]; draws pop from the end)"
    );
}

/// Min-clamp ladder: for every partition of opponent source counts, the
/// installed selection must demand exactly min(total, 4) picks. This
/// exercises every availability-ladder branch (the >=4 / ==3 / ==2 / ==1
/// arms and the per-arm partition decompositions).
#[test]
fn ex7_023_wd_min_clamp_ladder_demands_min_of_total_and_four() {
    // (opponent stack source-counts, expected mandatory pick count)
    let configs: &[(&[usize], u8)] = &[
        // total >= 4 → exactly 4, across all partitions of 4.
        (&[4], 4),
        (&[5], 4),
        (&[3, 1], 4),
        (&[2, 2], 4),
        (&[2, 1, 1], 4),
        (&[1, 1, 1, 1], 4),
        // total == 3 → exactly 3, across all partitions of 3.
        (&[3], 3),
        (&[2, 1], 3),
        (&[1, 1, 1], 3),
        // total == 2 → exactly 2.
        (&[2], 2),
        (&[1, 1], 2),
        // total == 1 → exactly 1.
        (&[1], 1),
    ];

    for (stacks, expected) in configs {
        let mut runner = wd_builder().start();
        let base = runner.place_on_field(0, "BASE", Some(0));
        place_opp_stacks(&mut runner, stacks);

        digivolve_ex7_023(&mut runner, base);

        assert_eq!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti {
                min: *expected,
                max: *expected,
                picked: 0
            }),
            "opponent stacks {stacks:?}: expected a mandatory exactly-{expected} pick"
        );
        // Drive it to completion so the runner ends cleanly.
        pick_n_sources(&mut runner, *expected as usize);
        runner.auto_resolve().expect("finish When Digivolving");
        assert_eq!(
            runner.trash_size(1),
            *expected as usize,
            "opponent stacks {stacks:?}: exactly {expected} sources trashed"
        );
    }
}

/// Mid-ladder behavioral check: total 3 (2+1) trashes all 3, leaving the
/// opponent sourceless — the Tamer is then bottomed.
#[test]
fn ex7_023_wd_clamped_three_trashes_all_then_bottoms_tamer() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    place_opp_stacks(&mut runner, &[2, 1]);
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    digivolve_ex7_023(&mut runner, base);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 3,
            max: 3,
            picked: 0
        }),
        "total 3 available → mandatory exactly-3 pick"
    );
    pick_n_sources(&mut runner, 3);

    let view = runner
        .pending_selection_view()
        .expect("the Tamer selection must install after the clamped trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Tamer");
    runner.auto_resolve().expect("finish When Digivolving");

    assert_eq!(
        runner.game.players[1].deck[0].card_id(&runner.game.card_data),
        "OPP-TAMER",
        "the Tamer is bottomed after the clamped 3-trash empties all stacks"
    );
}

/// The "Then" half runs even when NOTHING could be trashed: a bare opponent
/// Digimon plus a Tamer → no source selection installs, but the Tamer select
/// fires immediately and the Tamer is bottomed.
#[test]
fn ex7_023_wd_zero_sources_skips_trash_but_then_half_runs() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(1, "TOP-0", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    digivolve_ex7_023(&mut runner, base);

    // No sources anywhere → the first pending selection must already be the
    // Tamer pick, not a source pick.
    let view = runner
        .pending_selection_view()
        .expect("the Tamer selection must install even with nothing to trash");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "no digivolution card exists — the only prompt is the Tamer pick"
    );
    assert_eq!(view.valid_action_ids.len(), 1, "only the Tamer is legal");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Tamer");
    runner.auto_resolve().expect("finish When Digivolving");

    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
    assert_eq!(
        runner.game.players[1].deck[0].card_id(&runner.game.card_data),
        "OPP-TAMER",
        "the Tamer is bottomed — DCGO runs the 'Then' check unconditionally"
    );
}

/// NEGATIVE (gate): a 5-source stack leaves 1 digivolution card behind after
/// the 4 trashes — a sourced opponent Digimon remains, so the Tamer must NOT
/// be selected or returned even though one is present.
#[test]
fn ex7_023_wd_no_tamer_return_while_sourced_digimon_remains() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    place_opp_stacks(&mut runner, &[5]);
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    let deck_before = runner.deck_size(1);

    digivolve_ex7_023(&mut runner, base);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 4,
            max: 4,
            picked: 0
        })
    );
    pick_n_sources(&mut runner, 4);
    assert!(
        runner.pending_selection().is_none(),
        "a sourced opponent Digimon remains (5 - 4 = 1) — no Tamer selection may install"
    );
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        2,
        "the stack keeps 1 digivolution card (6 cards - 4 trashed = 2)"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-TAMER"),
        "the Tamer must stay on the field"
    );
    assert_eq!(runner.deck_size(1), deck_before, "no card returns to deck");
}

/// NEGATIVE (no Tamer): opponent left sourceless but has no Tamer — the
/// select is skipped entirely (DCGO HasMatchConditionPermanent guard) and the
/// effect resolves cleanly.
#[test]
fn ex7_023_wd_no_tamer_select_when_opponent_has_no_tamers() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    place_opp_stacks(&mut runner, &[2, 2]);

    digivolve_ex7_023(&mut runner, base);

    pick_n_sources(&mut runner, 4);
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Tamer exists — the mandatory select must silently skip"
    );
    runner.auto_resolve().ok();

    assert_eq!(runner.trash_size(1), 4, "the trash half still resolved");
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        2,
        "both (now sourceless) Digimon stay on the field"
    );
}

/// Fully empty opponent board: no prompts at all, no crash — the effect
/// resolves vacuously.
#[test]
fn ex7_023_wd_resolves_with_empty_opponent_board() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));

    digivolve_ex7_023(&mut runner, base);

    assert!(
        runner.pending_selection().is_none(),
        "empty opponent board — no selection may install"
    );
    runner.auto_resolve().ok();
    assert!(runner.game.players[1].trash.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — [Opponent's Turn] suspend lock (modifier installation set)
// ─────────────────────────────────────────────────────────────────────────────

/// Build the aura board: EX7-023 belongs to the NON-turn player (so it is
/// the owner's opponent's turn) with `self_sources` digivolution cards;
/// returns (owner, opp, hexe handle).
fn aura_runner(self_sources: usize) -> (DebugRunner, u8, u8, PermanentHandle) {
    let mut b = base_builder().add_card(opp_tamer("OPP-TAMER"));
    for i in 0..6 {
        b = b.add_card(junk(&format!("SRC-{i}")));
    }
    for i in 0..4 {
        b = b.add_card(junk(&format!("TOP-{i}")));
    }
    let mut runner = b.start();
    let tp = runner.game.turn_player();
    let owner = 1 - tp;

    let mut ids: Vec<String> = (0..self_sources).map(|i| format!("SRC-{i}")).collect();
    ids.push(CARD_ID.to_string());
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let hexe = runner.place_stack(owner, &refs);

    (runner, owner, tp, hexe)
}

/// Place a Digimon stack for `player` with `n` sources (uses the SRC-3..5 /
/// TOP-t pools so it never collides with the owner stack).
fn place_aura_opp_stack(
    runner: &mut DebugRunner,
    player: u8,
    t: usize,
    n: usize,
) -> PermanentHandle {
    let mut ids: Vec<String> = (0..n).map(|i| format!("SRC-{}", 3 + i)).collect();
    ids.push(format!("TOP-{t}"));
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    runner.place_stack(player, &refs)
}

/// POSITIVE: on the opponent's turn, opponent Digimon with AS MANY (1 = 1)
/// or FEWER (0 < 1) digivolution cards than Hexeblaumon get CannotSuspend;
/// a Digimon with MORE (2 > 1) stays free, and an opponent Tamer is never
/// locked (the lock reads "Digimon").
#[test]
fn ex7_023_opp_turn_aura_locks_equal_and_fewer_sources_not_more() {
    let (mut runner, _owner, tp, hexe) = aura_runner(1);

    let opp_fewer = runner.place_on_field(tp, "TOP-0", Some(0));
    let opp_equal = place_aura_opp_stack(&mut runner, tp, 1, 1);
    let opp_more = place_aura_opp_stack(&mut runner, tp, 2, 2);
    let opp_tamer = runner.place_on_field(tp, "OPP-TAMER", Some(0));

    runner.game.tick_declarative_effects();

    let _ = hexe;
    assert!(
        runner
            .game
            .modifiers
            .has(opp_fewer, ModifierType::CannotSuspend),
        "0 sources < 1 — FEWER digivolution cards must be locked"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(opp_equal, ModifierType::CannotSuspend),
        "1 source == 1 — AS MANY digivolution cards must be locked"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_more, ModifierType::CannotSuspend),
        "2 sources > 1 — MORE digivolution cards must stay free"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_tamer, ModifierType::CannotSuspend),
        "the lock reads 'Digimon' — an opponent Tamer is never locked"
    );
}

/// NEGATIVE (timing): on the OWNER's turn the aura is off — no opponent
/// Digimon is locked even when the source-count compare matches.
#[test]
fn ex7_023_opp_turn_aura_inactive_on_owners_turn() {
    let mut b = base_builder();
    for i in 0..2 {
        b = b.add_card(junk(&format!("SRC-{i}")));
    }
    b = b.add_card(junk("TOP-0"));
    let mut runner = b.start();
    let tp = runner.game.turn_player();
    let non_tp = 1 - tp;

    // Hexeblaumon belongs to the TURN player → it is NOT the opponent's turn
    // from the owner's perspective.
    let _hexe = runner.place_stack(tp, &["SRC-0", CARD_ID]);
    let opp_digimon = runner.place_on_field(non_tp, "TOP-0", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has(opp_digimon, ModifierType::CannotSuspend),
        "[Opponent's Turn] gate: no lock may install on the owner's own turn"
    );
}

/// The compare is LIVE against Hexeblaumon's own stack: with ZERO sources
/// under it, only sourceless opponent Digimon (0 <= 0) are locked; a 1-source
/// Digimon (1 > 0) stays free.
#[test]
fn ex7_023_opp_turn_aura_compares_live_own_source_count() {
    let (mut runner, _owner, tp, _hexe) = aura_runner(0);

    let opp_bare = runner.place_on_field(tp, "TOP-0", Some(0));
    let opp_one_src = place_aura_opp_stack(&mut runner, tp, 1, 1);

    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .has(opp_bare, ModifierType::CannotSuspend),
        "0 sources <= 0 — the bare Digimon is locked"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_one_src, ModifierType::CannotSuspend),
        "1 source > 0 — the sourced Digimon must stay free (live self-count compare)"
    );
}

/// ENFORCEMENT (shared with BT25-026/028, EX9-019, BT20-084, EX8-023): a
/// CannotSuspend-locked Digimon must be unable to declare an attack
/// (general_rule.pdf 11-2-5; consult site in `Game::can_attack*`).
#[test]
fn ex7_023_locked_opponent_digimon_cannot_declare_attack() {
    let (mut runner, owner, tp, _hexe) = aura_runner(1);

    // A locked (sourceless) opponent Digimon on its controller's own turn.
    let locked = runner.place_on_field(tp, "TOP-0", Some(0));
    runner.game.tick_declarative_effects();
    assert!(runner
        .game
        .modifiers
        .has(locked, ModifierType::CannotSuspend));

    let security_before = runner.security_count(owner);
    let result = runner.attack_player(locked, owner, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        result,
        AttackResult::Invalid,
        "a CannotSuspend Digimon cannot suspend to declare an attack"
    );
    assert_eq!(
        runner.security_count(owner),
        security_before,
        "no security check may resolve from the blocked attack"
    );
    assert!(
        !runner.game.players[tp as usize].battle_area[locked.index as usize].is_suspended,
        "the locked Digimon must remain unsuspended"
    );
}
