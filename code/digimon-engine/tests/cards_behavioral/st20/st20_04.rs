//! ST20-04 Garudamon — Digimon, Lv.5, Red, DP 7000, play cost 7.
//! Traits: Birdkin, ADVENTURE. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-04.md)
//!
//! ```text
//! [On Play] [When Digivolving] For the turn, 1 of your Digimon gains
//! <Security A. +1>, and for every 2 colors your Tamers have, gets +2000 DP.
//! (This Digimon checks 1 additional security card.)
//! [Your Turn] [Once Per Turn] When your other Digimon are played or digivolve,
//! if any of them have the [ADVENTURE] trait, 1 of your Digimon gains
//! <Alliance> for the turn. Then, 1 of your Digimon may attack.
//!
//! Inherited Effect
//! <Alliance> (When this Digimon attacks, by suspending 1 of your other
//! Digimon, this Digimon gains <Security A. +1> and the suspended Digimon's
//! DP for the attack.)
//!
//! evo_costs:  Red Lv.4: Cost 3
//! xros_req:   [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3
//! Official Q&A: the [On Play]/[When Digivolving] recipient pick is mandatory.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_04.cs
//!   - Alternate Digivolution Requirement: `HasAdventureTraits && Level == 4`,
//!     cost 3, requirements not ignored.
//!   - [On Play]/[When Digivolving]: mandatory own-Digimon pick (`canNoSelect:
//!     false`) -> `ChangeDigimonSAttack(+1)` + `ChangeDigimonDP(2000 *
//!     (distinct colours across ALL own Tamers / 2))`, both UntilEachTurnEnd.
//!   - [Your Turn][OPT] (hash "alliance-attack-ST20_04"): fires when ANOTHER
//!     own Digimon is played or digivolves; if the played/digivolved Digimon has
//!     [ADVENTURE]: mandatory Alliance recipient pick (UntilEachTurnEnd), then an
//!     optional own-Digimon attack.
//!   - Alliance Inherit: `AllianceSelfEffect(isInheritedEffect: true)`.
//!
//! # Patterns (docs/RUST_DSL_TEST_API.md §4.3)
//! - E  mandatory own-Digimon target pick, modifiers expiring at end of turn
//! - G  OPT observer over play AND digivolve of OTHER own Digimon, event-gated
//!      on trait / owner / source-exclusion, with [Your Turn] gate
//! - H  inherited keyword grant (stack-walk, ST20-10 mold)
//! - Alt-path digivolve (standard circle + xros_req box), cost deducted
//!
//! The Alliance recipient and the attacker prompts are real player choices; the
//! tests drive them through the mask (`valid_action_ids`) rather than
//! `auto_resolve`, except where only the end state matters.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "ST20-04";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, dp: i32, cost: u16, adventure: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.colors = vec![CardColor::Red];
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// A Lv.4 red Digimon that digivolves from a Lv.3 red base for free.
fn evo_from_lv3(id: &str, adventure: bool) -> CardData {
    let mut c = digimon(id, 4, 4000, 4, adventure);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 0,
    }];
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c
}

/// Runner with the whole fixture set registered; `hand` is P0's hand.
fn garudamon_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-04 in embedded DSL pack")
        .add_card(digimon("ALLY", 4, 3000, 3, false))
        .add_card(digimon("ADV-PLAY", 3, 2000, 1, true))
        .add_card(digimon("ADV-PLAY-2", 3, 2000, 1, true))
        .add_card(digimon("PLAIN-PLAY", 3, 2000, 1, false))
        .add_card(digimon("BASE-LV3", 3, 2000, 2, false))
        .add_card(digimon("BASE-LV4", 4, 4000, 4, false))
        .add_card(evo_from_lv3("ADV-EVO", true))
        .add_card(evo_from_lv3("PLAIN-EVO", false))
        .add_card(digimon("OPP-ADV", 3, 2000, 1, true))
        .add_card(digimon("CARRIER", 6, 6000, 7, false))
        .add_card(tamer("TAMER-R", CardColor::Red))
        .add_card(tamer("TAMER-R2", CardColor::Red))
        .add_card(tamer("TAMER-B", CardColor::Blue))
        .add_card(tamer("TAMER-G", CardColor::Green))
        .add_card(tamer("TAMER-Y", CardColor::Yellow))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, hand)
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(10)
        .start()
}

