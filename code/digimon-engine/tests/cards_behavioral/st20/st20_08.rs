//! ST20-08 Kabuterimon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Insectoid / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-08.md)
//!
//! ```text
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer
//! card with the [ADVENTURE] trait from your hand without paying the cost.
//!
//! Inherited: <Piercing> (When this Digimon attacks and deletes your
//!            opponent's Digimon in battle, it checks security before the
//!            attack ends.)
//!
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2       (xros_req box)
//! [evo_costs] Lv.3 Green: Cost 2
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Green/ST20_08.cs
//! - EffectTiming.None AddSelfDigivolutionRequirementStaticEffect: Lv.3 +
//!   HasAdventureTraits, cost 2, requirements respected.
//! - OnEnterFieldAnyone (When Digivolving), isOptional=false: own Tamer count
//!   <= 1 and a playable [ADVENTURE] Tamer in hand; SelectHandEffect
//!   canNoSelect=true then PlayPermanentCards(payCost false).
//! - OnDetermineDoSecurityCheck PierceSelfEffect(isInheritedEffect: true).
//!
//! # Patterns this test file covers
//!
//! | Clause                                      | Pattern                                               |
//! |---------------------------------------------|-------------------------------------------------------|
//! | Standard digivolve Lv.3 Green / cost 2      | alt_paths + route legality                            |
//! | xros_req Lv.3 ADVENTURE / cost 2            | alt_paths trait gate, positive + negatives            |
//! | [When Digivolving] <=1 Tamers, free Tamer   | count gate 0/1/2, trait filter, decline, WD-not-OP    |
//! | Inherited <Piercing>                        | H3 inherited keyword (stack-walk) + integrated attack |

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_attack, encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "ST20-08";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-08 found in embedded DSL pack");
    // (id, level, color, [ADVENTURE]?, dp)
    for (id, level, color, adventure, dp) in [
        ("GREEN-LV3", 3, CardColor::Green, false, 3000),
        ("ADV-RED-LV3", 3, CardColor::Red, true, 3000),
        ("RED-LV3", 3, CardColor::Red, false, 3000),
        ("ADV-GREEN-LV2", 2, CardColor::Green, true, 2000),
        ("HOST", 5, CardColor::Red, false, 6000),
        ("WEAK", 3, CardColor::Red, false, 1000),
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
    for (id, adventure) in [
        ("ADV-TAMER", true),
        ("ADV-TAMER-2", true),
        ("PLAIN-TAMER", false),
    ] {
        let mut t = make_test_card(id, id);
        t.card_kind = CardKind::Tamer;
        t.level = None;
        t.dp = None;
        if adventure {
            t.traits = vec!["ADVENTURE".to_string()];
        }
        b = b.add_card(t);
    }
    let mut r = b
        .hand(0, hand)
        .deck(0, &["WEAK"; 12])
        .deck(1, &["WEAK"; 12])
        .security(1, &["WEAK"; 5])
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

fn accept_first(r: &mut DebugRunner) {
    let action = *r
        .pending_selection_view()
        .expect("a pending selection")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a non-PASS choice");
    choose(r, action);
}

fn top_card_id(r: &DebugRunner, player: usize, index: usize) -> String {
    r.game.players[player].battle_area[index]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn field_has(r: &DebugRunner, player: usize, card_id: &str) -> bool {
    r.game.players[player]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == card_id)
}

/// Digivolve the permanent at `base` into the Kabuterimon at hand index 0 via
/// the real digivolve action (so the [When Digivolving] queue runs).
fn digivolve_hand0_onto(r: &mut DebugRunner, base: u16) {
    let action = encode_digivolve(0, base);
    assert!(
        build_action_mask(&r.game, 0)[action as usize] > 0.5,
        "digivolve action must be mask-legal"
    );
    r.game.decode_action(action, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_08_identity_matches_printed_card() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-08 compiled");
    assert_eq!(card.name, "Kabuterimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    for t in ["Insectoid", "ADVENTURE"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "missing trait {t}, got {:?}",
            card.traits
        );
    }
    assert_eq!(card.attribute, ["Vaccine"]);
}

#[test]
fn st20_08_declares_standard_and_xros_alt_paths() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-08 compiled");
    assert_eq!(card.alt_paths.len(), 2, "standard + xros_req");

    let standard = card
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.color_is == Some(CompiledColor::Green))
        })
        .expect("standard Lv.3 Green path");
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
fn st20_08_when_digivolving_clause_is_not_a_clause_level_optional() {
    // DCGO isOptional=false: the "you may" is carried by the optional hand
    // pick (canNoSelect=true), not by a separate accept/decline prompt.
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-08 compiled");
    let wd = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("[When Digivolving] clause");
    assert!(!wd.optional);
    assert!(
        !wd.when.contains(&CompiledTiming::OnPlay),
        "text is [When Digivolving] only"
    );
    assert!(!wd.once_per_turn);
}

