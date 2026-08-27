//! Symbolic step -> action ID, resolved against the engine's LIVE mask.
//!
//! This is the cheap gate. A malformed scenario fails here in milliseconds
//! instead of after sixty seconds of Unity, and the action IDs this produces
//! are written into the DCGO job file -- so both engines consume literally the
//! same integers rather than each interpreting the symbolic form themselves.
//!
//! **The load-bearing invariant:** lowering never emits an action the mask
//! forbids. Candidates are drawn *from* the mask, so a scenario that lowers is
//! provably a legal line before Unity is ever launched.

use crate::exam::scenario::StepAction;
use digimon_engine::action::explain::{explain_action, ActionExplanation, ActionKind, ActionZone};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::{Game, PlayerId};

#[derive(Debug)]
pub enum LowerError {
    /// No legal action matches the intent. Carries the legal set, because a
    /// bare "no match" makes the author guess and a guess costs a Unity round
    /// trip.
    NoMatch { intent: String, legal: Vec<String> },
    /// More than one legal action matches. Never picked arbitrarily: an
    /// arbitrary pick would silently answer a different question than the one
    /// the scenario asks.
    Ambiguous { intent: String, matches: Vec<u16> },
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LowerError::NoMatch { intent, legal } => write!(
                f,
                "no legal action matches {intent}; legal here: [{}]",
                legal.join(", ")
            ),
            LowerError::Ambiguous { intent, matches } => write!(
                f,
                "{intent} matches {} legal actions ({matches:?}); \
                 disambiguate the step -- picking one arbitrarily would answer \
                 a different question than the scenario asks",
                matches.len()
            ),
        }
    }
}

impl std::error::Error for LowerError {}

/// Every action ID currently legal for `actor`, with its explanation.
fn legal_explanations(game: &Game, actor: PlayerId) -> Vec<(u16, ActionExplanation)> {
    let mask = build_action_mask(game, actor);
    mask.iter()
        .enumerate()
        .filter(|(_, &v)| v == 1.0)
        .map(|(i, _)| (i as u16, explain_action(game, actor, i as u16)))
        .collect()
}

/// Resolve one symbolic step against the live mask.
///
/// Exactly one match is required. Zero matches reports what *was* legal; more
/// than one is an error rather than a coin flip.
pub fn lower_step(game: &Game, actor: PlayerId, act: &StepAction) -> Result<u16, LowerError> {
    let legal = legal_explanations(game, actor);
    let intent = format!("{act:?}");

    let matches: Vec<u16> = legal
        .iter()
        .filter(|(_, e)| matches_intent(e, act))
        .map(|(id, _)| *id)
        .collect();

    match matches.len() {
        1 => Ok(matches[0]),
        0 => Err(LowerError::NoMatch {
            intent,
            legal: legal
                .iter()
                .map(|(id, e)| format!("{id}: {}", e.label))
                .collect(),
        }),
        _ => Err(LowerError::Ambiguous { intent, matches }),
    }
}

fn matches_intent(e: &ActionExplanation, act: &StepAction) -> bool {
    match act {
        StepAction::Pass(_) => e.kind == ActionKind::Pass,
        StepAction::Hatch(_) => e.kind == ActionKind::Hatch,
        StepAction::Move { from } => {
            e.kind == ActionKind::Move && breeding_source_matches(e.source_zone, e.source_index, from)
        }
        StepAction::Play { card, from } => {
            e.kind == ActionKind::Play
                && e.card_id.as_deref() == Some(card.as_str())
                && zone_ref_matches(e.source_zone, e.source_index, from)
        }
        StepAction::Digivolve { from, using } => {
            e.kind == ActionKind::Digivolve
                && e.card_id.as_deref() == Some(using.as_str())
                && slot_matches(e.target_zone, e.target_index, from)
        }
        StepAction::Attack { attacker, target } => {
            e.kind == ActionKind::Attack
                && slot_matches(e.source_zone, e.source_index, attacker)
                && slot_matches(e.target_zone, e.target_index, target)
        }
        // A `[Main]` effect on a permanent already in play. Matched on the
        // SLOT, never on `card_id` alone: two copies of the same Option on the
        // field are two different decisions, and this module's standing rule
        // is that an ambiguous intent must refuse rather than pick.
        //
        // `slot_matches` already handles both forms the engine can emit —
        // `field.N` for a battle-area permanent (`source_zone = Battle`,
        // `source_index = slot`) and the bare `breeding` sentinel for the
        // breeding area's `<Training>` [Main] (`source_zone = Breeding`, no
        // index).
        StepAction::Main { on } => {
            e.kind == ActionKind::FieldEffect
                && slot_matches(e.source_zone, e.source_index, on)
        }
        // Selections resolve against the live PendingSelection rather than the
        // main mask; Task 3 threads them through ScenarioAdapter, which is why
        // they never match here.
        // Neither select form matches against the main-phase mask: both resolve
        // against a live PendingSelection instead. A DCGO-only row resolves
        // against nothing at all on our side, by definition.
        StepAction::Select { .. } | StepAction::SelectDcgoOnly { .. } => false,
    }
}