fn own_field_action(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

/// Pick `h` in the pending own-Digimon prompt, asserting it is legal.
fn pick_own(runner: &mut DebugRunner, h: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = own_field_action(h);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: {action} not in {:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

fn assert_mandatory_prompt(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(!runner.pending_is_optional(), "{label}: must be mandatory");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal"
    );
}

fn play_hand_card(runner: &mut DebugRunner, hand_index: usize) -> PermanentHandle {
    let index = runner.play(0, hand_index).expect("play from hand");
    PermanentHandle {
        player: 0,
        index: index as u8,
    }
}

fn sec_atk(runner: &DebugRunner, h: PermanentHandle) -> i32 {
    runner
        .modifiers()
        .sum(h, ModifierType::SecurityAttackChange)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_04_identity_matches_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Garudamon");
    assert_eq!(c.level, Some(5));
    assert_eq!(c.dp, Some(7000));
    assert_eq!(c.cost, Some(7));
    let traits: Vec<String> = c.all_traits().iter().map(|t| t.to_lowercase()).collect();
    for want in ["birdkin", "adventure", "vaccine"] {
        assert!(
            traits.contains(&want.to_string()),
            "missing trait {want}: {traits:?}"
        );
    }
}

/// Two requirement-respecting cost-3 digivolve paths from Lv.4.
#[test]
fn st20_04_declares_standard_and_xros_req_alt_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.alt_paths.len(), 2, "standard circle + xros_req box");
    for path in &c.alt_paths {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(path.cost, Some(CompiledCost::Literal(3)));
        assert!(
            !path.ignore_requirements,
            "DCGO: ignoreDigivolutionRequirement false"
        );
        assert_eq!(path.from.as_ref().expect("from").level_eq, Some(4));
    }
    assert!(
        c.alt_paths
            .iter()
            .any(|p| p.from.as_ref().unwrap().trait_has.as_deref() == Some("ADVENTURE")),
        "an [ADVENTURE] source path must exist"
    );
}

/// Clause layout: two face-up triggered clauses (mandatory) + inherited Alliance.
#[test]
fn st20_04_clause_layout() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.effects.len(), 3);

    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);

    let boost = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving])
        .expect("[On Play][When Digivolving] clause");
    assert_eq!(boost.scope, CompiledScope::FaceUp);
    assert!(
        !boost.optional,
        "the recipient pick is mandatory (official Q&A)"
    );
    assert!(!boost.once_per_turn);

    let observer = triggered
        .iter()
        .find(|t| {
            t.when
                == vec![
                    CompiledTiming::OnEnterFieldAnyone,
                    CompiledTiming::OnDigivolve,
                ]
        })
        .expect("[Your Turn][OPT] observer over play and digivolve");
    assert_eq!(observer.scope, CompiledScope::FaceUp);
    assert!(observer.once_per_turn, "[Once Per Turn]");
    assert!(!observer.optional, "no \"may\" on the trigger itself");
    assert!(observer.active_when.is_some(), "[Your Turn] gate");
    assert!(
        observer.trigger_if.is_some(),
        "other-Digimon / ADVENTURE event gate"
    );

    let inherited_alliance = c.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Alliance")
        )
    });
    assert!(inherited_alliance, "inherited <Alliance> grant");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] / [When Digivolving]: <Security A. +1> and DP per 2 colours
// ═══════════════════════════════════════════════════════════════════════════

/// Play Garudamon with `tamers` on the field and ALLY (3000 DP) as the pick;
/// returns (runner, ally).
fn boost_ally(tamers: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = garudamon_runner(&[CARD_ID]);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    for t in tamers {
        runner.place_on_field(0, t, None);
    }
    play_hand_card(&mut runner, 0);
    assert_mandatory_prompt(&runner, "recipient pick");
    pick_own(&mut runner, ally, "pick ALLY");
    (runner, ally)
}

