//! ST20-05 Gatomon — Digimon, Lv.4, Yellow, DP 4000, Cost 4.
//! Traits: Holy Beast / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-05.md)
//!
//! ```text
//! [Security] At the end of the battle, play this card without paying the
//! cost.
//! [On Play] [When Digivolving] Give 2 of your opponent's Digimon
//! <Security A. -1> until their turn ends. (This Digimon checks 1 fewer
//! security card.)
//!
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//!            gets -2000 DP for the turn.
//!
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2       (xros_req box)
//! [evo_costs] Lv.3 Yellow: Cost 2
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Yellow/ST20_05.cs
//! - AddSelfDigivolutionRequirementStaticEffect: Lv.3 + HasAdventureTraits,
//!   cost 2, requirements respected.
//! - SecuritySkill: PlaySelfDigimonAfterBattleSecurityEffect.
//! - OnEnterFieldAnyone x2 (On Play / When Digivolving): maxCount =
//!   min(2, opponent Digimon), canNoSelect=false, canEndNotMax=false ->
//!   exactly min(2, N) distinct opponent Digimon, mandatory;
//!   ChangeDigimonSAttack(-1, UntilOpponentTurnEnd).
//! - OnAllyAttack (inherited, OPT): 1 opponent Digimon, canNoSelect=false,
//!   ChangeDigimonDP(-2000, UntilEachTurnEnd).
//!
//! # Patterns this test file covers
//!
//! | Clause                                     | Pattern                                             |
//! |--------------------------------------------|-----------------------------------------------------|
//! | Standard digivolve Lv.3 Yellow / cost 2    | alt_paths + route legality                          |
//! | xros_req Lv.3 ADVENTURE / cost 2           | alt_paths trait gate, positive + negatives          |
//! | [Security] play at end of battle           | on_security play_from_security (BT21-043 idiom)      |
//! | [On Play][When Digivolving] 2 x SA -1      | H4 Security A. -1, exact-2 distinct mandatory picks |
//! | Inherited [When Attacking][OPT] -2000 DP   | G4 inherited when_attacking OPT (P-134 idiom)       |

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const ID: &str = "ST20-05";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup_with_security(hand: &[&str], p1_security: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-05 found in embedded DSL pack");
    // (id, level, color, [ADVENTURE]?, dp)
    for (id, level, color, adventure, dp) in [
        ("YELLOW-LV3", 3, CardColor::Yellow, false, 3000),
        ("ADV-RED-LV3", 3, CardColor::Red, true, 3000),
        ("RED-LV3", 3, CardColor::Red, false, 3000),
        ("ADV-YELLOW-LV2", 2, CardColor::Yellow, true, 2000),
        ("CARRIER", 5, CardColor::Red, false, 6000),
        ("OPP-A", 4, CardColor::Blue, false, 5000),
        ("OPP-B", 4, CardColor::Blue, false, 6000),
        ("OPP-C", 4, CardColor::Blue, false, 7000),
        ("ATK-WEAK", 3, CardColor::Red, false, 1000),
        ("ATK-STRONG", 5, CardColor::Red, false, 9000),
        ("FILL", 3, CardColor::Red, false, 1000),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(level);
        c.colors = vec![color];
        c.dp = Some(dp);
        if adventure {
            c.traits = vec!["ADVENTURE".to_string()];
        }
        b = b.add_card(c);
    }
    let mut tamer = make_test_card("OPP-TAMER", "OPP-TAMER");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    b = b.add_card(tamer);

    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(1, p1_security)
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}

fn setup(hand: &[&str]) -> DebugRunner {
    setup_with_security(hand, &["FILL"; 5])
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

fn sa_change(r: &DebugRunner, h: PermanentHandle) -> i32 {
    r.game.modifiers.sum(h, ModifierType::SecurityAttackChange)
}

fn top_card_id(r: &DebugRunner, player: usize, index: usize) -> String {
    r.game.players[player].battle_area[index]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
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
fn st20_05_identity_matches_printed_card() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-05 compiled");
    assert_eq!(card.name, "Gatomon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    for t in ["Holy Beast", "ADVENTURE"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "missing trait {t}, got {:?}",
            card.traits
        );
    }
    assert_eq!(card.attribute, ["Vaccine"]);
}

