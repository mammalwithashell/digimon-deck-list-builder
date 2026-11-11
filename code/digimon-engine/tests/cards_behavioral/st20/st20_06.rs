//! ST20-06 Angewomon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Archangel / ADVENTURE. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-06.md)
//!
//! ```text
//! [On Play] [When Digivolving] 1 of your other Digimon may digivolve into a
//! Digimon card with the [ADVENTURE] trait in the hand without paying the cost.
//! [Your Turn] [Once Per Turn] When your other Digimon are played or
//! digivolve, if any of them have the [ADVENTURE] trait, 1 of your Digimon
//! gains <Alliance> for the turn. Then, 1 of your Digimon may attack.
//!
//! Inherited: <Alliance>
//!
//! [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3       (xros_req box)
//! [evo_costs] Lv.4 Yellow: Cost 3
//!
//! Official Q&A: the <Alliance> must be given to 1 of YOUR Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Yellow/ST20_06.cs
//! - AddSelfDigivolutionRequirementStaticEffect: Lv.4 + HasAdventureTraits,
//!   cost 3, requirements respected.
//! - OnEnterFieldAnyone x2 (On Play / When Digivolving): host = own battle-area
//!   permanent != this one with an [ADVENTURE] hand card that
//!   CanPlayCardTargetFrame; canNoSelect=true; DigivolveIntoHandOrTrashCard
//!   (payCost false, isOptional default true).
//! - OnEnterFieldAnyone "Your Turn" (OPT, hash alliance-attack-ST20_06):
//!   IsOwnerTurn, an own Digimon other than this one played / digivolved, and
//!   (IsSkippable otherwise) the permanent has the ADVENTURE trait. Mandatory
//!   own-Digimon pick -> GainAlliance(UntilEachTurnEnd); then optional pick of
//!   an own Digimon that CanAttack -> SelectAttackEffect.
//! - OnAllyAttack AllianceSelfEffect(isInheritedEffect: true).
//!
//! # Patterns this test file covers
//!
//! | Clause                                         | Pattern                                           |
//! |------------------------------------------------|---------------------------------------------------|
//! | Standard digivolve Lv.4 Yellow / cost 3        | alt_paths + route legality                        |
//! | xros_req Lv.4 ADVENTURE / cost 3               | alt_paths trait gate, positive + negatives        |
//! | [OP][WD] other Digimon digivolves free         | E-optional host pick + hand pick, `other`, routes |
//! | [Your Turn][OPT] Alliance grant then attack    | F-observer (BT16-017), OPT, trigger gates, expiry |
//! | Inherited <Alliance>                           | H10 inherited keyword (stack-walk)                |
//!
//! # Interplay worth remembering
//! The free digivolve of the first clause is itself an "other Digimon
//! digivolves" event with an [ADVENTURE] result, so the second (OPT) clause
//! fires right after it; the tests below consume that follow-up explicitly.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "ST20-06";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-06 found in embedded DSL pack");
    // (id, level, color, [ADVENTURE]?, dp, evo row)
    let cards: [(&str, u8, CardColor, bool, i32, Option<(CardColor, u8, u16)>); 12] = [
        ("YELLOW-LV4", 4, CardColor::Yellow, false, 4000, None),
        ("ADV-RED-LV4", 4, CardColor::Red, true, 4000, None),
        ("RED-LV4", 4, CardColor::Red, false, 4000, None),
        ("ADV-YELLOW-LV3", 3, CardColor::Yellow, true, 3000, None),
        ("HOST-LV3", 3, CardColor::Red, false, 3000, None),
        // Hand-side digivolution results.
        (
            "ADV-EVO",
            4,
            CardColor::Red,
            true,
            5000,
            Some((CardColor::Red, 3, 2)),
        ),
        (
            "PLAIN-EVO",
            4,
            CardColor::Red,
            false,
            5000,
            Some((CardColor::Red, 3, 2)),
        ),
        (
            "ADV-BLUE-EVO",
            4,
            CardColor::Blue,
            true,
            5000,
            Some((CardColor::Blue, 3, 2)),
        ),
        (
            "ADV-LV6",
            6,
            CardColor::Yellow,
            true,
            9000,
            Some((CardColor::Yellow, 5, 3)),
        ),
        // Cards that get played / digivolved to trigger the [Your Turn] clause.
        ("ADV-ROOKIE", 3, CardColor::Red, true, 3000, None),
        ("PLAIN-ROOKIE", 3, CardColor::Red, false, 3000, None),
        ("FILL", 3, CardColor::Red, false, 1000, None),
    ];
    for (id, level, color, adventure, dp, evo) in cards {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(level);
        c.colors = vec![color];
        c.dp = Some(dp);
        if adventure {
            c.traits = vec!["ADVENTURE".to_string()];
        }
        if let Some((evo_color, from_level, cost)) = evo {
            c.evo_costs = vec![EvoCost {
                card_color: evo_color as u8,
                level: from_level,
                memory_cost: cost,
            }];
        }
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(1, &["FILL"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}

fn choose(r: &mut DebugRunner, action: u16) {
    let view = r.pending_selection_view().expect("a pending selection");
    assert!(
        (action == PASS && view.is_optional) || view.valid_action_ids.contains(&action),
        "{view:?} does not offer {action}"
    );
    r.execute_action(view.selecting_player, action)
        .expect("selection resolves");
}

/// Settle every remaining prompt: mandatory picks take the first choice,
/// optional picks (the attack pick / attack confirm) are declined.
fn settle(r: &mut DebugRunner) {
    for _ in 0..16 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        choose(r, action);
    }
    panic!("prompts did not settle");
}

fn top_card_id(r: &DebugRunner, player: usize, index: usize) -> String {
    r.game.players[player].battle_area[index]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn alliance_holders(r: &DebugRunner) -> Vec<usize> {
    (0..r.game.players[0].battle_area.len())
        .filter(|&i| r.game.has_keyword(r.perm_handle(0, i), Keyword::Alliance))
        .collect()
}

/// Rotate the turn player to `target`, flushing the end-of-turn action window
/// so end-of-turn modifier expiry fires (BT21-043 / AD1-016 idiom).
fn advance_to_turn_player(runner: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if runner.turn_player() == target {
            return;
        }
        let _ = runner.auto_resolve();
        runner.pass_turn();
        let _ = runner.auto_resolve();
        if runner.game.current_phase == GamePhase::EndOfTurnAction {
            runner.game.pass_end_of_turn_action();
            let _ = runner.auto_resolve();
        }
    }
    assert_eq!(runner.turn_player(), target, "failed to rotate turn player");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_06_identity_matches_printed_card() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-06 compiled");
    assert_eq!(card.name, "Angewomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    for t in ["Archangel", "ADVENTURE"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "missing trait {t}, got {:?}",
            card.traits
        );
    }
    assert_eq!(card.attribute, ["Vaccine"]);
}