/// POSITIVE: 3 Tamer colours -> floor(3/2) = 1 -> +2000 DP, and <Security A. +1>.
#[test]
fn st20_04_on_play_three_tamer_colours_gives_one_security_attack_and_2000_dp() {
    let (runner, ally) = boost_ally(&["TAMER-R", "TAMER-B", "TAMER-G"]);
    assert_eq!(runner.effective_dp(ally), Some(5000));
    assert_eq!(sec_atk(&runner, ally), 1);
    assert!(
        runner.pending_selection().is_none(),
        "Garudamon's own play must not also fire its [Your Turn] observer (source exclusion)"
    );
}

/// POSITIVE: 4 Tamer colours -> floor(4/2) = 2 -> +4000 DP ("for every 2").
#[test]
fn st20_04_on_play_four_tamer_colours_gives_4000_dp() {
    let (runner, ally) = boost_ally(&["TAMER-R", "TAMER-B", "TAMER-G", "TAMER-Y"]);
    assert_eq!(runner.effective_dp(ally), Some(7000));
    assert_eq!(sec_atk(&runner, ally), 1);
}

/// POSITIVE: 2 Tamer colours -> exactly +2000 DP.
#[test]
fn st20_04_on_play_two_tamer_colours_gives_2000_dp() {
    let (runner, ally) = boost_ally(&["TAMER-R", "TAMER-B"]);
    assert_eq!(runner.effective_dp(ally), Some(5000));
}

/// Colours are DISTINCT: two red Tamers + one blue Tamer is 2 colours, not 3.
#[test]
fn st20_04_duplicate_tamer_colours_count_once() {
    let (runner, ally) = boost_ally(&["TAMER-R", "TAMER-R2", "TAMER-B"]);
    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "2 distinct colours -> +2000, not +2000 x1.5"
    );
}

/// NEGATIVE: 1 Tamer colour -> no DP, but the <Security A. +1> is still gained.
#[test]
fn st20_04_on_play_one_tamer_colour_gives_security_attack_but_no_dp() {
    let (runner, ally) = boost_ally(&["TAMER-R"]);
    assert_eq!(runner.effective_dp(ally), Some(3000), "floor(1/2) = 0");
    assert_eq!(
        sec_atk(&runner, ally),
        1,
        "<Security A. +1> is unconditional"
    );
}

/// NEGATIVE: with no Tamers at all the DP bonus is 0 and the pick still happened.
#[test]
fn st20_04_on_play_no_tamers_gives_security_attack_but_no_dp() {
    let (runner, ally) = boost_ally(&[]);
    assert_eq!(runner.effective_dp(ally), Some(3000));
    assert_eq!(sec_atk(&runner, ally), 1);
}

/// The recipient may be Garudamon itself ("1 of your Digimon").
#[test]
fn st20_04_recipient_may_be_garudamon_itself() {
    let mut runner = garudamon_runner(&[CARD_ID]);
    runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(0, "TAMER-R", None);
    runner.place_on_field(0, "TAMER-B", None);
    let garudamon = play_hand_card(&mut runner, 0);
    assert_mandatory_prompt(&runner, "recipient pick");
    pick_own(&mut runner, garudamon, "pick Garudamon");
    assert_eq!(
        runner.effective_dp(garudamon),
        Some(9000),
        "7000 + 2000 (2 colours)"
    );
    assert_eq!(sec_atk(&runner, garudamon), 1);
}

/// The recipient prompt offers every one of your Digimon and no Tamers.
#[test]
fn st20_04_recipient_prompt_offers_each_own_digimon_and_no_tamer() {
    let mut runner = garudamon_runner(&[CARD_ID]);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let tamer = runner.place_on_field(0, "TAMER-R", None);
    let garudamon = play_hand_card(&mut runner, 0);
    let view = runner.pending_selection_view().expect("recipient pick");
    assert!(view.valid_action_ids.contains(&own_field_action(ally)));
    assert!(view.valid_action_ids.contains(&own_field_action(garudamon)));
    assert!(
        !view.valid_action_ids.contains(&own_field_action(tamer)),
        "a Tamer is not \"1 of your Digimon\""
    );
}