#[test]
fn st20_05_declares_standard_and_xros_alt_paths() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-05 compiled");
    assert_eq!(card.alt_paths.len(), 2, "standard + xros_req");

    let standard = card
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.color_is == Some(CompiledColor::Yellow))
        })
        .expect("standard Lv.3 Yellow path");
    assert_eq!(standard.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(standard.cost, Some(CompiledCost::Literal(2)));
    assert_eq!(standard.from.as_ref().unwrap().level_eq, Some(3));

    let xros = card
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("ADVENTURE"))
        })
        .expect("xros_req Lv.3 ADVENTURE path");
    assert_eq!(xros.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(xros.cost, Some(CompiledCost::Literal(2)));
    assert_eq!(xros.from.as_ref().unwrap().level_eq, Some(3));
    assert!(
        !xros.ignore_requirements,
        "DCGO ignoreDigivolutionRequirement: false"
    );
}

#[test]
fn st20_05_clause_shapes() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-05 compiled");

    let security = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
        _ => None,
    });
    assert!(security.is_some(), "[Security] on_security clause");

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
    assert!(!op_wd.optional, "Security A. -1 grant is mandatory");
    assert!(!op_wd.once_per_turn);

    let inherited = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenAttacking) => {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited [When Attacking] clause");
    assert!(matches!(inherited.scope, CompiledScope::Inherited));
    assert!(inherited.once_per_turn, "[Once Per Turn]");
    assert!(!inherited.optional, "DCGO canNoSelect=false");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve routes (standard + xros_req)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_05_digivolve_routes_and_cost() {
    for (base, legal) in [
        ("YELLOW-LV3", true),      // standard Lv.3 Yellow circle
        ("ADV-RED-LV3", true),     // xros_req: Lv.3 w/[ADVENTURE], any colour
        ("RED-LV3", false),        // neither Yellow nor [ADVENTURE]
        ("ADV-YELLOW-LV2", false), // [ADVENTURE] but Lv.2
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
            // Consume the [When Digivolving] prompts (no opponent Digimon here,
            // so none are expected).
            assert!(r.pending_selection().is_none(), "{base}");
            assert_eq!(before - r.memory(), 2, "{base}: digivolution cost is 2");
            assert_eq!(top_card_id(&r, 0, h.index as usize), ID);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play][When Digivolving] Security A. -1 on 2 opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// Integrated [On Play]: 3 opponent Digimon -> two sequential mandatory picks
/// (the second excludes the first); exactly 2 of the 3 end up with SA -1.
#[test]
fn st20_05_on_play_gives_security_attack_minus_one_to_exactly_two_opponent_digimon() {
    let mut r = setup(&[ID]);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    let c = r.place_on_field(1, "OPP-C", Some(0));

    r.play(0, 0).expect("Gatomon plays from hand");

    let first = r.pending_selection_view().expect("first pick");
    assert_eq!(first.kind, SelectionKind::OppField);
    assert!(!first.is_optional, "mandatory: no PASS");
    assert_eq!(first.valid_action_ids.len(), 3, "all 3 Digimon are legal");
    let first_action = first.valid_action_ids[0];
    choose(&mut r, first_action);

    let second = r.pending_selection_view().expect("second pick");
    assert_eq!(second.kind, SelectionKind::OppField);
    assert!(!second.is_optional, "mandatory: no PASS");
    assert_eq!(
        second.valid_action_ids.len(),
        2,
        "the first pick is excluded from the second"
    );
    assert!(!second.valid_action_ids.contains(&first_action));
    choose(&mut r, second.valid_action_ids[0]);
    r.auto_resolve().expect("nothing else pending");
    assert!(
        r.pending_selection().is_none(),
        "exactly two picks, no third"
    );

    let values = [sa_change(&r, a), sa_change(&r, b), sa_change(&r, c)];
    assert_eq!(
        values.iter().filter(|&&v| v == -1).count(),
        2,
        "exactly two Digimon get Security A. -1, got {values:?}"
    );
    assert_eq!(
        values.iter().filter(|&&v| v == 0).count(),
        1,
        "the third is untouched, got {values:?}"
    );
}

/// Integrated [When Digivolving] through the real digivolve action; the
/// digivolution cost (2) is paid, then the same two picks surface.
#[test]
fn st20_05_when_digivolving_gives_security_attack_minus_one_to_two_opponent_digimon() {
    let mut r = setup(&[ID]);
    let h = r.place_on_field(0, "YELLOW-LV3", Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));

    let before = r.memory();
    let action = encode_digivolve(0, h.index as u16);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
    assert_eq!(before - r.memory(), 2, "digivolution cost is 2");
    assert_eq!(top_card_id(&r, 0, h.index as usize), ID);

    let first = r.pending_selection_view().expect("first pick");
    assert_eq!(first.kind, SelectionKind::OppField);
    choose(&mut r, first.valid_action_ids[0]);
    let second = r.pending_selection_view().expect("second pick");
    choose(&mut r, second.valid_action_ids[0]);
    r.auto_resolve().expect("nothing else pending");

    assert_eq!(sa_change(&r, a), -1);
    assert_eq!(sa_change(&r, b), -1);
}

/// Boundary: exactly 1 opponent Digimon -> a single pick (min(2, 1) = 1) and
/// it gets -1 once, not -2.
#[test]
fn st20_05_on_play_with_one_opponent_digimon_gives_it_security_attack_minus_one_once() {
    let mut r = setup(&[ID]);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    r.play(0, 0).expect("Gatomon plays from hand");

    let first = r.pending_selection_view().expect("single pick");
    assert_eq!(first.valid_action_ids.len(), 1);
    assert!(!first.is_optional);
    choose(&mut r, first.valid_action_ids[0]);
    r.auto_resolve().expect("nothing else pending");
    assert!(r.pending_selection().is_none(), "no second prompt");
    assert_eq!(sa_change(&r, a), -1, "-1 exactly once");
}

/// Negative: no opponent Digimon -> no prompt.
#[test]
fn st20_05_on_play_with_no_opponent_digimon_has_no_prompt() {
    let mut r = setup(&[ID]);
    r.play(0, 0).expect("Gatomon plays from hand");
    assert!(r.pending_selection().is_none());
}

/// Negative: only Digimon are legal targets — an opponent Tamer is never offered.
#[test]
fn st20_05_on_play_never_targets_opponent_tamers() {
    let mut r = setup(&[ID]);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    let t = r.place_on_field(1, "OPP-TAMER", Some(0));
    r.play(0, 0).expect("Gatomon plays from hand");

    let first = r.pending_selection_view().expect("first pick");
    assert_eq!(
        first.valid_action_ids.len(),
        2,
        "only the 2 Digimon are legal, not the Tamer"
    );
    choose(&mut r, first.valid_action_ids[0]);
    let second = r.pending_selection_view().expect("second pick");
    assert_eq!(second.valid_action_ids.len(), 1);
    choose(&mut r, second.valid_action_ids[0]);
    r.auto_resolve().expect("nothing else pending");

    assert_eq!(sa_change(&r, a), -1);
    assert_eq!(sa_change(&r, b), -1);
    assert_eq!(sa_change(&r, t), 0, "the Tamer is untouched");
}

/// Duration: "until their turn ends" — the modifier survives into the
/// opponent's turn and is gone once that turn ends.
#[test]
fn st20_05_security_attack_minus_one_lasts_until_the_opponents_turn_ends() {
    let mut r = setup(&[ID]);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    r.play(0, 0).expect("Gatomon plays from hand");
    let only = r
        .pending_selection_view()
        .expect("single pick")
        .valid_action_ids[0];
    choose(&mut r, only);
    r.auto_resolve().expect("nothing else pending");
    assert_eq!(sa_change(&r, a), -1);

    advance_to_turn_player(&mut r, 1);
    assert_eq!(
        sa_change(&r, a),
        -1,
        "still in effect throughout the opponent's turn"
    );

    advance_to_turn_player(&mut r, 0);
    assert_eq!(
        sa_change(&r, a),
        0,
        "expired when the opponent's turn ended"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] At the end of the battle, play this card for free
// ═══════════════════════════════════════════════════════════════════════════

/// Integrated: Gatomon (4000) in P1's security is checked by a weaker attacker,
/// survives the battle, and is played onto P1's field. Its [On Play] then
/// gives Security A. -1 to P0's remaining Digimon.
#[test]
fn st20_05_security_plays_after_battle_and_its_on_play_fires() {
    let mut r = setup_with_security(&[], &[ID]);
    let attacker = r.place_on_field(0, "ATK-WEAK", Some(0));
    let bystander = r.place_on_field(0, "OPP-A", Some(0));
    assert_eq!(r.security_count(1), 1);

    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(1), 0, "the security card was checked");
    assert_eq!(
        r.battle_area_size(1),
        1,
        "Gatomon was played onto P1's field"
    );
    assert_eq!(top_card_id(&r, 1, 0), ID);
    assert_eq!(
        sa_change(&r, bystander),
        -1,
        "the played Gatomon's [On Play] gave P0's remaining Digimon Security A. -1"
    );
}

/// Rule 14-2-3: "Security Digimon aren't deleted even when they lose a battle."
/// So even when the attacker is stronger, Gatomon survives the security battle
/// and its [Security] "At the end of the battle, play this card" still plays it
/// (DCGO `PlaySelfDigimonAfterBattleSecurityEffect` only requires that the card
/// is no longer in the library/security stack).
#[test]
fn st20_05_security_is_still_played_after_losing_the_battle() {
    let mut r = setup_with_security(&[], &[ID]);
    let attacker = r.place_on_field(0, "ATK-STRONG", Some(0));

    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(1), 0);
    assert_eq!(
        r.battle_area_size(1),
        1,
        "rule 14-2-3: a Security Digimon is not deleted by losing the battle, so it is still played"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [When Attacking][Once Per Turn] -2000 DP
// ═══════════════════════════════════════════════════════════════════════════

fn fire_when_attacking(r: &mut DebugRunner, carrier: PermanentHandle) {
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    r.game.drain_effect_queue();
}

/// Integrated: a carrier with ST20-05 beneath attacks -> the controller picks
/// 1 opponent Digimon, which gets -2000 DP for the turn.
#[test]
fn st20_05_inherited_when_attacking_gives_minus_2000_dp_for_the_turn() {
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    let other = r.place_on_field(1, "OPP-B", Some(0));

    fire_when_attacking(&mut r, carrier);
    let view = r.pending_selection_view().expect("opponent target pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional, "DCGO canNoSelect=false");
    assert_eq!(view.valid_action_ids.len(), 2);
    choose(&mut r, view.valid_action_ids[0]);
    r.auto_resolve().expect("finish DP modifier");

    let dps = [r.effective_dp(opp).unwrap(), r.effective_dp(other).unwrap()];
    assert!(
        dps == [3000, 6000] || dps == [5000, 4000],
        "exactly one opponent Digimon lost 2000 DP, got {dps:?}"
    );

    // "for the turn": restored once the turn ends.
    advance_to_turn_player(&mut r, 1);
    advance_to_turn_player(&mut r, 0);
    assert_eq!(r.effective_dp(opp), Some(5000));
    assert_eq!(r.effective_dp(other), Some(6000));
}

/// OPT: a second attack trigger in the same turn does nothing; the lockout
/// clears on the next turn.
#[test]
fn st20_05_inherited_when_attacking_is_once_per_turn() {
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "CARRIER"]);
    r.place_on_field(1, "OPP-C", Some(0));

    fire_when_attacking(&mut r, carrier);
    assert!(r.pending_selection().is_some(), "first trigger prompts");
    r.auto_resolve().expect("resolve first trigger");

    fire_when_attacking(&mut r, carrier);
    assert!(
        r.pending_selection().is_none(),
        "second trigger in the same turn is locked out"
    );

    advance_to_turn_player(&mut r, 1);
    advance_to_turn_player(&mut r, 0);
    fire_when_attacking(&mut r, carrier);
    assert!(
        r.pending_selection().is_some(),
        "the lockout clears on the next turn"
    );
}

/// Negative: the printed (face-up) Gatomon is not a carrier — the inherited
/// [When Attacking] only works from beneath another card.
#[test]
fn st20_05_inherited_when_attacking_does_not_fire_from_the_face_up_card() {
    let mut r = setup(&[]);
    let face_up = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "OPP-A", Some(0));

    fire_when_attacking(&mut r, face_up);
    assert!(
        r.pending_selection().is_none(),
        "inherited effect must not fire from the face-up card"
    );
}