#[test]
fn st20_06_declares_standard_and_xros_alt_paths() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-06 compiled");
    assert_eq!(card.alt_paths.len(), 2, "standard + xros_req");

    let standard = card
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.color_is == Some(CompiledColor::Yellow))
        })
        .expect("standard Lv.4 Yellow path");
    assert_eq!(standard.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(standard.cost, Some(CompiledCost::Literal(3)));
    assert_eq!(standard.from.as_ref().unwrap().level_eq, Some(4));

    let xros = card
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("ADVENTURE"))
        })
        .expect("xros_req Lv.4 ADVENTURE path");
    assert_eq!(xros.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(xros.cost, Some(CompiledCost::Literal(3)));
    assert_eq!(xros.from.as_ref().unwrap().level_eq, Some(4));
    assert!(
        !xros.ignore_requirements,
        "DCGO ignoreDigivolutionRequirement: false"
    );
}

#[test]
fn st20_06_clause_shapes() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-06 compiled");

    let op_wd = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[On Play][When Digivolving] clause");
    assert!(!op_wd.once_per_turn);

    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnAllyPlayed)
                    && t.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[Your Turn] played / digivolve observer");
    assert!(observer.once_per_turn, "[Once Per Turn]");
    assert!(!observer.optional, "the Alliance grant is mandatory");

    let inherited_alliance = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Alliance")
        )
    });
    assert!(inherited_alliance, "inherited Alliance grant");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve routes (standard + xros_req)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_06_digivolve_routes_and_cost() {
    for (base, legal) in [
        ("YELLOW-LV4", true),      // standard Lv.4 Yellow circle
        ("ADV-RED-LV4", true),     // xros_req: Lv.4 w/[ADVENTURE], any colour
        ("RED-LV4", false),        // neither Yellow nor [ADVENTURE]
        ("ADV-YELLOW-LV3", false), // [ADVENTURE] but Lv.3
    ] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, base, Some(0));
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.5,
            legal,
            "{base}"
        );
        if legal {
            let before = r.memory();
            r.game.decode_action(action, 0);
            assert!(r.pending_selection().is_none(), "{base}: no other Digimon");
            assert_eq!(before - r.memory(), 3, "{base}: digivolution cost is 3");
            assert_eq!(top_card_id(&r, 0, h.index as usize), ID);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play][When Digivolving] other Digimon digivolves for free