/// "For the turn": the Security Attack and the DP bonus expire at end of turn.
#[test]
fn st20_04_on_play_boosts_expire_at_end_of_turn() {
    let (mut runner, ally) = boost_ally(&["TAMER-R", "TAMER-B", "TAMER-G"]);
    assert_eq!(runner.effective_dp(ally), Some(5000));
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert_eq!(
        sec_atk(&runner, ally),
        0,
        "<Security A. +1> is for the turn only"
    );
    assert_eq!(
        runner.effective_dp(ally),
        Some(3000),
        "+2000 DP is for the turn only"
    );
}

/// [When Digivolving]: digivolving a Lv.4 red base into Garudamon (cost 3) fires
/// the same mandatory pick; the digivolved Garudamon can be the recipient.
#[test]
fn st20_04_when_digivolving_grants_the_same_boost() {
    let mut runner = garudamon_runner(&[CARD_ID]);
    runner.place_on_field(0, "ALLY", Some(0));
    let base = runner.place_on_field(0, "BASE-LV4", Some(0));
    runner.place_on_field(0, "TAMER-R", None);
    runner.place_on_field(0, "TAMER-B", None);
    let before = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    assert_eq!(before - runner.memory(), 3, "Red Lv.4 -> cost 3");
    assert_mandatory_prompt(&runner, "recipient pick");
    pick_own(&mut runner, base, "pick the digivolved Garudamon");
    assert_eq!(
        runner.effective_dp(base),
        Some(9000),
        "7000 + 2000 (2 colours)"
    );
    assert_eq!(sec_atk(&runner, base), 1);
    assert!(
        runner.pending_selection().is_none(),
        "Garudamon's own digivolution must not fire its [Your Turn] observer"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn][OPT]: other ADVENTURE Digimon played / digivolved
// ═══════════════════════════════════════════════════════════════════════════

/// Garudamon and ALLY already on the field (able to attack), `hand` in P0's hand.
fn observer_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = garudamon_runner(hand);
    let garudamon = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    (runner, garudamon, ally)
}

/// Resolve the observer fully: pick `recipient` for Alliance, then decline the
/// optional attack.
fn resolve_observer_declining_attack(runner: &mut DebugRunner, recipient: PermanentHandle) {
    assert_mandatory_prompt(runner, "Alliance recipient");
    pick_own(runner, recipient, "pick the Alliance recipient");
    assert!(
        runner.pending_is_optional(),
        "\"may attack\" must be declinable"
    );
    runner.execute_action(0, PASS).expect("decline the attack");
    assert!(runner.pending_selection().is_none());
}

/// POSITIVE (play): playing an [ADVENTURE] Digimon gives ANY Digimon <Alliance>
/// for the turn, then offers the optional attack.
#[test]
fn st20_04_playing_an_adventure_digimon_grants_alliance_then_offers_an_attack() {
    let (mut runner, garudamon, ally) = observer_runner(&["ADV-PLAY"]);
    assert!(!runner.game.has_keyword(ally, Keyword::Alliance));
    let played = play_hand_card(&mut runner, 0);

    let view = runner
        .pending_selection_view()
        .expect("Alliance recipient prompt");
    for h in [garudamon, ally, played] {
        assert!(
            view.valid_action_ids.contains(&own_field_action(h)),
            "every Digimon of yours is a legal recipient"
        );
    }
    resolve_observer_declining_attack(&mut runner, ally);
    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the chosen non-ADVENTURE Digimon gained <Alliance>"
    );
    assert!(!runner.game.has_keyword(garudamon, Keyword::Alliance));
    assert!(!runner.game.has_keyword(played, Keyword::Alliance));
}

/// "Then, 1 of your Digimon may attack": the attacker prompt lists Digimon that
/// can attack (not the just-played, summoning-sick one) and PASS is legal.
#[test]
fn st20_04_attack_prompt_lists_only_digimon_that_can_attack() {
    let (mut runner, garudamon, ally) = observer_runner(&["ADV-PLAY"]);
    let played = play_hand_card(&mut runner, 0);
    pick_own(&mut runner, ally, "pick the Alliance recipient");

    let view = runner.pending_selection_view().expect("attacker prompt");
    assert!(runner.pending_is_optional());
    assert!(view.valid_action_ids.contains(&own_field_action(ally)));
    assert!(view.valid_action_ids.contains(&own_field_action(garudamon)));
    assert!(
        !view.valid_action_ids.contains(&own_field_action(played)),
        "the Digimon played this turn is summoning-sick and cannot attack"
    );
}