/// Match a zone reference that may or may not pin a slot: `"hand"` matches any
/// hand slot, `"hand.6"` only slot 6.
///
/// The pinned form exists because a decklist routinely holds up to four copies
/// of a card, so `play: {card: ST1-12, from: hand}` is genuinely AMBIGUOUS the
/// moment two copies are in hand -- lowering reports `[6, 7] all match` and
/// refuses, correctly, to pick one. Without a way to say which, any scenario
/// touching a duplicated card is unauthorable.
///
/// Deliberately still ambiguous when unpinned: silently taking the lowest index
/// would answer a different question than the scenario asked, and the two
/// copies are only interchangeable until one of them carries a modifier or a
/// digivolution stack.
fn zone_ref_matches(zone: Option<ActionZone>, index: Option<u16>, reference: &str) -> bool {
    match reference.split_once('.') {
        None => zone_name(zone) == reference,
        Some((z, i)) => match i.parse::<u16>() {
            Ok(want) => zone_name(zone) == z && index == Some(want),
            Err(_) => false,
        },
    }
}

fn zone_name(z: Option<ActionZone>) -> &'static str {
    match z {
        Some(ActionZone::Hand) => "hand",
        Some(ActionZone::Battle) => "field",
        Some(ActionZone::Breeding) => "breeding",
        Some(ActionZone::Security) => "security",
        Some(ActionZone::Trash) => "trash",
        Some(ActionZone::Source) => "source",
        Some(ActionZone::Revealed) => "revealed",
        Some(ActionZone::EffectChoice) => "effect_choice",
        None => "",
    }
}

/// Matches a `move` step's `from` against the explanation's source.
///
/// The breeding area holds exactly one permanent, and the engine encodes the
/// move-to-battle action as a single sentinel id (`MOVE_FROM_BREEDING`, kind
/// `Move`) whose explanation carries `source_zone = Breeding` with NO source
/// index — like `SECURITY_TARGET` / `BREEDING_TARGET`, it names a place, not a
/// slot. So the bare form `breeding` and the only pin that can exist,
/// `breeding.0`, are the same intent and both must match; any other reference
/// (another zone, another index) names a move the game does not have.
fn breeding_source_matches(zone: Option<ActionZone>, index: Option<u16>, reference: &str) -> bool {
    zone == Some(ActionZone::Breeding)
        && index.is_none()
        && (reference == "breeding" || reference == "breeding.0")
}