#[test]
fn st20_08_declares_inherited_piercing() {
    let r = setup(&[]);
    let card = r.compiled_card(ID).expect("ST20-08 compiled");
    let piercing = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Piercing")
        )
    });
    assert!(piercing, "ST20-08 must declare an inherited Piercing grant");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve routes (standard + xros_req)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_08_digivolve_routes_and_cost() {
    // (base, legal)
    for (base, legal) in [
        ("GREEN-LV3", true),      // standard Lv.3 Green circle
        ("ADV-RED-LV3", true),    // xros_req: Lv.3 w/[ADVENTURE], any colour
        ("RED-LV3", false),       // neither Green nor [ADVENTURE]
        ("ADV-GREEN-LV2", false), // [ADVENTURE] but Lv.2
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
            assert_eq!(before - r.memory(), 2, "{base}: digivolution cost is 2");
            assert_eq!(top_card_id(&r, 0, h.index as usize), ID);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving] free [ADVENTURE] Tamer (<= 1 Tamers)
// ═══════════════════════════════════════════════════════════════════════════

/// Integrated positive: 0 Tamers, one [ADVENTURE] and one plain Tamer in hand.
/// Only the [ADVENTURE] Tamer is offered; accepting plays it free.
#[test]
fn st20_08_when_digivolving_plays_adventure_tamer_free() {
    let mut r = setup(&[ID, "ADV-TAMER", "PLAIN-TAMER"]);
    let h = r.place_on_field(0, "GREEN-LV3", Some(0));
    let before = r.memory();
    digivolve_hand0_onto(&mut r, h.index as u16);

    let view = r
        .pending_selection_view()
        .expect("optional Tamer pick surfaces");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "'you may' must expose PASS");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the [ADVENTURE] Tamer is a legal pick (PLAIN-TAMER lacks the trait)"
    );

    accept_first(&mut r);
    r.auto_resolve().expect("nothing else pending");

    assert!(
        field_has(&r, 0, "ADV-TAMER"),
        "[ADVENTURE] Tamer was played"
    );
    assert!(!field_has(&r, 0, "PLAIN-TAMER"));
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(
        before - r.memory(),
        2,
        "only the digivolution cost was paid; the Tamer is played without paying"
    );
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "PLAIN-TAMER"),
        "the unchosen Tamer stays in hand"
    );
}

/// Count gate: 0 and 1 Tamers on the field -> the prompt appears; 2 Tamers ->
/// "1 or fewer" is false, so no prompt and nothing is played.
#[test]
fn st20_08_when_digivolving_tamer_count_gate() {
    for tamers_on_field in 0..=2usize {
        let mut r = setup(&[ID, "ADV-TAMER"]);
        let h = r.place_on_field(0, "GREEN-LV3", Some(0));
        for _ in 0..tamers_on_field {
            r.place_on_field(0, "PLAIN-TAMER", Some(0));
        }
        digivolve_hand0_onto(&mut r, h.index as u16);

        if tamers_on_field <= 1 {
            let view = r
                .pending_selection_view()
                .unwrap_or_else(|| panic!("{tamers_on_field} Tamers: prompt expected"));
            assert_eq!(view.kind, SelectionKind::Hand);
            accept_first(&mut r);
            assert!(
                field_has(&r, 0, "ADV-TAMER"),
                "{tamers_on_field} Tamers: ADV-TAMER should be played"
            );
            assert_eq!(
                r.game.players[0].battle_area.len(),
                tamers_on_field + 2,
                "Kabuterimon's base slot + existing Tamers + the new Tamer"
            );
        } else {
            assert!(
                r.pending_selection().is_none(),
                "2 Tamers: the condition fails, no prompt"
            );
            assert!(!field_has(&r, 0, "ADV-TAMER"));
            assert_eq!(r.game.players[0].battle_area.len(), 3);
        }
    }
}