/// "Then, 1 of your Digimon may attack": choosing an attacker declares the
/// attack (the attacker suspends).
#[test]
fn st20_04_choosing_an_attacker_makes_it_attack() {
    let (mut runner, _garudamon, ally) = observer_runner(&["ADV-PLAY"]);
    play_hand_card(&mut runner, 0);
    pick_own(&mut runner, ally, "pick the Alliance recipient");
    pick_own(&mut runner, ally, "pick ALLY as the attacker");
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0].battle_area[ally.index as usize].is_suspended,
        "ALLY declared an attack and is suspended"
    );
}

/// The <Alliance> is for the turn only.
#[test]
fn st20_04_granted_alliance_expires_at_end_of_turn() {
    let (mut runner, _garudamon, ally) = observer_runner(&["ADV-PLAY"]);
    play_hand_card(&mut runner, 0);
    resolve_observer_declining_attack(&mut runner, ally);
    assert!(runner.game.has_keyword(ally, Keyword::Alliance));
    runner.end_turn();
    assert!(
        !runner.game.has_keyword(ally, Keyword::Alliance),
        "<Alliance> expires at end of turn"
    );
}

/// POSITIVE (digivolve): digivolving an [ADVENTURE] Digimon fires the observer.
#[test]
fn st20_04_digivolving_an_adventure_digimon_grants_alliance() {
    let (mut runner, _garudamon, ally) = observer_runner(&["ADV-EVO"]);
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    resolve_observer_declining_attack(&mut runner, ally);
    assert!(runner.game.has_keyword(ally, Keyword::Alliance));
}

/// NEGATIVE (play): a Digimon WITHOUT [ADVENTURE] does not trigger anything.
#[test]
fn st20_04_non_adventure_digimon_played_does_nothing() {
    let (mut runner, _garudamon, ally) = observer_runner(&["PLAIN-PLAY"]);
    play_hand_card(&mut runner, 0);
    assert!(
        runner.pending_selection().is_none(),
        "no [ADVENTURE] Digimon was played: no prompt and no attack offer"
    );
    assert!(!runner.game.has_keyword(ally, Keyword::Alliance));
}

/// NEGATIVE (digivolve): a Digimon WITHOUT [ADVENTURE] digivolving does nothing.
#[test]
fn st20_04_non_adventure_digimon_digivolved_does_nothing() {
    let (mut runner, _garudamon, _ally) = observer_runner(&["PLAIN-EVO"]);
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    assert!(runner.pending_selection().is_none());
}

/// NEGATIVE: the OPPONENT's [ADVENTURE] Digimon entering play is not "your
/// other Digimon".
#[test]
fn st20_04_opponents_adventure_digimon_does_not_trigger() {
    let (mut runner, _garudamon, _ally) = observer_runner(&[]);
    let opp = runner.place_on_field(1, "OPP-ADV", Some(0));
    runner.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert!(runner.pending_selection().is_none());
}

/// NEGATIVE: [Your Turn] — an [ADVENTURE] Digimon of yours entering on the
/// OPPONENT's turn does not trigger.
#[test]
fn st20_04_does_not_trigger_on_the_opponents_turn() {
    let (mut runner, _garudamon, ally) = observer_runner(&[]);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    let entering = runner.place_on_field(0, "ADV-PLAY", Some(0));
    runner.fire_play_event_triggers(0, entering.index as usize, true, false);
    assert!(runner.pending_selection().is_none());
    assert!(!runner.game.has_keyword(ally, Keyword::Alliance));
}

/// OPT: a second [ADVENTURE] Digimon in the same turn does not trigger again.
#[test]
fn st20_04_once_per_turn_locks_the_second_trigger() {
    let (mut runner, _garudamon, ally) = observer_runner(&["ADV-PLAY", "ADV-PLAY-2"]);
    play_hand_card(&mut runner, 0);
    resolve_observer_declining_attack(&mut runner, ally);

    play_hand_card(&mut runner, 0);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must block the second [ADVENTURE] Digimon this turn"
    );
}