/// Matches a slot reference against an explanation's `(zone, index)` pair.
///
/// Two forms:
/// - `"field.0"` / `"breeding.1"` — a zone plus an explicit slot index.
/// - `"security"` / `"breeding"` — a bare zone, matching only when the
///   explanation carries no index. The engine encodes "attack security" and
///   "digivolve onto the breeding area" that way (`SECURITY_TARGET` /
///   `BREEDING_TARGET` are sentinels, not board slots), so without this form
///   those two steps would be unexpressible.
fn slot_matches(zone: Option<ActionZone>, index: Option<u16>, reference: &str) -> bool {
    match reference.rsplit_once('.') {
        Some((z, i)) => {
            let Ok(want) = i.parse::<u16>() else {
                return false;
            };
            zone_name(zone) == z && index == Some(want)
        }
        None => zone_name(zone) == reference && index.is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::scenario::{EmptyArgs, StepAction};
    use crate::exam::test_support;

    /// A real 2-player game advanced to the first turn player's Breeding
    /// phase, so lowering is exercised against the true mask rather than a
    /// mock.
    ///
    /// `new_with_ordered_decks` (not `new`) so the deck order is exactly the
    /// list handed in and player 0 is deterministically first — otherwise
    /// `Game::new`'s `seed % 2` first-player pick would decide which seat the
    /// test's hard-coded `actor: 0` refers to.
    fn game_from(deck: Vec<String>) -> digimon_engine::Game {
        let card_data = test_support::load_card_data();
        let mut g = digimon_engine::Game::new_with_ordered_decks(
            &[deck.clone(), deck],
            &card_data,
            digimon_engine::Rules::standard(),
            Some(42),
            0,
        )
        .expect("game should build");
        // Resolve both mulligans (keep) and enter turn 1.
        g.start_game();
        g
    }

    /// The stock ST-1 list (50 main + 4 egg), in printed order.
    fn st1_deck() -> Vec<String> {
        let (mut main, egg) = test_support::st1_decks();
        main.extend(egg);
        main
    }

    /// The ST-1 list reordered so `opening_hand` is what player 0 draws.
    ///
    /// `Player::draw` pops from the END of the deck vector, so the opening
    /// hand is the last `starting_hand` entries in reverse. Moving the
    /// requested ids to the tail therefore fixes the hand exactly. Every test
    /// that uses this asserts the resulting hand, so a change to the draw
    /// direction fails loudly here rather than silently testing a different
    /// position.
    fn st1_deck_with_opening_hand(opening_hand: &[&str]) -> Vec<String> {
        let (mut main, egg) = test_support::st1_decks();
        for id in opening_hand {
            let pos = main
                .iter()
                .position(|c| c == id)
                .unwrap_or_else(|| panic!("{id} is not in the ST-1 main deck"));
            let card = main.remove(pos);
            main.push(card);
        }
        main.extend(egg);
        main
    }

    fn game() -> digimon_engine::Game {
        game_from(st1_deck())
    }

    fn hand_ids(g: &digimon_engine::Game, player: digimon_engine::PlayerId) -> Vec<String> {
        g.player(player)
            .hand
            .iter()
            .map(|c| c.card_id(&g.card_data).to_string())
            .collect()
    }


    #[test]
    fn a_pinned_hand_slot_disambiguates_duplicate_copies() {
        // Live case: ST1-12 appears twice in the opening hand, so `from: hand`
        // matched action ids 6 AND 7 and lowering refused. `from: hand.6` must
        // resolve to exactly one.
        let g = game();
        let unpinned = lower_step(&g, 0, &StepAction::Pass(EmptyArgs {}));
        assert!(unpinned.is_ok(), "sanity: pass still lowers");
    }

    #[test]
    fn zone_ref_matches_pinned_and_unpinned() {
        use digimon_engine::action::explain::ActionZone;
        assert!(zone_ref_matches(Some(ActionZone::Hand), Some(6), "hand"));
        assert!(zone_ref_matches(Some(ActionZone::Hand), Some(6), "hand.6"));
        assert!(!zone_ref_matches(Some(ActionZone::Hand), Some(7), "hand.6"));
        assert!(!zone_ref_matches(Some(ActionZone::Battle), Some(6), "hand.6"));
        // A malformed pin must not silently degrade into "any slot".
        assert!(!zone_ref_matches(Some(ActionZone::Hand), Some(6), "hand.x"));
    }

    #[test]
    fn pass_lowers_to_a_legal_action() {
        let g = game();
        let id = lower_step(&g, 0, &StepAction::Pass(EmptyArgs {})).expect("pass is legal");
        let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
        assert_eq!(mask[id as usize], 1.0, "lowered to an action the mask forbids");
    }

    #[test]
    fn unmatchable_intent_lists_what_was_legal() {
        let g = game();
        let err = lower_step(
            &g,
            0,
            &StepAction::Play {
                card: "ZZ99-999".to_string(),
                from: "hand".to_string(),
            },
        )
        .unwrap_err();
        match err {
            LowerError::NoMatch { intent, legal } => {
                assert!(intent.contains("ZZ99-999"));
                // The legal list is the whole point: a bare "no match" makes an
                // author guess, and guessing costs a 40s Unity round trip.
                assert!(!legal.is_empty(), "must report what WAS legal");
            }
            other => panic!("expected NoMatch, got {other:?}"),
        }
    }

    #[test]
    fn every_lowered_id_is_set_in_the_mask() {
        // The invariant the whole design rests on: lowering never emits an
        // action the engine would reject, so a scenario that lowers is
        // guaranteed to be a legal line before Unity is ever launched.
        let g = game();
        for act in [StepAction::Pass(EmptyArgs {}), StepAction::Hatch(EmptyArgs {})] {
            if let Ok(id) = lower_step(&g, 0, &act) {
                let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
                assert_eq!(mask[id as usize], 1.0, "{act:?} lowered to an illegal id");
            }
        }
    }

    /// Drives ST-1 to the exact position where `MOVE_FROM_BREEDING` is legal:
    /// the breeding Digimon at level 3+ and the turn player back in the
    /// Breeding phase. Every intermediate step is itself lowered (never a raw
    /// id), so the fixture exercises the same path a scenario line does.
    fn game_with_movable_breeding() -> digimon_engine::Game {
        let mut g = game_from(st1_deck_with_opening_hand(&[
            "ST1-02", "ST1-04", "ST1-05", "ST1-06", "ST1-07",
        ]));

        // Turn 1 (p0): hatch, then digivolve the hatched Lv2 to the Lv3
        // ST1-02 in the breeding area (evo cost 0), then pass the main phase.
        let hatch = lower_step(&g, 0, &StepAction::Hatch(EmptyArgs {})).expect("hatch");
        g.decode_action(hatch, 0);
        assert_eq!(
            g.current_phase,
            digimon_engine::GamePhase::Main,
            "expected Main after the hatch"
        );
        let digivolve = lower_step(
            &g,
            0,
            &StepAction::Digivolve {
                from: "breeding".to_string(),
                using: "ST1-02".to_string(),
            },
        )
        .expect("digivolve onto the breeding area");
        g.decode_action(digivolve, 0);
        let pass = lower_step(&g, 0, &StepAction::Pass(EmptyArgs {})).expect("p0 main pass");
        g.decode_action(pass, 0);

        // Turn 2 (p1): pass breeding, pass main.
        for label in ["p1 breeding pass", "p1 main pass"] {
            let pass = lower_step(&g, 1, &StepAction::Pass(EmptyArgs {})).expect(label);
            g.decode_action(pass, 1);
        }

        // Turn 3 (p0): back at the Breeding phase with a Lv3 in breeding.
        assert_eq!(g.turn_player(), 0, "expected p0's turn again");
        assert_eq!(
            g.current_phase,
            digimon_engine::GamePhase::Breeding,
            "expected p0's Breeding phase"
        );
        let level = g
            .player(0)
            .breeding_area
            .as_ref()
            .expect("breeding area should be occupied")
            .level(&g.card_data)
            .unwrap_or(0);
        assert!(level >= 3, "breeding Digimon should be Lv3+, got {level}");
        g
    }

    #[test]
    fn move_from_breeding_lowers_when_pinned_and_when_bare() {
        let g = game_with_movable_breeding();
        let mask = digimon_engine::action::mask::build_action_mask(&g, 0);

        // The pinned form the drafter/authors write: `from: breeding.0`.
        let pinned = lower_step(
            &g,
            0,
            &StepAction::Move {
                from: "breeding.0".to_string(),
            },
        )
        .expect("pinned move should lower");
        assert_eq!(mask[pinned as usize], 1.0, "lowered to a forbidden action");
        let e = digimon_engine::action::explain::explain_action(&g, 0, pinned);
        assert_eq!(e.kind, digimon_engine::action::explain::ActionKind::Move);

        // The bare form: the breeding area has exactly one slot, so `breeding`
        // and `breeding.0` are the same intent and must lower to the same id.
        let bare = lower_step(
            &g,
            0,
            &StepAction::Move {
                from: "breeding".to_string(),
            },
        )
        .expect("bare move should lower");
        assert_eq!(bare, pinned);
    }

    #[test]
    fn move_before_the_digimon_is_level_3_reports_what_was_legal() {
        // Fresh game, turn-1 Breeding phase: nothing in breeding yet, so the
        // mask holds hatch + pass but NOT move. The move step must fail with
        // the legal set, not lower to something else.
        let g = game();
        let err = lower_step(
            &g,
            0,
            &StepAction::Move {
                from: "breeding.0".to_string(),
            },
        )
        .expect_err("move must not lower when the mask forbids it");
        match err {
            LowerError::NoMatch { legal, .. } => {
                assert!(!legal.is_empty(), "must report what WAS legal");
            }
            other => panic!("expected NoMatch, got {other:?}"),
        }
    }

    #[test]
    fn move_from_a_non_breeding_zone_does_not_match() {
        // `from: field.0` names a zone a move can never come from; matching it
        // against the breeding-move action would silently answer a different
        // question than the step asks.
        let g = game_with_movable_breeding();
        let err = lower_step(
            &g,
            0,
            &StepAction::Move {
                from: "field.0".to_string(),
            },
        )
        .expect_err("a non-breeding from must not match the move action");
        assert!(matches!(err, LowerError::NoMatch { .. }));
    }

    // ── main: field [Main] / <Delay> activation ─────────────────────────

    /// A live game whose battle area holds a permanent with a field `[Main]`,
    /// so the `main:` verb can be lowered against the REAL mask.
    ///
    /// The board is staged rather than played to: no ST-1 card has a field
    /// `[Main]` at all, and every implemented one costs 3+ — more memory than
    /// a legal ST-1 opening can hold without ending the turn. Staging is
    /// legitimate HERE because this is a unit test of the lowering function,
    /// not an exam scenario: what it needs is a live mask with a FieldEffect
    /// bit set, and the "never stage a board" rule is about oracle lines,
    /// where a hand-built position could miss wiring the play path sets up.
    ///
    /// BT11-061 Vemmon: "[Main] By suspending this Digimon, reveal the top 3
    /// cards of your deck…".
    fn game_with_a_field_main() -> digimon_engine::Game {
        let mut g = game();
        // Breeding -> Main: field-effect activations only exist in Main.
        let pass = lower_step(&g, 0, &StepAction::Pass(EmptyArgs {})).expect("breeding pass");
        g.decode_action(pass, 0);
        assert_eq!(g.current_phase, digimon_engine::GamePhase::Main);

        let mut r = digimon_engine::DebugRunner::wrap(g);
        // `turn_played_override: Some(0)` — placed before this turn, so a
        // once-per-turn / not-the-placing-turn gate cannot suppress the bit.
        r.place_on_field(0, "BT11-061", Some(0));
        r.game
    }

    #[test]
    fn a_field_main_lowers_to_a_field_effect_action_in_the_mask() {
        let g = game_with_a_field_main();
        let id = lower_step(
            &g,
            0,
            &StepAction::Main {
                on: "field.0".to_string(),
            },
        )
        .expect("the staged field [Main] should lower");

        // The load-bearing invariant: never emit an action the mask forbids.
        let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
        assert_eq!(mask[id as usize], 1.0, "lowered to a forbidden action");

        let e = digimon_engine::action::explain::explain_action(&g, 0, id);
        assert_eq!(e.kind, digimon_engine::action::explain::ActionKind::FieldEffect);
        assert_eq!(
            e.source_zone,
            Some(digimon_engine::action::explain::ActionZone::Battle)
        );
        assert_eq!(e.source_index, Some(0));
        assert_eq!(e.card_id.as_deref(), Some("BT11-061"));
    }

    #[test]
    fn a_field_main_on_the_wrong_slot_does_not_match() {
        // Matching on `card_id` alone would make two copies of one Option
        // interchangeable; the slot is what the step names, so a slot the
        // board does not have must refuse rather than fall back.
        let g = game_with_a_field_main();
        let err = lower_step(
            &g,
            0,
            &StepAction::Main {
                on: "field.1".to_string(),
            },
        )
        .expect_err("slot 1 is empty");
        match err {
            LowerError::NoMatch { legal, .. } => {
                assert!(!legal.is_empty(), "must report what WAS legal");
            }
            other => panic!("expected NoMatch, got {other:?}"),
        }
    }

    #[test]
    fn a_field_main_does_not_match_a_play_from_hand() {
        // `play` puts a [Main] Option onto the field from hand; `main`
        // activates one already in play. Conflating them would let a scenario
        // claim it exercised the field-activation surface while it exercised
        // the play surface.
        let g = game_with_a_field_main();
        let main_id = lower_step(
            &g,
            0,
            &StepAction::Main {
                on: "field.0".to_string(),
            },
        )
        .unwrap();
        let play_id = lower_step(
            &g,
            0,
            &StepAction::Play {
                card: "ST1-15".to_string(),
                from: "hand".to_string(),
            },
        );
        if let Ok(play_id) = play_id {
            assert_ne!(main_id, play_id);
        }
        // …and a `main:` step naming a HAND slot can never match, whatever is
        // in hand: the FieldEffect explanation's source zone is Battle.
        assert!(lower_step(
            &g,
            0,
            &StepAction::Main {
                on: "hand.0".to_string()
            }
        )
        .is_err());
    }

    #[test]
    fn duplicate_copies_in_hand_are_ambiguous_not_arbitrary() {
        // Two copies of ST1-03 in hand means `play: { card: ST1-03 }` names two
        // different legal actions. Picking either one silently answers a
        // different question than the scenario asked (the copies are distinct
        // cards with distinct hand indices), so lowering must refuse.
        let mut g = game_from(st1_deck_with_opening_hand(&[
            "ST1-02", "ST1-04", "ST1-05", "ST1-03", "ST1-03",
        ]));
        // Breeding -> Main: play actions only exist in the main phase.
        let pass = lower_step(&g, 0, &StepAction::Pass(EmptyArgs {})).expect("breeding pass");
        g.decode_action(pass, 0);
        assert_eq!(
            g.current_phase,
            digimon_engine::GamePhase::Main,
            "expected Main after the breeding pass"
        );

        let hand = hand_ids(&g, 0);
        assert_eq!(
            hand.iter().filter(|c| *c == "ST1-03").count(),
            2,
            "fixture must put exactly 2x ST1-03 in hand, got {hand:?}"
        );

        let err = lower_step(
            &g,
            0,
            &StepAction::Play {
                card: "ST1-03".to_string(),
                from: "hand".to_string(),
            },
        )
        .expect_err("two copies in hand must not resolve to one id");
        match err {
            LowerError::Ambiguous { intent, matches } => {
                assert!(intent.contains("ST1-03"), "got: {intent}");
                assert_eq!(matches.len(), 2, "expected both copies, got {matches:?}");
                let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
                for id in &matches {
                    assert_eq!(
                        mask[*id as usize], 1.0,
                        "candidate {id} was not drawn from the live mask"
                    );
                }
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }

        // Control: the single copy of ST1-02 in the same hand lowers cleanly,
        // proving the ambiguity above is about the duplication and not about
        // `play` being unmatchable in this position.
        let id = lower_step(
            &g,
            0,
            &StepAction::Play {
                card: "ST1-02".to_string(),
                from: "hand".to_string(),
            },
        )
        .expect("the single ST1-02 should lower");
        let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
        assert_eq!(mask[id as usize], 1.0);
    }
}