/// Decline: PASS plays nothing and the Tamers stay in hand.
#[test]
fn st20_08_when_digivolving_decline_plays_nothing() {
    let mut r = setup(&[ID, "ADV-TAMER", "ADV-TAMER-2"]);
    let h = r.place_on_field(0, "GREEN-LV3", Some(0));
    digivolve_hand0_onto(&mut r, h.index as u16);

    let view = r.pending_selection_view().expect("optional pick");
    assert!(view.is_optional);
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "both [ADVENTURE] Tamers are legal picks"
    );
    choose(&mut r, PASS);
    r.auto_resolve().ok();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "no Tamer was played"
    );
    assert_eq!(
        r.game.players[0]
            .hand
            .iter()
            .filter(|c| c.card_id(&r.game.card_data).starts_with("ADV-TAMER"))
            .count(),
        2
    );
}

/// Negative: with no [ADVENTURE] Tamer in hand nothing can be played. (The
/// engine may skip the empty pick or show a PASS-only pick; either way no
/// Tamer enters play.)
#[test]
fn st20_08_when_digivolving_no_adventure_tamer_in_hand() {
    let mut r = setup(&[ID, "PLAIN-TAMER"]);
    let h = r.place_on_field(0, "GREEN-LV3", Some(0));
    digivolve_hand0_onto(&mut r, h.index as u16);
    if r.pending_selection().is_some() {
        assert!(r.pending_is_optional());
        assert_eq!(r.pending_action_count(), 0, "no playable candidates");
        choose(&mut r, PASS);
    }
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(!field_has(&r, 0, "PLAIN-TAMER"));
}

/// Negative: the text is [When Digivolving] only — playing Kabuterimon from
/// hand ([On Play]) must not offer the Tamer.
#[test]
fn st20_08_playing_from_hand_does_not_trigger_tamer_play() {
    let mut r = setup(&[ID, "ADV-TAMER"]);
    r.play(0, 0).expect("Kabuterimon plays from hand");
    r.auto_resolve().ok();
    assert!(r.pending_selection().is_none());
    assert!(!field_has(&r, 0, "ADV-TAMER"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited <Piercing>
// ═══════════════════════════════════════════════════════════════════════════

/// Stack-walk: a carrier with ST20-08 among its digivolution sources has
/// Piercing; the same carrier without it does not; the face-up ST20-08 itself
/// does not (inherited text applies only from under another card).
#[test]
fn st20_08_inherited_piercing_grants_to_carrier_only() {
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "HOST"]);
    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "HOST with ST20-08 beneath must have <Piercing>"
    );

    let plain = r.place_on_field(0, "HOST", Some(0));
    assert!(
        !r.game.has_keyword(plain, Keyword::Piercing),
        "a carrier without ST20-08 beneath must NOT have <Piercing>"
    );

    let face_up = r.place_on_field(0, ID, Some(0));
    assert!(
        !r.game.has_keyword(face_up, Keyword::Piercing),
        "the printed <Piercing> is inherited text and does not apply to the face-up card"
    );
}

/// Integrated: a carrier with the inherited Piercing deletes a suspended
/// Digimon and then performs a security check; without ST20-08 it does not.
#[test]
fn st20_08_inherited_piercing_checks_security_after_deleting_a_digimon() {
    for (stack, expected_security) in [(vec![ID, "HOST"], 4usize), (vec!["HOST"], 5usize)] {
        let mut r = setup(&[]);
        r.place_stack(0, &stack);
        r.place_field_stack(1, &["WEAK"], true, 0);
        assert_eq!(r.security_count(1), 5);

        let action = encode_attack(0, 0);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
        r.game.decode_action(action, 0);
        r.auto_resolve().ok();

        assert_eq!(
            r.battle_area_size(1),
            0,
            "the 1000 DP defender is deleted by the 6000 DP carrier ({stack:?})"
        );
        assert_eq!(
            r.security_count(1),
            expected_security,
            "stack {stack:?}: Piercing performs exactly one security check"
        );
    }
}
