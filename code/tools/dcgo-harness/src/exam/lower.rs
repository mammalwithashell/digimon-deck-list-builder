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
        // Selections resolve against the live PendingSelection rather than the
        // main mask; Task 3 threads them through ScenarioAdapter, which is why
        // they never match here.
        StepAction::Select { .. } => false,
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