// ═══════════════════════════════════════════════════════════════════════════

/// Integrated [On Play]: pick the host (Angewomon herself is not offered),
/// pick the [ADVENTURE] card (the plain one is not offered), digivolve free.
/// The digivolved [ADVENTURE] Digimon then triggers the [Your Turn] clause.
#[test]
fn st20_06_on_play_other_digimon_digivolves_into_adventure_card_for_free() {
    let mut r = setup(&[ID, "ADV-EVO", "PLAIN-EVO"]);
    let host = r.place_on_field(0, "HOST-LV3", Some(0));

    r.play(0, 0).expect("Angewomon plays from hand");
    assert_eq!(r.memory(), 3, "Angewomon costs 7");

    let host_pick = r.pending_selection_view().expect("host pick");
    assert_eq!(host_pick.kind, SelectionKind::OwnField);
    assert!(host_pick.is_optional, "'may' exposes PASS");
    assert_eq!(
        host_pick
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1,
        "only HOST-LV3 is a legal host (Angewomon is not an 'other' Digimon)"
    );
    choose(&mut r, host_pick.valid_action_ids[0]);

    let hand_pick = r.pending_selection_view().expect("hand pick");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    assert!(hand_pick.is_optional);
    assert_eq!(
        hand_pick
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1,
        "only the [ADVENTURE] card is legal (PLAIN-EVO lacks the trait)"
    );
    choose(&mut r, hand_pick.valid_action_ids[0]);

    // The free digivolve made an [ADVENTURE] Digimon digivolve on our turn,
    // so the second clause fires: mandatory Alliance pick among OUR Digimon.
    let alliance_pick = r.pending_selection_view().expect("follow-up Alliance pick");
    assert_eq!(alliance_pick.kind, SelectionKind::OwnField);
    assert!(!alliance_pick.is_optional);
    settle(&mut r);

    assert_eq!(top_card_id(&r, 0, host.index as usize), "ADV-EVO");
    assert_eq!(r.memory(), 3, "the digivolution was free");
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "PLAIN-EVO"),
        "the unchosen card stays in hand"
    );
}

/// Integrated [When Digivolving]: Angewomon digivolves from a Lv.4 and the
/// same pair of picks is offered for ANOTHER Digimon.
#[test]
fn st20_06_when_digivolving_other_digimon_digivolves_into_adventure_card_for_free() {
    let mut r = setup(&[ID, "ADV-EVO"]);
    let base = r.place_on_field(0, "YELLOW-LV4", Some(0));
    let host = r.place_on_field(0, "HOST-LV3", Some(0));

    let before = r.memory();
    let action = encode_digivolve(0, base.index as u16);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
    assert_eq!(before - r.memory(), 3, "digivolution cost is 3");
    assert_eq!(top_card_id(&r, 0, base.index as usize), ID);

    let host_pick = r.pending_selection_view().expect("host pick");
    assert_eq!(host_pick.kind, SelectionKind::OwnField);
    assert_eq!(
        host_pick
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1,
        "the freshly digivolved Angewomon is excluded; only HOST-LV3 is offered"
    );
    choose(&mut r, host_pick.valid_action_ids[0]);
    let hand_pick = r.pending_selection_view().expect("hand pick");
    choose(&mut r, hand_pick.valid_action_ids[0]);
    settle(&mut r);

    assert_eq!(top_card_id(&r, 0, host.index as usize), "ADV-EVO");
    assert_eq!(r.memory(), before - 3, "the second digivolution was free");
}