/// OPT is shared by play AND digivolve: after a play trigger, a digivolve of an
/// [ADVENTURE] Digimon in the same turn does not trigger.
#[test]
fn st20_04_once_per_turn_is_shared_between_play_and_digivolve() {
    let (mut runner, _garudamon, ally) = observer_runner(&["ADV-PLAY", "ADV-EVO"]);
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));
    play_hand_card(&mut runner, 0);
    resolve_observer_declining_attack(&mut runner, ally);

    // ADV-EVO is now hand slot 0.
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    assert!(
        runner.pending_selection().is_none(),
        "one shared per-turn counter"
    );
}

/// OPT clears the next time it is your turn.
#[test]
fn st20_04_once_per_turn_clears_on_the_next_turn() {
    let (mut runner, _garudamon, ally) = observer_runner(&["ADV-PLAY"]);
    play_hand_card(&mut runner, 0);
    resolve_observer_declining_attack(&mut runner, ally);

    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 0, "back to your turn");

    let entering = runner.place_on_field(0, "ADV-PLAY-2", Some(0));
    runner.fire_play_event_triggers(0, entering.index as usize, false, false);
    assert!(
        runner.pending_selection().is_some(),
        "the OPT lockout clears on a new turn: the observer fires again"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited <Alliance>
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: a carrier with Garudamon in its digivolution stack has <Alliance>.
#[test]
fn st20_04_inherited_alliance_grants_to_carrier_via_stack() {
    let mut runner = garudamon_runner(&[]);
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Alliance),
        "CARRIER with ST20-04 beneath it inherits <Alliance>"
    );
}

/// NEGATIVE: a carrier without Garudamon in its stack does not have <Alliance>.
#[test]
fn st20_04_inherited_alliance_absent_without_source() {
    let mut runner = garudamon_runner(&[]);
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    assert!(!runner.game.has_keyword(carrier, Keyword::Alliance));
}

/// NEGATIVE: Alliance is printed in the INHERITED box only, so face-up Garudamon
/// itself does not have it.
#[test]
fn st20_04_face_up_garudamon_does_not_have_alliance() {
    let mut runner = garudamon_runner(&[]);
    let garudamon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner.game.has_keyword(garudamon, Keyword::Alliance));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Alternate digivolution (evo_costs circle + xros_req box), cost 3
// ═══════════════════════════════════════════════════════════════════════════

fn lv_base(id: &str, level: u8, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![color];
    c.dp = Some(4000);
    c.play_cost = 4;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// Try to digivolve Garudamon (in hand) onto `b`; returns (succeeded, memory spent).
fn try_digivolve(b: CardData) -> (bool, i16) {
    let base_id = b.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-04 in embedded DSL pack")
        .add_card(b)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();
    let handle = r.place_on_field(0, &base_id, Some(0));
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, 0, handle.index as usize, PlaySource::ByDigivolve);
    let spent = before - r.memory();
    let _ = r.auto_resolve();
    (ok, spent)
}

/// Standard circle: Red Lv.4 -> cost 3.
#[test]
fn st20_04_digivolves_from_red_level_4_for_3() {
    assert_eq!(
        try_digivolve(lv_base("RED-LV4", 4, CardColor::Red, false)),
        (true, 3)
    );
}

/// xros_req box: a non-red Lv.4 [ADVENTURE] Digimon -> cost 3.
#[test]
fn st20_04_digivolves_from_non_red_level_4_adventure_for_3() {
    assert_eq!(
        try_digivolve(lv_base("BLUE-LV4-ADV", 4, CardColor::Blue, true)),
        (true, 3)
    );
}

/// NEGATIVE: a non-red Lv.4 without [ADVENTURE] is not a legal base.
#[test]
fn st20_04_rejects_non_red_level_4_without_adventure() {
    assert!(!try_digivolve(lv_base("BLUE-LV4", 4, CardColor::Blue, false)).0);
}

/// NEGATIVE: the level still has to be 4 — a Lv.3 [ADVENTURE] base is rejected.
#[test]
fn st20_04_rejects_level_3_adventure_base() {
    assert!(!try_digivolve(lv_base("RED-LV3-ADV", 3, CardColor::Red, true)).0);
}