/// Decline at the host pick: nothing changes, no hand prompt.
#[test]
fn st20_06_declining_the_host_pick_digivolves_nothing() {
    let mut r = setup(&[ID, "ADV-EVO"]);
    let host = r.place_on_field(0, "HOST-LV3", Some(0));
    r.play(0, 0).expect("Angewomon plays from hand");

    let host_pick = r.pending_selection_view().expect("host pick");
    assert!(host_pick.is_optional);
    choose(&mut r, PASS);
    assert!(
        r.pending_selection().is_none(),
        "no hand prompt after declining"
    );
    assert_eq!(top_card_id(&r, 0, host.index as usize), "HOST-LV3");
    assert_eq!(r.hand_size(0), 1, "ADV-EVO stays in hand");
}

/// Decline at the hand pick: the host stays as it was.
#[test]
fn st20_06_declining_the_hand_pick_digivolves_nothing() {
    let mut r = setup(&[ID, "ADV-EVO"]);
    let host = r.place_on_field(0, "HOST-LV3", Some(0));
    r.play(0, 0).expect("Angewomon plays from hand");

    let host_pick = r.pending_selection_view().expect("host pick");
    choose(&mut r, host_pick.valid_action_ids[0]);
    let hand_pick = r.pending_selection_view().expect("hand pick");
    assert!(hand_pick.is_optional);
    choose(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(top_card_id(&r, 0, host.index as usize), "HOST-LV3");
    assert_eq!(r.hand_size(0), 1, "ADV-EVO stays in hand");
}

/// Negative: Angewomon herself is never the host, even when she is the only
/// legal base for the [ADVENTURE] hand card (ADV-LV6 digivolves from Lv.5 Yellow).
#[test]
fn st20_06_other_digimon_excludes_angewomon_herself() {
    let mut r = setup(&[ID, "ADV-LV6"]);
    r.play(0, 0).expect("Angewomon plays from hand");
    assert!(
        r.pending_selection().is_none(),
        "no 'other' Digimon: no host prompt"
    );
    assert_eq!(r.hand_size(0), 1, "ADV-LV6 stays in hand");
    assert_eq!(top_card_id(&r, 0, 0), ID);
}

/// Negative: the [ADVENTURE] hand card has no digivolution route onto the
/// only other Digimon -> that Digimon is not a legal host.
#[test]
fn st20_06_host_needs_a_legal_route_for_the_adventure_card() {
    let mut r = setup(&[ID, "ADV-BLUE-EVO"]);
    r.place_on_field(0, "HOST-LV3", Some(0)); // red Lv.3; ADV-BLUE-EVO needs blue Lv.3
    r.play(0, 0).expect("Angewomon plays from hand");
    assert!(
        r.pending_selection().is_none(),
        "no legal route: no host prompt"
    );
}

/// Negative: a non-[ADVENTURE] card in hand does not enable the effect.
#[test]
fn st20_06_requires_an_adventure_card_in_hand() {
    let mut r = setup(&[ID, "PLAIN-EVO"]);
    r.place_on_field(0, "HOST-LV3", Some(0));
    r.play(0, 0).expect("Angewomon plays from hand");
    assert!(r.pending_selection().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Your Turn][OPT] Alliance for the turn, then a Digimon may attack
// ═══════════════════════════════════════════════════════════════════════════

/// Integrated: another [ADVENTURE] Digimon is played on our turn -> mandatory
/// Alliance pick among our Digimon (all 3, never the opponent's), then an
/// optional attack pick limited to Digimon that can attack.
#[test]
fn st20_06_adventure_digimon_played_grants_alliance_to_one_digimon() {
    let mut r = setup(&["ADV-ROOKIE"]);
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "HOST-LV3", Some(0));
    r.play(0, 0).expect("ADV-ROOKIE plays from hand");

    let alliance_pick = r.pending_selection_view().expect("Alliance pick");
    assert_eq!(alliance_pick.kind, SelectionKind::OwnField);
    assert!(
        !alliance_pick.is_optional,
        "mandatory (DCGO canNoSelect=false)"
    );
    assert_eq!(
        alliance_pick.valid_action_ids.len(),
        3,
        "Angewomon, HOST-LV3 and the new ADV-ROOKIE are all legal"
    );
    assert!(alliance_holders(&r).is_empty());
    choose(&mut r, *alliance_pick.valid_action_ids.last().unwrap());

    assert_eq!(
        alliance_holders(&r).len(),
        1,
        "exactly one Digimon gained <Alliance>"
    );

    // Then: optional attack pick; the summoning-sick newcomer is not offered.
    let attack_pick = r.pending_selection_view().expect("attack pick");
    assert_eq!(attack_pick.kind, SelectionKind::OwnField);
    assert!(attack_pick.is_optional, "'may attack' exposes PASS");
    assert_eq!(
        attack_pick
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        2,
        "only Angewomon and HOST-LV3 can attack (ADV-ROOKIE was just played)"
    );
    choose(&mut r, PASS);
    assert!(r.pending_selection().is_none());
}

/// "for the turn": the granted Alliance is gone after the turn ends.
#[test]
fn st20_06_granted_alliance_expires_at_end_of_turn() {
    let mut r = setup(&["ADV-ROOKIE"]);
    r.place_on_field(0, ID, Some(0));
    r.play(0, 0).expect("ADV-ROOKIE plays from hand");
    let pick = r.pending_selection_view().expect("Alliance pick");
    choose(&mut r, pick.valid_action_ids[0]);
    settle(&mut r);
    assert_eq!(alliance_holders(&r).len(), 1);

    advance_to_turn_player(&mut r, 1);
    advance_to_turn_player(&mut r, 0);
    assert!(
        alliance_holders(&r).is_empty(),
        "<Alliance> lasted only for the turn"
    );
}

/// "Then, 1 of your Digimon may attack": pick an attacker and the attack
/// proceeds against the opponent's security.
#[test]
fn st20_06_then_one_digimon_may_attack() {
    let mut r = setup(&["ADV-ROOKIE"]);
    r.place_on_field(0, ID, Some(0)); // index 0
    r.place_on_field(0, "HOST-LV3", Some(0)); // index 1
    r.play(0, 0).expect("ADV-ROOKIE plays from hand"); // index 2

    // Give <Alliance> to the newcomer (last), so the attacker we pick next
    // (Angewomon, first) attacks without an Alliance prompt / extra check.
    let pick = r.pending_selection_view().expect("Alliance pick");
    choose(&mut r, *pick.valid_action_ids.last().unwrap());

    let attack_pick = r.pending_selection_view().expect("attack pick");
    assert!(attack_pick.is_optional);
    assert_eq!(r.security_count(1), 5);
    choose(&mut r, attack_pick.valid_action_ids[0]);

    // The chosen Digimon now selects its attack target (optional confirm).
    let target = r.pending_selection_view().expect("attack target pick");
    let action = *target
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a legal attack target");
    choose(&mut r, action);
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        4,
        "the attack hit the opponent's security: 1 check"
    );
}

/// Negative: a non-[ADVENTURE] Digimon being played does not trigger it.
#[test]
fn st20_06_non_adventure_digimon_played_does_not_trigger() {
    let mut r = setup(&["PLAIN-ROOKIE"]);
    r.place_on_field(0, ID, Some(0));
    r.play(0, 0).expect("PLAIN-ROOKIE plays from hand");
    assert!(r.pending_selection().is_none());
    assert!(alliance_holders(&r).is_empty());
}

/// Negative: "your other Digimon" — Angewomon herself being played (she has
/// the ADVENTURE trait) does not trigger her own clause.
#[test]
fn st20_06_angewomon_herself_being_played_does_not_trigger_the_observer() {
    let mut r = setup(&[ID]);
    r.play(0, 0).expect("Angewomon plays from hand");
    assert!(r.pending_selection().is_none());
    assert!(alliance_holders(&r).is_empty());
}

/// Negative: an OPPONENT's [ADVENTURE] Digimon entering play does not trigger it.
#[test]
fn st20_06_opponents_adventure_digimon_does_not_trigger() {
    let mut r = setup(&[]);
    r.place_on_field(0, ID, Some(0));
    let opp = r.place_on_field(1, "ADV-ROOKIE", None);
    r.fire_play_event_triggers(1, opp.index as usize, true, false);
    let _ = r.auto_resolve();
    assert!(alliance_holders(&r).is_empty());
    assert!(r.pending_selection().is_none());
}

/// Negative: [Your Turn] only.
#[test]
fn st20_06_does_not_trigger_on_the_opponents_turn() {
    let mut r = setup(&[]);
    r.place_on_field(0, ID, Some(0));
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    let field = r.place_on_field(0, "ADV-ROOKIE", None);
    r.fire_play_event_triggers(0, field.index as usize, true, false);
    assert!(
        r.pending_selection().is_none(),
        "[Your Turn] clause must be inactive on the opponent's turn"
    );
    assert!(alliance_holders(&r).is_empty());
}

/// Digivolve half of the observer: an [ADVENTURE] result digivolving onto our
/// Digimon triggers it; a non-[ADVENTURE] result does not.
#[test]
fn st20_06_adventure_digimon_digivolving_triggers_but_plain_does_not() {
    for (result, triggers) in [("ADV-EVO", true), ("PLAIN-EVO", false)] {
        let mut r = setup(&[result]);
        r.place_on_field(0, ID, Some(0));
        let base = r.place_on_field(0, "HOST-LV3", Some(0));
        let action = encode_digivolve(0, base.index as u16);
        assert!(
            build_action_mask(&r.game, 0)[action as usize] > 0.5,
            "{result}"
        );
        r.game.decode_action(action, 0);

        if triggers {
            let pick = r.pending_selection_view().expect("Alliance pick");
            assert_eq!(pick.kind, SelectionKind::OwnField, "{result}");
            assert!(!pick.is_optional, "{result}");
            settle(&mut r);
            assert_eq!(alliance_holders(&r).len(), 1, "{result}");
        } else {
            assert!(r.pending_selection().is_none(), "{result}");
            assert!(alliance_holders(&r).is_empty(), "{result}");
        }
    }
}

/// OPT: a second qualifying event in the same turn is locked out; the lockout
/// clears on the next turn.
#[test]
fn st20_06_observer_is_once_per_turn_and_resets() {
    let mut r = setup(&["ADV-ROOKIE", "ADV-ROOKIE"]);
    r.place_on_field(0, ID, Some(0));

    r.play(0, 0).expect("first ADV-ROOKIE");
    assert!(r.pending_selection().is_some(), "first event triggers");
    settle(&mut r);

    r.play(0, 0).expect("second ADV-ROOKIE");
    assert!(
        r.pending_selection().is_none(),
        "second qualifying event in the same turn is locked out"
    );

    advance_to_turn_player(&mut r, 1);
    advance_to_turn_player(&mut r, 0);
    let field = r.place_on_field(0, "ADV-ROOKIE", None);
    r.fire_play_event_triggers(0, field.index as usize, false, false);
    assert!(
        r.pending_selection().is_some(),
        "the lockout clears on the next turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited <Alliance>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_06_inherited_alliance_grants_to_carrier_only() {
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "HOST-LV3"]);
    assert!(
        r.game.has_keyword(carrier, Keyword::Alliance),
        "HOST-LV3 with ST20-06 beneath must have <Alliance>"
    );

    let plain = r.place_on_field(0, "HOST-LV3", Some(0));
    assert!(
        !r.game.has_keyword(plain, Keyword::Alliance),
        "a carrier without ST20-06 beneath must NOT have <Alliance>"
    );

    let face_up = r.place_on_field(0, ID, Some(0));
    assert!(
        !r.game.has_keyword(face_up, Keyword::Alliance),
        "the printed <Alliance> is inherited text and does not apply to the face-up card"
    );
}
